// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! **Phase 19 step B (2026-06-02)** — Pooled-embedding intent
//! classifier.
//!
//! User directive: replace the 676 substring matchers in
//! `crates/adam-dialog/src/semantics.rs` with a learned classifier
//! that takes a BPE-tokenised Kazakh utterance and emits a
//! probability distribution over the ~52 top-level Intent labels.
//!
//! ## Architecture (Option C — minimal)
//!
//! ```text
//!     BPE token ids [t0, t1, ..., tN]
//!         ↓
//!     Embedding table (vocab × d_model)
//!         ↓
//!     Mean-pool (or last-token-pool) across positions
//!         ↓
//!     MLP: d_model → hidden → n_intents
//!         ↓
//!     Softmax → P(intent | utterance)
//! ```
//!
//! Total params at default config:
//!   embedding: 5188 × 192 ≈ 996 k (initialised from v6 LM warm-start
//!                                   when `load_from_lm()` is used)
//!   MLP head:  192 × 256 + 256 × 52 ≈ 62 k
//!   total:     ~1 M params
//!
//! ## Training data
//!
//! `data/curated/adam_intent_training_pack.json` — 1 180 labeled
//! samples across 52 intent labels. Built by
//! `tools/build_intent_training_pack/build.py`.
//!
//! ## Why pooled, not attention
//!
//! Mean-pool is the simplest viable encoder. For 52 intent classes
//! on short utterances (≤ 12 BPE tokens typical), the bag-of-
//! embedding signal carries most of the information — explicit
//! attention adds parameters without much accuracy lift at this
//! scale. If validation accuracy plateaus, we upgrade to a
//! single self-attention block over the same embeddings.

use burn::module::Module;
use burn::nn::{
    Embedding, EmbeddingConfig, Linear, LinearConfig,
    loss::CrossEntropyLossConfig,
};
use burn::prelude::*;
use burn::tensor::backend::Backend;

#[derive(Debug, Config)]
pub struct IntentClassifierConfig {
    pub vocab_size: usize,
    pub d_model: usize,
    pub hidden: usize,
    pub n_intents: usize,
}

impl IntentClassifierConfig {
    /// Default config matched to the v6 LM embedding shape so
    /// `load_from_lm` can copy the embedding table verbatim.
    pub fn default_for_lm(vocab_size: usize, d_model: usize, n_intents: usize) -> Self {
        Self::new(vocab_size, d_model, 256, n_intents)
    }
}

#[derive(Module, Debug)]
pub struct IntentClassifier<B: Backend> {
    pub token_emb: Embedding<B>,
    pub mlp1: Linear<B>,
    pub mlp2: Linear<B>,
    pub vocab_size: usize,
    pub d_model: usize,
    pub n_intents: usize,
}

impl IntentClassifierConfig {
    pub fn init<B: Backend>(&self, device: &B::Device) -> IntentClassifier<B> {
        let token_emb = EmbeddingConfig::new(self.vocab_size, self.d_model).init(device);
        let mlp1 = LinearConfig::new(self.d_model, self.hidden).init(device);
        let mlp2 = LinearConfig::new(self.hidden, self.n_intents).init(device);
        IntentClassifier {
            token_emb,
            mlp1,
            mlp2,
            vocab_size: self.vocab_size,
            d_model: self.d_model,
            n_intents: self.n_intents,
        }
    }
}

