// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `build_pcm_bank` — Phase 7 step 2 (2026-05-29).
//!
//! Replaces the formant-synth-derived `pcm_templates.bin`
//! (which sounded like «robot from the 80s» per user
//! diagnosis) with **real human-speech PCM segments** harvested
//! from forced-aligned audio. Source preferences in order:
//!
//! 1. **kaz-tili.kz** — short studio-quality letter / morpheme
//!    drills, deliberately well-articulated. Best PCM-template
//!    source: each clip is short, the speaker is clear, and
//!    the transcript is exactly the phoneme(s) being taught.
//! 2. **Wikimedia** — public-domain Kazakh recordings (UDHR,
//!    sample sentences).
//! 3. **FLEURS train/dev** — large natural-speech pool, useful
//!    for phonemes that kaz-tili+Wikimedia don't cover.
//!
//! For each utterance:
//!
//! 1. Load WAV → 16 kHz mono PCM.
//! 2. mfcc + CMVN → MfccSequence for alignment.
//! 3. cyrillic_to_phonemes(transcript, true) → phoneme sequence.
//! 4. forced-align → PhoneSegments (frame indices).
//! 5. For each segment: convert frame range to sample range via
//!    `hop_length` and the audio's sample rate, extract the
//!    PCM slice, and record it as a candidate for that
//!    phoneme.
//!
//! After all utterances, for each phoneme pick the **best**
//! candidate by alignment cost (per-frame mean Euclidean
//! distance to the bootstrap centroid). Single PcmTemplate per
//! phoneme — the existing PcmBank format is single-template,
//! and TTS quality benefits more from a clean centred clip
//! than from many noisy ones.

use adam_audio::cmvn::normalise_in_place;
use adam_audio::mfcc::{MfccConfig, mfcc};
use adam_audio::wav::read_wav;
use adam_forced_aligner::align;
use adam_phoneme::cyrillic::cyrillic_to_phonemes_prayer_aware;
use adam_stt_phoneme::PhonemeBank;
use adam_tts_phoneme::{PcmBank, PcmTemplate};
use clap::Parser;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "build_pcm_bank",
    about = "Forced-align human Kazakh audio → per-phoneme PCM template bank."
)]
struct Cli {
    /// Bank directory holding MANIFEST.jsonl + audio/.
    #[arg(long, default_value = "data/v6_3_phoneme_bank")]
    bank_dir: PathBuf,

    /// Bootstrap MFCC bank for forced alignment.
    #[arg(long, default_value = "data/v6_3_phoneme_bank/templates.bin")]
    bootstrap_bank: PathBuf,

    /// Output PCM bank path.
    #[arg(long, default_value = "data/v6_3_phoneme_bank/pcm_templates_human.bin")]
    out: PathBuf,

    /// Source filter: `prefer-clean` walks kaz-tili first, then
    /// wikimedia, then FLEURS train/dev — stops collecting for a
    /// phoneme once any clip is found from the cleaner source.
    /// `all-human` mixes everything. `kaz-tili-only` for the
    /// strictest letter-drill-only output.
    #[arg(long, default_value = "prefer-clean")]
    source: String,

    /// Cap on candidate clips per phoneme during collection — caps
    /// memory + time. Final bank keeps **one** per phoneme.
    #[arg(long, default_value_t = 50)]
    candidates_per_phoneme: usize,

    /// Minimum phoneme duration in milliseconds — clips shorter
    /// than this are rejected (they're almost always alignment
    /// artifacts: the aligner squashed a phoneme into 1 frame at
    /// an utterance edge). Default 50 ms ≈ 800 samples at 16 kHz.
    #[arg(long, default_value_t = 50)]
    min_duration_ms: u32,

    /// Maximum phoneme duration in milliseconds — clips longer
    /// than this are rejected (the aligner gave one phoneme a
    /// whole word's worth of frames, likely because adjacent
    /// segments collapsed to length 1). Default 300 ms which
    /// comfortably contains any realistic single phoneme.
    #[arg(long, default_value_t = 300)]
    max_duration_ms: u32,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct ManifestRow {
    label: String,
    transcript: String,
    source_class: String,
    wav_path: String,
    duration_s: f64,
}

#[allow(dead_code)]
struct Candidate {
    samples: Vec<f32>,
    sample_rate: u32,
    cost: f32,
    source_class: String,
    label: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    eprintln!(
        "[build_pcm_bank] bank_dir={} bootstrap={} out={} source={} cap/phoneme={}",
        cli.bank_dir.display(),
        cli.bootstrap_bank.display(),
        cli.out.display(),
        cli.source,
        cli.candidates_per_phoneme,
    );

    let bootstrap = PhonemeBank::load_from_file(&cli.bootstrap_bank)?;
    eprintln!(
        "[build_pcm_bank] bootstrap covers {} phonemes / {} templates",
        bootstrap.len(),
        bootstrap.template_count()
    );

