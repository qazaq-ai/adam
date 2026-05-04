//! **v4.45.0** — Stage B foundation: selection weights over rule-
//! generated candidates.
//!
//! ## What this is
//!
//! The substrate for the project's Stage B architectural arc. When
//! the SearchGraph or NLG layer produces multiple candidate
//! responses for a single user query, this module scores each
//! candidate via a small dot-product over hand-extracted features,
//! and the planner picks the top-1.
//!
//! ## Why selection weights
//!
//! Per the project thesis (`project_retrieval_not_neural_v2.md`,
//! refined 2026-05-03): adam aims to be a NEW class of generative
//! AI that uses the agglutinative paradigm — typed primitives +
//! rule-based composition + **tiny selection weights** — to be
//! safe / cheap / predictable while reaching LLM-comparable
//! abilities. Stage A (v4.42.0–v4.43.9) built the typed primitives
//! and rule-based NLG. Stage B starts here: selection among rule-
//! generated candidates.
//!
//! Weights are KB-scale (not billions), trained on the clean Kazakh
//! corpus (3.87 M → 100 M, NOT internet), discrete and inspectable.
//! Each weight is readable as a number in a table. No softmax
//! sampling, no opaque attention; deterministic per seed.
//!
//! ## Status (v4.45.0 — Stage B bundle 1)
//!
//! Foundation only:
//! - [`CandidateFeatures`] — hand-extracted features for a single
//!   candidate (predicate-kind, confidence, raw_text richness,
//!   subject/object token overlap, recency-match).
//! - [`SelectionWeights`] — discrete weight table, currently
//!   hand-set to defaults that approximate the v4.38.0 SearchGraph
//!   ranker tiers.
//! - [`extract_features`] — pure function from
//!   `(ReasFact, query_tokens, last_topic)` → `CandidateFeatures`.
//! - [`score`] — dot-product of features × weights → `f32`.
//! - [`select_top`] — picks the highest-scoring candidate;
//!   stable-tie-break by index (preserves caller-provided order).
//!
//! NOT YET in v4.45.0:
//! - Training pipeline (Stage B bundle 2+).
//! - Production wiring (Stage B bundle 3+: route SearchGraph's
//!   top-N through the selector).
//! - Per-domain weight specialization.
//! - Categorical predicate-kind one-hot encoding (currently dropped
//!   from features as low-leverage in v0).
//!
//! ## Layered stack (project thesis)
//!
//! ```text
//! 1. FST morphology         (already)
//! 2. Typed SemFrame IR      (already, partial)
//! 3. world_core knowledge   (already)
//! 4. Reasoner               (already)
//! 5. Rule-based sentence NLG (Stage A — already)
//! 6. Selection weights      ← THIS MODULE (Stage B)
//! 7. Realiser (FST forward) (already)
//! ```

use adam_reasoning::{ConfidenceKind, Fact as ReasFact};

/// Hand-extracted features for a single candidate fact. Each field
/// is normalized to roughly `[0.0, 1.0]` so weight magnitudes are
/// comparable. `score` is the dot-product
/// `Σ wᵢ · fᵢ + bias`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CandidateFeatures {
    /// Confidence score: 1.0 for `HumanApproved` (curated
    /// world_core), 0.5 for `RuleInferred` (reasoner derivation),
    /// 0.0 for `Grammar` (extractor-pattern match — least trusted).
    pub confidence: f32,
    /// Richness of the curated `raw_text`. Long descriptive
    /// `raw_text` (e.g. the rich Kazakhstan IsA fact) carries more
    /// information than a bare mechanical composition. Normalized
    /// as `min(raw_text.chars().count() / 100.0, 1.0)`.
    pub raw_text_richness: f32,
    /// Token-level overlap between the fact's subject root and the
    /// query tokens. `(matching_tokens / max(query_tokens, 1))`,
    /// clamped to `[0.0, 1.0]`.
    pub subject_overlap: f32,
    /// Token-level overlap between the fact's object root and the
    /// query tokens. Same shape as `subject_overlap`.
    pub object_overlap: f32,
    /// Discrete recency signal: 1.0 if the fact's subject root
    /// matches the previous turn's `last_query_topic` (sticky-
    /// subject continuation, v4.13.0+ DialogContext), else 0.0.
    pub recency_match: f32,
}

/// Tiny inspectable weight table. Six numbers + bias = total of
/// `~28 bytes` for the v0 model. Fits in any constant-data section;
/// no allocation; deterministic per seed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectionWeights {
    pub bias: f32,
    pub w_confidence: f32,
    pub w_richness: f32,
    pub w_subject_overlap: f32,
    pub w_object_overlap: f32,
    pub w_recency: f32,
}

impl SelectionWeights {
    /// **v4.45.0 default weights** — hand-set to approximate the
    /// v4.38.0 SearchGraph ranker tiers. Subject overlap is the
    /// strongest signal (matches the v4.38.0 quantity-aware /
    /// list-aware tiers); confidence sets a floor; richness is a
    /// soft tiebreaker; recency is a small additive bonus.
    ///
    /// These defaults aren't trained — they're the "seed" weights
    /// that make the selector a no-op vs. the existing ranker on
    /// the canonical evals. Stage B bundle 2+ will replace this
    /// with weights trained on the committed corpus.
    pub fn default_v0() -> Self {
        Self {
            bias: 0.0,
            w_confidence: 1.0,
            w_richness: 0.3,
            w_subject_overlap: 2.0,
            w_object_overlap: 1.0,
            w_recency: 0.5,
        }
    }
}

impl Default for SelectionWeights {
    fn default() -> Self {
        Self::default_v0()
    }
}

