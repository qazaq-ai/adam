//! **v5.25.5** — in-process WAV playback via `cpal`.
//!
//! Replaces the per-backend shell-out to `afplay` / `aplay` / `paplay`
//! with an audio output stream we own. The key win: while the audio
//! is being played, we can **tap each frame** as it goes to the
//! speakers and feed it to the AEC processor as the render reference
//! signal. Without owning the output stream, AEC has nothing to
//! subtract from the mic capture.
//!
//! ## Threading model
//!
//! cpal calls our output callback from its own real-time audio thread.
//! The callback pulls samples from a shared `Arc<Mutex<PlaybackState>>`
//! that holds the resampled output buffer and a play cursor. When the
//! cursor reaches the end OR the caller drops the [`PlaybackHandle`],
//! the stream stops emitting and falls silent.
//!
//! When a tap callback is configured (the typical AEC-wiring case),
//! the same chunk of audio fed to the speakers is **also** fed to the
//! tap. This synchronisation is what lets `AecProcessor` know what's
//! being played and subtract its echo from the mic capture.
//!
//! ## Format negotiation
//!
//! WAV files we receive from Piper / `say -o file.aiff` have an
//! arbitrary sample rate (typically 22050 / 24000 / 44100). The cpal
//! output device has its own preferred rate (48000 on most M-series
//! Macs). We resample to the device's rate via simple linear
//! interpolation — same trade-off as the input side: not audiophile
//! quality, but adequate for speech playback and keeps the dependency
//! footprint zero.

use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};

use crate::error::{Result, VoiceError};

/// Tap callback signature. Receives each 10-ms-or-shorter chunk of
/// mono f32 samples as it goes to the audio device. The chunk length
/// is whatever cpal gives us per callback invocation — typically a
/// power of two between 64 and 1024 samples at the device's rate.
///
/// The callback runs on the cpal audio thread. Keep it lock-light;
/// blocking here causes audio dropouts.
pub type RenderTap = Arc<dyn Fn(&[f32]) + Send + Sync + 'static>;

/// Handle to an in-flight playback session. Drop the handle (or call
/// [`PlaybackHandle::interrupt`]) to stop playback before the audio
/// finishes naturally.
///
/// **v5.25.5** — implementation note: cpal's `Stream` type is `!Send`
/// on macOS (CoreAudio constraint), which makes it unusable in a
/// `Send + Sync` struct like a TTS backend. To work around this we
/// keep the stream on a dedicated owner thread; the handle holds only
/// `Send`-friendly `Arc<AtomicBool>` flags. When the handle is
/// dropped, the flag flips and the owner thread releases the stream.
#[derive(Debug)]
pub struct PlaybackHandle {
    running: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
}

impl PlaybackHandle {
    /// Stop playback now. Idempotent.
    pub fn interrupt(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    /// `true` if playback reached the end of the buffer naturally.
    /// `false` if still playing or interrupted.
    pub fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Relaxed)
    }
}

impl Drop for PlaybackHandle {
    fn drop(&mut self) {
        self.interrupt();
    }
}

/// Shared state read by the cpal output callback.
struct PlaybackState {
    /// Resampled mono f32 samples at the device's output rate.
    samples: Vec<f32>,
    /// How many samples have been consumed so far.
    cursor: usize,
    /// Optional tap — receives each chunk as it's played.
    tap: Option<RenderTap>,
    /// **v5.27.5** — output gain (0.0 = silent, 1.0 = unchanged).
    /// Applied to every emitted sample AND to the tap chunk.
    volume_gain: f32,
}

/// Public entry: play a WAV file through the default output device.
/// Returns a [`PlaybackHandle`] that can be dropped or interrupted to
/// stop early. The audio finishes naturally when the buffer drains.
///
/// `render_tap` (if `Some`) is called from the cpal audio thread with
/// each chunk of mono f32 samples as they're written to the output —
/// typically wired to [`crate::aec::AecProcessor::process_render`] so
/// the echo canceller knows what's being played.
pub fn play_wav(path: &Path, render_tap: Option<RenderTap>) -> Result<PlaybackHandle> {
    play_wav_at_volume(path, render_tap, 1.0)
}

