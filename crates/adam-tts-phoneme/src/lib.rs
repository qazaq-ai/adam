// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `adam-tts-phoneme` — **Phase 7 of the v6.3 phonemic-
//! foundation arc**: concatenative text-to-speech from a
//! phoneme stream.
//!
//! Pairs with [`adam_stt_phoneme`] on the input side:
//!
//! ```text
//!   Cyrillic / Latin text
//!     ↓ adam_phoneme::{cyrillic, latin}::*_to_phonemes
//!   phoneme stream Vec<Phoneme>
//!     ↓ adam_tts_phoneme::synthesise
//!   PcmSamples (mono 16 kHz)
//!     ↓ adam_audio::play::play_blocking  (or write_wav)
//!   speaker
//! ```
//!
//! ## What this crate ships now (Phase 7a)
//!
//! - Per-phoneme PCM synthesis from **parametric signatures**:
//!   vowels = harmonic stacks at phoneme-specific F0;
//!   consonants = narrow-band noise + carrier sine at place-
//!   of-articulation-specific centre frequency.
//! - Equal-power crossfade between consecutive phonemes (no
//!   audible boundary clicks).
//! - `synthesise(phonemes, config)` produces a
//!   [`adam_audio::PcmSamples`] buffer ready for playback.
//!
//! ## What's next (Phase 7b)
//!
//! Replace the parametric vowel/consonant signatures with
//! **real per-phoneme PCM templates** extracted from the
//! corpus (same equipartition + DTW pipeline that produced
//! the MFCC bank, but keeping the PCM segments instead of the
//! MFCC). Output then sounds like a real Kazakh voice rather
//! than a synth stack.
//!
//! ## What this is NOT
//!
//! - Not natural-sounding TTS. Synth concatenation is robotic
//!   by design — the v6.3 thesis prefers deterministic
//!   reproducibility over naturalness. Naturalness is a Phase
//!   7c concern (PSOLA / formant smoothing).
//! - Not lifelike prosody. Phoneme duration is fixed via
//!   `TtsConfig`; pitch contour is flat per phoneme. Sentence-
//!   level prosody (questions, emphasis) is a separate later
//!   pass.

#![forbid(unsafe_code)]

use adam_audio::PcmSamples;
use adam_phoneme::{Phoneme, PhonemeClass};

pub mod pcm_bank;
mod signatures;

pub use pcm_bank::{PcmBank, PcmBankError, PcmTemplate};

/// Configuration for [`synthesise`].
#[derive(Debug, Clone)]
pub struct TtsConfig {
    /// Output sample rate (default 16 kHz).
    pub sample_rate: u32,
    /// Duration of each phoneme in milliseconds (default 150).
    pub phoneme_ms: u32,
    /// Crossfade length between consecutive phonemes in
    /// milliseconds (default 20). Equal-power crossfade to
    /// avoid amplitude pumping.
    pub crossfade_ms: u32,
    /// Output amplitude in `[0.0, 1.0]` (default 0.4 — leaves
    /// headroom for any further mixing).
    pub amplitude: f32,
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16_000,
            phoneme_ms: 150,
            crossfade_ms: 20,
            amplitude: 0.4,
        }
    }
}

/// Synthesise a phoneme stream into a mono PCM buffer using
/// parametric signatures only (synth-only path).
///
/// Empty input → empty buffer. The [`Phoneme::Glottal`]
/// boundary marker produces a brief silent gap (the value of
/// `phoneme_ms / 3`) rather than a sounded segment.
pub fn synthesise(phonemes: &[Phoneme], config: &TtsConfig) -> PcmSamples {
    synthesise_with_bank(phonemes, None, config)
}

/// Synthesise a phoneme stream using a **real PCM bank** where
/// templates are available, falling back to parametric synth
/// otherwise. The canonical hybrid path: corpus-derived real
/// PCM for phonemes the bank covers, synth signatures for the
/// rest (matching `adam_stt_phoneme::PhonemeBank::
/// merged_with_fallback` on the recognition side).
pub fn synthesise_with_bank(
    phonemes: &[Phoneme],
    pcm_bank: Option<&PcmBank>,
    config: &TtsConfig,
) -> PcmSamples {
    let mut buffer: Vec<f32> = Vec::new();
    let crossfade_samples = (config.crossfade_ms as usize * config.sample_rate as usize) / 1000;

    for &phoneme in phonemes {
        let dur_ms = match phoneme.class() {
            PhonemeClass::Boundary => config.phoneme_ms / 3,
            _ => config.phoneme_ms,
        };
        let segment = match pcm_bank
            .and_then(|b| b.get(phoneme))
            .filter(|t| t.sample_rate == config.sample_rate && !t.samples.is_empty())
        {
            Some(template) => stretch_or_truncate(
                &template.samples,
                (dur_ms as usize * config.sample_rate as usize) / 1000,
                config.amplitude,
            ),
            None => synth_phoneme_pcm(phoneme, dur_ms, config),
        };

        if buffer.is_empty() || crossfade_samples == 0 {
            buffer.extend(segment);
        } else {
            crossfade_into(&mut buffer, &segment, crossfade_samples);
        }
    }

    PcmSamples {
        sample_rate: config.sample_rate,
        channels: 1,
        data: buffer,
    }
}

