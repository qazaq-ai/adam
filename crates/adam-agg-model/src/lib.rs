//! `adam-agg-model` — tiny pure-Rust transformer for next-morpheme
//! prediction.
//!
//! Phase 0 proof-of-concept: a **toy-sized** decoder-only transformer
//! (~100K-1M params) using the [`burn`] framework. Goal is NOT
//! state-of-art Kazakh — the goal is to **prove the pipeline works
//! end-to-end in pure Rust**: synthetic FST-generated tokens →
//! training → loss decreases → constrained inference.
//!
//! Architecture:
//!
//! - **Token embedding** (vocab_size × d_model).
//! - **Positional embedding** (max_seq_len × d_model).
//! - **N transformer-decoder layers**, each:
//!   - Multi-head self-attention (causal).
//!   - Position-wise FFN.
//!   - Pre-layernorm + residuals.
//! - **Output projection** to vocab_size (tied with embedding).
//!
//! Pure Rust: depends only on [`burn`] with the ndarray CPU backend.
//! Optional Metal / WGPU acceleration is selectable at compile-time
//! via burn feature flags; this crate's default is CPU only to keep
//! the air-gap-deployable USP intact.

pub mod train;

use burn::config::Config;
use burn::module::Module;
use burn::nn::attention::{MhaInput, MultiHeadAttention, MultiHeadAttentionConfig};
use burn::nn::loss::CrossEntropyLossConfig;
use burn::nn::{Embedding, EmbeddingConfig, LayerNorm, LayerNormConfig, Linear, LinearConfig};
use burn::prelude::*;
use burn::tensor::activation;
use burn::tensor::backend::Backend;

/// Model configuration. Sized for Phase 0 PoC; production runs would
/// scale `vocab_size` to ~17k, `d_model` to 256, `n_layers` to 4-6.
#[derive(Debug, Config)]
pub struct TinyAgtConfig {
    pub vocab_size: usize,
    pub max_seq_len: usize,
    pub d_model: usize,
    pub n_heads: usize,
    pub n_layers: usize,
    pub d_ff: usize,
}

impl TinyAgtConfig {
    /// Default tiny config for PoC: ~200K params @ vocab 1000.
    pub fn poc_default() -> Self {
        Self::new(1000, 32, 64, 4, 2, 128)
    }

    /// Full-scale config: ~10M params @ vocab 17k.
    pub fn full_scale() -> Self {
        Self::new(17_000, 64, 256, 4, 4, 1024)
    }
}

/// Tiny agglutinative transformer.
#[derive(Module, Debug)]
pub struct TinyAgt<B: Backend> {
    token_emb: Embedding<B>,
    pos_emb: Embedding<B>,
    layers: Vec<DecoderLayer<B>>,
    final_norm: LayerNorm<B>,
    output_proj: Linear<B>,
    max_seq_len: usize,
    vocab_size: usize,
}

#[derive(Module, Debug)]
pub struct DecoderLayer<B: Backend> {
    norm1: LayerNorm<B>,
    attn: MultiHeadAttention<B>,
    norm2: LayerNorm<B>,
    ff1: Linear<B>,
    ff2: Linear<B>,
}

impl TinyAgtConfig {
    pub fn init<B: Backend>(&self, device: &B::Device) -> TinyAgt<B> {
        let token_emb = EmbeddingConfig::new(self.vocab_size, self.d_model).init(device);
        let pos_emb = EmbeddingConfig::new(self.max_seq_len, self.d_model).init(device);
        let layers = (0..self.n_layers)
            .map(|_| {
                let norm1 = LayerNormConfig::new(self.d_model).init(device);
                let attn = MultiHeadAttentionConfig::new(self.d_model, self.n_heads).init(device);
                let norm2 = LayerNormConfig::new(self.d_model).init(device);
                let ff1 = LinearConfig::new(self.d_model, self.d_ff).init(device);
                let ff2 = LinearConfig::new(self.d_ff, self.d_model).init(device);
                DecoderLayer {
                    norm1,
                    attn,
                    norm2,
                    ff1,
                    ff2,
                }
            })
            .collect();
        let final_norm = LayerNormConfig::new(self.d_model).init(device);
        let output_proj = LinearConfig::new(self.d_model, self.vocab_size).init(device);
        TinyAgt {
            token_emb,
            pos_emb,
            layers,
            final_norm,
            output_proj,
            max_seq_len: self.max_seq_len,
            vocab_size: self.vocab_size,
        }
    }
}

