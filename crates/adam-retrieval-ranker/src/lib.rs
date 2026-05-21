// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! # E3 — discriminative retrieval re-ranker
//!
//! Third experiment in the third-path research arc on the
//! `experimental/e3-retrieval-ranker` branch. Targets the fact-
//! selection step in the retrieval pipeline. Currently
//! implemented in
//! `crates/adam-dialog/src/selection.rs::select_top` with a
//! linear scorer and **hand-set weights**
//! (`SelectionWeights::default_v0`). E3 replaces the hand-set
//! weights with **learned** ones plus a richer feature set.
//!
//! **Hypothesis under test.** A pointwise learn-to-rank model
//! trained on `(query, fact, picked_by_cascade)` triples can
//! replace the hand-set weights at equal-or-better
//! pick-rate-at-1 vs the cascade-picked fact, ≤ 5 ms ranking
//! latency, ≤ 5 MB on disk, and **zero hallucination by
//! construction** (the model emits scores; the caller's argmax
//! over the closed candidate set picks the winning fact — no
//! novel output is generated).
//!
//! See [`docs/e3_retrieval_ranker_design.md`](../../../docs/e3_retrieval_ranker_design.md)
//! for the architecture ladder, data sourcing, evaluation
//! contract, and the binary success criteria.
//!
//! **This is a stub.** v0.0.1 ships the public types only;
//! actual scoring returns `Err(RankerError::NotLoaded)` until
//! a trained artefact is wired in.

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

/// The full feature vector the trained ranker consumes per
/// candidate. Mirrors `adam_dialog::selection::CandidateFeatures`
/// (the existing hand-set scorer's input) plus 5 additional
/// engineered features that the hand-set scorer doesn't use.
/// Field names are explicit so the trained artefact stays
/// human-readable when serialised.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateFeatures {
    /// Confidence band: 0.0 (Grammar) … 1.0 (HumanApproved).
    /// Same scale as `selection::extract_features`.
    pub confidence: f32,
    /// Raw-text length richness, normalised to [0, 1] via
    /// `min(len_chars / 100.0, 1.0)`.
    pub raw_text_richness: f32,
    /// Overlap between fact subject root and query tokens.
    /// 1.0 = exact match, 0.0 = no token overlap.
    pub subject_overlap: f32,
    /// Same for fact object root.
    pub object_overlap: f32,
    /// 1.0 iff fact's subject root == previous turn's topic.
    pub recency_match: f32,
    // ---------- additions ----------
    /// Cosine similarity between query token vector and fact
    /// `raw_text` token vector (TF-IDF weighted). 0.0 when
    /// the trainer/inference path has no IDF table available.
    pub tfidf_cosine: f32,
    /// 1.0 iff the fact's predicate matches the predicate
    /// dispatch the cascade landed on for this query
    /// (`IsA` / `Has` / `RelatedTo` / etc.); 0.0 otherwise.
    pub predicate_match: f32,
    /// IsA-graph hop distance from the query's noun_hint to
    /// the fact's subject, capped at 8 and divided by 8 so it
    /// lives in [0, 1]. 0.0 = identical, 1.0 = far / unknown.
    pub isa_distance: f32,
    /// Raw-text length in characters, divided by 200 and
    /// clamped to [0, 1]. Separate from richness because
    /// richness saturates at 100 chars; this distinguishes
    /// 120-char from 200-char facts.
    pub raw_len_norm: f32,
    /// Position of the candidate in the pre-filter list,
    /// divided by candidate-list length. 0.0 = first, near
    /// 1.0 = last. The pre-filter has its own ordering signal
    /// the ranker can choose to respect or discard.
    pub cand_pos: f32,
}

impl CandidateFeatures {
    /// Field count — useful for trainers building flat weight
    /// vectors.
    pub const N: usize = 10;

    /// Convert to a dense vector in field-declaration order.
    /// The order is the **canonical** layout for both
    /// `score()` and the trained artefact's `weights` vector.
    pub fn as_vec(&self) -> [f32; Self::N] {
        [
            self.confidence,
            self.raw_text_richness,
            self.subject_overlap,
            self.object_overlap,
            self.recency_match,
            self.tfidf_cosine,
            self.predicate_match,
            self.isa_distance,
            self.raw_len_norm,
            self.cand_pos,
        ]
    }
}

