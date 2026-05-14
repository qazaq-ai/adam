//! Voice-layer error type. Distinct from the dialog-layer errors so
//! the architectural boundary stays explicit — a `VoiceError` is a
//! probabilistic-input failure (mic disconnected, whisper binary
//! missing, transcript empty), never a kernel-deterministic-state
//! failure.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum VoiceError {
    /// No default input device available (mic disconnected, OS
    /// permissions revoked, etc.).
    #[error("no default input device available")]
    NoInputDevice,

    /// cpal stream-build failure. Usually means the requested config
    /// (channels / sample rate / format) doesn't match any supported
    /// device config.
    #[error("audio stream build failed: {0}")]
    StreamBuild(String),

    /// cpal stream-play failure (system audio API rejected play()).
    #[error("audio stream play failed: {0}")]
    StreamPlay(String),

    /// I/O error while writing the captured WAV buffer to disk for
    /// the STT shell-out.
    #[error("wav write failed at {path:?}: {source}")]
    WavWrite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Whisper binary not found at the configured path. Surface
    /// includes the path so the user can fix env var or install.
    #[error(
        "whisper binary not found at {0:?} — install whisper.cpp and set ADAM_WHISPER_BIN, or pass --whisper-bin"
    )]
    WhisperBinaryMissing(PathBuf),

    /// Whisper invocation returned non-zero. stderr captured for
    /// audit so the user can see compile / model / arg errors.
    #[error("whisper invocation failed (exit {exit_code}): {stderr}")]
    WhisperFailed { exit_code: i32, stderr: String },

    /// Whisper output file missing. Means the binary ran but didn't
    /// emit the expected `--output-txt` artifact (rare; usually a
    /// model-load failure that didn't return non-zero).
    #[error("whisper transcript file missing at {0:?}")]
    WhisperOutputMissing(PathBuf),

    /// Generic I/O wrapping for spawn() / read errors.
    #[error("voice I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Hound (WAV writer) error.
    #[error("wav encoder error: {0}")]
    Hound(#[from] hound::Error),

    /// **v5.25.0** — AEC pipeline construction failure. Wraps the
    /// underlying `aec3` builder error (typically a format mismatch
    /// between render / capture configs).
    #[error("AEC pipeline build failed: {0}")]
    AecBuild(String),

    /// **v5.25.0** — AEC frame-processing failure. Wraps any runtime
    /// error from `process_render_frame` / `process_capture_frame`
    /// (format mismatch, internal queue overflow, etc.).
    #[error("AEC processing failed: {0}")]
    AecProcess(String),

    /// **v5.25.0** — Frame size mismatch on AEC input. Both render and
    /// capture frames must be exactly 10 ms at the configured sample
    /// rate; the AEC processor rejects mis-sized buffers.
    #[error("AEC frame-size mismatch: expected {expected} samples, got {actual}")]
    FrameSize { expected: usize, actual: usize },
}

pub type Result<T> = std::result::Result<T, VoiceError>;
