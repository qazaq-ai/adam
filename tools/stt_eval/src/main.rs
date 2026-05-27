// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `stt_eval` — Phoneme Error Rate (PER) over a held-out split.
//!
//! Replaces the two-word Wikimedia smoke tests with a real
//! held-out metric. For every manifest entry matching the
//! requested split it:
//!
//!   1. loads the 16 kHz WAV,
//!   2. recognises a phoneme stream via [`recognise_word`]
//!      (CMVN-normalised, Phase 11),
//!   3. derives the ground-truth phoneme sequence from the
//!      transcript via [`cyrillic_to_phonemes`],
//!   4. computes Levenshtein distance between the two phoneme
//!      sequences.
//!
//! PER = Σ edit_distance / Σ reference_length, the standard
//! speech-recognition phoneme error rate. We also report the
//! recognised/reference length ratio (a quick diagnostic for
//! gross under- or over-segmentation) and a per-phoneme
//! confusion summary for the worst offenders.

use std::collections::HashMap;
use std::path::PathBuf;

use adam_phoneme::Phoneme;
use adam_phoneme::cyrillic::cyrillic_to_phonemes;
use adam_stt_phoneme::{PhonemeBank, WordConfig, recognise_word};
use clap::Parser;
use serde::Deserialize;

#[derive(Debug, Parser)]
#[command(
    name = "stt_eval",
    about = "Phoneme Error Rate over a held-out manifest split."
)]
struct Cli {
    /// Bank directory (holds MANIFEST.jsonl + templates.bin +
    /// audio/).
    #[arg(long, default_value = "data/v6_3_phoneme_bank")]
    bank_dir: PathBuf,
    /// Which `source_class` to evaluate.
    #[arg(long, default_value = "fleurs")]
    source_class: String,
    /// FLEURS split substring to match in the label
    /// (`fleurs_<split>_<id>`). Use "test" for the held-out set.
    #[arg(long, default_value = "test")]
    split: String,
    /// Cap the number of utterances (0 = all). Useful for a
    /// quick probe before the full run.
    #[arg(long, default_value = "0")]
    max: usize,
    /// Use the synthetic-only bank (no real templates). Handy to
    /// sanity-check the harness itself.
    #[arg(long)]
    synthetic_only: bool,
}

#[derive(Debug, Deserialize)]
struct ManifestEntry {
    label: String,
    transcript: String,
    source_class: String,
    wav_path: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Build the recognition bank: real templates win, synthetic
    // fills the gaps (same hybrid policy the real-audio tests use).
    let bank = if cli.synthetic_only {
        PhonemeBank::synthetic(16_000)
    } else {
        let real = PhonemeBank::load_from_file(cli.bank_dir.join("templates.bin"))?;
        let synth = PhonemeBank::synthetic(16_000);
        real.merged_with_fallback(&synth)
    };
    println!("[stt_eval] bank: {} phoneme templates", bank.len());

    let manifest_path = cli.bank_dir.join("MANIFEST.jsonl");
    let manifest = std::fs::read_to_string(&manifest_path)?;

    let cfg = WordConfig::default();

    let mut total_ref = 0_usize;
    let mut total_edits = 0_usize;
    let mut total_ref_len = 0_usize;
    let mut total_hyp_len = 0_usize;
    let mut n_utts = 0_usize;
    // (reference phoneme) → count of times it was substituted/deleted.
    let mut error_by_ref: HashMap<Phoneme, usize> = HashMap::new();

    for line in manifest.lines() {
        let entry: ManifestEntry = match serde_json::from_str(line) {
            Ok(e) => e,
            Err(_) => continue,
        };
        if entry.source_class != cli.source_class {
            continue;
        }
        if !entry.label.contains(&format!("_{}_", cli.split)) {
            continue;
        }

        let reference = cyrillic_to_phonemes(&entry.transcript, true);
        if reference.is_empty() {
            continue;
        }

        let wav_path = cli.bank_dir.join(&entry.wav_path);
        let pcm = match adam_audio::wav::read_wav(&wav_path) {
            Ok(p) => p,
            Err(_) => continue, // gitignored / not regenerated locally
        };
        let pcm_mono = if pcm.channels > 1 { pcm.to_mono() } else { pcm };

        let hyp = recognise_word(&pcm_mono.data, pcm_mono.sample_rate, &bank, &cfg);

        let (dist, ops) = levenshtein_with_ops(&reference, &hyp);
        total_edits += dist;
        total_ref += reference.len();
        total_ref_len += reference.len();
        total_hyp_len += hyp.len();
        n_utts += 1;
        for r in ops {
            *error_by_ref.entry(r).or_insert(0) += 1;
        }

        if cli.max > 0 && n_utts >= cli.max {
            break;
        }
    }

