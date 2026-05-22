// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! # `intent_train_rung_a`
//!
//! Trains the Rung-A linear classifier on the dataset emitted by
//! `intent_dataset_build`. Splits the data 80/10/10 (train / dev
//! / test) stratified by intent label, trains with AdaGrad over a
//! configurable number of epochs, evaluates on dev each epoch for
//! early stopping, and saves the final model to
//! `data/intent_classifier/v1/rung_a.json`. The frozen test split
//! is **never seen** during training — see the design doc.
//!
//! Usage: `cargo run -p adam-intent-dataset --bin intent_train_rung_a`

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Instant;

use adam_intent_classifier::features::DEFAULT_BUCKET_COUNT;
use adam_intent_classifier::rung_a::Trainer;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct LabelledExample {
    #[allow(dead_code)]
    id: String,
    input: String,
    intent: String,
}

const DATASET_IN: &str = "data/intent_classifier/v1/dataset.jsonl";
const MODEL_OUT: &str = "data/intent_classifier/v1/rung_a.json";
const SPLIT_OUT: &str = "data/intent_classifier/v1/split.json";

const TRAIN_RATIO: f64 = 0.80;
const DEV_RATIO: f64 = 0.10;
// Remaining 0.10 is the frozen test split.

const MAX_EPOCHS: usize = 50;
const EARLY_STOP_PATIENCE: usize = 5;
const RNG_SEED: u64 = 0xc0de_d00d;

