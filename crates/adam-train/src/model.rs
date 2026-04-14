use candle_core::{DType, Device, IndexOp, Result, Tensor};
use candle_nn::{
    Embedding, LayerNorm, Linear, Module, VarBuilder, embedding, layer_norm, linear_no_bias, ops,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelConfig {
    pub vocab_size: usize,
    pub hidden_dim: usize,
    pub num_heads: usize,
    pub num_layers: usize,
    pub ffn_dim: usize,
    pub max_seq_len: usize,
    pub dropout: f32,
}

impl ModelConfig {
    pub fn tiny() -> Self {
        Self {
            // Must match the BPE vocab size written by train_bpe.
            // Bumped to 1390 in v0.0.87 (lexicon-seeded vocab retrain).
            vocab_size: 1390,
            // Scaled up in v0.0.89 to ~3.06M params.
            hidden_dim: 224,
            num_heads: 8,
            num_layers: 4,
            ffn_dim: 896,
            max_seq_len: 128,
            // Tuned in v0.0.90: 0.1 was too heavy for the current corpus size;
            // with grad clipping + longer training, 0.05 generalizes better.
            dropout: 0.05,
        }
    }

    pub fn head_dim(&self) -> usize {
        self.hidden_dim / self.num_heads
    }

    pub fn parameter_count_estimate(&self) -> usize {
        let emb = self.vocab_size * self.hidden_dim;
        let pos = self.max_seq_len * self.hidden_dim;
        let qkv = self.hidden_dim * self.hidden_dim * 3;
        let out_proj = self.hidden_dim * self.hidden_dim;
        let ffn_up = self.hidden_dim * self.ffn_dim;
        let ffn_down = self.ffn_dim * self.hidden_dim;
        let layernorms = self.hidden_dim * 4;
        let per_layer = qkv + out_proj + ffn_up + ffn_down + layernorms;
        let final_ln = self.hidden_dim * 2;
        let head = self.hidden_dim * self.vocab_size;
        emb + pos + self.num_layers * per_layer + final_ln + head
    }
}

struct SelfAttention {
    qkv: Linear,
    out: Linear,
    num_heads: usize,
    head_dim: usize,
}

impl SelfAttention {
    fn new(cfg: &ModelConfig, vb: VarBuilder) -> Result<Self> {
        let qkv = linear_no_bias(cfg.hidden_dim, cfg.hidden_dim * 3, vb.pp("qkv"))?;
        let out = linear_no_bias(cfg.hidden_dim, cfg.hidden_dim, vb.pp("out"))?;
        Ok(Self {
            qkv,
            out,
            num_heads: cfg.num_heads,
            head_dim: cfg.head_dim(),
        })
    }

    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let (b, t, _c) = x.dims3()?;
        let qkv = self.qkv.forward(x)?;
        let qkv = qkv.reshape((b, t, 3, self.num_heads, self.head_dim))?;
        let q = qkv.i((.., .., 0))?.transpose(1, 2)?.contiguous()?;
        let k = qkv.i((.., .., 1))?.transpose(1, 2)?.contiguous()?;
        let v = qkv.i((.., .., 2))?.transpose(1, 2)?.contiguous()?;

        let scale = 1.0 / (self.head_dim as f64).sqrt();
        let att = q.matmul(&k.transpose(2, 3)?.contiguous()?)?;
        let att = (att * scale)?;
        let mask = causal_mask(t, x.device())?;
        let att = att.broadcast_add(&mask)?;
        let att = ops::softmax_last_dim(&att)?;
        let y = att.matmul(&v)?;
        let y = y.transpose(1, 2)?.contiguous()?.reshape((b, t, ()))?;
        self.out.forward(&y)
    }
}

struct Mlp {
    up: Linear,
    down: Linear,
}

impl Mlp {
    fn new(cfg: &ModelConfig, vb: VarBuilder) -> Result<Self> {
        let up = linear_no_bias(cfg.hidden_dim, cfg.ffn_dim, vb.pp("up"))?;
        let down = linear_no_bias(cfg.ffn_dim, cfg.hidden_dim, vb.pp("down"))?;
        Ok(Self { up, down })
    }

    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let x = self.up.forward(x)?;
        let x = x.gelu()?;
        self.down.forward(&x)
    }
}