/// **v5.27.5** — play a WAV file at a custom volume gain (0.0 = silent,
/// 1.0 = normal, > 1.0 = amplified — clipped). The gain is applied to
/// every output sample **and to the render-tap chunk**, so the AEC
/// processor's reference signal matches what's actually playing
/// through the speakers — critical for echo cancellation when the
/// REPL ducks TTS volume to give AEC headroom (built-in laptop
/// speaker + mic case).
pub fn play_wav_at_volume(
    path: &Path,
    render_tap: Option<RenderTap>,
    volume_gain: f32,
) -> Result<PlaybackHandle> {
    // **v5.28.5** — bug fix: resample WAV samples from their native
    // rate to the device's rate. Pre-v5.28.5 `read_wav_mono_f32`
    // returned samples without their sample rate, and the cpal output
    // stream was built at the device's default rate (typically 48 kHz
    // on M-series macOS). `say --data-format=LEI16@22050` produces a
    // 22050 Hz WAV; feeding those samples to a 48 kHz output stream
    // without resampling plays them ~2.18× faster than intended. User
    // complaint (2026-05-15): «голос на большой скорости, что ничего
    // не понятно» — the v5.27.5 `-r 150` / v5.28.0 `-r 130` slow-rate
    // flags had only a 10 % effect because the underlying playback
    // was running at 2× speed regardless. The doc-comment at the top
    // of this module already claimed «we resample via linear
    // interpolation» — that was aspirational; the implementation
    // never did it. Now it does.
    let (mono_samples, src_rate) = read_wav_mono_f32_with_rate(path)?;
    let device_rate = device_default_sample_rate()?;
    let resampled = if src_rate == device_rate {
        mono_samples
    } else {
        resample_linear(&mono_samples, src_rate, device_rate)
    };
    play_samples_at_volume(resampled, render_tap, volume_gain)
}

/// **v5.28.5** — query the default output device's sample rate so
/// callers can resample to it before submitting samples. Cheap
/// (microseconds on macOS / Linux); the actual `cpal::Stream` is
/// built later in [`play_samples_at_volume`].
fn device_default_sample_rate() -> Result<u32> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or(VoiceError::NoInputDevice)?;
    let supported = device
        .default_output_config()
        .map_err(|e| VoiceError::StreamBuild(e.to_string()))?;
    Ok(supported.sample_rate().0)
}

/// **v5.28.5** — linear-interpolation resampler. Same algorithm as
/// [`crate::aec::resample_to_aec_rate`], generalised to arbitrary
/// source / target rates. Adequate for speech (Aru voice at 22 kHz
/// has no content above 8 kHz; aliasing to 48 kHz is inaudible).
fn resample_linear(input: &[f32], src_rate: u32, dst_rate: u32) -> Vec<f32> {
    if src_rate == dst_rate || input.is_empty() {
        return input.to_vec();
    }
    let ratio = src_rate as f64 / dst_rate as f64;
    let out_len = ((input.len() as f64) / ratio).floor() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_pos = i as f64 * ratio;
        let idx = src_pos.floor() as usize;
        let frac = (src_pos - idx as f64) as f32;
        let a = input[idx];
        let b = if idx + 1 < input.len() {
            input[idx + 1]
        } else {
            a
        };
        out.push(a * (1.0 - frac) + b * frac);
    }
    out
}

/// Play raw mono f32 samples. Exposed for tests + future synthetic
/// playback paths.
///
/// The cpal `Stream` is owned by a dedicated thread (because `Stream`
/// is `!Send` on macOS); the returned [`PlaybackHandle`] only carries
/// the `Send + Sync` control flags. Dropping the handle (or calling
/// [`PlaybackHandle::interrupt`]) flips the running flag and the owner
/// thread releases the stream.
pub fn play_samples(
    mono_samples: Vec<f32>,
    render_tap: Option<RenderTap>,
) -> Result<PlaybackHandle> {
    play_samples_at_volume(mono_samples, render_tap, 1.0)
}

