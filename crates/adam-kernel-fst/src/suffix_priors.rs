//! `SuffixPriors` — frequency-based prior over suffix-chain
//! signatures.
//!
//! **v4.15.0 — first compositional ML layer.** The Kazakh
//! agglutinative grammar IS typed function composition: every word
//! is `root + suffix_1 + suffix_2 + ... + suffix_n`, and each suffix
//! is a typed function that transforms the previous state. Pre-
//! v4.15.0 the FST analyser returned candidate parses in a
//! lexicographic deterministic order ([`crate::parser::analyse`]
//! v3.2.0 contract), but had no notion of which suffix-chains are
//! actually *common* in Kazakh usage.
//!
//! `SuffixPriors` adds the missing distributional signal: a frozen
//! lookup table from suffix-chain signature (e.g. `noun:None|Sg|None|Locative|None`)
//! to its log-probability under the committed corpus. The table is
//! built **offline** by `crates/adam-corpus/src/bin/train_suffix_priors.rs`
//! and shipped as `data/retrieval/suffix_chain_priors.json`. Runtime
//! cost is **zero** during training — at inference, a single hashmap
//! lookup per parse.
//!
//! **Why this matters.** When the FST returns multiple candidate
//! parses for an ambiguous surface, the v3.2.0 deterministic order
//! picks one by `(root, id)` lexicographic sort. That's
//! reproducible but not semantically informed: parses with very rare
//! suffix-chains (e.g. analysing a closed-class adverb as a
//! locative-cased noun) can win over the actually-common reading.
//! `SuffixPriors` is the substrate v4.15.5+ runtime integration uses
//! to break ambiguity on the side of the more probable chain.
//!
//! **Zero ML at runtime.** The training step is a pure frequency
//! count; no embeddings, no gradient, no back-propagation. The
//! frozen artifact is a `HashMap<String, f32>`. This fits the
//! agglutinative-first directive: `корень + функция^n` — each
//! lookup is a single hashmap probe, fits a CPU register, fully
//! inspectable.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::morphotactics::{NounFeatures, VerbFeatures};

/// Format-versioned wrapper around the chain → log-prob map.
/// Wrapping in a struct lets future versions add fields (e.g.
/// per-POS marginals, smoothing parameters) without breaking the
/// JSON contract.
///
/// **v4.16.0** — extended with `transition_log_prob` for context-
/// aware bigram scoring (`P(chain | preceding_chain)`). Schema
/// version bumped from 1 to 2; v1 artifacts are explicitly
/// rejected by `load()` so callers fail fast and regenerate via
/// the training binary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SuffixPriors {
    /// Format version. Incremented when the on-disk schema changes
    /// in a backward-incompatible way. v4.15.0 = 1; v4.16.0 = 2
    /// (added `transition_log_prob`).
    pub version: u32,
    /// Total token count over which the priors were estimated.
    /// Useful as a sanity check + for future smoothing.
    #[serde(default)]
    pub trained_on_tokens: u64,
    /// chain signature → natural-log probability under uniform-
    /// attribution counting. Empty when no training has been
    /// performed yet (the `empty()` constructor).
    pub chain_log_prob: HashMap<String, f32>,
    /// **v4.16.0** — bigram transition log-probabilities:
    /// `previous_chain → current_chain → log P(curr | prev)`.
    /// Captures local morphological agreement (e.g. Genitive
    /// followed by 3sg-Possessive — «жасушаның ядросы» — is much
    /// more probable than Genitive followed by Imperative). Empty
    /// when running v4.15.0-style unigram-only priors;
    /// `serde(default)` makes deserialisation tolerate missing
    /// field, but `load()` enforces `version >= 2` so practical
    /// callers regenerate the artifact via the training binary.
    #[serde(default)]
    pub transition_log_prob: HashMap<String, HashMap<String, f32>>,
}

impl Default for SuffixPriors {
    fn default() -> Self {
        Self::empty()
    }
}

impl SuffixPriors {
    /// Empty prior — `score_*` always returns the default
    /// log-probability for an unseen chain. Used by tests and as
    /// the no-op fallback when no priors file is bundled.
    pub fn empty() -> Self {
        Self {
            version: SCHEMA_VERSION,
            trained_on_tokens: 0,
            chain_log_prob: HashMap::new(),
            transition_log_prob: HashMap::new(),
        }
    }

