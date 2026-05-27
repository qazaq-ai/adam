// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! WAV file read / write via `hound`.
//!
//! Reads any WAV that `hound` understands (8 / 16 / 24 / 32-bit
//! integer, 32-bit float; mono / stereo / N-channel) and
//! normalises to `f32 [-1.0, 1.0]` in [`PcmSamples`]. Writes
//! 16-bit-integer PCM by default — the format MFA and most
//! ASR / TTS toolchains expect.

use crate::{AudioError, PcmSamples};
use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use std::path::Path;

/// Read a WAV file into a [`PcmSamples`] buffer.
///
/// The bit-depth and sample format are normalised:
///   - 16-bit PCM → `f32` divided by `32768.0`.
///   - 24-bit PCM → `f32` divided by `8_388_608.0`.
///   - 32-bit PCM → `f32` divided by `2_147_483_648.0`.
///   - 32-bit float → passed through.
///   - Other integer widths → handled via `hound`'s default
///     `Sample::as_i32` conversion, then normalised.
pub fn read_wav(path: impl AsRef<Path>) -> Result<PcmSamples, AudioError> {
    let mut reader = WavReader::open(path)?;
    let spec = reader.spec();

    let data: Vec<f32> = match spec.sample_format {
        SampleFormat::Float => reader.samples::<f32>().collect::<Result<Vec<_>, _>>()?,
        SampleFormat::Int => {
            let raw: Vec<i32> = reader.samples::<i32>().collect::<Result<Vec<_>, _>>()?;
            let scale = match spec.bits_per_sample {
                8 => 128.0_f32,
                16 => 32_768.0_f32,
                24 => 8_388_608.0_f32,
                32 => 2_147_483_648.0_f32,
                other => {
                    return Err(AudioError::ConfigMismatch {
                        requested: "8/16/24/32-bit integer".into(),
                        actual: format!("{other}-bit integer"),
                    });
                }
            };
            raw.into_iter().map(|s| s as f32 / scale).collect()
        }
    };

    Ok(PcmSamples {
        sample_rate: spec.sample_rate,
        channels: spec.channels,
        data,
    })
}

/// Write a [`PcmSamples`] buffer to a 16-bit-integer PCM WAV
/// file. Float samples are clamped to `[-1.0, 1.0]` and scaled
/// to `i16`.
pub fn write_wav(path: impl AsRef<Path>, samples: &PcmSamples) -> Result<(), AudioError> {
    let spec = WavSpec {
        channels: samples.channels,
        sample_rate: samples.sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec)?;
    for &s in &samples.data {
        let clipped = s.clamp(-1.0, 1.0);
        let i = (clipped * 32_767.0) as i16;
        writer.write_sample(i)?;
    }
    writer.finalize()?;
    Ok(())
}

/// Write [`PcmSamples`] as a 32-bit-float WAV file (lossless).
/// Use when downstream consumers need the full f32 range
/// (e.g. forced-aligner input).
pub fn write_wav_f32(path: impl AsRef<Path>, samples: &PcmSamples) -> Result<(), AudioError> {
    let spec = WavSpec {
        channels: samples.channels,
        sample_rate: samples.sample_rate,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let mut writer = WavWriter::create(path, spec)?;
    for &s in &samples.data {
        writer.write_sample(s)?;
    }
    writer.finalize()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// Round-trip: write f32 buffer to 16-bit WAV, read back,
    /// values are within 16-bit quantisation tolerance.
    #[test]
    fn round_trip_int16() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.wav");
        let original =
            PcmSamples::from_mono(16_000, vec![0.0, 0.5, -0.5, 0.999, -0.999, 0.25, -0.25]);
        write_wav(&path, &original).unwrap();
        let back = read_wav(&path).unwrap();
        assert_eq!(back.sample_rate, 16_000);
        assert_eq!(back.channels, 1);
        assert_eq!(back.data.len(), original.data.len());
        for (a, b) in original.data.iter().zip(back.data.iter()) {
            // 16-bit quantisation tolerance: ~3e-5.
            assert!((a - b).abs() < 1e-4, "sample mismatch: {a} vs {b}",);
        }
    }

    /// Round-trip: write f32 buffer to 32-bit-float WAV, read
    /// back losslessly.
    #[test]
    fn round_trip_f32_lossless() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_f32.wav");
        let original = PcmSamples::from_mono(48_000, vec![0.0, 0.123_456, -0.987_654, 0.5, -0.5]);
        write_wav_f32(&path, &original).unwrap();
        let back = read_wav(&path).unwrap();
        assert_eq!(back.data, original.data, "f32 round-trip must be exact");
    }

    /// Stereo WAV write + read preserves frame count and channel
    /// count.
    #[test]
    fn round_trip_stereo() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("stereo.wav");
        let stereo = PcmSamples {
            sample_rate: 44_100,
            channels: 2,
            data: vec![0.5, -0.5, 0.25, -0.25, 0.1, -0.1],
        };
        write_wav(&path, &stereo).unwrap();
        let back = read_wav(&path).unwrap();
        assert_eq!(back.channels, 2);
        assert_eq!(back.frame_count(), 3);
        assert_eq!(back.sample_rate, 44_100);
    }

    /// Reader rejects a missing file.
    #[test]
    fn missing_file_errors() {
        let r = read_wav("/tmp/does-not-exist-adam-audio-test.wav");
        assert!(r.is_err());
    }
}
