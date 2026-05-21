// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! # Rung A — linear classifier
//!
//! Sparse linear classifier over the hash-feature space defined
//! in [`crate::features`]. Pure Rust, zero new deps. AdaGrad-
//! based online training; softmax + argmax inference.
//!
//! ## Storage layout
//!
//! Weights are stored as `Vec<f32>` of length `bucket_count *
//! num_classes`. Index for `(bucket, class)` is
//! `bucket * num_classes + class`. This row-major layout keeps a
//! single bucket's per-class weights contiguous, which matters
//! during inference: the hot loop is "for each non-zero feature,
//! add its slice into the score vector". Cache-friendly.
//!
//! ## Why AdaGrad
//!
//! The feature space is sparse and irregular: hash buckets that
//! hold a single high-information trigram («қалайс») want a
//! large learning rate, while buckets with many low-information
//! n-grams want a small one. AdaGrad gives each bucket its own
//! step size proportional to `1 / sqrt(Σ gradient²)`, which is
//! exactly what we want for sparse text features.

use serde::{Deserialize, Serialize};

#[cfg(test)]
use crate::features::DEFAULT_BUCKET_COUNT;
use crate::features::extract;

const SCHEMA_VERSION: &str = "0.0.1";

/// The on-disk artefact for a trained Rung-A classifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RungAModel {
    pub schema_version: String,
    pub bucket_count: usize,
    pub labels: Vec<String>,
    pub weights: Vec<f32>,
}

impl RungAModel {
    /// Allocate a fresh model with zero weights.
    pub fn new_empty(labels: Vec<String>, bucket_count: usize) -> Self {
        let n_classes = labels.len();
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            bucket_count,
            labels,
            weights: vec![0.0; bucket_count * n_classes],
        }
    }

    /// Number of classes (label inventory size).
    pub fn num_classes(&self) -> usize {
        self.labels.len()
    }

    /// Look up the label index of a string, if known.
    pub fn label_index(&self, label: &str) -> Option<usize> {
        self.labels.iter().position(|l| l == label)
    }

    /// Run inference on `input`. Returns scores per class in
    /// label-index order. Caller can softmax + argmax these.
    pub fn score(&self, input: &str) -> Vec<f32> {
        let n_classes = self.num_classes();
        let mut scores = vec![0.0_f32; n_classes];
        for (bucket, value) in extract(input, self.bucket_count) {
            let base = (bucket as usize) * n_classes;
            for (cls, score) in scores.iter_mut().enumerate() {
                *score += value * self.weights[base + cls];
            }
        }
        scores
    }

    /// Softmax + argmax. Returns `(top_label_index, top_score,
    /// runners_up_indices_and_scores)`.
    pub fn predict(&self, input: &str) -> (usize, f32, Vec<(usize, f32)>) {
        let scores = self.score(input);
        let probs = softmax(&scores);
        let mut indexed: Vec<(usize, f32)> = probs.iter().copied().enumerate().collect();
        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let top = indexed[0];
        let runners_up = indexed.iter().skip(1).take(4).copied().collect();
        (top.0, top.1, runners_up)
    }
}

/// AdaGrad trainer state. Holds the model plus per-weight
/// gradient accumulator.
pub struct Trainer {
    pub model: RungAModel,
    /// Per-weight running sum of squared gradients. Same layout
    /// as `model.weights`.
    grad_sq: Vec<f32>,
    pub learning_rate: f32,
    /// L2 weight decay applied to every update.
    pub l2: f32,
    /// AdaGrad numerical stability constant.
    epsilon: f32,
}

impl Trainer {
    pub fn new(labels: Vec<String>, bucket_count: usize) -> Self {
        let model = RungAModel::new_empty(labels, bucket_count);
        let n_weights = model.weights.len();
        Self {
            model,
            grad_sq: vec![0.0; n_weights],
            learning_rate: 0.5,
            l2: 1e-6,
            epsilon: 1e-8,
        }
    }

