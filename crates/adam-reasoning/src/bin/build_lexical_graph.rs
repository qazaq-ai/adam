//! `build_lexical_graph` — project `data/retrieval/facts.json` into a
//! deterministic lexical graph `data/retrieval/lexical_graph.json`.
//!
//! Reads the v2.1+ facts artifact, delegates construction to
//! `adam_reasoning::graph::LexicalGraph::from_facts`, and writes the
//! resulting node + edge structure alongside summary statistics.
//!
//! **Zero new extraction**, zero new heuristics — the graph is a pure
//! projection. Regenerate whenever `facts.json` changes.
//!
//! ## v3.1.0 iteration harness
//!
//! Projection is O(|facts|) with tiny per-fact work (hash insert into
//! `BTreeMap`). On 1M facts it finishes in seconds — no rayon needed,
//! but we still honour the harness contract for consistency: every
//! binary in the reasoning pipeline accepts `--time-budget`,
//! `--progress-interval`, and SIGINT; downstream consumers see the
//! same output schema regardless of the run's outcome.

use std::{env, fs, path::Path, process::ExitCode, sync::Arc, time::Duration};

use adam_reasoning::{
    Fact,
    graph::LexicalGraph,
    harness::{IterationBudget, ProgressCounter, ProgressMonitor, StopReason},
};
use serde::{Deserialize, Serialize};

const FACTS_PATH: &str = "data/retrieval/facts.json";
const OUTPUT_PATH: &str = "data/retrieval/lexical_graph.json";
const DEFAULT_PROGRESS_INTERVAL_SECS: u64 = 30;

#[derive(Debug, Deserialize)]
struct FactsFile {
    facts: Vec<Fact>,
    /// v3.1.0 — optional, read back so we can surface upstream's
    /// status in our own artifact's `built_from_status` field.
    #[serde(default)]
    status: Option<String>,
}

#[derive(Debug, Serialize)]
struct Artifact {
    version: String,
    status: String,
    elapsed_s: u64,
    built_from: String,
    /// v3.1.0 — `facts.json`'s own status (if set), for cross-artifact
    /// auditability. `None` when the input is v3.0.x.
    #[serde(skip_serializing_if = "Option::is_none")]
    built_from_status: Option<String>,
    summary: Summary,
    graph: LexicalGraph,
}

#[derive(Debug, Serialize)]
struct Summary {
    total_nodes: usize,
    total_edges: usize,
    facts_ingested: usize,
    most_connected: Vec<(String, usize)>,
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

    let facts_raw = match fs::read_to_string(FACTS_PATH) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot read {FACTS_PATH}: {e}");
            eprintln!("hint: run `extract_facts` first");
            return ExitCode::FAILURE;
        }
    };
    let facts_file: FactsFile = match serde_json::from_str(&facts_raw) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("parse {FACTS_PATH}: {e}");
            return ExitCode::FAILURE;
        }
    };

    eprintln!(
        "build_lexical_graph: loaded {} facts (upstream status: {})",
        facts_file.facts.len(),
        facts_file.status.as_deref().unwrap_or("<pre-v3.1.0>"),
    );

    let counters = ProgressCounter::new();
    counters.add_extra(facts_file.facts.len() as u64);
    let monitor = ProgressMonitor::spawn(
        budget.clone(),
        Arc::clone(&counters),
        progress_interval,
        "build_lexical_graph",
    );

    // Projection itself doesn't check the budget — it's O(|facts|)
    // with tiny per-fact work. Budget only matters if the whole binary
    // is spawned after the outer deadline has already passed; the
    // harness check below catches that.
    let graph = if budget.should_stop() {
        // Degenerate: deadline already past. Commit an empty graph
        // with the appropriate status so downstream can distinguish.
        LexicalGraph::from_facts(&[])
    } else {
        LexicalGraph::from_facts(&facts_file.facts)
    };
    counters.add_items(graph.edges.len());
    eprintln!(
        "build_lexical_graph: {} facts → {} nodes, {} edges",
        graph.facts_ingested,
        graph.nodes.len(),
        graph.edges.len(),
    );

    // Most-connected report — top 5 by (out + in) degree.
    let mut by_degree: Vec<(String, usize)> = graph
        .nodes
        .iter()
        .map(|(root, s)| (root.clone(), s.out_degree + s.in_degree))
        .collect();
    by_degree.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let most_connected: Vec<(String, usize)> = by_degree.into_iter().take(5).collect();
    for (root, deg) in &most_connected {
        eprintln!("  most-connected: {root} (degree {deg})");
    }

    let reason = budget.stop_reason();
    let artifact = Artifact {
        version: env!("CARGO_PKG_VERSION").to_string(),
        status: reason.as_str().to_string(),
        elapsed_s: budget.elapsed_secs(),
        built_from: FACTS_PATH.to_string(),
        built_from_status: facts_file.status,
        summary: Summary {
            total_nodes: graph.nodes.len(),
            total_edges: graph.edges.len(),
            facts_ingested: graph.facts_ingested,
            most_connected,
        },
        graph,
    };

    monitor.join();

    if reason != StopReason::Completed {
        eprintln!(
            "NOTE: graph build ended as `{}` (elapsed {}s)",
            artifact.status, artifact.elapsed_s
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
