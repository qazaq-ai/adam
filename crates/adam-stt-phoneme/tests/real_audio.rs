// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! **End-to-end on real Wikimedia Kazakh audio.**
//!
//! The first test in the project that runs the v6.3 recogniser
//! on actual recorded Kazakh speech (not synth). For each
//! audio file from `data/v6_3_phoneme_bank/audio/`:
//!
//! 1. Load the WAV.
//! 2. Recognise through the **hybrid bank** (real templates
//!    where Phase 2c built them, synthetic fallback for the
//!    remaining 17 phonemes).
//! 3. Inspect the output — currently exploratory, not asserted.
//!
//! These tests are **skipped silently** if
//! `data/v6_3_phoneme_bank/templates.bin` is absent (e.g.
//! when the test runs in a checkout without the data
//! committed). They are not regression-floor tests yet —
//! synthetic templates are too crude for the recogniser to
//! land specific phonemes on every word. They prove the
//! **pipeline runs end-to-end on real audio** and surface
//! per-word output for inspection / regression baseline once
//! Phase 2d-plus delivers higher-quality templates.

use adam_audio::wav::read_wav;
use adam_stt_phoneme::{PhonemeBank, WordConfig, recognise_word, rescore};
use std::path::PathBuf;

fn bank_dir() -> Option<PathBuf> {
    for candidate in ["data/v6_3_phoneme_bank", "../../data/v6_3_phoneme_bank"] {
        let p = PathBuf::from(candidate);
        if p.join("templates.bin").exists() {
            return Some(p);
        }
    }
    None
}

/// Build the hybrid bank: real templates win, synthetic fills
/// the gaps. Returns `None` if no real bank is on disk.
fn hybrid_bank(sample_rate: u32) -> Option<PhonemeBank> {
    let dir = bank_dir()?;
    let real = PhonemeBank::load_from_file(dir.join("templates.bin")).ok()?;
    let synth = PhonemeBank::synthetic(sample_rate);
    Some(real.merged_with_fallback(&synth))
}

/// Pipeline runs end-to-end on the «жібек» recording without
/// crashing and produces at least one phoneme. The rescored
/// output should preserve all phonemes (no rescoring needed).
#[test]
fn jibek_pipeline_runs_end_to_end() {
    let Some(dir) = bank_dir() else {
        eprintln!("skipping: templates.bin not found");
        return;
    };
    let Some(bank) = hybrid_bank(16_000) else {
        eprintln!("skipping: could not build hybrid bank");
        return;
    };

    let wav_path = dir.join("audio/jibek.wav");
    let pcm = read_wav(&wav_path).expect("read жібек.wav");
    assert_eq!(pcm.channels, 1, "expected mono");
    assert_eq!(pcm.sample_rate, 16_000, "expected 16 kHz");

    let raw = recognise_word(&pcm.data, pcm.sample_rate, &bank, &WordConfig::default());
    let rescored = rescore(&raw);
    eprintln!("[real-audio] «жібек» raw       → {raw:?}");
    eprintln!("[real-audio] «жібек» rescored  → {rescored:?}");
    assert!(
        !rescored.is_empty(),
        "expected at least one recognised phoneme from жібек.wav",
    );
}

/// «қазақ» — pipeline + rescoring. The raw DTW output may
/// include spurious consonants from the small-corpus bias;
/// rescoring should clean them up by enforcing the
/// `(C)V(CC(C))` shape per syllable.
#[test]
fn qazaq_pipeline_with_rescore() {
    let Some(dir) = bank_dir() else {
        eprintln!("skipping: templates.bin not found");
        return;
    };
    let Some(bank) = hybrid_bank(16_000) else {
        eprintln!("skipping: could not build hybrid bank");
        return;
    };

    let pcm = read_wav(dir.join("audio/kk_kazakh.wav")).expect("read kk_kazakh.wav");
    let raw = recognise_word(&pcm.data, pcm.sample_rate, &bank, &WordConfig::default());
    let rescored = rescore(&raw);
    eprintln!("[real-audio] «қазақ» raw       → {raw:?}");
    eprintln!("[real-audio] «қазақ» rescored  → {rescored:?}");
    assert!(!rescored.is_empty());
    // The rescored stream must obey the syllable shape — no
    // initial consonant cluster, max coda 3.
    let n_initial_cons = rescored.iter().take_while(|p| p.is_consonant()).count();
    assert!(
        n_initial_cons <= 1,
        "rescored stream has initial cluster of {n_initial_cons}: {rescored:?}"
    );
}

/// Hybrid bank covers every sounded phoneme of the inventory
/// (real where Phase 2c had data; synthetic fallback for the
/// rest).
#[test]
fn hybrid_bank_has_full_inventory_coverage() {
    use adam_phoneme::{Phoneme, PhonemeClass};
    let Some(bank) = hybrid_bank(16_000) else {
        eprintln!("skipping: templates.bin not found");
        return;
    };
    for &p in Phoneme::ALL {
        if matches!(p.class(), PhonemeClass::Boundary) {
            continue;
        }
        assert!(bank.get(p).is_some(), "hybrid bank missing phoneme {p:?}");
    }
}

/// The hybrid bank has **at least one** template that
/// differs from the corresponding synth-only template
/// (i.e. some real-data templates DID override). Pins that
/// real templates actually made it into the bank.
#[test]
fn hybrid_bank_contains_real_overrides() {
    let Some(bank) = hybrid_bank(16_000) else {
        eprintln!("skipping: templates.bin not found");
        return;
    };
    let synth = PhonemeBank::synthetic(16_000);

    let mut differing = 0_usize;
    for (phoneme, hybrid_t) in bank.iter() {
        if let Some(synth_t) = synth.get(*phoneme)
            && hybrid_t.mfcc != synth_t.mfcc
        {
            differing += 1;
        }
    }
    assert!(
        differing >= 10,
        "expected ≥10 real overrides in hybrid bank, got {differing}",
    );
    eprintln!("[real-audio] hybrid bank has {differing} real-data overrides over synthetic");
}
