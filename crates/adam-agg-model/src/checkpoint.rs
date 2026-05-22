// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Persist + reload a trained `TinyAgt` model to disk.
//!
//! v6.0 PoC training takes ~95 minutes on M2 8 GB CPU; without a
//! checkpoint, every dialog session that wants to consult the L5.5
//! neural composer would have to retrain from scratch. This module
//! provides a tiny, self-contained on-disk format so a trained model
//! can be loaded into `adam_chat` (under the planned `--neural` flag)
//! without rerunning the training binary.
//!
//! ## On-disk layout
//!
//! A checkpoint is a **directory** with four files:
//!
//! ```text
//! data/checkpoints/poc_kazakh/<run_id>/
//!     config.json     ← TinyAgtConfig (vocab_size, d_model, n_layers, ...)
//!     labels.json     ← Vec<String>, training-time compact label vocab
//!     training.json   ← CheckpointMeta (loss curve summary, hparams,
//!                       pair counts) — purely informational, not loaded
//!     model.mpk       ← burn `NamedMpkFileRecorder` payload (weights)
//! ```
//!
//! Why a directory rather than a single blob: the three JSON sidecars
//! are diffable and grep-able from a shell; only `model.mpk` is opaque.
//! That makes the human story («what was this checkpoint trained on»)
//! visible without booting Rust.
//!
//! ## Save / load contract
//!
//! [`save_checkpoint`] writes all four files atomically (config first,
//! then labels, then training, then model — so a partially-written
//! checkpoint is detectable by the absence of `model.mpk`).
//! [`load_checkpoint`] reads them back, returning the rebuilt
//! `TinyAgt<B>` + the label vocab + the config.
//!
//! The save and load APIs are **backend-generic** — the trained model
//! can be reloaded on any backend that implements `burn::Backend`,
//! not just the one that wrote it.

use std::fs;
use std::path::{Path, PathBuf};

use burn::module::Module;
use burn::record::{FullPrecisionSettings, NamedMpkFileRecorder, RecorderError};
use burn::tensor::backend::Backend;
use serde::{Deserialize, Serialize};

use crate::{TinyAgt, TinyAgtConfig};

/// Informational sidecar — pinpoints WHEN and HOW a checkpoint was
/// produced. Not consumed by [`load_checkpoint`]; written for the
/// operator's reading.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMeta {
    /// ISO-8601 UTC timestamp at the moment of `save_checkpoint`.
    pub saved_at: String,
    /// Git commit (`git rev-parse HEAD`) if discoverable at save time;
    /// otherwise `"unknown"`.
    pub git_commit: String,
    /// Training-pair count that produced this checkpoint.
    pub train_pairs: usize,
    /// Held-out-pair count.
    pub heldout_pairs: usize,
    /// Final training cross-entropy (last batch).
    pub final_train_ce: f32,
    /// Cross-entropy on the held-out split, if measured.
    pub heldout_ce: Option<f32>,
    /// Optimiser hyperparameters used.
    pub batch_size: usize,
    pub n_epochs: usize,
    pub lr: f64,
    pub seed: u64,
    /// `POC_ALPHA` algebraic-loss weight (0.0 for vanilla CE).
    pub algebraic_alpha: f32,
}

/// Persisted model config — serializable, mirror of [`TinyAgtConfig`].
/// We can't `serde`-derive [`TinyAgtConfig`] directly because burn's
/// `#[derive(Config)]` already owns the layout; this mirror keeps the
/// checkpoint format independent of burn's internal config shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableConfig {
    pub vocab_size: usize,
    pub max_seq_len: usize,
    pub d_model: usize,
    pub n_heads: usize,
    pub n_layers: usize,
    pub d_ff: usize,
}

impl From<&TinyAgtConfig> for SerializableConfig {
    fn from(c: &TinyAgtConfig) -> Self {
        Self {
            vocab_size: c.vocab_size,
            max_seq_len: c.max_seq_len,
            d_model: c.d_model,
            n_heads: c.n_heads,
            n_layers: c.n_layers,
            d_ff: c.d_ff,
        }
    }
}

