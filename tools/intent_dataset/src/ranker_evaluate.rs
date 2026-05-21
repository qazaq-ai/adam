// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! # `ranker_eval_rung_a`
//!
//! Evaluator for the E3 retrieval re-ranker. Loads the trained
//! artefact + the frozen test split, computes:
//!
//!   - **Pick-rate-at-1** vs cascade pick (cascade IS the oracle
//!     by dataset construction, so 1.000 is the upper bound).
//!   - **Mean Reciprocal Rank** of the cascade-picked candidate
//!     in the learned model's ranked list.
//!   - **Top-3 precision** (cascade-picked candidate is in
//!     learned top-3).
//!   - **Latency** distribution per ranking pass.
//!   - **Per-feature mean(positive) − mean(negative)** gap — a
//!     diagnostic of which features carry separation signal.
//!
//! Output: stderr summary +
//! `data/retrieval_ranker/v1/eval/rung_a_test.json` machine-
//! readable report.
//!
//! Usage: `cargo run -p adam-intent-dataset --bin ranker_eval_rung_a`

use std::fs;
use std::path::Path;
use std::time::Instant;

use adam_retrieval_ranker::CandidateFeatures as RF;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct CandidateRow {
    features: [f32; RF::N],
    picked: u8,
    #[allow(dead_code)]
    subject: String,
    #[allow(dead_code)]
    object: String,
}

#[derive(Debug, Deserialize)]
struct LabelledQuery {
    id: String,
    #[allow(dead_code)]
    query: String,
    candidates: Vec<CandidateRow>,
    picked_idx: usize,
}

#[derive(Debug, Deserialize)]
struct Artefact {
    schema_version: String,
    bias: f32,
    weights: [f32; RF::N],
}

#[derive(Debug, Serialize)]
struct EvalReport {
    test_count: usize,
    pick_rate_at_1: f64,
    mrr: f64,
    top3_precision: f64,
    p50_us: f64,
    p99_us: f64,
    per_feature_gap: Vec<(String, f64)>,
}

const MODEL_IN: &str = "data/retrieval_ranker/v1/rung_a.json";
const DATASET_IN: &str = "data/retrieval_ranker/v1/dataset.jsonl";
const SPLIT_IN: &str = "data/retrieval_ranker/v1/split.json";
const EVAL_OUT: &str = "data/retrieval_ranker/v1/eval/rung_a_test.json";

fn sigmoid(z: f32) -> f32 {
    1.0 / (1.0 + (-z).exp())
}

