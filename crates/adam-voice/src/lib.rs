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
//! ## Scope at v5.19.0 (V3 shipped; V4–V5 pending)
//!
//! - **Push-to-talk** (V0, v5.14.0): user presses Enter to start
//!   recording, Enter again to stop. Configurable via
//!   [`MicConfig::push_to_talk`].
//! - **Energy-VAD continuous listening** (V1, v5.15.0): default mode.
//!   30 ms RMS frames; silence after speech > 1500 ms ends the turn,
//!   armed only after 600 ms cumulative speech. Returns
//!   [`mic::VadStopReason`]. Multi-clause utterances split by
//!   `adam_dialog::discourse::split_compound_utterance`.
//! - **WhisperCli** (v5.14.0+): shell-out to an external
//!   `whisper-cli` (or `main` from whisper.cpp build) binary. No FFI —
//!   keeps the build surface small and avoids `unsafe`. Binary path
//!   is configurable via env var or constructor argument.
//! - **Confidence gate** (V2, v5.16.0): WhisperCli defaults to
//!   `--output-json`; [`stt::parse_whisper_json`] computes a
//!   duration-weighted geometric mean of `exp(avg_logprob)` and
//!   populates [`stt::Transcript::confidence`]. Low-confidence
//!   transcripts route through the `voice_low_confidence`
//!   clarification template family in `adam-dialog`. Default
//!   threshold 0.5 (configurable via `--whisper-confidence-threshold`).
//!   Use [`stt::WhisperCli::with_text_mode`] to opt out.
//! - **Kazakh transcript normalizer** (V3, v5.19.0): default
//!   `--prompt` priming via [`KAZAKH_PRIMING_PROMPT`] + post-processor
//!   [`normalize_kazakh_transcript`] that fixes common Whisper-medium
//!   mishearings («Салим» → «Сәлем» in greeting context, «дау лет» →
//!   «Дәулет», «менім» → «менің», «Танысайыр (тим)» → «Танысайық»,
//!   «есіңің» → «есімде» in name-statement context). Layered: word-
//!   boundary mergers + context-conditional phoneme substitutions +
//!   artifact trimming. Per-rule unit tests. Pure pattern rewriting,
//!   no ML.
//! - **No barge-in / TTS interruption** — V4, v5.20.0+.
//! - **No golden audio corpus** — V5, v5.21.0+.
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
pub mod normalizer;
pub mod stt;

pub use error::{Result, VoiceError};
pub use mic::{MicCapture, MicConfig, VadStopReason, write_wav};
pub use normalizer::normalize_kazakh_transcript;
pub use stt::{KAZAKH_PRIMING_PROMPT, Transcript, WhisperCli};