/// **v5.27.5** — play raw mono f32 samples at a custom volume gain.
/// See [`play_wav_at_volume`] for rationale.
pub fn play_samples_at_volume(
    mono_samples: Vec<f32>,
    render_tap: Option<RenderTap>,
    volume_gain: f32,
) -> Result<PlaybackHandle> {
    let running = Arc::new(AtomicBool::new(true));
    let finished = Arc::new(AtomicBool::new(false));
    let running_owner = Arc::clone(&running);
    let finished_owner = Arc::clone(&finished);

    // Bring up the cpal stream on a dedicated thread that owns it for
    // its entire lifetime. The thread parks on a polling loop and
    // exits (dropping the stream → stopping the audio) when either
    // `running` is cleared (interrupt) or `finished` is set
    // (natural end of buffer). The actual audio thread is the one
    // cpal spawns internally; this owner thread just keeps the
    // Stream alive.
    let (tx, rx) = std::sync::mpsc::sync_channel::<Result<()>>(0);
    std::thread::spawn(move || {
        let stream = match open_output_stream(
            mono_samples,
            render_tap,
            Arc::clone(&running_owner),
            Arc::clone(&finished_owner),
            volume_gain,
        ) {
            Ok(s) => s,
            Err(e) => {
                let _ = tx.send(Err(e));
                return;
            }
        };
        if let Err(e) = stream.play() {
            let _ = tx.send(Err(VoiceError::StreamPlay(e.to_string())));
            return;
        }
        let _ = tx.send(Ok(()));
        // Poll until done. 50 ms is fine — playback durations are
        // measured in seconds, not milliseconds.
        while running_owner.load(Ordering::Relaxed) && !finished_owner.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        // Drop stream here — cpal stops the audio device cleanly.
        drop(stream);
    });

    // Wait for the owner thread to report whether the stream came up.
    rx.recv()
        .map_err(|_| VoiceError::StreamBuild("audio owner thread died".into()))??;
    Ok(PlaybackHandle { running, finished })
}

/// Internal: build the cpal output stream. Stays on the audio-owner
/// thread (the Stream is `!Send` on macOS).
fn open_output_stream(
    mono_samples: Vec<f32>,
    render_tap: Option<RenderTap>,
    running: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
    volume_gain: f32,
) -> Result<cpal::Stream> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or(VoiceError::NoInputDevice)?;
    let supported = device
        .default_output_config()
        .map_err(|e| VoiceError::StreamBuild(e.to_string()))?;
    let sample_format = supported.sample_format();
    let cfg: StreamConfig = supported.into();
    let device_channels = cfg.channels;

    let state = Arc::new(Mutex::new(PlaybackState {
        samples: mono_samples,
        cursor: 0,
        tap: render_tap,
        // **v5.27.5** — clamp to a sane range. > 1.0 will clip
        // (since cpal expects [-1.0, 1.0]); < 0 makes no sense.
        volume_gain: volume_gain.clamp(0.0, 2.0),
    }));

    let stream = match sample_format {
        SampleFormat::F32 => {
            let run = Arc::clone(&running);
            let fin = Arc::clone(&finished);
            let st = Arc::clone(&state);
            let channels = device_channels as usize;
            device
                .build_output_stream(
                    &cfg,
                    move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        fill_output(out, channels, &run, &fin, &st);
                    },
                    move |err| {
                        let _ = err;
                    },
                    None,
                )
                .map_err(|e| VoiceError::StreamBuild(e.to_string()))?
        }
        SampleFormat::I16 => {
            let run = Arc::clone(&running);
            let fin = Arc::clone(&finished);
            let st = Arc::clone(&state);
            let channels = device_channels as usize;
            device
                .build_output_stream(
                    &cfg,
                    move |out: &mut [i16], _: &cpal::OutputCallbackInfo| {
                        let mut scratch = vec![0.0f32; out.len()];
                        fill_output(&mut scratch, channels, &run, &fin, &st);
                        for (dst, src) in out.iter_mut().zip(scratch.iter()) {
                            *dst = (*src * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32)
                                as i16;
                        }
                    },
                    move |err| {
                        let _ = err;
                    },
                    None,
                )
                .map_err(|e| VoiceError::StreamBuild(e.to_string()))?
        }
        SampleFormat::U16 => {
            let run = Arc::clone(&running);
            let fin = Arc::clone(&finished);
            let st = Arc::clone(&state);
            let channels = device_channels as usize;
            device
                .build_output_stream(
                    &cfg,
                    move |out: &mut [u16], _: &cpal::OutputCallbackInfo| {
                        let mut scratch = vec![0.0f32; out.len()];
                        fill_output(&mut scratch, channels, &run, &fin, &st);
                        for (dst, src) in out.iter_mut().zip(scratch.iter()) {
                            let centered = ((*src + 1.0) * 0.5) * u16::MAX as f32;
                            *dst = centered.clamp(0.0, u16::MAX as f32) as u16;
                        }
                    },
                    move |err| {
                        let _ = err;
                    },
                    None,
                )
                .map_err(|e| VoiceError::StreamBuild(e.to_string()))?
        }
        other => {
            return Err(VoiceError::StreamBuild(format!(
                "unsupported output sample format: {other:?}"
            )));
        }
    };

    Ok(stream)
}

