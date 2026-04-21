//! `run_reasoner` — forward-chaining over `data/retrieval/facts.json`.
//!
//! Reads the v2.1+ facts artifact, runs `adam_reasoning::reasoner::run`,
//! writes `data/retrieval/derived_facts.json` with every rule-inferred
//! fact + its source chain + the rule id that fired.
//!
//! Idempotent by construction — calling again on the union of inputs
//! and outputs converges to the same fixed point.

use std::{fs, path::Path, process::ExitCode};

use adam_reasoning::{Fact, reasoner};
use serde::{Deserialize, Serialize};

const FACTS_PATH: &str = "data/retrieval/facts.json";
const OUTPUT_PATH: &str = "data/retrieval/derived_facts.json";

#[derive(Debug, Deserialize)]
struct FactsFile {
    facts: Vec<Fact>,
}

#[derive(Debug, Serialize)]
struct Artifact {
    version: String,
    built_from: String,
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
        "run_reasoner: {} initial facts → forward-chaining …",
        input.facts.len()
    );
    let derived = reasoner::run(&input.facts);

    let mut by_rule: std::collections::BTreeMap<String, usize> = Default::default();
    for d in &derived {
        *by_rule.entry(d.rule_id.clone()).or_insert(0) += 1;
    }
    eprintln!(
        "run_reasoner: derived {} facts (fixpoint reached)",
        derived.len()
    );
    for (rule, n) in &by_rule {
        eprintln!("  {rule}: {n}");
    }

    let artifact = Artifact {
        version: env!("CARGO_PKG_VERSION").to_string(),
        built_from: FACTS_PATH.to_string(),
        counts: Counts {
            initial_facts: input.facts.len(),
            derived_facts: derived.len(),
            by_rule,
        },
        derived,
    };

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
