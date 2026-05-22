// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! # `intent_dataset_baseline`
//!
//! Reads the JSONL emitted by `intent_dataset_build` and measures
//! the **cascade's own** latency / consistency baseline.
//!
//! The cascade was used as the labelling oracle, so its accuracy
//! on the training set is by definition 100 %. What we report here
//! is **latency distribution** (so we know the budget the
//! classifier must beat) and **per-class throughput** (so the
//! classifier's eval harness has a comparison baseline).
//!
//! Output:
//!   - `data/intent_classifier/v1/baseline.json` — machine-readable
//!   - stderr — human-readable summary
//!
//! Usage: `cargo run -p adam-intent-dataset --bin intent_dataset_baseline`

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Instant;

use adam_dialog::Intent;
use adam_dialog::conversation::IntentKind;
use adam_kernel_fst::lexicon::LexiconV1;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct LabelledExample {
    #[allow(dead_code)]
    id: String,
    input: String,
    intent: String,
    #[allow(dead_code)]
    source_file: String,
    #[allow(dead_code)]
    confidence: String,
}

#[derive(Debug, Serialize)]
struct ClassMetrics {
    label: String,
    sample_count: usize,
    p50_ns: u128,
    p95_ns: u128,
    p99_ns: u128,
}

#[derive(Debug, Serialize)]
struct BaselineReport {
    dataset_path: String,
    total_examples: usize,
    overall_p50_ns: u128,
    overall_p95_ns: u128,
    overall_p99_ns: u128,
    overall_max_ns: u128,
    /// 100.0 — the cascade IS the oracle. Recorded explicitly so
    /// downstream comparisons don't accidentally treat the
    /// classifier's "matches the cascade" rate as truth.
    self_consistency_pct: f64,
    per_class: Vec<ClassMetrics>,
}

const DATASET_IN: &str = "data/intent_classifier/v1/dataset.jsonl";
const BASELINE_OUT: &str = "data/intent_classifier/v1/baseline.json";
const LEXICON_CURATED: &str = "data/tokenizer/segmentation_roots.json";
const LEXICON_APERTIUM: &str = "data/lexicon_v1/apertium_imported_roots.json";

fn percentile(sorted: &[u128], p: f64) -> u128 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let lexicon = LexiconV1::load(LEXICON_CURATED, LEXICON_APERTIUM)?;

    let raw = fs::read_to_string(DATASET_IN).map_err(|e| {
        format!("could not read {DATASET_IN} — run `intent_dataset_build` first ({e})")
    })?;
    let examples: Vec<LabelledExample> = raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;

    let mut overall_latencies: Vec<u128> = Vec::with_capacity(examples.len());
    let mut per_class: HashMap<String, Vec<u128>> = HashMap::new();
    let mut self_consistent = 0usize;

    for ex in &examples {
        let parses: Vec<adam_kernel_fst::parser::Analysis> = ex
            .input
            .split_whitespace()
            .flat_map(|tok| {
                let cleaned: String = tok
                    .chars()
                    .filter(|c| c.is_alphabetic() || c.is_ascii_digit() || *c == '-')
                    .flat_map(|c| c.to_lowercase())
                    .collect();
                if cleaned.is_empty() {
                    Vec::new()
                } else {
                    adam_kernel_fst::parser::analyse(&cleaned, &lexicon)
                }
            })
            .collect();

        let start = Instant::now();
        let intent: Intent =
            adam_dialog::semantics::interpret_text_with_lexicon(&ex.input, &parses, Some(&lexicon));
        let elapsed = start.elapsed().as_nanos();

        let kind: IntentKind = (&intent).into();
        let label = format!("{kind:?}");
        if label == ex.intent {
            self_consistent += 1;
        }
        overall_latencies.push(elapsed);
        per_class
            .entry(ex.intent.clone())
            .or_default()
            .push(elapsed);
    }

    overall_latencies.sort();
    let overall_p50 = percentile(&overall_latencies, 0.50);
    let overall_p95 = percentile(&overall_latencies, 0.95);
    let overall_p99 = percentile(&overall_latencies, 0.99);
    let overall_max = overall_latencies.last().copied().unwrap_or(0);
    let self_consistency_pct = 100.0 * self_consistent as f64 / examples.len().max(1) as f64;

    let mut per_class_metrics: Vec<ClassMetrics> = per_class
        .into_iter()
        .map(|(label, mut latencies)| {
            latencies.sort();
            let sample_count = latencies.len();
            ClassMetrics {
                label,
                sample_count,
                p50_ns: percentile(&latencies, 0.50),
                p95_ns: percentile(&latencies, 0.95),
                p99_ns: percentile(&latencies, 0.99),
            }
        })
        .collect();
    per_class_metrics.sort_by(|a, b| b.sample_count.cmp(&a.sample_count));

    let report = BaselineReport {
        dataset_path: DATASET_IN.to_string(),
        total_examples: examples.len(),
        overall_p50_ns: overall_p50,
        overall_p95_ns: overall_p95,
        overall_p99_ns: overall_p99,
        overall_max_ns: overall_max,
        self_consistency_pct,
        per_class: per_class_metrics,
    };

    let out_path = Path::new(BASELINE_OUT);
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(out_path, serde_json::to_string_pretty(&report)?)?;

    eprintln!("=== E1 cascade baseline ===");
    eprintln!("dataset:           {DATASET_IN}");
    eprintln!("total examples:    {}", report.total_examples);
    eprintln!(
        "self-consistency:  {:.2}% (expected 100% — the cascade IS the oracle)",
        report.self_consistency_pct
    );
    eprintln!(
        "latency  p50:      {:.1} µs",
        report.overall_p50_ns as f64 / 1_000.0
    );
    eprintln!(
        "latency  p95:      {:.1} µs",
        report.overall_p95_ns as f64 / 1_000.0
    );
    eprintln!(
        "latency  p99:      {:.1} µs",
        report.overall_p99_ns as f64 / 1_000.0
    );
    eprintln!(
        "latency  max:      {:.1} µs",
        report.overall_max_ns as f64 / 1_000.0
    );
    eprintln!("output:            {BASELINE_OUT}");
    eprintln!();
    eprintln!("--- per-class p99 latency (sorted by sample count) ---");
    for c in &report.per_class {
        eprintln!(
            "  {:30}  n={:5}  p50={:>7.1} µs  p99={:>7.1} µs",
            c.label,
            c.sample_count,
            c.p50_ns as f64 / 1_000.0,
            c.p99_ns as f64 / 1_000.0,
        );
    }

    Ok(())
}
