// SPDX-License-Identifier: MIT
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Training loop for the tiny agglutinative transformer.
//!
//! Phase 0 proof: feed it synthetic morpheme sequences from
//! [`adam_agg_synth`], optimise next-token prediction loss, show
//! that loss decreases. Pure Rust, CPU, no Python.

use burn::optim::adaptor::OptimizerAdaptor;
use burn::optim::{Adam, AdamConfig, GradientsParams, Optimizer};
use burn::prelude::*;
use burn::tensor::backend::AutodiffBackend;

use crate::TinyAgt;

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

/// Extended step report including the algebraic-loss split.
#[derive(Debug, Clone, Copy)]
pub struct AlgStepReport {
    pub epoch: usize,
    pub step: usize,
    pub ce: f32,
    pub alg: f32,
    pub total: f32,
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

/// Train with cross-entropy **plus** an algebraic penalty term that
/// pushes probability mass off morpho-invalid continuations.
///
/// `state_ids_per_seq[i][j]` = validator state-id (0..24) at position
/// `j` of sequence `i`, computed from the prefix `seq[0..j]`.
/// `invalid_mask_table[s]` = `[vocab_size]` mask with `1.0` at every
/// token id that is morphotactically illegal from state `s`, else
/// `0.0`. Both are built once in the caller via
/// [`crate::generate::compute_state_ids`] and
/// [`crate::generate::build_invalid_mask_table`].
///
/// L_total = L_CE + alpha · mean over (b,t) of Σ_v softmax(logits)[b,t,v] · invalid_mask[state(b,t)][v]
pub fn train_next_token_with_alg_loss<B: AutodiffBackend>(
    model: TinyAgt<B>,
    sequences: &[Vec<i64>],
    state_ids_per_seq: &[Vec<u8>],
    invalid_mask_table: &[Vec<f32>],
    cfg: &TrainConfig,
    alpha: f32,
    device: &B::Device,
) -> (TinyAgt<B>, Vec<AlgStepReport>)
where
    B::InnerBackend: Backend,
{
    use burn::tensor::activation::softmax;

    assert_eq!(
        sequences.len(),
        state_ids_per_seq.len(),
        "sequences and state_ids must align"
    );
    assert!(
        !invalid_mask_table.is_empty(),
        "invalid_mask_table must be non-empty"
    );
    let vocab_size = invalid_mask_table[0].len();
    let num_states = invalid_mask_table.len();

    let mut optim: OptimizerAdaptor<Adam, TinyAgt<B>, B> = AdamConfig::new().init();
    let mut model = model;
    let mut reports = Vec::new();

    let max_seq_len = model.max_seq_len();

    for epoch in 0..cfg.n_epochs {
        for (step, batch_pair) in sequences
            .chunks(cfg.batch_size)
            .zip(state_ids_per_seq.chunks(cfg.batch_size))
            .enumerate()
        {
            let (chunk, state_chunk) = batch_pair;
            let batch_size = chunk.len();
            let mut input_data = vec![0i64; batch_size * max_seq_len];
            let mut target_data = vec![0i64; batch_size * max_seq_len];
            // Per-position validator state; default = 0 (initial).
            let mut state_data = vec![0u8; batch_size * max_seq_len];
            // 1.0 at positions that contribute to the algebraic loss
            // (active positions inside the actual sequence), 0.0 at pads.
            let mut pos_mask_data = vec![0.0f32; batch_size * max_seq_len];

            for (i, (seq, sids)) in chunk.iter().zip(state_chunk.iter()).enumerate() {
                let take = seq.len().min(max_seq_len);
                if take == 0 {
                    continue;
                }
                for j in 0..take - 1 {
                    input_data[i * max_seq_len + j] = seq[j];
                    target_data[i * max_seq_len + j] = seq[j + 1];
                    // State before predicting seq[j+1] = state at position (j+1).
                    state_data[i * max_seq_len + j] = sids.get(j + 1).copied().unwrap_or(0);
                    pos_mask_data[i * max_seq_len + j] = 1.0;
                }
            }

            // Build mask tensor [B, T, V] by gathering from invalid_mask_table.
            let mut mask_data = vec![0.0f32; batch_size * max_seq_len * vocab_size];
            for i in 0..batch_size {
                for j in 0..max_seq_len {
                    let sid = state_data[i * max_seq_len + j] as usize;
                    let sid = sid.min(num_states - 1);
                    let row = &invalid_mask_table[sid];
                    let dst_base = (i * max_seq_len + j) * vocab_size;
                    mask_data[dst_base..dst_base + vocab_size].copy_from_slice(row);
                }
            }
            let active_positions: f32 = pos_mask_data.iter().sum::<f32>().max(1.0);

            let input: Tensor<B, 2, Int> = Tensor::from_data(
                burn::tensor::TensorData::new(input_data, [batch_size, max_seq_len]),
                device,
            );
            let target: Tensor<B, 2, Int> = Tensor::from_data(
                burn::tensor::TensorData::new(target_data, [batch_size, max_seq_len]),
                device,
            );
            let mask: Tensor<B, 3> = Tensor::from_data(
                burn::tensor::TensorData::new(mask_data, [batch_size, max_seq_len, vocab_size]),
                device,
            );
            let pos_mask: Tensor<B, 2> = Tensor::from_data(
                burn::tensor::TensorData::new(pos_mask_data, [batch_size, max_seq_len]),
                device,
            );

            // Forward.
            let logits = model.forward(input); // [B, T, V]
            let ce = model.loss(logits.clone(), target);

            // Algebraic penalty: invalid probability mass per active position.
            let probs = softmax(logits, 2); // [B, T, V]
            let invalid_mass = (probs * mask).sum_dim(2).squeeze::<2>(2); // [B, T]
            let masked_mass = invalid_mass * pos_mask;
            let denom: Tensor<B, 1> = Tensor::from_data(
                burn::tensor::TensorData::new(vec![active_positions], [1]),
                device,
            );
            let alg = masked_mass.sum() / denom;

            let alpha_t: Tensor<B, 1> =
                Tensor::from_data(burn::tensor::TensorData::new(vec![alpha], [1]), device);
            let total = ce.clone() + alg.clone() * alpha_t;

            let ce_v = ce.clone().into_scalar().elem::<f32>();
            let alg_v = alg.clone().into_scalar().elem::<f32>();
            let total_v = total.clone().into_scalar().elem::<f32>();

            reports.push(AlgStepReport {
                epoch,
                step,
                ce: ce_v,
                alg: alg_v,
                total: total_v,
            });

            let grads = total.backward();
            let grads = GradientsParams::from_grads(grads, &model);
            model = optim.step(cfg.lr, model, grads);
        }
    }

    (model, reports)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TinyAgtConfig;
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
