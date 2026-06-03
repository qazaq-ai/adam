// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `build_morpheme_bank` — Phase 12 step 2 (2026-05-31).
//!
//! User architecture directive (2026-05-31):
//!
//! > «Сначала фундамент алфавита, потом морфемы, потом слова,
//! >  потом предложения.»
//!
//! The alphabet layer (`pcm_templates.bin`) was shipped in
//! Phase 7 step 2 — kaz-tili clean letter drills as per-phoneme
//! PCM clips. This tool builds the **next layer up**: a
//! per-**morpheme** PCM bank, where each entry is a token like
//! «ша», «тайын», «дейін», «бала» recorded as a single
//! continuous articulation.
//!
//! ## How
//!
//! For each kaz-tili manifest entry:
//!
//! 1. **Tokenise** the transcript into morphemes. Drills use
//!    `/` (alternation), `,` (list), whitespace (sequence),
//!    `?` `.` `!` `;` `:` `(` `)` `-` `—` `–` (punctuation).
//!    A morpheme is any contiguous run of Cyrillic letters
//!    between separators.
//! 2. **Phonemize** each token via
//!    `cyrillic_to_phonemes_prayer_aware(text, true)`. Concat
//!    the per-token sequences into the full phoneme stream the
//!    forced aligner will use.
//! 3. **Forced-align** the audio (CMVN-normalised MFCC) against
//!    the full phoneme stream using the active phoneme bank.
//!    Records frame ranges for each phoneme.
//! 4. **Group** the frame ranges by source token: token *i* owns
//!    phonemes `[token_start_i, token_end_i)`. Convert that
//!    frame range to a sample range
//!    (`frame_idx × hop_length`) and extract the PCM slice.
//! 5. **Quality gate**: 50ms ≤ duration ≤ 700ms; non-empty
//!    sample slice. Keeps the bank to plausible morpheme
//!    durations and drops alignment edge artifacts.
//! 6. **Cost rank**: per token, keep the lowest-cost extraction
//!    across utterances (cost = mean Euclidean per-frame
//!    distance to the bootstrap centroid). Avoids over-writing
//!    a clean exemplar with a noisier one when the same morpheme
//!    appears in multiple drills.
//!
//! Output: `data/v6_3_phoneme_bank/morpheme_templates.bin`
//! (binary format version 1, see
//! `adam_tts_phoneme::morpheme_bank`).

use adam_audio::cmvn::normalise_in_place;
use adam_audio::mfcc::{MfccConfig, mfcc};
use adam_audio::wav::read_wav;
use adam_forced_aligner::align;
use adam_phoneme::cyrillic::cyrillic_to_phonemes_prayer_aware;
use adam_stt_phoneme::PhonemeBank;
use adam_tts_phoneme::{MorphemeBank, MorphemeTemplate};
use clap::Parser;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "build_morpheme_bank",
    about = "Forced-align kaz-tili drills → per-morpheme PCM bank."
)]
struct Cli {
    /// Bank directory (holds `MANIFEST.jsonl` and `audio/`).
    #[arg(long, default_value = "data/v6_3_phoneme_bank")]
    bank_dir: PathBuf,

    /// Bootstrap MFCC bank used for forced alignment.
    #[arg(long, default_value = "data/v6_3_phoneme_bank/templates.bin")]
    bootstrap_bank: PathBuf,

    /// Output morpheme bank path.
    #[arg(long, default_value = "data/v6_3_phoneme_bank/morpheme_templates.bin")]
    out: PathBuf,

    /// Minimum morpheme duration in milliseconds. Below this we
    /// reject (alignment edge artifact, not a real morpheme).
    #[arg(long, default_value_t = 50)]
    min_duration_ms: u32,

    /// Maximum morpheme duration in milliseconds. Above this we
    /// reject (likely two morphemes glued, or trailing silence).
    #[arg(long, default_value_t = 700)]
    max_duration_ms: u32,

    /// Cap on utterances consumed. `0` = no cap.
    #[arg(long, default_value_t = 0)]
    max: usize,
}

