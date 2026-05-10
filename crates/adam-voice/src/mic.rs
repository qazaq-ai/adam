//! **v5.14.0 (V0).** Push-to-talk microphone capture using `cpal`.
//!
//! Goal: capture mono audio at 16 kHz (whisper.cpp's preferred input
//! shape) into a `Vec<i16>` buffer that the caller writes to WAV for
//! the STT shell-out. v5.14.0 stops on Enter (or a configurable
//! timeout); v5.15.0 adds VAD-based auto-stop.
//!
//! ## Sample-rate handling
//!
//! cpal exposes whatever the OS audio device offers — typically
//! 44.1 / 48 / 96 kHz on consumer mics. We negotiate the closest
//! supported config and resample to 16 kHz on the fly via simple
//! linear interpolation (NOT a high-fidelity resampler — that's
//! overkill for STT input which Whisper itself reprocesses through
//! mel filterbanks). The boundary contract is "give the STT engine
//! 16 kHz mono i16 samples", and that's what we deliver.
//!
//! ## Threading model
//!
//! cpal hands us a real-time callback on its own thread. We push
//! decoded samples through a [`std::sync::mpsc`] channel; the main
//! thread drains the channel into the output buffer. No locks on the
//! audio path.

use std::path::Path;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};

use crate::error::{Result, VoiceError};

/// Target sample rate for whisper.cpp input. Whisper internally
/// expects 16 kHz mono; we resample to this regardless of what the
/// mic device produces.
pub const WHISPER_SAMPLE_RATE: u32 = 16_000;

/// User-tunable mic capture parameters.
#[derive(Debug, Clone)]
pub struct MicConfig {
    /// Hard upper bound on a single recording in seconds. v5.14.0
    /// uses this as the auto-stop trigger (push-to-talk loops have
    /// no VAD yet); v5.15.0 will make it the safety cap with VAD as
    /// the primary stop signal.
    pub max_duration: Duration,
}

impl Default for MicConfig {
    fn default() -> Self {
        Self {
            max_duration: Duration::from_secs(30),
        }
    }
}

/// Push-to-talk capture session. Holds the cpal stream + the
/// sample-collecting receiver until the user calls [`stop`].
///
/// Drop on the returned struct is silent — no panic if the audio
/// device hangs up mid-stream.
pub struct MicCapture {
    config: MicConfig,
    stream: cpal::Stream,
    rx: Receiver<Vec<i16>>,
    started_at: Instant,
    device_sample_rate: u32,
    device_channels: u16,
}

impl MicCapture {
    /// Open the default input device, negotiate a config, and start
    /// the capture stream. Returns a [`MicCapture`] handle that
    /// continues capturing until [`stop`] is called or the configured
    /// `max_duration` elapses.
    pub fn start(config: MicConfig) -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or(VoiceError::NoInputDevice)?;
        let supported = device
            .default_input_config()
            .map_err(|e| VoiceError::StreamBuild(e.to_string()))?;
        let sample_format = supported.sample_format();
        let cfg: StreamConfig = supported.into();
        let device_sample_rate = cfg.sample_rate.0;
        let device_channels = cfg.channels;

        let (tx, rx): (Sender<Vec<i16>>, Receiver<Vec<i16>>) = channel();

        let err_tx = tx.clone();
        let stream = match sample_format {
            SampleFormat::I16 => device
                .build_input_stream(
                    &cfg,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        let _ = tx.send(data.to_vec());
                    },
                    move |e| {
                        eprintln!("[adam-voice mic] stream error: {e}");
                        let _ = err_tx.send(Vec::new());
                    },
                    None,
                )
                .map_err(|e| VoiceError::StreamBuild(e.to_string()))?,
            SampleFormat::F32 => device
                .build_input_stream(
                    &cfg,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        let converted: Vec<i16> = data
                            .iter()
                            .map(|&s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                            .collect();
                        let _ = tx.send(converted);
                    },
                    move |e| {
                        eprintln!("[adam-voice mic] stream error: {e}");
                        let _ = err_tx.send(Vec::new());
                    },
                    None,
                )
                .map_err(|e| VoiceError::StreamBuild(e.to_string()))?,
            other => {
                return Err(VoiceError::StreamBuild(format!(
                    "unsupported sample format {other:?}"
                )));
            }
        };
        stream
            .play()
            .map_err(|e| VoiceError::StreamPlay(e.to_string()))?;
        Ok(Self {
            config,
            stream,
            rx,
            started_at: Instant::now(),
            device_sample_rate,
            device_channels,
        })
    }

    /// Drain the channel into a flat `Vec<i16>`, downmixing
    /// multi-channel input to mono and resampling to 16 kHz. Stops
    /// when no more samples arrive within a 100 ms quiet window OR
    /// the configured `max_duration` elapsed.
    pub fn stop(self) -> Result<Vec<i16>> {
        let mut raw: Vec<i16> = Vec::new();
        let drain_window = Duration::from_millis(100);
        while let Ok(chunk) = self.rx.recv_timeout(drain_window) {
            raw.extend(chunk);
            if self.started_at.elapsed() >= self.config.max_duration {
                break;
            }
        }
        // Stream drops here implicitly via `self.stream`.
        drop(self.stream);
        let mono = downmix_to_mono(&raw, self.device_channels);
        let resampled = resample_linear(&mono, self.device_sample_rate, WHISPER_SAMPLE_RATE);
        Ok(resampled)
    }
}

