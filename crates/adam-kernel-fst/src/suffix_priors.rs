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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SuffixPriors {
    /// Format version. Incremented when the on-disk schema changes
    /// in a backward-incompatible way. v4.15.0 ships with `1`.
    pub version: u32,
    /// Total token count over which the priors were estimated.
    /// Useful as a sanity check + for future smoothing.
    #[serde(default)]
    pub trained_on_tokens: u64,
    /// chain signature → natural-log probability under uniform-
    /// attribution counting. Empty when no training has been
    /// performed yet (the `empty()` constructor).
    pub chain_log_prob: HashMap<String, f32>,
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
        }
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
pub const SCHEMA_VERSION: u32 = 1;

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
    fn schema_version_is_one_at_v4_15_0() {
        // v4.15.0 ships SCHEMA_VERSION = 1. Bumping this constant
        // requires regenerating the artifact and bumping the
        // `data/retrieval/suffix_chain_priors.json` accordingly.
        assert_eq!(SCHEMA_VERSION, 1);
    }
}