/// What can go wrong when scoring.
#[derive(Debug, Error)]
pub enum RankerError {
    /// No artefact loaded.
    #[error("ranker has no loaded model — call `Ranker::from_path` first")]
    NotLoaded,
    /// I/O failure reading the model artefact.
    #[error("model load i/o failure at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    /// Malformed artefact JSON.
    #[error("model artefact parse failure at {path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: serde_json::Error,
    },
    /// Unsupported schema version.
    #[error("unsupported model schema version: got {got}, this crate supports {supported}")]
    SchemaVersion { got: String, supported: String },
}

/// On-disk artefact. Linear pointwise model — 10 float weights
/// + bias. The artefact is ≤ 1 KB on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Artefact {
    schema_version: String,
    bias: f32,
    weights: [f32; CandidateFeatures::N],
}

const SUPPORTED_SCHEMA_VERSION: &str = "0.0.1";

/// A trained ranker ready for scoring.
#[derive(Debug, Clone, Default)]
pub struct Ranker {
    artefact: Option<Artefact>,
}

impl Ranker {
    /// Build an empty ranker — placeholder for callers
    /// compiling against the public API before any artefact
    /// exists.
    pub fn empty() -> Self {
        Self { artefact: None }
    }

    /// Load a trained ranker from a JSON artefact on disk.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, RankerError> {
        let path_str = path.as_ref().display().to_string();
        let bytes = std::fs::read(path.as_ref()).map_err(|e| RankerError::Io {
            path: path_str.clone(),
            source: e,
        })?;
        let artefact: Artefact =
            serde_json::from_slice(&bytes).map_err(|e| RankerError::Parse {
                path: path_str.clone(),
                source: e,
            })?;
        if !artefact.schema_version.starts_with("0.0.") {
            return Err(RankerError::SchemaVersion {
                got: artefact.schema_version,
                supported: SUPPORTED_SCHEMA_VERSION.to_string(),
            });
        }
        Ok(Self {
            artefact: Some(artefact),
        })
    }

    /// Score one candidate. Higher = better. Caller argmaxes
    /// over candidates to pick the top fact. Returns
    /// `NotLoaded` when no artefact is attached.
    ///
    /// **Stub** — the actual dot-product happens here once the
    /// trainer ships, but the public API is final.
    pub fn score(&self, features: &CandidateFeatures) -> Result<f32, RankerError> {
        let artefact = self.artefact.as_ref().ok_or(RankerError::NotLoaded)?;
        let v = features.as_vec();
        let mut score = artefact.bias;
        for (w, x) in artefact.weights.iter().zip(v.iter()) {
            score += w * x;
        }
        Ok(score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_features() -> CandidateFeatures {
        CandidateFeatures {
            confidence: 1.0,
            raw_text_richness: 0.6,
            subject_overlap: 1.0,
            object_overlap: 0.0,
            recency_match: 0.0,
            tfidf_cosine: 0.7,
            predicate_match: 1.0,
            isa_distance: 0.0,
            raw_len_norm: 0.5,
            cand_pos: 0.0,
        }
    }

    #[test]
    fn empty_ranker_returns_not_loaded() {
        let r = Ranker::empty();
        let err = r.score(&dummy_features()).unwrap_err();
        assert!(matches!(err, RankerError::NotLoaded));
    }

    #[test]
    fn features_vec_length_matches_const() {
        let f = dummy_features();
        assert_eq!(f.as_vec().len(), CandidateFeatures::N);
    }

    #[test]
    fn features_vec_preserves_declaration_order() {
        let f = dummy_features();
        let v = f.as_vec();
        assert_eq!(v[0], f.confidence);
        assert_eq!(v[1], f.raw_text_richness);
        assert_eq!(v[2], f.subject_overlap);
        assert_eq!(v[3], f.object_overlap);
        assert_eq!(v[4], f.recency_match);
        assert_eq!(v[5], f.tfidf_cosine);
        assert_eq!(v[6], f.predicate_match);
        assert_eq!(v[7], f.isa_distance);
        assert_eq!(v[8], f.raw_len_norm);
        assert_eq!(v[9], f.cand_pos);
    }
}
