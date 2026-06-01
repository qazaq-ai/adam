// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! **Phase 15g.C.2 step 2 (2026-06-01)** — sequence scoring.
//!
//! `score_sequence(model, tokens) → log_prob` evaluates the
//! probability the model assigns to a candidate token sequence
//! under teacher forcing.  Used by the voice REPL's fuzzy
//! rescorer: given a Whisper output and N phonetic-neighbour
//! candidates, the rescorer scores each replacement in its
//! sentence context and picks the highest.
//!
//! ## Semantics
//!
//! For a sequence `t_0, t_1, ..., t_{n-1}` (typically
//! `[<bos>, w_1, w_2, ..., w_k, <eos>]`):
//!
//! ```text
//!   log P(t_0, ..., t_{n-1}) = Σ_{i=0..n-2} log P(t_{i+1} | t_0..t_i)
//! ```
//!
//! Position 0's log-prob is excluded (no context to condition on),
//! and the model only predicts positions 1..n-1.  The returned
//! `total_log_prob` is the **summed** log-likelihood; the
//! `per_token` field is `total / (n-1)`, useful for comparing
//! sequences of different lengths.
//!
//! ## Padding
//!
//! Sequences shorter than `model.max_seq_len()` are zero-padded
//! (the same convention as `train_next_token`); pad positions
//! contribute their own log-prob to the sum, which is fine for
//! relative comparison but means the absolute number is not a
//! pure language-model perplexity.  For Phase 15g.C.2's
//! rescoring use case (compare N candidates of equal length in
//! the same sentence frame) padding cancels out.

use burn::prelude::*;
use burn::tensor::activation::log_softmax;
use burn::tensor::backend::Backend;

use crate::TinyAgt;

#[derive(Debug, Clone, Copy)]
pub struct SequenceScore {
    /// Σ log P over scored positions (1..n-1).
    pub total_log_prob: f32,
    /// `total / max(1, n-1)` — convenient for length-normalised
    /// comparison.
    pub per_token: f32,
    /// Number of positions that contributed to the sum.
    pub scored_positions: usize,
}

/// Score a single token sequence under teacher forcing.
///
/// `tokens`: input id sequence — caller's responsibility to prepend
/// `<bos>` and append `<eos>` if their tokenizer convention
/// requires it (the model was trained with `<bos>` = 1, `<eos>` = 2).
///
/// Returns `None` when the sequence is empty or has fewer than 2
/// tokens (no conditional positions to score).
pub fn score_sequence<B: Backend>(
    model: &TinyAgt<B>,
    tokens: &[i64],
    device: &B::Device,
) -> Option<SequenceScore> {
    let n = tokens.len();
    if n < 2 {
        return None;
    }
    let max_seq_len = model.max_seq_len();
    let take = n.min(max_seq_len);

    // Build the input batch [1, max_seq_len] with seq[..take-1] in
    // positions [0..take-1] and zeros elsewhere — matches the
    // training-time padding convention.
    let mut input_buf = vec![0i64; max_seq_len];
    let mut target_buf = vec![0i64; max_seq_len];
    for i in 0..take - 1 {
        input_buf[i] = tokens[i];
        target_buf[i] = tokens[i + 1];
    }
    let scored_positions = take - 1;

    let input: Tensor<B, 2, Int> = Tensor::from_data(
        burn::tensor::TensorData::new(input_buf, [1, max_seq_len]),
        device,
    );

    // Forward: [1, max_seq_len, vocab]
    let logits = model.forward(input);
    // log P over vocab at each position.
    let log_probs = log_softmax(logits, 2);

    // Gather log_probs at the target token id for each position
    // 0..scored_positions; positions ≥ scored_positions still get
    // accumulated against `target_buf[i] = 0`, but those are pad,
    // and for relative scoring across same-length sequences they
    // contribute the same constant offset — we trim them out below.
    let target: Tensor<B, 2, Int> = Tensor::from_data(
        burn::tensor::TensorData::new(target_buf, [1, max_seq_len]),
        device,
    );

    // gather wants the index tensor to share rank with log_probs.
    // Shape it to [1, max_seq_len, 1] then squeeze afterwards.
    let target_3d = target.unsqueeze_dim::<3>(2);
    let gathered = log_probs.gather(2, target_3d); // [1, max_seq_len, 1]
    let gathered = gathered.squeeze::<2>(2); // [1, max_seq_len]
    let row: Vec<f32> = gathered
        .reshape([max_seq_len])
        .into_data()
        .convert::<f32>()
        .into_vec()
        .ok()?;

    let mut total = 0.0_f32;
    for i in 0..scored_positions {
        total += row[i];
    }
    let per_token = if scored_positions == 0 {
        0.0
    } else {
        total / scored_positions as f32
    };
    Some(SequenceScore {
        total_log_prob: total,
        per_token,
        scored_positions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TinyAgtConfig;
    use burn::backend::NdArray;

    type B = NdArray<f32>;

    #[test]
    fn empty_sequence_returns_none() {
        let cfg = TinyAgtConfig::poc_default();
        let device = Default::default();
        let model: TinyAgt<B> = cfg.init(&device);
        assert!(score_sequence(&model, &[], &device).is_none());
        assert!(score_sequence(&model, &[1], &device).is_none());
    }

    #[test]
    fn scores_a_short_sequence() {
        let cfg = TinyAgtConfig::poc_default();
        let device = Default::default();
        let model: TinyAgt<B> = cfg.init(&device);
        // Sequence of 5 tokens → 4 scored positions.
        let s = score_sequence(&model, &[1, 50, 100, 200, 2], &device).unwrap();
        assert_eq!(s.scored_positions, 4);
        assert!(s.total_log_prob.is_finite());
        assert!(s.per_token.is_finite());
        assert!(s.total_log_prob < 0.0, "log-prob should be negative");
    }

    #[test]
    fn longer_sequence_has_more_scored_positions() {
        let cfg = TinyAgtConfig::poc_default();
        let device = Default::default();
        let model: TinyAgt<B> = cfg.init(&device);
        let s_short = score_sequence(&model, &[1, 50, 2], &device).unwrap();
        let s_long = score_sequence(&model, &[1, 50, 100, 200, 2], &device).unwrap();
        assert!(s_long.scored_positions > s_short.scored_positions);
    }
}
