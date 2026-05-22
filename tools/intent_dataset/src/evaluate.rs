// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! # `intent_eval_rung_a`
//!
//! Runs the trained Rung-A classifier against the frozen test
//! split produced by `intent_train_rung_a` and emits the
//! 2 × 2 contingency table specified in the design doc:
//!
//! | cascade ✓ | cascade ✗
//! ----------------------------------|-----------|----------
//! **neural ✓** | double win       | improve
//! **neural ✗** | neural regression | shared blind spot
//!
//! Also reports per-class precision / recall and overall latency
//! distribution. Output: stderr human summary +
//! `data/intent_classifier/v1/eval/rung_a_test.json` machine
//! report.
//!
//! Usage: `cargo run -p adam-intent-dataset --bin intent_eval_rung_a`

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Instant;

use adam_dialog::Intent;
use adam_dialog::conversation::IntentKind;
use adam_intent_classifier::Classifier;
use adam_kernel_fst::lexicon::LexiconV1;
use serde::Serialize;

const MODEL_IN: &str = "data/intent_classifier/v1/rung_a.json";
const SPLIT_IN: &str = "data/intent_classifier/v1/split.json";
const EVAL_OUT: &str = "data/intent_classifier/v1/eval/rung_a_test.json";
const LEXICON_CURATED: &str = "data/tokenizer/segmentation_roots.json";
const LEXICON_APERTIUM: &str = "data/lexicon_v1/apertium_imported_roots.json";

#[derive(Debug, Serialize)]
struct EvalReport {
    classes: usize,
    test_count: usize,
    overall_neural_acc_pct: f64,
    /// 100 by construction (cascade is the oracle), but reported
    /// for symmetry and to surface any sampling bug.
    overall_cascade_acc_pct: f64,
    contingency_double_win: usize,
    contingency_neural_regression: usize,
    contingency_neural_improve: usize,
    contingency_shared_blind_spot: usize,
    neural_p50_us: f64,
    neural_p95_us: f64,
    neural_p99_us: f64,
    neural_max_us: f64,
    cascade_p50_us: f64,
    cascade_p99_us: f64,
    per_class_precision: HashMap<String, f64>,
    per_class_recall: HashMap<String, f64>,
}

fn percentile_us(sorted_ns: &[u128], p: f64) -> f64 {
    if sorted_ns.is_empty() {
        return 0.0;
    }
    let idx = ((sorted_ns.len() - 1) as f64 * p).round() as usize;
    sorted_ns[idx] as f64 / 1_000.0
}