    // Bootstrap centroid for cost scoring (mean of all
    // exemplar means).
    let mut centroids: HashMap<adam_phoneme::Phoneme, Vec<f32>> = HashMap::new();
    for (p, tmpls) in bootstrap.iter_all() {
        if tmpls.is_empty() {
            continue;
        }
        let dim = tmpls[0].mfcc.dim();
        let mut acc = vec![0.0_f32; dim];
        let mut n = 0_usize;
        for tmpl in tmpls {
            for frame in &tmpl.mfcc.frames {
                for (a, x) in acc.iter_mut().zip(frame.iter()) {
                    *a += *x;
                }
                n += 1;
            }
        }
        if n > 0 {
            let inv = 1.0 / n as f32;
            for a in &mut acc {
                *a *= inv;
            }
            centroids.insert(*p, acc);
        }
    }

    let manifest_path = cli.bank_dir.join("MANIFEST.jsonl");
    let manifest = std::fs::read_to_string(&manifest_path)?;
    let mut rows: Vec<ManifestRow> = Vec::new();
    for line in manifest.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(r) = serde_json::from_str::<ManifestRow>(line) {
            rows.push(r);
        }
    }
    eprintln!("[build_pcm_bank] manifest rows: {}", rows.len());

    // Source priority order — used by prefer-clean mode.
    let priority = |sc: &str| -> u8 {
        match sc {
            "kaz_tili" => 0,
            "wikimedia" => 1,
            "fleurs" => 2,
            _ => 9,
        }
    };
    let in_scope = |r: &ManifestRow| -> bool {
        match cli.source.as_str() {
            "kaz-tili-only" => r.source_class == "kaz_tili",
            "prefer-clean" | "all-human" => {
                r.source_class == "kaz_tili"
                    || r.source_class == "wikimedia"
                    || (r.source_class == "fleurs" && !r.label.starts_with("fleurs_test_"))
                    || r.source_class == "common_voice"
            }
            other => {
                eprintln!("[build_pcm_bank] unknown --source {other:?}");
                false
            }
        }
    };

    // Sort by source priority so prefer-clean walks kaz-tili
    // utterances first.
    rows.retain(in_scope);
    rows.sort_by_key(|r| priority(&r.source_class));
    eprintln!(
        "[build_pcm_bank] kept {} utterances in scope (kaz-tili: {}, wiki: {}, fleurs: {})",
        rows.len(),
        rows.iter().filter(|r| r.source_class == "kaz_tili").count(),
        rows.iter()
            .filter(|r| r.source_class == "wikimedia")
            .count(),
        rows.iter().filter(|r| r.source_class == "fleurs").count(),
    );

    let cap = cli.candidates_per_phoneme;
    let mut by_phoneme: HashMap<adam_phoneme::Phoneme, Vec<Candidate>> = HashMap::new();
    let mut stats = Stats::default();

