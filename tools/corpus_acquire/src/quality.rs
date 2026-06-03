// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Acoustic quality gate for ingested audio.
//!
//! User directive (2026-05-28):
//!
//! > «большинство аудио файлов скаченных тобой бракованные. ...
//! >  необходимо проверить все эти файлы на соответствие и
//! >  пригодность для обучения. В последующем, при скачивании
//! >  сразу проверять на качество аудио файлов, чтобы потом не
//! >  искать причину неудачь.»
//!
//! This module turns "is this clip usable for training?" into a
//! single deterministic check. It runs at two points in the
//! pipeline:
//!
//! 1. **Ingest** (`pull_one`, `process_fleurs_entry`): a clip
//!    that fails the gate is **rejected before** it ever reaches
//!    the manifest or the disk-resident WAV. No bad data enters
//!    the bank by construction.
//! 2. **Audit** (`corpus_acquire audit`): the same check is
//!    re-applied to every existing manifest entry so the user
//!    can prune historical bad downloads in one pass.

use adam_audio::PcmSamples;

/// Audio-quality verdict. `Pass` means the clip is acceptable
/// for training; every other variant carries the specific
/// failure mode so we can report it in audit and ingest logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QualityVerdict {
    Pass,
    /// Decoded buffer has no samples.
    Empty,
    /// Total duration below `min_duration_s` (typical FLEURS
    /// utterance ≥ 4 s; a < 0.3 s clip is almost certainly a
    /// truncated download).
    TooShort,
    /// Mean signal energy too low — the recording is silent or
    /// near-silent.
    NearSilent,
    /// Peak amplitude saturates the ±1.0 f32 range across many
    /// frames — the clip is clipped or corrupted.
    Clipped,
    /// Sample buffer contains a NaN or ±∞ — decoder bug or
    /// corrupt source.
    NonFinite,
}

impl std::fmt::Display for QualityVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pass => write!(f, "pass"),
            Self::Empty => write!(f, "empty"),
            Self::TooShort => write!(f, "too-short"),
            Self::NearSilent => write!(f, "near-silent"),
            Self::Clipped => write!(f, "clipped"),
            Self::NonFinite => write!(f, "non-finite samples"),
        }
    }
}

/// Quality-gate thresholds.
#[derive(Debug, Clone, Copy)]
pub struct QualityConfig {
    /// Minimum acceptable duration in seconds. Anything shorter
    /// is almost certainly a truncated / failed download.
    pub min_duration_s: f32,
    /// Minimum RMS over the whole clip. The unit is the f32
    /// sample magnitude (range [-1, 1]); typical voiced speech
    /// sits around 0.05–0.3. We accept anything ≥ this floor.
    pub min_rms: f32,
    /// Fraction of frames at full-scale (|x| ≥ `clip_amplitude`)
    /// above which we call the clip clipped/corrupted.
    pub max_clipped_fraction: f32,
    /// Magnitude considered "at full scale" for the clipping
    /// fraction check.
    pub clip_amplitude: f32,
}

impl Default for QualityConfig {
    fn default() -> Self {
        Self {
            min_duration_s: 0.30,
            // 0.0015 ≈ 30 dB below typical speech. Tight enough
            // to flag dead-silent FLEURS clips but loose enough
            // to keep low-volume but still intelligible
            // recordings.
            min_rms: 0.0015,
            // > 1% of frames at peak = real clipping / corruption.
            max_clipped_fraction: 0.01,
            clip_amplitude: 0.999,
        }
    }
}

/// Compute basic acoustic statistics over a PCM buffer.
#[derive(Debug, Clone, Copy)]
pub struct AudioStats {
    pub duration_s: f32,
    pub rms: f32,
    pub peak: f32,
    pub clipped_fraction: f32,
    pub has_non_finite: bool,
}

