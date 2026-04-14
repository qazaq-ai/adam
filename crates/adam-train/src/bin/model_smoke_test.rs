use std::process::ExitCode;

use adam_train::model::{AdamBaseline, ModelConfig, default_device};
use candle_core::{DType, IndexOp, Tensor};
use candle_nn::{VarBuilder, VarMap};

fn main() -> ExitCode {
    let cfg = ModelConfig::tiny();
    let device = match default_device() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("device init: {e}");
            return ExitCode::FAILURE;
        }
    };

    eprintln!(
        "device: {:?}, metal_available: {}",
        device,
        candle_core::utils::metal_is_available()
    );
    eprintln!(
        "config: vocab={}, hidden={}, heads={}, layers={}, ffn={}, max_seq_len={}",
        cfg.vocab_size, cfg.hidden_dim, cfg.num_heads, cfg.num_layers, cfg.ffn_dim, cfg.max_seq_len
    );
    eprintln!(
        "parameter count (estimate): {} (~{:.2} MB at f32)",
        cfg.parameter_count_estimate(),
        cfg.parameter_count_estimate() as f64 * 4.0 / 1_048_576.0
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

    // Actual parameter count from varmap
    let vars = varmap.all_vars();
    let actual_params: usize = vars.iter().map(|v| v.elem_count()).sum();
    eprintln!(
        "actual parameter count: {} (~{:.2} MB)",
        actual_params,
        actual_params as f64 * 4.0 / 1_048_576.0
    );

    // Dummy input: batch=2, seq_len=32, ids in [0, vocab_size)
    let batch = 2usize;
    let seq_len = 32usize;
    let ids: Vec<u32> = (0..batch * seq_len)
        .map(|i| (i as u32) % cfg.vocab_size as u32)
        .collect();
    let ids_tensor = match Tensor::from_vec(ids, (batch, seq_len), &device) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("tensor build: {e}");
            return ExitCode::FAILURE;
        }
    };

    let logits = match model.forward(&ids_tensor) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("forward: {e}");
            return ExitCode::FAILURE;
        }
    };

    let shape = logits.shape().clone();
    eprintln!(
        "forward OK: input ({}, {}) -> output {:?}",
        batch, seq_len, shape
    );

    // Sanity: last timestep logits as host Vec<f32>
    let last = match logits.i((0, seq_len - 1, ..)) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("slice last: {e}");
            return ExitCode::FAILURE;
        }
    };
    let last_host: Vec<f32> = match last.to_vec1() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("to_vec1: {e}");
            return ExitCode::FAILURE;
        }
    };
    let (max_idx, max_val) =
        last_host
            .iter()
            .enumerate()
            .fold((0usize, f32::NEG_INFINITY), |acc, (i, v)| {
                if *v > acc.1 { (i, *v) } else { acc }
            });
    let min_val = last_host.iter().cloned().fold(f32::INFINITY, f32::min);
    eprintln!(
        "last-position logits stats: min={:.4}, max={:.4} @ id={}",
        min_val, max_val, max_idx
    );

    eprintln!("smoke test passed");
    ExitCode::SUCCESS
}
