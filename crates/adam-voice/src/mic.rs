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
//! cpal hands us a real-time callback on its own thread. The
//! callback appends decoded samples to a shared `Arc<Mutex<Vec<i16>>>`
//! buffer, gated by an `Arc<AtomicBool>` running flag. The main
//! thread reads the flag at stop time and copies the buffer once the
//! stream is dropped (cpal joins the audio thread on drop, so by
//! then no more callbacks are firing). v5.14.5 replaced the v5.14.0
//! mpsc-channel-with-drain-timeout design after a live-test bug
//! report — the drain timeout cut recordings short on natural speech
//! pauses; the stop signal is now fully user-driven.

use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};

use crate::error::{Result, VoiceError};

/// Target sample rate for whisper.cpp input. Whisper internally
/// expects 16 kHz mono; we resample to this regardless of what the
/// mic device produces.
pub const WHISPER_SAMPLE_RATE: u32 = 16_000;

/// **v5.15.0 (V1).** Why [`MicCapture::wait_for_vad_stop`] returned —
/// the caller may want to print a different UX prompt for a natural
/// end-of-utterance vs the safety-cap fire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VadStopReason {
    /// Speech detected, then silence threshold exceeded — natural
    /// end of utterance.
    Silence,
    /// `config.max_duration` elapsed without VAD firing. Either the
    /// user kept speaking past the cap, or the mic captured pure
    /// silence and nothing armed the VAD.
    MaxDuration,
}

/// User-tunable mic capture parameters.
#[derive(Debug, Clone)]
pub struct MicConfig {
    /// Hard safety cap on a single recording — applies to both
    /// push-to-talk and VAD modes. v5.15.0 made VAD the primary stop
    /// signal; this is the upper bound the audio thread will not
    /// exceed under any circumstance.
    pub max_duration: Duration,
    /// **v5.15.0 (V1).** Auto-stop after this much continuous silence
    /// following the first detected speech. Suppresses the «mic cut
    /// me off mid-word» class of bug (we wait for a post-speech
    /// silence window, not an inter-chunk gap).
    pub vad_silence_after_speech: Duration,
    /// **v5.15.0 (V1).** RMS amplitude threshold below which a 30 ms
    /// frame counts as silence. Default 0.02 × i16::MAX (≈ −34 dBFS)
    /// — comfortably above quiet-room noise floor on laptop mics.
    pub vad_amplitude_threshold: f32,
    /// **v5.15.0 (V1).** Minimum total speech-energy time before VAD
    /// arms silence detection. Default 600 ms — covers a Kazakh
    /// greeting like «Сәлеметсіз бе» which has natural intra-
    /// utterance gaps.
    pub vad_min_speech_before_silence: Duration,
    /// **v5.15.0 (V1).** When `true`, the recorder runs continuous-
    /// listening with VAD-based auto-stop; when `false`, the caller
    /// stops the capture manually (push-to-talk, v5.14.0 behaviour).
    /// Both modes share the same `max_duration` safety cap.
    pub vad_enabled: bool,
}

impl Default for MicConfig {
    fn default() -> Self {
        Self {
            max_duration: Duration::from_secs(30),
            vad_silence_after_speech: Duration::from_millis(1500),
            vad_amplitude_threshold: 0.02 * (i16::MAX as f32),
            vad_min_speech_before_silence: Duration::from_millis(600),
            vad_enabled: true,
        }
    }
}

impl MicConfig {
    /// Push-to-talk preset — VAD off, caller controls stop signal.
    /// Equivalent to v5.14.0 behaviour.
    pub fn push_to_talk() -> Self {
        Self {
            vad_enabled: false,
            ..Self::default()
        }
    }
}

/// Push-to-talk capture session. Holds the cpal stream + a shared
/// sample buffer the audio callback writes into. The stream stays
/// open until [`stop`] is called explicitly OR the configured
/// `max_duration` elapses (safety cap, queried by the caller via
/// [`elapsed`]).
///
/// **v5.14.5** — replaced the v5.14.0 `recv_timeout`-drain stop
/// signal which had a subtle bug: cpal callbacks can pause >100 ms
/// between chunks (low CPU load, large buffer sizes), and the drain
/// loop would terminate as soon as the first such pause hit — cutting
/// the recording before the user finished speaking. Post-v5.14.5
/// the stop signal is **fully user-driven** (Enter in adam_chat),
/// not derived from inter-chunk timing.
pub struct MicCapture {
    config: MicConfig,
    stream: cpal::Stream,
    samples: Arc<Mutex<Vec<i16>>>,
    running: Arc<AtomicBool>,
    started_at: Instant,
    device_sample_rate: u32,
    device_channels: u16,
}