/// Pure feature extractor. `query_tokens` is the lowered, token-
/// split user input; `last_topic` is the prior turn's topic root
/// (for the recency-match feature) or `None` on the first turn.
pub fn extract_features(
    fact: &ReasFact,
    query_tokens: &[&str],
    last_topic: Option<&str>,
) -> CandidateFeatures {
    let confidence = match fact.confidence {
        ConfidenceKind::HumanApproved => 1.0,
        ConfidenceKind::CuratedQuote => 0.8,
        ConfidenceKind::RuleInferred => 0.5,
        ConfidenceKind::RepeatedPattern => 0.3,
        ConfidenceKind::Grammar => 0.0,
    };
    let raw_len = fact.raw_text.chars().count() as f32;
    let raw_text_richness = (raw_len / 100.0).min(1.0);
    let subject_overlap = token_overlap(&fact.subject.root, query_tokens);
    let object_overlap = token_overlap(&fact.object.root, query_tokens);
    let recency_match = match last_topic {
        Some(topic) if topic == fact.subject.root => 1.0,
        _ => 0.0,
    };
    CandidateFeatures {
        confidence,
        raw_text_richness,
        subject_overlap,
        object_overlap,
        recency_match,
    }
}

/// Compute dot-product `Σ wᵢ · fᵢ + bias`.
pub fn score(features: &CandidateFeatures, weights: &SelectionWeights) -> f32 {
    weights.bias
        + weights.w_confidence * features.confidence
        + weights.w_richness * features.raw_text_richness
        + weights.w_subject_overlap * features.subject_overlap
        + weights.w_object_overlap * features.object_overlap
        + weights.w_recency * features.recency_match
}

/// Select the highest-scoring candidate. Stable-tie-break by index
/// (preserves caller-provided order — important for byte-identical
/// regression on canonical evals where the existing ranker's order
/// matters when scores tie). Returns `None` if `candidates` is
/// empty.
pub fn select_top<'a>(
    candidates: &'a [&'a ReasFact],
    weights: &SelectionWeights,
    query_tokens: &[&str],
    last_topic: Option<&str>,
) -> Option<&'a ReasFact> {
    if candidates.is_empty() {
        return None;
    }
    let mut best_idx = 0usize;
    let mut best_score = f32::NEG_INFINITY;
    for (i, fact) in candidates.iter().enumerate() {
        let f = extract_features(fact, query_tokens, last_topic);
        let s = score(&f, weights);
        if s > best_score {
            best_score = s;
            best_idx = i;
        }
    }
    Some(candidates[best_idx])
}