    /// One training step on a single example.
    ///
    /// Returns the cross-entropy loss for the example so the
    /// caller can log progress.
    pub fn step(&mut self, input: &str, true_label_idx: usize) -> f32 {
        let n_classes = self.model.num_classes();
        let features = extract(input, self.model.bucket_count);

        // Forward.
        let mut scores = vec![0.0_f32; n_classes];
        for &(bucket, value) in &features {
            let base = (bucket as usize) * n_classes;
            for (cls, score) in scores.iter_mut().enumerate() {
                *score += value * self.model.weights[base + cls];
            }
        }
        let probs = softmax(&scores);
        let loss = -probs[true_label_idx].max(1e-9).ln();

        // Backward — for softmax + cross-entropy, gradient w.r.t.
        // each class logit is `(prob_class - one_hot_class)`.
        let mut grad_per_class = probs.clone();
        grad_per_class[true_label_idx] -= 1.0;

        // Apply per-feature update.
        for &(bucket, value) in &features {
            let base = (bucket as usize) * n_classes;
            for cls in 0..n_classes {
                let g = grad_per_class[cls] * value + self.l2 * self.model.weights[base + cls];
                self.grad_sq[base + cls] += g * g;
                let step =
                    self.learning_rate * g / (self.grad_sq[base + cls].sqrt() + self.epsilon);
                self.model.weights[base + cls] -= step;
            }
        }
        loss
    }

    /// Evaluate the model on a batch. Returns
    /// `(accuracy, avg_loss)`.
    pub fn evaluate(&self, examples: &[(String, usize)]) -> (f64, f64) {
        if examples.is_empty() {
            return (0.0, 0.0);
        }
        let mut correct = 0usize;
        let mut loss_sum = 0.0_f64;
        for (input, true_idx) in examples {
            let scores = self.model.score(input);
            let probs = softmax(&scores);
            let predicted = argmax(&probs);
            if predicted == *true_idx {
                correct += 1;
            }
            loss_sum += -probs[*true_idx].max(1e-9).ln() as f64;
        }
        let acc = correct as f64 / examples.len() as f64;
        (acc, loss_sum / examples.len() as f64)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn untrained_model_outputs_uniform_probs() {
        let m = RungAModel::new_empty(
            vec!["A".into(), "B".into(), "C".into()],
            DEFAULT_BUCKET_COUNT,
        );
        let scores = m.score("сәлем");
        // All weights are zero, so all logits are zero, so softmax
        // is uniform 1/3.
        for s in scores {
            assert!((s - 0.0).abs() < 1e-6);
        }
    }

    #[test]
    fn trainer_reduces_loss_on_single_example() {
        let mut t = Trainer::new(vec!["A".into(), "B".into()], DEFAULT_BUCKET_COUNT);
        let loss_before = t.step("сәлем", 0);
        let loss_after = t.step("сәлем", 0);
        // After one update for the same example, the loss must
        // strictly decrease (we trained on it).
        assert!(
            loss_after < loss_before,
            "loss did not decrease: before={loss_before} after={loss_after}"
        );
    }

    #[test]
    fn trainer_separates_two_classes_after_overfitting() {
        // Trivial separability check: train the model to memorise
        // two examples that map to different classes; the model
        // should reach 100 % accuracy on them.
        let mut t = Trainer::new(
            vec!["GreetingClass".into(), "FarewellClass".into()],
            DEFAULT_BUCKET_COUNT,
        );
        let train_set = vec![
            ("сәлем достар".to_string(), 0),
            ("сау бол досым".to_string(), 1),
        ];
        for _ in 0..50 {
            for (input, label) in &train_set {
                t.step(input, *label);
            }
        }
        let (acc, _) = t.evaluate(&train_set);
        assert!(
            (acc - 1.0).abs() < 1e-6,
            "expected 100% memorisation, got {acc}"
        );
    }

    #[test]
    fn predict_returns_top_and_runners_up() {
        let mut t = Trainer::new(
            vec!["A".into(), "B".into(), "C".into(), "D".into()],
            DEFAULT_BUCKET_COUNT,
        );
        for _ in 0..30 {
            t.step("alpha", 0);
            t.step("beta", 1);
        }
        let (top, top_score, runners_up) = t.model.predict("alpha");
        assert_eq!(top, 0, "expected class 0 (A) to win for 'alpha'");
        assert!(top_score > 0.25);
        assert!(!runners_up.is_empty());
    }
}
