// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Microphone capture via `cpal`, with VAD-driven auto-stop.

use crate::{AudioError, PcmSamples, vad};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Configuration for [`record_until_silence`].
#[derive(Debug, Clone)]
pub struct RecordConfig {
    /// Hard cap on recording time (regardless of VAD state).
    pub max_duration: Duration,
    /// Stop recording after this much continuous silence has
    /// been observed *after* speech has begun. If the user never
    /// speaks, the recorder waits up to `max_duration`.
    pub silence_timeout: Duration,
    /// RMS amplitude threshold for [`vad::is_silence`]
    /// classification. `[0.0, 1.0]`.
    pub silence_threshold: f32,
}

impl Default for RecordConfig {
    fn default() -> Self {
        Self {
            max_duration: Duration::from_secs(30),
            silence_timeout: Duration::from_millis(1500),
            silence_threshold: vad::DEFAULT_SILENCE_THRESHOLD,
        }
    }
}

/// Record from the default input device until silence is
/// detected (or `max_duration` is reached).
///
/// The recording starts immediately (no key-press hold-to-talk
/// gate at this layer — that belongs to the caller). Returns
/// the captured audio downmixed to mono at the device's native
/// sample rate. Resampling to 16 kHz (Phase 6 norm for STT)
/// is the caller's responsibility for now.
pub fn record_until_silence(config: RecordConfig) -> Result<PcmSamples, AudioError> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or(AudioError::NoDevice("input"))?;
    let supported = device.default_input_config()?;
    let sample_rate = supported.sample_rate().0;
    let channels = supported.channels();
    let sample_format = supported.sample_format();
    let stream_config: StreamConfig = supported.into();

    let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let err_buf: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    let stream = build_input_stream(&device, &stream_config, sample_format, &buffer, &err_buf)?;
    stream.play()?;

    let frame_samples = (sample_rate as f32 * 0.030) as usize * channels as usize;
    let poll_interval = Duration::from_millis(30);
    let started = Instant::now();
    let mut speech_seen = false;
    let mut last_speech_at = Instant::now();

    loop {
        std::thread::sleep(poll_interval);

        // Surface any error reported from the cpal callback.
        if let Some(e) = err_buf.lock().unwrap().take() {
            drop(stream);
            return Err(AudioError::Io(std::io::Error::other(e)));
        }

        // Hard cap: stop unconditionally after `max_duration`.
        if started.elapsed() >= config.max_duration {
            break;
        }

        // VAD over the last frame.
        let buf_locked = buffer.lock().unwrap();
        let len = buf_locked.len();
        if len >= frame_samples {
            let frame = &buf_locked[len - frame_samples..];
            let frame_is_silence = vad::is_silence(frame, config.silence_threshold);
            if !frame_is_silence {
                speech_seen = true;
                last_speech_at = Instant::now();
            } else if speech_seen && last_speech_at.elapsed() >= config.silence_timeout {
                drop(buf_locked);
                break;
            }
        }
    }

    drop(stream);
    let interleaved = std::mem::take(&mut *buffer.lock().unwrap());
    finalize_buffer(interleaved, sample_rate, channels)
}

/// Record exactly `duration` seconds with no VAD. Useful for
/// scripted tests and recording known-length audio for the
/// phoneme bank (Phase 2a corpus acquisition).
pub fn record_fixed_duration(duration: Duration) -> Result<PcmSamples, AudioError> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or(AudioError::NoDevice("input"))?;
    let supported = device.default_input_config()?;
    let sample_rate = supported.sample_rate().0;
    let channels = supported.channels();
    let sample_format = supported.sample_format();
    let stream_config: StreamConfig = supported.into();

    let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let err_buf: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let stream = build_input_stream(&device, &stream_config, sample_format, &buffer, &err_buf)?;
    stream.play()?;
    std::thread::sleep(duration);
    drop(stream);

    if let Some(e) = err_buf.lock().unwrap().take() {
        return Err(AudioError::Io(std::io::Error::other(e)));
    }

    let interleaved = std::mem::take(&mut *buffer.lock().unwrap());
    finalize_buffer(interleaved, sample_rate, channels)
}

// ─── helpers ──────────────────────────────────────────────────────────

fn build_input_stream(
    device: &cpal::Device,
    config: &StreamConfig,
    sample_format: SampleFormat,
    buffer: &Arc<Mutex<Vec<f32>>>,
    err_buf: &Arc<Mutex<Option<String>>>,
) -> Result<cpal::Stream, AudioError> {
    let err_cb = {
        let err = Arc::clone(err_buf);
        move |e: cpal::StreamError| {
            *err.lock().unwrap() = Some(format!("{e}"));
        }
    };

    let stream = match sample_format {
        SampleFormat::F32 => {
            let buf = Arc::clone(buffer);
            device.build_input_stream(
                config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    buf.lock().unwrap().extend_from_slice(data);
                },
                err_cb,
                None,
            )?
        }
        SampleFormat::I16 => {
            let buf = Arc::clone(buffer);
            device.build_input_stream(
                config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    let mut b = buf.lock().unwrap();
                    b.extend(data.iter().map(|&s| s as f32 / 32_768.0));
                },
                err_cb,
                None,
            )?
        }
        SampleFormat::U16 => {
            let buf = Arc::clone(buffer);
            device.build_input_stream(
                config,
                move |data: &[u16], _: &cpal::InputCallbackInfo| {
                    let mut b = buf.lock().unwrap();
                    b.extend(data.iter().map(|&s| (s as f32 - 32_768.0) / 32_768.0));
                },
                err_cb,
                None,
            )?
        }
        other => {
            return Err(AudioError::ConfigMismatch {
                requested: "f32 / i16 / u16".into(),
                actual: format!("{other:?}"),
            });
        }
    };
    Ok(stream)
}

fn finalize_buffer(
    interleaved: Vec<f32>,
    sample_rate: u32,
    channels: u16,
) -> Result<PcmSamples, AudioError> {
    let multi = PcmSamples {
        sample_rate,
        channels,
        data: interleaved,
    };
    Ok(multi.to_mono())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Default config is reasonable.
    #[test]
    fn default_config_values() {
        let c = RecordConfig::default();
        assert!(c.max_duration >= Duration::from_secs(10));
        assert!(c.silence_timeout >= Duration::from_millis(500));
        assert!(c.silence_threshold > 0.0 && c.silence_threshold < 0.5);
    }

    /// `finalize_buffer` downmixes stereo interleaved to mono.
    #[test]
    fn finalize_downmixes_to_mono() {
        let interleaved = vec![1.0, -1.0, 0.5, 0.5];
        let p = finalize_buffer(interleaved, 16_000, 2).unwrap();
        assert_eq!(p.channels, 1);
        assert_eq!(p.data, vec![0.0, 0.5]);
    }

    /// `finalize_buffer` keeps mono untouched.
    #[test]
    fn finalize_preserves_mono() {
        let interleaved = vec![0.1, 0.2, 0.3];
        let p = finalize_buffer(interleaved.clone(), 16_000, 1).unwrap();
        assert_eq!(p.channels, 1);
        assert_eq!(p.data, interleaved);
    }
}
