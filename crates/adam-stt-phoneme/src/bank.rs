// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Per-phoneme MFCC template bank.
//!
//! Each [`Phoneme`] in the v6.3 inventory is associated with
//! one reference MFCC sequence — a short MFCC time-series
//! that the DTW recogniser compares incoming query segments
//! against.
//!
//! ## Source of templates
//!
//! - **Production (Phase 2d):** averaged MFCCs extracted from
//!   forced-aligned corpus audio. Each template is the
//!   centroid of ~100 instances of the phoneme across
//!   speakers.
//! - **First-pass (this commit):** synthetic templates
//!   generated from `harmonic_voice` with formant-like
//!   spectral envelopes. Enough to validate the DTW
//!   recognition algorithm and prove the API; **not
//!   production STT quality**.
//!
//! The [`PhonemeBank`] API is identical regardless of source,
//! so swapping in the Phase 2d corpus-derived templates is a
//! drop-in.

use adam_audio::mfcc::{MfccConfig, MfccSequence, mfcc};
use adam_audio::pitch::harmonic_voice;
use adam_phoneme::{Phoneme, PhonemeClass};
use std::collections::HashMap;

/// A reference MFCC time-series for one phoneme.
#[derive(Debug, Clone)]
pub struct PhonemeTemplate {
    pub phoneme: Phoneme,
    pub mfcc: MfccSequence,
}

/// A bank of phoneme templates: one [`PhonemeTemplate`] per
/// [`Phoneme`] variant the recogniser should consider.
#[derive(Debug, Clone, Default)]
pub struct PhonemeBank {
    templates: HashMap<Phoneme, PhonemeTemplate>,
}

impl PhonemeBank {
    /// Empty bank — register templates with [`Self::insert`].
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    /// Add or replace a template.
    pub fn insert(&mut self, template: PhonemeTemplate) {
        self.templates.insert(template.phoneme, template);
    }

    /// Look up a template.
    pub fn get(&self, phoneme: Phoneme) -> Option<&PhonemeTemplate> {
        self.templates.get(&phoneme)
    }

    /// Iterate over all (phoneme, template) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&Phoneme, &PhonemeTemplate)> {
        self.templates.iter()
    }

    /// Number of templates in the bank.
    pub fn len(&self) -> usize {
        self.templates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }

    /// Build a **synthetic** phoneme bank for development and
    /// algorithm validation.
    ///
    /// Each phoneme is assigned a parametric synth signature:
    ///
    /// - **Vowels**: harmonic stack with the formant-like
    ///   F0 we want the recogniser to associate with that vowel.
    /// - **Consonants**: high-frequency noise burst whose
    ///   spectral centre approximates the place of articulation.
    ///   (This is a coarse approximation — real consonants are
    ///   transient and spectrally complex; the synthetic
    ///   template is just rich enough to be distinguishable
    ///   from other consonants in DTW.)
    /// - **Boundary marker `Glottal`**: not in the bank
    ///   (matched by FST-level rules, not acoustic templates).
    ///
    /// This is **not production STT quality** — it exists so
    /// the recognition pipeline can be tested end-to-end before
    /// Phase 2d delivers real templates.
    pub fn synthetic(sample_rate: u32) -> Self {
        let mut bank = Self::new();
        let cfg = MfccConfig::default();

        for &phoneme in Phoneme::ALL {
            if matches!(phoneme.class(), PhonemeClass::Boundary) {
                continue;
            }
            let mfcc_seq = synth_template_mfcc(phoneme, sample_rate, &cfg);
            bank.insert(PhonemeTemplate {
                phoneme,
                mfcc: mfcc_seq,
            });
        }
        bank
    }
}

/// Synthesise one phoneme's MFCC template.
fn synth_template_mfcc(phoneme: Phoneme, sample_rate: u32, cfg: &MfccConfig) -> MfccSequence {
    // 200 ms of synthesised audio is enough for ~20 MFCC
    // frames at 10 ms hop — comparable to a real spoken
    // phoneme.
    let duration_s = 0.20;
    let samples = match phoneme.class() {
        PhonemeClass::Vowel => synth_vowel(phoneme, duration_s, sample_rate),
        PhonemeClass::Consonant => synth_consonant(phoneme, duration_s, sample_rate),
        PhonemeClass::Boundary => unreachable!(),
    };
    mfcc(&samples, sample_rate, cfg)
}

