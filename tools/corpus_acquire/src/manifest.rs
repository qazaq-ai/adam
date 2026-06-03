// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Append-only manifest for the v6.3 phoneme bank.

use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct ManifestEntry {
    /// Short identifier (filename root).
    pub label: String,
    /// Source URL we downloaded from.
    pub source_url: String,
    /// Cyrillic transcript of the spoken content.
    pub transcript: String,
    /// Speaker gender hint: "male" / "female" / "mixed" / "unknown".
    pub gender: String,
    /// Provenance class (wikimedia / archive-org / common-voice / …).
    pub source_class: String,
    /// Bytes downloaded from the original source.
    pub original_bytes: u64,
    /// Duration of the curated 16 kHz mono audio (seconds).
    pub duration_s: f32,
    /// Path to the persisted 16 kHz mono WAV (relative to out_dir).
    pub wav_path: String,
    pub wav_bytes: u64,
    /// Path to the persisted MFCC binary file (relative to out_dir).
    pub mfcc_path: String,
    pub mfcc_frames: usize,
    pub mfcc_bytes: u64,
    /// ISO-8601 date of collection.
    pub collected_at: String,
    /// Flipped to `true` after the entry passes phoneme-bank
    /// quality gates (Phase 2d acceptance).
    pub used_in_bank: bool,
}

/// Append one manifest entry as one JSONL line.
pub fn append_manifest(path: &Path, entry: &ManifestEntry) -> std::io::Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let json = serde_json::to_string(entry).map_err(std::io::Error::other)?;
    writeln!(file, "{json}")?;
    Ok(())
}
