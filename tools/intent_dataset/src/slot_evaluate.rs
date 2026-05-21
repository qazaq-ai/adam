// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! # `slot_eval_rung_a`
//!
//! Span-level evaluator for the E2 slot extractor. Runs the
//! trained model against three benches:
//!
//!   - The **frozen test split** from training (gazetteer-
//!     covered — same lexical vocab as the synth gazetteer).
//!     Reports the upper-bound performance.
//!   - The **held-out OOV gazetteer**
//!     (`data/slot_extractor/v1/holdout_examples.jsonl`) —
//!     hand-authored sentences using names / cities /
//!     occupations / ages **never seen during training**. This
//!     is the honest generalisation number.
//!   - The **cascade contingency** on the held-out set:
//!     for every row, run both the deterministic cascade and
//!     the neural extractor and report the 2 × 2 win / regress
//!     table the E2 design doc predeclared.
//!
//! Output: stderr summary + machine-readable
//! `data/slot_extractor/v1/eval/rung_a_test.json`.
//!
//! Usage: `cargo run -p adam-intent-dataset --bin slot_eval_rung_a`

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Instant;

use adam_dialog::Intent;
use adam_kernel_fst::lexicon::LexiconV1;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone)]
struct LabelledExample {
    #[allow(dead_code)]
    id: String,
    tokens: Vec<String>,
    tags: Vec<String>,
    #[serde(default)]
    #[allow(dead_code)]
    source_file: String,
}

#[derive(Debug, Deserialize)]
struct SlotModel {
    schema_version: String,
    bucket_count: usize,
    labels: Vec<String>,
    sparse_weights: Vec<(u32, Vec<f32>)>,
}

#[derive(Debug, Serialize, Clone)]
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
struct BenchReport {
    name: String,
    row_count: usize,
    per_slot: Vec<ClassMetrics>,
    micro_precision: f64,
    micro_recall: f64,
    micro_f1: f64,
    neural_p50_us: f64,
    neural_p99_us: f64,
    neural_max_us: f64,
}

#[derive(Debug, Serialize)]
struct Contingency {
    double_win: usize,
    neural_only: usize,
    cascade_only: usize,
    both_miss: usize,
    cascade_p99_us: f64,
}

#[derive(Debug, Serialize)]
struct FullReport {
    test_split: BenchReport,
    holdout: BenchReport,
    holdout_contingency_vs_cascade: Contingency,
}

const MODEL_IN: &str = "data/slot_extractor/v1/rung_a.json";
const DATASET_IN: &str = "data/slot_extractor/v1/dataset.jsonl";
const SPLIT_IN: &str = "data/slot_extractor/v1/split.json";
const HOLDOUT_IN: &str = "data/slot_extractor/v1/holdout_examples.jsonl";
const EVAL_OUT: &str = "data/slot_extractor/v1/eval/rung_a_test.json";
const LEXICON_CURATED: &str = "data/tokenizer/segmentation_roots.json";
const LEXICON_APERTIUM: &str = "data/lexicon_v1/apertium_imported_roots.json";

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