/// How many of the query tokens match the candidate's slot root,
/// normalized by the query's token count. Tokenization on whitespace
/// + casefold; tokens shorter than 3 chars are skipped (closed-class
/// noise reduction, mirrors NOT_A_TOPIC's spirit).
fn token_overlap(slot_root: &str, query_tokens: &[&str]) -> f32 {
    if query_tokens.is_empty() {
        return 0.0;
    }
    let slot_lower = slot_root.to_lowercase();
    let slot_tokens: Vec<&str> = slot_lower
        .split_whitespace()
        .filter(|t| t.chars().count() >= 3)
        .collect();
    if slot_tokens.is_empty() {
        return 0.0;
    }
    let mut matches = 0usize;
    let mut comparable_tokens = 0usize;
    for q in query_tokens {
        if q.chars().count() < 3 {
            continue;
        }
        comparable_tokens += 1;
        let q_lower = q.to_lowercase();
        if slot_tokens.iter().any(|s| q_lower.contains(s)) {
            matches += 1;
        }
    }
    if comparable_tokens == 0 {
        return 0.0;
    }
    (matches as f32 / comparable_tokens as f32).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use adam_reasoning::{FactSource, Predicate, SlotRef};

    fn make_fact(
        subject: &str,
        predicate: Predicate,
        object: &str,
        raw: &str,
        confidence: ConfidenceKind,
    ) -> ReasFact {
        ReasFact {
            subject: SlotRef {
                surface: subject.to_string(),
                root: subject.to_string(),
                pos: "noun".to_string(),
            },
            predicate,
            object: SlotRef {
                surface: object.to_string(),
                root: object.to_string(),
                pos: "noun".to_string(),
            },
            pattern: "test".to_string(),
            source: FactSource {
                pack: "test".to_string(),
                sample_id: "test".to_string(),
            },
            confidence,
            raw_text: raw.to_string(),
        }
    }

    #[test]
    fn weights_default_v0_is_deterministic() {
        let a = SelectionWeights::default_v0();
        let b = SelectionWeights::default_v0();
        assert_eq!(a, b);
        // Default is the trait-default.
        assert_eq!(SelectionWeights::default(), a);
    }

    #[test]
    fn extract_features_human_approved_confidence_is_one() {
        let fact = make_fact(
            "қазақстан",
            Predicate::IsA,
            "мемлекет",
            "Қазақстан — мемлекет.",
            ConfidenceKind::HumanApproved,
        );
        let f = extract_features(&fact, &[], None);
        assert!((f.confidence - 1.0).abs() < 1e-6);
    }

    #[test]
    fn extract_features_grammar_confidence_is_zero() {
        let fact = make_fact(
            "қазақстан",
            Predicate::IsA,
            "мемлекет",
            "",
            ConfidenceKind::Grammar,
        );
        let f = extract_features(&fact, &[], None);
        assert_eq!(f.confidence, 0.0);
    }

    #[test]
    fn extract_features_richness_caps_at_one() {
        let long_raw = "ы".repeat(500);
        let fact = make_fact(
            "тақырып",
            Predicate::IsA,
            "сипаттама",
            &long_raw,
            ConfidenceKind::HumanApproved,
        );
        let f = extract_features(&fact, &[], None);
        assert_eq!(f.raw_text_richness, 1.0);
    }

    #[test]
    fn extract_features_richness_short_raw_is_proportional() {
        let fact = make_fact(
            "тақырып",
            Predicate::IsA,
            "сипаттама",
            "10 chars..",
            ConfidenceKind::HumanApproved,
        );
        let f = extract_features(&fact, &[], None);
        // 10 chars / 100.0 = 0.10
        assert!((f.raw_text_richness - 0.10).abs() < 1e-6);
    }

    #[test]
    fn extract_features_subject_overlap_matches_query_token() {
        let fact = make_fact(
            "қазақстан",
            Predicate::IsA,
            "мемлекет",
            "Қазақстан — мемлекет.",
            ConfidenceKind::HumanApproved,
        );
        let query = ["қазақстан", "туралы", "айтыңыз"];
        let f = extract_features(&fact, &query, None);
        // 1 of 3 query tokens overlaps subject ≥ 3-char prefix → 1/3.
        assert!((f.subject_overlap - 1.0 / 3.0).abs() < 1e-3);
    }

    #[test]
    fn extract_features_recency_match_when_subject_equals_last_topic() {
        let fact = make_fact(
            "rust",
            Predicate::IsA,
            "тіл",
            "Rust — тіл.",
            ConfidenceKind::HumanApproved,
        );
        let f = extract_features(&fact, &[], Some("rust"));
        assert_eq!(f.recency_match, 1.0);
    }

    #[test]
    fn extract_features_recency_zero_when_topic_differs() {
        let fact = make_fact(
            "rust",
            Predicate::IsA,
            "тіл",
            "Rust — тіл.",
            ConfidenceKind::HumanApproved,
        );
        let f = extract_features(&fact, &[], Some("python"));
        assert_eq!(f.recency_match, 0.0);
    }

    #[test]
    fn score_is_dot_product_plus_bias() {
        let f = CandidateFeatures {
            confidence: 1.0,
            raw_text_richness: 0.5,
            subject_overlap: 0.5,
            object_overlap: 0.0,
            recency_match: 0.0,
        };
        let w = SelectionWeights {
            bias: 0.1,
            w_confidence: 1.0,
            w_richness: 0.3,
            w_subject_overlap: 2.0,
            w_object_overlap: 1.0,
            w_recency: 0.5,
        };
        // 0.1 + 1.0*1.0 + 0.3*0.5 + 2.0*0.5 + 1.0*0.0 + 0.5*0.0
        // = 0.1 + 1.0 + 0.15 + 1.0 + 0.0 + 0.0 = 2.25
        assert!((score(&f, &w) - 2.25).abs() < 1e-6);
    }

    #[test]
    fn select_top_picks_highest_scoring() {
        // Two candidates: one HumanApproved with rich raw_text,
        // one Grammar with empty raw_text. Default weights should
        // prefer the curated one by a wide margin.
        let curated = make_fact(
            "қазақстан",
            Predicate::IsA,
            "мемлекет",
            "Қазақстан — мемлекет; Орталық Азиядағы тәуелсіз ел.",
            ConfidenceKind::HumanApproved,
        );
        let grammar = make_fact(
            "қазақстан",
            Predicate::IsA,
            "адам",
            "",
            ConfidenceKind::Grammar,
        );
        let candidates = [&curated, &grammar];
        let w = SelectionWeights::default_v0();
        let query = ["қазақстан"];
        let picked = select_top(&candidates, &w, &query, None).expect("non-empty");
        assert_eq!(picked.object.root, "мемлекет");
    }

    #[test]
    fn select_top_stable_tie_break_preserves_first_index() {
        // Two identical-feature facts → score ties → first index wins.
        let a = make_fact(
            "x",
            Predicate::IsA,
            "y1",
            "Same length raw.",
            ConfidenceKind::HumanApproved,
        );
        let b = make_fact(
            "x",
            Predicate::IsA,
            "y2",
            "Same length raw.",
            ConfidenceKind::HumanApproved,
        );
        let candidates = [&a, &b];
        let w = SelectionWeights::default_v0();
        let picked = select_top(&candidates, &w, &[], None).expect("non-empty");
        assert_eq!(picked.object.root, "y1");
    }

    #[test]
    fn select_top_empty_returns_none() {
        let w = SelectionWeights::default_v0();
        let candidates: [&ReasFact; 0] = [];
        assert!(select_top(&candidates, &w, &[], None).is_none());
    }

    #[test]
    fn token_overlap_skips_short_tokens() {
        // Short query tokens (< 3 chars) are filtered out as
        // closed-class noise. Subject "rust" + query ["а", "rust"]
        // → only "rust" counts → 1/1 = 1.0.
        let f = extract_features(
            &make_fact(
                "rust",
                Predicate::IsA,
                "тіл",
                "",
                ConfidenceKind::HumanApproved,
            ),
            &["а", "rust"],
            None,
        );
        assert!((f.subject_overlap - 1.0).abs() < 1e-6);
    }
}

// ---------------------------------------------------------------------------
// **v4.45.5** — Stage B bundle 2: training pipeline.
//
// Pairwise margin-perceptron training of [`SelectionWeights`]. Given a list
// of gold pairs `(positive, negative)` — where the positive candidate
// SHOULD score higher than the negative under correct weights — the loop
// nudges weights toward positives and away from negatives until either
// every pair satisfies `score(positive) ≥ score(negative) + margin` or
// `max_epochs` epochs elapse.
//
// Why pairwise margin perceptron:
// - Deterministic per (initial weights, training pairs, hyperparameters)
//   — no random sampling, no SGD batching variance.
// - Convex update on the per-pair margin loss — simple to reason about.
// - O(epochs × pairs) update cost, no gradient computation graph, no
//   library dependency.
// - Outputs a tiny inspectable weight table (still ~28 bytes for v0).
//
// The training loop respects the project thesis: small clean training
// data → small inspectable weights. A fully-converged v0 model on a
// hand-curated 50-pair eval suite occupies the same 28 bytes as the
// hand-set defaults.

