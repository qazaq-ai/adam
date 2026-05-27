// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Real-input FFT wrapper over `rustfft`.
//!
//! `rustfft` operates on complex slices. Audio frames are
//! real-valued — converting to / from complex by hand at every
//! call site is noisy. This module provides:
//!
//! - [`magnitude_spectrum`] — windowed FFT of a real frame,
//!   returning the magnitude of the first `N/2 + 1` bins
//!   (positive frequencies only).
//! - [`power_spectrum`] — same but magnitude², the canonical
//!   input to a Mel filterbank.
//! - [`bin_frequency`] — convert bin index to Hz.
//!
//! All entry points are stateful through a reusable
//! [`FftPlanner`] when called repeatedly (the planner caches
//! twiddle factors); for one-off calls the free functions
//! create and discard a planner internally.

use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;

/// Compute the **magnitude spectrum** of a real frame.
///
/// Returns `frame.len() / 2 + 1` non-negative-frequency bins;
/// the DC bin is at index 0 and the Nyquist bin at index
/// `frame.len() / 2`.
///
/// For a length-2048 frame this returns 1025 bins. For
/// downstream consumers that want only positive frequencies
/// (Mel filterbank, magnitude visualisation) this is what they
/// need.
pub fn magnitude_spectrum(frame: &[f32]) -> Vec<f32> {
    let mut planner: FftPlanner<f32> = FftPlanner::new();
    let fft = planner.plan_fft_forward(frame.len());
    magnitude_with_plan(frame, fft.as_ref())
}

/// Same as [`magnitude_spectrum`] but with a pre-built FFT
/// plan — call this in a hot loop where the frame length is
/// constant.
pub fn magnitude_with_plan(frame: &[f32], fft: &dyn Fft<f32>) -> Vec<f32> {
    debug_assert_eq!(
        frame.len(),
        fft.len(),
        "frame length must match planned FFT length",
    );
    let mut buf: Vec<Complex<f32>> = frame.iter().map(|&s| Complex::new(s, 0.0)).collect();
    fft.process(&mut buf);
    let n_pos = frame.len() / 2 + 1;
    buf.iter()
        .take(n_pos)
        .map(|c| (c.re * c.re + c.im * c.im).sqrt())
        .collect()
}

/// Compute the **power spectrum** (magnitude²) of a real frame.
/// This is the canonical input to a Mel filterbank.
pub fn power_spectrum(frame: &[f32]) -> Vec<f32> {
    let mut planner: FftPlanner<f32> = FftPlanner::new();
    let fft = planner.plan_fft_forward(frame.len());
    power_with_plan(frame, fft.as_ref())
}

/// Same as [`power_spectrum`] but with a pre-built FFT plan.
pub fn power_with_plan(frame: &[f32], fft: &dyn Fft<f32>) -> Vec<f32> {
    debug_assert_eq!(frame.len(), fft.len());
    let mut buf: Vec<Complex<f32>> = frame.iter().map(|&s| Complex::new(s, 0.0)).collect();
    fft.process(&mut buf);
    let n_pos = frame.len() / 2 + 1;
    buf.iter()
        .take(n_pos)
        .map(|c| c.re * c.re + c.im * c.im)
        .collect()
}

/// Convert a bin index to its centre frequency in Hz.
/// `bin = i`, `n` is the FFT length, `sr` is the sample rate.
pub fn bin_frequency(bin: usize, fft_size: usize, sample_rate: u32) -> f32 {
    bin as f32 * sample_rate as f32 / fft_size as f32
}

/// Convert a frequency (Hz) to the nearest bin index.
pub fn frequency_to_bin(freq_hz: f32, fft_size: usize, sample_rate: u32) -> usize {
    (freq_hz * fft_size as f32 / sample_rate as f32).round() as usize
}

/// Cached FFT planner for hot paths.
///
/// Internally wraps [`rustfft::FftPlanner`]; constructing one
/// FFT of a given size is cheap, but reusing the planner
/// avoids the planning overhead in tight loops (STFT,
/// per-frame MFCC).
pub struct AdamFftPlanner {
    inner: FftPlanner<f32>,
}

impl AdamFftPlanner {
    pub fn new() -> Self {
        Self {
            inner: FftPlanner::new(),
        }
    }

    /// Get a forward FFT plan for the given size. The plan is
    /// reusable: call `.process(&mut buf)` on the returned
    /// `Arc<dyn Fft<f32>>` to run the FFT.
    pub fn plan_forward(&mut self, size: usize) -> Arc<dyn Fft<f32>> {
        self.inner.plan_fft_forward(size)
    }
}