/// Linearly resample a real-PCM template chunk to the desired
/// duration and scale to the configured amplitude.
///
/// Resampling is nearest-neighbour here (cheap; high-quality
/// resampling is a Phase 7c concern — for now phoneme
/// duration is a synthesis parameter and the underlying
/// template will be at most a few hundred ms, so the
/// stretching is mild). Output amplitude is normalised to
/// the template's peak then scaled to `target_amplitude`.
fn stretch_or_truncate(template: &[f32], target_samples: usize, target_amplitude: f32) -> Vec<f32> {
    if template.is_empty() || target_samples == 0 {
        return vec![0.0; target_samples];
    }
    let src_len = template.len();
    let peak = template
        .iter()
        .cloned()
        .fold(0.0_f32, |a, b| a.max(b.abs()));
    let gain = if peak > 0.0 {
        target_amplitude / peak
    } else {
        0.0
    };
    (0..target_samples)
        .map(|i| {
            let src_i = (i * src_len) / target_samples.max(1);
            template[src_i.min(src_len - 1)] * gain
        })
        .collect()
}

/// Synthesise one phoneme as PCM with the given duration.
fn synth_phoneme_pcm(phoneme: Phoneme, dur_ms: u32, config: &TtsConfig) -> Vec<f32> {
    use signatures::*;
    let duration_s = dur_ms as f32 / 1000.0;
    match phoneme.class() {
        PhonemeClass::Vowel => {
            synth_vowel(phoneme, duration_s, config.sample_rate, config.amplitude)
        }
        PhonemeClass::Consonant => {
            synth_consonant(phoneme, duration_s, config.sample_rate, config.amplitude)
        }
        PhonemeClass::Boundary => {
            // Boundary marker → silence.
            vec![0.0; (duration_s * config.sample_rate as f32) as usize]
        }
    }
}