    /// Load a JSON priors file from disk. The file must match the
    /// current `SCHEMA_VERSION`; mismatched versions return a
    /// dedicated error so callers can fail fast instead of silently
    /// using a stale prior.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, SuffixPriorsLoadError> {
        let bytes = std::fs::read(path.as_ref()).map_err(SuffixPriorsLoadError::Io)?;
        let priors: Self = serde_json::from_slice(&bytes).map_err(SuffixPriorsLoadError::Json)?;
        if priors.version != SCHEMA_VERSION {
            return Err(SuffixPriorsLoadError::VersionMismatch {
                expected: SCHEMA_VERSION,
                got: priors.version,
            });
        }
        Ok(priors)
    }

    /// Build a prior from raw chain counts. Computes natural-log
    /// probabilities with simple add-one (Laplace) smoothing so
    /// unseen chains get a non-zero floor.
    pub fn from_counts(counts: HashMap<String, u64>) -> Self {
        let total: u64 = counts.values().sum();
        // Add-one smoothing across the observed support set. Unseen
        // chains fall to the per-vocabulary floor handled in
        // `unseen_log_prob`; we don't pre-allocate slots for chains
        // we never observed (keeps the file size bounded by the
        // observed support, not the cartesian product).
        let vocab = counts.len() as u64;
        let denom = (total + vocab) as f32;
        let mut chain_log_prob = HashMap::with_capacity(counts.len());
        for (chain, count) in counts {
            let smoothed = (count + 1) as f32 / denom;
            chain_log_prob.insert(chain, smoothed.ln());
        }
        Self {
            version: SCHEMA_VERSION,
            trained_on_tokens: total,
            chain_log_prob,
            transition_log_prob: HashMap::new(),
        }
    }

    /// **v4.16.0** — build a prior with both unigram counts AND
    /// bigram transition counts. The transition map is keyed by
    /// (`previous_chain`, `current_chain`) — for each previous
    /// chain, the inner map gives the conditional log-probability
    /// `log P(curr | prev)` with add-one smoothing applied within
    /// the row (so unseen `curr` for a known `prev` gets a small
    /// floor relative to the row's vocabulary).
    pub fn from_counts_with_bigrams(
        unigram_counts: HashMap<String, u64>,
        bigram_counts: HashMap<(String, String), u64>,
    ) -> Self {
        let mut priors = Self::from_counts(unigram_counts);

        // Group bigram counts by previous chain.
        let mut by_prev: HashMap<String, HashMap<String, u64>> = HashMap::new();
        for ((prev, curr), count) in bigram_counts {
            by_prev.entry(prev).or_default().insert(curr, count);
        }

        let mut transition_log_prob: HashMap<String, HashMap<String, f32>> =
            HashMap::with_capacity(by_prev.len());
        for (prev, row) in by_prev {
            let row_total: u64 = row.values().sum();
            let row_vocab = row.len() as u64;
            let denom = (row_total + row_vocab) as f32;
            let mut inner = HashMap::with_capacity(row.len());
            for (curr, count) in row {
                let smoothed = (count + 1) as f32 / denom;
                inner.insert(curr, smoothed.ln());
            }
            transition_log_prob.insert(prev, inner);
        }
        priors.transition_log_prob = transition_log_prob;
        priors
    }

    /// Number of distinct chains in the support. Diagnostic only.
    pub fn len(&self) -> usize {
        self.chain_log_prob.len()
    }

    /// `true` when no training has been performed.
    pub fn is_empty(&self) -> bool {
        self.chain_log_prob.is_empty()
    }

    /// Score a noun feature bundle. Returns the log-probability of
    /// the chain under the trained prior. Unseen chains return
    /// `unseen_log_prob` — a floor that's lower than the rarest
    /// observed chain so unseen parses can still be ranked behind
    /// every observed one.
    pub fn score_noun(&self, features: &NounFeatures) -> f32 {
        let chain = noun_chain_key(features);
        self.chain_log_prob
            .get(&chain)
            .copied()
            .unwrap_or_else(|| self.unseen_log_prob())
    }

    /// Score a verb feature bundle.
    pub fn score_verb(&self, features: &VerbFeatures) -> f32 {
        let chain = verb_chain_key(features);
        self.chain_log_prob
            .get(&chain)
            .copied()
            .unwrap_or_else(|| self.unseen_log_prob())
    }

    /// **v4.16.0** — context-aware score: log-probability of the
    /// current chain given the previous token's chain.
    ///
    /// When `prev_chain` is `Some`, returns
    /// `log P(curr | prev) = transition_log_prob[prev][curr]`.
    /// Falls back to the unigram score when:
    /// - `prev_chain` is `None` (sentence start / no context)
    /// - the prev chain isn't in `transition_log_prob` (unseen
    ///   context — runtime gracefully degrades to unigram)
    /// - the (prev, curr) pair wasn't observed (Laplace-smoothed
    ///   floor for the row, falling back to row-unseen if the
    ///   row exists)
    pub fn score_chain_given_prev(&self, current_chain: &str, prev_chain: Option<&str>) -> f32 {
        if let Some(prev) = prev_chain {
            if let Some(row) = self.transition_log_prob.get(prev) {
                if let Some(score) = row.get(current_chain).copied() {
                    return score;
                }
                // prev is known, curr isn't seen in this row →
                // floor to the row's rarest observation minus
                // ln(2). Same shape as unigram unseen handling.
                let min_in_row = row.values().copied().fold(f32::INFINITY, f32::min);
                if min_in_row.is_finite() {
                    return min_in_row - std::f32::consts::LN_2;
                }
            }
        }
        // No prev context, or prev isn't in transition map →
        // fall back to unigram.
        self.chain_log_prob
            .get(current_chain)
            .copied()
            .unwrap_or_else(|| self.unseen_log_prob())
    }

    /// **v4.16.0** — convenience: score a NounFeatures bundle in
    /// the bigram-aware mode.
    pub fn score_noun_given_prev(&self, features: &NounFeatures, prev_chain: Option<&str>) -> f32 {
        let chain = noun_chain_key(features);
        self.score_chain_given_prev(&chain, prev_chain)
    }

    /// **v4.16.0** — convenience: score a VerbFeatures bundle in
    /// the bigram-aware mode.
    pub fn score_verb_given_prev(&self, features: &VerbFeatures, prev_chain: Option<&str>) -> f32 {
        let chain = verb_chain_key(features);
        self.score_chain_given_prev(&chain, prev_chain)
    }

    /// **v4.16.5** — Jelinek-Mercer interpolation between unigram
    /// and bigram log-probabilities.
    ///
    /// Returns `α · log P(curr) + (1-α) · log P(curr | prev)`.
    /// Equivalent to `P(curr)^α · P(curr|prev)^(1-α)` on the
    /// probability scale — a classic smoothing recipe that lets
    /// callers tune how aggressively bigrams influence parse
    /// selection:
    ///
    /// - `α = 0.0` — pure bigram (same as `score_chain_given_prev`).
    /// - `α = 1.0` — pure unigram (same as `score_chain`).
    /// - `α ≈ 0.3` — bigram dominates but unigram smooths out
    ///    sparse rows; the recommended default.
    ///
    /// `α` is clamped to `[0.0, 1.0]`; out-of-range values are
    /// silently bounded so callers can't accidentally negate the
    /// interpolation.
    pub fn score_chain_smoothed(
        &self,
        current_chain: &str,
        prev_chain: Option<&str>,
        alpha: f32,
    ) -> f32 {
        let alpha = alpha.clamp(0.0, 1.0);
        let unigram = self
            .chain_log_prob
            .get(current_chain)
            .copied()
            .unwrap_or_else(|| self.unseen_log_prob());
        // Skip the bigram lookup when α is fully unigram — saves
        // one hashmap probe per parse on the hot path.
        if alpha >= 0.999_99 {
            return unigram;
        }
        let bigram = self.score_chain_given_prev(current_chain, prev_chain);
        // Symmetric early-out for α≈0 (pure bigram path).
        if alpha <= 1e-5 {
            return bigram;
        }
        alpha * unigram + (1.0 - alpha) * bigram
    }

    /// **v4.16.5** — convenience: smoothed score for a noun bundle.
    pub fn score_noun_smoothed(
        &self,
        features: &NounFeatures,
        prev_chain: Option<&str>,
        alpha: f32,
    ) -> f32 {
        let chain = noun_chain_key(features);
        self.score_chain_smoothed(&chain, prev_chain, alpha)
    }

    /// **v4.16.5** — convenience: smoothed score for a verb bundle.
    pub fn score_verb_smoothed(
        &self,
        features: &VerbFeatures,
        prev_chain: Option<&str>,
        alpha: f32,
    ) -> f32 {
        let chain = verb_chain_key(features);
        self.score_chain_smoothed(&chain, prev_chain, alpha)
    }

    /// Floor for unseen chains. Computed as `min(observed) - ln(2)`
    /// so unseen chains rank strictly below the rarest observed
    /// one. Empty prior returns `f32::NEG_INFINITY` so it's always
    /// strictly worse than any seen chain in any other prior.
    fn unseen_log_prob(&self) -> f32 {
        if self.chain_log_prob.is_empty() {
            return f32::NEG_INFINITY;
        }
        // f32::min isn't `Ord` — fold over the values.
        let min_observed = self
            .chain_log_prob
            .values()
            .copied()
            .fold(f32::INFINITY, f32::min);
        // ln(2) ≈ 0.693; pushes unseen ~half the rarest observed.
        min_observed - std::f32::consts::LN_2
    }
}

