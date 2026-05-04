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
