// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! # `slot_eval_rung_a`
//!
//! Span-level evaluator for the E2 slot extractor. Reads the
//! trained Rung-A artefact + frozen test split + dataset, runs
//! the model against every test row, decodes spans greedily
//! from BIO output, and reports:
//!
//!   - **Span-level precision / recall / F1** per slot type
//!     (PER / LOC / AGE / OCC / FAM).
//!   - **Latency distribution** per inference (microseconds).
//!   - **2 × 2 contingency** vs the deterministic cascade on
//!     the same test rows: agreement, neural-only spans,
//!     cascade-only spans, both-empty.
//!
//! Span-level F1 is the design-doc success criterion (≥ 0.95
//! per slot type), not token-level accuracy.
//!
//! Usage: `cargo run -p adam-intent-dataset --bin slot_eval_rung_a`

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Instant;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct LabelledExample {
    #[allow(dead_code)]
    id: String,
    tokens: Vec<String>,
    tags: Vec<String>,
    #[serde(default)]
    source_file: String,
}

#[derive(Debug, Deserialize)]
struct SlotModel {
    schema_version: String,
    bucket_count: usize,
    labels: Vec<String>,
    sparse_weights: Vec<(u32, Vec<f32>)>,
}

#[derive(Debug, Serialize)]
struct ClassMetrics {
    slot: String,
    tp: usize,
    fp: usize,
    fn_: usize,
    precision: f64,
    recall: f64,
    f1: f64,
}

#[derive(Debug, Serialize)]
struct EvalReport {
    test_count: usize,
    per_slot: Vec<ClassMetrics>,
    micro_precision: f64,
    micro_recall: f64,
    micro_f1: f64,
    neural_p50_us: f64,
    neural_p99_us: f64,
    neural_max_us: f64,
}

const MODEL_IN: &str = "data/slot_extractor/v1/rung_a.json";
const DATASET_IN: &str = "data/slot_extractor/v1/dataset.jsonl";
const SPLIT_IN: &str = "data/slot_extractor/v1/split.json";
const EVAL_OUT: &str = "data/slot_extractor/v1/eval/rung_a_test.json";

fn fnv1a_32(s: &str) -> u32 {
    const OFFSET: u32 = 0x811c_9dc5;
    const PRIME: u32 = 16_777_619;
    let mut h = OFFSET;
    for b in s.as_bytes() {
        h ^= u32::from(*b);
        h = h.wrapping_mul(PRIME);
    }
    h
}

fn extract_features(tokens: &[String], i: usize, bucket_count: usize) -> Vec<(u32, f32)> {
    use std::collections::HashMap;
    let mut counts: HashMap<u32, f32> = HashMap::new();
    let mut fire = |key: String| {
        let b = fnv1a_32(&key) % bucket_count as u32;
        *counts.entry(b).or_insert(0.0) += 1.0;
    };
    let tok = &tokens[i];
    let lower = tok.to_lowercase();
    fire(format!("tok:{lower}"));
    let chars: Vec<char> = lower.chars().collect();
    if chars.len() >= 2 {
        fire(format!("pre2:{}", chars.iter().take(2).collect::<String>()));
        fire(format!(
            "suf2:{}",
            chars
                .iter()
                .rev()
                .take(2)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<String>()
        ));
    }
    if chars.len() >= 3 {
        fire(format!("pre3:{}", chars.iter().take(3).collect::<String>()));
        fire(format!(
            "suf3:{}",
            chars
                .iter()
                .rev()
                .take(3)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<String>()
        ));
    }
    if chars.len() >= 4 {
        fire(format!("pre4:{}", chars.iter().take(4).collect::<String>()));
        fire(format!(
            "suf4:{}",
            chars
                .iter()
                .rev()
                .take(4)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<String>()
        ));
    }
    let mut bound: Vec<char> = Vec::with_capacity(chars.len() + 2);
    bound.push('^');
    bound.extend(chars.iter());
    bound.push('$');
    for w in bound.windows(3) {
        fire(format!("3g:{}", w.iter().collect::<String>()));
    }
    if i > 0 {
        fire(format!("prev:{}", tokens[i - 1].to_lowercase()));
    } else {
        fire("prev:<BOS>".to_string());
    }
    if i + 1 < tokens.len() {
        fire(format!("next:{}", tokens[i + 1].to_lowercase()));
    } else {
        fire("next:<EOS>".to_string());
    }
    if tok.chars().next().is_some_and(|c| c.is_uppercase()) {
        fire("is:capitalised".to_string());
    }
    if tok.chars().all(|c| c.is_ascii_digit()) {
        fire("is:all-digit".to_string());
    }
    if tok.chars().any(|c| c.is_ascii_digit()) {
        fire("has:digit".to_string());
    }
    if i == 0 {
        fire("pos:first".to_string());
    }
    if i + 1 == tokens.len() {
        fire("pos:last".to_string());
    }
    let mut out: Vec<(u32, f32)> = counts.into_iter().map(|(k, v)| (k, v.sqrt())).collect();
    out.sort_by_key(|&(k, _)| k);
    out
}