/// Average each frame across channels — produces a mono `Vec<i16>`.
/// Intentionally simple: we're feeding STT, not mastering a record.
pub(crate) fn downmix_to_mono(samples: &[i16], channels: u16) -> Vec<i16> {
    if channels <= 1 {
        return samples.to_vec();
    }
    let n = channels as usize;
    let mut out = Vec::with_capacity(samples.len() / n);
    for frame in samples.chunks_exact(n) {
        let sum: i32 = frame.iter().map(|&s| s as i32).sum();
        out.push((sum / n as i32) as i16);
    }
    out
}

/// Linear-interpolation resampler. Adequate for STT input — Whisper
/// re-projects through a mel filterbank anyway, so any aliasing we
/// introduce here is dwarfed by the engine's own band-limiting.
pub(crate) fn resample_linear(samples: &[i16], src_rate: u32, dst_rate: u32) -> Vec<i16> {
    if src_rate == dst_rate || samples.is_empty() {
        return samples.to_vec();
    }
    let ratio = src_rate as f64 / dst_rate as f64;
    let dst_len = (samples.len() as f64 / ratio) as usize;
    let mut out = Vec::with_capacity(dst_len);
    for i in 0..dst_len {
        let pos = i as f64 * ratio;
        let lo = pos.floor() as usize;
        let hi = (lo + 1).min(samples.len() - 1);
        let frac = pos - lo as f64;
        let s = (samples[lo] as f64) * (1.0 - frac) + (samples[hi] as f64) * frac;
        out.push(s.round() as i16);
    }
    out
}

/// Write a mono 16 kHz `Vec<i16>` to a 16-bit PCM WAV file. The
/// resulting file is what `WhisperCli::transcribe` consumes.
pub fn write_wav(samples: &[i16], path: &Path) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: WHISPER_SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).map_err(|e| match e {
        hound::Error::IoError(io) => VoiceError::WavWrite {
            path: path.to_path_buf(),
            source: io,
        },
        other => VoiceError::Hound(other),
    })?;
    for &s in samples {
        writer.write_sample(s)?;
    }
    writer.finalize()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downmix_stereo_averages_channels() {
        let stereo: Vec<i16> = vec![100, 200, 300, 400, 500, 600];
        let mono = downmix_to_mono(&stereo, 2);
        assert_eq!(mono, vec![150, 350, 550]);
    }

    #[test]
    fn downmix_mono_is_passthrough() {
        let mono_in: Vec<i16> = vec![1, 2, 3];
        assert_eq!(downmix_to_mono(&mono_in, 1), mono_in);
    }

    #[test]
    fn resample_identity_when_rates_match() {
        let s = vec![1i16, 2, 3, 4];
        assert_eq!(resample_linear(&s, 16_000, 16_000), s);
    }

    #[test]
    fn resample_downsamples_to_target_length() {
        // 8 samples at 32 kHz → ~4 samples at 16 kHz.
        let s = vec![0i16, 100, 200, 300, 400, 500, 600, 700];
        let out = resample_linear(&s, 32_000, 16_000);
        assert_eq!(out.len(), 4);
        // First sample preserved (linear interp at pos=0 is exact).
        assert_eq!(out[0], 0);
    }

    #[test]
    fn resample_handles_empty_input() {
        assert!(resample_linear(&[], 48_000, 16_000).is_empty());
    }
}