#[derive(Deserialize, Debug)]
struct ManifestRow {
    label: String,
    transcript: String,
    source_class: String,
    wav_path: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct Candidate {
    samples: Vec<f32>,
    sample_rate: u32,
    cost: f32,
    duration_ms: u32,
    label: String,
}

const MORPHEME_SEPARATORS: &[char] = &[
    '/', ',', ' ', '\t', '?', '.', '!', ';', ':', '(', ')', '[', ']', '{', '}', '-', '—', '–', '«',
    '»', '"', '\'', '\n', '\r',
];

fn tokenize(transcript: &str) -> Vec<String> {
    transcript
        .split(|c: char| MORPHEME_SEPARATORS.contains(&c))
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty() && s.chars().any(|c| c.is_alphabetic()))
        .collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    eprintln!(
        "[build_morpheme_bank] bank={} bootstrap={} out={} dur=[{}..{}ms]",
        cli.bank_dir.display(),
        cli.bootstrap_bank.display(),
        cli.out.display(),
        cli.min_duration_ms,
        cli.max_duration_ms,
    );
    let bootstrap = PhonemeBank::load_from_file(&cli.bootstrap_bank)?;
    eprintln!(
        "[build_morpheme_bank] bootstrap: {} phonemes / {} templates",
        bootstrap.len(),
        bootstrap.template_count()
    );

    // Bootstrap centroid for cost scoring.
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