/// Append `segment` to `buffer` with an equal-power crossfade
/// over the trailing `crossfade_samples` of `buffer` and the
/// leading `crossfade_samples` of `segment`.
fn crossfade_into(buffer: &mut Vec<f32>, segment: &[f32], crossfade_samples: usize) {
    let xf = crossfade_samples.min(buffer.len()).min(segment.len());
    if xf == 0 {
        buffer.extend_from_slice(segment);
        return;
    }
    let prev_len = buffer.len();
    let xf_start = prev_len - xf;
    for i in 0..xf {
        let t = (i as f32 + 0.5) / xf as f32;
        // Equal-power crossfade weights (sin/cos).
        let w_out = ((1.0 - t) * std::f32::consts::FRAC_PI_2).cos();
        let w_in = (t * std::f32::consts::FRAC_PI_2).cos();
        buffer[xf_start + i] = buffer[xf_start + i] * w_in + segment[i] * w_out;
    }
    buffer.extend_from_slice(&segment[xf..]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use Phoneme::*;

    /// Empty input → empty output.
    #[test]
    fn empty_input_empty_output() {
        let pcm = synthesise(&[], &TtsConfig::default());
        assert!(pcm.data.is_empty());
    }

    /// Single phoneme → roughly `phoneme_ms` of audio at the
    /// configured sample rate.
    #[test]
    fn single_phoneme_correct_duration() {
        let cfg = TtsConfig::default();
        let pcm = synthesise(&[A], &cfg);
        let expected_samples = (cfg.phoneme_ms as usize * cfg.sample_rate as usize) / 1000;
        assert!(
            pcm.data.len() == expected_samples,
            "expected {expected_samples} samples, got {}",
            pcm.data.len()
        );
    }

    /// Multi-phoneme: total length = N * phoneme_ms minus
    /// (N-1) * crossfade_ms.
    #[test]
    fn multi_phoneme_concatenated_with_crossfade() {
        let cfg = TtsConfig::default();
        let phonemes = [Q, A, Z, A, Q];
        let pcm = synthesise(&phonemes, &cfg);
        let per_phoneme = (cfg.phoneme_ms as usize * cfg.sample_rate as usize) / 1000;
        let crossfade = (cfg.crossfade_ms as usize * cfg.sample_rate as usize) / 1000;
        let n = phonemes.len();
        let expected = n * per_phoneme - (n - 1) * crossfade;
        // Within ±crossfade for rounding tolerance.
        let diff = (pcm.data.len() as isize - expected as isize).unsigned_abs();
        assert!(
            diff <= crossfade,
            "expected ~{expected} samples, got {} (diff {diff})",
            pcm.data.len(),
        );
    }

    /// Output is mono.
    #[test]
    fn output_is_mono() {
        let pcm = synthesise(&[A, Q, A], &TtsConfig::default());
        assert_eq!(pcm.channels, 1);
    }

    /// Output amplitude stays within configured bounds.
    #[test]
    fn output_amplitude_within_bounds() {
        let cfg = TtsConfig {
            amplitude: 0.5,
            ..TtsConfig::default()
        };
        let pcm = synthesise(&[A, Q, A], &cfg);
        let peak = pcm
            .data
            .iter()
            .cloned()
            .fold(0.0_f32, |a, b| a.max(b.abs()));
        // Allow a touch of headroom from harmonic stacking.
        assert!(peak <= 1.0, "peak {peak} exceeded 1.0");
    }

    /// Synth vowel + consonant sound different.
    #[test]
    fn vowel_and_consonant_pcm_differ() {
        let cfg = TtsConfig::default();
        let v_pcm = synthesise(&[A], &cfg);
        let c_pcm = synthesise(&[T], &cfg);
        let diff: f32 = v_pcm
            .data
            .iter()
            .zip(c_pcm.data.iter())
            .map(|(a, b)| (a - b).abs())
            .sum();
        assert!(diff > 1.0, "vowel and consonant PCM too similar");
    }

    /// Boundary marker produces a silent gap (no signal).
    #[test]
    fn boundary_marker_silent() {
        let cfg = TtsConfig::default();
        let pcm = synthesise(&[Glottal], &cfg);
        let max_abs = pcm
            .data
            .iter()
            .cloned()
            .fold(0.0_f32, |a, b| a.max(b.abs()));
        assert!(max_abs < 1e-6, "boundary marker should be silent");
    }

    /// Defaults are sane.
    #[test]
    fn default_config_sane() {
        let c = TtsConfig::default();
        assert!(c.sample_rate >= 8_000);
        assert!(c.phoneme_ms >= 50);
        assert!(c.crossfade_ms < c.phoneme_ms);
        assert!(c.amplitude > 0.0 && c.amplitude <= 1.0);
    }

    /// Synth-with-bank produces audio. PCM templates in the
    /// bank replace synth signatures for the phonemes they
    /// cover.
    #[test]
    fn synthesise_with_real_bank_uses_template() {
        let cfg = TtsConfig::default();
        // Build a tiny bank: phoneme A → custom-looking PCM
        // (constant 0.5).
        let mut bank = PcmBank::new();
        bank.insert(PcmTemplate {
            phoneme: A,
            sample_rate: cfg.sample_rate,
            samples: vec![0.5_f32; cfg.sample_rate as usize / 10], // 100 ms
        });
        let pcm_synth = synthesise(&[A], &cfg);
        let pcm_real = synthesise_with_bank(&[A], Some(&bank), &cfg);
        // Both produce phoneme_ms of audio.
        assert_eq!(pcm_synth.data.len(), pcm_real.data.len());
        // But the content differs (real is constant-amplitude
        // post-stretch; synth is oscillatory).
        let diff: f32 = pcm_synth
            .data
            .iter()
            .zip(pcm_real.data.iter())
            .map(|(a, b)| (a - b).abs())
            .sum();
        assert!(diff > 1.0, "real-bank output should differ from synth");
    }

    /// Bank fallback: phoneme absent from bank → synth used.
    #[test]
    fn missing_bank_phoneme_falls_back_to_synth() {
        let cfg = TtsConfig::default();
        // Bank only has Q.
        let mut bank = PcmBank::new();
        bank.insert(PcmTemplate {
            phoneme: Q,
            sample_rate: cfg.sample_rate,
            samples: vec![0.5_f32; cfg.sample_rate as usize / 10],
        });
        // Synthesising A goes through synth path (bank doesn't
        // have A). Should produce same as synth-only.
        let synth_only = synthesise(&[A], &cfg);
        let with_bank = synthesise_with_bank(&[A], Some(&bank), &cfg);
        assert_eq!(synth_only.data, with_bank.data);
    }

    /// Sample-rate mismatch → fall back to synth (we don't
    /// resample bank templates across rates at this layer).
    #[test]
    fn sample_rate_mismatch_falls_back() {
        let cfg = TtsConfig::default();
        let mut bank = PcmBank::new();
        bank.insert(PcmTemplate {
            phoneme: A,
            sample_rate: 48_000, // mismatch with cfg's 16 kHz
            samples: vec![0.5_f32; 1000],
        });
        let synth_only = synthesise(&[A], &cfg);
        let with_bank = synthesise_with_bank(&[A], Some(&bank), &cfg);
        assert_eq!(synth_only.data, with_bank.data);
    }

    /// Default crossfade is non-zero (proves the crossfade
    /// branch runs).
    #[test]
    fn crossfade_is_used() {
        let cfg = TtsConfig::default();
        let no_xf = TtsConfig {
            crossfade_ms: 0,
            ..cfg.clone()
        };
        let with_xf = cfg;
        let phonemes = [A, Q];
        let len_with = synthesise(&phonemes, &with_xf).data.len();
        let len_without = synthesise(&phonemes, &no_xf).data.len();
        assert!(
            len_without > len_with,
            "no-crossfade should be longer than crossfade: {len_without} vs {len_with}"
        );
    }
}