/// Simple deterministic LCG so the split is reproducible
/// without pulling in `rand`.
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load dataset.
    let raw = fs::read_to_string(DATASET_IN).map_err(|e| {
        format!("could not read {DATASET_IN} — run `intent_dataset_build` first ({e})")
    })?;
    let examples: Vec<LabelledExample> = raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    eprintln!("loaded {} examples", examples.len());

    // Build label inventory (deterministic alphabetical order).
    let mut label_set: Vec<String> = examples
        .iter()
        .map(|e| e.intent.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    label_set.sort();
    let label_idx: HashMap<String, usize> = label_set
        .iter()
        .enumerate()
        .map(|(i, l)| (l.clone(), i))
        .collect();
    eprintln!("labels: {} classes", label_set.len());

    // Group by class for stratified split.
    let mut by_class: HashMap<String, Vec<(String, usize)>> = HashMap::new();
    for ex in &examples {
        let idx = label_idx[&ex.intent];
        by_class
            .entry(ex.intent.clone())
            .or_default()
            .push((ex.input.clone(), idx));
    }

    // Stratified 80/10/10 split with deterministic LCG.
    let mut rng = Lcg(RNG_SEED);
    let mut train_set: Vec<(String, usize)> = Vec::new();
    let mut dev_set: Vec<(String, usize)> = Vec::new();
    let mut test_set: Vec<(String, usize)> = Vec::new();
    for (_label, mut items) in by_class {
        // Shuffle via deterministic LCG.
        for i in (1..items.len()).rev() {
            let j = (rng.next_f64() * (i as f64 + 1.0)) as usize;
            items.swap(i, j);
        }
        let n = items.len();
        let n_train = ((n as f64) * TRAIN_RATIO).round() as usize;
        let n_dev = ((n as f64) * DEV_RATIO).round() as usize;
        let n_train = n_train.min(n.saturating_sub(1)); // leave at least 1 for dev+test
        let n_dev = n_dev.min(n.saturating_sub(n_train + 1));
        train_set.extend(items.iter().take(n_train).cloned());
        dev_set.extend(items.iter().skip(n_train).take(n_dev).cloned());
        test_set.extend(items.iter().skip(n_train + n_dev).cloned());
    }
    eprintln!(
        "split: train={} dev={} test={}",
        train_set.len(),
        dev_set.len(),
        test_set.len()
    );

    // Save split provenance so the evaluator picks the same rows.
    let split = serde_json::json!({
        "rng_seed": RNG_SEED,
        "train_count": train_set.len(),
        "dev_count": dev_set.len(),
        "test_count": test_set.len(),
        "train_inputs": train_set.iter().map(|(i, _)| i).collect::<Vec<_>>(),
        "dev_inputs": dev_set.iter().map(|(i, _)| i).collect::<Vec<_>>(),
        "test_inputs": test_set.iter().map(|(i, _)| i).collect::<Vec<_>>(),
    });
    fs::write(SPLIT_OUT, serde_json::to_string_pretty(&split)?)?;

    // Train.
    let mut trainer = Trainer::new(label_set.clone(), DEFAULT_BUCKET_COUNT);
    let mut best_dev_acc = 0.0_f64;
    let mut best_epoch = 0usize;
    let mut epochs_since_best = 0usize;
    let mut best_weights = trainer.model.dense.clone();

    let start = Instant::now();
    for epoch in 1..=MAX_EPOCHS {
        // Shuffle train each epoch for stochastic ordering.
        for i in (1..train_set.len()).rev() {
            let j = (rng.next_f64() * (i as f64 + 1.0)) as usize;
            train_set.swap(i, j);
        }
        let mut epoch_loss = 0.0_f64;
        for (input, label) in &train_set {
            epoch_loss += trainer.step(input, *label) as f64;
        }
        epoch_loss /= train_set.len().max(1) as f64;
        let (dev_acc, dev_loss) = trainer.evaluate(&dev_set);
        eprintln!(
            "epoch {epoch:>2}  train_loss={epoch_loss:.4}  dev_loss={dev_loss:.4}  dev_acc={:.2}%",
            dev_acc * 100.0
        );
        if dev_acc > best_dev_acc + 1e-6 {
            best_dev_acc = dev_acc;
            best_epoch = epoch;
            best_weights = trainer.model.dense.clone();
            epochs_since_best = 0;
        } else {
            epochs_since_best += 1;
            if epochs_since_best >= EARLY_STOP_PATIENCE {
                eprintln!(
                    "early stop after epoch {epoch} (no dev improvement for {EARLY_STOP_PATIENCE} epochs)"
                );
                break;
            }
        }
    }
    let train_secs = start.elapsed().as_secs_f64();

    // Restore best-dev weights.
    trainer.model.dense = best_weights;

    // Final eval on dev + frozen test.
    let (dev_acc_final, dev_loss_final) = trainer.evaluate(&dev_set);
    let (test_acc_final, test_loss_final) = trainer.evaluate(&test_set);

    // Save model — compact dense weights into the sparse layout
    // so the on-disk artefact only carries non-zero buckets.
    let model_path = Path::new(MODEL_OUT);
    if let Some(parent) = model_path.parent() {
        fs::create_dir_all(parent)?;
    }
    trainer.model.compact_to_sparse();
    fs::write(model_path, serde_json::to_string(&trainer.model)?)?;
    let model_size_kb = fs::metadata(model_path)?.len() as f64 / 1024.0;

    eprintln!();
    eprintln!("=== Rung A training summary ===");
    eprintln!("classes:           {}", label_set.len());
    eprintln!("examples:          {}", examples.len());
    eprintln!(
        "split:             train={} dev={} test={}",
        train_set.len(),
        dev_set.len(),
        test_set.len()
    );
    eprintln!(
        "best dev epoch:    {best_epoch} (acc={:.2}%)",
        best_dev_acc * 100.0
    );
    eprintln!(
        "final dev:         acc={:.2}%  loss={dev_loss_final:.4}",
        dev_acc_final * 100.0
    );
    eprintln!(
        "FROZEN test:       acc={:.2}%  loss={test_loss_final:.4}",
        test_acc_final * 100.0
    );
    eprintln!("training time:     {train_secs:.1}s");
    eprintln!("model on disk:     {model_size_kb:.1} KB  ({MODEL_OUT})");

    Ok(())
}
