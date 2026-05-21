// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! # E2 — discriminative slot extractor (NER)
//!
//! Second experiment in the third-path research arc on the
//! `experimental/e2-slot-extractor` branch. Targets the next
//! biggest bundle of hand-written cascade code after E1 closed
//! intent classification: the `detect_statement_of_*` functions
//! in [`adam_dialog::semantics`] for the slots `name`, `age`,
//! `city`, `occupation`, and `family`.
//!
//! **Hypothesis under test.** A small (~ 100 K – 1 M parameter)
//! sequence-labelling model trained on FST-synthesised +
//! cascade-labelled Kazakh utterances can replace those
//! extractors at equal-or-better span-F1, ≤ 5 ms latency, ≤ 5 MB
//! on disk, and **zero hallucination by construction** (per-token
//! output is one of 9 BIO labels — nothing else can come out).
//!
//! See [`docs/e2_slot_extractor_design.md`](../../../docs/e2_slot_extractor_design.md)
//! for the architecture ladder (Rungs A → B → C), data sourcing
//! policy, evaluation harness contract, and the binary success
//! criteria that decide whether the experiment ships.
//!
//! **This is a stub.** v0.0.1 ships the public types only;
//! actual extraction returns `Err(SlotExtractorError::NotLoaded)`
//! until a trained artefact is wired in.

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

/// The closed BIO label inventory. Per-token output is exactly
/// one of these — hallucination is structurally impossible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BioTag {
    /// Outside any slot span.
    O,
    /// First token of a person-name span.
    BPer,
    /// Continuation token of a person-name span.
    IPer,
    /// First token of a location span.
    BLoc,
    /// Continuation token of a location span.
    ILoc,
    /// First token of an age-value span (integer + optional «жас» noun).
    BAge,
    /// Continuation token of an age-value span.
    IAge,
    /// First token of an occupation noun span.
    BOcc,
    /// Continuation token of an occupation noun span.
    IOcc,
    /// First token of a family-relation span.
    BFam,
    /// Continuation token of a family-relation span.
    IFam,
}

impl BioTag {
    /// Slug used in dataset JSONL.
    pub fn slug(self) -> &'static str {
        match self {
            BioTag::O => "O",
            BioTag::BPer => "B-PER",
            BioTag::IPer => "I-PER",
            BioTag::BLoc => "B-LOC",
            BioTag::ILoc => "I-LOC",
            BioTag::BAge => "B-AGE",
            BioTag::IAge => "I-AGE",
            BioTag::BOcc => "B-OCC",
            BioTag::IOcc => "I-OCC",
            BioTag::BFam => "B-FAM",
            BioTag::IFam => "I-FAM",
        }
    }

    /// The slot type a non-`O` tag belongs to.
    pub fn slot(self) -> Option<SlotType> {
        match self {
            BioTag::O => None,
            BioTag::BPer | BioTag::IPer => Some(SlotType::Person),
            BioTag::BLoc | BioTag::ILoc => Some(SlotType::Location),
            BioTag::BAge | BioTag::IAge => Some(SlotType::Age),
            BioTag::BOcc | BioTag::IOcc => Some(SlotType::Occupation),
            BioTag::BFam | BioTag::IFam => Some(SlotType::Family),
        }
    }

    /// Whether this tag starts a new span (B-* tags).
    pub fn is_begin(self) -> bool {
        matches!(
            self,
            BioTag::BPer | BioTag::BLoc | BioTag::BAge | BioTag::BOcc | BioTag::BFam
        )
    }

    /// Parse a slug (`"B-PER"`, `"O"`, …) into a tag.
    pub fn parse(slug: &str) -> Option<Self> {
        match slug {
            "O" => Some(BioTag::O),
            "B-PER" => Some(BioTag::BPer),
            "I-PER" => Some(BioTag::IPer),
            "B-LOC" => Some(BioTag::BLoc),
            "I-LOC" => Some(BioTag::ILoc),
            "B-AGE" => Some(BioTag::BAge),
            "I-AGE" => Some(BioTag::IAge),
            "B-OCC" => Some(BioTag::BOcc),
            "I-OCC" => Some(BioTag::IOcc),
            "B-FAM" => Some(BioTag::BFam),
            "I-FAM" => Some(BioTag::IFam),
            _ => None,
        }
    }
}

/// The five high-level slot types. Each maps to one B-* / I-*
/// pair in the BIO inventory above.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotType {
    Person,
    Location,
    Age,
    Occupation,
    Family,
}

impl SlotType {
    pub fn slug(self) -> &'static str {
        match self {
            SlotType::Person => "person",
            SlotType::Location => "location",
            SlotType::Age => "age",
            SlotType::Occupation => "occupation",
            SlotType::Family => "family",
        }
    }
}

/// A single extracted span. `start` / `end` are token indices
/// (half-open: `tokens[start..end]` is the span).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Span {
    pub slot: SlotType,
    pub start: usize,
    pub end: usize,
    /// Text the span covers — convenience for callers that don't
    /// hold the input tokens. Always equal to
    /// `tokens[start..end].join(" ")` when the extractor produced
    /// the span; preserved on disk for diagnostic readability.
    pub text: String,
    /// Confidence score in [0, 1]. Models that don't produce
    /// calibrated probabilities set this to a heuristic
    /// (e.g. CRF marginal mass).
    pub score: f32,
}

/// What the extractor returns for one input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Extraction {
    /// Per-token tags, same length as the input tokens vector.
    pub tags: Vec<BioTag>,
    /// Decoded spans, one per contiguous non-`O` run.
    pub spans: Vec<Span>,
}