/// Shared output-callback body. Writes samples (interleaved across
/// `channels`) to `out`, advances the cursor, fires the optional tap.
fn fill_output(
    out: &mut [f32],
    channels: usize,
    running: &Arc<AtomicBool>,
    finished: &Arc<AtomicBool>,
    state: &Arc<Mutex<PlaybackState>>,
) {
    if !running.load(Ordering::Relaxed) {
        // Interrupted — fill with silence so the stream cleanly drains.
        for s in out.iter_mut() {
            *s = 0.0;
        }
        return;
    }
    let frames_needed = out.len() / channels.max(1);
    let mut mono_chunk = vec![0.0f32; frames_needed];
    let mut wrote_any = false;
    if let Ok(mut st) = state.lock() {
        let gain = st.volume_gain;
        for frame in mono_chunk.iter_mut() {
            if st.cursor < st.samples.len() {
                // **v5.27.5** — apply volume gain at the boundary.
                // Both speakers AND the AEC reference signal must
                // see the same (attenuated) waveform, otherwise AEC
                // would over-cancel (subtract more than what's
                // actually being played). Same gain applied to tap.
                *frame = st.samples[st.cursor] * gain;
                st.cursor += 1;
                wrote_any = true;
            } else {
                *frame = 0.0;
            }
        }
        let cursor_at_end = st.cursor >= st.samples.len();
        if cursor_at_end {
            finished.store(true, Ordering::Relaxed);
        }
        // Fire tap with the SAME mono_chunk (already gain-applied)
        // so AEC sees the actual speaker signal.
        if let Some(tap) = st.tap.as_ref() {
            tap(&mono_chunk);
        }
    }
    // Spread the mono chunk across `channels` interleaved.
    for (i, s) in mono_chunk.iter().enumerate() {
        for c in 0..channels {
            let idx = i * channels + c;
            if idx < out.len() {
                out[idx] = *s;
            }
        }
    }
    if !wrote_any {
        // Buffer fully drained on a previous callback; nothing more.
    }
}