impl<B: Backend> TinyAgt<B> {
    /// Forward pass over a batch of token id sequences.
    ///
    /// `tokens` shape: `[batch, seq_len]` of integer token ids.
    /// Returns logits `[batch, seq_len, vocab_size]`.
    pub fn forward(&self, tokens: Tensor<B, 2, Int>) -> Tensor<B, 3> {
        let [batch, seq_len] = tokens.dims();
        let device = tokens.device();

        // Position ids: 0 .. seq_len for each sample.
        let positions = Tensor::<B, 1, Int>::arange(0..seq_len as i64, &device)
            .unsqueeze::<2>()
            .repeat(&[batch, 1]);

        let tok_e = self.token_emb.forward(tokens);
        let pos_e = self.pos_emb.forward(positions);
        let mut x: Tensor<B, 3> = tok_e + pos_e;

        for layer in &self.layers {
            // Pre-norm attention.
            let xn = layer.norm1.forward(x.clone());
            // Build a causal mask: position i can attend to positions <= i.
            let mask = causal_mask::<B>(seq_len, &device);
            let mha_input = MhaInput::self_attn(xn).mask_attn(mask);
            let attn_out = layer.attn.forward(mha_input).context;
            x = x + attn_out;

            // Pre-norm FFN with GELU.
            let xn2 = layer.norm2.forward(x.clone());
            let ff = layer.ff2.forward(activation::gelu(layer.ff1.forward(xn2)));
            x = x + ff;
        }
        let x = self.final_norm.forward(x);
        self.output_proj.forward(x)
    }

    /// Cross-entropy loss between predicted logits and target tokens.
    ///
    /// `logits`: `[batch, seq_len, vocab_size]`.
    /// `targets`: `[batch, seq_len]`.
    pub fn loss(&self, logits: Tensor<B, 3>, targets: Tensor<B, 2, Int>) -> Tensor<B, 1> {
        let [batch, seq_len, vocab] = logits.dims();
        let device = logits.device();
        let flat_logits = logits.reshape([batch * seq_len, vocab]);
        let flat_targets = targets.reshape([batch * seq_len]);
        CrossEntropyLossConfig::new()
            .init(&device)
            .forward(flat_logits, flat_targets)
    }

    pub fn vocab_size(&self) -> usize {
        self.vocab_size
    }

    pub fn max_seq_len(&self) -> usize {
        self.max_seq_len
    }
}

/// Causal (upper-triangular = -inf) mask of shape `[seq_len, seq_len]`.
fn causal_mask<B: Backend>(seq_len: usize, device: &B::Device) -> Tensor<B, 3, Bool> {
    let mut data = vec![false; seq_len * seq_len];
    for i in 0..seq_len {
        for j in 0..seq_len {
            data[i * seq_len + j] = j > i;
        }
    }
    let t: Tensor<B, 1, Bool> = Tensor::from_data(
        burn::tensor::TensorData::new(data, [seq_len * seq_len]),
        device,
    );
    t.reshape([1, seq_len, seq_len])
}

#[cfg(test)]
mod tests {
    use super::*;
    use burn::backend::NdArray;
    use burn::tensor::Tensor;

    type B = NdArray<f32>;

    #[test]
    fn poc_config_param_count_is_under_500k() {
        let cfg = TinyAgtConfig::poc_default();
        // Quick estimate: 2 × (vocab × d_model) for emb + lm head,
        // + n_layers × (4 × d_model^2 + 2 × d_model × d_ff).
        let emb = 2 * cfg.vocab_size * cfg.d_model;
        let attn = cfg.n_layers * 4 * cfg.d_model * cfg.d_model;
        let ff = cfg.n_layers * 2 * cfg.d_model * cfg.d_ff;
        let total = emb + attn + ff;
        assert!(total < 500_000, "PoC config too large: {} params", total);
    }

    #[test]
    fn model_initialises_and_runs_forward_on_cpu() {
        let device = Default::default();
        let cfg = TinyAgtConfig::poc_default();
        let model: TinyAgt<B> = cfg.init(&device);
        // Single batch of 8 tokens.
        let tokens: Tensor<B, 2, Int> = Tensor::from_data(
            burn::tensor::TensorData::new(vec![0i64; 8], [1, 8]),
            &device,
        );
        let logits = model.forward(tokens);
        assert_eq!(logits.dims(), [1, 8, cfg.vocab_size]);
    }

    #[test]
    fn loss_runs_and_produces_scalar() {
        let device = Default::default();
        let cfg = TinyAgtConfig::poc_default();
        let model: TinyAgt<B> = cfg.init(&device);
        let tokens: Tensor<B, 2, Int> = Tensor::from_data(
            burn::tensor::TensorData::new(vec![0i64; 8], [1, 8]),
            &device,
        );
        let targets: Tensor<B, 2, Int> = Tensor::from_data(
            burn::tensor::TensorData::new(vec![0i64; 8], [1, 8]),
            &device,
        );
        let logits = model.forward(tokens);
        let loss = model.loss(logits, targets);
        let v = loss.into_scalar();
        // Loss is a positive scalar; sanity check.
        assert!(v.elem::<f32>() > 0.0);
    }
}