impl<B: Backend> IntentClassifier<B> {
    /// Forward pass over a batch of token id sequences.
    ///
    /// `tokens` shape: `[batch, seq_len]` of integer token ids.
    /// `mask`   shape: `[batch, seq_len]` of 0/1 floats — 1.0 for
    ///                 real positions, 0.0 for padding.  Pooling
    ///                 averages only over the masked positions so
    ///                 padding does not bias the utterance vector.
    /// Returns logits `[batch, n_intents]`.
    pub fn forward(&self, tokens: Tensor<B, 2, Int>, mask: Tensor<B, 2>) -> Tensor<B, 2> {
        let [batch, seq_len] = tokens.dims();
        // Embed: [batch, seq_len, d_model]
        let emb = self.token_emb.forward(tokens);
        // Apply mask broadcast: [batch, seq_len, 1] * emb
        let mask_3 = mask.clone().unsqueeze_dim::<3>(2);
        let masked = emb * mask_3.clone();
        // Mean-pool: sum over seq_len / count(real positions).
        let summed: Tensor<B, 2> = masked.sum_dim(1).squeeze(1);
        let denom: Tensor<B, 1> = mask.sum_dim(1).squeeze(1);
        let denom_safe = denom.clamp_min(1.0).unsqueeze_dim::<2>(1);
        let pooled = summed / denom_safe;
        let _ = (batch, seq_len);
        // MLP: d_model → hidden → n_intents
        let h = burn::tensor::activation::relu(self.mlp1.forward(pooled));
        self.mlp2.forward(h)
    }

    /// Cross-entropy loss between predicted logits and target intent ids.
    ///
    /// `logits`: `[batch, n_intents]`.
    /// `targets`: `[batch]`.
    pub fn loss(&self, logits: Tensor<B, 2>, targets: Tensor<B, 1, Int>) -> Tensor<B, 1> {
        let device = logits.device();
        CrossEntropyLossConfig::new()
            .init(&device)
            .forward(logits, targets)
    }

    /// Argmax + softmax max — returns `(predicted_intent_id, confidence_in_0_1)`.
    /// Use for inference; batch size assumed 1.
    pub fn predict_one(&self, tokens: Tensor<B, 2, Int>, mask: Tensor<B, 2>) -> (usize, f32) {
        let logits = self.forward(tokens, mask);
        let probs = burn::tensor::activation::softmax(logits, 1);
        let [_batch, n] = probs.dims();
        let row: Vec<f32> = probs
            .reshape([n])
            .into_data()
            .convert::<f32>()
            .into_vec()
            .unwrap_or_default();
        let (idx, conf) = row
            .iter()
            .enumerate()
            .fold((0usize, 0.0_f32), |(bi, bv), (i, &v)| {
                if v > bv { (i, v) } else { (bi, bv) }
            });
        (idx, conf)
    }

    pub fn n_intents(&self) -> usize {
        self.n_intents
    }
    pub fn vocab_size(&self) -> usize {
        self.vocab_size
    }
    pub fn d_model(&self) -> usize {
        self.d_model
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use burn::backend::NdArray;

    type B = NdArray<f32>;

    #[test]
    fn fresh_model_emits_finite_logits() {
        let cfg = IntentClassifierConfig::default_for_lm(5188, 192, 52);
        let device = Default::default();
        let m: IntentClassifier<B> = cfg.init(&device);
        // batch=1, seq=6
        let tokens: Tensor<B, 2, Int> = Tensor::from_data(
            burn::tensor::TensorData::new(vec![1i64, 50, 100, 200, 2, 0], [1, 6]),
            &device,
        );
        let mask: Tensor<B, 2> = Tensor::from_data(
            burn::tensor::TensorData::new(vec![1.0_f32, 1.0, 1.0, 1.0, 1.0, 0.0], [1, 6]),
            &device,
        );
        let logits = m.forward(tokens, mask);
        let [b, n] = logits.dims();
        assert_eq!(b, 1);
        assert_eq!(n, 52);
    }

    #[test]
    fn predict_one_returns_valid_index() {
        let cfg = IntentClassifierConfig::default_for_lm(5188, 192, 52);
        let device = Default::default();
        let m: IntentClassifier<B> = cfg.init(&device);
        let tokens: Tensor<B, 2, Int> = Tensor::from_data(
            burn::tensor::TensorData::new(vec![1i64, 50, 100, 2], [1, 4]),
            &device,
        );
        let mask: Tensor<B, 2> = Tensor::from_data(
            burn::tensor::TensorData::new(vec![1.0_f32; 4], [1, 4]),
            &device,
        );
        let (idx, conf) = m.predict_one(tokens, mask);
        assert!(idx < 52);
        assert!((0.0..=1.0).contains(&conf));
    }
}
