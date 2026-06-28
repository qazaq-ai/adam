// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Persistent JSONL store for the ingestion queue.
//!
//! Two files per pipeline run, both one-record-per-line:
//!
//!   * `<root>/facts.jsonl`      — `CandidateFact` records
//!   * `<root>/procedures.jsonl` — `CandidateProcedure` records
//!
//! [`CandidateStore`] gives the pipeline stages a uniform
//! «load all, update one, save all» interface.  Concurrent
//! writers are NOT supported — the pipeline runs as
//! sequential CLI stages (extractor → validator → reviewer →
//! integrator), so file locks aren't needed.  Atomicity is
//! at the file-write level: writes go to `<file>.tmp` then
//! rename, so a partial crash doesn't corrupt the queue.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::candidate::{CandidateFact, CandidateId, CandidateProcedure, ParseError};
use crate::status::{IngestionStatus, StatusTransitionError};

/// Persistent store for ingestion candidates.  Owns the
/// on-disk queue files; provides typed load / save / update
/// operations.
pub struct CandidateStore {
    root: PathBuf,
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("io error on {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("parse error on line {line} of {path}: {source}")]
    Parse {
        path: PathBuf,
        line: usize,
        #[source]
        source: ParseError,
    },
    #[error("status transition error: {0}")]
    Transition(#[from] StatusTransitionError),
    #[error("candidate id `{0}` not found")]
    NotFound(CandidateId),
}

impl CandidateStore {
    /// Open (or implicitly create) a store rooted at `root`.
    /// The directory is created if it doesn't exist; the
    /// jsonl files are created lazily on first write.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, StoreError> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root).map_err(|e| StoreError::Io {
            path: root.clone(),
            source: e,
        })?;
        Ok(Self { root })
    }

    pub fn facts_path(&self) -> PathBuf {
        self.root.join("facts.jsonl")
    }

    pub fn procedures_path(&self) -> PathBuf {
        self.root.join("procedures.jsonl")
    }

    /// Load every fact candidate from disk.  Empty / missing
    /// file → empty Vec.  Per-line parse errors surface as
    /// `StoreError::Parse` with the offending line number.
    pub fn load_facts(&self) -> Result<Vec<CandidateFact>, StoreError> {
        let path = self.facts_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let text = fs::read_to_string(&path).map_err(|e| StoreError::Io {
            path: path.clone(),
            source: e,
        })?;
        let mut out = Vec::new();
        for (idx, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            let parsed = CandidateFact::from_jsonl_line(line).map_err(|e| StoreError::Parse {
                path: path.clone(),
                line: idx + 1,
                source: e,
            })?;
            out.push(parsed);
        }
        Ok(out)
    }

    pub fn load_procedures(&self) -> Result<Vec<CandidateProcedure>, StoreError> {
        let path = self.procedures_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let text = fs::read_to_string(&path).map_err(|e| StoreError::Io {
            path: path.clone(),
            source: e,
        })?;
        let mut out = Vec::new();
        for (idx, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            let parsed =
                CandidateProcedure::from_jsonl_line(line).map_err(|e| StoreError::Parse {
                    path: path.clone(),
                    line: idx + 1,
                    source: e,
                })?;
            out.push(parsed);
        }
        Ok(out)
    }

    /// Replace the facts file with `facts` (atomic via
    /// `<file>.tmp` rename).  Caller is responsible for
    /// holding the full set in memory — appropriate for the
    /// hundreds-to-low-thousands-of-records regime the
    /// pipeline operates in.
    pub fn save_facts(&self, facts: &[CandidateFact]) -> Result<(), StoreError> {
        let path = self.facts_path();
        let tmp = path.with_extension("jsonl.tmp");
        let mut file = fs::File::create(&tmp).map_err(|e| StoreError::Io {
            path: tmp.clone(),
            source: e,
        })?;
        for f in facts {
            let line = serde_json::to_string(f).map_err(|e| StoreError::Parse {
                path: path.clone(),
                line: 0,
                source: ParseError::Json(e),
            })?;
            writeln!(file, "{line}").map_err(|e| StoreError::Io {
                path: tmp.clone(),
                source: e,
            })?;
        }
        drop(file);
        fs::rename(&tmp, &path).map_err(|e| StoreError::Io {
            path: path.clone(),
            source: e,
        })?;
        Ok(())
    }

    pub fn save_procedures(&self, procs: &[CandidateProcedure]) -> Result<(), StoreError> {
        let path = self.procedures_path();
        let tmp = path.with_extension("jsonl.tmp");
        let mut file = fs::File::create(&tmp).map_err(|e| StoreError::Io {
            path: tmp.clone(),
            source: e,
        })?;
        for p in procs {
            let line = serde_json::to_string(p).map_err(|e| StoreError::Parse {
                path: path.clone(),
                line: 0,
                source: ParseError::Json(e),
            })?;
            writeln!(file, "{line}").map_err(|e| StoreError::Io {
                path: tmp.clone(),
                source: e,
            })?;
        }
        drop(file);
        fs::rename(&tmp, &path).map_err(|e| StoreError::Io {
            path: path.clone(),
            source: e,
        })?;
        Ok(())
    }

    /// Update a candidate fact's status in place.  Enforces
    /// the state-machine via `IngestionStatus::can_transition`.
    /// Persists the updated set back to disk.
    pub fn update_fact_status(
        &self,
        id: &CandidateId,
        new_status: IngestionStatus,
        note: &str,
    ) -> Result<(), StoreError> {
        let mut facts = self.load_facts()?;
        let f = facts
            .iter_mut()
            .find(|f| &f.id == id)
            .ok_or_else(|| StoreError::NotFound(id.clone()))?;
        if !f.status.can_transition(new_status) {
            return Err(StatusTransitionError {
                from: f.status,
                to: new_status,
            }
            .into());
        }
        f.status = new_status;
        if !note.trim().is_empty() {
            if !f.notes.is_empty() {
                f.notes.push_str("; ");
            }
            f.notes.push_str(note.trim());
        }
        self.save_facts(&facts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceRef;
    use crate::status::IngestionStatus;

    fn tmp_root() -> PathBuf {
        // ThreadId's debug repr is stable enough to distinguish
        // parallel test runs without depending on the unstable
        // `as_u64()` accessor.
        let tid = format!("{:?}", std::thread::current().id()).replace([':', ' ', '(', ')'], "_");
        let dir = std::env::temp_dir().join(format!("adam-ingestion-test-{tid}"));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn sample_fact(id: &str) -> CandidateFact {
        CandidateFact {
            id: id.into(),
            subject: "алматы".into(),
            predicate: "is_a".into(),
            object: "қала".into(),
            source_sentence: "Алматы — қала.".into(),
            source: SourceRef::manual("shaman"),
            status: IngestionStatus::Pending,
            confidence: 0.9,
            created_at: "2026-06-28".into(),
            notes: String::new(),
        }
    }

    #[test]
    fn open_creates_directory() {
        let root = tmp_root();
        let store = CandidateStore::open(&root).expect("open");
        assert!(root.exists());
        let _ = fs::remove_dir_all(&root);
        drop(store);
    }

    #[test]
    fn save_then_load_facts_round_trip() {
        let root = tmp_root();
        let store = CandidateStore::open(&root).expect("open");
        let facts = vec![sample_fact("a"), sample_fact("b")];
        store.save_facts(&facts).expect("save");
        let loaded = store.load_facts().expect("load");
        assert_eq!(loaded, facts);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn load_facts_returns_empty_when_file_absent() {
        let root = tmp_root();
        let store = CandidateStore::open(&root).expect("open");
        let loaded = store.load_facts().expect("load");
        assert!(loaded.is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn update_fact_status_enforces_state_machine() {
        let root = tmp_root();
        let store = CandidateStore::open(&root).expect("open");
        store.save_facts(&[sample_fact("a")]).expect("save");
        // Legal: Pending → AutoAccepted.
        store
            .update_fact_status(&"a".into(), IngestionStatus::AutoAccepted, "validator ok")
            .expect("legal transition");
        let loaded = store.load_facts().expect("load");
        assert_eq!(loaded[0].status, IngestionStatus::AutoAccepted);
        assert!(loaded[0].notes.contains("validator ok"));
        // Illegal: AutoAccepted → AutoRejected.
        let err = store
            .update_fact_status(&"a".into(), IngestionStatus::AutoRejected, "")
            .expect_err("illegal transition");
        assert!(matches!(err, StoreError::Transition(_)));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn update_fact_status_returns_not_found_for_unknown_id() {
        let root = tmp_root();
        let store = CandidateStore::open(&root).expect("open");
        let err = store
            .update_fact_status(&"missing".into(), IngestionStatus::AutoAccepted, "")
            .expect_err("not found");
        assert!(matches!(err, StoreError::NotFound(_)));
        let _ = fs::remove_dir_all(&root);
    }
}