fn predict(art: &Artefact, x: &[f32; RF::N]) -> f32 {
    let mut z = art.bias;
    for (w, v) in art.weights.iter().zip(x.iter()) {
        z += w * v;
    }
    sigmoid(z)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model_raw = fs::read_to_string(MODEL_IN)?;
    let model: Artefact = serde_json::from_str(&model_raw)?;
    if !model.schema_version.starts_with("0.0.") {
        return Err(format!("unsupported schema {}", model.schema_version).into());
    }

    let split_raw = fs::read_to_string(SPLIT_IN)?;
    let split: serde_json::Value = serde_json::from_str(&split_raw)?;
    let test_ids: std::collections::HashSet<String> = split["test_ids"]
        .as_array()
        .ok_or("split.json missing test_ids")?
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    let dataset_raw = fs::read_to_string(DATASET_IN)?;
    let queries: Vec<LabelledQuery> = dataset_raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    let test_set: Vec<&LabelledQuery> = queries
        .iter()
        .filter(|q| test_ids.contains(&q.id))
        .collect();
    eprintln!("test rows: {}", test_set.len());

    let mut correct_at_1 = 0usize;
    let mut mrr_sum = 0.0_f64;
    let mut top3_hits = 0usize;
    let mut latencies_ns: Vec<u128> = Vec::with_capacity(test_set.len());

    // Per-feature mean(positive) - mean(negative) accumulators.
    let mut pos_sum = [0.0_f64; RF::N];
    let mut pos_count = 0usize;
    let mut neg_sum = [0.0_f64; RF::N];
    let mut neg_count = 0usize;

    for q in &test_set {
        let t = Instant::now();
        let scores: Vec<f32> = q
            .candidates
            .iter()
            .map(|c| predict(&model, &c.features))
            .collect();
        latencies_ns.push(t.elapsed().as_nanos());

        // Rank candidates by score DESC.
        let mut indexed: Vec<(usize, f32)> = scores.iter().copied().enumerate().collect();
        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Find rank of cascade pick.
        let pick_rank = indexed
            .iter()
            .position(|(i, _)| *i == q.picked_idx)
            .unwrap_or(indexed.len());
        if pick_rank == 0 {
            correct_at_1 += 1;
        }
        if pick_rank < 3 {
            top3_hits += 1;
        }
        mrr_sum += 1.0 / (pick_rank + 1) as f64;

        // Feature gap accumulation.
        for (i, cand) in q.candidates.iter().enumerate() {
            for k in 0..RF::N {
                if i == q.picked_idx {
                    pos_sum[k] += cand.features[k] as f64;
                } else {
                    neg_sum[k] += cand.features[k] as f64;
                }
            }
            if i == q.picked_idx {
                pos_count += 1;
            } else {
                neg_count += 1;
            }
        }
    }

    latencies_ns.sort();
    let n = test_set.len().max(1);
    let pick_rate = correct_at_1 as f64 / n as f64;
    let mrr = mrr_sum / n as f64;
    let top3 = top3_hits as f64 / n as f64;
    let percentile = |p: f64| -> f64 {
        if latencies_ns.is_empty() {
            return 0.0;
        }
        let idx = ((latencies_ns.len() - 1) as f64 * p).round() as usize;
        latencies_ns[idx] as f64 / 1_000.0
    };

    let names = [
        "confidence",
        "raw_text_richness",
        "subject_overlap",
        "object_overlap",
        "recency_match",
        "tfidf_cosine",
        "predicate_match",
        "isa_distance",
        "raw_len_norm",
        "cand_pos",
    ];
    let mut per_feature_gap: Vec<(String, f64)> = (0..RF::N)
        .map(|k| {
            let pmean = pos_sum[k] / pos_count.max(1) as f64;
            let nmean = neg_sum[k] / neg_count.max(1) as f64;
            (names[k].to_string(), pmean - nmean)
        })
        .collect();
    per_feature_gap.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let report = EvalReport {
        test_count: test_set.len(),
        pick_rate_at_1: pick_rate,
        mrr,
        top3_precision: top3,
        p50_us: percentile(0.50),
        p99_us: percentile(0.99),
        per_feature_gap: per_feature_gap.clone(),
    };
    let out_path = Path::new(EVAL_OUT);
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(out_path, serde_json::to_string_pretty(&report)?)?;

    eprintln!("=== E3 Rung A evaluation ===");
    eprintln!("test queries:    {}", report.test_count);
    eprintln!("pick-rate-at-1:  {:.4}", report.pick_rate_at_1);
    eprintln!("MRR:             {:.4}", report.mrr);
    eprintln!("top-3 precision: {:.4}", report.top3_precision);
    eprintln!(
        "latency:         p50={:.1}µs  p99={:.1}µs",
        report.p50_us, report.p99_us
    );
    eprintln!();
    eprintln!("--- per-feature mean(positive) − mean(negative) ---");
    for (name, gap) in &per_feature_gap {
        eprintln!("  {name:20} {gap:+.4}");
    }
    eprintln!();
    eprintln!("--- baseline comparison ---");
    eprintln!(
        "  cascade `default_v0` pick-rate@1: 1.0000 (by dataset construction — cascade IS the oracle)"
    );
    eprintln!(
        "  learned ranker pick-rate@1:       {:.4}",
        report.pick_rate_at_1
    );
    eprintln!(
        "  delta vs baseline:                {:+.4} pp",
        100.0 * (report.pick_rate_at_1 - 1.0)
    );
    eprintln!();
    eprintln!("output: {EVAL_OUT}");
    Ok(())
}