/// One labelled training example: `positive` is the candidate whose
/// score should EXCEED `negative.score + margin` after training.
#[derive(Debug, Clone, Copy)]
pub struct TrainingPair {
    pub positive: CandidateFeatures,
    pub negative: CandidateFeatures,
}

/// Hyperparameters for the perceptron training loop. Defaults are
/// hand-tuned for the v0 feature set on synthetic linearly-separable
/// data; revisit when real REPL-transcript pairs become available.
#[derive(Debug, Clone, Copy)]
pub struct TrainingConfig {
    /// Step size for each weight update.
    pub learning_rate: f32,
    /// Required margin: `score(positive) ≥ score(negative) + margin`
    /// before the pair is considered satisfied.
    pub margin: f32,
    /// Maximum number of full passes over the training pairs.
    pub max_epochs: usize,
}

impl TrainingConfig {
    /// **v4.45.5 default** — tuned for the v0 feature scale (`[0, 1]`)
    /// and the hand-set initial weights in [`SelectionWeights::default_v0`].
    pub fn default_v0() -> Self {
        Self {
            learning_rate: 0.1,
            margin: 0.5,
            max_epochs: 200,
        }
    }
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self::default_v0()
    }
}

/// What the training loop reports back. Useful for tests, telemetry,
/// and convergence checks. Never inspected by the live planner —
/// production wiring (Stage B bundle 3+) consumes only the trained
/// `SelectionWeights`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrainingStats {
    /// Number of epochs that ran (≤ `config.max_epochs`).
    pub epochs_run: usize,
    /// Number of pair violations in the FINAL epoch — pairs where
    /// `score(positive) < score(negative) + margin`. Zero ⇒
    /// converged.
    pub final_violations: usize,
    /// Did training converge before hitting `max_epochs`?
    pub converged: bool,
}

/// Pairwise margin-perceptron training. Returns the trained weights
/// + convergence stats. Pure function — same `(initial, pairs,
/// config)` always yields the same output.
pub fn train_perceptron(
    initial: SelectionWeights,
    pairs: &[TrainingPair],
    config: TrainingConfig,
) -> (SelectionWeights, TrainingStats) {
    let mut w = initial;
    let mut last_violations = pairs.len();
    let mut epochs_run = 0usize;
    let mut converged = false;
    for _ in 0..config.max_epochs {
        epochs_run += 1;
        let mut violations = 0usize;
        for pair in pairs {
            let s_pos = score(&pair.positive, &w);
            let s_neg = score(&pair.negative, &w);
            if s_pos < s_neg + config.margin {
                violations += 1;
                // Bump weights toward (positive - negative). Each
                // weight component is updated by
                //   lr * (pos_feature - neg_feature).
                w.bias += config.learning_rate * 0.0; // bias has no feature
                w.w_confidence +=
                    config.learning_rate * (pair.positive.confidence - pair.negative.confidence);
                w.w_richness += config.learning_rate
                    * (pair.positive.raw_text_richness - pair.negative.raw_text_richness);
                w.w_subject_overlap += config.learning_rate
                    * (pair.positive.subject_overlap - pair.negative.subject_overlap);
                w.w_object_overlap += config.learning_rate
                    * (pair.positive.object_overlap - pair.negative.object_overlap);
                w.w_recency += config.learning_rate
                    * (pair.positive.recency_match - pair.negative.recency_match);
            }
        }
        last_violations = violations;
        if violations == 0 {
            converged = true;
            break;
        }
    }
    (
        w,
        TrainingStats {
            epochs_run,
            final_violations: last_violations,
            converged,
        },
    )
}

#[cfg(test)]
mod training_tests {
    use super::*;

    fn pair(pos: CandidateFeatures, neg: CandidateFeatures) -> TrainingPair {
        TrainingPair {
            positive: pos,
            negative: neg,
        }
    }

    fn feats(c: f32, r: f32, s: f32, o: f32, recency: f32) -> CandidateFeatures {
        CandidateFeatures {
            confidence: c,
            raw_text_richness: r,
            subject_overlap: s,
            object_overlap: o,
            recency_match: recency,
        }
    }

    #[test]
    fn training_config_default_is_deterministic() {
        let a = TrainingConfig::default_v0();
        let b = TrainingConfig::default_v0();
        assert_eq!(a.learning_rate, b.learning_rate);
        assert_eq!(a.margin, b.margin);
        assert_eq!(a.max_epochs, b.max_epochs);
    }

    #[test]
    fn perceptron_converges_on_linearly_separable_pairs() {
        // Gold rule: prefer high-confidence facts over low-confidence
        // facts. One clean dimension; should converge in few epochs.
        let pairs = vec![
            pair(
                feats(1.0, 0.5, 0.5, 0.5, 0.0),
                feats(0.0, 0.5, 0.5, 0.5, 0.0),
            ),
            pair(
                feats(0.8, 0.3, 0.3, 0.3, 0.0),
                feats(0.0, 0.3, 0.3, 0.3, 0.0),
            ),
        ];
        // Initial weights all zero — no preference. Training must
        // discover that confidence matters.
        let initial = SelectionWeights {
            bias: 0.0,
            w_confidence: 0.0,
            w_richness: 0.0,
            w_subject_overlap: 0.0,
            w_object_overlap: 0.0,
            w_recency: 0.0,
        };
        let (trained, stats) = train_perceptron(initial, &pairs, TrainingConfig::default_v0());
        assert!(stats.converged);
        assert_eq!(stats.final_violations, 0);
        // After training, confidence weight must be positive.
        assert!(trained.w_confidence > 0.0);
    }

