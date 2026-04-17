use std::{env, path::PathBuf, process::ExitCode, time::Instant};

use adam_train::{
    data::DataLoader,
    model::{AdamBaseline, ModelConfig, default_device},
};
use candle_core::{DType, backprop::GradStore};
use candle_nn::{AdamW, Optimizer, ParamsAdamW, VarBuilder, VarMap, loss};

const PACK_PATH: &str = "data/curated/adam_training_ids_pack.json";

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let max_steps: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(2000);
    let batch_size: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(16);
    let seq_len: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(64);
    let base_lr: f64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(3e-4);
    let warmup_steps: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(100);
    let log_every: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(50);
    let checkpoint_path: PathBuf = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data/training/adam_baseline_checkpoint.safetensors"));
    let seed: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(42);

    let device = match default_device() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("device: {e}");
            return ExitCode::FAILURE;
        }
    };

    let cfg = ModelConfig::tiny();
    eprintln!(
        "training config: steps={} batch={} seq_len={} lr={} warmup={} device={:?}",
        max_steps, batch_size, seq_len, base_lr, warmup_steps, device
    );
    eprintln!(
        "model: vocab={} hidden={} heads={} layers={} params~{}",
        cfg.vocab_size,
        cfg.hidden_dim,
        cfg.num_heads,
        cfg.num_layers,
        cfg.parameter_count_estimate()
    );

    let mut loader = match DataLoader::from_pack(PACK_PATH, batch_size, seq_len, seed) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("loader: {e}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!(
        "data pack: vocab={} tokens={}",
        loader.vocab_size,
        loader.total_tokens()
    );

    let varmap = VarMap::new();
    let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);
    let model = match AdamBaseline::new(&cfg, vb) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("model init: {e}");
            return ExitCode::FAILURE;
        }
    };

    let opt_params = ParamsAdamW {
        lr: base_lr,
        ..Default::default()
    };
    let mut opt = match AdamW::new(varmap.all_vars(), opt_params) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("optimizer: {e}");
            return ExitCode::FAILURE;
        }
    };

    let start = Instant::now();
    let mut first_loss: Option<f32> = None;
    let mut running_loss: f32 = 0.0;
    let mut running_count: usize = 0;
    let mut last_loss: f32 = f32::NAN;

    for step in 1..=max_steps {
        let lr = compute_lr(step, warmup_steps, max_steps, base_lr);
        opt.set_learning_rate(lr);

        let (input, target) = match loader.next_batch(&device) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("batch: {e}");
                return ExitCode::FAILURE;
            }
        };

        let logits = match model.forward(&input, true) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("forward: {e}");
                return ExitCode::FAILURE;
            }
        };
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
        let loss_t = match loss::cross_entropy(&logits_flat, &target_flat) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("loss: {e}");
                return ExitCode::FAILURE;
            }
        };

        let mut grads = match loss_t.backward() {
            Ok(g) => g,
            Err(e) => {
                eprintln!("backward: {e}");
                return ExitCode::FAILURE;
            }
        };
        if let Err(e) = clip_grad_norm(&varmap, &mut grads, 1.0) {
            eprintln!("clip_grad_norm: {e}");
            return ExitCode::FAILURE;
        }
        if let Err(e) = opt.step(&grads) {
            eprintln!("opt step: {e}");
            return ExitCode::FAILURE;
        }

        let loss_val = loss_t.to_scalar::<f32>().unwrap_or(f32::NAN);
        if first_loss.is_none() {
            first_loss = Some(loss_val);
        }
        last_loss = loss_val;
        running_loss += loss_val;
        running_count += 1;

        if step % log_every == 0 || step == 1 {
            let avg = running_loss / running_count as f32;
            let elapsed = start.elapsed().as_secs_f64();
            let steps_per_sec = step as f64 / elapsed;
            eprintln!(
                "step {:>5}/{}: loss={:.4} avg_{}={:.4} lr={:.2e} ({:.2} steps/s, {:.1}s)",
                step, max_steps, loss_val, log_every, avg, lr, steps_per_sec, elapsed
            );
            running_loss = 0.0;
            running_count = 0;
        }

        // Periodic checkpoint — added v0.4.0 after a reboot lost 13k steps
        // of an uncheckpointed run. Save every 2000 steps (plus log_every=500
        // schedule this aligns with) so a crash at most loses ~45 min of work.
        if step > 0 && step % 2000 == 0 {
            if let Err(e) = varmap.save(&checkpoint_path) {
                eprintln!("periodic checkpoint failed at step {}: {}", step, e);
            } else {
                eprintln!("periodic checkpoint saved at step {}", step);
            }
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    eprintln!(
        "training complete in {:.2}s: first_loss={:.4} last_loss={:.4} (×{:.2} reduction)",
        elapsed,
        first_loss.unwrap_or(0.0),
        last_loss,
        first_loss.unwrap_or(1.0) / last_loss.max(1e-6),
    );

    if let Some(parent) = checkpoint_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("cannot create checkpoint dir: {e}");
                return ExitCode::FAILURE;
            }
        }
    }
    match varmap.save(&checkpoint_path) {
        Ok(()) => eprintln!("checkpoint saved: {}", checkpoint_path.display()),
        Err(e) => {
            eprintln!("checkpoint save: {e}");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

fn compute_lr(step: usize, warmup: usize, max_steps: usize, peak_lr: f64) -> f64 {
    if warmup > 0 && step <= warmup {
        peak_lr * (step as f64) / (warmup as f64)
    } else {
        let denom = (max_steps - warmup).max(1) as f64;
        let progress = ((step - warmup) as f64 / denom).min(1.0);
        let cos = 0.5 * (1.0 + (std::f64::consts::PI * progress).cos());
        // minimum lr = 10% of peak
        peak_lr * (0.1 + 0.9 * cos)
    }
}

/// Clip the global L2 norm of all gradients in `grads` to `max_norm`.
/// Returns the pre-clip global norm (for logging).
fn clip_grad_norm(
    varmap: &VarMap,
    grads: &mut GradStore,
    max_norm: f32,
) -> candle_core::Result<f32> {
    let vars = varmap.all_vars();
    let mut sum_sq = 0.0_f64;
    for v in &vars {
        if let Some(g) = grads.get(v.as_tensor()) {
            sum_sq += g.sqr()?.sum_all()?.to_scalar::<f32>()? as f64;
        }
    }
    let total_norm = sum_sq.sqrt() as f32;
    if total_norm > max_norm && max_norm > 0.0 {
        let scale = (max_norm / total_norm) as f64;
        let mut updates: Vec<(usize, candle_core::Tensor)> = Vec::new();
        for (idx, v) in vars.iter().enumerate() {
            if let Some(g) = grads.get(v.as_tensor()) {
                let scaled = (g * scale)?;
                updates.push((idx, scaled));
            }
        }
        for (idx, scaled) in updates {
            grads.insert(vars[idx].as_tensor(), scaled);
        }
    }
    Ok(total_norm)
}
