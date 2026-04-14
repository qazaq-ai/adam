use std::fs;

use candle_core::{Device, Tensor};
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DataError {
    #[error("cannot read pack file: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid pack JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("pack has no samples")]
    EmptyPack,
    #[error("token stream length {stream_len} is not enough for seq_len {seq_len} + 1")]
    StreamTooShort { stream_len: usize, seq_len: usize },
    #[error("batch_size must be > 0")]
    ZeroBatchSize,
    #[error("seq_len must be > 0")]
    ZeroSeqLen,
    #[error("tensor error: {0}")]
    Tensor(#[from] candle_core::Error),
}

#[derive(Debug, Deserialize)]
struct IdSample {
    #[allow(dead_code)]
    id: String,
    ids: Vec<u32>,
}

#[derive(Debug, Deserialize)]
struct IdPack {
    pad_id: u32,
    bos_id: u32,
    eos_id: u32,
    unk_id: u32,
    vocab_size: usize,
    samples: Vec<IdSample>,
}

/// Streams packed training batches from a JSON id pack.
///
/// Concatenates every sample's `ids` (each already wrapped in `<bos>...<eos>`) into a single
/// contiguous stream. Each call to `next_batch` draws `batch_size` random offsets into the stream
/// and yields (input, target) tensors of shape `[batch_size, seq_len]` where target is input
/// shifted by 1 (standard next-token prediction).
pub struct DataLoader {
    token_stream: Vec<u32>,
    batch_size: usize,
    seq_len: usize,
    rng_state: u64,
    pub pad_id: u32,
    pub bos_id: u32,
    pub eos_id: u32,
    pub unk_id: u32,
    pub vocab_size: usize,
}

impl DataLoader {
    pub fn from_pack(
        pack_path: &str,
        batch_size: usize,
        seq_len: usize,
        seed: u64,
    ) -> Result<Self, DataError> {
        if batch_size == 0 {
            return Err(DataError::ZeroBatchSize);
        }
        if seq_len == 0 {
            return Err(DataError::ZeroSeqLen);
        }
        let raw = fs::read_to_string(pack_path)?;
        let pack: IdPack = serde_json::from_str(&raw)?;
        if pack.samples.is_empty() {
            return Err(DataError::EmptyPack);
        }

        let mut token_stream: Vec<u32> = Vec::new();
        for s in &pack.samples {
            token_stream.extend_from_slice(&s.ids);
        }
        if token_stream.len() < seq_len + 1 {
            return Err(DataError::StreamTooShort {
                stream_len: token_stream.len(),
                seq_len,
            });
        }

        Ok(Self {
            token_stream,
            batch_size,
            seq_len,
            rng_state: seed,
            pad_id: pack.pad_id,
            bos_id: pack.bos_id,
            eos_id: pack.eos_id,
            unk_id: pack.unk_id,
            vocab_size: pack.vocab_size,
        })
    }

    pub fn total_tokens(&self) -> usize {
        self.token_stream.len()
    }

    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    pub fn seq_len(&self) -> usize {
        self.seq_len
    }

    fn next_u64(&mut self) -> u64 {
        self.rng_state = self
            .rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.rng_state
    }

    pub fn next_batch(&mut self, device: &Device) -> Result<(Tensor, Tensor), DataError> {
        let stream_len = self.token_stream.len();
        let max_start = stream_len - self.seq_len - 1;
        let total = self.batch_size * self.seq_len;
        let mut input_flat: Vec<u32> = Vec::with_capacity(total);
        let mut target_flat: Vec<u32> = Vec::with_capacity(total);

        for _ in 0..self.batch_size {
            let r = self.next_u64() as usize;
            let start = if max_start == 0 {
                0
            } else {
                r % (max_start + 1)
            };
            input_flat.extend_from_slice(&self.token_stream[start..start + self.seq_len]);
            target_flat.extend_from_slice(&self.token_stream[start + 1..start + self.seq_len + 1]);
        }

        let input = Tensor::from_vec(input_flat, (self.batch_size, self.seq_len), device)?;
        let target = Tensor::from_vec(target_flat, (self.batch_size, self.seq_len), device)?;
        Ok((input, target))
    }
}
