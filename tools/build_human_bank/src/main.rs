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

    /// Drop the top-q fraction of segments per phoneme, ranked
    /// by per-segment alignment cost (mean Euclidean distance
    /// of segment frames to the bootstrap-bank centroid for
    /// that phoneme). `0.0` = keep everything (rounds 1..4
    /// behaviour). Typical Phase 6 step 2 round 5 setting is
    /// `0.10` — drop the worst-aligned 10 % which are most
    /// likely mis-labelled exemplars.
    #[arg(long, default_value_t = 0.0)]
    cost_drop_quantile: f32,

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
                    || r.source_class == "kaz_tili"
            }
            "human-all" => matches!(
                r.source_class.as_str(),
                "wikimedia" | "fleurs" | "common_voice" | "kaz_tili"
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

    // Cache bootstrap-bank centroids so per-segment cost
    // scoring is one HashMap hit per phoneme, not per frame.
    let mut bootstrap_centroids: HashMap<adam_phoneme::Phoneme, Vec<f32>> = HashMap::new();
    for (p, tmpls) in bootstrap.iter_all() {
        if tmpls.is_empty() {
            continue;
        }
        // Average centroid across all exemplars — same shape the
        // forced aligner uses.
        let dim = tmpls[0].mfcc.dim();
        let mut acc = vec![0.0_f32; dim];
        let mut n_frames_total = 0_usize;
        for tmpl in tmpls {
            for frame in &tmpl.mfcc.frames {
                for (a, x) in acc.iter_mut().zip(frame.iter()) {
                    *a += *x;
                }
                n_frames_total += 1;
            }
        }
        if n_frames_total > 0 {
            let inv = 1.0 / n_frames_total as f32;
            for a in &mut acc {
                *a *= inv;
            }
            bootstrap_centroids.insert(*p, acc);
        }
    }

    let mut candidates: Vec<Candidate> = Vec::new();
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
                collect_segments(
                    &mut candidates,
                    &audio,
                    &alignment.segments,
                    &bootstrap_centroids,
                );
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

        if (i + 1) % 500 == 0 {
            eprintln!(
                "[build_human_bank] {}/{} utterances processed; candidates: {}",
                i + 1,
                kept.len(),
                candidates.len()
            );
        }
    }

    // Cost-quantile filter + per-phoneme cap: sort each
    // phoneme's candidates by cost ascending, drop the worst
    // `cost_drop_quantile` fraction, then keep at most
    // `per_phoneme_cap` of the survivors. Order-of-insertion
    // bias from rounds 1..4 (first-K-utterances wins) is gone —
    // every segment gets a fair comparison against every other
    // segment of the same phoneme.
    let mut by_phoneme: HashMap<adam_phoneme::Phoneme, Vec<Candidate>> = HashMap::new();
    for c in candidates.drain(..) {
        by_phoneme.entry(c.phoneme).or_default().push(c);
    }
    let q = cli.cost_drop_quantile.clamp(0.0, 0.99);
    let mut bank = PhonemeBank::new();
    let mut dropped_quantile = 0_usize;
    let mut dropped_cap = 0_usize;
    let mut admitted = 0_usize;
    let mut keys: Vec<adam_phoneme::Phoneme> = by_phoneme.keys().copied().collect();
    keys.sort_by_key(|p| format!("{p:?}"));
    for p in keys {
        let mut group = by_phoneme.remove(&p).unwrap();
        group.sort_by(|a, b| {
            a.cost
                .partial_cmp(&b.cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let drop_n = (group.len() as f32 * q).floor() as usize;
        let kept_after_quantile = group.len().saturating_sub(drop_n);
        dropped_quantile += drop_n;
        let take = kept_after_quantile.min(cli.per_phoneme_cap);
        let drop_cap = kept_after_quantile.saturating_sub(take);
        dropped_cap += drop_cap;
        for c in group.into_iter().take(take) {
            bank.insert(PhonemeTemplate {
                phoneme: c.phoneme,
                mfcc: c.mfcc,
            });
            admitted += 1;
        }
    }
    eprintln!(
        "[build_human_bank] quantile filter   : dropped {} segments (top {:.0}% cost)",
        dropped_quantile,
        q * 100.0
    );
    eprintln!(
        "[build_human_bank] per-phoneme cap   : dropped {} extra segments (cap {})",
        dropped_cap, cli.per_phoneme_cap
    );
    eprintln!("[build_human_bank] admitted segments  : {}", admitted);

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
    let _ = (admitted, dropped_quantile, dropped_cap); // already logged above

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

/// One forced-aligned phoneme segment with the audio MFCC slice
/// and the segment's alignment cost (mean Euclidean distance
/// per frame to the bootstrap centroid for that phoneme).
struct Candidate {
    phoneme: adam_phoneme::Phoneme,
    mfcc: MfccSequence,
    cost: f32,
}

/// Walk a forced-alignment result and push each segment into
/// the candidate pool with its per-frame mean cost. Cost is
/// computed against the bootstrap-bank centroid for the
/// segment's phoneme — phonemes absent from the bootstrap
/// (shouldn't happen, but defensive) get `f32::INFINITY` so the
/// quantile filter drops them first.
fn collect_segments(
    candidates: &mut Vec<Candidate>,
    audio: &MfccSequence,
    segments: &[adam_forced_aligner::PhoneSegment],
    bootstrap_centroids: &HashMap<adam_phoneme::Phoneme, Vec<f32>>,
) {
    for seg in segments {
        if seg.end <= seg.start {
            continue;
        }
        let centroid = match bootstrap_centroids.get(&seg.phoneme) {
            Some(c) => c,
            None => continue,
        };
        let mut sum_d = 0.0_f32;
        let mut n = 0_usize;
        for frame in &audio.frames[seg.start..seg.end] {
            if frame.len() != centroid.len() {
                continue;
            }
            let mut acc = 0.0_f32;
            for (x, y) in frame.iter().zip(centroid.iter()) {
                let d = x - y;
                acc += d * d;
            }
            sum_d += acc.sqrt();
            n += 1;
        }
        let cost = if n > 0 {
            sum_d / n as f32
        } else {
            f32::INFINITY
        };
        let frames: Vec<Vec<f32>> = audio.frames[seg.start..seg.end].to_vec();
        let mfcc = MfccSequence {
            frames,
            sample_rate: audio.sample_rate,
            hop_length: audio.hop_length,
            n_mfcc: audio.n_mfcc,
        };
        candidates.push(Candidate {
            phoneme: seg.phoneme,
            mfcc,
            cost,
        });
    }
}

#[allow(dead_code)]
fn touch(_: &Path) {}
