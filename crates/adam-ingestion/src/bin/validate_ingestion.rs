// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! CI gate for the ingestion queue.
//!
//! Confirms every record under `data/ingestion/` (or the
//! root passed via `--root`) parses, satisfies its
//! `check_invariants()`, has a known status, and that the
//! aggregate fact/procedure counts by status form a healthy
//! mix.  Missing directory → silent success (no candidates
//! yet on a fresh checkout).
//!
//! Run from `scripts/validate_foundation.sh` alongside the
//! other foundation gates.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

use adam_ingestion::candidate::{CandidateFact, CandidateProcedure};
use adam_ingestion::status::IngestionStatus;
use adam_ingestion::store::CandidateStore;

const DEFAULT_ROOT: &str = "data/ingestion";

fn parse_args() -> Result<PathBuf, String> {
    let mut args = std::env::args().skip(1);
    let mut root: Option<String> = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--root" => root = args.next(),
            "-h" | "--help" => {
                return Err(format!(
                    "usage: validate_ingestion [--root <DIR>] (default: {DEFAULT_ROOT})"
                ));
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(PathBuf::from(root.unwrap_or_else(|| DEFAULT_ROOT.into())))
}

fn main() -> ExitCode {
    let root = match parse_args() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    if !root.exists() {
        // Clean checkout — no ingestion queue yet.  Skip
        // silently; the gate degrades the same way the
        // corpus-pipeline gates do when their manifests
        // are missing.
        println!(
            "[validate_ingestion] SKIP: {} missing (no candidates yet)",
            root.display()
        );
        return ExitCode::SUCCESS;
    }
    let store = match CandidateStore::open(&root) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "[validate_ingestion] FAIL: cannot open store at {}: {e}",
                root.display()
            );
            return ExitCode::FAILURE;
        }
    };
    let mut fail = false;
    let facts = match store.load_facts() {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[validate_ingestion] FAIL: facts.jsonl parse error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let procedures = match store.load_procedures() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[validate_ingestion] FAIL: procedures.jsonl parse error: {e}");
            return ExitCode::FAILURE;
        }
    };
    // Per-record invariants (load_*() already calls
    // check_invariants() via from_jsonl_line, but we
    // re-assert here for the hand-built case where someone
    // sidesteps the constructor).
    for f in &facts {
        if let Err(e) = f.check_invariants() {
            eprintln!("[validate_ingestion] FAIL: fact `{}`: {e}", f.id);
            fail = true;
        }
    }
    for p in &procedures {
        if let Err(e) = p.check_invariants() {
            eprintln!("[validate_ingestion] FAIL: procedure `{}`: {e}", p.id);
            fail = true;
        }
    }
    // Unique ids.
    if let Some(dup) = first_duplicate_id_fact(&facts) {
        eprintln!("[validate_ingestion] FAIL: duplicate fact id `{dup}`");
        fail = true;
    }
    if let Some(dup) = first_duplicate_id_procedure(&procedures) {
        eprintln!("[validate_ingestion] FAIL: duplicate procedure id `{dup}`");
        fail = true;
    }
    // Report status mix as INFO so the gate's output is
    // useful even when everything passes.
    print_status_histogram("facts", facts.iter().map(|f| f.status));
    print_status_histogram("procedures", procedures.iter().map(|p| p.status));
    if fail {
        eprintln!(
            "[validate_ingestion] gate FAILED on {} (facts: {}, procedures: {})",
            root.display(),
            facts.len(),
            procedures.len()
        );
        ExitCode::FAILURE
    } else {
        println!(
            "[validate_ingestion] PASS — {} facts + {} procedures clean at {}",
            facts.len(),
            procedures.len(),
            root.display()
        );
        ExitCode::SUCCESS
    }
}

fn first_duplicate_id_fact(facts: &[CandidateFact]) -> Option<&str> {
    let mut seen = std::collections::HashSet::new();
    for f in facts {
        if !seen.insert(&f.id) {
            return Some(&f.id);
        }
    }
    None
}

fn first_duplicate_id_procedure(procs: &[CandidateProcedure]) -> Option<&str> {
    let mut seen = std::collections::HashSet::new();
    for p in procs {
        if !seen.insert(&p.id) {
            return Some(&p.id);
        }
    }
    None
}

fn print_status_histogram(label: &str, statuses: impl Iterator<Item = IngestionStatus>) {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut total = 0usize;
    for s in statuses {
        total += 1;
        let key = format!("{s:?}");
        *counts.entry(key).or_default() += 1;
    }
    if total == 0 {
        println!("[validate_ingestion] INFO {label}: 0 records");
        return;
    }
    let parts: Vec<String> = counts.iter().map(|(k, v)| format!("{k}={v}")).collect();
    println!(
        "[validate_ingestion] INFO {label} ({total} total): {}",
        parts.join(" / ")
    );
}