impl MicCapture {
    /// Open the default input device, negotiate a config, and start
    /// the capture stream. The capture continues until [`stop`] is
    /// called or [`elapsed`] exceeds `config.max_duration` (caller is
    /// responsible for polling).
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

        let samples: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::with_capacity(
            // Pre-allocate ~1 s worth at 48 kHz × max channels.
            (device_sample_rate as usize * device_channels as usize).max(48_000),
        )));
        let running = Arc::new(AtomicBool::new(true));

        let stream = match sample_format {
            SampleFormat::I16 => {
                let buf = Arc::clone(&samples);
                let run = Arc::clone(&running);
                device
                    .build_input_stream(
                        &cfg,
                        move |data: &[i16], _: &cpal::InputCallbackInfo| {
                            if !run.load(Ordering::Relaxed) {
                                return;
                            }
                            if let Ok(mut g) = buf.lock() {
                                g.extend_from_slice(data);
                            }
                        },
                        move |e| {
                            eprintln!("[adam-voice mic] stream error: {e}");
                        },
                        None,
                    )
                    .map_err(|e| VoiceError::StreamBuild(e.to_string()))?
            }
            SampleFormat::F32 => {
                let buf = Arc::clone(&samples);
                let run = Arc::clone(&running);
                device
                    .build_input_stream(
                        &cfg,
                        move |data: &[f32], _: &cpal::InputCallbackInfo| {
                            if !run.load(Ordering::Relaxed) {
                                return;
                            }
                            if let Ok(mut g) = buf.lock() {
                                g.reserve(data.len());
                                for &s in data {
                                    g.push((s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16);
                                }
                            }
                        },
                        move |e| {
                            eprintln!("[adam-voice mic] stream error: {e}");
                        },
                        None,
                    )
                    .map_err(|e| VoiceError::StreamBuild(e.to_string()))?
            }
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
            samples,
            running,
            started_at: Instant::now(),
            device_sample_rate,
            device_channels,
        })
    }

    /// Wall-clock time since [`start`] returned. Caller polls this
    /// against `config.max_duration` to enforce the safety cap from
    /// the main thread without coupling the audio callback to time.
    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Configured `max_duration` from the [`MicConfig`].
    pub fn max_duration(&self) -> Duration {
        self.config.max_duration
    }

    /// **v5.15.0 (V1).** Block the main thread until VAD declares
    /// the utterance complete (silence-after-speech > threshold) OR
    /// `max_duration` elapses. Returns when one of those triggers
    /// fires; caller then calls [`stop`] to retrieve the audio.
    ///
    /// Uses a simple energy-based VAD over the shared sample buffer
    /// — periodically peeks the tail, computes RMS over a 30 ms
    /// window, increments a silence counter when energy is below
    /// threshold, resets it on speech energy. Arms silence detection
    /// only after `vad_min_speech_before_silence` of cumulative speech
    /// energy has been observed, so early intra-syllable pauses don't
    /// prematurely stop the recording.
    ///
    /// Returns [`VadStopReason`] indicating which trigger fired —
    /// useful for the caller's UX (different prompt on max-duration
    /// vs natural end-of-utterance).
    pub fn wait_for_vad_stop(&self) -> Result<VadStopReason> {
        let frame_samples = (self.device_sample_rate as usize / 1000) * 30; // ~30 ms
        let frame_samples = frame_samples.max(64);
        let frame_duration = Duration::from_millis(30);
        let threshold = self.config.vad_amplitude_threshold;
        let silence_target = self.config.vad_silence_after_speech;
        let min_speech = self.config.vad_min_speech_before_silence;

        let mut speech_accum = Duration::ZERO;
        let mut silence_accum = Duration::ZERO;
        let mut last_read_pos: usize = 0;
        let poll_interval = Duration::from_millis(30);

        loop {
            std::thread::sleep(poll_interval);
            if self.elapsed() >= self.config.max_duration {
                return Ok(VadStopReason::MaxDuration);
            }
            let tail = {
                let g = match self.samples.lock() {
                    Ok(g) => g,
                    Err(_) => continue,
                };
                if g.len() <= last_read_pos {
                    continue;
                }
                let start = last_read_pos;
                last_read_pos = g.len();
                g[start..].to_vec()
            };
            // Slice the new tail into ~30 ms frames; classify each
            // as speech or silence by RMS amplitude. The frame size
            // accounts for the device's native sample rate × channel
            // count (we haven't downmixed yet at this point).
            let per_frame = frame_samples * self.device_channels.max(1) as usize;
            for frame in tail.chunks(per_frame) {
                if frame.is_empty() {
                    continue;
                }
                let rms = rms_amplitude(frame);
                if rms >= threshold {
                    speech_accum += frame_duration;
                    silence_accum = Duration::ZERO;
                } else if speech_accum >= min_speech {
                    silence_accum += frame_duration;
                    if silence_accum >= silence_target {
                        return Ok(VadStopReason::Silence);
                    }
                }
            }
        }
    }

    /// Stop capture and return the captured samples downmixed to
    /// mono + resampled to 16 kHz. The audio callback observes the
    /// `running` flag and stops appending immediately; we then drop
    /// the stream and copy the accumulated buffer.
    pub fn stop(self) -> Result<Vec<i16>> {
        // Signal the callback to stop appending. This is purely
        // cooperative — the callback may still be mid-call on cpal's
        // audio thread when we set the flag, and any data already
        // written before the next callback observation stays in the
        // buffer (acceptable; that's a few ms of speech).
        self.running.store(false, Ordering::Relaxed);
        // Drop the stream. cpal joins the audio thread on drop, so
        // by the time `drop` returns, no more callbacks will fire.
        drop(self.stream);
        let raw = self.samples.lock().map(|g| g.clone()).unwrap_or_default();
        let mono = downmix_to_mono(&raw, self.device_channels);
        let resampled = resample_linear(&mono, self.device_sample_rate, WHISPER_SAMPLE_RATE);
        Ok(resampled)
    }
}

