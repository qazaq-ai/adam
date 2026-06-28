// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Typed candidate records — the data the ingestion queue
//! moves around.
//!
//! [`CandidateFact`] mirrors the world_core JSONL fact
//! shape (`subject` / `predicate` / `object`) plus the
//! ingestion-side fields (`source` / `status` / `confidence`
//! / `created_at`).  [`CandidateProcedure`] is the
//! procedure-side equivalent for SOP material that
//! eventually lands in `data/procedures/`.
//!
//! Both share the [`CandidateId`] string identifier — opaque
//! stable string the pipeline generates and the integrator
//! preserves when writing into world_core / procedure
//! jsonl.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::source::SourceRef;
use crate::status::IngestionStatus;

/// Stable opaque identifier for a candidate.  Pipeline
/// generates these from a deterministic source-position +
/// hash so re-runs of the same extractor on the same
/// source don't duplicate.
pub type CandidateId = String;

/// A typed candidate fact, queued for validation / review /
/// integration into world_core.  Mirrors the shape
/// production already consumes (subject + predicate +
/// object) so the integrator is a thin write — no
/// reshaping at the boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateFact {
    pub id: CandidateId,
    /// Lower-cased Kazakh surface for the agent / subject
    /// of the fact.  Same convention as world_core's
    /// `facts[].subject`.
    pub subject: String,
    /// Predicate name from the closed enum the existing
    /// reasoner understands («is_a», «part_of»,
    /// «born_in», «located_in», etc.).  Kept as `String`
    /// here because the closed enum lives in
    /// `adam-reasoning`; the validator will check this
    /// string against the known set.
    pub predicate: String,
    /// Lower-cased Kazakh surface for the object /
    /// complement.
    pub object: String,
    /// Optional full source sentence the fact was
    /// extracted from.  Lets the reviewer see the original
    /// context.  Empty for manual entries.
    #[serde(default)]
    pub source_sentence: String,
    /// Provenance — where did this candidate come from?
    pub source: SourceRef,
    /// Where in the queue this candidate sits.
    pub status: IngestionStatus,
    /// Extractor's self-assessed confidence in [0.0, 1.0].
    /// Manual entries default to 1.0; pattern-based
    /// extractors emit lower numbers; statistical
    /// extractors emit calibrated scores.
    pub confidence: f32,
    /// ISO date (`YYYY-MM-DD`) when this candidate was
    /// created.  Pipeline stamps this; integrator preserves
    /// it as the world_core entry's `reviewed_at` field.
    pub created_at: String,
    /// Free-text reviewer notes that accumulate across
    /// status transitions.  Empty at creation; the human
    /// reviewer appends a one-liner explaining why
    /// approved / rejected.
    #[serde(default)]
    pub notes: String,
}

/// A typed candidate procedure — the SOP-side equivalent
/// of [`CandidateFact`].  Shape mirrors `adam_algebra::
/// ProcedureIR` so the integrator is again a thin write
/// into `data/procedures/*.jsonl`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateProcedure {
    pub id: CandidateId,
    /// Kazakh title.  Same convention as
    /// `ProcedureIR::title_kk`.
    pub title_kk: String,
    /// Optional Russian / English titles — added per
    /// industrial-pilot-from-day-1 direction (v6.8.27).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_ru: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_en: Option<String>,
    /// Domain bucket («okhrana_truda» / «metallurgy» /
    /// «automotive» / «construction» / «other») — used to
    /// route candidates to domain-specific validators.
    pub domain: String,
    /// Step descriptions in order.  Free-text Kazakh for
    /// the first ingestion pass; later schema lift turns
    /// these into `ProcedureStep` records.
    pub step_descriptions: Vec<String>,
    /// Provenance.
    pub source: SourceRef,
    pub status: IngestionStatus,
    pub confidence: f32,
    pub created_at: String,
    #[serde(default)]
    pub notes: String,
}

