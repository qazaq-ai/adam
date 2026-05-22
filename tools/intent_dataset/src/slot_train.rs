// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! # `slot_train_rung_a`
//!
//! Trains the Rung-A per-token classifier for the E2 slot
//! extractor. Reads `data/slot_extractor/v1/dataset.jsonl`,
//! stratified 80/10/10 split, AdaGrad-trained averaged
//! perceptron over hash features per token (token + char-3gram
//! + context window of ±1 neighbours + hand-rolled binary
//! signals).
//!
//! Output: `data/slot_extractor/v1/rung_a.json` — sparse JSON
//! artefact, same layout pattern as the E1 model.
//!
//! Usage: `cargo run -p adam-intent-dataset --bin slot_train_rung_a`

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
}

#[derive(Debug, Serialize)]
struct SlotModel {
    schema_version: String,
    bucket_count: usize,
    labels: Vec<String>,
    sparse_weights: Vec<(u32, Vec<f32>)>,
}

const DATASET_IN: &str = "data/slot_extractor/v1/dataset.jsonl";
const MODEL_OUT: &str = "data/slot_extractor/v1/rung_a.json";
const SPLIT_OUT: &str = "data/slot_extractor/v1/split.json";

const BUCKET_COUNT: usize = 32_768;
const MAX_EPOCHS: usize = 30;
const EARLY_STOP_PATIENCE: usize = 5;
const RNG_SEED: u64 = 0xc0de_d00d;
const TRAIN_RATIO: f64 = 0.80;
const DEV_RATIO: f64 = 0.10;

/// FNV-1a 32-bit hash.
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