/// Current on-disk schema version. Bump when the format changes
/// in a way that requires regenerating the artifact.
///
/// - v1 (v4.15.0): unigram only, `chain_log_prob`.
/// - v2 (v4.16.0): adds `transition_log_prob` for context-aware
///   bigram scoring.
pub const SCHEMA_VERSION: u32 = 2;

/// Build the chain key for a noun feature bundle. Stable string
/// format so the JSON artifact is self-describing.
pub fn noun_chain_key(features: &NounFeatures) -> String {
    format!(
        "noun:{}|{}|{}|{}|{}",
        opt_debug(features.derivation),
        opt_debug(features.number),
        opt_debug(features.possessive),
        opt_debug(features.case),
        opt_debug(features.predicate),
    )
}

/// Build the chain key for a verb feature bundle.
pub fn verb_chain_key(features: &VerbFeatures) -> String {
    format!(
        "verb:{}|{}|{}|{}|{}|{}",
        opt_debug(features.voice),
        features.negation,
        opt_debug(features.tense),
        opt_debug(features.person),
        opt_debug(features.number),
        features.polite,
    )
}

fn opt_debug<T: std::fmt::Debug>(value: Option<T>) -> String {
    match value {
        Some(v) => format!("{v:?}"),
        None => "None".to_string(),
    }
}