/// Synthesise a vowel: harmonic stack with a phoneme-specific
/// F0. Different vowels get different F0 anchors so their
/// MFCC spectra are distinguishable — not because real vowel
/// identity is in F0 (it's in formants), but because this
/// produces a separable synthetic bank for algorithm testing.
fn synth_vowel(phoneme: Phoneme, duration_s: f32, sample_rate: u32) -> Vec<f32> {
    use Phoneme::*;
    let f0 = match phoneme {
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
    };
    harmonic_voice(f0, duration_s, sample_rate, 0.4, 4)
}

/// Synthesise a consonant: a noise burst with a
/// phoneme-specific spectral centre approximating its
/// place of articulation.
fn synth_consonant(phoneme: Phoneme, duration_s: f32, sample_rate: u32) -> Vec<f32> {
    use Phoneme::*;
    // Centre frequencies for spectral noise — rough proxies
    // for the place of articulation. (Real consonants are
    // far more complex; this is for synthetic-bank
    // separability.)
    let centre_hz: f32 = match phoneme {
        P | B | M => 500.0,                       // bilabial — low
        F | V => 700.0,                           // labiodental
        T | D | S | Z | N | L | R | Ts => 4000.0, // alveolar — high
        Sh | Zh | Ch => 3500.0,                   // postalveolar
        Shch => 3200.0,                           // alveolopalatal
        J => 2500.0,                              // palatal
        K | G | Ng | X => 2000.0,                 // velar
        W => 800.0,                               // labiovelar
        Q | Gh => 1500.0,                         // uvular
        H => 1200.0,                              // glottal
        _ => 2000.0,
    };
    let n = (duration_s * sample_rate as f32) as usize;
    // Narrow-band noise: sine at centre + small random
    // pseudo-noise modulation (deterministic via LCG).
    let mut state: u64 = (phoneme as u8 as u64).wrapping_mul(0x9E3779B97F4A7C15);
    (0..n)
        .map(|i| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let noise = ((state >> 11) as f32) / ((1_u64 << 53) as f32) - 0.5;
            let t = i as f32 / sample_rate as f32;
            let carrier = (2.0 * std::f32::consts::PI * centre_hz * t).sin();
            0.3 * carrier + 0.2 * noise
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use adam_phoneme::Phoneme;

    #[test]
    fn empty_bank_is_empty() {
        let b = PhonemeBank::new();
        assert!(b.is_empty());
        assert_eq!(b.len(), 0);
        assert!(b.get(Phoneme::A).is_none());
    }

    #[test]
    fn insert_and_lookup() {
        let mut b = PhonemeBank::new();
        let mfcc = mfcc(
            &harmonic_voice(120.0, 0.2, 16_000, 0.4, 4),
            16_000,
            &MfccConfig::default(),
        );
        b.insert(PhonemeTemplate {
            phoneme: Phoneme::A,
            mfcc,
        });
        assert_eq!(b.len(), 1);
        assert!(b.get(Phoneme::A).is_some());
        assert!(b.get(Phoneme::B).is_none());
    }

    /// Synthetic bank covers every non-boundary phoneme.
    #[test]
    fn synthetic_bank_covers_all_sounded_phonemes() {
        let b = PhonemeBank::synthetic(16_000);
        for &p in Phoneme::ALL {
            if matches!(p.class(), PhonemeClass::Boundary) {
                assert!(b.get(p).is_none(), "{p:?} should not be in bank");
            } else {
                assert!(b.get(p).is_some(), "missing template for {p:?}");
            }
        }
        // 37 total - 1 boundary = 36.
        assert_eq!(b.len(), 36);
    }

    /// Each template's MFCC has the expected dimensionality.
    #[test]
    fn synthetic_templates_have_correct_dim() {
        let b = PhonemeBank::synthetic(16_000);
        for (p, t) in b.iter() {
            assert_eq!(t.mfcc.dim(), 13, "phoneme {p:?} has wrong dim");
            assert!(t.mfcc.num_frames() > 5, "phoneme {p:?} too few frames");
        }
    }

    /// Two **different** synthesised phonemes have
    /// distinguishable MFCC templates.
    #[test]
    fn distinct_phonemes_have_distinct_templates() {
        use crate::distance::euclidean_distance;
        let b = PhonemeBank::synthetic(16_000);
        let t_a = b.get(Phoneme::A).unwrap();
        let t_q = b.get(Phoneme::Q).unwrap();
        // Centre-frame distance between A (vowel, F0=100) and Q
        // (uvular consonant, centre=1500) should be large.
        let d = euclidean_distance(
            &t_a.mfcc.frames[t_a.mfcc.num_frames() / 2],
            &t_q.mfcc.frames[t_q.mfcc.num_frames() / 2],
        );
        assert!(d > 1.0, "synthetic A and Q templates too similar: {d}");
    }
}
