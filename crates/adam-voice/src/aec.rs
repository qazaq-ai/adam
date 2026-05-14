//! **v5.25.0** — Voice arc V4 part 2 (AEC infrastructure).
//!
//! Acoustic Echo Cancellation wrapper around the `aec3` crate
//! (RubyBit's pure-Rust port of WebRTC AEC3). This is the foundation
//! for VAD-during-TTS barge-in: when the system is speaking through
//! the speakers AND the microphone is open at the same time, the mic
//! picks up the TTS audio as «echo». Without AEC, a VAD would
//! false-trigger on adam's own voice and try to interrupt itself.
//!
//! ## Architectural boundary
//!
//! AEC sits on the **input side** of the voice pipeline, before VAD:
//!
//! ```text
//!   mic capture (cpal, 16 kHz mono i16)
//!     → AecProcessor::process_capture(mic_frame, render_frame)
//!         (render_frame = what was sent to speakers ~116 ms ago)
//!     → echo-cancelled frame
//!     → VAD / Whisper STT
//! ```
//!
//! The `aec3` algorithm needs BOTH the mic capture AND a reference
//! («render») signal — what's currently being played through the
//! speakers. Without the reference signal, AEC cannot do anything;
//! it isn't a magic noise reducer. The caller is responsible for
//! tapping the TTS audio output and feeding it via [`AecProcessor::
//! process_render`].
//!
//! v5.25.0 ships the AEC processor as a building block, fully tested
//! on synthetic echo signals. v5.25.5+ will wire it into the actual
//! TTS playback path (which requires moving TTS playback in-process
//! so the render signal is accessible).
//!
//! ## Frame format
//!
//! `aec3` operates on 10 ms f32 frames at 16, 32, or 48 kHz. We use
//! **16 kHz mono** to match Whisper's input format — this avoids an
//! extra resampling pass and keeps the data flow simple. 10 ms at
//! 16 kHz = 160 samples per frame.

use std::sync::{Arc, Mutex};

use aec3::nodes::audio::AudioFormat;
use aec3::pipelines::linear::{self, LinearPipeline};

use crate::error::{Result, VoiceError};

/// **v5.26.0** — shared queue of resampled render samples flowing
/// from the TTS playback thread to the mic-processor thread. cpal
/// playback callback (via [`RenderTap`](crate::playback::RenderTap))
/// pushes 16 kHz mono f32 samples here; the mic-processor drains
/// them and feeds [`AecProcessor::process_render`] before processing
/// each paired capture frame.
///
/// Why a queue instead of `Arc<Mutex<AecProcessor>>`: the underlying
/// `aec3::Runtime` is `!Send` (its node trait objects don't carry a
/// `Send` bound), so the AEC processor can't cross threads. The queue
/// is `Send + Sync` because it holds plain `Vec<f32>` data.
pub type RenderQueue = Arc<Mutex<Vec<f32>>>;

/// 10 ms frame size at 16 kHz mono.
pub const FRAME_SAMPLES: usize = 160;

/// Sample rate the AEC pipeline operates at. Matched to Whisper's
/// input format so AEC output feeds VAD / STT without extra resampling.
pub const AEC_SAMPLE_RATE: u32 = 16_000;

/// Acoustic echo canceller wrapper.
///
/// Holds the `aec3` linear pipeline (HPF → AEC3 → NS → AGC2) and
/// provides two methods:
/// - [`process_render`] — call with what's being sent to the speakers
///   (TTS audio frames)
/// - [`process_capture`] — call with mic frames; returns the echo-
///   cancelled output
///
/// Both methods take 10 ms frames (160 samples at 16 kHz mono).
///
/// **Thread safety:** `AecProcessor` is `!Sync` (the underlying
/// pipeline holds mutable state). Wrap in `Mutex` if multiple threads
/// need access.
pub struct AecProcessor {
    pipeline: LinearPipeline,
}