struct Block {
    ln1: LayerNorm,
    attn: SelfAttention,
    ln2: LayerNorm,
    mlp: Mlp,
}

impl Block {
    fn new(cfg: &ModelConfig, vb: VarBuilder) -> Result<Self> {
        let ln1 = layer_norm(cfg.hidden_dim, 1e-5, vb.pp("ln1"))?;
        let attn = SelfAttention::new(cfg, vb.pp("attn"))?;
        let ln2 = layer_norm(cfg.hidden_dim, 1e-5, vb.pp("ln2"))?;
        let mlp = Mlp::new(cfg, vb.pp("mlp"))?;
        Ok(Self {
            ln1,
            attn,
            ln2,
            mlp,
        })
    }

    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let a = self.attn.forward(&self.ln1.forward(x)?)?;
        let x = (x + a)?;
        let m = self.mlp.forward(&self.ln2.forward(&x)?)?;
        x + m
    }
}

pub struct AdamBaseline {
    tok_emb: Embedding,
    pos_emb: Embedding,
    blocks: Vec<Block>,
    final_ln: LayerNorm,
    head: Linear,
    max_seq_len: usize,
    dropout: f32,
}

impl AdamBaseline {
    pub fn new(cfg: &ModelConfig, vb: VarBuilder) -> Result<Self> {
        let tok_emb = embedding(cfg.vocab_size, cfg.hidden_dim, vb.pp("tok_emb"))?;
        let pos_emb = embedding(cfg.max_seq_len, cfg.hidden_dim, vb.pp("pos_emb"))?;
        let mut blocks = Vec::with_capacity(cfg.num_layers);
        for i in 0..cfg.num_layers {
            blocks.push(Block::new(cfg, vb.pp(format!("block_{i}")))?);
        }
        let final_ln = layer_norm(cfg.hidden_dim, 1e-5, vb.pp("final_ln"))?;
        let head = linear_no_bias(cfg.hidden_dim, cfg.vocab_size, vb.pp("head"))?;
        Ok(Self {
            tok_emb,
            pos_emb,
            blocks,
            final_ln,
            head,
            max_seq_len: cfg.max_seq_len,
            dropout: cfg.dropout,
        })
    }

    pub fn forward(&self, ids: &Tensor, train: bool) -> Result<Tensor> {
        let (_b, t) = ids.dims2()?;
        if t > self.max_seq_len {
            candle_core::bail!("sequence length {t} > max_seq_len {}", self.max_seq_len);
        }
        let positions = Tensor::arange(0u32, t as u32, ids.device())?;
        let tok = self.tok_emb.forward(ids)?;
        let pos = self
            .pos_emb
            .forward(&positions)?
            .unsqueeze(0)?
            .broadcast_as(tok.shape())?;
        let mut x = (tok + pos)?;
        if train && self.dropout > 0.0 {
            x = candle_nn::ops::dropout(&x, self.dropout)?;
        }
        for block in &self.blocks {
            x = block.forward(&x)?;
            if train && self.dropout > 0.0 {
                x = candle_nn::ops::dropout(&x, self.dropout)?;
            }
        }
        let x = self.final_ln.forward(&x)?;
        self.head.forward(&x)
    }
}

fn causal_mask(seq_len: usize, device: &Device) -> Result<Tensor> {
    let mask: Vec<f32> = (0..seq_len)
        .flat_map(|i| (0..seq_len).map(move |j| if j <= i { 0.0 } else { f32::NEG_INFINITY }))
        .collect();
    Tensor::from_slice(&mask, (1, 1, seq_len, seq_len), device)?.to_dtype(DType::F32)
}

pub fn default_device() -> Result<Device> {
    #[cfg(target_os = "macos")]
    {
        if candle_core::utils::metal_is_available() {
            return Device::new_metal(0);
        }
    }
    Ok(Device::Cpu)
}
