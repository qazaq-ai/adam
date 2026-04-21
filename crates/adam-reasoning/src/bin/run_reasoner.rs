//! `run_reasoner` — forward-chaining over `data/retrieval/facts.json`.
//!
//! Reads the v2.1+ facts artifact, runs `adam_reasoning::reasoner::run`,
//! writes `data/retrieval/derived_facts.json` with every rule-inferred
//! fact + its source chain + the rule id that fired.
//!
//! Idempotent by construction — calling again on the union of inputs
//! and outputs converges to the same fixed point.
//!
//! ## v3.1.0 iteration harness
//!
//! The reasoner itself is bounded: `MAX_ITER = 8` forward-chaining
//! iterations at the fixpoint loop. v3.1.0 adds a budget-aware
//! entrypoint [`reasoner::run_with_budget`] that checks the deadline
//! and interrupt flag **between iterations**. On stop, the partial
//! derivation set is written out with `status` ∈ {`"timed_out"`,
//! `"interrupted"`}.
//!
//! Accepted flags: `--time-budget <SEC>`, `--time-budget-mins <MIN>`,
//! `--progress-interval <SEC>` (default 30). SIGINT / SIGTERM →
//! graceful commit.

use std::{env, fs, path::Path, process::ExitCode, sync::Arc, time::Duration};

use adam_reasoning::{
    Fact,
    harness::{IterationBudget, ProgressCounter, ProgressMonitor, StopReason},
    reasoner,
};
use serde::{Deserialize, Serialize};

const FACTS_PATH: &str = "data/retrieval/facts.json";
const OUTPUT_PATH: &str = "data/retrieval/derived_facts.json";
const DEFAULT_PROGRESS_INTERVAL_SECS: u64 = 30;

#[derive(Debug, Deserialize)]
struct FactsFile {
    facts: Vec<Fact>,
    #[serde(default)]
    status: Option<String>,
}

#[derive(Debug, Serialize)]
struct Artifact {
    version: String,
    status: String,
    elapsed_s: u64,
    built_from: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    built_from_status: Option<String>,
    /// v3.1.0 — how many forward-chaining passes completed before
    /// either fixpoint or budget hit. Bounded above by `MAX_ITER = 8`.
    iterations_completed: usize,
    counts: Counts,
    derived: Vec<adam_reasoning::reasoner::DerivedFact>,
}

#[derive(Debug, Serialize)]
struct Counts {
    initial_facts: usize,
    derived_facts: usize,
    by_rule: std::collections::BTreeMap<String, usize>,
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let progress_interval = Duration::from_secs(
        parse_usize_flag(&args, "--progress-interval")
            .map(|n| n as u64)
            .unwrap_or(DEFAULT_PROGRESS_INTERVAL_SECS),
    );
    let budget = IterationBudget::from_args(&args);
    budget.install_signal_handler();

    let raw = match fs::read_to_string(FACTS_PATH) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot read {FACTS_PATH}: {e}");
            eprintln!("hint: run `extract_facts` first");
            return ExitCode::FAILURE;
        }
    };
    let input: FactsFile = match serde_json::from_str(&raw) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("parse {FACTS_PATH}: {e}");
            return ExitCode::FAILURE;
        }
    };

    eprintln!(
        "run_reasoner: {} initial facts (upstream status: {}) → forward-chaining …",
        input.facts.len(),
        input.status.as_deref().unwrap_or("<pre-v3.1.0>"),
    );
    if let Some(rem) = budget.remaining_secs() {
        eprintln!("run_reasoner: time budget = {rem}s, progress interval = {progress_interval:?}");
    }

    let counters = ProgressCounter::new();
    counters.add_extra(input.facts.len() as u64);
    let monitor = ProgressMonitor::spawn(
        budget.clone(),
        Arc::clone(&counters),
        progress_interval,
        "run_reasoner",
    );

    let (derived, iterations_completed) = reasoner::run_with_budget(&input.facts, &budget);
    counters.add_items(derived.len());

    let mut by_rule: std::collections::BTreeMap<String, usize> = Default::default();
    for d in &derived {
        *by_rule.entry(d.rule_id.clone()).or_insert(0) += 1;
    }

    let reason = budget.stop_reason();
    let status = reason.as_str().to_string();
    let elapsed_s = budget.elapsed_secs();

    monitor.join();

    eprintln!(
        "run_reasoner: derived {} facts in {iterations_completed} pass(es) — status: {status}, elapsed {elapsed_s}s",
        derived.len(),
    );
    for (rule, n) in &by_rule {
        eprintln!("  {rule}: {n}");
    }

    let artifact = Artifact {
        version: env!("CARGO_PKG_VERSION").to_string(),
        status,
        elapsed_s,
        built_from: FACTS_PATH.to_string(),
        built_from_status: input.status,
        iterations_completed,
        counts: Counts {
            initial_facts: input.facts.len(),
            derived_facts: derived.len(),
            by_rule,
        },
        derived,
    };

    if reason != StopReason::Completed {
        eprintln!(
            "NOTE: reasoner ended as `{}` after {iterations_completed}/{} iterations",
            artifact.status,
            8, // MAX_ITER, kept in sync with reasoner.rs
        );
    }

    if let Some(parent) = Path::new(OUTPUT_PATH).parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                eprintln!("cannot create {}: {e}", parent.display());
                return ExitCode::FAILURE;
            }
        }
    }
    let json = match serde_json::to_string_pretty(&artifact) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("serialise: {e}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = fs::write(OUTPUT_PATH, json) {
        eprintln!("write {OUTPUT_PATH}: {e}");
        return ExitCode::FAILURE;
    }
    eprintln!("wrote {OUTPUT_PATH}");
    ExitCode::SUCCESS
}

fn parse_usize_flag(args: &[String], name: &str) -> Option<usize> {
    let idx = args.iter().position(|a| a == name)?;
    args.get(idx + 1).and_then(|s| s.parse().ok())
}