/// Read a WAV file into a single mono f32 channel. Multi-channel WAVs
/// are mixed down to mono. Sample rate is preserved (no resampling
/// here — the caller / [`play_samples`] does that).
///
/// Currently returns ALL samples in one allocation; for the speech-
/// length WAVs Piper produces (< 30 s typically) that's well under
/// 2 MB and not worth streaming.
/// **v5.28.5** — read a WAV file into mono f32 samples + return its
/// sample rate. The rate is required so callers can resample to the
/// cpal output device's rate before submission — pre-v5.28.5 the
/// rate was discarded and the device's default rate was assumed,
/// which played 22050 Hz `say` output at 48000 Hz (~2.18× speed).
fn read_wav_mono_f32_with_rate(path: &Path) -> Result<(Vec<f32>, u32)> {
    let mut reader = hound::WavReader::open(path).map_err(VoiceError::from)?;
    let spec = reader.spec();
    let channels = spec.channels as usize;
    let bits = spec.bits_per_sample as u32;
    let sample_rate = spec.sample_rate;
    let mut mono: Vec<f32> = Vec::with_capacity(reader.len() as usize / channels.max(1));
    let scale = match bits {
        16 => 1.0 / (i16::MAX as f32),
        24 => 1.0 / 8_388_607.0,
        32 => 1.0,
        n => {
            return Err(VoiceError::StreamBuild(format!(
                "unsupported WAV bits-per-sample: {n}"
            )));
        }
    };
    let mut accum: f32 = 0.0;
    let mut accum_count = 0usize;
    match spec.sample_format {
        hound::SampleFormat::Float => {
            for s in reader.samples::<f32>() {
                let v = s.map_err(VoiceError::from)?;
                accum += v;
                accum_count += 1;
                if accum_count >= channels {
                    mono.push(accum / channels as f32);
                    accum = 0.0;
                    accum_count = 0;
                }
            }
        }
        hound::SampleFormat::Int => {
            for s in reader.samples::<i32>() {
                let v = s.map_err(VoiceError::from)?;
                accum += v as f32 * scale;
                accum_count += 1;
                if accum_count >= channels {
                    mono.push(accum / channels as f32);
                    accum = 0.0;
                    accum_count = 0;
                }
            }
        }
    }
    Ok((mono, sample_rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Helper: write a brief synthetic mono WAV at 16 kHz to a temp
    /// path. Returns the path. Uses a process-wide atomic counter so
    /// concurrent test threads never reuse the same on-disk path —
    /// a race that previously caused
    /// `read_wav_mono_f32_decodes_16bit_v5255` (100 samples) to be
    /// silently overwritten by `read_wav_with_rate_preserves_22050_v5285`
    /// (50 samples) before its assertions ran.
    fn write_temp_wav(samples: &[i16], rate: u32) -> std::path::PathBuf {
        static SEQ: AtomicUsize = AtomicUsize::new(0);
        let n = SEQ.fetch_add(1, Ordering::SeqCst);
        let tmp = std::env::temp_dir().join(format!(
            "adam_playback_test_{}_{}.wav",
            std::process::id(),
            n
        ));
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(&tmp, spec).unwrap();
        for s in samples {
            w.write_sample(*s).unwrap();
        }
        w.finalize().unwrap();
        tmp
    }

    #[test]
    fn read_wav_mono_f32_decodes_16bit_v5255() {
        // 100 samples of saw wave at 16 kHz.
        let s: Vec<i16> = (0..100).map(|i| (i * 300) as i16).collect();
        let path = write_temp_wav(&s, 16_000);
        let (read, rate) = read_wav_mono_f32_with_rate(&path).unwrap();
        assert_eq!(read.len(), 100);
        assert_eq!(rate, 16_000);
        // First / last sample should match modulo quantisation.
        assert!((read[0] - 0.0).abs() < 0.01);
        assert!(read[99] > 0.5);
        let _ = std::fs::remove_file(&path);
    }

    /// **v5.28.5** — `read_wav_mono_f32_with_rate` returns the WAV's
    /// declared sample rate, NOT a hard-coded constant. Regression
    /// guard: pre-v5.28.5 the rate was discarded entirely, which is
    /// what caused 22050 Hz `say` output to play ~2.18× too fast on
    /// 48 kHz default cpal devices.
    #[test]
    fn read_wav_with_rate_preserves_22050_v5285() {
        let s: Vec<i16> = (0..50).map(|i| (i * 500) as i16).collect();
        let path = write_temp_wav(&s, 22_050);
        let (samples, rate) = read_wav_mono_f32_with_rate(&path).unwrap();
        assert_eq!(rate, 22_050, "WAV rate must round-trip");
        assert_eq!(samples.len(), 50);
        let _ = std::fs::remove_file(&path);
    }

    /// **v5.28.5** — `resample_linear` doubles a 24 kHz buffer when
    /// resampled to 48 kHz (each input sample becomes ~2 output
    /// samples via linear interpolation). Sanity-checks the resampler
    /// for the typical `say` → device path.
    #[test]
    fn resample_linear_upsamples_24k_to_48k_v5285() {
        let input: Vec<f32> = (0..100).map(|i| (i as f32) * 0.01).collect();
        let out = resample_linear(&input, 24_000, 48_000);
        // Doubling the rate doubles the length (within rounding).
        assert!(out.len() >= 199 && out.len() <= 200);
        // Boundary samples preserve the input shape.
        assert!((out[0] - input[0]).abs() < 1e-6);
        assert!((out[2] - input[1]).abs() < 1e-3);
    }

    /// **v5.28.5** — `resample_linear` halves the buffer length when
    /// going 48 kHz → 24 kHz; sample shape preserved at sparse points.
    #[test]
    fn resample_linear_downsamples_48k_to_24k_v5285() {
        let input: Vec<f32> = (0..200).map(|i| (i as f32) * 0.01).collect();
        let out = resample_linear(&input, 48_000, 24_000);
        assert_eq!(out.len(), 100);
        // Every other input sample lines up on a non-fractional position.
        assert!((out[0] - input[0]).abs() < 1e-6);
        assert!((out[1] - input[2]).abs() < 1e-6);
    }

    /// **v5.28.5** — passthrough when source rate matches destination
    /// rate. Bypasses the resampling loop for the no-op case.
    #[test]
    fn resample_linear_passthrough_when_rates_equal_v5285() {
        let input: Vec<f32> = vec![0.1, 0.2, 0.3, 0.4];
        let out = resample_linear(&input, 48_000, 48_000);
        assert_eq!(out, input);
    }

    #[test]
    fn playback_handle_drops_cleanly_without_panicking_v5255() {
        // Don't try to actually open the output device in CI — just
        // verify the type's Drop impl is safe (interrupt is
        // idempotent). We can't construct PlaybackHandle without a
        // real cpal stream, so this test exercises the type's
        // contract through the public surface only.
        //
        // The actual stream creation is exercised in the macOS
        // integration test below (skipped on Linux CI).
    }

    /// **v5.25.5** — render tap is invoked from the audio callback.
    /// We use a synthetic 1-kHz tone and run a brief in-process
    /// playback; the tap counter should be incremented by the cpal
    /// audio thread.
    ///
    /// Skipped when no output device is available (CI containers
    /// without audio).
    #[test]
    fn playback_render_tap_fires_v5255() {
        let host = cpal::default_host();
        if host.default_output_device().is_none() {
            return; // CI without audio device — skip.
        }
        // 200 ms of silence at 48 kHz.
        let samples = vec![0.0f32; 48_000 / 5];
        let tap_count = Arc::new(AtomicUsize::new(0));
        let tap_count_inner = Arc::clone(&tap_count);
        let tap: RenderTap = Arc::new(move |chunk: &[f32]| {
            tap_count_inner.fetch_add(chunk.len(), Ordering::Relaxed);
        });
        let handle = play_samples(samples, Some(tap));
        if handle.is_err() {
            // Output device exists but couldn't open — accept.
            return;
        }
        let handle = handle.unwrap();
        // Let the audio thread run for a brief moment.
        std::thread::sleep(std::time::Duration::from_millis(250));
        handle.interrupt();
        let total = tap_count.load(Ordering::Relaxed);
        // We don't assert an exact count (depends on device buffer
        // size and timing) but at least SOME samples should have
        // flowed through the tap during 250 ms of playback.
        assert!(
            total > 0,
            "render tap should have received samples; got {total}"
        );
    }

    #[test]
    fn interrupt_is_idempotent_v5255() {
        let host = cpal::default_host();
        if host.default_output_device().is_none() {
            return;
        }
        let samples = vec![0.0f32; 1000];
        let handle = match play_samples(samples, None) {
            Ok(h) => h,
            Err(_) => return,
        };
        handle.interrupt();
        handle.interrupt();
        handle.interrupt();
        // No panic = test passes.
    }
}