#[derive(Debug)]
pub enum SuffixPriorsLoadError {
    Io(std::io::Error),
    Json(serde_json::Error),
    VersionMismatch { expected: u32, got: u32 },
}

impl std::fmt::Display for SuffixPriorsLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io error reading suffix priors: {e}"),
            Self::Json(e) => write!(f, "json parse error: {e}"),
            Self::VersionMismatch { expected, got } => write!(
                f,
                "suffix priors version mismatch: expected {expected}, got {got} \
                 (regenerate via `cargo run --bin train_suffix_priors`)",
            ),
        }
    }
}

impl std::error::Error for SuffixPriorsLoadError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::morphotactics::{Case, Number, Tense};

    #[test]
    fn empty_prior_returns_neg_infinity_for_any_chain() {
        let p = SuffixPriors::empty();
        let nf = NounFeatures::default();
        let s = p.score_noun(&nf);
        assert!(s.is_infinite() && s.is_sign_negative());
    }

    #[test]
    fn from_counts_assigns_higher_log_prob_to_more_frequent_chains() {
        let mut counts = HashMap::new();
        counts.insert("noun:None|Singular|None|Nominative|None".to_string(), 100);
        counts.insert("noun:None|Singular|None|Locative|None".to_string(), 5);
        let p = SuffixPriors::from_counts(counts);
        let nominative = p
            .chain_log_prob
            .get("noun:None|Singular|None|Nominative|None")
            .copied()
            .unwrap();
        let locative = p
            .chain_log_prob
            .get("noun:None|Singular|None|Locative|None")
            .copied()
            .unwrap();
        assert!(
            nominative > locative,
            "nominative ({nominative}) should outrank locative ({locative})"
        );
    }

    #[test]
    fn unseen_chain_falls_below_rarest_observed() {
        let mut counts = HashMap::new();
        counts.insert("noun:None|Singular|None|Nominative|None".to_string(), 100);
        counts.insert("noun:None|Singular|None|Locative|None".to_string(), 5);
        let p = SuffixPriors::from_counts(counts);
        let mut nf_unseen = NounFeatures::default();
        nf_unseen.case = Some(Case::Instrumental);
        let unseen_score = p.score_noun(&nf_unseen);
        let locative_score = *p
            .chain_log_prob
            .get("noun:None|Singular|None|Locative|None")
            .unwrap();
        assert!(
            unseen_score < locative_score,
            "unseen chain ({unseen_score}) must rank below rarest observed ({locative_score})"
        );
    }

    #[test]
    fn noun_chain_key_is_deterministic() {
        let mut nf = NounFeatures::default();
        nf.number = Some(Number::Singular);
        nf.case = Some(Case::Locative);
        let k1 = noun_chain_key(&nf);
        let k2 = noun_chain_key(&nf);
        assert_eq!(k1, k2);
        assert!(k1.starts_with("noun:"));
        assert!(k1.contains("Singular"));
        assert!(k1.contains("Locative"));
    }

    #[test]
    fn verb_chain_key_includes_negation_and_polite_flags() {
        let mut vf = VerbFeatures::default();
        vf.tense = Some(Tense::Present);
        vf.negation = true;
        vf.polite = false;
        let key = verb_chain_key(&vf);
        assert!(key.starts_with("verb:"));
        assert!(key.contains("Present"));
        assert!(key.contains("true"));
        assert!(key.ends_with("false"));
    }

    #[test]
    fn round_trip_via_json_preserves_priors() {
        let mut counts = HashMap::new();
        counts.insert("noun:None|Singular|None|Nominative|None".to_string(), 42);
        counts.insert(
            "verb:None|false|Some(Present)|Some(Third)|None|false".to_string(),
            17,
        );
        let original = SuffixPriors::from_counts(counts);
        let json = serde_json::to_string(&original).expect("serialize");
        let recovered: SuffixPriors = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, recovered);
        assert_eq!(recovered.trained_on_tokens, 59);
    }

    #[test]
    fn version_mismatch_returns_dedicated_error() {
        let bad = r#"{"version":999,"trained_on_tokens":0,"chain_log_prob":{}}"#;
        let path = std::env::temp_dir().join("adam_suffix_priors_bad.json");
        std::fs::write(&path, bad).unwrap();
        match SuffixPriors::load(&path) {
            Err(SuffixPriorsLoadError::VersionMismatch { expected, got }) => {
                assert_eq!(expected, SCHEMA_VERSION);
                assert_eq!(got, 999);
            }
            other => panic!("expected VersionMismatch, got {other:?}"),
        }
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn schema_version_is_two_at_v4_16_0() {
        // v4.16.0 bumps SCHEMA_VERSION from 1 to 2 to add
        // `transition_log_prob`. Any future bump requires
        // regenerating `data/retrieval/suffix_chain_priors.json`
        // via the training binary.
        assert_eq!(SCHEMA_VERSION, 2);
    }

    #[test]
    fn from_counts_with_bigrams_populates_transition_log_prob() {
        let mut unigrams: HashMap<String, u64> = HashMap::new();
        unigrams.insert("noun:None|None|None|Genitive|None".to_string(), 50);
        unigrams.insert("noun:None|None|Some(P3)|None|None".to_string(), 100);
        unigrams.insert("noun:None|Singular|None|Nominative|None".to_string(), 200);

        let mut bigrams: HashMap<(String, String), u64> = HashMap::new();
        // «жасушаның ядросы»: Genitive → 3sg-Possessive — common pattern
        bigrams.insert(
            (
                "noun:None|None|None|Genitive|None".to_string(),
                "noun:None|None|Some(P3)|None|None".to_string(),
            ),
            40,
        );
        // Genitive → bare Nominative — rare in this dataset
        bigrams.insert(
            (
                "noun:None|None|None|Genitive|None".to_string(),
                "noun:None|Singular|None|Nominative|None".to_string(),
            ),
            5,
        );

        let p = SuffixPriors::from_counts_with_bigrams(unigrams, bigrams);
        assert!(!p.transition_log_prob.is_empty());

        let with_3sg = p.score_chain_given_prev(
            "noun:None|None|Some(P3)|None|None",
            Some("noun:None|None|None|Genitive|None"),
        );
        let with_nom = p.score_chain_given_prev(
            "noun:None|Singular|None|Nominative|None",
            Some("noun:None|None|None|Genitive|None"),
        );
        assert!(
            with_3sg > with_nom,
            "P(3sg-Poss | Genitive) ({with_3sg}) should beat \
             P(Nominative | Genitive) ({with_nom})"
        );
    }

    #[test]
    fn score_chain_given_prev_falls_back_to_unigram_when_no_context() {
        let mut unigrams: HashMap<String, u64> = HashMap::new();
        unigrams.insert("noun:None|Singular|None|Nominative|None".to_string(), 100);
        let p = SuffixPriors::from_counts(unigrams);
        // No prev context AND no transition map at all → unigram path.
        let s = p.score_chain_given_prev("noun:None|Singular|None|Nominative|None", None);
        let unigram = p
            .chain_log_prob
            .get("noun:None|Singular|None|Nominative|None")
            .copied()
            .unwrap();
        assert!(
            (s - unigram).abs() < 1e-6,
            "with prev=None, score should equal unigram (got {s}, want {unigram})"
        );
    }

    #[test]
    fn score_chain_smoothed_alpha_zero_equals_bigram() {
        let mut unigrams: HashMap<String, u64> = HashMap::new();
        unigrams.insert("noun:None|None|None|Genitive|None".to_string(), 50);
        unigrams.insert("noun:None|None|Some(P3)|None|None".to_string(), 100);
        let mut bigrams: HashMap<(String, String), u64> = HashMap::new();
        bigrams.insert(
            (
                "noun:None|None|None|Genitive|None".to_string(),
                "noun:None|None|Some(P3)|None|None".to_string(),
            ),
            40,
        );
        let p = SuffixPriors::from_counts_with_bigrams(unigrams, bigrams);
        let pure_bigram = p.score_chain_given_prev(
            "noun:None|None|Some(P3)|None|None",
            Some("noun:None|None|None|Genitive|None"),
        );
        let alpha_zero = p.score_chain_smoothed(
            "noun:None|None|Some(P3)|None|None",
            Some("noun:None|None|None|Genitive|None"),
            0.0,
        );
        assert!(
            (pure_bigram - alpha_zero).abs() < 1e-5,
            "α=0 must equal pure bigram (got {alpha_zero}, want {pure_bigram})"
        );
    }

    #[test]
    fn score_chain_smoothed_alpha_one_equals_unigram() {
        let mut unigrams: HashMap<String, u64> = HashMap::new();
        unigrams.insert("noun:None|Singular|None|Nominative|None".to_string(), 100);
        let p = SuffixPriors::from_counts(unigrams);
        let pure_unigram = *p
            .chain_log_prob
            .get("noun:None|Singular|None|Nominative|None")
            .unwrap();
        let alpha_one =
            p.score_chain_smoothed("noun:None|Singular|None|Nominative|None", None, 1.0);
        assert!(
            (pure_unigram - alpha_one).abs() < 1e-5,
            "α=1 must equal pure unigram (got {alpha_one}, want {pure_unigram})"
        );
    }

    #[test]
    fn score_chain_smoothed_clamps_alpha_to_unit_interval() {
        let mut unigrams: HashMap<String, u64> = HashMap::new();
        unigrams.insert("noun:None|Singular|None|Nominative|None".to_string(), 100);
        let p = SuffixPriors::from_counts(unigrams);
        let neg = p.score_chain_smoothed("noun:None|Singular|None|Nominative|None", None, -0.5);
        let zero = p.score_chain_smoothed("noun:None|Singular|None|Nominative|None", None, 0.0);
        let plus_two = p.score_chain_smoothed("noun:None|Singular|None|Nominative|None", None, 2.0);
        let one = p.score_chain_smoothed("noun:None|Singular|None|Nominative|None", None, 1.0);
        assert!((neg - zero).abs() < 1e-5, "α<0 must clamp to 0");
        assert!((plus_two - one).abs() < 1e-5, "α>1 must clamp to 1");
    }

    #[test]
    fn score_chain_given_prev_falls_back_when_prev_unseen() {
        let mut unigrams: HashMap<String, u64> = HashMap::new();
        unigrams.insert("noun:None|Singular|None|Nominative|None".to_string(), 100);
        let bigrams: HashMap<(String, String), u64> = HashMap::new();
        let p = SuffixPriors::from_counts_with_bigrams(unigrams, bigrams);
        // prev "x" is unseen in transition map → unigram fallback.
        let s = p.score_chain_given_prev(
            "noun:None|Singular|None|Nominative|None",
            Some("noun:UnseenChain"),
        );
        let unigram = p
            .chain_log_prob
            .get("noun:None|Singular|None|Nominative|None")
            .copied()
            .unwrap();
        assert!(
            (s - unigram).abs() < 1e-6,
            "unseen prev should fall back to unigram (got {s}, want {unigram})"
        );
    }
}
