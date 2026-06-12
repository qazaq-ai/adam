// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! **Phase 15g.C.2 (2026-06-01)** — train a contextual language
//! model on the BPE pretokenized Kazakh corpus.
//!
//! Replaces the static fuzzy-vocabulary patch loop with a real
//! contextual rescorer. The voice REPL post-Whisper pipeline will
//! query this model with `score_sequence(tokens) -> log_prob` and
//! pick the highest-probability candidate among phonetic
//! neighbours.
//!
//! Training data:
//!   `data/curated/adam_training_ids_pack.json`
//!     63 362 sequences / 759k tokens
//!     vocab_size = 5188 (BPE), bos=1, eos=2, pad=0, unk=3
//!   `data/curated/adam_validation_ids_pack.json`
//!     ~3 300 held-out sequences (loss only, no weight update)
//!
//! Output:
//!   `data/checkpoints/contextual_lm/`
//!     config.json + labels.json + model.mpk + training.json
//!
//! Run from repo root:
//!   cargo run --release -p adam-agg-model --bin train_contextual_lm
//!
//! Hyperparameters can be overridden via env vars:
//!   CLM_EPOCHS  = number of epochs (default 3)
//!   CLM_BATCH   = batch size (default 32)
//!   CLM_LR      = learning rate (default 1e-3)
//!   CLM_DMODEL  = hidden width (default 128)
//!   CLM_LAYERS  = transformer layers (default 2)
//!   CLM_HEADS   = attention heads (default 4)
//!   CLM_DFF     = FFN inner width (default 256)
//!   CLM_MAXSEQ  = max sequence length (default 64)
//!   CLM_OUTDIR  = checkpoint dir (default data/checkpoints/contextual_lm)
//!
//! Hardware: CPU only (ndarray backend). On M2 8 GB expect roughly
//! 30-60 minutes per epoch at the default ~2 M-param size; the
//! whole run with 3 epochs ≈ 2-3 hours. Smaller config
//! (`CLM_DMODEL=64 CLM_LAYERS=2`) drops to ~1 M params and ~10-20 min
//! per epoch — useful for the first sanity pass.

use std::path::PathBuf;
use std::time::Instant;

use adam_agg_model::TinyAgt;
use adam_agg_model::TinyAgtConfig;
use adam_agg_model::checkpoint::{CheckpointMeta, load_checkpoint, save_checkpoint};
use adam_agg_model::train::{TrainConfig, train_next_token};
use burn::backend::Autodiff;
use burn::backend::wgpu::{Wgpu, WgpuDevice};
use serde::Deserialize;

// **Phase 15g.F (2026-06-01)** — Wgpu backend variant. On Apple
// Silicon (M1/M2/M3) wgpu auto-selects Metal under the hood, so
// this binary trains on the integrated GPU (~10 cores on M2)
// instead of the CPU. Larger batch sizes pay off here — matmul
// throughput is the dominant cost on GPU.
type B = Autodiff<Wgpu<f32, i32>>;

// **v6.6 generative pivot (2026-06-11)** — paths overridable via env
// so we can train on the 18.7M-token merged 8-pack corpus without
// renaming files. CLM_TRAIN_PACK / CLM_VAL_PACK fall back to the
// rc27 baseline 759k-token pack when unset.
const TRAIN_PACK_DEFAULT: &str = "data/curated/adam_training_ids_pack.json";
const VAL_PACK_DEFAULT: &str = "data/curated/adam_validation_ids_pack.json";

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Sample {
    #[serde(default)]
    id: String,
    #[serde(default)]
    text: String,
    ids: Vec<i64>,
}

