// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/caz
//! `ingest_kaz_tili` — Phase 6 step 3 (2026-05-29).
//!
//! Reads `data/kaz_tili/transcripts.jsonl` (the output of
//! `tools/scrape_kaz_tili/scrape.py`), and for each entry:
//!
//! 1. Loads the 16 kHz mono WAV.
//! 2. Computes MFCC with the canonical 13-coeff / hop 160 config.
//! 3. Writes `<bank-dir>/mfcc/<label>.bin`.
//! 4. Copies the WAV to `<bank-dir>/audio/<label>.wav`.
//! 5. Appends a manifest row with `source_class = "kaz_tili"`.
//!
//! Idempotent: labels already in the manifest are skipped, so
//! re-running picks up any newly scraped entries without
//! re-encoding.

use adam_audio::mfcc::{MfccConfig, mfcc, write_binary};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "ingest_kaz_tili",
    about = "Ingest scraped kaz-tili.kz audio into the v6.3 phoneme-bank manifest."
)]
struct Cli {
    /// Source directory containing `transcripts.jsonl` + `audio/`.
    #[arg(long, default_value = "data/kaz_tili")]
    src_dir: PathBuf,

    /// Destination phoneme-bank directory (the place that already
    /// holds `MANIFEST.jsonl`, `audio/`, `mfcc/`).
    #[arg(long, default_value = "data/v6_3_phoneme_bank")]
    bank_dir: PathBuf,

    /// Cap how many manifest rows to ingest in this run. `0` = no cap.
    #[arg(long, default_value_t = 0)]
    max: usize,
}

#[derive(Deserialize, Debug)]
struct ScrapeRow {
    label: String,
    transcript: String,
    duration_s: f64,
    source_url: String,
    source_page: String,
}

#[derive(Serialize, Debug)]
struct ManifestRow {
    label: String,
    source_url: String,
    transcript: String,
    gender: &'static str,
    source_class: &'static str,
    duration_s: f64,
    wav_path: String,
    wav_bytes: u64,
    mfcc_path: String,
    mfcc_frames: usize,
    mfcc_bytes: u64,
    collected_at: &'static str,
    used_in_bank: bool,
    source_page: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    eprintln!(
        "[ingest_kaz_tili] src={} bank={} max={}",
        cli.src_dir.display(),
        cli.bank_dir.display(),
        cli.max
    );

    let manifest_path = cli.bank_dir.join("MANIFEST.jsonl");
    std::fs::create_dir_all(cli.bank_dir.join("audio"))?;
    std::fs::create_dir_all(cli.bank_dir.join("mfcc"))?;

    // Load already-known labels for idempotent re-runs.
    let mut known_labels: HashSet<String> = HashSet::new();
    if manifest_path.exists() {
        let s = std::fs::read_to_string(&manifest_path)?;
        for line in s.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(l) = v.get("label").and_then(|x| x.as_str()) {
                    known_labels.insert(l.to_string());
                }
            }
        }
    }
    eprintln!(
        "[ingest_kaz_tili] manifest already has {} labels",
        known_labels.len()
    );

    let src_jsonl = cli.src_dir.join("transcripts.jsonl");
    let rows: Vec<ScrapeRow> = {
        let s = std::fs::read_to_string(&src_jsonl)?;
        let mut out = Vec::new();
        for line in s.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            match serde_json::from_str::<ScrapeRow>(line) {
                Ok(r) => out.push(r),
                Err(e) => eprintln!("[warn] parse: {e}"),
            }
        }
        out
    };
    eprintln!("[ingest_kaz_tili] scrape rows: {}", rows.len());

    let mut manifest_handle = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&manifest_path)?;

    let mut ingested = 0_usize;
    let mut skipped_dup = 0_usize;
    let mut skipped_io = 0_usize;
    let mut skipped_mfcc = 0_usize;

    for row in &rows {
        if known_labels.contains(&row.label) {
            skipped_dup += 1;
            continue;
        }
        if cli.max > 0 && ingested >= cli.max {
            break;
        }
        let src_wav = cli.src_dir.join("audio").join(format!("{}.wav", row.label));
        if !src_wav.exists() {
            eprintln!(
                "[warn] {}: source wav not found at {}",
                row.label,
                src_wav.display()
            );
            skipped_io += 1;
            continue;
        }
        let pcm = match adam_audio::wav::read_wav(&src_wav) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[warn] {}: read_wav: {e}", row.label);
                skipped_io += 1;
                continue;
            }
        };
        if pcm.sample_rate != 16_000 {
            eprintln!(
                "[warn] {}: sample_rate {} ≠ 16000 (skipping; ffmpeg downconvert expected)",
                row.label, pcm.sample_rate
            );
            skipped_mfcc += 1;
            continue;
        }
        let mono = if pcm.channels > 1 { pcm.to_mono() } else { pcm };
        let mfcc_seq = mfcc(&mono.data, mono.sample_rate, &MfccConfig::default());
        if mfcc_seq.num_frames() == 0 {
            eprintln!("[warn] {}: zero-frame mfcc", row.label);
            skipped_mfcc += 1;
            continue;
        }

        // Persist WAV (copy) + MFCC bin under bank-dir.
        let dst_wav = cli
            .bank_dir
            .join("audio")
            .join(format!("{}.wav", row.label));
        if !dst_wav.exists() {
            std::fs::copy(&src_wav, &dst_wav)?;
        }
        let dst_mfcc = cli.bank_dir.join("mfcc").join(format!("{}.bin", row.label));
        let bytes = write_binary(&mfcc_seq);
        std::fs::write(&dst_mfcc, &bytes)?;

        let manifest_row = ManifestRow {
            label: row.label.clone(),
            source_url: row.source_url.clone(),
            transcript: row.transcript.clone(),
            gender: "unknown",
            source_class: "kaz_tili",
            duration_s: row.duration_s,
            wav_path: format!("audio/{}.wav", row.label),
            wav_bytes: std::fs::metadata(&dst_wav)?.len(),
            mfcc_path: format!("mfcc/{}.bin", row.label),
            mfcc_frames: mfcc_seq.num_frames(),
            mfcc_bytes: std::fs::metadata(&dst_mfcc)?.len(),
            collected_at: "2026-05-29",
            used_in_bank: false,
            source_page: row.source_page.clone(),
        };
        let json = serde_json::to_string(&manifest_row)?;
        use std::io::Write;
        writeln!(manifest_handle, "{json}")?;
        ingested += 1;
        known_labels.insert(row.label.clone());

        if ingested % 50 == 0 {
            eprintln!("[ingest_kaz_tili] {ingested} ingested");
        }
    }
    eprintln!("[ingest_kaz_tili] === done ===");
    eprintln!("[ingest_kaz_tili] ingested        : {ingested}");
    eprintln!("[ingest_kaz_tili] skipped (dup)   : {skipped_dup}");
    eprintln!("[ingest_kaz_tili] skipped (io)    : {skipped_io}");
    eprintln!("[ingest_kaz_tili] skipped (mfcc)  : {skipped_mfcc}");
    Ok(())
}