    // Load manifest, keep kaz_tili only.
    let manifest = std::fs::read_to_string(cli.bank_dir.join("MANIFEST.jsonl"))?;
    let mut rows: Vec<ManifestRow> = Vec::new();
    for line in manifest.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(r) = serde_json::from_str::<ManifestRow>(line) {
            if r.source_class == "kaz_tili" {
                rows.push(r);
            }
        }
    }
    if cli.max > 0 && rows.len() > cli.max {
        rows.truncate(cli.max);
    }
    eprintln!("[build_morpheme_bank] kaz-tili utterances: {}", rows.len());

    let mut by_morpheme: HashMap<String, Candidate> = HashMap::new();
    let mut stats = Stats::default();

    for row in &rows {
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
        let cfg = MfccConfig::default();
        let mut mfcc_seq = mfcc(&mono.data, mono.sample_rate, &cfg);
        normalise_in_place(&mut mfcc_seq);
        let hop_length = mfcc_seq.hop_length;
        let sample_rate = mono.sample_rate;
        let n_samples_total = mono.data.len();

        let tokens = tokenize(&row.transcript);
        if tokens.is_empty() {
            stats.skipped_empty += 1;
            continue;
        }

        // Build full phoneme sequence with per-token boundaries.
        let mut full_phonemes: Vec<adam_phoneme::Phoneme> = Vec::new();
        let mut token_ranges: Vec<(usize, usize)> = Vec::new();
        for tok in &tokens {
            let start = full_phonemes.len();
            let phs = cyrillic_to_phonemes_prayer_aware(tok, true);
            if phs.is_empty() {
                token_ranges.push((start, start));
                continue;
            }
            full_phonemes.extend(phs);
            token_ranges.push((start, full_phonemes.len()));
        }
        if full_phonemes.is_empty() {
            stats.skipped_empty += 1;
            continue;
        }
        if full_phonemes.len() > mfcc_seq.num_frames() {
            stats.skipped_short += 1;
            continue;
        }
        let alignment = match align(&mfcc_seq, &full_phonemes, &bootstrap) {
            Ok(a) => a,
            Err(e) => {
                eprintln!("[warn] {}: align failed: {e}", row.label);
                stats.skipped_align += 1;
                continue;
            }
        };

        for (tok, (p_start, p_end)) in tokens.iter().zip(token_ranges.iter()) {
            if p_end <= p_start {
                continue;
            }
            let segs = &alignment.segments[*p_start..*p_end];
            if segs.is_empty() {
                continue;
            }
            let frame_start = segs[0].start;
            let frame_end = segs[segs.len() - 1].end;
            if frame_end <= frame_start {
                continue;
            }
            let sample_start = frame_start * hop_length;
            let mut sample_end = frame_end * hop_length;
            if sample_end > n_samples_total {
                sample_end = n_samples_total;
            }
            if sample_start >= sample_end {
                continue;
            }
            let n = sample_end - sample_start;
            let dur_ms = (n as u64 * 1000 / sample_rate as u64) as u32;
            if dur_ms < cli.min_duration_ms {
                stats.dropped_short += 1;
                continue;
            }
            if dur_ms > cli.max_duration_ms {
                stats.dropped_long += 1;
                // Trim trailing silence to bring it back into
                // window when feasible — too aggressive a
                // tail-trim risks losing the morpheme's nucleus,
                // so just reject for now.
                continue;
            }
            // Per-token alignment cost = mean of per-phoneme
            // mean-distances across the segs.
            let mut cost_sum = 0.0_f32;
            let mut cost_n = 0_usize;
            for seg in segs {
                if let Some(c) = centroids.get(&seg.phoneme) {
                    for frame in &mfcc_seq.frames[seg.start..seg.end] {
                        if frame.len() != c.len() {
                            continue;
                        }
                        let mut acc = 0.0_f32;
                        for (x, y) in frame.iter().zip(c.iter()) {
                            let d = x - y;
                            acc += d * d;
                        }
                        cost_sum += acc.sqrt();
                        cost_n += 1;
                    }
                }
            }
            let cost = if cost_n > 0 {
                cost_sum / cost_n as f32
            } else {
                f32::INFINITY
            };

            let new_cand = Candidate {
                samples: mono.data[sample_start..sample_end].to_vec(),
                sample_rate,
                cost,
                duration_ms: dur_ms,
                label: row.label.clone(),
            };
            // Keep the lowest-cost candidate per morpheme.
            match by_morpheme.get(tok) {
                Some(existing) if existing.cost <= cost => {
                    stats.dropped_higher_cost += 1;
                }
                _ => {
                    by_morpheme.insert(tok.clone(), new_cand);
                    stats.admitted += 1;
                }
            }
            // Compensate: insertion above counts every winning
            // pass, but we want one count per unique morpheme.
            // Use the final hashmap length at print time.
            let _ = sample_start;
        }
    }

    let mut bank = MorphemeBank::new();
    let mut keys: Vec<String> = by_morpheme.keys().cloned().collect();
    keys.sort();
    for k in keys {
        let c = &by_morpheme[&k];
        bank.insert(MorphemeTemplate {
            cyrillic: k.into_boxed_str(),
            sample_rate: c.sample_rate,
            samples: c.samples.clone(),
        });
    }

    eprintln!("[build_morpheme_bank] === done ===");
    eprintln!(
        "[build_morpheme_bank] morphemes admitted: {} (unique tokens)",
        bank.len()
    );
    eprintln!(
        "[build_morpheme_bank] dropped short (<{}ms) : {}",
        cli.min_duration_ms, stats.dropped_short
    );
    eprintln!(
        "[build_morpheme_bank] dropped long  (>{}ms) : {}",
        cli.max_duration_ms, stats.dropped_long
    );
    eprintln!(
        "[build_morpheme_bank] dropped higher-cost dup: {}",
        stats.dropped_higher_cost
    );
    eprintln!(
        "[build_morpheme_bank] skipped IO/short/align/empty: {}/{}/{}/{}",
        stats.skipped_io, stats.skipped_short, stats.skipped_align, stats.skipped_empty
    );

    // Sample summary.
    eprintln!("[build_morpheme_bank] sample morphemes:");
    for (_shown, (k, t)) in bank.iter().enumerate().take(25) {
        eprintln!(
            "  {:30} samples={:>6} ({:>4}ms)",
            k,
            t.samples.len(),
            t.samples.len() * 1000 / t.sample_rate as usize
        );
    }

    if let Some(parent) = cli.out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    bank.save_to_file(&cli.out)?;
    eprintln!("[build_morpheme_bank] wrote {}", cli.out.display());
    Ok(())
}

#[derive(Default, Debug)]
struct Stats {
    admitted: usize,
    dropped_short: usize,
    dropped_long: usize,
    dropped_higher_cost: usize,
    skipped_io: usize,
    skipped_short: usize,
    skipped_align: usize,
    skipped_empty: usize,
}