pub fn stats(pcm: &PcmSamples) -> AudioStats {
    let n = pcm.data.len();
    if n == 0 {
        return AudioStats {
            duration_s: 0.0,
            rms: 0.0,
            peak: 0.0,
            clipped_fraction: 0.0,
            has_non_finite: false,
        };
    }
    let sample_rate = pcm.sample_rate.max(1) as f32;
    // Multi-channel PCM is interleaved in PcmSamples — divide by
    // channels to get the per-channel frame count.
    let frame_count = (n / pcm.channels.max(1) as usize) as f32;
    let duration_s = frame_count / sample_rate;

    let mut sum_sq = 0.0_f32;
    let mut peak = 0.0_f32;
    let mut clipped = 0_usize;
    let mut has_non_finite = false;
    let clip_threshold = QualityConfig::default().clip_amplitude;
    for &x in &pcm.data {
        if !x.is_finite() {
            has_non_finite = true;
            continue;
        }
        let a = x.abs();
        if a > peak {
            peak = a;
        }
        if a >= clip_threshold {
            clipped += 1;
        }
        sum_sq += x * x;
    }
    let rms = (sum_sq / n as f32).sqrt();
    let clipped_fraction = clipped as f32 / n as f32;
    AudioStats {
        duration_s,
        rms,
        peak,
        clipped_fraction,
        has_non_finite,
    }
}

/// Apply the quality gate. Returns the verdict plus the
/// computed stats (useful for audit reporting).
pub fn check(pcm: &PcmSamples, cfg: &QualityConfig) -> (QualityVerdict, AudioStats) {
    let st = stats(pcm);
    if pcm.data.is_empty() {
        return (QualityVerdict::Empty, st);
    }
    if st.has_non_finite {
        return (QualityVerdict::NonFinite, st);
    }
    if st.duration_s < cfg.min_duration_s {
        return (QualityVerdict::TooShort, st);
    }
    if st.rms < cfg.min_rms {
        return (QualityVerdict::NearSilent, st);
    }
    if st.clipped_fraction > cfg.max_clipped_fraction {
        return (QualityVerdict::Clipped, st);
    }
    (QualityVerdict::Pass, st)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pcm_of(data: Vec<f32>, sample_rate: u32) -> PcmSamples {
        PcmSamples {
            sample_rate,
            channels: 1,
            data,
        }
    }

    #[test]
    fn empty_audio_flagged() {
        let p = pcm_of(vec![], 16_000);
        assert_eq!(
            check(&p, &QualityConfig::default()).0,
            QualityVerdict::Empty
        );
    }

    #[test]
    fn silent_audio_flagged() {
        // 1 s of pure zeros at 16 kHz.
        let p = pcm_of(vec![0.0_f32; 16_000], 16_000);
        assert_eq!(
            check(&p, &QualityConfig::default()).0,
            QualityVerdict::NearSilent
        );
    }

    #[test]
    fn very_short_audio_flagged() {
        // 100 ms — under the 300 ms floor.
        let p = pcm_of(vec![0.2_f32; 1_600], 16_000);
        assert_eq!(
            check(&p, &QualityConfig::default()).0,
            QualityVerdict::TooShort
        );
    }

    #[test]
    fn clipped_audio_flagged() {
        // 1 s of full-scale samples — clipped.
        let p = pcm_of(vec![1.0_f32; 16_000], 16_000);
        assert_eq!(
            check(&p, &QualityConfig::default()).0,
            QualityVerdict::Clipped
        );
    }

    #[test]
    fn nan_audio_flagged() {
        let mut data = vec![0.1_f32; 16_000];
        data[100] = f32::NAN;
        let p = pcm_of(data, 16_000);
        assert_eq!(
            check(&p, &QualityConfig::default()).0,
            QualityVerdict::NonFinite
        );
    }

    #[test]
    fn typical_speech_passes() {
        // 1 s of a 200 Hz sine at -20 dB (≈ 0.1 amplitude) —
        // typical voiced-speech magnitude, well above the floor.
        let sr = 16_000;
        let data: Vec<f32> = (0..sr)
            .map(|i| {
                let t = i as f32 / sr as f32;
                0.1 * (2.0 * std::f32::consts::PI * 200.0 * t).sin()
            })
            .collect();
        let p = pcm_of(data, sr);
        let (v, s) = check(&p, &QualityConfig::default());
        assert_eq!(v, QualityVerdict::Pass, "stats: {s:?}");
    }
}
