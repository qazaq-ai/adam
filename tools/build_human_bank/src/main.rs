// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `build_human_bank` — Phase 6 step 2 (2026-05-29).
//!
//! Reads `data/v6_3_phoneme_bank/MANIFEST.jsonl`, filters to
//! **human-voice** utterances (Wikimedia + FLEURS train/dev,
//! never FLEURS test — the PER eval split must stay held-out),
//! and for each one:
//!
//! 1. Loads the cached MFCC (`mfcc/<label>.bin`).
//! 2. Converts the transcript to phonemes via
//!    `cyrillic_to_phonemes(*, true)` (per-word `is_native_root`
//!    is honoured token by token).
//! 3. Forced-aligns the audio against the phoneme sequence using
//!    the **bootstrap** bank passed via `--bootstrap-bank` (the
//!    existing synth `templates.bin` — good enough for rough
//!    boundary placement).
//! 4. Walks the resulting `PhoneSegment`s and pushes one MFCC
//!    sub-sequence per segment into the output bank, capped at
//!    `--per-phoneme-cap` exemplars per phoneme to keep on-disk
//!    size bounded.
//!
//! The output bank is written via `PhonemeBank::save_to_file`
//! using format v2 (multi-template). Replaces the synth bank's
//! tone-and-formant fingerprints with **real human-speech MFCC
//! exemplars** — closes the MFCC-space mismatch that pinned
//! FLEURS PER at the 84 % floor.
//!
//! Tokens whose forced alignment fails (uncovered phoneme,
//! n_phones > n_frames, empty audio) are skipped with an
//! `[warn]` line, not aborted — bad transcripts shouldn't kill a
//! 4000-utterance batch.

use adam_audio::mfcc::{MfccSequence, read_binary};
use adam_forced_aligner::{AlignError, align};
use adam_phoneme::cyrillic::cyrillic_to_phonemes;
use adam_stt_phoneme::{PhonemeBank, PhonemeTemplate};
use clap::Parser;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(
    name = "build_human_bank",
    about = "Forced-align human Kazakh utterances → per-phoneme MFCC bank."
)]
struct Cli {
    /// Path to the v6.3 phoneme bank directory (must contain
    /// `MANIFEST.jsonl` and `mfcc/`).
    #[arg(long, default_value = "data/v6_3_phoneme_bank")]
    bank_dir: PathBuf,

    /// Bootstrap bank used for the first round of forced
    /// alignment. Typically the existing synth-derived
    /// `templates.bin`.
    #[arg(long, default_value = "data/v6_3_phoneme_bank/templates.bin")]
    bootstrap_bank: PathBuf,

    /// Output bank path (multi-template format v2).
    #[arg(long, default_value = "data/v6_3_phoneme_bank/templates_human.bin")]
    out: PathBuf,

    /// Max exemplars per phoneme. Same default as the stream
    /// recogniser's K.
    #[arg(long, default_value_t = 50)]
    per_phoneme_cap: usize,

    /// Cap how many manifest rows to consume. `0` = no cap.
    #[arg(long, default_value_t = 0)]
    max: usize,

    /// Which manifest rows to consume. `human` excludes
    /// FLEURS test. `human-all` includes FLEURS test (useful
    /// for diagnostics only — DO NOT use for a bank that will
    /// score the PER eval). `wikimedia-only` is the tiniest
    /// MVP slice.
    #[arg(long, default_value = "human")]
    source: String,
}

#[derive(Deserialize, Debug)]
struct ManifestRow {
    label: String,
    transcript: String,
    source_class: String,
    mfcc_path: String,
    mfcc_frames: usize,
    duration_s: f64,
}

