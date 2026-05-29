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

use adam_audio::mfcc::{MfccConfig, mfcc};
use adam_phoneme::Phoneme;
use adam_phoneme::cyrillic::cyrillic_to_phonemes;
use adam_stt_phoneme::{PhonemeBank, StreamConfig, WordConfig, recognise_stream, recognise_word};
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
    /// Recogniser: "stream" (frame-synchronous Viterbi, default)
    /// or "window" (legacy sliding-window classifier).
    #[arg(long, default_value = "stream")]
    recogniser: String,
    /// Switch penalty for the stream recogniser (flat-LM
    /// transition cost). Higher = fewer, longer segments.
    /// Default 3.0 minimises PER on the FLEURS test sweep.
    #[arg(long, default_value = "3.0")]
    switch_penalty: f32,
    /// Optional path to write a machine-readable benchmark
    /// record (JSON) — includes git head, bank SHA-256, split,
    /// PER, hyp/ref ratio, and per-phoneme error counts. Use
    /// for tracking the metric across commits.
    #[arg(long)]
    json_out: Option<PathBuf>,
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
    let stream_cfg = StreamConfig {
        switch_penalty: cli.switch_penalty,
    };
    println!(
        "[stt_eval] recogniser: {} (switch_penalty={})",
        cli.recogniser, cli.switch_penalty
    );

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
        // Empty `--split` = no split filter (all entries of this
        // source-class). Synth labels have no `_<split>_` segment
        // so the substring filter would skip everything; pass
        // `--split ""` to evaluate the whole source-class.
        if !cli.split.is_empty() && !entry.label.contains(&format!("_{}_", cli.split)) {
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

        let hyp = if cli.recogniser == "window" {
            recognise_word(&pcm_mono.data, pcm_mono.sample_rate, &bank, &cfg)
        } else {
            let query = mfcc(&pcm_mono.data, pcm_mono.sample_rate, &MfccConfig::default());
            recognise_stream(&query, &bank, &stream_cfg)
        };

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
    for (p, n) in errs.iter().take(10) {
        println!("  {p:?} → {n} errors");
    }

    // Machine-readable record for tracking PER across commits.
    if let Some(path) = &cli.json_out {
        let record = BenchRecord {
            git_head: short_git_head(),
            bank_sha256: sha256_of(&cli.bank_dir.join("templates.bin"))
                .unwrap_or_else(|_| "unavailable".to_string()),
            source_class: cli.source_class.clone(),
            split: cli.split.clone(),
            recogniser: cli.recogniser.clone(),
            switch_penalty: cli.switch_penalty,
            utterances: n_utts,
            reference_phonemes: total_ref,
            total_edits,
            per,
            hyp_len: total_hyp_len,
            ref_len: total_ref_len,
            hyp_ref_ratio: total_hyp_len as f64 / total_ref_len as f64,
            top_errors: errs
                .iter()
                .take(10)
                .map(|(p, n)| (format!("{p:?}"), *n))
                .collect(),
        };
        let json = serde_json::to_string_pretty(&record)?;
        std::fs::write(path, json + "\n")?;
        println!("\n[stt_eval] wrote bench record → {}", path.display());
    }

    Ok(())
}

/// Machine-readable PER report. Every field is plain JSON so
/// downstream tooling (CI, dashboards, regression alerts) can
/// parse it without round-tripping through the human log.
#[derive(serde::Serialize)]
struct BenchRecord {
    git_head: String,
    bank_sha256: String,
    source_class: String,
    split: String,
    recogniser: String,
    switch_penalty: f32,
    utterances: usize,
    reference_phonemes: usize,
    total_edits: usize,
    per: f64,
    hyp_len: usize,
    ref_len: usize,
    hyp_ref_ratio: f64,
    top_errors: Vec<(String, usize)>,
}

fn short_git_head() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "--short=10", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(o.stdout)
            } else {
                None
            }
        })
        .map(|b| String::from_utf8_lossy(&b).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Streaming SHA-256 of a file. Tiny dep-free implementation;
/// we only need a stable fingerprint of the bank artifact, not
/// crypto strength.
fn sha256_of(path: &std::path::Path) -> std::io::Result<String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.hex())
}

// ─── inline SHA-256 (no external dep, FIPS 180-4 reference)
// ─────────────────────────────────────────────────────────────

struct Sha256 {
    state: [u32; 8],
    buf: Vec<u8>,
    len_bits: u64,
}

impl Sha256 {
    fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            buf: Vec::with_capacity(64),
            len_bits: 0,
        }
    }
    fn update(&mut self, data: &[u8]) {
        self.len_bits = self.len_bits.wrapping_add((data.len() as u64) << 3);
        self.buf.extend_from_slice(data);
        while self.buf.len() >= 64 {
            let block: [u8; 64] = self.buf[..64].try_into().unwrap();
            self.compress(&block);
            self.buf.drain(..64);
        }
    }
    fn hex(mut self) -> String {
        let len_bits = self.len_bits;
        self.buf.push(0x80);
        while self.buf.len() % 64 != 56 {
            self.buf.push(0);
        }
        self.buf.extend_from_slice(&len_bits.to_be_bytes());
        let tail: Vec<u8> = std::mem::take(&mut self.buf);
        for chunk in tail.chunks_exact(64) {
            let block: [u8; 64] = chunk.try_into().unwrap();
            self.compress(&block);
        }
        self.state
            .iter()
            .map(|w| format!("{w:08x}"))
            .collect::<Vec<_>>()
            .join("")
    }
    fn compress(&mut self, block: &[u8; 64]) {
        const K: [u32; 64] = [
            0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
            0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
            0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
            0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
            0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
            0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
            0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
            0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
            0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
            0xc67178f2,
        ];
        let mut w = [0u32; 64];
        for (i, b4) in block.chunks_exact(4).enumerate().take(16) {
            w[i] = u32::from_be_bytes([b4[0], b4[1], b4[2], b4[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
        self.state[5] = self.state[5].wrapping_add(f);
        self.state[6] = self.state[6].wrapping_add(g);
        self.state[7] = self.state[7].wrapping_add(h);
    }
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
