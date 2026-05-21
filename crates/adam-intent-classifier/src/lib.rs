// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! # E1 — discriminative intent classifier
//!
//! First experiment in the third-path research arc on the
//! `experimental/e1-intent-classifier` branch.
//!
//! **Hypothesis under test.** A small (~10K – 1M-parameter)
//! discriminative model trained on FST-synthesised + unit-test-
//! labelled Kazakh phrases can replace the hand-written
//! `detect_*` cascade in [`adam_dialog::semantics`] at equal-or-
//! better accuracy, ≤ 5 ms inference latency on M2, ≤ 5 MB on
//! disk, and **zero hallucination by construction** (the output
//! space is a closed enum of ~80 intent labels — nothing else can
//! come out).
//!
//! See [`docs/e1_intent_classifier_design.md`](../../../docs/e1_intent_classifier_design.md)
//! for the architecture ladder (Rungs A → B → C), data sourcing
//! policy, evaluation harness contract, and the binary success
//! criteria that decide whether the experiment ships.
//!
//! **This is a stub.** v0.0.1 ships the public types only;
//! actual classification returns `Err(ClassifierError::NotLoaded)`
//! until a trained artefact is wired in.

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

/// The closed enum of intent labels the classifier can emit.
///
/// **Why a string label and not a typed enum.** The design doc
/// (Open question #3) flags that some intents may be collapsed at
/// training time and re-distinguished later via slot extraction.
/// Carrying the label as a string keeps that fluid; the consumer
/// is responsible for mapping back to its own typed `Intent` /
/// `IntentKind`. We can promote this to a generated enum once the
/// label inventory stabilises.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentLabel(pub String);

impl IntentLabel {
    /// Wrap a string label.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Borrow the underlying label as a `&str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One classification result with full uncertainty inventory.
///
/// `top` is the argmax (the predicted label); `runners_up` is the
/// next two scoring labels so consumers can implement the
/// confidence-gap fall-back to the deterministic cascade described
/// in `docs/e1_intent_classifier_design.md` under "Production
/// wiring".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Classification {
    pub top: IntentLabel,
    pub top_score: f32,
    pub runners_up: Vec<(IntentLabel, f32)>,
}

impl Classification {
    /// Confidence gap between the argmax and the second-best
    /// label. Used by the production hook to decide whether to
    /// trust the prediction or fall back to the cascade. The
    /// design doc fixes the threshold at `0.15`.
    pub fn confidence_gap(&self) -> f32 {
        let runner_up_score = self.runners_up.first().map(|(_, s)| *s).unwrap_or(0.0);
        self.top_score - runner_up_score
    }
}

/// What can go wrong when classifying.
#[derive(Debug, Error)]
pub enum ClassifierError {
    /// No model artefact has been loaded yet.
    #[error("classifier has no loaded model — call `Classifier::from_path` first")]
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
    /// The model artefact carries a schema version this crate
    /// does not understand.
    #[error("unsupported model schema version: got {got}, this crate supports {supported}")]
    SchemaVersion { got: String, supported: String },
}

/// A trained classifier ready for inference.
///
/// v0.0.1 is a stub — `classify` always returns
/// `Err(ClassifierError::NotLoaded)`. The training pipeline is
/// being scaffolded; subsequent commits on this branch will fill
/// in:
///   - artefact (on-disk JSON / binary)
///   - feature extractor (hash trick over char n-grams + tokens)
///   - inference (sparse dot product → softmax over label scores)
///
/// **Public surface is intentionally minimal**: load + classify +
/// `labels()` for harness inspection. Anything more (training,
/// dataset assembly, eval comparison vs. the cascade) lives in
/// the companion `tools/intent_dataset/` and
/// `crates/adam-intent-classifier/examples/` binaries — keeps
/// production callers from accidentally importing training code.
#[derive(Debug, Clone)]
pub struct Classifier {
    /// Stub holds nothing yet. The trained artefact (Rung A:
    /// hash-feature weights; Rung B: embedding matrix + linear
    /// head; Rung C: tiny transformer) will land here.
    artefact: Option<Artefact>,
}

