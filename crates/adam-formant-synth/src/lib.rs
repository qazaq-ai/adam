// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `adam-formant-synth` — pure-Rust formant-based Kazakh
//! speech synthesiser.
//!
//! ## Why this exists
//!
//! The v6.3 STT path stalled at ~88% PER on real-world FLEURS
//! audio. Diagnosis: the per-phoneme acoustic model (one
//! centroid per phoneme, learned from heterogeneous multi-
//! speaker recordings) cannot separate Kazakh phonemes that
//! overlap heavily in cepstral-mean space.
//!
//! User directive (2026-05-28): generate the corpus
//! mathematically — sound is just sampled pressure waves; Rust
//! plus the physics of source-filter speech synthesis can
//! produce a controllable, structured, homogeneous bank
//! without depending on field recordings.
//!
//! ## What this crate provides
//!
//! Classic source-filter synthesis (Klatt / Fant), pure-Rust,
//! deterministic, no external dependencies:
//!
//! - [`source`] — glottal pulse train for voiced sources,
//!   deterministic XorShift white noise for unvoiced, linear
//!   attack/release envelope.
//! - [`filter`] — biquad band-pass resonator + cascaded
//!   formant filter + 6 dB/oct lip-radiation pre-emphasis.
//! - [`vowels`] — Kazakh vowel formant tables (F1/F2/F3 + BW)
//!   for all 10 vowels of the v6.3 inventory.
//! - [`consonants`] — manner-specific synthesis recipes for
//!   stops, fricatives, nasals, liquids, glides, affricates.
//! - [`synth_phoneme`] / [`synth_word`] — high-level entry
//!   points that consume a [`Phoneme`] sequence and return
//!   16 kHz mono PCM.
//!
//! ## Output is "one voice"
//!
//! By design every utterance produced by this crate comes out
//! of the same vocal-tract model with the same F0. That's a
//! feature for the v6.3 thesis test: a homogeneous bank built
//! from this synthesiser, recognised against synthesiser
//! output, gives the **upper bound** of recogniser accuracy
//! with the current pipeline. If PER is still high on that
//! corpus, the bottleneck is the recogniser, not the data.

#![forbid(unsafe_code)]

use adam_phoneme::Phoneme;

pub mod consonants;
pub mod filter;
pub mod source;
pub mod voice;
pub mod vowels;

pub use voice::{
    ALL_VOICES, BARITONE, BASS, CHILD_OLDER, CHILD_YOUNG, CONTRALTO, ELDERLY_FEMALE, ELDERLY_MALE,
    MEZZO, SOPRANO, TENOR, VoiceProfile, YOUTH_FEMALE, YOUTH_MALE,
};
pub use vowels::DEFAULT_F0_HZ;

/// Synthesis configuration.
///
/// Bundles a [`VoiceProfile`] (timbre) with per-utterance
/// rendering knobs (sample rate, phoneme duration, ramp).
/// The voice profile drives F0, formant scaling, jitter and
/// breath; the rest controls how individual phonemes are
/// rendered.
#[derive(Debug, Clone, Copy)]
pub struct SynthConfig {
    pub sample_rate: u32,
    /// Speaker timbre. `VoiceProfile::default()` = baritone.
    pub voice: VoiceProfile,
    /// Default duration per phoneme, seconds. Real phonemes
    /// vary (~50-200 ms); this is the per-phoneme baseline used
    /// when no explicit duration is supplied.
    pub phoneme_duration_s: f32,
    /// Attack / release ramp applied to each phoneme so
    /// concatenation doesn't click. Seconds.
    pub edge_ramp_s: f32,
    /// Seed for the deterministic noise PRNG. Same input ⇒
    /// byte-identical output.
    pub seed: u64,
}

impl Default for SynthConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16_000,
            voice: VoiceProfile::default(),
            phoneme_duration_s: 0.12,
            edge_ramp_s: 0.005,
            seed: 1,
        }
    }
}

