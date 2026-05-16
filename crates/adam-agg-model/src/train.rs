//! Training loop for the tiny agglutinative transformer.
//!
//! Phase 0 proof: feed it synthetic morpheme sequences from
//! [`adam_agg_synth`], optimise next-token prediction loss, show
//! that loss decreases. Pure Rust, CPU, no Python.

use burn::module::AutodiffModule;
use burn::optim::adaptor::OptimizerAdaptor;
use burn::optim::{Adam, AdamConfig, GradientsParams, Optimizer};
use burn::prelude::*;
use burn::tensor::backend::AutodiffBackend;

use crate::{TinyAgt, TinyAgtConfig};

/// Configuration for a tiny training run.
pub struct TrainConfig {
    pub batch_size: usize,
    pub n_epochs: usize,
    pub lr: f64,
    pub seed: u64,
}

impl Default for TrainConfig {
    fn default() -> Self {
        Self {
            batch_size: 16,
            n_epochs: 5,
            lr: 1e-3,
            seed: 42,
        }
    }
}

/// One training step result: pre-update loss value.
#[derive(Debug, Clone, Copy)]
pub struct StepReport {
    pub epoch: usize,
    pub step: usize,
    pub loss: f32,
}

/// Train the model on a fixed dataset of token sequences. Each
/// sequence is converted to (input, target) by shifting one token
/// (next-token prediction). Returns a report of loss per step so
/// the caller can plot / assert that loss decreases.
pub fn train_next_token<B: AutodiffBackend>(
    model: TinyAgt<B>,
    sequences: &[Vec<i64>],
    cfg: &TrainConfig,
    device: &B::Device,
) -> (TinyAgt<B>, Vec<StepReport>)
where
    B::InnerBackend: Backend,
{
    let mut optim: OptimizerAdaptor<Adam, TinyAgt<B>, B> = AdamConfig::new().init();
    let mut model = model;
    let mut reports = Vec::new();

    let max_seq_len = model.max_seq_len();

    for epoch in 0..cfg.n_epochs {
        for (step, chunk) in sequences.chunks(cfg.batch_size).enumerate() {
            // Build [batch, seq_len] tensors. Pad/truncate to max_seq_len.
            let batch_size = chunk.len();
            let mut input_data = vec![0i64; batch_size * max_seq_len];
            let mut target_data = vec![0i64; batch_size * max_seq_len];
            for (i, seq) in chunk.iter().enumerate() {
                let take = seq.len().min(max_seq_len);
                // Input is the seq[..take-1]; target is seq[1..take].
                // For positions beyond `take-1`, leave 0 (Pad).
                if take == 0 {
                    continue;
                }
                for j in 0..take - 1 {
                    input_data[i * max_seq_len + j] = seq[j];
                    target_data[i * max_seq_len + j] = seq[j + 1];
                }
            }
            let input: Tensor<B, 2, Int> = Tensor::from_data(
                burn::tensor::TensorData::new(input_data, [batch_size, max_seq_len]),
                device,
            );
            let target: Tensor<B, 2, Int> = Tensor::from_data(
                burn::tensor::TensorData::new(target_data, [batch_size, max_seq_len]),
                device,
            );

            // Forward + loss.
            let logits = model.forward(input);
            let loss = model.loss(logits, target);
            let loss_value = loss.clone().into_scalar();

            reports.push(StepReport {
                epoch,
                step,
                loss: loss_value.elem::<f32>(),
            });

            // Backward + optimise.
            let grads = loss.backward();
            let grads = GradientsParams::from_grads(grads, &model);
            model = optim.step(cfg.lr, model, grads);
        }
    }

    (model, reports)
}

#[cfg(test)]
mod tests {
    use super::*;
    use burn::backend::Autodiff;
    use burn::backend::ndarray::{NdArray, NdArrayDevice};

    type Backend = Autodiff<NdArray<f32>>;

    fn synthetic_sequences() -> Vec<Vec<i64>> {
        // 8 deterministic short sequences. Repeat a pattern; the model
        // should learn it after a few epochs.
        vec![
            vec![1, 2, 3, 4, 5, 6, 7, 8],
            vec![1, 2, 3, 4, 5, 6, 7, 8],
            vec![10, 11, 12, 13, 14, 15, 16, 17],
            vec![10, 11, 12, 13, 14, 15, 16, 17],
            vec![20, 21, 22, 23, 24, 25, 26, 27],
            vec![20, 21, 22, 23, 24, 25, 26, 27],
            vec![1, 5, 9, 13, 17, 21, 25, 29],
            vec![1, 5, 9, 13, 17, 21, 25, 29],
        ]
    }

    #[test]
    fn loss_decreases_over_short_run() {
        let device = NdArrayDevice::default();
        let model_cfg = TinyAgtConfig::poc_default();
        let model: TinyAgt<Backend> = model_cfg.init(&device);

        let train_cfg = TrainConfig {
            batch_size: 4,
            n_epochs: 6,
            lr: 5e-3,
            seed: 42,
        };

        let sequences = synthetic_sequences();
        let (_trained, reports) = train_next_token(model, &sequences, &train_cfg, &device);

        assert!(
            reports.len() >= 6,
            "expected ≥6 reports, got {}",
            reports.len()
        );
        let first = reports.first().unwrap().loss;
        let last = reports.last().unwrap().loss;
        eprintln!(
            "loss curve start={:.3} end={:.3} (n={} steps)",
            first,
            last,
            reports.len()
        );
        assert!(
            last < first,
            "loss did not decrease: first={:.3} last={:.3}",
            first,
            last
        );
        // Also check meaningful reduction (model is actually learning).
        assert!(
            last < first * 0.9,
            "loss reduction too small: first={:.3} last={:.3} (need ≥10% drop)",
            first,
            last
        );
    }
}
