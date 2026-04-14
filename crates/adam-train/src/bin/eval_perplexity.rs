use std::{env, fs, path::PathBuf, process::ExitCode};

use adam_train::model::{AdamBaseline, ModelConfig, default_device};
use candle_core::{DType, Tensor};
use candle_nn::{VarBuilder, VarMap, loss};
use serde::{Deserialize, Serialize};

const DEFAULT_VAL_PACK: &str = "data/curated/adam_validation_ids_pack.json";
const DEFAULT_CHECKPOINT: &str = "data/training/adam_baseline_checkpoint.safetensors";
const DEFAULT_REPORT: &str = "data/training/validation_perplexity_report.json";

#[derive(Debug, Deserialize)]
struct ValSample {
    id: String,
    text: String,
    ids: Vec<u32>,
}

#[derive(Debug, Deserialize)]
struct ValPack {
    vocab_size: usize,
    samples: Vec<ValSample>,
}

#[derive(Debug, Serialize)]
struct PerplexityReport {
    version: String,
    run_name: String,
    checkpoint_path: String,
    val_pack_path: String,
    sample_count: usize,
    token_count: usize,
    skipped_sample_count: usize,
    total_cross_entropy: f64,
    mean_cross_entropy: f64,
    perplexity: f64,
    vocab_size: usize,
    per_sample_stats: Vec<SampleStat>,
}

#[derive(Debug, Serialize)]
struct SampleStat {
    id: String,
    text: String,
    token_count: usize,
    mean_cross_entropy: f64,
    perplexity: f64,
}

fn main() -> ExitCode {
    let checkpoint_path: PathBuf = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CHECKPOINT));
    let val_pack_path = env::args()
        .nth(2)
        .unwrap_or_else(|| DEFAULT_VAL_PACK.to_string());
    let report_path: PathBuf = env::args()
        .nth(3)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_REPORT));

    let device = match default_device() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("device: {e}");
            return ExitCode::FAILURE;
        }
    };

    let pack_raw = match fs::read_to_string(&val_pack_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot read val pack: {e}");
            return ExitCode::FAILURE;
        }
    };
    let pack: ValPack = match serde_json::from_str(&pack_raw) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("invalid val pack: {e}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!(
        "loaded val pack: {} samples, vocab={}",
        pack.samples.len(),
        pack.vocab_size
    );

    let cfg = ModelConfig::tiny();
    if cfg.vocab_size != pack.vocab_size {
        eprintln!(
            "WARN: model vocab {} != pack vocab {}",
            cfg.vocab_size, pack.vocab_size
        );
    }
    let mut varmap = VarMap::new();
    let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);
    let model = match AdamBaseline::new(&cfg, vb) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("model init: {e}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = varmap.load(&checkpoint_path) {
        eprintln!("checkpoint load: {e}");
        return ExitCode::FAILURE;
    }
    eprintln!("checkpoint loaded: {}", checkpoint_path.display());

    let mut total_ce = 0.0_f64;
    let mut total_tokens = 0usize;
    let mut skipped = 0usize;
    let mut per_sample: Vec<SampleStat> = Vec::with_capacity(pack.samples.len());

    for s in &pack.samples {
        if s.ids.len() < 2 {
            skipped += 1;
            continue;
        }
        let len = s.ids.len().min(cfg.max_seq_len);
        let input_ids: Vec<u32> = s.ids[..len - 1].to_vec();
        let target_ids: Vec<u32> = s.ids[1..len].to_vec();
        let t = input_ids.len();
        if t == 0 {
            skipped += 1;
            continue;
        }
        let input = match Tensor::from_vec(input_ids, (1, t), &device) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("tensor input: {e}");
                return ExitCode::FAILURE;
            }
        };
        let target = match Tensor::from_vec(target_ids, (1, t), &device) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("tensor target: {e}");
                return ExitCode::FAILURE;
            }
        };
        let logits = match model.forward(&input, false) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("forward: {e}");
                return ExitCode::FAILURE;
            }
        };
        let (_b, seqt, v) = match logits.dims3() {
            Ok(d) => d,
            Err(e) => {
                eprintln!("dims3: {e}");
                return ExitCode::FAILURE;
            }
        };
        let lf = match logits.reshape((seqt, v)) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("reshape logits: {e}");
                return ExitCode::FAILURE;
            }
        };
        let tf = match target.reshape((seqt,)) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("reshape target: {e}");
                return ExitCode::FAILURE;
            }
        };
        let ce = match loss::cross_entropy(&lf, &tf) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("cross_entropy: {e}");
                return ExitCode::FAILURE;
            }
        };
        let ce_val = ce.to_scalar::<f32>().unwrap_or(f32::NAN) as f64;

        total_ce += ce_val * seqt as f64;
        total_tokens += seqt;
        per_sample.push(SampleStat {
            id: s.id.clone(),
            text: s.text.clone(),
            token_count: seqt,
            mean_cross_entropy: ce_val,
            perplexity: ce_val.exp(),
        });
    }

    let mean_ce = if total_tokens == 0 {
        f64::NAN
    } else {
        total_ce / total_tokens as f64
    };
    let perplexity = mean_ce.exp();

    let report = PerplexityReport {
        version: env!("CARGO_PKG_VERSION").to_string(),
        run_name: "adam-baseline-perplexity".to_string(),
        checkpoint_path: checkpoint_path.display().to_string(),
        val_pack_path: val_pack_path.clone(),
        sample_count: pack.samples.len(),
        token_count: total_tokens,
        skipped_sample_count: skipped,
        total_cross_entropy: total_ce,
        mean_cross_entropy: mean_ce,
        perplexity,
        vocab_size: pack.vocab_size,
        per_sample_stats: per_sample,
    };

    if let Some(parent) = report_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            let _ = fs::create_dir_all(parent);
        }
    }
    match fs::write(
        &report_path,
        serde_json::to_string_pretty(&report).expect("serialize"),
    ) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("cannot write report: {e}");
            return ExitCode::FAILURE;
        }
    }

    eprintln!(
        "val_samples={} (skipped={}), token_count={}, mean_ce={:.4}, perplexity={:.2}",
        report.sample_count,
        report.skipped_sample_count,
        report.token_count,
        report.mean_cross_entropy,
        report.perplexity,
    );
    eprintln!("report written: {}", report_path.display());
    ExitCode::SUCCESS
}
