use std::process::ExitCode;

use adam_train::{
    data::DataLoader,
    model::{AdamBaseline, ModelConfig, default_device},
};
use candle_core::{DType, IndexOp, Tensor};
use candle_nn::{VarBuilder, VarMap, loss};

const PACK_PATH: &str = "data/curated/adam_training_ids_pack.json";

fn main() -> ExitCode {
    let batch_size: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(8);
    let seq_len: usize = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(64);
    let seed: u64 = std::env::args()
        .nth(3)
        .and_then(|s| s.parse().ok())
        .unwrap_or(42);

    let device = match default_device() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("device: {e}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!(
        "device: {:?}, batch={}, seq_len={}",
        device, batch_size, seq_len
    );

    let mut loader = match DataLoader::from_pack(PACK_PATH, batch_size, seq_len, seed) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("loader: {e}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!(
        "pack loaded: vocab_size={}, total_tokens={}, bos={}, eos={}, pad={}, unk={}",
        loader.vocab_size,
        loader.total_tokens(),
        loader.bos_id,
        loader.eos_id,
        loader.pad_id,
        loader.unk_id
    );

    // Draw a few batches, assert target = input shifted by 1, report shapes.
    for step in 1..=3 {
        let (input, target) = match loader.next_batch(&device) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("next_batch: {e}");
                return ExitCode::FAILURE;
            }
        };
        let in_shape = input.shape().clone();
        let tg_shape = target.shape().clone();
        eprintln!("step {}: input {:?}, target {:?}", step, in_shape, tg_shape);

        // Verify shift-by-1 on row 0: input[0, 1..] should equal target[0, ..seq_len-1]
        match verify_shift(&input, &target) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("shift-by-1 verification failed: {e}");
                return ExitCode::FAILURE;
            }
        }
    }
    eprintln!("shift-by-1 property holds");

    // Now: build model, run forward + cross-entropy on one batch.
    let cfg = ModelConfig::tiny();
    if cfg.vocab_size != loader.vocab_size {
        eprintln!(
            "warning: model.vocab_size {} != loader.vocab_size {}",
            cfg.vocab_size, loader.vocab_size
        );
    }
    let varmap = VarMap::new();
    let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);
    let model = match AdamBaseline::new(&cfg, vb) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("model init: {e}");
            return ExitCode::FAILURE;
        }
    };

    let (input, target) = match loader.next_batch(&device) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("next_batch: {e}");
            return ExitCode::FAILURE;
        }
    };
    let logits = match model.forward(&input, false) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("forward: {e}");
            return ExitCode::FAILURE;
        }
    };
    // Flatten (B, T, V) -> (B*T, V) and (B, T) -> (B*T,)
    let (b, t, v) = match logits.dims3() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("dims3: {e}");
            return ExitCode::FAILURE;
        }
    };
    let logits_flat = match logits.reshape((b * t, v)) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("reshape logits: {e}");
            return ExitCode::FAILURE;
        }
    };
    let target_flat = match target.reshape((b * t,)) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("reshape target: {e}");
            return ExitCode::FAILURE;
        }
    };

    let loss_tensor = match loss::cross_entropy(&logits_flat, &target_flat) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("cross_entropy: {e}");
            return ExitCode::FAILURE;
        }
    };
    let loss_val = match loss_tensor.to_scalar::<f32>() {
        Ok(x) => x,
        Err(e) => {
            eprintln!("to_scalar: {e}");
            return ExitCode::FAILURE;
        }
    };
    let expected_random_loss = (cfg.vocab_size as f32).ln();
    eprintln!(
        "random-init cross-entropy loss: {:.4} (expected ~ln({}) = {:.4})",
        loss_val, cfg.vocab_size, expected_random_loss
    );
    // Random-init loss should be roughly log(vocab_size) but the exact value depends on
    // the variance of the default candle initializer; accept up to ±25% before warning.
    if (loss_val - expected_random_loss).abs() / expected_random_loss > 0.25 {
        eprintln!("warning: loss deviates >25% from log(vocab_size) — check init");
    }

    eprintln!("data loader smoke test passed");
    ExitCode::SUCCESS
}

fn verify_shift(input: &Tensor, target: &Tensor) -> candle_core::Result<()> {
    let (b, t) = input.dims2()?;
    if b == 0 || t < 2 {
        return Ok(());
    }
    // input row 0, positions 1..t  (length t-1)
    let in_slice = input.i((0, 1..t))?.to_vec1::<u32>()?;
    let tg_slice = target.i((0, 0..t - 1))?.to_vec1::<u32>()?;
    if in_slice != tg_slice {
        candle_core::bail!(
            "input[0, 1..] != target[0, ..t-1]; got in={:?} tg={:?}",
            &in_slice[..8.min(in_slice.len())],
            &tg_slice[..8.min(tg_slice.len())],
        );
    }
    Ok(())
}
