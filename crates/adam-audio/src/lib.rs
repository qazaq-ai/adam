// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `adam-audio` — **Phase 5 of the v6.3 phonemic-foundation arc**.
//!
//! Pure-Rust audio primitives. The v6.3 thesis (see
//! [`docs/v6_3_phonemic_foundation.md`](../../../docs/v6_3_phonemic_foundation.md))
//! requires the audio stack to be C-dependency-free so the
//! eventual ARM / watch deployment target works on a plain
//! `cargo build`. This crate is the foundation layer for that
//! goal:
//!
//! - [`PcmSamples`] — mono f32 buffer with a sample-rate tag.
//! - [`wav`] — WAV file read / write via `hound`.
//! - [`vad`] — energy-based voice-activity detection (RMS over
//!   frames + silence-duration thresholding).
//! - [`record`] — microphone capture via `cpal`, with the VAD
//!   driving an auto-stop on silence.
//! - [`play`] — speaker playback via `cpal`.
//!
//! No `whisper.cpp`. No macOS `say`. No external binaries. Build
//! with `cargo build -p adam-audio` and the entire audio pipeline
//! is statically present.
//!
//! ## Relationship to `adam-voice`
//!
//! `adam-voice` is the v6.2-era voice transducer — peripheral mic
//! capture + `whisper.cpp` STT shell-out. It will eventually be
//! rebuilt on top of `adam-audio` (with phoneme-level STT
//! replacing whisper), but the two coexist on `main` until v6.3
//! reaches Phase 8.
//!
//! ## What this crate does NOT do (yet)
//!
//! - FFT / MFCC extraction (Phase 2d will add it).
//! - Resampling (Phase 6 — for now caller is responsible).
//! - Echo cancellation (already lives in `adam-voice::aec`).
//! - Phoneme-level STT (Phase 6).
//! - Concatenative TTS (Phase 7).

#![forbid(unsafe_code)]

pub mod error;
pub mod pcm;
pub mod pitch;
pub mod play;
pub mod record;
pub mod speaker_profile;
pub mod vad;
pub mod wav;

pub use error::AudioError;
pub use pcm::PcmSamples;
pub use pitch::detect_f0;
pub use speaker_profile::{AgeBand, Gender, SpeakerProfile, detect_profile, suggest_honorific};
pub use vad::{is_silence, rms};