/// What can go wrong when extracting.
#[derive(Debug, Error)]
pub enum SlotExtractorError {
    /// No model artefact has been loaded yet.
    #[error("slot extractor has no loaded model — call `SlotExtractor::from_path` first")]
    NotLoaded,
    /// I/O failure reading the model artefact.
    #[error("model load i/o failure at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    /// The model artefact JSON is malformed.
    #[error("model artefact parse failure at {path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: serde_json::Error,
    },
    /// The model artefact carries an unsupported schema version.
    #[error("unsupported model schema version: got {got}, this crate supports {supported}")]
    SchemaVersion { got: String, supported: String },
}

/// A trained slot extractor ready for inference.
///
/// v0.0.1 is a stub — `extract` always returns
/// `Err(SlotExtractorError::NotLoaded)`. The training pipeline
/// is being scaffolded; subsequent commits on this branch will
/// fill in:
///   - artefact (on-disk JSON / binary)
///   - feature extractor (hash trick over (token, prev-tag) +
///     char trigrams + hand-rolled binary signals)
///   - inference (Viterbi decode through a 9-state CRF, or
///     equivalent BiLSTM forward pass)
///
/// **Public surface is intentionally minimal**: load + extract.
/// Anything more (training, dataset assembly, eval comparison
/// vs the cascade) lives in companion binaries that depend on
/// this crate but ship separately from the production loader.
#[derive(Debug, Clone, Default)]
pub struct SlotExtractor {
    artefact: Option<Artefact>,
}

/// Stub model artefact. The on-disk JSON schema will be
/// documented in
/// `docs/e2_slot_extractor_design.md` § Architectural ladder
/// once a real artefact lands.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Artefact {
    schema_version: String,
}

const SUPPORTED_SCHEMA_VERSION: &str = "0.0.1";

impl SlotExtractor {
    /// Build an empty extractor — useful as a placeholder in
    /// callers that need to compile against the public API
    /// before a trained artefact exists.
    pub fn empty() -> Self {
        Self { artefact: None }
    }

    /// Load a trained extractor from a JSON artefact on disk.
    /// **Stub** — currently only validates the schema version;
    /// inference plumbing lands in a follow-up.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, SlotExtractorError> {
        let path_str = path.as_ref().display().to_string();
        let bytes = std::fs::read(path.as_ref()).map_err(|e| SlotExtractorError::Io {
            path: path_str.clone(),
            source: e,
        })?;
        let artefact: Artefact =
            serde_json::from_slice(&bytes).map_err(|e| SlotExtractorError::Parse {
                path: path_str.clone(),
                source: e,
            })?;
        if !artefact.schema_version.starts_with("0.0.") {
            return Err(SlotExtractorError::SchemaVersion {
                got: artefact.schema_version,
                supported: SUPPORTED_SCHEMA_VERSION.to_string(),
            });
        }
        Ok(Self {
            artefact: Some(artefact),
        })
    }

    /// Extract slot spans from a tokenised utterance.
    ///
    /// **Stub** — returns `Err(SlotExtractorError::NotLoaded)`
    /// until the inference path is wired. The closed-set
    /// invariant is preserved: even when implemented, every
    /// emitted tag is guaranteed to be one of the 11 BIO labels
    /// — hallucination is structurally impossible.
    pub fn extract(&self, _tokens: &[&str]) -> Result<Extraction, SlotExtractorError> {
        if self.artefact.is_none() {
            return Err(SlotExtractorError::NotLoaded);
        }
        Err(SlotExtractorError::NotLoaded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bio_tag_roundtrip_slug() {
        for &tag in &[
            BioTag::O,
            BioTag::BPer,
            BioTag::IPer,
            BioTag::BLoc,
            BioTag::ILoc,
            BioTag::BAge,
            BioTag::IAge,
            BioTag::BOcc,
            BioTag::IOcc,
            BioTag::BFam,
            BioTag::IFam,
        ] {
            let slug = tag.slug();
            let parsed = BioTag::parse(slug).expect("slug must round-trip");
            assert_eq!(parsed, tag);
        }
    }

    #[test]
    fn bio_tag_unknown_slug_returns_none() {
        assert_eq!(BioTag::parse("garbage"), None);
        assert_eq!(BioTag::parse(""), None);
    }

    #[test]
    fn begin_tags_classified_correctly() {
        assert!(BioTag::BPer.is_begin());
        assert!(BioTag::BLoc.is_begin());
        assert!(BioTag::BAge.is_begin());
        assert!(BioTag::BOcc.is_begin());
        assert!(BioTag::BFam.is_begin());
        assert!(!BioTag::IPer.is_begin());
        assert!(!BioTag::O.is_begin());
    }

    #[test]
    fn slot_types_distinguished() {
        assert_eq!(BioTag::BPer.slot(), Some(SlotType::Person));
        assert_eq!(BioTag::ILoc.slot(), Some(SlotType::Location));
        assert_eq!(BioTag::BAge.slot(), Some(SlotType::Age));
        assert_eq!(BioTag::IOcc.slot(), Some(SlotType::Occupation));
        assert_eq!(BioTag::BFam.slot(), Some(SlotType::Family));
        assert_eq!(BioTag::O.slot(), None);
    }

    #[test]
    fn empty_extractor_returns_not_loaded() {
        let e = SlotExtractor::empty();
        let err = e.extract(&["сәлем"]).unwrap_err();
        assert!(matches!(err, SlotExtractorError::NotLoaded));
    }
}