/// Compute the cascade's slot-extraction on a sentence and
/// return `(slot_type_slug, start, end_exclusive)` for every
/// span the cascade identifies. The cascade emits Intent::* with
/// a slot value; we locate the value in the token vector by
/// suffix-prefix containment.
fn cascade_spans(
    input: &str,
    tokens: &[String],
    lexicon: &LexiconV1,
) -> Vec<(String, usize, usize)> {
    let parses: Vec<adam_kernel_fst::parser::Analysis> = input
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
                adam_kernel_fst::parser::analyse(&cleaned, lexicon)
            }
        })
        .collect();
    let intent: Intent =
        adam_dialog::semantics::interpret_text_with_lexicon(input, &parses, Some(lexicon));

    fn locate(tokens: &[String], value: &str) -> Option<(usize, usize)> {
        let value_lower = value.to_lowercase();
        let value_tokens: Vec<&str> = value_lower.split_whitespace().collect();
        if value_tokens.is_empty() {
            return None;
        }
        let n = value_tokens.len();
        for start in 0..tokens.len().saturating_sub(n - 1) {
            if tokens[start..start + n]
                .iter()
                .zip(value_tokens.iter())
                .all(|(t, v)| t.eq_ignore_ascii_case(v))
            {
                return Some((start, start + n));
            }
        }
        // Single-token fallback — prefix containment.
        let head = value_tokens[0];
        for (i, tok) in tokens.iter().enumerate() {
            if tok.to_lowercase().starts_with(head) || head.starts_with(&tok.to_lowercase()) {
                return Some((i, i + 1));
            }
        }
        None
    }

    let mut out: Vec<(String, usize, usize)> = Vec::new();
    match &intent {
        Intent::StatementOfName { name } => {
            if let Some((s, e)) = locate(tokens, name) {
                out.push(("PER".into(), s, e));
            }
        }
        Intent::StatementOfAge { years } => {
            if let Some(y) = years {
                if let Some((s, e)) = locate(tokens, &y.to_string()) {
                    out.push(("AGE".into(), s, e));
                }
            }
        }
        Intent::StatementOfLocation { city } => {
            if let Some(c) = city {
                if let Some((s, e)) = locate(tokens, c) {
                    out.push(("LOC".into(), s, e));
                }
            }
        }
        Intent::StatementOfOccupation { occupation } => {
            if let Some(o) = occupation {
                if let Some((s, e)) = locate(tokens, o) {
                    out.push(("OCC".into(), s, e));
                }
            }
        }
        _ => {}
    }
    out
}

