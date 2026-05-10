//! `RootAffinity` — sparse root-pair co-occurrence prior.
//!
//! **v4.29.0 — Track A first statistical layer beyond chains.**
//! `SuffixPriors` (v4.15→v4.20) operates over **suffix chains**:
//! `P(chain | preceding chain)`, `P(root | unambiguous attribution)`.
//! That's the morphology-level signal.
//!
//! `RootAffinity` adds the **discourse-level signal**: which roots
//! tend to appear *together* in the same Kazakh sample (sentence,
//! proverb, paragraph). Useful for retrieval reranking — when two
//! candidate facts have equal chain-priority and equal overlap, the
//! one whose subject root has higher affinity to the user's
//! recent topic should win. PMI is the metric: how much more often
//! does pair `(a, b)` co-occur compared to chance, given each
//! root's marginal frequency.
//!
//! **Why PMI not raw count.** Raw co-occurrence is dominated by
//! ubiquitous roots (`мен / ол / бір`); they appear with everything
//! and produce no useful signal. PMI normalizes by marginals:
//! `PMI(a, b) = log(P(a, b) / (P(a) · P(b)))`. High PMI = pair
//! co-occurs much more than chance. Low PMI = pair appears together
//! only because both are common. We filter by `min_pair_count` to
//! suppress noise (a pair seen 1-2 times has unreliable PMI).
//!
//! **Sparse storage.** A full `9602 × 9601 / 2 ≈ 46M` cell matrix
//! is intractable as JSON. We keep only pairs above the `min_pair_count`
//! threshold *and* with positive PMI — typically 50k-500k cells
//! depending on threshold. Lookup at runtime: outer
//! `HashMap<root_a, HashMap<root_b, f32>>` indexed by
//! lex-sorted-smaller root first, so callers normalize the pair
//! before lookup.
//!
//! **Zero ML at runtime.** Pure frequency count, no embeddings, no
//! gradients. Same paradigm as `SuffixPriors`: trained offline in
//! one pass, frozen artifact at runtime, single hashmap probe per
//! query. Stays interpretable: each PMI value is `log_2` ratio of
//! observed-to-expected co-occurrence — auditable, reproducible.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Format-versioned wrapper around the root-pair co-occurrence
/// matrix. Same schema-bump policy as `SuffixPriors`: `load()`
/// rejects mismatched `version` fields so callers fail fast.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RootAffinity {
    /// Format version. Incremented when the on-disk schema changes
    /// in a backward-incompatible way. v4.29.0 = 1.
    pub version: u32,
    /// Total sample count over which co-occurrence was tallied.
    /// Useful as denominator for marginal-probability calculations.
    #[serde(default)]
    pub trained_on_samples: u64,
    /// Minimum pair count threshold used during training. Pairs
    /// observed fewer times were filtered out as too-noisy. Recorded
    /// here so loading callers can verify consistency with their
    /// expectation.
    #[serde(default)]
    pub min_pair_count: u32,
    /// Marginal log-probability per root: `log(count(root) / N)` where
    /// N is `trained_on_samples`. Empty when no training has been
    /// performed.
    #[serde(default)]
    pub root_log_prob: HashMap<String, f32>,
    /// Sparse PMI scores: `outer_root → inner_root → PMI`. Pairs are
    /// stored with `outer_root` lex-smaller than `inner_root` to
    /// avoid double-counting; callers must normalize their query
    /// pair before lookup. Stored in natural log units (matches
    /// `root_log_prob`).
    #[serde(default)]
    pub pair_pmi: HashMap<String, HashMap<String, f32>>,
}

impl Default for RootAffinity {
    fn default() -> Self {
        Self::empty()
    }
}

impl RootAffinity {
    /// Empty affinity prior — `score` always returns `0.0` (additive
    /// identity for callers combining with chain scores). Used by
    /// tests and as the no-op fallback when no priors file is
    /// bundled.
    pub fn empty() -> Self {
        Self {
            version: SCHEMA_VERSION,
            trained_on_samples: 0,
            min_pair_count: 0,
            root_log_prob: HashMap::new(),
            pair_pmi: HashMap::new(),
        }
    }