impl AecProcessor {
    /// Construct a processor at 16 kHz mono with the default initial
    /// delay estimate. The estimate is refined adaptively as the
    /// algorithm sees more render/capture pairs.
    pub fn new() -> Result<Self> {
        let format = AudioFormat::ten_ms(AEC_SAMPLE_RATE, 1);
        let pipeline = linear::builder(format, format)
            // 116 ms is the WebRTC default initial delay estimate
            // (mic→speaker round-trip on consumer hardware). The
            // algorithm refines this online.
            .initial_delay_ms(116)
            .build()
            .map_err(|e| VoiceError::AecBuild(e.to_string()))?;
        Ok(Self { pipeline })
    }

    /// Feed a 10 ms render frame (what's being played through the
    /// speakers — TTS audio). The frame is consumed; the caller can
    /// still play the same data through its audio device. AEC uses
    /// this internally as the reference signal.
    ///
    /// Returns `VoiceError::FrameSize` if `frame.len() != FRAME_SAMPLES`.
    pub fn process_render(&mut self, frame: &[f32]) -> Result<()> {
        if frame.len() != FRAME_SAMPLES {
            return Err(VoiceError::FrameSize {
                expected: FRAME_SAMPLES,
                actual: frame.len(),
            });
        }
        self.pipeline
            .handle_render_frame(frame)
            .map_err(|e| VoiceError::AecProcess(e.to_string()))?;
        Ok(())
    }

    /// Process a 10 ms capture frame (mic input, including echo from
    /// the speakers) and write the echo-cancelled output into `output`.
    /// Both slices must be exactly [`FRAME_SAMPLES`] long.
    ///
    /// Returns `true` when AEC produced an output for this frame
    /// (typically every call after the algorithm warms up). When the
    /// pipeline is still buffering and didn't emit anything, returns
    /// `false`; in that case `output` was not touched and the caller
    /// should treat the original capture frame as the AEC output (or
    /// drop it).
    pub fn process_capture(&mut self, capture: &[f32], output: &mut [f32]) -> Result<bool> {
        if capture.len() != FRAME_SAMPLES {
            return Err(VoiceError::FrameSize {
                expected: FRAME_SAMPLES,
                actual: capture.len(),
            });
        }
        if output.len() != FRAME_SAMPLES {
            return Err(VoiceError::FrameSize {
                expected: FRAME_SAMPLES,
                actual: output.len(),
            });
        }
        self.pipeline
            .process_capture_frame(capture, output)
            .map_err(|e| VoiceError::AecProcess(e.to_string()))
    }
}

impl std::fmt::Debug for AecProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AecProcessor")
            .field("sample_rate_hz", &AEC_SAMPLE_RATE)
            .field("frame_samples", &FRAME_SAMPLES)
            .finish()
    }
}

/// Convert a 16-bit PCM mic frame to f32 in `[-1.0, 1.0]` — the
/// numeric format `aec3` expects. The reverse direction is provided by
/// [`f32_frame_to_i16`] for paths that need to write the echo-
/// cancelled output back to a `Vec<i16>` (WAV / Whisper input).
pub fn i16_frame_to_f32(input: &[i16], output: &mut [f32]) {
    debug_assert_eq!(input.len(), output.len());
    const SCALE: f32 = 1.0 / (i16::MAX as f32);
    for (i, s) in input.iter().enumerate() {
        output[i] = (*s as f32) * SCALE;
    }
}

/// Inverse of [`i16_frame_to_f32`]. Clamps to `i16` range to handle
/// AEC outputs that occasionally exceed `[-1.0, 1.0]` after gain
/// processing.
pub fn f32_frame_to_i16(input: &[f32], output: &mut [i16]) {
    debug_assert_eq!(input.len(), output.len());
    const SCALE: f32 = i16::MAX as f32;
    for (i, s) in input.iter().enumerate() {
        let scaled = (*s * SCALE).clamp(i16::MIN as f32, i16::MAX as f32);
        output[i] = scaled as i16;
    }
}

