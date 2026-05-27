// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Short-Time Fourier Transform (STFT) — a spectrogram of an
//! audio signal computed by sliding a window across the
//! samples and FFT-ing each windowed slice.
//!
//! This is the **time–frequency representation** every higher
//! DSP layer (MFCC, phoneme matching, voice activity) builds
//! on. Output: a `frames × (fft_size / 2 + 1)` matrix of
//! magnitude or power spectra.
//!
//! Standard parameters for 16 kHz speech (matching Kaldi /
//! librosa defaults):
//! - Window length: 25 ms → 400 samples
//! - Hop length: 10 ms → 160 samples (62.5 % overlap)
//! - FFT size: 512 (next power of 2 of 400, zero-padded)
//! - Window: Hamming

use crate::fft::{AdamFftPlanner, power_with_plan};
use crate::window;

/// STFT configuration. Defaults match canonical speech-pipeline
/// values (Kaldi / librosa) at 16 kHz.
#[derive(Debug, Clone)]
pub struct StftConfig {
    /// Number of samples per window (e.g. 400 = 25 ms @ 16 kHz).
    pub window_length: usize,
    /// Number of samples between consecutive frames
    /// (e.g. 160 = 10 ms @ 16 kHz, 62.5 % overlap).
    pub hop_length: usize,
    /// FFT size (must be ≥ `window_length`; usually next power of 2).
    pub fft_size: usize,
    /// Window function (Hamming / Hann).
    pub window_type: WindowType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowType {
    Hamming,
    Hann,
}

impl StftConfig {
    /// Canonical defaults for 16 kHz speech: 25 ms window,
    /// 10 ms hop, 512-pt FFT, Hamming.
    pub fn speech_16khz() -> Self {
        Self {
            window_length: 400,
            hop_length: 160,
            fft_size: 512,
            window_type: WindowType::Hamming,
        }
    }

    /// Window vector matching this config.
    pub fn build_window(&self) -> Vec<f32> {
        match self.window_type {
            WindowType::Hamming => window::hamming(self.window_length),
            WindowType::Hann => window::hann(self.window_length),
        }
    }
}

/// A spectrogram: `frames × (fft_size / 2 + 1)` power values.
#[derive(Debug, Clone, PartialEq)]
pub struct Spectrogram {
    /// Power spectrum per frame; outer length = frames,
    /// inner length = `fft_size / 2 + 1`.
    pub frames: Vec<Vec<f32>>,
    /// Sample rate the original audio was at (kept for
    /// downstream frequency-axis conversions).
    pub sample_rate: u32,
    /// FFT size used (the inner-vector length is `fft_size/2 + 1`).
    pub fft_size: usize,
    /// Hop length used (samples between consecutive frame starts).
    pub hop_length: usize,
}

impl Spectrogram {
    /// Number of time frames.
    pub fn num_frames(&self) -> usize {
        self.frames.len()
    }

    /// Number of frequency bins (`fft_size / 2 + 1`).
    pub fn num_bins(&self) -> usize {
        if self.frames.is_empty() {
            self.fft_size / 2 + 1
        } else {
            self.frames[0].len()
        }
    }
}

/// Compute the **power-spectrum STFT** of a signal.
///
/// The signal is split into overlapping `window_length`-sample
/// frames at `hop_length` intervals. Each frame is windowed,
/// zero-padded to `fft_size`, FFT'd, and the power spectrum
/// (magnitude²) of the positive-frequency bins is stored.
///
/// Frames that fall off the end of the signal are dropped (no
/// padding) — typical for speech where edge frames are not
/// linguistically informative.
pub fn stft(samples: &[f32], sample_rate: u32, config: &StftConfig) -> Spectrogram {
    assert!(
        config.fft_size >= config.window_length,
        "fft_size ({}) must be ≥ window_length ({})",
        config.fft_size,
        config.window_length,
    );

    let window = config.build_window();
    let mut planner = AdamFftPlanner::new();
    let plan = planner.plan_forward(config.fft_size);

    let mut frames: Vec<Vec<f32>> = Vec::new();
    let mut offset = 0_usize;
    while offset + config.window_length <= samples.len() {
        let mut frame: Vec<f32> = samples[offset..offset + config.window_length]
            .iter()
            .zip(window.iter())
            .map(|(&s, &w)| s * w)
            .collect();
        // Zero-pad to fft_size if needed.
        frame.resize(config.fft_size, 0.0);
        let power = power_with_plan(&frame, plan.as_ref());
        frames.push(power);
        offset += config.hop_length;
    }

    Spectrogram {
        frames,
        sample_rate,
        fft_size: config.fft_size,
        hop_length: config.hop_length,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    /// STFT of a 1-second sine produces ~100 frames at default
    /// 10 ms hop.
    #[test]
    fn stft_frame_count_matches_hop() {
        let sample_rate = 16_000;
        let samples: Vec<f32> = (0..sample_rate)
            .map(|i| (2.0 * PI * 220.0 * i as f32 / sample_rate as f32).sin())
            .collect();
        let s = stft(&samples, sample_rate, &StftConfig::speech_16khz());
        // n_frames = (n_samples - window_length) / hop_length + 1
        // ≈ (16_000 - 400) / 160 + 1 = 98.5 → 98 with floor + 1
        let expected = (samples.len() - 400) / 160 + 1;
        assert_eq!(s.num_frames(), expected, "got {}", s.num_frames());
        assert_eq!(s.num_bins(), 257); // 512/2 + 1
    }

    /// **Realistic voice**: STFT of a harmonic voice shows
    /// stable energy at F0 and its overtones across every
    /// frame.
    #[test]
    fn stft_voice_shows_stable_harmonic_lines() {
        use crate::fft::frequency_to_bin;
        let sample_rate = 16_000;
        let f0 = 150.0_f32;
        let samples: Vec<f32> = (0..sample_rate)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                let mut s = 0.0_f32;
                for h in 1..=4 {
                    s += (2.0 * PI * f0 * h as f32 * t).sin() / (h as f32);
                }
                s
            })
            .collect();
        let cfg = StftConfig::speech_16khz();
        let spec = stft(&samples, sample_rate, &cfg);

        // Sample a middle frame; F0 and 2F0 bins must dominate
        // local neighbourhood.
        let mid = spec.num_frames() / 2;
        let bin_f0 = frequency_to_bin(f0, cfg.fft_size, sample_rate);
        let bin_2f0 = frequency_to_bin(2.0 * f0, cfg.fft_size, sample_rate);
        let bin_mid = frequency_to_bin(f0 * 1.5, cfg.fft_size, sample_rate); // between harmonics

        assert!(
            spec.frames[mid][bin_f0] > spec.frames[mid][bin_mid],
            "F0 bin not dominant over mid"
        );
        assert!(
            spec.frames[mid][bin_2f0] > spec.frames[mid][bin_mid],
            "2F0 bin not dominant over mid"
        );
    }

