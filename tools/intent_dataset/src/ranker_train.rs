// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! # `ranker_train_rung_a`
//!
//! Pointwise logistic-regression trainer for the E3 retrieval
//! re-ranker. Reads `data/retrieval_ranker/v1/dataset.jsonl`,
//! stratified 80/10/10 split by query (so all candidates from
//! one query stay in the same partition), trains a 10-weight
//! linear model with sigmoid + binary cross-entropy via
//! AdaGrad, and saves the artefact to
//! `data/retrieval_ranker/v1/rung_a.json`.
//!
//! Usage: `cargo run -p adam-intent-dataset --bin ranker_train_rung_a`

use std::fs;
use std::path::Path;
use std::time::Instant;

use adam_retrieval_ranker::CandidateFeatures as RF;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone)]
struct CandidateRow {
    features: [f32; RF::N],
    picked: u8,
    #[allow(dead_code)]
    subject: String,
    #[allow(dead_code)]
    object: String,
}

#[derive(Debug, Deserialize, Clone)]
struct LabelledQuery {
    id: String,
    query: String,
    candidates: Vec<CandidateRow>,
    #[allow(dead_code)]
    picked_idx: usize,
}

#[derive(Debug, Serialize)]
struct Artefact {
    schema_version: String,
    bias: f32,
    weights: [f32; RF::N],
}

const DATASET_IN: &str = "data/retrieval_ranker/v1/dataset.jsonl";
const MODEL_OUT: &str = "data/retrieval_ranker/v1/rung_a.json";
const SPLIT_OUT: &str = "data/retrieval_ranker/v1/split.json";

const MAX_EPOCHS: usize = 60;
const EARLY_STOP_PATIENCE: usize = 8;
const RNG_SEED: u64 = 0xc0de_d00d;
const TRAIN_RATIO: f64 = 0.80;
const DEV_RATIO: f64 = 0.10;

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

fn sigmoid(z: f32) -> f32 {
    1.0 / (1.0 + (-z).exp())
}

fn predict(weights: &[f32; RF::N], bias: f32, features: &[f32; RF::N]) -> f32 {
    let mut z = bias;
    for (w, x) in weights.iter().zip(features.iter()) {
        z += w * x;
    }
    sigmoid(z)
}

/// Pick-rate-at-1: fraction of queries where the learned
/// argmax matches the cascade-picked candidate. After the
/// dataset-builder shuffle, the cascade pick is at index
/// `q.picked_idx` (not always 0).
fn pick_rate_at_1(weights: &[f32; RF::N], bias: f32, queries: &[&LabelledQuery]) -> f64 {
    if queries.is_empty() {
        return 0.0;
    }
    let mut correct = 0usize;
    for q in queries {
        let mut best_idx = 0usize;
        let mut best_score = f32::NEG_INFINITY;
        for (i, cand) in q.candidates.iter().enumerate() {
            let s = predict(weights, bias, &cand.features);
            if s > best_score {
                best_score = s;
                best_idx = i;
            }
        }
        if best_idx == q.picked_idx {
            correct += 1;
        }
    }
    correct as f64 / queries.len() as f64
}

