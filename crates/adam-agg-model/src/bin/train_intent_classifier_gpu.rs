// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! **Phase 19 step C (2026-06-02)** — Train the pooled-embedding
//! intent classifier on the labeled pack from step A.
//!
//! ## Inputs
//!
//!   data/curated/adam_intent_training_pack.json
//!     1 180 (text, intent) labeled samples × 52 intent labels.
//!
//!   data/tokenizer/bpe_vocab.json + bpe_merges.json + segmentation_*.json
//!     The same BPE-5188 tokenizer the v6 LM uses, so embedding
//!     warm-start (Phase 19.B `load_from_lm` future enhancement)
//!     stays aligned.
//!
//!   data/checkpoints/contextual_lm/  (optional)
//!     v6 LM checkpoint — we read its `vocab_size` and `d_model`
//!     so the classifier's embedding layer matches.  Warm-start
//!     copy of the embedding weights is not yet implemented in
//!     this pass; for v0 we train embeddings from random init and
//!     measure baseline accuracy.  If accuracy is too low, the
//!     follow-up commit can copy v6 LM embeddings.
//!
//! ## Output
//!
//!   data/checkpoints/intent_classifier/
//!     config.json + labels.json (= the 52 intent strings) +
//!     model.mpk + training.json.
//!
//! ## Run
//!
//!   cargo run --release -p adam-agg-model --bin train_intent_classifier_gpu
//!
//! Env vars: ICX_EPOCHS (8), ICX_BATCH (32), ICX_LR (1e-3),
//!           ICX_DMODEL (192), ICX_HIDDEN (256), ICX_MAXSEQ (32),
//!           ICX_VAL_FRAC (0.1), ICX_OUTDIR (data/checkpoints/intent_classifier).
//!
//! ## Notes
//!
//!   - Trains on GPU via burn-wgpu (Metal on Apple Silicon).
//!   - Splits 90/10 train/val deterministically by index hash.
//!   - Reports per-epoch train+val loss + val top-1 accuracy.
//!   - Padding is mask-aware so short utterances (most of them)
//!     pool only over real tokens.

use std::path::{Path, PathBuf};
use std::time::Instant;

use adam_agg_model::checkpoint::{CheckpointMeta, save_checkpoint};
use adam_agg_model::intent_classifier::{IntentClassifier, IntentClassifierConfig};
use adam_kernel::{SegmentationLexicon, SegmentationRuleSet};
use adam_tokenizer::bpe::BpeTokenizer;
use burn::backend::Autodiff;
use burn::backend::wgpu::{Wgpu, WgpuDevice};
use burn::optim::adaptor::OptimizerAdaptor;
use burn::optim::{Adam, AdamConfig, GradientsParams, Optimizer};
use burn::prelude::*;
use burn::record::{FullPrecisionSettings, NamedMpkFileRecorder};
use serde::Deserialize;

type B = Autodiff<Wgpu<f32, i32>>;

const TRAIN_PACK: &str = "data/curated/adam_intent_training_pack.json";
const BPE_VOCAB: &str = "data/tokenizer/bpe_vocab.json";
const BPE_MERGES: &str = "data/tokenizer/bpe_merges.json";
const SEG_ROOTS: &str = "data/tokenizer/segmentation_roots.json";
const SEG_RULES: &str = "data/tokenizer/segmentation_rules.json";

#[derive(Debug, Deserialize)]
struct Sample {
    text: String,
    intent: String,
}