    /// Load a JSON affinity file from disk. Mismatched versions
    /// return a dedicated error so callers can fail fast instead of
    /// silently using a stale prior.
    ///
    /// **v5.6.0** — switched from `fs::read` + `from_slice` (which
    /// holds the entire 27 MB file as `Vec<u8>` alongside the parsed
    /// struct at peak) to `BufReader::new(File::open)` +
    /// `from_reader`. Cuts peak RSS during load by ~27 MB.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, RootAffinityLoadError> {
        let file = std::fs::File::open(path.as_ref()).map_err(RootAffinityLoadError::Io)?;
        let reader = std::io::BufReader::new(file);
        let priors: Self = serde_json::from_reader(reader).map_err(RootAffinityLoadError::Json)?;
        if priors.version != SCHEMA_VERSION {
            return Err(RootAffinityLoadError::VersionMismatch {
                expected: SCHEMA_VERSION,
                got: priors.version,
            });
        }
        Ok(priors)
    }

    /// Build affinity from raw counts: total samples, per-root single
    /// counts, per-pair joint counts. Pair counts must already use
    /// lex-sorted-smaller-first key order. Pairs below `min_pair_count`
    /// are filtered out; PMI is computed for the rest.
    pub fn from_counts(
        total_samples: u64,
        single_counts: HashMap<String, u64>,
        pair_counts: HashMap<(String, String), u64>,
        min_pair_count: u32,
    ) -> Self {
        let n = total_samples as f64;
        let mut root_log_prob = HashMap::with_capacity(single_counts.len());
        for (root, count) in &single_counts {
            let p = (*count as f64) / n;
            root_log_prob.insert(root.clone(), p.ln() as f32);
        }
        let mut pair_pmi: HashMap<String, HashMap<String, f32>> = HashMap::new();
        for ((a, b), count) in pair_counts {
            if count < min_pair_count as u64 {
                continue;
            }
            let p_ab = (count as f64) / n;
            let p_a = (*single_counts.get(&a).unwrap_or(&0) as f64) / n;
            let p_b = (*single_counts.get(&b).unwrap_or(&0) as f64) / n;
            if p_a <= 0.0 || p_b <= 0.0 {
                continue;
            }
            let pmi = (p_ab / (p_a * p_b)).ln() as f32;
            // Filter to positive-PMI pairs only — negative PMI means
            // the pair co-occurs *less* than chance, which is also
            // signal but adds storage cost without clear ranking
            // value. Threshold of 0.0 keeps roughly half the pairs
            // (those above chance) and discards the rest.
            if pmi <= 0.0 {
                continue;
            }
            pair_pmi.entry(a).or_default().insert(b, pmi);
        }
        Self {
            version: SCHEMA_VERSION,
            trained_on_samples: total_samples,
            min_pair_count,
            root_log_prob,
            pair_pmi,
        }
    }

    /// Score affinity between two roots. Returns the PMI value (in
    /// natural log units) when the pair is in the support, or `0.0`
    /// otherwise (additive identity — callers combining with other
    /// scores see no contribution). Order-insensitive: callers can
    /// pass `(a, b)` or `(b, a)`; internally normalized to lex order.
    pub fn score(&self, root_a: &str, root_b: &str) -> f32 {
        if root_a == root_b {
            return 0.0;
        }
        let (outer, inner) = if root_a < root_b {
            (root_a, root_b)
        } else {
            (root_b, root_a)
        };
        self.pair_pmi
            .get(outer)
            .and_then(|row| row.get(inner).copied())
            .unwrap_or(0.0)
    }

    /// Marginal log-probability of a root. Returns `f32::NEG_INFINITY`
    /// when the root isn't in the support — pushes unseen roots
    /// strictly below any observed one. Empty prior returns `0.0`
    /// so callers see no contribution in the no-data case.
    pub fn root_log_p(&self, root: &str) -> f32 {
        if self.root_log_prob.is_empty() {
            return 0.0;
        }
        self.root_log_prob
            .get(root)
            .copied()
            .unwrap_or(f32::NEG_INFINITY)
    }

    /// Number of root pairs in the support. Diagnostic only.
    pub fn pair_count(&self) -> usize {
        self.pair_pmi.values().map(|row| row.len()).sum()
    }

    /// Number of distinct roots with non-zero marginal. Diagnostic.
    pub fn root_count(&self) -> usize {
        self.root_log_prob.len()
    }

    /// `true` when no training has been performed.
    pub fn is_empty(&self) -> bool {
        self.pair_pmi.is_empty() && self.root_log_prob.is_empty()
    }
}