#[derive(Debug, Deserialize)]
struct Pack {
    vocab_size: usize,
    samples: Vec<Sample>,
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn env_f64(key: &str, default: f64) -> f64 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn env_string(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn main() {
    let device = WgpuDevice::default();

    // ---- 1. Load BPE-pretokenized data ----------------------------
    let train_path = env_string("CLM_TRAIN_PACK", TRAIN_PACK_DEFAULT);
    if !PathBuf::from(&train_path).exists() {
        eprintln!("[clm-train] missing {train_path}; run from repo root");
        std::process::exit(2);
    }
    eprintln!("[clm-train] train pack: {train_path}");
    let train_bytes = std::fs::read(&train_path).expect("read train pack");
    let train_pack: Pack = serde_json::from_slice(&train_bytes).expect("parse train pack");
    eprintln!(
        "[1/4] Train pack: {} sequences, vocab_size={}",
        train_pack.samples.len(),
        train_pack.vocab_size
    );

    // **Phase 15g.D / 15g.E (2026-06-01)** — optional dialog /
    // Q&A corpus packs. `CLM_DIALOG_PACK` accepts a single path
    // OR a colon-separated list of paths; every pack is loaded
    // and concatenated, then the combined set is upsampled
    // `CLM_DIALOG_UPSAMPLE` times relative to the natural pack.
    let dialog_packs: Vec<Pack> = env_string("CLM_DIALOG_PACK", "")
        .split(':')
        .filter(|s| !s.is_empty())
        .filter_map(|p| {
            let path = PathBuf::from(p);
            if !path.exists() {
                eprintln!("       dialog pack missing: {p}");
                return None;
            }
            std::fs::read(&path)
                .ok()
                .and_then(|b| serde_json::from_slice::<Pack>(&b).ok())
                .inspect(|pack| {
                    eprintln!(
                        "       Dialog pack: {} loaded ({} sequences)",
                        p,
                        pack.samples.len()
                    );
                })
        })
        .collect();
    let dialog_upsample = env_usize("CLM_DIALOG_UPSAMPLE", 6);
    if !dialog_packs.is_empty() {
        let total: usize = dialog_packs.iter().map(|p| p.samples.len()).sum();
        eprintln!(
            "       Total dialog sequences: {} (upsample ×{} → {} mixed-in)",
            total,
            dialog_upsample,
            total * dialog_upsample
        );
    }

    let val_path = env_string("CLM_VAL_PACK", VAL_PACK_DEFAULT);
    let val_pack: Option<Pack> = if PathBuf::from(&val_path).exists() {
        eprintln!("[clm-train] val pack: {val_path}");
        let bytes = std::fs::read(&val_path).expect("read val pack");
        Some(serde_json::from_slice(&bytes).expect("parse val pack"))
    } else {
        None
    };
    if let Some(v) = &val_pack {
        eprintln!("       Validation pack: {} sequences", v.samples.len());
    }

    let vocab_size = train_pack.vocab_size;

    // ---- 2. Build model ------------------------------------------
    let d_model = env_usize("CLM_DMODEL", 128);
    let n_layers = env_usize("CLM_LAYERS", 2);
    let n_heads = env_usize("CLM_HEADS", 4);
    let d_ff = env_usize("CLM_DFF", 256);
    let max_seq_len = env_usize("CLM_MAXSEQ", 64);
    let cfg = TinyAgtConfig::new(vocab_size, max_seq_len, d_model, n_heads, n_layers, d_ff);
    eprintln!(
        "[2/4] Model: vocab={} d_model={} layers={} heads={} d_ff={} max_seq={}",
        vocab_size, d_model, n_layers, n_heads, d_ff, max_seq_len
    );
    // **Phase 15g.D** — optional resume-from-checkpoint. When set,
    // we load weights from an existing run instead of initialising
    // fresh, then keep training. Config dims must match.
    let resume_from = env_string("CLM_RESUME_FROM", "");
    let model: TinyAgt<B> = if !resume_from.is_empty() {
        match load_checkpoint::<B>(std::path::Path::new(&resume_from), &device) {
            Ok(c) => {
                eprintln!("       Resuming from checkpoint: {resume_from}");
                c.model
            }
            Err(e) => {
                eprintln!("       Resume failed ({e}); falling back to fresh init");
                cfg.init(&device)
            }
        }
    } else {
        cfg.init(&device)
    };

    // ---- 3. Build training sequences (i64) ------------------------
    // Drop empties; truncate at max_seq_len; the model's `train_next_token`
    // does its own pad/truncate so we just pass Vec<i64> per sample.
    let mut sequences: Vec<Vec<i64>> = train_pack
        .samples
        .into_iter()
        .filter_map(|s| if s.ids.is_empty() { None } else { Some(s.ids) })
        .collect();
    let natural_count = sequences.len();

    // **Phase 15g.D / 15g.E** — mix in every dialog pack
    // `dialog_upsample` times so they have comparable weight to
    // the natural-text pack.
    let mut dialog_added = 0usize;
    let mut all_dialog_seqs: Vec<Vec<i64>> = Vec::new();
    for d in dialog_packs {
        for s in d.samples {
            if !s.ids.is_empty() {
                all_dialog_seqs.push(s.ids);
            }
        }
    }
    for _ in 0..dialog_upsample {
        for seq in &all_dialog_seqs {
            sequences.push(seq.clone());
            dialog_added += 1;
        }
    }
    eprintln!(
        "[3/4] Built {} training sequences ({} natural + {} dialog repeats)",
        sequences.len(),
        natural_count,
        dialog_added
    );

    // ---- 4. Train -------------------------------------------------
    let tc = TrainConfig {
        batch_size: env_usize("CLM_BATCH", 32),
        n_epochs: env_usize("CLM_EPOCHS", 3),
        lr: env_f64("CLM_LR", 1e-3),
        seed: 42,
    };
    eprintln!(
        "[4/4] Training: batch={} epochs={} lr={}",
        tc.batch_size, tc.n_epochs, tc.lr
    );
    let started = Instant::now();
    let (trained, reports) = train_next_token::<B>(model, &sequences, &tc, &device);
    let elapsed = started.elapsed();

    // ---- Summary --------------------------------------------------
    if let (Some(first), Some(last)) = (reports.first(), reports.last()) {
        eprintln!(
            "[train] {} steps over {:.1}s. loss: {:.4} → {:.4}",
            reports.len(),
            elapsed.as_secs_f32(),
            first.loss,
            last.loss
        );
    }

    // ---- Save -----------------------------------------------------
    let outdir = env_string("CLM_OUTDIR", "data/checkpoints/contextual_lm");
    let outpath = PathBuf::from(&outdir);
    // Empty label vocab — the BPE token IDs are dense 0..vocab_size,
    // so we don't need a label sidecar for inference (the consumer
    // reads bpe_vocab.json directly).
    let labels: Vec<String> = Vec::new();
    let final_train_ce = reports.last().map(|r| r.loss).unwrap_or(f32::NAN);
    let meta = CheckpointMeta {
        saved_at: env_string("CLM_TIMESTAMP", "phase-15g-c2-contextual-lm"),
        git_commit: std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        train_pairs: sequences.len(),
        heldout_pairs: val_pack.as_ref().map(|v| v.samples.len()).unwrap_or(0),
        final_train_ce,
        heldout_ce: None,
        batch_size: tc.batch_size,
        n_epochs: tc.n_epochs,
        lr: tc.lr,
        seed: tc.seed,
        algebraic_alpha: 0.0,
    };
    save_checkpoint(&outpath, trained, &cfg, &labels, &meta).expect("save checkpoint");
    eprintln!("[save] checkpoint → {}", outpath.display());
}