    if n_utts == 0 {
        eprintln!(
            "[stt_eval] no utterances matched source_class={} split={} (are the FLEURS WAVs regenerated locally?)",
            cli.source_class, cli.split
        );
        return Ok(());
    }

    let per = total_edits as f64 / total_ref as f64;
    println!("\n[stt_eval] ===== Phoneme Error Rate =====");
    println!("[stt_eval] utterances evaluated : {n_utts}");
    println!("[stt_eval] reference phonemes    : {total_ref}");
    println!("[stt_eval] total edits           : {total_edits}");
    println!(
        "[stt_eval] PER                   : {:.4} ({:.1}%)",
        per,
        per * 100.0
    );
    println!(
        "[stt_eval] hyp/ref length ratio   : {:.3} (hyp {} / ref {})",
        total_hyp_len as f64 / total_ref_len as f64,
        total_hyp_len,
        total_ref_len,
    );

    // Worst-confused reference phonemes (substitution/deletion mass).
    let mut errs: Vec<(Phoneme, usize)> = error_by_ref.into_iter().collect();
    errs.sort_by(|a, b| b.1.cmp(&a.1));
    println!("\n[stt_eval] top-10 most-errored reference phonemes:");
    for (p, n) in errs.into_iter().take(10) {
        println!("  {p:?} → {n} errors");
    }

    Ok(())
}

/// Levenshtein distance between two phoneme sequences, plus the
/// list of reference phonemes that were deleted or substituted
/// (for the confusion summary). Insertions don't map to a
/// reference phoneme so they aren't recorded there.
fn levenshtein_with_ops(reference: &[Phoneme], hyp: &[Phoneme]) -> (usize, Vec<Phoneme>) {
    let n = reference.len();
    let m = hyp.len();
    // dp[i][j] = edit distance between reference[..i] and hyp[..j].
    let mut dp = vec![vec![0_usize; m + 1]; n + 1];
    for (i, row) in dp.iter_mut().enumerate() {
        row[0] = i;
    }
    for j in 0..=m {
        dp[0][j] = j;
    }
    for i in 1..=n {
        for j in 1..=m {
            let cost = if reference[i - 1] == hyp[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }

    // Back-trace to collect reference phonemes hit by a
    // substitution or deletion.
    let mut ops: Vec<Phoneme> = Vec::new();
    let (mut i, mut j) = (n, m);
    while i > 0 || j > 0 {
        if i > 0
            && j > 0
            && dp[i][j] == dp[i - 1][j - 1] + usize::from(reference[i - 1] != hyp[j - 1])
        {
            if reference[i - 1] != hyp[j - 1] {
                ops.push(reference[i - 1]); // substitution
            }
            i -= 1;
            j -= 1;
        } else if i > 0 && dp[i][j] == dp[i - 1][j] + 1 {
            ops.push(reference[i - 1]); // deletion
            i -= 1;
        } else {
            j -= 1; // insertion — no reference phoneme
        }
    }

    (dp[n][m], ops)
}

#[cfg(test)]
mod tests {
    use super::*;
    use adam_phoneme::Phoneme::*;

    #[test]
    fn identical_sequences_zero_distance() {
        let (d, ops) = levenshtein_with_ops(&[Q, A, Z, A, Q], &[Q, A, Z, A, Q]);
        assert_eq!(d, 0);
        assert!(ops.is_empty());
    }

    #[test]
    fn single_substitution() {
        // Z → S substitution in the middle.
        let (d, ops) = levenshtein_with_ops(&[Q, A, Z, A, Q], &[Q, A, S, A, Q]);
        assert_eq!(d, 1);
        assert_eq!(ops, vec![Z]);
    }

    #[test]
    fn deletion_counted() {
        let (d, ops) = levenshtein_with_ops(&[Q, A, Z], &[Q, Z]);
        assert_eq!(d, 1);
        assert_eq!(ops, vec![A]);
    }

    #[test]
    fn empty_hyp_is_full_reference_length() {
        let (d, _) = levenshtein_with_ops(&[Q, A, Z, A, Q], &[]);
        assert_eq!(d, 5);
    }

    /// PER over a tiny corpus computes as Σedits / Σref.
    #[test]
    fn per_aggregates_correctly() {
        // utt1: 1 edit / 5 ref; utt2: 2 edits / 4 ref.
        // PER = 3 / 9 = 0.333…
        let (d1, _) = levenshtein_with_ops(&[Q, A, Z, A, Q], &[Q, A, S, A, Q]);
        let (d2, _) = levenshtein_with_ops(&[B, A, L, A], &[B, E, L, E]);
        let total_edits = d1 + d2;
        let total_ref = 5 + 4;
        assert_eq!(d1, 1);
        assert_eq!(d2, 2);
        assert!(((total_edits as f64 / total_ref as f64) - 0.3333).abs() < 1e-3);
    }
}