    #[test]
    fn perceptron_is_deterministic() {
        let pairs = vec![pair(
            feats(1.0, 0.5, 0.5, 0.5, 0.0),
            feats(0.0, 0.5, 0.5, 0.5, 0.0),
        )];
        let initial = SelectionWeights::default_v0();
        let cfg = TrainingConfig::default_v0();
        let (a, sa) = train_perceptron(initial, &pairs, cfg);
        let (b, sb) = train_perceptron(initial, &pairs, cfg);
        assert_eq!(a, b);
        assert_eq!(sa, sb);
    }

    #[test]
    fn perceptron_no_op_when_pairs_already_satisfy_margin() {
        // Initial weights already separate these pairs by > margin —
        // training must NOT update weights.
        let pairs = vec![pair(
            feats(1.0, 0.5, 0.5, 0.5, 0.0),
            feats(0.0, 0.5, 0.5, 0.5, 0.0),
        )];
        let initial = SelectionWeights {
            bias: 0.0,
            w_confidence: 10.0, // pos scores 10 + 1 = 11; neg scores 0 + 1 = 1; margin OK
            w_richness: 0.0,
            w_subject_overlap: 1.0,
            w_object_overlap: 0.0,
            w_recency: 0.0,
        };
        let (trained, stats) = train_perceptron(initial, &pairs, TrainingConfig::default_v0());
        assert!(stats.converged);
        assert_eq!(stats.final_violations, 0);
        assert_eq!(stats.epochs_run, 1);
        assert_eq!(trained, initial);
    }

    #[test]
    fn perceptron_reports_violations_when_max_epochs_reached() {
        // Construct an unsatisfiable contradiction: gold says A > B,
        // but A and B have IDENTICAL features → score gap is zero,
        // never reaches margin. Loop should hit max_epochs.
        let same = feats(0.5, 0.5, 0.5, 0.5, 0.0);
        let pairs = vec![pair(same, same)];
        let initial = SelectionWeights::default_v0();
        let cfg = TrainingConfig {
            max_epochs: 5,
            ..TrainingConfig::default_v0()
        };
        let (_w, stats) = train_perceptron(initial, &pairs, cfg);
        assert!(!stats.converged);
        assert_eq!(stats.epochs_run, 5);
        assert_eq!(stats.final_violations, 1);
    }

    #[test]
    fn perceptron_empty_pairs_returns_initial_immediately() {
        let initial = SelectionWeights::default_v0();
        let cfg = TrainingConfig::default_v0();
        let (trained, stats) = train_perceptron(initial, &[], cfg);
        // No pairs ⇒ no violations, converges in epoch 1.
        assert!(stats.converged);
        assert_eq!(stats.final_violations, 0);
        assert_eq!(trained, initial);
    }

    #[test]
    fn perceptron_learns_recency_signal() {
        // Gold: prefer the recency-matching candidate (anaphor
        // continuation) when other features are equal.
        let pairs = vec![pair(
            feats(0.5, 0.5, 0.5, 0.5, 1.0),
            feats(0.5, 0.5, 0.5, 0.5, 0.0),
        )];
        let initial = SelectionWeights {
            bias: 0.0,
            w_confidence: 0.0,
            w_richness: 0.0,
            w_subject_overlap: 0.0,
            w_object_overlap: 0.0,
            w_recency: 0.0,
        };
        let (trained, stats) = train_perceptron(initial, &pairs, TrainingConfig::default_v0());
        assert!(stats.converged);
        // Recency weight must end positive.
        assert!(trained.w_recency > 0.0);
        // Other weights remain zero — single-dimension signal.
        assert_eq!(trained.w_confidence, 0.0);
        assert_eq!(trained.w_subject_overlap, 0.0);
    }

    #[test]
    fn perceptron_zero_learning_rate_makes_no_progress() {
        let pairs = vec![pair(
            feats(1.0, 0.5, 0.5, 0.5, 0.0),
            feats(0.0, 0.5, 0.5, 0.5, 0.0),
        )];
        let initial = SelectionWeights {
            bias: 0.0,
            w_confidence: 0.0,
            w_richness: 0.0,
            w_subject_overlap: 0.0,
            w_object_overlap: 0.0,
            w_recency: 0.0,
        };
        let cfg = TrainingConfig {
            learning_rate: 0.0,
            max_epochs: 50,
            ..TrainingConfig::default_v0()
        };
        let (trained, stats) = train_perceptron(initial, &pairs, cfg);
        assert!(!stats.converged);
        assert_eq!(trained, initial);
    }

    #[test]
    fn perceptron_converges_within_epoch_budget_on_realistic_pairs() {
        // Synthetic mirror of the v4.38.0 ranker tiers: HumanApproved
        // beats Grammar; subject-overlap dominates; recency adds a
        // small bonus.
        let pairs = vec![
            // HumanApproved should beat Grammar.
            pair(
                feats(1.0, 0.5, 0.5, 0.0, 0.0),
                feats(0.0, 0.5, 0.5, 0.0, 0.0),
            ),
            // Higher subject_overlap should beat lower.
            pair(
                feats(0.5, 0.0, 1.0, 0.0, 0.0),
                feats(0.5, 0.0, 0.0, 0.0, 0.0),
            ),
            // Recency tiebreaker.
            pair(
                feats(0.5, 0.5, 0.5, 0.5, 1.0),
                feats(0.5, 0.5, 0.5, 0.5, 0.0),
            ),
        ];
        let initial = SelectionWeights::default_v0();
        let (trained, stats) = train_perceptron(initial, &pairs, TrainingConfig::default_v0());
        assert!(stats.converged);
        // All three signal weights must be positive after training.
        assert!(trained.w_confidence > 0.0);
        assert!(trained.w_subject_overlap > 0.0);
        assert!(trained.w_recency > 0.0);
    }