/// Per-bench evaluation: predict on each row, score per-slot
/// span-F1 + latency distribution.
fn evaluate_bench(
    name: &str,
    examples: &[&LabelledExample],
    model: &SlotModel,
    weights: &[f32],
) -> BenchReport {
    let mut per_slot_tp: HashMap<String, usize> = HashMap::new();
    let mut per_slot_fp: HashMap<String, usize> = HashMap::new();
    let mut per_slot_fn: HashMap<String, usize> = HashMap::new();
    let mut latencies_ns: Vec<u128> = Vec::with_capacity(examples.len());

    for ex in examples {
        let t = Instant::now();
        let pred_tags = predict(model, weights, &ex.tokens);
        latencies_ns.push(t.elapsed().as_nanos());

        let gold_spans: std::collections::HashSet<(String, usize, usize)> =
            decode_spans(&ex.tags).into_iter().collect();
        let pred_spans: std::collections::HashSet<(String, usize, usize)> =
            decode_spans(&pred_tags).into_iter().collect();

        for span in &pred_spans {
            if gold_spans.contains(span) {
                *per_slot_tp.entry(span.0.clone()).or_default() += 1;
            } else {
                *per_slot_fp.entry(span.0.clone()).or_default() += 1;
            }
        }
        for span in &gold_spans {
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

    BenchReport {
        name: name.to_string(),
        row_count: examples.len(),
        per_slot,
        micro_precision: micro_p,
        micro_recall: micro_r,
        micro_f1,
        neural_p50_us: percentile(0.50),
        neural_p99_us: percentile(0.99),
        neural_max_us: latencies_ns.last().copied().unwrap_or(0) as f64 / 1_000.0,
    }
}

fn print_bench(report: &BenchReport) {
    eprintln!("--- {} ({} rows) ---", report.name, report.row_count);
    eprintln!(
        "  micro:    P={:.3}  R={:.3}  F1={:.3}",
        report.micro_precision, report.micro_recall, report.micro_f1
    );
    eprintln!(
        "  latency:  p50={:.1}µs  p99={:.1}µs  max={:.1}µs",
        report.neural_p50_us, report.neural_p99_us, report.neural_max_us
    );
    for c in &report.per_slot {
        eprintln!(
            "    {:5}  TP={:>3}  FP={:>3}  FN={:>3}  P={:.3}  R={:.3}  F1={:.3}",
            c.slot, c.tp, c.fp, c.fn_, c.precision, c.recall, c.f1
        );
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raw = fs::read_to_string(MODEL_IN)?;
    let model: SlotModel = serde_json::from_str(&raw)?;
    if !model.schema_version.starts_with("0.0.") {
        return Err(format!("unsupported model schema {}", model.schema_version).into());
    }
    let weights = densify(&model);

    // --- Test split (gazetteer-covered, upper bound). ---
    let split_raw = fs::read_to_string(SPLIT_IN)?;
    let split: serde_json::Value = serde_json::from_str(&split_raw)?;
    let test_inputs: Vec<String> = split["test_inputs"]
        .as_array()
        .ok_or("split.json missing test_inputs")?
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();
    let dataset_raw = fs::read_to_string(DATASET_IN)?;
    let test_examples: Vec<LabelledExample> = dataset_raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    let by_input: HashMap<String, &LabelledExample> = test_examples
        .iter()
        .map(|e| (e.tokens.join(" "), e))
        .collect();
    let test_refs: Vec<&LabelledExample> = test_inputs
        .iter()
        .filter_map(|i| by_input.get(i).copied())
        .collect();
    let test_report = evaluate_bench(
        "test_split (gazetteer-covered)",
        &test_refs,
        &model,
        &weights,
    );

    // --- Holdout (OOV gazetteer, honest generalisation). ---
    let holdout_raw = fs::read_to_string(HOLDOUT_IN)?;
    let holdout_examples: Vec<LabelledExample> = holdout_raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    let holdout_refs: Vec<&LabelledExample> = holdout_examples.iter().collect();
    let holdout_report = evaluate_bench("holdout (OOV gazetteer)", &holdout_refs, &model, &weights);

    // --- Cascade contingency on holdout. ---
    let lexicon = LexiconV1::load(LEXICON_CURATED, LEXICON_APERTIUM)?;
    let mut win = 0;
    let mut neural_only = 0;
    let mut cascade_only = 0;
    let mut both_miss = 0;
    let mut cascade_latencies_ns: Vec<u128> = Vec::with_capacity(holdout_examples.len());
    for ex in &holdout_examples {
        let input = ex.tokens.join(" ");
        let gold_spans: std::collections::HashSet<(String, usize, usize)> =
            decode_spans(&ex.tags).into_iter().collect();
        let pred_neural: std::collections::HashSet<(String, usize, usize)> =
            decode_spans(&predict(&model, &weights, &ex.tokens))
                .into_iter()
                .collect();
        let t = Instant::now();
        let pred_cascade: std::collections::HashSet<(String, usize, usize)> =
            cascade_spans(&input, &ex.tokens, &lexicon)
                .into_iter()
                .collect();
        cascade_latencies_ns.push(t.elapsed().as_nanos());
        // Score by exact-span match.
        let neural_ok = pred_neural == gold_spans;
        let cascade_ok = pred_cascade == gold_spans;
        match (neural_ok, cascade_ok) {
            (true, true) => win += 1,
            (true, false) => neural_only += 1,
            (false, true) => cascade_only += 1,
            (false, false) => both_miss += 1,
        }
    }
    cascade_latencies_ns.sort();
    let cascade_p99_us = if cascade_latencies_ns.is_empty() {
        0.0
    } else {
        let idx = ((cascade_latencies_ns.len() - 1) as f64 * 0.99).round() as usize;
        cascade_latencies_ns[idx] as f64 / 1_000.0
    };

    let report = FullReport {
        test_split: test_report,
        holdout: holdout_report,
        holdout_contingency_vs_cascade: Contingency {
            double_win: win,
            neural_only,
            cascade_only,
            both_miss,
            cascade_p99_us,
        },
    };

    let out_path = Path::new(EVAL_OUT);
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(out_path, serde_json::to_string_pretty(&report)?)?;

    eprintln!("=== E2 Rung A — three-bench evaluation ===");
    eprintln!();
    print_bench(&report.test_split);
    eprintln!();
    print_bench(&report.holdout);
    eprintln!();
    eprintln!("--- Holdout contingency: neural vs cascade ---");
    eprintln!("                  cascade ✓     cascade ✗");
    eprintln!(
        "  neural ✓        {:>3} (double)  {:>3} (neural-only)",
        report.holdout_contingency_vs_cascade.double_win,
        report.holdout_contingency_vs_cascade.neural_only
    );
    eprintln!(
        "  neural ✗        {:>3} (regress)  {:>3} (both miss)",
        report.holdout_contingency_vs_cascade.cascade_only,
        report.holdout_contingency_vs_cascade.both_miss
    );
    eprintln!(
        "  cascade p99:    {:.1} µs",
        report.holdout_contingency_vs_cascade.cascade_p99_us
    );
    eprintln!();
    eprintln!("output: {EVAL_OUT}");
    Ok(())
}
