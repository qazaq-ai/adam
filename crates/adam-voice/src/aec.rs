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

use aec3::nodes::audio::AudioFormat;
use aec3::pipelines::linear::{self, LinearPipeline};

use crate::error::{Result, VoiceError};

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
}