/// Errors raised when a candidate fails its self-check
/// invariants on construction or deserialisation.
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("required field `{0}` is empty")]
    EmptyField(&'static str),
    #[error("confidence {0} out of range [0.0, 1.0]")]
    ConfidenceOutOfRange(f32),
    #[error("invalid created_at date `{0}` (expected YYYY-MM-DD)")]
    MalformedDate(String),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

impl CandidateFact {
    /// Run structural invariants.  Called automatically by
    /// `from_jsonl_line`; exposed so hand-built instances
    /// can be checked in unit tests.
    pub fn check_invariants(&self) -> Result<(), ParseError> {
        if self.id.trim().is_empty() {
            return Err(ParseError::EmptyField("id"));
        }
        if self.subject.trim().is_empty() {
            return Err(ParseError::EmptyField("subject"));
        }
        if self.predicate.trim().is_empty() {
            return Err(ParseError::EmptyField("predicate"));
        }
        if self.object.trim().is_empty() {
            return Err(ParseError::EmptyField("object"));
        }
        if !(0.0..=1.0).contains(&self.confidence) {
            return Err(ParseError::ConfidenceOutOfRange(self.confidence));
        }
        if !iso_date_well_formed(&self.created_at) {
            return Err(ParseError::MalformedDate(self.created_at.clone()));
        }
        Ok(())
    }

    pub fn from_jsonl_line(line: &str) -> Result<Self, ParseError> {
        let parsed: Self = serde_json::from_str(line)?;
        parsed.check_invariants()?;
        Ok(parsed)
    }
}

impl CandidateProcedure {
    pub fn check_invariants(&self) -> Result<(), ParseError> {
        if self.id.trim().is_empty() {
            return Err(ParseError::EmptyField("id"));
        }
        if self.title_kk.trim().is_empty() {
            return Err(ParseError::EmptyField("title_kk"));
        }
        if self.domain.trim().is_empty() {
            return Err(ParseError::EmptyField("domain"));
        }
        if self.step_descriptions.is_empty() {
            return Err(ParseError::EmptyField("step_descriptions"));
        }
        if !(0.0..=1.0).contains(&self.confidence) {
            return Err(ParseError::ConfidenceOutOfRange(self.confidence));
        }
        if !iso_date_well_formed(&self.created_at) {
            return Err(ParseError::MalformedDate(self.created_at.clone()));
        }
        Ok(())
    }

    pub fn from_jsonl_line(line: &str) -> Result<Self, ParseError> {
        let parsed: Self = serde_json::from_str(line)?;
        parsed.check_invariants()?;
        Ok(parsed)
    }
}

fn iso_date_well_formed(s: &str) -> bool {
    if s.len() != 10 {
        return false;
    }
    let bytes = s.as_bytes();
    bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(i, b)| matches!(i, 4 | 7) || b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceRef;
    use crate::status::IngestionStatus;

    fn sample_fact() -> CandidateFact {
        CandidateFact {
            id: "cand_001".into(),
            subject: "алматы".into(),
            predicate: "is_a".into(),
            object: "қала".into(),
            source_sentence: "Алматы — Қазақстанның ірі қаласы.".into(),
            source: SourceRef::manual("shaman"),
            status: IngestionStatus::Pending,
            confidence: 0.95,
            created_at: "2026-06-28".into(),
            notes: String::new(),
        }
    }

    #[test]
    fn fact_invariants_accept_sample() {
        sample_fact().check_invariants().expect("valid sample");
    }

    #[test]
    fn fact_invariants_reject_empty_subject() {
        let mut f = sample_fact();
        f.subject = "  ".into();
        assert!(matches!(
            f.check_invariants(),
            Err(ParseError::EmptyField("subject"))
        ));
    }

    #[test]
    fn fact_invariants_reject_out_of_range_confidence() {
        let mut f = sample_fact();
        f.confidence = 1.5;
        assert!(matches!(
            f.check_invariants(),
            Err(ParseError::ConfidenceOutOfRange(_))
        ));
    }

    #[test]
    fn fact_invariants_reject_malformed_date() {
        let mut f = sample_fact();
        f.created_at = "yesterday".into();
        assert!(matches!(
            f.check_invariants(),
            Err(ParseError::MalformedDate(_))
        ));
    }

    #[test]
    fn fact_jsonl_round_trip() {
        let f = sample_fact();
        let line = serde_json::to_string(&f).expect("serialize");
        let back = CandidateFact::from_jsonl_line(&line).expect("round trip");
        assert_eq!(back, f);
    }

    #[test]
    fn procedure_invariants_reject_empty_steps() {
        let p = CandidateProcedure {
            id: "proc_001".into(),
            title_kk: "Бастапқы инструктаж".into(),
            title_ru: None,
            title_en: None,
            domain: "okhrana_truda".into(),
            step_descriptions: vec![],
            source: SourceRef::manual("shaman"),
            status: IngestionStatus::Pending,
            confidence: 0.9,
            created_at: "2026-06-28".into(),
            notes: String::new(),
        };
        assert!(matches!(
            p.check_invariants(),
            Err(ParseError::EmptyField("step_descriptions"))
        ));
    }
}
