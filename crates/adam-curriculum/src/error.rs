// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Typed errors for the curriculum layer.

use thiserror::Error;

/// Top-level error type for `adam-curriculum`. Specific variants
/// surface the failure mode so dialog / UI code can branch on the
/// reason, not parse a free-text message.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum CurriculumError {
    /// A concept id referenced by the caller is not present in the
    /// loaded curriculum graph.
    #[error("unknown concept id: {0}")]
    UnknownConcept(String),

    /// A prerequisite cycle was detected in the curriculum graph
    /// (e.g. concept A requires B which requires A). This is a
    /// curriculum-authoring bug; the graph must be a DAG.
    #[error("prerequisite cycle through concept: {0}")]
    PrerequisiteCycle(String),

    /// A concept is missing a required field: `definition`,
    /// `examples`, `test_items`, or `mastery_threshold`.
    #[error("concept {concept} missing required field: {field}")]
    MissingField {
        concept: String,
        field: &'static str,
    },

    /// Threshold value is outside [0.0, 1.0].
    #[error("threshold {0} out of range [0.0, 1.0]")]
    ThresholdOutOfRange(f32),

    /// Caller asked for the next lesson for a learner whose graph
    /// is fully mastered. UI should switch to a "course complete"
    /// state, not an error.
    #[error("learner has mastered every concept in scope")]
    AllConceptsMastered,

    /// I/O or JSON-deserialisation failure when loading a curriculum
    /// or learner record from disk.
    #[error("curriculum load: {0}")]
    Load(String),
}

pub type Result<T> = std::result::Result<T, CurriculumError>;