    #[test]
    fn perceptron_trained_weights_correctly_rank_via_score() {
        let pairs = vec![pair(
            feats(1.0, 0.0, 0.0, 0.0, 0.0),
            feats(0.0, 0.0, 0.0, 0.0, 0.0),
        )];
        let initial = SelectionWeights {
            bias: 0.0,
            w_confidence: 0.0,
            w_richness: 0.0,
            w_subject_overlap: 0.0,
            w_object_overlap: 0.0,
            w_recency: 0.0,
        };
        let (trained, stats) = train_perceptron(initial, &pairs, TrainingConfig::default_v0());
        assert!(stats.converged);
        // After convergence, score(positive) ≥ score(negative) + margin.
        let s_pos = score(&pairs[0].positive, &trained);
        let s_neg = score(&pairs[0].negative, &trained);
        assert!(s_pos >= s_neg + TrainingConfig::default_v0().margin);
    }

    #[test]
    fn training_stats_reports_epoch_count_correctly() {
        // Linearly separable, so converges in 1 epoch (no violations
        // on the second pass).
        let pairs = vec![pair(
            feats(1.0, 0.5, 0.5, 0.5, 0.0),
            feats(0.0, 0.5, 0.5, 0.5, 0.0),
        )];
        let initial = SelectionWeights::default_v0();
        let (_, stats) = train_perceptron(initial, &pairs, TrainingConfig::default_v0());
        assert!(stats.converged);
        assert!(stats.epochs_run >= 1);
        assert_eq!(stats.final_violations, 0);
    }
}

// ---------------------------------------------------------------------------
// **v4.46.0** — Stage B bundle 3: canonical training pairs + audit
// substrate.
//
// `canonical_training_pairs_v0` returns a hand-curated set of gold
// `(positive, negative)` pairs covering the disambiguation scenarios
// the v4.38.0 SearchGraph ranker tiers handle today. Training on this
// set with `train_perceptron` produces a model that — by construction
// — agrees with the existing heuristic on every canonical scenario.
// This is the v0 training substrate; v4.46.5+ will wire it into the
// production trace path so disagreements with the heuristic ranker
// become visible.
//
// `AuditResult` + `audit_compare` form the pure-function audit
// substrate. Given a list of candidates and the heuristic ranker's
// top choice, audit_compare computes the selector's top choice and
// reports both indices + a `disagreement` boolean. NOT YET wired
// into production in v4.46.0.

/// **v4.46.0** — Hand-curated canonical training pairs covering the
/// disambiguation scenarios that the v4.38.0 SearchGraph ranker
/// tiers handle today. Each pair is a single-axis test of one
/// feature dimension; the training loop must learn that the named
/// feature outweighs noise on the other dimensions.
///
/// Sources: synthetic mirror of v4.38.0 ranker tier tests +
/// generalizations of the v4.42.x–v4.44.0 transcript-driven gap
/// closures. No external dependencies; pure deterministic constant.
pub fn canonical_training_pairs_v0() -> Vec<TrainingPair> {
    fn p(pos: CandidateFeatures, neg: CandidateFeatures) -> TrainingPair {
        TrainingPair {
            positive: pos,
            negative: neg,
        }
    }
    fn f(c: f32, r: f32, s: f32, o: f32, recency: f32) -> CandidateFeatures {
        CandidateFeatures {
            confidence: c,
            raw_text_richness: r,
            subject_overlap: s,
            object_overlap: o,
            recency_match: recency,
        }
    }
    vec![
        // ---- Confidence axis (HumanApproved beats lower kinds) ----
        // (1) HumanApproved (1.0) beats Grammar (0.0) at equal everything else.
        p(f(1.0, 0.5, 0.5, 0.5, 0.0), f(0.0, 0.5, 0.5, 0.5, 0.0)),
        // (2) HumanApproved (1.0) beats RuleInferred (0.5).
        p(f(1.0, 0.5, 0.5, 0.5, 0.0), f(0.5, 0.5, 0.5, 0.5, 0.0)),
        // (3) CuratedQuote (0.8) beats Grammar (0.0).
        p(f(0.8, 0.0, 0.5, 0.5, 0.0), f(0.0, 0.0, 0.5, 0.5, 0.0)),
        // ---- Subject-overlap axis (the dominant tier in v4.38.0) ----
        // (4) Subject overlap 1.0 beats 0.0 at equal confidence.
        p(f(0.5, 0.0, 1.0, 0.0, 0.0), f(0.5, 0.0, 0.0, 0.0, 0.0)),
        // (5) Subject overlap with curated raw_text dominates.
        p(f(1.0, 0.5, 1.0, 0.0, 0.0), f(1.0, 0.5, 0.0, 0.0, 0.0)),
        // (6) Subject overlap dominates over richness alone.
        p(f(0.5, 0.0, 1.0, 0.0, 0.0), f(0.5, 1.0, 0.0, 0.0, 0.0)),
        // ---- Object-overlap axis (secondary tier) ----
        // (7) Object overlap 1.0 beats 0.0 at equal subject.
        p(f(0.5, 0.0, 0.5, 1.0, 0.0), f(0.5, 0.0, 0.5, 0.0, 0.0)),
        // ---- Recency axis (tiebreaker) ----
        // (8) Recency match wins when other features are equal.
        p(f(0.5, 0.5, 0.5, 0.5, 1.0), f(0.5, 0.5, 0.5, 0.5, 0.0)),
        // (9) Recency match doesn't override confidence gap.
        // (Negative test: HumanApproved with no recency beats Grammar with recency.)
        p(f(1.0, 0.5, 0.5, 0.5, 0.0), f(0.0, 0.5, 0.5, 0.5, 1.0)),
        // ---- Richness axis (soft tier) ----
        // (10) Richness 1.0 beats 0.0 at equal everything else.
        p(f(0.5, 1.0, 0.5, 0.5, 0.0), f(0.5, 0.0, 0.5, 0.5, 0.0)),
        // ---- Multi-signal interactions ----
        // (11) HumanApproved + matching subject beats Grammar + non-matching.
        p(f(1.0, 0.0, 1.0, 0.0, 0.0), f(0.0, 0.0, 0.0, 0.0, 0.0)),
        // (12) Curated rich raw_text + recency beats grammar with no signal.
        p(f(1.0, 1.0, 0.5, 0.5, 1.0), f(0.0, 0.0, 0.0, 0.0, 0.0)),
        // (13) Subject overlap + recency beats subject overlap alone.
        p(f(0.5, 0.0, 1.0, 0.0, 1.0), f(0.5, 0.0, 1.0, 0.0, 0.0)),
        // (14) Confidence dominates over recency tiebreaker.
        p(f(1.0, 0.0, 0.0, 0.0, 0.0), f(0.5, 0.0, 0.0, 0.0, 1.0)),
        // (15) Subject + object overlap beats neither.
        p(f(0.5, 0.0, 1.0, 1.0, 0.0), f(0.5, 0.0, 0.0, 0.0, 0.0)),
    ]
}

