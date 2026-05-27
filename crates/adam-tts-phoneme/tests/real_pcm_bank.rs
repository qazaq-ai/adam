// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! **TTS with real PCM bank** — integration tests using the
//! corpus-derived `pcm_templates.bin`.
//!
//! Skipped silently if the bank file isn't present
//! (`data/v6_3_phoneme_bank/pcm_templates.bin`).

use adam_phoneme::cyrillic::cyrillic_to_phonemes;
use adam_tts_phoneme::{PcmBank, TtsConfig, synthesise, synthesise_with_bank};
use std::path::PathBuf;

fn bank_path() -> Option<PathBuf> {
    for candidate in [
        "data/v6_3_phoneme_bank/pcm_templates.bin",
        "../../data/v6_3_phoneme_bank/pcm_templates.bin",
    ] {
        let p = PathBuf::from(candidate);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

fn load_bank() -> Option<PcmBank> {
    PcmBank::load_from_file(bank_path()?).ok()
}

/// PCM bank loads cleanly and contains the expected ≥19
/// phonemes (single-word entries) + extra from multi-word.
#[test]
fn pcm_bank_loads_with_expected_coverage() {
    let Some(bank) = load_bank() else {
        eprintln!("skipping: pcm_templates.bin not found");
        return;
    };
    let n = bank.len();
    assert!(n >= 19, "expected ≥19 phonemes, got {n}");
    eprintln!("[real-pcm] bank covers {n} phonemes");
}

/// Every template in the bank has non-empty PCM and a
/// consistent sample rate.
#[test]
fn pcm_templates_well_formed() {
    let Some(bank) = load_bank() else {
        return;
    };
    let mut sample_rates = std::collections::HashSet::new();
    for (p, t) in bank.iter() {
        assert!(!t.samples.is_empty(), "{p:?} template empty");
        sample_rates.insert(t.sample_rate);
    }
    assert_eq!(
        sample_rates.len(),
        1,
        "expected one sample rate across the bank, got {sample_rates:?}"
    );
}

/// Synthesising «жібек» with the real PCM bank should produce
/// audio that **differs** from the synth-only synthesis (the
/// real templates replace at least one phoneme's signature).
#[test]
fn jibek_with_real_bank_differs_from_synth_only() {
    let Some(bank) = load_bank() else {
        return;
    };
    let phonemes = cyrillic_to_phonemes("жібек", true);
    let cfg = TtsConfig::default();
    let synth_only = synthesise(&phonemes, &cfg);
    let with_bank = synthesise_with_bank(&phonemes, Some(&bank), &cfg);
    assert_eq!(synth_only.data.len(), with_bank.data.len());
    let diff: f32 = synth_only
        .data
        .iter()
        .zip(with_bank.data.iter())
        .map(|(a, b)| (a - b).abs())
        .sum();
    assert!(
        diff > 1.0,
        "real-bank жібек should differ from synth-only, got diff {diff}"
    );
}

/// «қазақ» with the real PCM bank covers all 5 of its phonemes
/// from the bank (Q, A, Z, A, Q) — every one of these has real
/// data from the corpus.
#[test]
fn qazaq_phonemes_all_in_real_bank() {
    use adam_phoneme::Phoneme;
    let Some(bank) = load_bank() else {
        return;
    };
    for p in [Phoneme::Q, Phoneme::A, Phoneme::Z] {
        assert!(
            bank.get(p).is_some(),
            "expected real-bank coverage for {p:?}"
        );
    }
}

/// Output PcmSamples is mono and at the configured sample rate
/// regardless of whether bank or synth was used.
#[test]
fn output_consistency() {
    let Some(bank) = load_bank() else {
        return;
    };
    let phonemes = cyrillic_to_phonemes("қазақ", true);
    let cfg = TtsConfig::default();
    let pcm = synthesise_with_bank(&phonemes, Some(&bank), &cfg);
    assert_eq!(pcm.channels, 1);
    assert_eq!(pcm.sample_rate, cfg.sample_rate);
    assert!(!pcm.data.is_empty());
}