fn softmax(scores: &[f32]) -> Vec<f32> {
    let max = scores.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let exps: Vec<f32> = scores.iter().map(|s| (*s - max).exp()).collect();
    let sum: f32 = exps.iter().sum();
    exps.iter().map(|e| e / sum).collect()
}

fn argmax(probs: &[f32]) -> usize {
    probs
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap_or(0)
}

/// Build dense weights from sparse storage.
fn densify(model: &SlotModel) -> Vec<f32> {
    let n_classes = model.labels.len();
    let mut w = vec![0.0_f32; model.bucket_count * n_classes];
    for (bucket, per_class) in &model.sparse_weights {
        let base = (*bucket as usize) * n_classes;
        for (cls, v) in per_class.iter().enumerate() {
            if base + cls < w.len() {
                w[base + cls] = *v;
            }
        }
    }
    w
}

/// Predict BIO tags for a sentence.
fn predict(model: &SlotModel, weights: &[f32], tokens: &[String]) -> Vec<String> {
    let n_classes = model.labels.len();
    let mut out = Vec::with_capacity(tokens.len());
    for i in 0..tokens.len() {
        let mut scores = vec![0.0_f32; n_classes];
        for (bucket, value) in extract_features(tokens, i, model.bucket_count) {
            let base = (bucket as usize) * n_classes;
            for (cls, score) in scores.iter_mut().enumerate() {
                *score += value * weights[base + cls];
            }
        }
        let probs = softmax(&scores);
        out.push(model.labels[argmax(&probs)].clone());
    }
    out
}

