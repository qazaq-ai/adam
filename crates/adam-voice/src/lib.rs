//! `adam-voice` — voice I/O transducer for the Qazaq IR research
//! kernel.
//!
//! ## Architectural boundary
//!
//! Voice input is **probabilistic**: STT engines (whisper.cpp, vosk,
//! etc.) produce the most likely transcription with an associated
//! confidence, which is fundamentally a different shape than the
//! deterministic, source-grounded kernel that consumes the result.
//! This crate handles every probabilistic step **outside** the kernel
//! and hands the dialog layer a single normalised string + confidence
//! metadata. The kernel's proof / verifier path is unchanged — every
//! emitted claim still traces to a curated source or a grounded
//! reasoning chain.
//!
//! ```text
//!   mic capture (cpal)
//!     → audio buffer (WAV, 16kHz mono)
//!     → STT (whisper.cpp shell-out)
//!     → (text, confidence)
//!     → Conversation::turn(text, ...)   ← deterministic kernel from here
//! ```
//!
//! ## Scope at v5.14.0 (V0)
//!
//! - **Push-to-talk:** user presses Enter to start recording, Enter
//!   again (or 30 s timeout) to stop. No VAD / continuous listening
//!   yet — that's v5.15.0.
//! - **WhisperCli:** shell-out to an external `whisper-cli` (or
//!   `main` from whisper.cpp build) binary. No FFI — keeps the build
//!   surface small and avoids `unsafe`. Binary path is configurable
//!   via env var or constructor argument.
//! - **No confidence gate / clarification turn:** the v5.16.0
//!   milestone routes low-confidence transcripts through a
//!   clarification template family. v5.14.0 surfaces the raw
//!   transcript directly — the user-visible loop is honest about
//!   what the engine returned.
//! - **No barge-in / TTS interruption** — v5.18.0.
//!
//! ## Why a separate crate, not a module in `adam-dialog`?
//!
//! The dialog layer is deterministic, allocation-light, and has zero
//! audio dependencies. Pulling cpal / hound / shell-out into
//! adam-dialog would (a) bloat its compile time, (b) couple the
//! deterministic kernel to a probabilistic transducer, (c) make
//! cross-language port harder (the same architectural boundary
//! applies whether STT is whisper.cpp or vosk or any other engine).
//! Keeping voice in its own crate preserves the architectural
//! invariant.

pub mod error;
pub mod mic;
pub mod stt;

pub use error::{Result, VoiceError};
pub use mic::{MicCapture, MicConfig, write_wav};
pub use stt::{Transcript, WhisperCli};