/// **v4.46.0** — Audit substrate. Given a list of candidate facts,
/// the heuristic ranker's choice (by index in the list), and the
/// trained selector weights, return a side-by-side comparison: which
/// candidate the selector would pick, whether that disagrees with
/// the heuristic, and the score gap between selector top-1 and the
/// heuristic's choice.
///
/// Pure function. NOT YET wired into production in v4.46.0 —
/// v4.46.5+ will route trace logs through this so disagreements
/// with the heuristic ranker become visible.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AuditResult {
    /// Index of the heuristic ranker's top choice in the candidate
    /// list. Echoed back from the input — useful for trace logs.
    pub heuristic_top_idx: usize,
    /// Index of the selector's top choice (by `score`). Stable-tie-
    /// break by first index, matching `select_top`.
    pub selector_top_idx: usize,
    /// `true` when `heuristic_top_idx != selector_top_idx`.
    pub disagreement: bool,
    /// `score(selector_top) - score(heuristic_top)`. Always `>= 0`
    /// when there's disagreement (selector picked something with a
    /// strictly higher score). Zero when there's agreement; can be
    /// zero with disagreement only when scores tie and the stable-
    /// tie-break diverges from the heuristic — which `select_top`'s
    /// "first index wins" policy makes impossible by construction
    /// when `heuristic_top_idx == 0`.
    pub score_gap: f32,
}

/// Compute the audit comparison. Returns `None` if `candidates` is
/// empty or `heuristic_top_idx` is out of bounds.
pub fn audit_compare(
    candidates: &[&ReasFact],
    heuristic_top_idx: usize,
    weights: &SelectionWeights,
    query_tokens: &[&str],
    last_topic: Option<&str>,
) -> Option<AuditResult> {
    if candidates.is_empty() || heuristic_top_idx >= candidates.len() {
        return None;
    }
    let mut best_idx = 0usize;
    let mut best_score = f32::NEG_INFINITY;
    let mut heuristic_score = f32::NEG_INFINITY;
    for (i, fact) in candidates.iter().enumerate() {
        let f = extract_features(fact, query_tokens, last_topic);
        let s = score(&f, weights);
        if i == heuristic_top_idx {
            heuristic_score = s;
        }
        if s > best_score {
            best_score = s;
            best_idx = i;
        }
    }
    Some(AuditResult {
        heuristic_top_idx,
        selector_top_idx: best_idx,
        disagreement: best_idx != heuristic_top_idx,
        score_gap: best_score - heuristic_score,
    })
}

#[cfg(test)]
mod audit_tests {
    use super::*;
    use adam_reasoning::{FactSource, Predicate, SlotRef};

    fn make_fact(
        subject: &str,
        predicate: Predicate,
        object: &str,
        raw: &str,
        confidence: ConfidenceKind,
    ) -> ReasFact {
        ReasFact {
            subject: SlotRef {
                surface: subject.to_string(),
                root: subject.to_string(),
                pos: "noun".to_string(),
            },
            predicate,
            object: SlotRef {
                surface: object.to_string(),
                root: object.to_string(),
                pos: "noun".to_string(),
            },
            pattern: "test".to_string(),
            source: FactSource {
                pack: "test".to_string(),
                sample_id: "test".to_string(),
            },
            confidence,
            raw_text: raw.to_string(),
        }
    }

    #[test]
    fn canonical_pairs_v0_is_non_empty_and_deterministic() {
        let a = canonical_training_pairs_v0();
        let b = canonical_training_pairs_v0();
        assert!(!a.is_empty());
        assert!(a.len() >= 15);
        assert_eq!(a.len(), b.len());
        // Each pair's positive must differ from negative on at least one feature.
        for pair in &a {
            assert!(
                pair.positive != pair.negative,
                "pair has identical positive and negative",
            );
        }
    }

    #[test]
    fn canonical_pairs_v0_each_pair_well_formed() {
        let pairs = canonical_training_pairs_v0();
        // Each feature value should be in [0, 1] (the v0 normalization range).
        for pair in &pairs {
            for f in [&pair.positive, &pair.negative] {
                assert!((0.0..=1.0).contains(&f.confidence));
                assert!((0.0..=1.0).contains(&f.raw_text_richness));
                assert!((0.0..=1.0).contains(&f.subject_overlap));
                assert!((0.0..=1.0).contains(&f.object_overlap));
                assert!((0.0..=1.0).contains(&f.recency_match));
            }
        }
    }