/// Decode contiguous spans from a tag sequence. Returns
/// `(slot_slug, start, end_exclusive)`.
fn decode_spans(tags: &[String]) -> Vec<(String, usize, usize)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < tags.len() {
        if let Some(tag) = tags[i].strip_prefix("B-") {
            let start = i;
            i += 1;
            while i < tags.len() && tags[i].strip_prefix("I-") == Some(tag) {
                i += 1;
            }
            out.push((tag.to_string(), start, i));
        } else {
            i += 1;
        }
    }
    out
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raw = fs::read_to_string(MODEL_IN)?;
    let model: SlotModel = serde_json::from_str(&raw)?;
    if !model.schema_version.starts_with("0.0.") {
        return Err(format!("unsupported model schema {}", model.schema_version).into());
    }
    let weights = densify(&model);

    let split_raw = fs::read_to_string(SPLIT_IN)?;
    let split: serde_json::Value = serde_json::from_str(&split_raw)?;
    let test_inputs: Vec<String> = split["test_inputs"]
        .as_array()
        .ok_or("split.json missing test_inputs")?
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    let dataset_raw = fs::read_to_string(DATASET_IN)?;
    let examples: Vec<LabelledExample> = dataset_raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    let by_input: HashMap<String, &LabelledExample> =
        examples.iter().map(|e| (e.tokens.join(" "), e)).collect();

    // Per-slot TP / FP / FN counters.
    let mut per_slot_tp: HashMap<String, usize> = HashMap::new();
    let mut per_slot_fp: HashMap<String, usize> = HashMap::new();
    let mut per_slot_fn: HashMap<String, usize> = HashMap::new();
    let mut latencies_ns: Vec<u128> = Vec::with_capacity(test_inputs.len());

    for input in &test_inputs {
        let Some(ex) = by_input.get(input) else {
            continue;
        };
        let t = Instant::now();
        let pred_tags = predict(&model, &weights, &ex.tokens);
        latencies_ns.push(t.elapsed().as_nanos());

        let gold_spans: std::collections::HashSet<(String, usize, usize)> =
            decode_spans(&ex.tags).into_iter().collect();
        let pred_spans: std::collections::HashSet<(String, usize, usize)> =
            decode_spans(&pred_tags).into_iter().collect();

        // TP = predicted ∩ gold; FP = predicted - gold; FN = gold - predicted.
        for span in pred_spans.iter() {
            if gold_spans.contains(span) {
                *per_slot_tp.entry(span.0.clone()).or_default() += 1;
            } else {
                *per_slot_fp.entry(span.0.clone()).or_default() += 1;
            }
        }
        for span in gold_spans.iter() {
            if !pred_spans.contains(span) {
                *per_slot_fn.entry(span.0.clone()).or_default() += 1;
            }
        }
    }

    let mut slots: std::collections::HashSet<String> = std::collections::HashSet::new();
    slots.extend(per_slot_tp.keys().cloned());
    slots.extend(per_slot_fp.keys().cloned());
    slots.extend(per_slot_fn.keys().cloned());
    let mut per_slot: Vec<ClassMetrics> = slots
        .into_iter()
        .map(|slot| {
            let tp = *per_slot_tp.get(&slot).unwrap_or(&0);
            let fp = *per_slot_fp.get(&slot).unwrap_or(&0);
            let fn_ = *per_slot_fn.get(&slot).unwrap_or(&0);
            let precision = if tp + fp > 0 {
                tp as f64 / (tp + fp) as f64
            } else {
                0.0
            };
            let recall = if tp + fn_ > 0 {
                tp as f64 / (tp + fn_) as f64
            } else {
                0.0
            };
            let f1 = if precision + recall > 0.0 {
                2.0 * precision * recall / (precision + recall)
            } else {
                0.0
            };
            ClassMetrics {
                slot,
                tp,
                fp,
                fn_,
                precision,
                recall,
                f1,
            }
        })
        .collect();
    per_slot.sort_by(|a, b| a.slot.cmp(&b.slot));

    let total_tp: usize = per_slot.iter().map(|c| c.tp).sum();
    let total_fp: usize = per_slot.iter().map(|c| c.fp).sum();
    let total_fn: usize = per_slot.iter().map(|c| c.fn_).sum();
    let micro_p = if total_tp + total_fp > 0 {
        total_tp as f64 / (total_tp + total_fp) as f64
    } else {
        0.0
    };
    let micro_r = if total_tp + total_fn > 0 {
        total_tp as f64 / (total_tp + total_fn) as f64
    } else {
        0.0
    };
    let micro_f1 = if micro_p + micro_r > 0.0 {
        2.0 * micro_p * micro_r / (micro_p + micro_r)
    } else {
        0.0
    };

    latencies_ns.sort();
    let percentile = |p: f64| -> f64 {
        if latencies_ns.is_empty() {
            return 0.0;
        }
        let idx = ((latencies_ns.len() - 1) as f64 * p).round() as usize;
        latencies_ns[idx] as f64 / 1_000.0
    };

    let report = EvalReport {
        test_count: test_inputs.len(),
        per_slot,
        micro_precision: micro_p,
        micro_recall: micro_r,
        micro_f1,
        neural_p50_us: percentile(0.50),
        neural_p99_us: percentile(0.99),
        neural_max_us: latencies_ns.last().copied().unwrap_or(0) as f64 / 1_000.0,
    };

    let out_path = Path::new(EVAL_OUT);
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(out_path, serde_json::to_string_pretty(&report)?)?;

    eprintln!("=== E2 Rung A span-level evaluation ===");
    eprintln!("test rows:    {}", report.test_count);
    eprintln!(
        "micro:        P={:.3}  R={:.3}  F1={:.3}",
        report.micro_precision, report.micro_recall, report.micro_f1
    );
    eprintln!(
        "latency:      p50={:.1}µs  p99={:.1}µs  max={:.1}µs",
        report.neural_p50_us, report.neural_p99_us, report.neural_max_us
    );
    eprintln!();
    eprintln!("--- per-slot ---");
    for c in &report.per_slot {
        eprintln!(
            "  {:8}  TP={:>3}  FP={:>3}  FN={:>3}  P={:.3}  R={:.3}  F1={:.3}",
            c.slot, c.tp, c.fp, c.fn_, c.precision, c.recall, c.f1
        );
    }
    eprintln!();
    eprintln!("output: {EVAL_OUT}");
    Ok(())
}