/// **v5.15.0 (V1).** Root-mean-square amplitude of an i16 sample
/// slice. Used by the energy-based VAD to classify a frame as
/// speech vs silence. Pure; no I/O.
pub(crate) fn rms_amplitude(frame: &[i16]) -> f32 {
    if frame.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = frame.iter().map(|&s| (s as f64).powi(2)).sum();
    (sum_sq / frame.len() as f64).sqrt() as f32
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

    #[test]
    fn rms_amplitude_zero_for_silence_v5150() {
        let silent: Vec<i16> = vec![0; 480];
        assert_eq!(rms_amplitude(&silent), 0.0);
    }

    #[test]
    fn rms_amplitude_empty_is_zero_v5150() {
        assert_eq!(rms_amplitude(&[]), 0.0);
    }

    #[test]
    fn rms_amplitude_grows_with_signal_v5150() {
        let quiet: Vec<i16> = (0..480).map(|i| ((i % 100) as i16) * 10).collect();
        let loud: Vec<i16> = (0..480).map(|i| ((i % 100) as i16) * 200).collect();
        let r_q = rms_amplitude(&quiet);
        let r_l = rms_amplitude(&loud);
        assert!(
            r_l > r_q * 5.0,
            "loud frame should be much higher RMS than quiet"
        );
    }

    #[test]
    fn push_to_talk_preset_disables_vad_v5150() {
        let cfg = MicConfig::push_to_talk();
        assert!(!cfg.vad_enabled);
        // Other defaults preserved.
        assert_eq!(cfg.max_duration, MicConfig::default().max_duration);
    }

    #[test]
    fn default_config_enables_vad_v5150() {
        let cfg = MicConfig::default();
        assert!(cfg.vad_enabled);
        assert!(cfg.vad_silence_after_speech >= Duration::from_millis(1000));
        assert!(cfg.vad_amplitude_threshold > 0.0);
    }
}