    for (i, row) in rows.iter().enumerate() {
        let wav_path = cli.bank_dir.join(&row.wav_path);
        let pcm = match read_wav(&wav_path) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[warn] {}: read_wav: {e}", row.label);
                stats.skipped_io += 1;
                continue;
            }
        };
        let mono = if pcm.channels > 1 { pcm.to_mono() } else { pcm };
        let mfcc_cfg = MfccConfig::default();
        let mut mfcc_seq = mfcc(&mono.data, mono.sample_rate, &mfcc_cfg);
        normalise_in_place(&mut mfcc_seq);
        let hop_length = mfcc_seq.hop_length;

        let phonemes = phonemes_from_transcript(&row.transcript);
        if phonemes.is_empty() {
            stats.skipped_empty += 1;
            continue;
        }
        if phonemes.len() > mfcc_seq.num_frames() {
            stats.skipped_short += 1;
            continue;
        }

        let alignment = match align(&mfcc_seq, &phonemes, &bootstrap) {
            Ok(a) => a,
            Err(e) => {
                eprintln!("[warn] {}: align failed: {e}", row.label);
                stats.skipped_align += 1;
                continue;
            }
        };

        // Should we even continue collecting? If we're in
        // prefer-clean and every phoneme in `phonemes` already
        // has `cap` candidates from an equal-or-higher-priority
        // source, skip this utterance for efficiency.
        for seg in &alignment.segments {
            // Skip empty / sentinel segments.
            if seg.end <= seg.start {
                continue;
            }
            let entry = by_phoneme.entry(seg.phoneme).or_default();
            if entry.len() >= cap {
                continue;
            }
            // Compute alignment cost vs bootstrap centroid.
            let cost = match centroids.get(&seg.phoneme) {
                Some(c) => mean_frame_distance(&mfcc_seq.frames[seg.start..seg.end], c),
                None => f32::INFINITY,
            };
            // Convert frame [start..end) → sample [start*hop ..
            // end*hop) and clamp to mono.data length.
            let sample_start = seg.start * hop_length;
            let mut sample_end = seg.end * hop_length;
            if sample_end > mono.data.len() {
                sample_end = mono.data.len();
            }
            if sample_end <= sample_start {
                continue;
            }
            let samples = mono.data[sample_start..sample_end].to_vec();
            entry.push(Candidate {
                samples,
                sample_rate: mono.sample_rate,
                cost,
                source_class: row.source_class.clone(),
                label: row.label.clone(),
            });
            stats.collected += 1;
        }

        if (i + 1) % 500 == 0 {
            let covered = by_phoneme.len();
            let total_cands: usize = by_phoneme.values().map(Vec::len).sum();
            eprintln!(
                "[build_pcm_bank] {}/{} utts processed; covered {} phonemes, {} candidates",
                i + 1,
                rows.len(),
                covered,
                total_cands
            );
        }
    }

    eprintln!("[build_pcm_bank] === collection done ===");
    eprintln!("[build_pcm_bank] candidates : {}", stats.collected);
    eprintln!("[build_pcm_bank] skipped IO   : {}", stats.skipped_io);
    eprintln!("[build_pcm_bank] skipped short: {}", stats.skipped_short);
    eprintln!("[build_pcm_bank] skipped empty: {}", stats.skipped_empty);
    eprintln!("[build_pcm_bank] skipped align: {}", stats.skipped_align);
    eprintln!("[build_pcm_bank] phonemes seen: {}", by_phoneme.len());

    // Pick the lowest-cost candidate per phoneme, **subject to
    // the duration window**. Sub-50 ms clips are alignment edge
    // artifacts (the aligner squashed a phoneme into 1 frame),
    // > 300 ms clips swallowed neighbouring phonemes — both are
    // unfit as TTS spectral basis. If no candidate is in window
    // we still emit the closest-to-window one as a fallback,
    // logged as such.
    let mut bank = PcmBank::new();
    let mut chosen_log: Vec<(adam_phoneme::Phoneme, f32, String, usize, &'static str)> = Vec::new();
    let mut sorted_keys: Vec<adam_phoneme::Phoneme> = by_phoneme.keys().copied().collect();
    sorted_keys.sort_by_key(|p| format!("{p:?}"));
    for p in sorted_keys {
        let mut cands = by_phoneme.remove(&p).unwrap();
        cands.sort_by(|a, b| {
            a.cost
                .partial_cmp(&b.cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let min_samples = (cli.min_duration_ms as usize * 16) // 16 samples / ms @ 16 kHz
            .max(64);
        let max_samples = cli.max_duration_ms as usize * 16;
        let mut tier = "in-window";
        let pick = cands
            .iter()
            .position(|c| c.samples.len() >= min_samples && c.samples.len() <= max_samples);
        let chosen = if let Some(idx) = pick {
            Some(cands.swap_remove(idx))
        } else {
            // Fallback: pick the candidate closest to the window.
            tier = "fallback-out-of-window";
            cands.into_iter().min_by_key(|c| {
                let n = c.samples.len();
                if n < min_samples {
                    min_samples.saturating_sub(n)
                } else {
                    n.saturating_sub(max_samples)
                }
            })
        };
        if let Some(best) = chosen {
            chosen_log.push((
                p,
                best.cost,
                best.source_class.clone(),
                best.samples.len(),
                tier,
            ));
            bank.insert(PcmTemplate {
                phoneme: p,
                sample_rate: best.sample_rate,
                samples: best.samples,
            });
        }
    }

    eprintln!("[build_pcm_bank] picked {} templates:", bank.len());
    for (p, cost, src, n_samples, tier) in &chosen_log {
        eprintln!(
            "  {:?}  cost={:.2}  source={:10} samples={}  dur={}ms  [{}]",
            p,
            cost,
            src,
            n_samples,
            n_samples * 1000 / 16_000,
            tier,
        );
    }

    if let Some(parent) = cli.out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    bank.save_to_file(&cli.out)?;
    eprintln!("[build_pcm_bank] wrote {}", cli.out.display());
    Ok(())
}

#[derive(Default)]
struct Stats {
    collected: usize,
    skipped_io: usize,
    skipped_short: usize,
    skipped_empty: usize,
    skipped_align: usize,
}

fn mean_frame_distance(frames: &[Vec<f32>], centroid: &[f32]) -> f32 {
    let mut sum = 0.0_f32;
    let mut n = 0_usize;
    for frame in frames {
        if frame.len() != centroid.len() {
            continue;
        }
        let mut acc = 0.0_f32;
        for (x, y) in frame.iter().zip(centroid.iter()) {
            let d = x - y;
            acc += d * d;
        }
        sum += acc.sqrt();
        n += 1;
    }
    if n > 0 { sum / n as f32 } else { f32::INFINITY }
}

fn phonemes_from_transcript(text: &str) -> Vec<adam_phoneme::Phoneme> {
    // Phase 11: prayer-aware so Arabic citations inside
    // literary transcripts (Abai, Шакарим, religious texts)
    // keep their «і»/«ы». Secular transcripts (FLEURS, KSC,
    // kaz-tili drills) trip the early-return path and stay
    // bit-identical to the prior implementation.
    cyrillic_to_phonemes_prayer_aware(text, /* is_native_root */ true)
}