/// Stub model artefact. The on-disk JSON schema is documented in
/// `docs/e1_intent_classifier_design.md` § Architectural ladder.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Artefact {
    /// Schema version of the on-disk artefact. Increment when
    /// the JSON layout changes in a non-backward-compatible way.
    schema_version: String,
    /// The closed label inventory the model was trained against.
    labels: Vec<IntentLabel>,
}

const SUPPORTED_SCHEMA_VERSION: &str = "0.0.1";

impl Classifier {
    /// Build an empty classifier — useful as a placeholder in
    /// callers that need to compile against the public API before
    /// a trained artefact exists.
    pub fn empty() -> Self {
        Self { artefact: None }
    }

    /// Load a trained classifier from a JSON artefact on disk.
    /// **Stub** — currently only validates the schema version
    /// and label list; inference plumbing lands in a follow-up.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ClassifierError> {
        let path_str = path.as_ref().display().to_string();
        let bytes = std::fs::read(path.as_ref()).map_err(|e| ClassifierError::Io {
            path: path_str.clone(),
            source: e,
        })?;
        let artefact: Artefact =
            serde_json::from_slice(&bytes).map_err(|e| ClassifierError::Parse {
                path: path_str.clone(),
                source: e,
            })?;
        if artefact.schema_version != SUPPORTED_SCHEMA_VERSION {
            return Err(ClassifierError::SchemaVersion {
                got: artefact.schema_version,
                supported: SUPPORTED_SCHEMA_VERSION.to_string(),
            });
        }
        Ok(Self {
            artefact: Some(artefact),
        })
    }

    /// The closed label inventory the loaded model can emit.
    /// Empty when no artefact is loaded.
    pub fn labels(&self) -> &[IntentLabel] {
        self.artefact.as_ref().map(|a| &a.labels[..]).unwrap_or(&[])
    }

    /// Classify a Kazakh utterance.
    ///
    /// **Stub** — returns `Err(ClassifierError::NotLoaded)`
    /// until the inference path is wired. The closed-set
    /// invariant is preserved: even when implemented, the output
    /// is guaranteed to be one of `self.labels()` — hallucination
    /// is structurally impossible.
    pub fn classify(&self, _input: &str) -> Result<Classification, ClassifierError> {
        if self.artefact.is_none() {
            return Err(ClassifierError::NotLoaded);
        }
        // Stub: even with an artefact loaded, the inference path
        // is not wired in v0.0.1. The next commit on this branch
        // will replace this with the Rung A linear classifier.
        Err(ClassifierError::NotLoaded)
    }
}

impl Default for Classifier {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_classifier_has_no_labels() {
        let c = Classifier::empty();
        assert!(c.labels().is_empty());
    }

    #[test]
    fn empty_classifier_returns_not_loaded() {
        let c = Classifier::empty();
        let err = c.classify("сәлем").unwrap_err();
        assert!(matches!(err, ClassifierError::NotLoaded));
    }

    #[test]
    fn confidence_gap_picks_top_vs_runner_up() {
        let c = Classification {
            top: IntentLabel::new("AskHowAreYou"),
            top_score: 0.97,
            runners_up: vec![
                (IntentLabel::new("Greeting"), 0.02),
                (IntentLabel::new("AskName"), 0.01),
            ],
        };
        assert!((c.confidence_gap() - 0.95).abs() < 1e-6);
    }

    #[test]
    fn confidence_gap_handles_empty_runners_up() {
        let c = Classification {
            top: IntentLabel::new("AskHowAreYou"),
            top_score: 0.42,
            runners_up: vec![],
        };
        assert!((c.confidence_gap() - 0.42).abs() < 1e-6);
    }
}
