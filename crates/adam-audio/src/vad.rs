// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Energy-based voice activity detection.
//!
//! The simplest VAD that works: compute the root-mean-square
//! energy of a short audio frame; classify as speech / silence
//! by comparing against a configurable threshold. Robust enough
//! for `record_until_silence` in a quiet recording environment;
//! Phase 6 may add zero-crossing-rate / spectral-flatness
//! features for noisier conditions.
//!
//! Frame size for the recorder is 30 ms (matching the WebRTC
//! VAD convention), which at 16 kHz mono is 480 samples per
//! frame. The threshold default is `0.01` (i.e. RMS amplitude
//! 1 % of full-scale) — quiet typing is below; ordinary speech
//! at 30 cm is comfortably above.

/// Compute the root-mean-square amplitude of a frame.
///
/// Returns `0.0` for an empty frame.
pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

/// Classify a frame as silence if its RMS amplitude is below the
/// threshold. The threshold is in linear `[-1.0, 1.0]` amplitude
/// units; `0.01` is a reasonable default for indoor speech.
pub fn is_silence(samples: &[f32], threshold: f32) -> bool {
    rms(samples) < threshold
}

/// Default VAD frame size at 16 kHz (30 ms = 480 samples).
pub const DEFAULT_FRAME_SIZE_16KHZ: usize = 480;

/// Default RMS silence threshold.
pub const DEFAULT_SILENCE_THRESHOLD: f32 = 0.01;

#[cfg(test)]
mod tests {
    use super::*;

    /// Empty frame has zero RMS.
    #[test]
    fn rms_empty_is_zero() {
        assert_eq!(rms(&[]), 0.0);
    }

    /// All-zero frame is silence at any positive threshold.
    #[test]
    fn all_zeros_is_silence() {
        let z = vec![0.0; 480];
        assert!(is_silence(&z, 0.01));
        assert!(is_silence(&z, 1e-6));
    }

    /// Loud sine wave is NOT silence.
    #[test]
    fn loud_signal_is_not_silence() {
        // ~0.5 amplitude sine wave; RMS is 0.5/sqrt(2) ≈ 0.354.
        let n = 480_usize;
        let samples: Vec<f32> = (0..n)
            .map(|i| 0.5 * ((2.0 * std::f32::consts::PI * 440.0 * i as f32 / 16_000.0).sin()))
            .collect();
        let r = rms(&samples);
        assert!(r > 0.3 && r < 0.4, "expected ~0.35, got {r}");
        assert!(!is_silence(&samples, 0.01));
    }

    /// Quiet noise at amplitude 0.001 is silence at threshold 0.01.
    #[test]
    fn quiet_noise_is_silence() {
        let samples = vec![0.001_f32; 480];
        assert!(is_silence(&samples, 0.01));
    }

    /// Threshold boundary: at the strict boundary the comparison
    /// goes "not silence"; well above threshold is silence.
    /// (Floating-point round-off in `sqrt` may shift RMS by ~1e-7
    /// from the analytic value, so we test the behavioural
    /// boundary, not the exact equality.)
    #[test]
    fn threshold_boundary_strict() {
        let r = 0.05_f32;
        let samples = vec![r; 100];
        // RMS of a constant signal equals its amplitude — up to
        // f32 rounding (< 1e-6).
        assert!((rms(&samples) - r).abs() < 1e-5);
        // A threshold well below the signal RMS classifies as
        // not-silence; a threshold well above classifies as
        // silence.
        assert!(!is_silence(&samples, r - 1e-3));
        assert!(is_silence(&samples, r + 1e-3));
    }

    /// Defaults are sane.
    #[test]
    fn defaults_within_expected_ranges() {
        assert_eq!(DEFAULT_FRAME_SIZE_16KHZ, 480);
        const _: () = assert!(DEFAULT_SILENCE_THRESHOLD > 0.0);
        const _: () = assert!(DEFAULT_SILENCE_THRESHOLD < 0.1);
    }
}