impl SynthConfig {
    /// Build a config with a specific voice preset, defaults
    /// for everything else.
    pub fn with_voice(voice: VoiceProfile) -> Self {
        Self {
            voice,
            ..Self::default()
        }
    }
}

/// Synthesise a single phoneme to PCM.
pub fn synth_phoneme(phoneme: Phoneme, cfg: &SynthConfig) -> Vec<f32> {
    let mut pcm = match (phoneme.is_vowel(), phoneme.is_consonant()) {
        (true, false) => synth_vowel(phoneme, cfg),
        (false, true) => consonants::synth_consonant(
            phoneme,
            cfg.phoneme_duration_s,
            cfg.sample_rate,
            cfg.seed,
            &cfg.voice,
        ),
        _ => vec![0.0; (cfg.phoneme_duration_s * cfg.sample_rate as f32) as usize],
    };
    // The resonator coefficient choice gives a small static
    // gain; normalise each phoneme to a target peak of 0.3 so
    // every output is at a consistent listenable level and well
    // above the quality-gate RMS floor. (Per-phoneme peak
    // normalisation is fine because we keep all phonemes ~same
    // loudness when concatenated.)
    normalise_to_peak(&mut pcm, 0.30);
    source::apply_envelope(&mut pcm, cfg.edge_ramp_s, cfg.edge_ramp_s, cfg.sample_rate);
    pcm
}

fn normalise_to_peak(samples: &mut [f32], target_peak: f32) {
    let peak = samples.iter().fold(0.0_f32, |m, &x| m.max(x.abs()));
    if peak < 1e-12 {
        return;
    }
    let g = target_peak / peak;
    for s in samples.iter_mut() {
        *s *= g;
    }
}

/// Synthesise a phoneme sequence (= a word, syllable, or
/// whole utterance) by per-phoneme synthesis + concatenation.
/// Smooth attack/release ramps on each phoneme avoid clicks at
/// boundaries.
pub fn synth_word(phonemes: &[Phoneme], cfg: &SynthConfig) -> Vec<f32> {
    let mut out: Vec<f32> = Vec::new();
    for &p in phonemes {
        out.extend(synth_phoneme(p, cfg));
    }
    out
}