fn avg_loss(weights: &[f32; RF::N], bias: f32, queries: &[&LabelledQuery]) -> f64 {
    let mut total = 0.0_f64;
    let mut count = 0usize;
    for q in queries {
        for cand in &q.candidates {
            let p = predict(weights, bias, &cand.features);
            let y = cand.picked as f32;
            let l = -(y * p.max(1e-9).ln() + (1.0 - y) * (1.0 - p).max(1e-9).ln());
            total += l as f64;
            count += 1;
        }
    }
    total / count.max(1) as f64
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raw = fs::read_to_string(DATASET_IN)?;
    let queries: Vec<LabelledQuery> = raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    eprintln!(
        "loaded {} queries ({} candidate rows)",
        queries.len(),
        queries.iter().map(|q| q.candidates.len()).sum::<usize>()
    );

    // Stratified split by query (all candidates of one query
    // stay in the same partition).
    let mut rng = Lcg(RNG_SEED);
    let mut order: Vec<usize> = (0..queries.len()).collect();
    for i in (1..order.len()).rev() {
        let j = (rng.next_f64() * (i as f64 + 1.0)) as usize;
        order.swap(i, j);
    }
    let n_train = ((queries.len() as f64) * TRAIN_RATIO).round() as usize;
    let n_dev = ((queries.len() as f64) * DEV_RATIO).round() as usize;
    let train_idx: Vec<usize> = order.iter().take(n_train).copied().collect();
    let dev_idx: Vec<usize> = order.iter().skip(n_train).take(n_dev).copied().collect();
    let test_idx: Vec<usize> = order.iter().skip(n_train + n_dev).copied().collect();
    let train_set: Vec<&LabelledQuery> = train_idx.iter().map(|&i| &queries[i]).collect();
    let dev_set: Vec<&LabelledQuery> = dev_idx.iter().map(|&i| &queries[i]).collect();
    let test_set: Vec<&LabelledQuery> = test_idx.iter().map(|&i| &queries[i]).collect();
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
            "train_ids": train_set.iter().map(|q| &q.id).collect::<Vec<_>>(),
            "dev_ids": dev_set.iter().map(|q| &q.id).collect::<Vec<_>>(),
            "test_ids": test_set.iter().map(|q| &q.id).collect::<Vec<_>>(),
            "train_queries": train_set.iter().map(|q| &q.query).collect::<Vec<_>>(),
            "dev_queries": dev_set.iter().map(|q| &q.query).collect::<Vec<_>>(),
            "test_queries": test_set.iter().map(|q| &q.query).collect::<Vec<_>>(),
        }))?,
    )?;

    // AdaGrad logistic regression on flat (features, picked).
    let mut weights = [0.0_f32; RF::N];
    let mut bias = 0.0_f32;
    let mut grad_sq_w = [0.0_f32; RF::N];
    let mut grad_sq_b = 0.0_f32;
    const LR: f32 = 0.5;
    const L2: f32 = 1e-6;
    const EPS: f32 = 1e-8;

    let mut best_dev_pick = 0.0_f64;
    let mut best_epoch = 0usize;
    let mut since_best = 0usize;
    let mut best_w = weights;
    let mut best_b = bias;

    let start = Instant::now();
    for epoch in 1..=MAX_EPOCHS {
        // Build flat (features, picked) and shuffle.
        let mut flat: Vec<(usize, usize)> = Vec::new();
        for (qi, q) in train_set.iter().enumerate() {
            for ci in 0..q.candidates.len() {
                flat.push((qi, ci));
            }
        }
        for i in (1..flat.len()).rev() {
            let j = (rng.next_f64() * (i as f64 + 1.0)) as usize;
            flat.swap(i, j);
        }
        let mut epoch_loss = 0.0_f64;
        for (qi, ci) in flat.iter().copied() {
            let cand = &train_set[qi].candidates[ci];
            let x = cand.features;
            let y = cand.picked as f32;
            let p = predict(&weights, bias, &x);
            let err = p - y;
            epoch_loss += -(y * p.max(1e-9).ln() + (1.0 - y) * (1.0 - p).max(1e-9).ln()) as f64;
            // Gradient: d/dw = err * x; d/db = err.
            for i in 0..RF::N {
                let g = err * x[i] + L2 * weights[i];
                grad_sq_w[i] += g * g;
                weights[i] -= LR * g / (grad_sq_w[i].sqrt() + EPS);
            }
            grad_sq_b += err * err;
            bias -= LR * err / (grad_sq_b.sqrt() + EPS);
        }
        let dev_pick = pick_rate_at_1(&weights, bias, &dev_set);
        let dev_loss = avg_loss(&weights, bias, &dev_set);
        eprintln!(
            "epoch {epoch:>2}  train_loss={:.4}  dev_loss={dev_loss:.4}  dev_pick@1={:.4}",
            epoch_loss / flat.len().max(1) as f64,
            dev_pick
        );
        if dev_pick > best_dev_pick + 1e-6 {
            best_dev_pick = dev_pick;
            best_epoch = epoch;
            best_w = weights;
            best_b = bias;
            since_best = 0;
        } else {
            since_best += 1;
            if since_best >= EARLY_STOP_PATIENCE {
                eprintln!("early stop after epoch {epoch}");
                break;
            }
        }
    }
    let train_secs = start.elapsed().as_secs_f64();
    weights = best_w;
    bias = best_b;

    let test_pick = pick_rate_at_1(&weights, bias, &test_set);
    let dev_pick_final = pick_rate_at_1(&weights, bias, &dev_set);

    // Save artefact.
    let model = Artefact {
        schema_version: "0.0.1".to_string(),
        bias,
        weights,
    };
    let model_path = Path::new(MODEL_OUT);
    if let Some(parent) = model_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(model_path, serde_json::to_string_pretty(&model)?)?;
    let kb = fs::metadata(model_path)?.len() as f64 / 1024.0;

    eprintln!();
    eprintln!("=== Rung A (ranker) training summary ===");
    eprintln!("queries:           {}", queries.len());
    eprintln!(
        "split:             train={} dev={} test={}",
        train_set.len(),
        dev_set.len(),
        test_set.len()
    );
    eprintln!(
        "best dev pick@1:   {:.4} at epoch {best_epoch}",
        best_dev_pick
    );
    eprintln!("final dev pick@1:  {:.4}", dev_pick_final);
    eprintln!("FROZEN test pick@1: {:.4}", test_pick);
    eprintln!("training time:     {train_secs:.1}s");
    eprintln!("model on disk:     {kb:.2} KB  ({MODEL_OUT})");
    eprintln!();
    eprintln!("--- learned weights (canonical order) ---");
    eprintln!("  bias:                  {bias:+.4}");
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
    for (n, w) in names.iter().zip(weights.iter()) {
        eprintln!("  {n:20} {w:+.4}");
    }
    Ok(())
}
