// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Speaker playback via `cpal`.

use crate::{AudioError, PcmSamples};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Play a [`PcmSamples`] buffer through the default output device,
/// blocking until playback finishes.
///
/// The function uses the device's default output config and
/// resamples nothing — if `samples.sample_rate` does not match
/// the device's preferred rate, playback speed will be off.
/// Resampling lands in Phase 6.
pub fn play_blocking(samples: &PcmSamples) -> Result<(), AudioError> {
    samples.require_non_empty()?;

    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or(AudioError::NoDevice("output"))?;
    let supported = device.default_output_config()?;
    let sample_format = supported.sample_format();
    let device_channels = supported.channels();
    let stream_config: StreamConfig = supported.into();

    // Distribute the mono / interleaved samples across the output
    // channels. If `samples` is mono and the device wants stereo,
    // duplicate each sample across channels.
    let frames: Vec<Vec<f32>> = build_output_frames(samples, device_channels);

    let position = Arc::new(AtomicUsize::new(0));
    let total = frames.len();
    let frames = Arc::new(frames);

    let err_buf: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    let stream = build_output_stream(
        &device,
        &stream_config,
        sample_format,
        &frames,
        &position,
        &err_buf,
    )?;
    stream.play()?;

    // Block until either an error surfaces or we've consumed every
    // frame. Poll every 10 ms.
    loop {
        if let Some(e) = err_buf.lock().unwrap().take() {
            drop(stream);
            return Err(AudioError::Io(std::io::Error::other(e)));
        }
        if position.load(Ordering::Acquire) >= total {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    // Small tail to let the audio device flush its internal
    // buffer (cpal does not expose a flush API).
    std::thread::sleep(Duration::from_millis(50));
    drop(stream);
    Ok(())
}

fn build_output_frames(samples: &PcmSamples, device_channels: u16) -> Vec<Vec<f32>> {
    let src_channels = samples.channels as usize;
    let dst_channels = device_channels as usize;
    let n_frames = samples.frame_count();
    let mut out = Vec::with_capacity(n_frames);

    for f in 0..n_frames {
        let mut frame = vec![0.0_f32; dst_channels];
        for c in 0..dst_channels {
            // If destination has more channels than source,
            // duplicate the matching source channel (or 0 if
            // source is mono and dest is wider, we duplicate
            // channel 0).
            let src_c = if c < src_channels { c } else { 0 };
            frame[c] = samples.data[f * src_channels + src_c];
        }
        out.push(frame);
    }
    out
}

fn build_output_stream(
    device: &cpal::Device,
    config: &StreamConfig,
    sample_format: SampleFormat,
    frames: &Arc<Vec<Vec<f32>>>,
    position: &Arc<AtomicUsize>,
    err_buf: &Arc<Mutex<Option<String>>>,
) -> Result<cpal::Stream, AudioError> {
    let err_cb = {
        let err = Arc::clone(err_buf);
        move |e: cpal::StreamError| {
            *err.lock().unwrap() = Some(format!("{e}"));
        }
    };

    let channels = config.channels as usize;

    let stream = match sample_format {
        SampleFormat::F32 => {
            let frames = Arc::clone(frames);
            let position = Arc::clone(position);
            device.build_output_stream(
                config,
                move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    write_frames_f32(out, channels, &frames, &position);
                },
                err_cb,
                None,
            )?
        }
        SampleFormat::I16 => {
            let frames = Arc::clone(frames);
            let position = Arc::clone(position);
            device.build_output_stream(
                config,
                move |out: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    write_frames_i16(out, channels, &frames, &position);
                },
                err_cb,
                None,
            )?
        }
        SampleFormat::U16 => {
            let frames = Arc::clone(frames);
            let position = Arc::clone(position);
            device.build_output_stream(
                config,
                move |out: &mut [u16], _: &cpal::OutputCallbackInfo| {
                    write_frames_u16(out, channels, &frames, &position);
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

fn write_frames_f32(out: &mut [f32], channels: usize, frames: &[Vec<f32>], position: &AtomicUsize) {
    for slot in out.chunks_mut(channels) {
        let idx = position.fetch_add(1, Ordering::AcqRel);
        if idx < frames.len() {
            let frame = &frames[idx];
            for (s, &v) in slot.iter_mut().zip(frame.iter()) {
                *s = v;
            }
        } else {
            for s in slot.iter_mut() {
                *s = 0.0;
            }
        }
    }
}

fn write_frames_i16(out: &mut [i16], channels: usize, frames: &[Vec<f32>], position: &AtomicUsize) {
    for slot in out.chunks_mut(channels) {
        let idx = position.fetch_add(1, Ordering::AcqRel);
        if idx < frames.len() {
            let frame = &frames[idx];
            for (s, &v) in slot.iter_mut().zip(frame.iter()) {
                *s = (v.clamp(-1.0, 1.0) * 32_767.0) as i16;
            }
        } else {
            for s in slot.iter_mut() {
                *s = 0;
            }
        }
    }
}

fn write_frames_u16(out: &mut [u16], channels: usize, frames: &[Vec<f32>], position: &AtomicUsize) {
    for slot in out.chunks_mut(channels) {
        let idx = position.fetch_add(1, Ordering::AcqRel);
        if idx < frames.len() {
            let frame = &frames[idx];
            for (s, &v) in slot.iter_mut().zip(frame.iter()) {
                *s = ((v.clamp(-1.0, 1.0) * 32_767.0) + 32_768.0) as u16;
            }
        } else {
            for s in slot.iter_mut() {
                *s = 32_768;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mono → mono: frames pass through verbatim.
    #[test]
    fn build_frames_mono_to_mono() {
        let s = PcmSamples::from_mono(16_000, vec![0.1, 0.2, 0.3]);
        let f = build_output_frames(&s, 1);
        assert_eq!(f, vec![vec![0.1], vec![0.2], vec![0.3]]);
    }

    /// Mono → stereo: each sample duplicated across both
    /// channels.
    #[test]
    fn build_frames_mono_to_stereo() {
        let s = PcmSamples::from_mono(16_000, vec![0.1, 0.2]);
        let f = build_output_frames(&s, 2);
        assert_eq!(f, vec![vec![0.1, 0.1], vec![0.2, 0.2]]);
    }

    /// Stereo → mono: only the first channel survives. (This is
    /// a lossy fallback when the device wants fewer channels
    /// than the source has. Phase 6 will replace with proper
    /// downmix.)
    #[test]
    fn build_frames_stereo_to_mono() {
        let s = PcmSamples {
            sample_rate: 16_000,
            channels: 2,
            data: vec![0.1, 0.9, 0.2, 0.8],
        };
        let f = build_output_frames(&s, 1);
        assert_eq!(f, vec![vec![0.1], vec![0.2]]);
    }
}
