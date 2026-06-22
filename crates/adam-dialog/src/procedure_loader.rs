// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `procedure_loader` — **v6.8.5 / L4.6 industrial-pilot foundation**.
//!
//! Eager-load every JSONL file under `data/procedures/` into a
//! process-wide cache.  Mirrors the [`crate::v6_2_router::shared_
//! corpus`] pattern (multiple candidate paths so the loader works
//! both from the repo root and from a sub-crate test dir), but
//! produces a `Vec<ProcedureIR>` instead of a `FrameIndex` since
//! procedures aren't (yet) indexed by morphology — typical
//! fixture sets are 50-500 records, well within reach of a
//! linear scan + simple keyword scoring.
//!
//! Loader failures (missing dir, bad JSON, invariant violation)
//! return an empty list rather than panicking — the retrieval
//! handler simply produces no answer and the cascade falls
//! through, same as for any other read-only data source.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use adam_algebra::ProcedureIR;

/// Process-wide shared procedure set, lazily loaded from
/// `data/procedures/*.jsonl` on first access.
pub fn shared_procedures() -> &'static [ProcedureIR] {
    static SET: OnceLock<Vec<ProcedureIR>> = OnceLock::new();
    SET.get_or_init(load_all_or_empty)
}

fn load_all_or_empty() -> Vec<ProcedureIR> {
    for candidate in CANDIDATE_DIRS {
        let path = Path::new(candidate);
        if path.is_dir()
            && let Ok(set) = load_dir(path)
            && !set.is_empty()
        {
            return set;
        }
    }
    Vec::new()
}

/// Candidate paths so the loader works from the repo root, from
/// the `crates/adam-dialog/` working directory, from integration-
/// test working directories, and from binary `cargo run` paths.
const CANDIDATE_DIRS: &[&str] = &[
    "data/procedures",
    "../data/procedures",
    "../../data/procedures",
    "../../../data/procedures",
];

fn load_dir(dir: &Path) -> std::io::Result<Vec<ProcedureIR>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let p: PathBuf = entry.path();
        if p.extension().and_then(|s| s.to_str()) != Some("jsonl") {
            continue;
        }
        let text = std::fs::read_to_string(&p)?;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            // Silently skip malformed records — currency / shape
            // failures are caught at fixture-author time by the
            // `procedure_fixtures` integration test, NOT at
            // runtime.  Production should never see a bad
            // record reach this code path.
            if let Ok(rec) = ProcedureIR::from_jsonl_line(line) {
                out.push(rec);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The eager loader picks up the canonical fixture set.  This
    /// test is the runtime mirror of the `procedure_fixtures`
    /// integration test in `adam-algebra` — same data, different
    /// access path.
    #[test]
    fn shared_procedures_loads_canonical_fixtures() {
        let set = shared_procedures();
        // Foundation commit ships 5; CI lints the file shape, so
        // any future addition that breaks invariants would never
        // land — meaning this lower bound stays load-bearing.
        assert!(
            set.len() >= 5,
            "expected ≥5 fixtures from data/procedures/, got {}",
            set.len(),
        );
        // Spot-check: every record has a non-empty title + a
        // non-empty step list (basic invariant relay).
        for p in set {
            assert!(!p.title_kk.is_empty(), "title_kk empty for {}", p.id);
            assert!(!p.steps.is_empty(), "steps empty for {}", p.id);
        }
    }
}