#[derive(Debug, Deserialize)]
struct Pack {
    intents: Vec<String>,
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

fn load_json<T: serde::de::DeserializeOwned>(path: &str) -> T {
    let raw = std::fs::read_to_string(path).expect(path);
    serde_json::from_str(&raw).expect(path)
}

/// Encode `text` through BPE + segmentation, prepend BOS, append EOS,
/// truncate to `max_seq_len`. Returns (ids, mask) padded to max_seq_len.
fn encode(
    text: &str,
    bpe: &BpeTokenizer,
    lex: &SegmentationLexicon,
    rules: &SegmentationRuleSet,
    max_seq_len: usize,
) -> (Vec<i64>, Vec<f32>) {
    let mut ids: Vec<u32> = vec![bpe.bos_id];
    ids.extend(bpe.encode(text, lex, rules));
    ids.push(bpe.eos_id);
    let take = ids.len().min(max_seq_len);
    let mut out_ids = vec![0i64; max_seq_len];
    let mut out_mask = vec![0.0_f32; max_seq_len];
    for i in 0..take {
        out_ids[i] = ids[i] as i64;
        out_mask[i] = 1.0;
    }
    (out_ids, out_mask)
}

fn main() {
    let device = WgpuDevice::default();

    // ---- 1. Load labeled pack -----------------------------------
    //
    // **v6.5.0-rc1 (Movement B)**: allow CLI override of the
    // training pack path so we can A/B train on the STT-noise-
    // augmented pack from `tools/intent_dataset/src/stt_augment.rs`
    // without losing the ability to retrain on the canonical
    // pack.  Usage:
    //   cargo run --release --bin train_intent_classifier_gpu \
    //     -- --pack data/curated/adam_intent_training_pack_stt_augmented.json
    //
    // Default keeps the historical behaviour — train on the
    // canonical pack.
    let pack_path = std::env::args()
        .collect::<Vec<_>>()
        .windows(2)
        .find_map(|w| {
            if w[0] == "--pack" {
                Some(w[1].clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| TRAIN_PACK.to_string());

    if !Path::new(&pack_path).exists() {
        eprintln!("[icx] missing {pack_path} — run intent-pack builder first");
        std::process::exit(2);
    }
    eprintln!("[icx] training pack: {pack_path}");
    let pack: Pack = load_json(&pack_path);
    eprintln!(
        "[1/4] Pack: {} samples × {} intent labels",
        pack.samples.len(),
        pack.intents.len()
    );

    // Intent string → dense id.
    let intent_to_id: std::collections::HashMap<&str, usize> = pack
        .intents
        .iter()
        .enumerate()
        .map(|(i, s)| (s.as_str(), i))
        .collect();

    // ---- 2. Load tokenizer --------------------------------------
    let bpe = BpeTokenizer::load(BPE_VOCAB, BPE_MERGES).expect("BPE load");
    let lex: SegmentationLexicon = load_json(SEG_ROOTS);
    let rules: SegmentationRuleSet = load_json(SEG_RULES);
    let vocab_size = bpe.vocab_size();
    eprintln!("       BPE vocab: {vocab_size} tokens");

    let max_seq_len = env_usize("ICX_MAXSEQ", 32);

    // ---- 3. Tokenise all samples --------------------------------
    let mut all_ids: Vec<Vec<i64>> = Vec::with_capacity(pack.samples.len());
    let mut all_mask: Vec<Vec<f32>> = Vec::with_capacity(pack.samples.len());
    let mut all_labels: Vec<i64> = Vec::with_capacity(pack.samples.len());
    let mut dropped = 0;
    for s in &pack.samples {
        let label = match intent_to_id.get(s.intent.as_str()) {
            Some(&id) => id as i64,
            None => {
                dropped += 1;
                continue;
            }
        };
        let (ids, mask) = encode(&s.text, &bpe, &lex, &rules, max_seq_len);
        all_ids.push(ids);
        all_mask.push(mask);
        all_labels.push(label);
    }
    if dropped > 0 {
        eprintln!("       Dropped {dropped} samples with unknown intent string");
    }

    // Deterministic 90/10 split by index.
    let val_frac = env_f64("ICX_VAL_FRAC", 0.1);
    let n = all_ids.len();
    let val_n = ((n as f64) * val_frac) as usize;
    let mut train_idx: Vec<usize> = Vec::with_capacity(n - val_n);
    let mut val_idx: Vec<usize> = Vec::with_capacity(val_n);
    for i in 0..n {
        // hash(i) % 10 == 0 → val.
        let h = (i as u64).wrapping_mul(2654435761).wrapping_add(0xDEADBEEF);
        if (h % 10) < (val_frac * 10.0) as u64 {
            val_idx.push(i);
        } else {
            train_idx.push(i);
        }
    }
    eprintln!(
        "[2/4] Split: {} train / {} val",
        train_idx.len(),
        val_idx.len()
    );

    // ---- 4. Build model -----------------------------------------
    let d_model = env_usize("ICX_DMODEL", 192);
    let hidden = env_usize("ICX_HIDDEN", 256);
    let cfg = IntentClassifierConfig::new(vocab_size, d_model, hidden, pack.intents.len());
    let mut model: IntentClassifier<B> = cfg.init(&device);
    eprintln!(
        "[3/4] Model: vocab={vocab_size} d_model={d_model} hidden={hidden} n_intents={}",
        pack.intents.len()
    );

    // ---- 5. Training loop ---------------------------------------
    let batch_size = env_usize("ICX_BATCH", 32);
    let epochs = env_usize("ICX_EPOCHS", 8);
    let lr = env_f64("ICX_LR", 1e-3);
    eprintln!("[4/4] Training: batch={batch_size} epochs={epochs} lr={lr}");

    let mut optim: OptimizerAdaptor<Adam, IntentClassifier<B>, B> = AdamConfig::new().init();

    let started = Instant::now();
    let mut final_train_ce = f32::NAN;

    for epoch in 0..epochs {
        // Shuffle train_idx (rotate by epoch number for determinism).
        let mut order: Vec<usize> = train_idx.clone();
        let rot = epoch % order.len().max(1);
        order.rotate_left(rot);

        let mut epoch_loss = 0.0_f32;
        let mut epoch_steps = 0usize;
        for chunk in order.chunks(batch_size) {
            let bs = chunk.len();
            let mut ids_buf: Vec<i64> = Vec::with_capacity(bs * max_seq_len);
            let mut mask_buf: Vec<f32> = Vec::with_capacity(bs * max_seq_len);
            let mut lbl_buf: Vec<i64> = Vec::with_capacity(bs);
            for &i in chunk {
                ids_buf.extend_from_slice(&all_ids[i]);
                mask_buf.extend_from_slice(&all_mask[i]);
                lbl_buf.push(all_labels[i]);
            }
            let tokens: Tensor<B, 2, Int> = Tensor::from_data(
                burn::tensor::TensorData::new(ids_buf, [bs, max_seq_len]),
                &device,
            );
            let mask: Tensor<B, 2> = Tensor::from_data(
                burn::tensor::TensorData::new(mask_buf, [bs, max_seq_len]),
                &device,
            );
            let labels: Tensor<B, 1, Int> =
                Tensor::from_data(burn::tensor::TensorData::new(lbl_buf, [bs]), &device);
            let logits = model.forward(tokens, mask);
            let loss = model.loss(logits, labels);
            let loss_value: f32 = loss.clone().into_scalar().elem();
            epoch_loss += loss_value;
            epoch_steps += 1;
            let grads = loss.backward();
            let grads = GradientsParams::from_grads(grads, &model);
            model = optim.step(lr, model, grads);
        }
        let train_ce = epoch_loss / epoch_steps.max(1) as f32;

        // Val pass.
        let mut val_correct = 0usize;
        let mut val_total = 0usize;
        let mut val_loss = 0.0_f32;
        for chunk in val_idx.chunks(batch_size) {
            let bs = chunk.len();
            let mut ids_buf: Vec<i64> = Vec::with_capacity(bs * max_seq_len);
            let mut mask_buf: Vec<f32> = Vec::with_capacity(bs * max_seq_len);
            let mut lbl_buf: Vec<i64> = Vec::with_capacity(bs);
            for &i in chunk {
                ids_buf.extend_from_slice(&all_ids[i]);
                mask_buf.extend_from_slice(&all_mask[i]);
                lbl_buf.push(all_labels[i]);
            }
            let tokens: Tensor<B, 2, Int> = Tensor::from_data(
                burn::tensor::TensorData::new(ids_buf, [bs, max_seq_len]),
                &device,
            );
            let mask: Tensor<B, 2> = Tensor::from_data(
                burn::tensor::TensorData::new(mask_buf, [bs, max_seq_len]),
                &device,
            );
            let labels: Tensor<B, 1, Int> = Tensor::from_data(
                burn::tensor::TensorData::new(lbl_buf.clone(), [bs]),
                &device,
            );
            let logits = model.forward(tokens, mask);
            let loss = model.loss(logits.clone(), labels);
            val_loss += loss.into_scalar().elem::<f32>();
            // Argmax per row → label match.
            let pred_data = logits.argmax(1).into_data().convert::<i64>();
            let preds: Vec<i64> = pred_data.into_vec().unwrap_or_default();
            for (p, t) in preds.iter().zip(lbl_buf.iter()) {
                if p == t {
                    val_correct += 1;
                }
                val_total += 1;
            }
        }
        let val_ce = val_loss / val_idx.chunks(batch_size).count().max(1) as f32;
        let val_acc = if val_total == 0 {
            0.0
        } else {
            val_correct as f32 / val_total as f32
        };
        eprintln!(
            "  epoch {:>2}/{}: train_ce={:.4} val_ce={:.4} val_acc={:.1}%",
            epoch + 1,
            epochs,
            train_ce,
            val_ce,
            val_acc * 100.0,
        );
        final_train_ce = train_ce;
    }
    let elapsed = started.elapsed();
    eprintln!("[train] done in {:.1}s", elapsed.as_secs_f32());

    // ---- 6. Save checkpoint -------------------------------------
    let outdir = env_string("ICX_OUTDIR", "data/checkpoints/intent_classifier");
    let outpath = PathBuf::from(&outdir);
    std::fs::create_dir_all(&outpath).expect("create outdir");

    // Save labels (intent strings) + config + meta + weights.
    std::fs::write(
        outpath.join("labels.json"),
        serde_json::to_vec_pretty(&pack.intents).unwrap(),
    )
    .unwrap();
    #[derive(serde::Serialize)]
    struct ClassifierConfig {
        vocab_size: usize,
        d_model: usize,
        hidden: usize,
        n_intents: usize,
        max_seq_len: usize,
    }
    let cfg_out = ClassifierConfig {
        vocab_size,
        d_model,
        hidden,
        n_intents: pack.intents.len(),
        max_seq_len,
    };
    std::fs::write(
        outpath.join("config.json"),
        serde_json::to_vec_pretty(&cfg_out).unwrap(),
    )
    .unwrap();
    let meta = CheckpointMeta {
        saved_at: env_string("ICX_TIMESTAMP", "phase-19-intent-classifier"),
        git_commit: std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        train_pairs: train_idx.len(),
        heldout_pairs: val_idx.len(),
        final_train_ce,
        heldout_ce: None,
        batch_size,
        n_epochs: epochs,
        lr,
        seed: 0,
        algebraic_alpha: 0.0,
    };
    std::fs::write(
        outpath.join("training.json"),
        serde_json::to_vec_pretty(&meta).unwrap(),
    )
    .unwrap();
    let recorder = NamedMpkFileRecorder::<FullPrecisionSettings>::new();
    use burn::module::Module;
    model
        .save_file(outpath.join("model"), &recorder)
        .expect("save model");
    eprintln!("[save] checkpoint → {}", outpath.display());

    // Silence the unused-ctor warning if save_checkpoint helper is
    // referenced elsewhere later — keep import alive.
    let _ = save_checkpoint::<B>;
}