fn main() -> std::io::Result<()> {
    let cli = Cli::parse();
    eprintln!(
        "[build_human_bank] bank_dir={} bootstrap={} out={} cap={} source={}",
        cli.bank_dir.display(),
        cli.bootstrap_bank.display(),
        cli.out.display(),
        cli.per_phoneme_cap,
        cli.source,
    );

    let bootstrap = PhonemeBank::load_from_file(&cli.bootstrap_bank).unwrap_or_else(|e| {
        eprintln!(
            "[build_human_bank] FATAL: cannot load bootstrap bank {}: {e}",
            cli.bootstrap_bank.display()
        );
        std::process::exit(2);
    });
    eprintln!(
        "[build_human_bank] bootstrap covers {} phonemes / {} templates",
        bootstrap.len(),
        bootstrap.template_count()
    );

    let manifest_path = cli.bank_dir.join("MANIFEST.jsonl");
    let manifest = std::fs::read_to_string(&manifest_path)?;
    let mut rows: Vec<ManifestRow> = Vec::new();
    for line in manifest.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<ManifestRow>(line) {
            Ok(r) => rows.push(r),
            Err(e) => eprintln!("[warn] manifest parse error ({e}): {line:.80}"),
        }
    }
    eprintln!("[build_human_bank] manifest rows: {}", rows.len());

    let mut kept: Vec<ManifestRow> = rows
        .into_iter()
        .filter(|r| match cli.source.as_str() {
            "wikimedia-only" => r.source_class == "wikimedia",
            "human" => {
                r.source_class == "wikimedia"
                    || (r.source_class == "fleurs" && !r.label.starts_with("fleurs_test_"))
                    || r.source_class == "common_voice"
            }
            "human-all" => matches!(
                r.source_class.as_str(),
                "wikimedia" | "fleurs" | "common_voice"
            ),
            other => {
                eprintln!("[build_human_bank] unknown --source {other:?}; matching nothing");
                false
            }
        })
        .collect();
    if cli.max > 0 && kept.len() > cli.max {
        kept.truncate(cli.max);
    }
    let total_dur: f64 = kept.iter().map(|r| r.duration_s).sum();
    eprintln!(
        "[build_human_bank] kept {} utterances / {:.1} min after filter",
        kept.len(),
        total_dur / 60.0
    );

    let mut bank = PhonemeBank::new();
    let mut stats = Stats::default();

    for (i, row) in kept.iter().enumerate() {
        let mfcc_path = cli.bank_dir.join(&row.mfcc_path);
        let mfcc_bytes = match std::fs::read(&mfcc_path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!(
                    "[warn] {}: read mfcc {} failed: {e}",
                    row.label,
                    mfcc_path.display()
                );
                stats.skipped_io += 1;
                continue;
            }
        };
        let mut audio = match read_binary(&mfcc_bytes) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[warn] {}: parse mfcc failed: {e}", row.label);
                stats.skipped_io += 1;
                continue;
            }
        };
        // CRITICAL: corpus_acquire persists RAW (un-CMVN'd) MFCC
        // to disk. The bootstrap synth bank lives in CMVN
        // space, and the recogniser CMVN-normalises every
        // query. We must normalise the loaded audio in-place
        // before alignment — otherwise the segments we extract
        // are in raw MFCC space and the resulting bank is in a
        // different feature space from the queries it'll be
        // matched against (the bug that drove first-pass human
        // bank PER 85 % vs synth 84 %).
        adam_audio::cmvn::normalise_in_place(&mut audio);
        if audio.num_frames() != row.mfcc_frames {
            eprintln!(
                "[warn] {}: mfcc frame mismatch (file {}, manifest {})",
                row.label,
                audio.num_frames(),
                row.mfcc_frames
            );
        }

        let phonemes = phonemes_from_transcript(&row.transcript);
        if phonemes.is_empty() {
            stats.skipped_empty_phonemes += 1;
            continue;
        }
        if phonemes.len() > audio.num_frames() {
            stats.skipped_too_short += 1;
            continue;
        }

        match align(&audio, &phonemes, &bootstrap) {
            Ok(alignment) => {
                push_segments(&mut bank, &audio, &alignment.segments, cli.per_phoneme_cap);
                stats.aligned += 1;
                stats.total_phonemes += alignment.segments.len();
            }
            Err(e) => {
                match e {
                    AlignError::UncoveredPhoneme(_) => stats.skipped_uncovered += 1,
                    AlignError::PhonemesExceedFrames { .. } => stats.skipped_too_short += 1,
                    AlignError::EmptyAudio | AlignError::EmptyPhonemes => {
                        stats.skipped_empty_phonemes += 1
                    }
                }
                eprintln!("[warn] {}: align failed: {e}", row.label);
            }
        }

        if (i + 1) % 200 == 0 {
            eprintln!(
                "[build_human_bank] {}/{} utterances processed; bank: {} phonemes / {} templates",
                i + 1,
                kept.len(),
                bank.len(),
                bank.template_count()
            );
        }
    }

    eprintln!("[build_human_bank] === done ===");
    eprintln!("[build_human_bank] aligned          : {}", stats.aligned);
    eprintln!(
        "[build_human_bank] total phoneme segs : {}",
        stats.total_phonemes
    );
    eprintln!(
        "[build_human_bank] skipped uncovered  : {} (bootstrap-bank gap)",
        stats.skipped_uncovered
    );
    eprintln!(
        "[build_human_bank] skipped too short  : {} (n_phones > n_frames)",
        stats.skipped_too_short
    );
    eprintln!(
        "[build_human_bank] skipped empty phon : {}",
        stats.skipped_empty_phonemes
    );
    eprintln!(
        "[build_human_bank] skipped IO error   : {}",
        stats.skipped_io
    );
    eprintln!(
        "[build_human_bank] resulting bank     : {} phonemes / {} templates",
        bank.len(),
        bank.template_count()
    );

    if let Some(parent) = cli.out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    bank.save_to_file(&cli.out)?;
    eprintln!("[build_human_bank] wrote {}", cli.out.display());

    Ok(())
}