    /// Spectrogram of silence is near-zero everywhere.
    #[test]
    fn silence_spectrogram_is_zero() {
        let samples = vec![0.0_f32; 16_000];
        let spec = stft(&samples, 16_000, &StftConfig::speech_16khz());
        let total: f32 = spec.frames.iter().flat_map(|f| f.iter()).sum();
        assert!(total < 1e-6, "silence spectrogram has energy {total}");
    }

    /// Short input → empty or near-empty spectrogram, no panic.
    #[test]
    fn short_input_no_panic() {
        let samples = vec![0.0_f32; 100];
        let spec = stft(&samples, 16_000, &StftConfig::speech_16khz());
        assert_eq!(spec.num_frames(), 0);
    }

    /// `speech_16khz()` defaults match documented values.
    #[test]
    fn speech_defaults_correct() {
        let cfg = StftConfig::speech_16khz();
        assert_eq!(cfg.window_length, 400);
        assert_eq!(cfg.hop_length, 160);
        assert_eq!(cfg.fft_size, 512);
        assert_eq!(cfg.window_type, WindowType::Hamming);
    }

    /// **F0 progression**: a 1-second chirp from 200 → 400 Hz
    /// should show the dominant bin migrating upward across
    /// the spectrogram.
    #[test]
    fn chirp_shows_migrating_peak() {
        use crate::fft::frequency_to_bin;
        let sample_rate = 16_000;
        let n = sample_rate as usize;
        let samples: Vec<f32> = (0..n)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                // Linear chirp from 200 Hz to 400 Hz over 1 s.
                let f = 200.0 + 200.0 * t;
                let phase = 2.0 * PI * (200.0 * t + 100.0 * t * t);
                phase.sin() * 0.5 + 0.0 * f // touch f to keep var alive
            })
            .collect();
        let cfg = StftConfig::speech_16khz();
        let spec = stft(&samples, sample_rate, &cfg);
        let early = spec
            .frames
            .iter()
            .take(10)
            .map(|f| {
                f.iter()
                    .enumerate()
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                    .unwrap()
                    .0
            })
            .sum::<usize>()
            / 10;
        let late_frames = spec.num_frames().saturating_sub(10);
        let late = spec
            .frames
            .iter()
            .skip(late_frames)
            .map(|f| {
                f.iter()
                    .enumerate()
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                    .unwrap()
                    .0
            })
            .sum::<usize>()
            / 10;
        // Late-frame peak bin should be higher than early-frame
        // peak bin (chirp went up).
        assert!(
            late > early,
            "chirp peak did not migrate: early bin {early}, late bin {late}",
        );
        // And both should be in the 200-400 Hz range (bins ~6-13).
        let bin_200 = frequency_to_bin(200.0, cfg.fft_size, sample_rate);
        let bin_400 = frequency_to_bin(400.0, cfg.fft_size, sample_rate);
        assert!(
            (bin_200..=bin_400 + 2).contains(&early),
            "early bin {early} outside chirp range",
        );
        assert!(
            (bin_200..=bin_400 + 2).contains(&late),
            "late bin {late} outside chirp range",
        );
    }
}