/// Voiced vowel via glottal pulse train through F1/F2/F3
/// formant cascade. Formant centres are scaled by
/// `cfg.voice.formant_scale` (vocal-tract-length normalisation),
/// the source carries `voice.jitter` / `voice.breath` perturbations,
/// and pulse frequency = `voice.f0_hz`.
fn synth_vowel(phoneme: Phoneme, cfg: &SynthConfig) -> Vec<f32> {
    let base = vowels::formants_of(phoneme).unwrap_or(vowels::VowelFormants {
        f1: (500.0, 80.0),
        f2: (1500.0, 90.0),
        f3: (2500.0, 120.0),
    });
    let scale = cfg.voice.formant_scale;
    let scaled: [(f32, f32); 3] = [
        (base.f1.0 * scale, base.f1.1),
        (base.f2.0 * scale, base.f2.1),
        (base.f3.0 * scale, base.f3.1),
    ];
    let excitation = source::glottal_pulse_train_v(
        cfg.voice.f0_hz,
        cfg.phoneme_duration_s,
        cfg.sample_rate,
        cfg.voice.jitter,
        cfg.voice.breath,
        cfg.seed,
    );
    let mut filt = filter::FormantFilter::new(&scaled, cfg.sample_rate);
    filt.process(&excitation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_inventory_phoneme_synthesises_audibly() {
        let cfg = SynthConfig::default();
        for &p in Phoneme::ALL {
            if !p.is_vowel() && !p.is_consonant() {
                continue;
            }
            let pcm = synth_phoneme(p, &cfg);
            assert!(!pcm.is_empty(), "{p:?} produced empty buffer");
            assert!(
                pcm.iter().all(|x| x.is_finite()),
                "{p:?} produced non-finite samples"
            );
            let rms = (pcm.iter().map(|x| x * x).sum::<f32>() / pcm.len() as f32).sqrt();
            assert!(rms > 1e-4, "{p:?} too quiet: rms={rms}");
        }
    }

    #[test]
    fn synth_is_deterministic() {
        let cfg = SynthConfig::default();
        let a = synth_phoneme(Phoneme::A, &cfg);
        let b = synth_phoneme(Phoneme::A, &cfg);
        assert_eq!(a, b, "same input gave different output");
    }

    #[test]
    fn synth_word_concatenates() {
        let cfg = SynthConfig::default();
        let sep = synth_phoneme(Phoneme::B, &cfg).len() + synth_phoneme(Phoneme::A, &cfg).len();
        let joined = synth_word(&[Phoneme::B, Phoneme::A], &cfg);
        assert_eq!(joined.len(), sep);
    }

    /// Every voice preset can synthesise the full vowel
    /// inventory without producing NaN / silence.
    #[test]
    fn every_voice_synthesises_every_vowel() {
        for voice in ALL_VOICES {
            let cfg = SynthConfig::with_voice(*voice);
            for &p in Phoneme::ALL {
                if !p.is_vowel() {
                    continue;
                }
                let pcm = synth_phoneme(p, &cfg);
                assert!(!pcm.is_empty(), "{} / {p:?} empty", voice.name);
                assert!(
                    pcm.iter().all(|x| x.is_finite()),
                    "{} / {p:?} non-finite",
                    voice.name
                );
                let rms = (pcm.iter().map(|x| x * x).sum::<f32>() / pcm.len() as f32).sqrt();
                assert!(rms > 1e-4, "{} / {p:?} too quiet: rms={rms}", voice.name);
            }
        }
    }

    /// Soprano output differs from bass output (different
    /// timbre — F0 and formant scale both shift). Quick sanity:
    /// at least one sample of vowel A differs by ≥0.001 between
    /// the two voices.
    #[test]
    fn soprano_distinct_from_bass() {
        let bass = synth_phoneme(Phoneme::A, &SynthConfig::with_voice(BASS));
        let sop = synth_phoneme(Phoneme::A, &SynthConfig::with_voice(SOPRANO));
        // Same duration ⇒ same length.
        assert_eq!(bass.len(), sop.len());
        let any_diff = bass
            .iter()
            .zip(sop.iter())
            .any(|(b, s)| (b - s).abs() > 0.001);
        assert!(any_diff, "soprano and bass produced identical samples");
    }

    /// Spectral sanity: the vowel A should have a stronger
    /// 750 Hz (F1) presence than the vowel I (F1 ≈ 300 Hz).
    /// Measured by short-window energy in a band around 750 Hz.
    #[test]
    fn vowel_a_has_more_low_formant_energy_than_i() {
        let cfg = SynthConfig {
            phoneme_duration_s: 0.20,
            ..Default::default()
        };
        let a = synth_phoneme(Phoneme::A, &cfg);
        let i = synth_phoneme(Phoneme::I, &cfg);
        // Coarse: pass each through a 750 Hz band-pass and
        // compare RMS — A should win.
        let mut bp_a = filter::Resonator::new(750.0, 100.0, cfg.sample_rate);
        let mut bp_i = filter::Resonator::new(750.0, 100.0, cfg.sample_rate);
        let a_rms = rms_of_filtered(&a, &mut bp_a);
        let i_rms = rms_of_filtered(&i, &mut bp_i);
        assert!(
            a_rms > i_rms,
            "A should dominate at 750 Hz: A={a_rms} I={i_rms}"
        );
    }

    fn rms_of_filtered(samples: &[f32], filt: &mut filter::Resonator) -> f32 {
        let out: Vec<f32> = samples.iter().map(|&x| filt.step(x)).collect();
        // Skip transient (first 5 ms ≈ 80 samples @ 16 kHz).
        let skip = 80.min(out.len() / 2);
        let tail = &out[skip..];
        (tail.iter().map(|x| x * x).sum::<f32>() / tail.len() as f32).sqrt()
    }
}
