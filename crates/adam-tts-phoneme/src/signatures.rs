// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Per-phoneme parametric PCM signatures.
//!
//! Each phoneme produces a short PCM segment whose acoustic
//! signature is enough to be **distinguishable** from other
//! phonemes when concatenated into a stream:
//!
//! - **Vowels** — harmonic stack (F0 + 4 overtones) at a
//!   phoneme-specific F0. Each vowel gets a different F0 so
//!   the resulting stream has spectral contrast.
//! - **Consonants** — narrow-band sine + small random noise
//!   at a centre frequency that roughly tracks place of
//!   articulation (bilabial ≈ 500 Hz, alveolar ≈ 4000 Hz,
//!   etc.).
//!
//! These signatures match the ones [`adam_stt_phoneme::bank`]
//! uses for the synthetic phoneme bank; the two systems agree
//! by construction so a synth-TTS output recognises through
//! synth-only bank cleanly.

use adam_phoneme::Phoneme;

/// Vowel: harmonic stack at the phoneme's F0 anchor.
pub(crate) fn synth_vowel(
    phoneme: Phoneme,
    duration_s: f32,
    sample_rate: u32,
    amplitude: f32,
) -> Vec<f32> {
    let f0 = vowel_f0(phoneme);
    harmonic_stack(f0, duration_s, sample_rate, amplitude, 4)
}

/// Consonant: noise-modulated sine at the phoneme's centre
/// frequency.
pub(crate) fn synth_consonant(
    phoneme: Phoneme,
    duration_s: f32,
    sample_rate: u32,
    amplitude: f32,
) -> Vec<f32> {
    let centre = consonant_centre_hz(phoneme);
    let n = (duration_s * sample_rate as f32) as usize;
    // Deterministic LCG-based pseudo-noise so the same phoneme
    // synthesises identically every run.
    let mut state: u64 = (phoneme as u8 as u64).wrapping_mul(0x9E3779B97F4A7C15);
    (0..n)
        .map(|i| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let noise = ((state >> 11) as f32) / ((1_u64 << 53) as f32) - 0.5;
            let t = i as f32 / sample_rate as f32;
            let carrier = (2.0 * std::f32::consts::PI * centre * t).sin();
            amplitude * (0.75 * carrier + 0.5 * noise)
        })
        .collect()
}

/// Vowel F0 anchor (Hz) — matches [`adam_stt_phoneme::bank`]'s
/// synthetic bank so synth TTS and synth STT bank agree.
fn vowel_f0(phoneme: Phoneme) -> f32 {
    use Phoneme::*;
    match phoneme {
        A => 100.0,
        Ae => 120.0,
        O => 140.0,
        Oe => 160.0,
        U => 180.0,
        Ue => 200.0,
        E => 220.0,
        I => 240.0,
        Y => 260.0,
        Yi => 280.0,
        _ => 150.0,
    }
}

/// Consonant spectral centre (Hz) — coarse proxy for place of
/// articulation, matching the synthetic phoneme bank.
fn consonant_centre_hz(phoneme: Phoneme) -> f32 {
    use Phoneme::*;
    match phoneme {
        P | B | M => 500.0,
        F | V => 700.0,
        T | D | S | Z | N | L | R | Ts => 4000.0,
        Sh | Zh | Ch => 3500.0,
        Shch => 3200.0,
        J => 2500.0,
        K | G | Ng | X => 2000.0,
        W => 800.0,
        Q | Gh => 1500.0,
        H => 1200.0,
        _ => 2000.0,
    }
}

/// Generate a harmonic stack: `F0 + (1/√h) * harmonic(h)` for
/// `h ∈ 1..=n_harmonics`, normalised so peak amplitude ≈
/// `amplitude`.
fn harmonic_stack(
    f0: f32,
    duration_s: f32,
    sample_rate: u32,
    amplitude: f32,
    n_harmonics: u32,
) -> Vec<f32> {
    let n = (duration_s * sample_rate as f32) as usize;
    (0..n)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            let mut s = 0.0_f32;
            for h in 1..=n_harmonics {
                let amp = amplitude / (h as f32).sqrt();
                s += amp * (2.0 * std::f32::consts::PI * f0 * h as f32 * t).sin();
            }
            s / (n_harmonics as f32).sqrt()
        })
        .collect()
}
