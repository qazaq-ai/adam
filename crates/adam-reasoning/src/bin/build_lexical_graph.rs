//! `build_lexical_graph` — project `data/retrieval/facts.json` into a
//! deterministic lexical graph `data/retrieval/lexical_graph.json`.
//!
//! Reads the v2.1+ facts artifact, delegates construction to
//! `adam_reasoning::graph::LexicalGraph::from_facts`, and writes the
//! resulting node + edge structure alongside summary statistics.
//!
//! **Zero new extraction**, zero new heuristics — the graph is a pure
//! projection. Regenerate whenever `facts.json` changes.

use std::{fs, path::Path, process::ExitCode};

use adam_reasoning::{Fact, graph::LexicalGraph};
use serde::{Deserialize, Serialize};

const FACTS_PATH: &str = "data/retrieval/facts.json";
const OUTPUT_PATH: &str = "data/retrieval/lexical_graph.json";

#[derive(Debug, Deserialize)]
struct FactsFile {
    facts: Vec<Fact>,
}

#[derive(Debug, Serialize)]
struct Artifact {
    version: String,
    built_from: String,
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

    let graph = LexicalGraph::from_facts(&facts_file.facts);
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

    let artifact = Artifact {
        version: env!("CARGO_PKG_VERSION").to_string(),
        built_from: FACTS_PATH.to_string(),
        summary: Summary {
            total_nodes: graph.nodes.len(),
            total_edges: graph.edges.len(),
            facts_ingested: graph.facts_ingested,
            most_connected,
        },
        graph,
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