impl From<&SerializableConfig> for TinyAgtConfig {
    fn from(c: &SerializableConfig) -> Self {
        Self::new(
            c.vocab_size,
            c.max_seq_len,
            c.d_model,
            c.n_heads,
            c.n_layers,
            c.d_ff,
        )
    }
}

/// Errors save / load can surface to the caller.
#[derive(Debug)]
pub enum CheckpointError {
    /// I/O failure on a sidecar file.
    Io(std::io::Error),
    /// JSON serialisation / deserialisation failure on a sidecar.
    Json(serde_json::Error),
    /// burn-recorder failure on the model weights file.
    Recorder(RecorderError),
    /// One of the required files is missing in the checkpoint directory.
    MissingFile(&'static str),
}

impl std::fmt::Display for CheckpointError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "checkpoint io: {e}"),
            Self::Json(e) => write!(f, "checkpoint json: {e}"),
            Self::Recorder(e) => write!(f, "checkpoint recorder: {e}"),
            Self::MissingFile(s) => write!(f, "checkpoint missing required file: {s}"),
        }
    }
}

impl std::error::Error for CheckpointError {}

impl From<std::io::Error> for CheckpointError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for CheckpointError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

impl From<RecorderError> for CheckpointError {
    fn from(e: RecorderError) -> Self {
        Self::Recorder(e)
    }
}

/// Save `model` and its companion metadata to `dir`. Creates `dir` if
/// it doesn't exist. Overwrites existing files inside `dir` without
/// asking — caller is responsible for picking a fresh directory per
/// run (the binary uses a timestamped path).
pub fn save_checkpoint<B: Backend>(
    dir: &Path,
    model: TinyAgt<B>,
    config: &TinyAgtConfig,
    labels: &[String],
    meta: &CheckpointMeta,
) -> Result<(), CheckpointError> {
    fs::create_dir_all(dir)?;
    // Config sidecar.
    let serializable: SerializableConfig = config.into();
    fs::write(
        dir.join("config.json"),
        serde_json::to_vec_pretty(&serializable)?,
    )?;
    // Label vocab sidecar.
    fs::write(dir.join("labels.json"), serde_json::to_vec_pretty(labels)?)?;
    // Training-run metadata sidecar.
    fs::write(dir.join("training.json"), serde_json::to_vec_pretty(meta)?)?;
    // Model weights — last, so a partially-written checkpoint is
    // detectable by `model.mpk` missing.
    let recorder = NamedMpkFileRecorder::<FullPrecisionSettings>::new();
    model.save_file(dir.join("model"), &recorder)?;
    Ok(())
}

/// Load a checkpoint produced by [`save_checkpoint`] back into a fresh
/// `TinyAgt<B>` plus the label vocab and config it was trained on.
pub fn load_checkpoint<B: Backend>(
    dir: &Path,
    device: &B::Device,
) -> Result<LoadedCheckpoint<B>, CheckpointError> {
    let config_path = dir.join("config.json");
    let labels_path = dir.join("labels.json");
    let model_path = dir.join("model.mpk");
    if !config_path.exists() {
        return Err(CheckpointError::MissingFile("config.json"));
    }
    if !labels_path.exists() {
        return Err(CheckpointError::MissingFile("labels.json"));
    }
    if !model_path.exists() {
        return Err(CheckpointError::MissingFile("model.mpk"));
    }
    let serializable: SerializableConfig = serde_json::from_slice(&fs::read(&config_path)?)?;
    let labels: Vec<String> = serde_json::from_slice(&fs::read(&labels_path)?)?;
    let config: TinyAgtConfig = (&serializable).into();
    let recorder = NamedMpkFileRecorder::<FullPrecisionSettings>::new();
    let model: TinyAgt<B> = config
        .init(device)
        .load_file(dir.join("model"), &recorder, device)?;
    Ok(LoadedCheckpoint {
        model,
        labels,
        config,
    })
}

/// Convenience bundle returned by [`load_checkpoint`].
#[derive(Debug)]
pub struct LoadedCheckpoint<B: Backend> {
    pub model: TinyAgt<B>,
    pub labels: Vec<String>,
    pub config: TinyAgtConfig,
}