fn cascade_label(input: &str, lexicon: &LexiconV1) -> String {
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
    match &intent {
        Intent::Unknown { noun_hint, .. } if noun_hint.is_some() => "FactualQuery".to_string(),
        _ => {
            let kind: IntentKind = (&intent).into();
            format!("{kind:?}")
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let classifier = Classifier::from_path(MODEL_IN)?;
    let lexicon = LexiconV1::load(LEXICON_CURATED, LEXICON_APERTIUM)?;

    let split: serde_json::Value = serde_json::from_str(&fs::read_to_string(SPLIT_IN)?)?;
    let test_inputs: Vec<String> = split["test_inputs"]
        .as_array()
        .ok_or("split.json missing test_inputs")?
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    // Reload the dataset to recover (input, true_label) pairs for
    // the test split. We map input → true_label using the dataset.
    let dataset_raw = fs::read_to_string("data/intent_classifier/v1/dataset.jsonl")?;
    let mut input_to_label: HashMap<String, String> = HashMap::new();
    for line in dataset_raw.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(line)?;
        let input = v["input"].as_str().unwrap_or("").to_string();
        let intent = v["intent"].as_str().unwrap_or("").to_string();
        input_to_label.insert(input, intent);
    }

    let mut neural_latencies_ns: Vec<u128> = Vec::with_capacity(test_inputs.len());
    let mut cascade_latencies_ns: Vec<u128> = Vec::with_capacity(test_inputs.len());
    let (mut win, mut neural_only, mut cascade_only, mut both_miss) = (0, 0, 0, 0);
    let mut neural_correct = 0usize;
    let mut cascade_correct = 0usize;
    let mut per_class_tp: HashMap<String, usize> = HashMap::new();
    let mut per_class_fp: HashMap<String, usize> = HashMap::new();
    let mut per_class_fn_: HashMap<String, usize> = HashMap::new();

    for input in &test_inputs {
        let truth = match input_to_label.get(input) {
            Some(t) => t,
            None => continue,
        };

        let t_n = Instant::now();
        let neural_out = classifier.classify(input)?;
        let neural_label = neural_out.top.as_str().to_string();
        neural_latencies_ns.push(t_n.elapsed().as_nanos());

        let t_c = Instant::now();
        let cascade_label_ = cascade_label(input, &lexicon);
        cascade_latencies_ns.push(t_c.elapsed().as_nanos());

        let neural_ok = neural_label == *truth;
        let cascade_ok = cascade_label_ == *truth;
        if neural_ok {
            neural_correct += 1;
        }
        if cascade_ok {
            cascade_correct += 1;
        }
        match (neural_ok, cascade_ok) {
            (true, true) => win += 1,
            (true, false) => neural_only += 1,
            (false, true) => cascade_only += 1,
            (false, false) => both_miss += 1,
        }
        // Per-class precision/recall against truth.
        if neural_label == *truth {
            *per_class_tp.entry(truth.clone()).or_default() += 1;
        } else {
            *per_class_fp.entry(neural_label.clone()).or_default() += 1;
            *per_class_fn_.entry(truth.clone()).or_default() += 1;
        }
    }

    let n = test_inputs.len().max(1);
    neural_latencies_ns.sort();
    cascade_latencies_ns.sort();

    let mut per_class_precision: HashMap<String, f64> = HashMap::new();
    let mut per_class_recall: HashMap<String, f64> = HashMap::new();
    for label in per_class_tp
        .keys()
        .chain(per_class_fp.keys())
        .chain(per_class_fn_.keys())
    {
        let tp = *per_class_tp.get(label).unwrap_or(&0) as f64;
        let fp = *per_class_fp.get(label).unwrap_or(&0) as f64;
        let fn_ = *per_class_fn_.get(label).unwrap_or(&0) as f64;
        let precision = if tp + fp > 0.0 { tp / (tp + fp) } else { 0.0 };
        let recall = if tp + fn_ > 0.0 { tp / (tp + fn_) } else { 0.0 };
        per_class_precision.insert(label.clone(), precision);
        per_class_recall.insert(label.clone(), recall);
    }

    let report = EvalReport {
        classes: classifier.labels().len(),
        test_count: test_inputs.len(),
        overall_neural_acc_pct: 100.0 * neural_correct as f64 / n as f64,
        overall_cascade_acc_pct: 100.0 * cascade_correct as f64 / n as f64,
        contingency_double_win: win,
        contingency_neural_regression: cascade_only,
        contingency_neural_improve: neural_only,
        contingency_shared_blind_spot: both_miss,
        neural_p50_us: percentile_us(&neural_latencies_ns, 0.50),
        neural_p95_us: percentile_us(&neural_latencies_ns, 0.95),
        neural_p99_us: percentile_us(&neural_latencies_ns, 0.99),
        neural_max_us: neural_latencies_ns.last().copied().unwrap_or(0) as f64 / 1_000.0,
        cascade_p50_us: percentile_us(&cascade_latencies_ns, 0.50),
        cascade_p99_us: percentile_us(&cascade_latencies_ns, 0.99),
        per_class_precision,
        per_class_recall,
    };

    let out_path = Path::new(EVAL_OUT);
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(out_path, serde_json::to_string_pretty(&report)?)?;

    eprintln!("=== Rung A test-set evaluation ===");
    eprintln!("classes:                   {}", report.classes);
    eprintln!("test examples:             {}", report.test_count);
    eprintln!(
        "neural accuracy:           {:.2}%",
        report.overall_neural_acc_pct
    );
    eprintln!(
        "cascade accuracy:          {:.2}%  (oracle baseline)",
        report.overall_cascade_acc_pct
    );
    eprintln!();
    eprintln!("--- 2x2 contingency ---");
    eprintln!("                cascade ✓     cascade ✗");
    eprintln!(
        "  neural ✓     {:>5} (double)  {:>5} (improve)",
        win, neural_only
    );
    eprintln!(
        "  neural ✗     {:>5} (regress)  {:>5} (both miss)",
        cascade_only, both_miss
    );
    eprintln!();
    eprintln!("--- latency comparison ---");
    eprintln!(
        "neural   p50={:.1}µs  p95={:.1}µs  p99={:.1}µs  max={:.1}µs",
        report.neural_p50_us, report.neural_p95_us, report.neural_p99_us, report.neural_max_us
    );
    eprintln!(
        "cascade  p50={:.1}µs  p99={:.1}µs",
        report.cascade_p50_us, report.cascade_p99_us
    );
    eprintln!();
    let mut classes_sorted: Vec<&String> = report.per_class_precision.keys().collect();
    classes_sorted.sort();
    eprintln!("--- per-class precision / recall ---");
    for label in classes_sorted {
        let p = report
            .per_class_precision
            .get(label)
            .copied()
            .unwrap_or(0.0);
        let r = report.per_class_recall.get(label).copied().unwrap_or(0.0);
        eprintln!("  {label:30} P={:.2}  R={:.2}", p, r);
    }
    eprintln!();
    eprintln!("output: {EVAL_OUT}");

    Ok(())
}