    #[test]
    fn training_on_canonical_pairs_v0_converges() {
        let pairs = canonical_training_pairs_v0();
        let initial = SelectionWeights::default_v0();
        let cfg = TrainingConfig::default_v0();
        let (trained, stats) = train_perceptron(initial, &pairs, cfg);
        assert!(
            stats.converged,
            "v0 training must converge on canonical pairs (epochs={}, violations={})",
            stats.epochs_run, stats.final_violations,
        );
        // Post-training: confidence and subject_overlap weights must
        // be positive (the dominant ranking signals).
        assert!(trained.w_confidence > 0.0);
        assert!(trained.w_subject_overlap > 0.0);
    }

    #[test]
    fn trained_weights_satisfy_every_canonical_pair() {
        let pairs = canonical_training_pairs_v0();
        let initial = SelectionWeights::default_v0();
        let cfg = TrainingConfig::default_v0();
        let (trained, _stats) = train_perceptron(initial, &pairs, cfg);
        for (i, pair) in pairs.iter().enumerate() {
            let s_pos = score(&pair.positive, &trained);
            let s_neg = score(&pair.negative, &trained);
            assert!(
                s_pos >= s_neg + cfg.margin,
                "pair {i}: positive score {s_pos} did not exceed negative {s_neg} by margin {}",
                cfg.margin,
            );
        }
    }

    #[test]
    fn audit_compare_reports_agreement_when_heuristic_matches_selector() {
        let curated = make_fact(
            "қазақстан",
            Predicate::IsA,
            "мемлекет",
            "Қазақстан — Орталық Азиядағы тәуелсіз ел.",
            ConfidenceKind::HumanApproved,
        );
        let grammar = make_fact(
            "қазақстан",
            Predicate::IsA,
            "адам",
            "",
            ConfidenceKind::Grammar,
        );
        let candidates = [&curated, &grammar];
        let weights = SelectionWeights::default_v0();
        let result =
            audit_compare(&candidates, 0, &weights, &["қазақстан"], None).expect("non-empty");
        // Heuristic picked index 0 (curated); selector also picks 0.
        assert_eq!(result.heuristic_top_idx, 0);
        assert_eq!(result.selector_top_idx, 0);
        assert!(!result.disagreement);
        assert_eq!(result.score_gap, 0.0);
    }

    #[test]
    fn audit_compare_reports_disagreement_with_score_gap() {
        // Heuristic supposedly picked the Grammar fact (index 1),
        // but the default-v0 selector should prefer the curated one.
        let curated = make_fact(
            "қазақстан",
            Predicate::IsA,
            "мемлекет",
            "Қазақстан — Орталық Азиядағы тәуелсіз ел.",
            ConfidenceKind::HumanApproved,
        );
        let grammar = make_fact(
            "қазақстан",
            Predicate::IsA,
            "адам",
            "",
            ConfidenceKind::Grammar,
        );
        let candidates = [&curated, &grammar];
        let weights = SelectionWeights::default_v0();
        let result =
            audit_compare(&candidates, 1, &weights, &["қазақстан"], None).expect("non-empty");
        assert_eq!(result.heuristic_top_idx, 1);
        assert_eq!(result.selector_top_idx, 0);
        assert!(result.disagreement);
        // score_gap = score(curated) - score(grammar) > 0.
        assert!(result.score_gap > 0.0);
    }

    #[test]
    fn audit_compare_empty_candidates_returns_none() {
        let weights = SelectionWeights::default_v0();
        let candidates: [&ReasFact; 0] = [];
        assert!(audit_compare(&candidates, 0, &weights, &[], None).is_none());
    }

    #[test]
    fn audit_compare_out_of_bounds_idx_returns_none() {
        let fact = make_fact("x", Predicate::IsA, "y", "", ConfidenceKind::HumanApproved);
        let candidates = [&fact];
        let weights = SelectionWeights::default_v0();
        // heuristic_top_idx = 5 with 1 candidate → out of bounds.
        assert!(audit_compare(&candidates, 5, &weights, &[], None).is_none());
    }

    #[test]
    fn audit_compare_single_candidate_always_agrees() {
        let fact = make_fact(
            "x",
            Predicate::IsA,
            "y",
            "Some raw text.",
            ConfidenceKind::HumanApproved,
        );
        let candidates = [&fact];
        let weights = SelectionWeights::default_v0();
        let result = audit_compare(&candidates, 0, &weights, &[], None).expect("non-empty");
        assert_eq!(result.heuristic_top_idx, 0);
        assert_eq!(result.selector_top_idx, 0);
        assert!(!result.disagreement);
        assert_eq!(result.score_gap, 0.0);
    }

    #[test]
    fn audit_compare_ties_resolve_by_first_index() {
        // Two identical-feature facts: heuristic picked index 1; the
        // selector's stable-tie-break picks index 0. So selector
        // disagrees in this edge case.
        let a = make_fact(
            "x",
            Predicate::IsA,
            "y1",
            "Same length raw.",
            ConfidenceKind::HumanApproved,
        );
        let b = make_fact(
            "x",
            Predicate::IsA,
            "y2",
            "Same length raw.",
            ConfidenceKind::HumanApproved,
        );
        let candidates = [&a, &b];
        let weights = SelectionWeights::default_v0();
        let result = audit_compare(&candidates, 1, &weights, &[], None).expect("non-empty");
        assert_eq!(result.selector_top_idx, 0);
        assert!(result.disagreement);
        // Identical features → score gap is exactly zero.
        assert!(result.score_gap.abs() < 1e-6);
    }
}