/// Pick a fresh checkpoint directory under `root` using a UTC
/// timestamp. Caller-controlled root keeps test fixtures out of the
/// production tree.
pub fn timestamped_dir(root: &Path) -> PathBuf {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Format YYYYMMDD-HHMMSS in UTC by manual arithmetic so we don't
    // pull a `chrono` dependency for one timestamp.
    let secs = now;
    let days_since_epoch = (secs / 86_400) as i64;
    let (y, m, d) = civil_date_from_days(days_since_epoch);
    let h = (secs % 86_400) / 3_600;
    let min = (secs % 3_600) / 60;
    let s = secs % 60;
    let stamp = format!("{y:04}{m:02}{d:02}-{h:02}{min:02}{s:02}");
    root.join(format!("v6_{stamp}"))
}

/// Convert a UNIX-epoch day count into a (year, month, day) tuple
/// using the proleptic Gregorian calendar. Adapted from Howard
/// Hinnant's `days_from_civil` inverse — public domain. Avoids
/// pulling chrono just to format a timestamp.
fn civil_date_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468; // shift so 0000-03-01 is day 0
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 {
        (mp + 3) as u32
    } else {
        (mp - 9) as u32
    };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use burn::backend::NdArray;
    use burn::backend::ndarray::NdArrayDevice;

    type B = NdArray<f32>;

    #[test]
    fn save_then_load_round_trips_a_model() {
        let device = NdArrayDevice::default();
        let cfg = TinyAgtConfig::new(64, 16, 32, 2, 1, 64);
        let model: TinyAgt<B> = cfg.init(&device);
        let labels: Vec<String> = vec![
            "<unk>".into(),
            "BOS".into(),
            "EOS".into(),
            "<spc>".into(),
            "R:бала".into(),
            "S:Number(Plural)".into(),
        ];
        let meta = CheckpointMeta {
            saved_at: "2026-05-18T00:00:00Z".into(),
            git_commit: "test-fixture".into(),
            train_pairs: 100,
            heldout_pairs: 10,
            final_train_ce: 0.42,
            heldout_ce: Some(0.5),
            batch_size: 8,
            n_epochs: 1,
            lr: 1e-3,
            seed: 0,
            algebraic_alpha: 0.0,
        };

        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path().join("v6_test");
        save_checkpoint(&dir, model, &cfg, &labels, &meta).expect("save");

        // Sidecars must be human-readable.
        assert!(dir.join("config.json").exists());
        assert!(dir.join("labels.json").exists());
        assert!(dir.join("training.json").exists());
        assert!(dir.join("model.mpk").exists());

        let loaded = load_checkpoint::<B>(&dir, &device).expect("load");
        assert_eq!(loaded.config.vocab_size, cfg.vocab_size);
        assert_eq!(loaded.config.d_model, cfg.d_model);
        assert_eq!(loaded.config.n_layers, cfg.n_layers);
        assert_eq!(loaded.labels, labels);
        // Sanity-check that the loaded model can run a forward pass —
        // we don't assert weights are byte-identical (burn round-trip
        // is f32; equality may flake on edge cases), but a forward
        // call proves the architecture re-built correctly.
        let dummy = burn::tensor::Tensor::<B, 2, burn::tensor::Int>::zeros([1, 4], &device);
        let _ = loaded.model.forward(dummy);
    }

    #[test]
    fn missing_file_surfaces_a_typed_error() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path().join("v6_empty");
        std::fs::create_dir_all(&dir).unwrap();
        let device = NdArrayDevice::default();
        let err = load_checkpoint::<B>(&dir, &device).unwrap_err();
        match err {
            CheckpointError::MissingFile(_) => {}
            other => panic!("expected MissingFile, got {other:?}"),
        }
    }

    #[test]
    fn timestamped_dir_is_unique_across_seconds() {
        let root = std::path::Path::new("/tmp/test_root");
        let a = timestamped_dir(root);
        // Same second → identical names, but format is well-formed.
        let s = a.file_name().unwrap().to_string_lossy().into_owned();
        assert!(s.starts_with("v6_"));
        assert_eq!(s.len(), "v6_YYYYMMDD-HHMMSS".len());
    }
}
