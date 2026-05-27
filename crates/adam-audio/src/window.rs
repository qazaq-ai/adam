// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Windowing functions for FFT-based spectral analysis.
//!
//! A naive FFT on raw audio frames introduces severe spectral
//! leakage — the discontinuities at frame boundaries scatter
//! energy across all frequency bins. Multiplying the frame by
//! a smooth window (Hamming / Hann / Blackman) attenuates the
//! discontinuities and gives clean spectra.
//!
//! For speech (where the relevant content is in the first few
//! kHz and we don't need extreme sidelobe suppression), the
//! **Hamming** window is the canonical choice; **Hann** is a
//! slightly broader-banded alternative used by some
//! reference pipelines (Kaldi defaults to Hamming, librosa
//! defaults to Hann). We expose both; MFCC and STFT default to
//! Hamming.

/// Hamming window coefficients of length `n`. Formula:
///   `w[i] = 0.54 - 0.46 * cos(2π i / (n - 1))`
pub fn hamming(n: usize) -> Vec<f32> {
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![1.0];
    }
    let denom = (n - 1) as f32;
    (0..n)
        .map(|i| 0.54 - 0.46 * (2.0 * std::f32::consts::PI * i as f32 / denom).cos())
        .collect()
}

/// Hann window coefficients of length `n`. Formula:
///   `w[i] = 0.5 * (1 - cos(2π i / (n - 1)))`
pub fn hann(n: usize) -> Vec<f32> {
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![1.0];
    }
    let denom = (n - 1) as f32;
    (0..n)
        .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / denom).cos()))
        .collect()
}

/// Apply a window to a frame in place: `frame[i] *= window[i]`.
/// `frame.len()` must equal `window.len()`.
pub fn apply(frame: &mut [f32], window: &[f32]) {
    assert_eq!(
        frame.len(),
        window.len(),
        "frame ({}) and window ({}) length must match",
        frame.len(),
        window.len(),
    );
    for (s, &w) in frame.iter_mut().zip(window.iter()) {
        *s *= w;
    }
}

/// Apply a window to a copy of a frame and return the windowed
/// frame.
pub fn windowed(frame: &[f32], window: &[f32]) -> Vec<f32> {
    assert_eq!(frame.len(), window.len());
    frame
        .iter()
        .zip(window.iter())
        .map(|(&s, &w)| s * w)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hamming has the canonical endpoint values 0.08.
    #[test]
    fn hamming_endpoints_are_zero_point_08() {
        let w = hamming(401);
        assert!((w[0] - 0.08).abs() < 1e-5, "{}", w[0]);
        assert!((w[400] - 0.08).abs() < 1e-5, "{}", w[400]);
    }

    /// Hamming peaks at 1.0 in the middle.
    #[test]
    fn hamming_peaks_at_one() {
        let w = hamming(401);
        assert!((w[200] - 1.0).abs() < 1e-5, "{}", w[200]);
    }

    /// Hann endpoints are exactly 0.0.
    #[test]
    fn hann_endpoints_are_zero() {
        let w = hann(401);
        assert!(w[0].abs() < 1e-5, "{}", w[0]);
        assert!(w[400].abs() < 1e-5, "{}", w[400]);
    }

    /// Hann peaks at 1.0 in the middle.
    #[test]
    fn hann_peaks_at_one() {
        let w = hann(401);
        assert!((w[200] - 1.0).abs() < 1e-5, "{}", w[200]);
    }

    /// Hamming and Hann are symmetric.
    #[test]
    fn windows_are_symmetric() {
        for n in [16, 64, 256, 1024] {
            let h = hamming(n);
            let g = hann(n);
            for i in 0..n {
                assert!(
                    (h[i] - h[n - 1 - i]).abs() < 1e-5,
                    "hamming asymmetric at i={i}, n={n}",
                );
                assert!(
                    (g[i] - g[n - 1 - i]).abs() < 1e-5,
                    "hann asymmetric at i={i}, n={n}",
                );
            }
        }
    }

    /// Empty / single windows are well-defined.
    #[test]
    fn edge_cases() {
        assert!(hamming(0).is_empty());
        assert!(hann(0).is_empty());
        assert_eq!(hamming(1), vec![1.0]);
        assert_eq!(hann(1), vec![1.0]);
    }

    /// Apply in-place matches windowed copy.
    #[test]
    fn apply_matches_windowed() {
        let frame = vec![1.0_f32; 64];
        let w = hamming(64);
        let copy = windowed(&frame, &w);
        let mut in_place = frame.clone();
        apply(&mut in_place, &w);
        assert_eq!(copy, in_place);
        // And actually attenuates the endpoints.
        assert!(in_place[0] < 0.1);
        assert!(in_place[32] > 0.9);
    }

    /// Apply panics on length mismatch (documented contract).
    #[test]
    #[should_panic(expected = "length must match")]
    fn apply_panics_on_length_mismatch() {
        let mut frame = vec![0.0_f32; 32];
        let w = hamming(64);
        apply(&mut frame, &w);
    }
}