impl Default for AdamFftPlanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    /// FFT of a pure sine wave concentrates energy in the
    /// matching frequency bin.
    #[test]
    fn pure_sine_lands_in_correct_bin() {
        let fft_size = 1024_usize;
        let sample_rate = 16_000_u32;
        let target_hz = 250.0_f32;
        let target_bin = frequency_to_bin(target_hz, fft_size, sample_rate);
        let samples: Vec<f32> = (0..fft_size)
            .map(|i| (2.0 * PI * target_hz * i as f32 / sample_rate as f32).sin())
            .collect();
        let mag = magnitude_spectrum(&samples);

        // The target bin must dominate the spectrum (be ≥ 5×
        // the average of all OTHER bins).
        let target_mag = mag[target_bin];
        let other_avg: f32 = mag
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                *i != target_bin && *i != target_bin.wrapping_sub(1) && *i != target_bin + 1
            })
            .map(|(_, &m)| m)
            .sum::<f32>()
            / (mag.len() - 3) as f32;
        assert!(
            target_mag > 5.0 * other_avg,
            "target bin {target_bin} ({target_mag}) not dominant over avg {other_avg}",
        );
    }

    /// DC component of a pure sine (no offset) is near zero.
    #[test]
    fn pure_sine_has_low_dc() {
        let samples: Vec<f32> = (0..1024)
            .map(|i| (2.0 * PI * 220.0 * i as f32 / 16_000.0).sin())
            .collect();
        let mag = magnitude_spectrum(&samples);
        // DC mag should be very small relative to peak.
        let peak = mag.iter().cloned().fold(0.0_f32, f32::max);
        assert!(mag[0] < 0.05 * peak, "DC={} peak={}", mag[0], peak);
    }

    /// DC offset shows up at bin 0.
    #[test]
    fn dc_offset_lands_at_bin_zero() {
        let samples = vec![1.0_f32; 1024];
        let mag = magnitude_spectrum(&samples);
        assert!(mag[0] > 100.0, "DC bin {} should be large", mag[0]);
        // No spurious peak elsewhere.
        for (i, &m) in mag.iter().enumerate().skip(2) {
            assert!(m < 1.0, "spurious peak at bin {i}: {m}");
        }
    }

    /// power_spectrum equals magnitude_spectrum².
    /// Uses relative tolerance: f32 accumulates ~1e-7 relative
    /// error in the FFT, then squaring doubles that — so the
    /// per-bin tolerance is set at 1e-5 of the peak value.
    #[test]
    fn power_is_magnitude_squared() {
        let samples: Vec<f32> = (0..512)
            .map(|i| (2.0 * PI * 440.0 * i as f32 / 16_000.0).sin())
            .collect();
        let mag = magnitude_spectrum(&samples);
        let pow = power_spectrum(&samples);
        let peak_pow = pow.iter().cloned().fold(0.0_f32, f32::max);
        let tol = 1e-5 * peak_pow.max(1.0);
        for (m, p) in mag.iter().zip(pow.iter()) {
            assert!(
                (m * m - p).abs() < tol,
                "{} vs {} (tol {tol})",
                m * m,
                p,
            );
        }
    }

    /// Bin frequency / inverse map round-trip.
    #[test]
    fn bin_frequency_roundtrip() {
        for &hz in &[100.0_f32, 250.0, 1000.0, 4000.0, 8000.0] {
            let bin = frequency_to_bin(hz, 1024, 16_000);
            let back = bin_frequency(bin, 1024, 16_000);
            assert!((back - hz).abs() < 16.0, "{hz} -> bin {bin} -> {back}");
        }
    }

    /// Spectrum length is N/2 + 1.
    #[test]
    fn spectrum_length_correct() {
        for n in [256_usize, 512, 1024, 2048] {
            let samples = vec![0.0_f32; n];
            assert_eq!(magnitude_spectrum(&samples).len(), n / 2 + 1);
            assert_eq!(power_spectrum(&samples).len(), n / 2 + 1);
        }
    }

    /// Planner is reusable: same plan applied to two different
    /// frames produces consistent results.
    #[test]
    fn planner_is_reusable() {
        let mut planner = AdamFftPlanner::new();
        let plan = planner.plan_forward(1024);
        let s1: Vec<f32> = (0..1024)
            .map(|i| (2.0 * PI * 200.0 * i as f32 / 16_000.0).sin())
            .collect();
        let s2: Vec<f32> = (0..1024)
            .map(|i| (2.0 * PI * 400.0 * i as f32 / 16_000.0).sin())
            .collect();
        let m1 = magnitude_with_plan(&s1, plan.as_ref());
        let m2 = magnitude_with_plan(&s2, plan.as_ref());
        assert_eq!(m1.len(), m2.len());
        // 200 Hz peak in m1, 400 Hz peak in m2 (different bins).
        let b1 = frequency_to_bin(200.0, 1024, 16_000);
        let b2 = frequency_to_bin(400.0, 1024, 16_000);
        assert!(m1[b1] > m1[b2], "m1 should peak at 200 Hz, got {m1:?}");
        assert!(m2[b2] > m2[b1], "m2 should peak at 400 Hz");
    }

    /// **Realistic voice**: harmonic stack at F0 = 150 Hz must
    /// show energy at 150, 300, 450, … Hz.
    #[test]
    fn harmonic_voice_shows_overtone_peaks() {
        let fft_size = 2048;
        let sample_rate = 16_000;
        let f0 = 150.0_f32;
        let samples: Vec<f32> = (0..fft_size)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                let mut s = 0.0_f32;
                for h in 1..=4 {
                    s += (2.0 * PI * f0 * h as f32 * t).sin() / (h as f32);
                }
                s
            })
            .collect();
        let mag = magnitude_spectrum(&samples);

        // F0, 2F0, 3F0, 4F0 bins should all exceed bins midway
        // between them.
        for h in 1..=4 {
            let bin = frequency_to_bin(f0 * h as f32, fft_size, sample_rate);
            let mid = frequency_to_bin(f0 * (h as f32 - 0.5), fft_size, sample_rate);
            assert!(
                mag[bin] > mag[mid],
                "harmonic {h} (bin {bin}, {}) not a peak vs midpoint (bin {mid}, {})",
                mag[bin],
                mag[mid],
            );
        }
    }
}