/// Token-context features. Per-token output: list of bucket
/// indices that fire (binary feature presence; multi-hits sum to
/// `sqrt(count)`).
fn extract_features(tokens: &[String], i: usize) -> Vec<(u32, f32)> {
    use std::collections::HashMap;
    let mut counts: HashMap<u32, f32> = HashMap::new();
    let mut fire = |key: String| {
        let b = fnv1a_32(&key) % BUCKET_COUNT as u32;
        *counts.entry(b).or_insert(0.0) += 1.0;
    };
    let tok = &tokens[i];
    let lower = tok.to_lowercase();
    // Token unigram.
    fire(format!("tok:{lower}"));
    // Prefix / suffix.
    let chars: Vec<char> = lower.chars().collect();
    if chars.len() >= 2 {
        let p2: String = chars.iter().take(2).collect();
        fire(format!("pre2:{p2}"));
        let s2: String = chars
            .iter()
            .rev()
            .take(2)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        fire(format!("suf2:{s2}"));
    }
    if chars.len() >= 3 {
        let p3: String = chars.iter().take(3).collect();
        fire(format!("pre3:{p3}"));
        let s3: String = chars
            .iter()
            .rev()
            .take(3)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        fire(format!("suf3:{s3}"));
    }
    if chars.len() >= 4 {
        let p4: String = chars.iter().take(4).collect();
        fire(format!("pre4:{p4}"));
        let s4: String = chars
            .iter()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        fire(format!("suf4:{s4}"));
    }
    // Char trigrams with boundary markers.
    let mut bound: Vec<char> = Vec::with_capacity(chars.len() + 2);
    bound.push('^');
    bound.extend(chars.iter());
    bound.push('$');
    for w in bound.windows(3) {
        let tri: String = w.iter().collect();
        fire(format!("3g:{tri}"));
    }
    // Context window — previous / next token unigrams.
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
    // Binary signals.
    if tok.chars().next().is_some_and(|c| c.is_uppercase()) {
        fire("is:capitalised".to_string());
    }
    if tok.chars().all(|c| c.is_ascii_digit()) {
        fire("is:all-digit".to_string());
    }
    if tok.chars().any(|c| c.is_ascii_digit()) {
        fire("has:digit".to_string());
    }
    // Position-in-sentence.
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

struct Lcg(u64);
impl Lcg {
    fn next_f64(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn evaluate(
    weights: &[f32],
    n_classes: usize,
    examples: &[&LabelledExample],
    label_idx: &HashMap<String, usize>,
) -> (f64, f64) {
    if examples.is_empty() {
        return (0.0, 0.0);
    }
    let mut correct = 0usize;
    let mut total = 0usize;
    let mut loss_sum = 0.0_f64;
    for ex in examples {
        for (i, true_tag) in ex.tags.iter().enumerate() {
            let true_idx = match label_idx.get(true_tag) {
                Some(idx) => *idx,
                None => continue,
            };
            let mut scores = vec![0.0_f32; n_classes];
            for (bucket, value) in extract_features(&ex.tokens, i) {
                let base = (bucket as usize) * n_classes;
                for (cls, score) in scores.iter_mut().enumerate() {
                    *score += value * weights[base + cls];
                }
            }
            let probs = softmax(&scores);
            if argmax(&probs) == true_idx {
                correct += 1;
            }
            loss_sum += -probs[true_idx].max(1e-9).ln() as f64;
            total += 1;
        }
    }
    let acc = correct as f64 / total.max(1) as f64;
    (acc, loss_sum / total.max(1) as f64)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raw = fs::read_to_string(DATASET_IN)?;
    let examples: Vec<LabelledExample> = raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    eprintln!("loaded {} examples", examples.len());

    // Build label inventory (deterministic).
    let mut label_set: Vec<String> = examples
        .iter()
        .flat_map(|e| e.tags.iter().cloned())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    label_set.sort();
    let label_idx: HashMap<String, usize> = label_set
        .iter()
        .enumerate()
        .map(|(i, l)| (l.clone(), i))
        .collect();
    let n_classes = label_set.len();
    eprintln!("labels: {n_classes} ({:?})", label_set);

    // Stratified split by majority label of each example (the
    // most-frequent non-O tag, falling back to O when there's
    // no slot — though we already drop those at build time).
    let mut by_slot: HashMap<String, Vec<&LabelledExample>> = HashMap::new();
    for ex in &examples {
        let primary = ex
            .tags
            .iter()
            .find(|t| t.as_str() != "O")
            .cloned()
            .unwrap_or_else(|| "O".to_string());
        by_slot.entry(primary).or_default().push(ex);
    }

    let mut rng = Lcg(RNG_SEED);
    let mut train_set: Vec<&LabelledExample> = Vec::new();
    let mut dev_set: Vec<&LabelledExample> = Vec::new();
    let mut test_set: Vec<&LabelledExample> = Vec::new();
    for (_slot, mut items) in by_slot {
        for i in (1..items.len()).rev() {
            let j = (rng.next_f64() * (i as f64 + 1.0)) as usize;
            items.swap(i, j);
        }
        let n = items.len();
        let n_train = ((n as f64) * TRAIN_RATIO).round() as usize;
        let n_dev = ((n as f64) * DEV_RATIO).round() as usize;
        let n_train = n_train.min(n.saturating_sub(1));
        let n_dev = n_dev.min(n.saturating_sub(n_train + 1));
        train_set.extend(items.iter().take(n_train).copied());
        dev_set.extend(items.iter().skip(n_train).take(n_dev).copied());
        test_set.extend(items.iter().skip(n_train + n_dev).copied());
    }
    eprintln!(
        "split: train={} dev={} test={}",
        train_set.len(),
        dev_set.len(),
        test_set.len()
    );

    fs::write(
        SPLIT_OUT,
        serde_json::to_string_pretty(&serde_json::json!({
            "rng_seed": RNG_SEED,
            "train_count": train_set.len(),
            "dev_count": dev_set.len(),
            "test_count": test_set.len(),
            "train_inputs": train_set.iter().map(|e| e.tokens.join(" ")).collect::<Vec<_>>(),
            "dev_inputs": dev_set.iter().map(|e| e.tokens.join(" ")).collect::<Vec<_>>(),
            "test_inputs": test_set.iter().map(|e| e.tokens.join(" ")).collect::<Vec<_>>(),
        }))?,
    )?;

    // Train — AdaGrad on softmax cross-entropy, per-token.
    let total_weights = BUCKET_COUNT * n_classes;
    let mut weights = vec![0.0_f32; total_weights];
    let mut grad_sq = vec![0.0_f32; total_weights];
    const LR: f32 = 0.5;
    const L2: f32 = 1e-6;
    const EPS: f32 = 1e-8;

    let mut best_dev_acc = 0.0_f64;
    let mut best_epoch = 0usize;
    let mut epochs_since_best = 0usize;
    let mut best_weights = weights.clone();

    let start = Instant::now();
    for epoch in 1..=MAX_EPOCHS {
        // Shuffle sentence order.
        let mut order: Vec<usize> = (0..train_set.len()).collect();
        for i in (1..order.len()).rev() {
            let j = (rng.next_f64() * (i as f64 + 1.0)) as usize;
            order.swap(i, j);
        }
        let mut epoch_loss = 0.0_f64;
        let mut tok_count = 0usize;
        for &sent_i in &order {
            let ex = train_set[sent_i];
            for (i, true_tag) in ex.tags.iter().enumerate() {
                let true_idx = match label_idx.get(true_tag) {
                    Some(idx) => *idx,
                    None => continue,
                };
                let features = extract_features(&ex.tokens, i);
                let mut scores = vec![0.0_f32; n_classes];
                for &(bucket, value) in &features {
                    let base = (bucket as usize) * n_classes;
                    for (cls, score) in scores.iter_mut().enumerate() {
                        *score += value * weights[base + cls];
                    }
                }
                let probs = softmax(&scores);
                let loss = -probs[true_idx].max(1e-9).ln();
                epoch_loss += loss as f64;
                tok_count += 1;

                let mut grad_per_class = probs.clone();
                grad_per_class[true_idx] -= 1.0;

                for &(bucket, value) in &features {
                    let base = (bucket as usize) * n_classes;
                    for cls in 0..n_classes {
                        let g = grad_per_class[cls] * value + L2 * weights[base + cls];
                        grad_sq[base + cls] += g * g;
                        let step = LR * g / (grad_sq[base + cls].sqrt() + EPS);
                        weights[base + cls] -= step;
                    }
                }
            }
        }
        let avg_loss = epoch_loss / tok_count.max(1) as f64;
        let (dev_acc, dev_loss) = evaluate(&weights, n_classes, &dev_set, &label_idx);
        eprintln!(
            "epoch {epoch:>2}  train_loss={avg_loss:.4}  dev_loss={dev_loss:.4}  dev_tok_acc={:.2}%",
            dev_acc * 100.0
        );
        if dev_acc > best_dev_acc + 1e-6 {
            best_dev_acc = dev_acc;
            best_epoch = epoch;
            best_weights = weights.clone();
            epochs_since_best = 0;
        } else {
            epochs_since_best += 1;
            if epochs_since_best >= EARLY_STOP_PATIENCE {
                eprintln!("early stop after epoch {epoch}");
                break;
            }
        }
    }
    let train_secs = start.elapsed().as_secs_f64();

    // Restore best.
    weights = best_weights;
    let (dev_acc_final, _) = evaluate(&weights, n_classes, &dev_set, &label_idx);
    let (test_acc_final, _) = evaluate(&weights, n_classes, &test_set, &label_idx);

    // Compact to sparse and save.
    let mut sparse: Vec<(u32, Vec<f32>)> = Vec::new();
    for bucket in 0..BUCKET_COUNT {
        let base = bucket * n_classes;
        let slice = &weights[base..base + n_classes];
        if slice.iter().any(|w| *w != 0.0) {
            sparse.push((bucket as u32, slice.to_vec()));
        }
    }
    let model = SlotModel {
        schema_version: "0.0.1".to_string(),
        bucket_count: BUCKET_COUNT,
        labels: label_set.clone(),
        sparse_weights: sparse,
    };
    let model_path = Path::new(MODEL_OUT);
    if let Some(parent) = model_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(model_path, serde_json::to_string(&model)?)?;
    let model_size_kb = fs::metadata(model_path)?.len() as f64 / 1024.0;

    eprintln!();
    eprintln!("=== Rung A (slot) training summary ===");
    eprintln!("labels:           {n_classes}");
    eprintln!("examples:         {}", examples.len());
    eprintln!(
        "split:            train={} dev={} test={}",
        train_set.len(),
        dev_set.len(),
        test_set.len()
    );
    eprintln!(
        "best dev epoch:   {best_epoch} (tok_acc={:.2}%)",
        best_dev_acc * 100.0
    );
    eprintln!("final dev tok_acc: {:.2}%", dev_acc_final * 100.0);
    eprintln!("FROZEN test tok_acc: {:.2}%", test_acc_final * 100.0);
    eprintln!("training time:    {train_secs:.1}s");
    eprintln!("model on disk:    {model_size_kb:.1} KB  ({MODEL_OUT})");
    Ok(())
}
