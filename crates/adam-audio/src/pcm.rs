// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Linear-PCM buffer with sample-rate / channel-count metadata.

use crate::AudioError;

/// Mono or stereo PCM buffer in `f32` samples normalised to
/// `[-1.0, 1.0]`. Channels are interleaved when `channels > 1`.
///
/// This is the **canonical in-memory audio representation** for
/// the v6.3 stack. Every audio source (WAV reader, microphone
/// recorder, synthesiser) produces a `PcmSamples`; every audio
/// sink (WAV writer, speaker playback, MFCC extractor) consumes
/// one. The single representation keeps callers from having to
/// track the f32 / i16 / i24 layout differences between the
/// underlying libraries.
#[derive(Debug, Clone, PartialEq)]
pub struct PcmSamples {
    /// Samples per second (e.g. 16_000, 22_050, 44_100, 48_000).
    pub sample_rate: u32,
    /// Channel count (1 = mono, 2 = stereo). Samples in `data`
    /// are interleaved when `channels > 1`.
    pub channels: u16,
    /// Interleaved samples in `[-1.0, 1.0]`. Length is
    /// `frames * channels`.
    pub data: Vec<f32>,
}

impl PcmSamples {
    /// New empty mono buffer at the given sample rate.
    pub fn empty(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            channels: 1,
            data: Vec::new(),
        }
    }

    /// New mono buffer from a `Vec<f32>`.
    pub fn from_mono(sample_rate: u32, data: Vec<f32>) -> Self {
        Self {
            sample_rate,
            channels: 1,
            data,
        }
    }

    /// Number of frames (samples per channel).
    pub fn frame_count(&self) -> usize {
        debug_assert!(
            self.channels > 0,
            "PcmSamples with zero channels is invalid"
        );
        self.data.len() / self.channels as usize
    }

    /// Duration in seconds.
    pub fn duration_s(&self) -> f32 {
        self.frame_count() as f32 / self.sample_rate as f32
    }

    /// Returns the mono samples slice. Panics if `channels != 1`.
    /// Use [`to_mono`](Self::to_mono) when channels may vary.
    pub fn as_mono(&self) -> &[f32] {
        assert_eq!(self.channels, 1, "as_mono called on non-mono buffer");
        &self.data
    }

    /// Downmix to mono by averaging interleaved channels.
    /// Returns a new buffer with `channels = 1`.
    pub fn to_mono(&self) -> Self {
        if self.channels == 1 {
            return self.clone();
        }
        let c = self.channels as usize;
        let frames = self.frame_count();
        let mut mono = Vec::with_capacity(frames);
        for frame_i in 0..frames {
            let mut sum = 0.0_f32;
            for ch in 0..c {
                sum += self.data[frame_i * c + ch];
            }
            mono.push(sum / c as f32);
        }
        Self {
            sample_rate: self.sample_rate,
            channels: 1,
            data: mono,
        }
    }

    /// Reject buffers that contain no samples — many consumers
    /// (encoders, the VAD, the MFCC extractor) need at least one
    /// sample to be meaningful.
    pub fn require_non_empty(&self) -> Result<(), AudioError> {
        if self.data.is_empty() {
            Err(AudioError::EmptyBuffer)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_constructor_is_zero_frames() {
        let p = PcmSamples::empty(16_000);
        assert_eq!(p.sample_rate, 16_000);
        assert_eq!(p.channels, 1);
        assert_eq!(p.frame_count(), 0);
        assert_eq!(p.duration_s(), 0.0);
    }

    #[test]
    fn mono_from_vec_has_correct_metadata() {
        let p = PcmSamples::from_mono(48_000, vec![0.0; 480]);
        assert_eq!(p.channels, 1);
        assert_eq!(p.frame_count(), 480);
        assert!((p.duration_s() - 0.01).abs() < 1e-6, "{}", p.duration_s());
    }

    #[test]
    fn to_mono_downmixes_stereo() {
        // Stereo, 4 frames, each channel constant.
        let stereo = PcmSamples {
            sample_rate: 16_000,
            channels: 2,
            data: vec![1.0, -1.0, 0.5, 0.5, 0.0, 0.0, -0.25, 0.75],
        };
        let mono = stereo.to_mono();
        assert_eq!(mono.channels, 1);
        // Per-frame averages: (1-1)/2, (0.5+0.5)/2, 0, (-0.25+0.75)/2.
        assert_eq!(mono.data, vec![0.0, 0.5, 0.0, 0.25]);
        assert_eq!(mono.frame_count(), 4);
    }

    #[test]
    fn to_mono_on_mono_clones() {
        let m = PcmSamples::from_mono(8_000, vec![0.1, 0.2, 0.3]);
        let m2 = m.to_mono();
        assert_eq!(m, m2);
    }

    #[test]
    fn require_non_empty_rejects_empty() {
        let p = PcmSamples::empty(16_000);
        assert!(p.require_non_empty().is_err());
    }

    #[test]
    fn require_non_empty_accepts_filled() {
        let p = PcmSamples::from_mono(16_000, vec![0.0]);
        assert!(p.require_non_empty().is_ok());
    }
}