#[derive(Default)]
struct Stats {
    aligned: usize,
    total_phonemes: usize,
    skipped_uncovered: usize,
    skipped_too_short: usize,
    skipped_empty_phonemes: usize,
    skipped_io: usize,
}

/// Convert a transcript into a flat phoneme sequence.
///
/// Tokens are split on whitespace and punctuation. For each
/// token, `is_native_root=true` is set (the v6.3 strict-«і»
/// rule applies to native lexical material; for loanwords we'd
/// pass `false`, but the corpus is curated to native KZ so this
/// is the right default for the manifest sources we ingest).
fn phonemes_from_transcript(text: &str) -> Vec<adam_phoneme::Phoneme> {
    let mut out = Vec::new();
    for token in text.split(|c: char| !c.is_alphabetic()) {
        if token.is_empty() {
            continue;
        }
        let lower = token.to_lowercase();
        let phones = cyrillic_to_phonemes(&lower, true);
        out.extend(phones);
    }
    out
}

/// Walk forced-alignment segments and push each one as a new
/// `PhonemeTemplate` exemplar, capped per phoneme. Empty
/// segments (`end == start`) are dropped — the aligner
/// guarantees `end > start`, but we double-check.
fn push_segments(
    bank: &mut PhonemeBank,
    audio: &MfccSequence,
    segments: &[adam_forced_aligner::PhoneSegment],
    per_phoneme_cap: usize,
) {
    // Track current per-phoneme count locally so we don't have
    // to call `bank.all(p).len()` on every iteration (HashMap
    // hit per call).
    let mut counts: HashMap<adam_phoneme::Phoneme, usize> = HashMap::new();
    for p in segments.iter().map(|s| s.phoneme) {
        counts.entry(p).or_insert_with(|| bank.all(p).len());
    }
    for seg in segments {
        if seg.end <= seg.start {
            continue;
        }
        let cnt = counts.entry(seg.phoneme).or_insert(0);
        if *cnt >= per_phoneme_cap {
            continue;
        }
        let frames: Vec<Vec<f32>> = audio.frames[seg.start..seg.end].to_vec();
        let mfcc = MfccSequence {
            frames,
            sample_rate: audio.sample_rate,
            hop_length: audio.hop_length,
            n_mfcc: audio.n_mfcc,
        };
        bank.insert(PhonemeTemplate {
            phoneme: seg.phoneme,
            mfcc,
        });
        *cnt += 1;
    }
}

#[allow(dead_code)]
fn touch(_: &Path) {}
