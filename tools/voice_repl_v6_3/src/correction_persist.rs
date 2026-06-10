// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! # Correction persist — v6.5 self-learning loop, learn layer
//!
//! When the [`rejection_detector`](crate::rejection_detector) fires
//! on the next turn, this module writes a record of the rejected
//! turn to `data/mistake_corrections.jsonl`.  rc8 will load that
//! file at startup and override the cascade on matching future
//! inputs.
//!
//! ## File format
//!
//! One JSON object per line (`.jsonl`).  Each line records exactly
//! ONE rejected turn:
//!
//! ```json
//! {
//!   "wrong_input": "Менің атым да улет",
//!   "wrong_input_normalised": "менің атым да улет",
//!   "wrong_intent": "StatementOfName",
//!   "wrong_intent_confidence": 1.0,
//!   "wrong_output": "Аты туралы мынаны айта аламын: …poem…",
//!   "rejection_kind": "rephrase",
//!   "rejection_hint": "Менің атым дауыледі",
//!   "wall_clock_secs_since_epoch": 1717930800
//! }
//! ```
//!
//! The file is APPEND-ONLY in this rc; rc8 will load + index it.
//! Old records survive forever (small per-entry, ≲ 300 bytes
//! typical) — there is no rotation policy yet.
//!
//! ## Why a flat file
//!
//! - Inspectable with `cat` / `grep` / `jq`.
//! - Versionable in git so the audit transcript and the learning
//!   record stay together.
//! - No DB dependency — keeps the binary single-file.

use crate::rejection_detector::{RejectionKind, RejectionSignal};
use crate::session_journal::JournalTurn;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Where the `.jsonl` file lives by default.
///
/// Relative to the CWD adam was launched from — matches the
/// rest of the voice REPL's data conventions (`data/checkpoints/...`,
/// `data/world_core/...`).
pub const DEFAULT_CORRECTION_PATH: &str = "data/mistake_corrections.jsonl";

/// One persisted correction record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionRecord {
    /// Raw STT input that was rejected.
    pub wrong_input: String,
    /// Post-normalisation form (cascade saw this).
    pub wrong_input_normalised: String,
    /// Intent classifier's label on the rejected turn.
    pub wrong_intent: Option<String>,
    /// Confidence on `wrong_intent`.
    pub wrong_intent_confidence: Option<f32>,
    /// adam's reply that the user rejected.
    pub wrong_output: String,
    /// Which signal flagged the rejection (`explicit`, `rephrase`,
    /// `correction`).
    pub rejection_kind: String,
    /// The user's next-turn utterance — the rephrase / correction
    /// / explicit-rejection text itself.  rc8 will use this as the
    /// "what did the user actually mean" hint when matching future
    /// similar inputs.
    pub rejection_hint: String,
    /// Wall-clock seconds since UNIX epoch.  Lets rc8 reason about
    /// recency (e.g. "prefer corrections from the last 30 days").
    pub wall_clock_secs_since_epoch: u64,
}

impl CorrectionRecord {
    /// Build a record from a rejection signal + the rejected turn's
    /// snapshot + the current input that triggered the rejection.
    pub fn from_signal(signal: &RejectionSignal, current_input: &str) -> Self {
        let rejected: &JournalTurn = &signal.rejected_turn;
        let kind = match signal.kind {
            RejectionKind::Explicit => "explicit",
            RejectionKind::Rephrase => "rephrase",
            RejectionKind::Correction => "correction",
        };
        Self {
            wrong_input: rejected.input_raw.clone(),
            wrong_input_normalised: rejected.input_normalised.clone(),
            wrong_intent: rejected.intent.clone(),
            wrong_intent_confidence: rejected.intent_confidence,
            wrong_output: rejected.output.clone(),
            rejection_kind: kind.into(),
            rejection_hint: current_input.into(),
            wall_clock_secs_since_epoch: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        }
    }
}

/// Append one record to the default path.  Creates the file (and
/// its parent directory tree) when missing.  Each record is one
/// line of compact JSON terminated by `\n`.
///
/// Errors are returned but the voice REPL caller swallows them
/// (printing a one-line warning) — failing to persist must never
/// crash the REPL.
pub fn append(record: &CorrectionRecord) -> std::io::Result<()> {
    append_to(DEFAULT_CORRECTION_PATH, record)
}

/// Variant that takes an explicit path.  Used by tests.
pub fn append_to<P: AsRef<Path>>(path: P, record: &CorrectionRecord) -> std::io::Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path.as_ref())?;
    let line = serde_json::to_string(record)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    f.write_all(line.as_bytes())?;
    f.write_all(b"\n")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rejection_detector::RejectionKind;
    use crate::session_journal::JournalTurn;
    use std::fs;
    use std::io::Read;

    fn fixture_turn() -> JournalTurn {
        JournalTurn {
            turn_no: 5,
            input_raw: "Менің атым да улет".into(),
            input_normalised: "менің атым да улет".into(),
            intent: Some("StatementOfName".into()),
            intent_confidence: Some(1.0),
            output: "Аты туралы мынаны айта аламын: …poem…".into(),
        }
    }

    #[test]
    fn from_signal_copies_all_fields() {
        let sig = RejectionSignal {
            kind: RejectionKind::Rephrase,
            rejected_turn: fixture_turn(),
        };
        let rec = CorrectionRecord::from_signal(&sig, "Менің атым дауыледі");
        assert_eq!(rec.wrong_input, "Менің атым да улет");
        assert_eq!(rec.wrong_intent.as_deref(), Some("StatementOfName"));
        assert_eq!(rec.wrong_intent_confidence, Some(1.0));
        assert_eq!(rec.rejection_kind, "rephrase");
        assert_eq!(rec.rejection_hint, "Менің атым дауыледі");
        assert!(rec.wall_clock_secs_since_epoch > 1_700_000_000);
    }

    #[test]
    fn append_writes_one_jsonl_line() {
        let dir = tempfile::tempdir().expect("tmpdir");
        let path = dir.path().join("test_corrections.jsonl");
        let sig = RejectionSignal {
            kind: RejectionKind::Explicit,
            rejected_turn: fixture_turn(),
        };
        let rec = CorrectionRecord::from_signal(&sig, "жоқ");
        append_to(&path, &rec).expect("append ok");
        append_to(&path, &rec).expect("append 2 ok");

        let mut s = String::new();
        fs::File::open(&path)
            .unwrap()
            .read_to_string(&mut s)
            .unwrap();
        let lines: Vec<&str> = s.lines().collect();
        assert_eq!(lines.len(), 2);
        // Round-trip parse each line.
        for l in &lines {
            let back: CorrectionRecord = serde_json::from_str(l).expect("parse");
            assert_eq!(back.wrong_input, "Менің атым да улет");
        }
    }

    #[test]
    fn explicit_kind_serialises_correctly() {
        let sig = RejectionSignal {
            kind: RejectionKind::Correction,
            rejected_turn: fixture_turn(),
        };
        let rec = CorrectionRecord::from_signal(&sig, "Я имел в виду писателей");
        assert_eq!(rec.rejection_kind, "correction");
        let json = serde_json::to_string(&rec).expect("serialise");
        assert!(json.contains(r#""rejection_kind":"correction""#));
    }
}