/// Current on-disk schema version. v1 (v4.29.0) ships with
/// `root_log_prob` + `pair_pmi`. Future bumps would be: v2 = add
/// per-domain affinity slices, v3 = context-aware (within-domain
/// vs cross-domain), etc.
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum RootAffinityLoadError {
    #[error("io error reading root affinity: {0}")]
    Io(#[from] std::io::Error),
    #[error("json parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("schema version mismatch: expected {expected}, got {got}")]
    VersionMismatch { expected: u32, got: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_affinity_returns_zero_score() {
        let aff = RootAffinity::empty();
        assert_eq!(aff.score("ат", "тау"), 0.0);
        assert_eq!(aff.score("anything", "anything"), 0.0);
    }

    #[test]
    fn from_counts_filters_below_min_count() {
        let mut singles: HashMap<String, u64> = HashMap::new();
        singles.insert("a".into(), 100);
        singles.insert("b".into(), 100);
        singles.insert("c".into(), 100);
        let mut pairs: HashMap<(String, String), u64> = HashMap::new();
        // (a, b) appears 50 times — strong co-occurrence
        pairs.insert(("a".into(), "b".into()), 50);
        // (a, c) appears 1 time — below threshold, should be filtered
        pairs.insert(("a".into(), "c".into()), 1);
        let aff = RootAffinity::from_counts(1000, singles, pairs, 3);
        // (a, b): present, positive PMI (50 / (100/1000 * 100/1000 * 1000) = 50/10 = 5×)
        assert!(aff.score("a", "b") > 0.0);
        // (a, c): filtered by min_pair_count
        assert_eq!(aff.score("a", "c"), 0.0);
    }

    #[test]
    fn score_is_order_insensitive() {
        let mut singles: HashMap<String, u64> = HashMap::new();
        singles.insert("alpha".into(), 50);
        singles.insert("beta".into(), 50);
        let mut pairs: HashMap<(String, String), u64> = HashMap::new();
        pairs.insert(("alpha".into(), "beta".into()), 25);
        let aff = RootAffinity::from_counts(500, singles, pairs, 3);
        assert_eq!(aff.score("alpha", "beta"), aff.score("beta", "alpha"));
    }

    #[test]
    fn version_mismatch_returns_dedicated_error() {
        let bad = r#"{"version":999,"trained_on_samples":0,"min_pair_count":0,"root_log_prob":{},"pair_pmi":{}}"#;
        let path = std::env::temp_dir().join("adam_root_affinity_bad.json");
        std::fs::write(&path, bad).unwrap();
        match RootAffinity::load(&path) {
            Err(RootAffinityLoadError::VersionMismatch { expected, got }) => {
                assert_eq!(expected, SCHEMA_VERSION);
                assert_eq!(got, 999);
            }
            other => panic!("expected VersionMismatch, got {other:?}"),
        }
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn schema_version_is_one_at_v4_29_0() {
        // v4.29.0 introduces the artifact at schema v1.
        assert_eq!(SCHEMA_VERSION, 1);
    }
}