/// **v5.26.0** — linear-interpolation resampler from `src_rate` to
/// [`AEC_SAMPLE_RATE`]. Used to bridge the cpal device rate (typically
/// 44.1 / 48 / 96 kHz) down to AEC's 16 kHz working rate.
///
/// Same trade-off as the mic-input path: not audiophile-quality, but
/// adequate for AEC reference signal and keeps the dependency
/// footprint zero. Source audio shouldn't have content above 8 kHz
/// (Nyquist for 16 kHz output), so aliasing is mild on speech.
pub fn resample_to_aec_rate(input: &[f32], src_rate: u32) -> Vec<f32> {
    if src_rate == AEC_SAMPLE_RATE {
        return input.to_vec();
    }
    if input.is_empty() {
        return Vec::new();
    }
    let ratio = src_rate as f64 / AEC_SAMPLE_RATE as f64;
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

/// **v5.26.0** — create a new empty [`RenderQueue`] sized for ~250 ms
/// of headroom at the AEC rate (typical playback-callback burst is
/// 10-20 ms; 250 ms gives plenty of slack before the mic processor
/// drains).
pub fn new_render_queue() -> RenderQueue {
    Arc::new(Mutex::new(Vec::with_capacity(AEC_SAMPLE_RATE as usize / 4)))
}

/// **v5.26.0** — build a [`RenderTap`](crate::playback::RenderTap)
/// closure that resamples device-rate playback samples to the AEC
/// rate and pushes them into the shared [`RenderQueue`].
///
/// The returned closure is `Send + Sync` and can be passed directly
/// to [`crate::playback::play_wav`] / [`crate::playback::play_samples`].
///
/// On lock-poison (queue poisoned by another thread), the closure
/// silently skips the chunk rather than panic — audio thread must
/// never panic.
pub fn aec_render_tap(queue: RenderQueue, device_rate: u32) -> crate::playback::RenderTap {
    Arc::new(move |chunk: &[f32]| {
        let resampled = resample_to_aec_rate(chunk, device_rate);
        if let Ok(mut q) = queue.lock() {
            q.extend_from_slice(&resampled);
        }
    })
}

/// **v5.26.0** — process a capture buffer through AEC in 10-ms-frame
/// chunks. For each capture frame, pulls the matching render frame
/// from the [`RenderQueue`] (or silence if the queue is empty,
/// indicating no TTS was playing during that capture), feeds both to
/// the AEC processor, and emits the echo-cancelled output as i16.
///
/// The AEC processor lives in the same thread as this call — the
/// pairing of render+capture happens here, fully synchronously. The
/// render queue is the cross-thread bridge from the playback callback.
///
/// Frames shorter than [`FRAME_SAMPLES`] at the tail of `capture_i16`
/// are passed through unchanged.
pub fn process_capture_chunked(
    aec: &mut AecProcessor,
    queue: &RenderQueue,
    capture_i16: &[i16],
) -> Result<Vec<i16>> {
    let mut output = Vec::with_capacity(capture_i16.len());
    let mut idx = 0;
    let mut capture_f32 = vec![0.0f32; FRAME_SAMPLES];
    let mut output_f32 = vec![0.0f32; FRAME_SAMPLES];
    let mut i16_out = vec![0i16; FRAME_SAMPLES];
    let silent_render = vec![0.0f32; FRAME_SAMPLES];
    while idx + FRAME_SAMPLES <= capture_i16.len() {
        let slice = &capture_i16[idx..idx + FRAME_SAMPLES];
        i16_frame_to_f32(slice, &mut capture_f32);
        // Drain one render frame from the queue, or use silence when
        // empty. If the queue has MORE than 1 frame's worth, we
        // drain just FRAME_SAMPLES; the next call picks up where
        // this left off.
        let render_frame: Vec<f32> = {
            let mut q = queue
                .lock()
                .map_err(|_| VoiceError::AecProcess("render queue poisoned".into()))?;
            if q.len() >= FRAME_SAMPLES {
                q.drain(..FRAME_SAMPLES).collect()
            } else {
                silent_render.clone()
            }
        };
        aec.process_render(&render_frame)?;
        let produced = aec.process_capture(&capture_f32, &mut output_f32)?;
        if produced {
            f32_frame_to_i16(&output_f32, &mut i16_out);
            output.extend_from_slice(&i16_out);
        } else {
            output.extend_from_slice(slice);
        }
        idx += FRAME_SAMPLES;
    }
    if idx < capture_i16.len() {
        output.extend_from_slice(&capture_i16[idx..]);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    /// Generate a 10 ms sine-wave frame at `freq_hz`, amplitude in
    /// `[-1.0, 1.0]`. Used as a synthetic render signal (TTS proxy).
    fn sine_frame(freq_hz: f32, amplitude: f32, phase: f32) -> Vec<f32> {
        let mut out = vec![0.0f32; FRAME_SAMPLES];
        let dt = 1.0 / AEC_SAMPLE_RATE as f32;
        for (i, s) in out.iter_mut().enumerate() {
            let t = i as f32 * dt;
            *s = amplitude * ((2.0 * PI * freq_hz * t) + phase).sin();
        }
        out
    }

    /// Frame energy (sum of squares). Used to verify AEC reduced the
    /// echo component of the capture signal.
    fn frame_energy(frame: &[f32]) -> f32 {
        frame.iter().map(|s| s * s).sum()
    }

    #[test]
    fn processor_constructs_v5250() {
        let proc = AecProcessor::new();
        assert!(proc.is_ok(), "AecProcessor::new should succeed");
    }

    #[test]
    fn process_render_rejects_wrong_frame_size_v5250() {
        let mut proc = AecProcessor::new().unwrap();
        let bad = vec![0.0f32; FRAME_SAMPLES + 1];
        let err = proc.process_render(&bad);
        assert!(matches!(err, Err(VoiceError::FrameSize { .. })));
    }

    #[test]
    fn process_capture_rejects_wrong_frame_size_v5250() {
        let mut proc = AecProcessor::new().unwrap();
        let bad_in = vec![0.0f32; FRAME_SAMPLES - 1];
        let mut out = vec![0.0f32; FRAME_SAMPLES];
        let err = proc.process_capture(&bad_in, &mut out);
        assert!(matches!(err, Err(VoiceError::FrameSize { .. })));
    }

    /// **v5.25.0** — core property test. Feed the same 1 kHz tone as
    /// BOTH the render and (a scaled copy of) the capture signal. The
    /// capture represents the «mic picking up the speaker echo». After
    /// processing N frames, the AEC output should have lower energy
    /// than the original capture — the algorithm subtracts the echo
    /// component. The exact reduction depends on `aec3` warm-up; we
    /// require at least a measurable drop after 200 frames of warmup.
    #[test]
    fn aec_reduces_echo_energy_on_synthetic_signal_v5250() {
        let mut proc = AecProcessor::new().unwrap();
        // 1 kHz tone, full amplitude — simulates TTS output.
        let tone = sine_frame(1000.0, 0.5, 0.0);
        // Capture = the same tone at 50% amplitude (speaker→mic
        // attenuation). No user speech, so AEC has nothing to
        // preserve and should suppress aggressively.
        let echo: Vec<f32> = tone.iter().map(|s| s * 0.5).collect();

        // Warm up the pipeline. AEC3 needs ~100-200 frames of
        // render/capture pairs to estimate the echo path before
        // emitting useful output.
        let mut output = vec![0.0f32; FRAME_SAMPLES];
        for _ in 0..200 {
            proc.process_render(&tone).unwrap();
            let _ = proc.process_capture(&echo, &mut output).unwrap();
        }

        // After warmup: original echo energy vs cancelled energy.
        let original_energy = frame_energy(&echo);
        proc.process_render(&tone).unwrap();
        let produced = proc.process_capture(&echo, &mut output).unwrap();
        let cancelled_energy = frame_energy(&output);

        assert!(produced, "AEC should be producing output by now");
        assert!(
            cancelled_energy < original_energy,
            "AEC output energy {cancelled_energy:.4} should be < original \
             capture energy {original_energy:.4} (echo not cancelled)"
        );
    }

    /// **v5.25.0** — when render is silent (no TTS playing), capture
    /// should pass through largely unchanged. Verifies AEC isn't
    /// destroying genuine user speech when there's no echo to cancel.
    #[test]
    fn aec_preserves_capture_when_render_silent_v5250() {
        let mut proc = AecProcessor::new().unwrap();
        let silent = vec![0.0f32; FRAME_SAMPLES];
        // User speech proxy: 300 Hz tone (human voice band).
        let speech = sine_frame(300.0, 0.3, 0.0);

        let mut output = vec![0.0f32; FRAME_SAMPLES];
        // Warmup with silent render + speech capture.
        for _ in 0..200 {
            proc.process_render(&silent).unwrap();
            let _ = proc.process_capture(&speech, &mut output).unwrap();
        }

        let original_energy = frame_energy(&speech);
        proc.process_render(&silent).unwrap();
        let produced = proc.process_capture(&speech, &mut output).unwrap();
        let preserved_energy = frame_energy(&output);

        assert!(produced);
        // With no render signal, AEC's HPF + NS + AGC may still touch
        // the signal mildly, but the speech energy should remain in
        // the same order of magnitude (within 6 dB = 4× ratio).
        let ratio = preserved_energy / original_energy;
        assert!(
            ratio > 0.25,
            "AEC destroyed too much non-echo capture: ratio={ratio:.3} \
             (preserved {preserved_energy:.4} / original {original_energy:.4})"
        );
    }

    #[test]
    fn i16_to_f32_round_trip_within_quantisation_v5250() {
        let original_i16: Vec<i16> = (0..FRAME_SAMPLES)
            .map(|i| ((i as i32 * 100) - 8000) as i16)
            .collect();
        let mut f32_buf = vec![0.0f32; FRAME_SAMPLES];
        i16_frame_to_f32(&original_i16, &mut f32_buf);
        let mut round_trip_i16 = vec![0i16; FRAME_SAMPLES];
        f32_frame_to_i16(&f32_buf, &mut round_trip_i16);
        // i16 → f32 → i16 loses ≤ 1 LSB per sample due to f32
        // rounding; checking max abs diff captures this exactly.
        let max_diff = original_i16
            .iter()
            .zip(round_trip_i16.iter())
            .map(|(a, b)| (a.abs_diff(*b)) as i32)
            .max()
            .unwrap_or(0);
        assert!(
            max_diff <= 1,
            "i16↔f32 round-trip should be within 1 LSB; max diff = {max_diff}"
        );
    }

    #[test]
    fn f32_to_i16_clamps_out_of_range_v5250() {
        let oversaturated = vec![2.0f32; FRAME_SAMPLES]; // above 1.0
        let mut clamped = vec![0i16; FRAME_SAMPLES];
        f32_frame_to_i16(&oversaturated, &mut clamped);
        assert!(clamped.iter().all(|&s| s == i16::MAX));

        let undersaturated = vec![-2.0f32; FRAME_SAMPLES];
        f32_frame_to_i16(&undersaturated, &mut clamped);
        assert!(clamped.iter().all(|&s| s == i16::MIN));
    }

    // ─── v5.26.0 — AEC pipeline wiring tests ────────────────────────

    #[test]
    fn resample_to_aec_rate_passthrough_when_already_16k_v5260() {
        let input: Vec<f32> = (0..160).map(|i| i as f32 / 160.0).collect();
        let out = resample_to_aec_rate(&input, AEC_SAMPLE_RATE);
        assert_eq!(out, input);
    }

    #[test]
    fn resample_to_aec_rate_downsamples_48k_to_16k_v5260() {
        // 480 samples @ 48 kHz = 10 ms = 160 samples @ 16 kHz.
        let input = vec![0.5f32; 480];
        let out = resample_to_aec_rate(&input, 48_000);
        // Floor division on length: floor(480 / 3) = 160 (exact ratio).
        assert_eq!(out.len(), 160);
        // Constant signal → constant output.
        assert!(out.iter().all(|s| (*s - 0.5).abs() < 1e-6));
    }

    #[test]
    fn resample_to_aec_rate_handles_empty_input_v5260() {
        let empty: Vec<f32> = Vec::new();
        let out = resample_to_aec_rate(&empty, 48_000);
        assert!(out.is_empty());
    }

    #[test]
    fn render_tap_pushes_to_queue_v5260() {
        let queue = new_render_queue();
        let tap = aec_render_tap(Arc::clone(&queue), AEC_SAMPLE_RATE);
        let chunk = vec![0.1f32; 100];
        tap(&chunk);
        let q = queue.lock().unwrap();
        assert_eq!(q.len(), 100);
        assert!((q[0] - 0.1).abs() < 1e-6);
    }

    #[test]
    fn render_tap_resamples_device_rate_to_aec_rate_v5260() {
        let queue = new_render_queue();
        let tap = aec_render_tap(Arc::clone(&queue), 48_000);
        // 480 samples @ 48 kHz → 160 samples @ 16 kHz.
        let chunk = vec![0.3f32; 480];
        tap(&chunk);
        let q = queue.lock().unwrap();
        assert_eq!(q.len(), 160);
    }

    /// **v5.26.0** — end-to-end paired test. Feed render frames via
    /// the tap (simulating TTS playback) AND matching capture frames
    /// via process_capture_chunked (simulating mic). After warmup,
    /// the chunked-AEC output should have lower energy than the
    /// original capture — same property as the v5.25.0 single-frame
    /// test, but exercising the public pipeline API.
    #[test]
    fn end_to_end_paired_render_capture_reduces_echo_v5260() {
        let mut aec = AecProcessor::new().unwrap();
        let queue = new_render_queue();
        let tap = aec_render_tap(Arc::clone(&queue), AEC_SAMPLE_RATE);

        let tone = sine_frame(1000.0, 0.5, 0.0);
        let echo: Vec<f32> = tone.iter().map(|s| s * 0.5).collect();
        let echo_i16: Vec<i16> = {
            let mut buf = vec![0i16; FRAME_SAMPLES];
            f32_frame_to_i16(&echo, &mut buf);
            buf
        };

        // Warmup: feed 200 paired frames.
        for _ in 0..200 {
            tap(&tone);
            let _ = process_capture_chunked(&mut aec, &queue, &echo_i16).unwrap();
        }

        // Post-warmup measurement.
        tap(&tone);
        let cleaned = process_capture_chunked(&mut aec, &queue, &echo_i16).unwrap();
        let cleaned_f32: Vec<f32> = cleaned
            .iter()
            .map(|&s| s as f32 / i16::MAX as f32)
            .collect();

        let original_energy: f32 = echo.iter().map(|s| s * s).sum();
        let cleaned_energy: f32 = cleaned_f32.iter().map(|s| s * s).sum();

        assert!(
            cleaned_energy < original_energy,
            "paired AEC pipeline should reduce echo: cleaned={cleaned_energy:.4} \
             vs original={original_energy:.4}"
        );
    }

    #[test]
    fn process_capture_chunked_handles_tail_smaller_than_frame_v5260() {
        let mut aec = AecProcessor::new().unwrap();
        let queue = new_render_queue();
        // 160 + 50 samples → one frame + 50-sample tail.
        let mut input = vec![0i16; FRAME_SAMPLES + 50];
        for (i, s) in input.iter_mut().enumerate() {
            *s = (i * 100) as i16;
        }
        let out = process_capture_chunked(&mut aec, &queue, &input).unwrap();
        assert_eq!(out.len(), input.len(), "output length must match input");
        assert_eq!(&out[FRAME_SAMPLES..], &input[FRAME_SAMPLES..]);
    }

    #[test]
    fn process_capture_falls_back_to_silent_render_when_queue_empty_v5260() {
        let mut aec = AecProcessor::new().unwrap();
        let queue = new_render_queue(); // empty
        let input = vec![0i16; FRAME_SAMPLES];
        // Should not panic / error even with empty queue.
        let out = process_capture_chunked(&mut aec, &queue, &input).unwrap();
        assert_eq!(out.len(), input.len());
    }
}
