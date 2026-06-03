// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Error type for the audio stack.

use thiserror::Error;

/// All errors surfaced by [`crate`].
#[derive(Debug, Error)]
pub enum AudioError {
    /// I/O error from the host (file system, etc).
    #[error("audio I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// WAV parser / encoder error from `hound`.
    #[error("WAV error: {0}")]
    Wav(#[from] hound::Error),

    /// No audio device of the requested direction (input / output)
    /// was found on the host.
    #[error("no default audio device for direction: {0}")]
    NoDevice(&'static str),

    /// `cpal` device enumeration / configuration error.
    #[error("cpal device error: {0}")]
    Device(#[from] cpal::DevicesError),

    /// `cpal` default-config query error.
    #[error("cpal default-config error: {0}")]
    DefaultConfig(#[from] cpal::DefaultStreamConfigError),

    /// `cpal` stream construction error.
    #[error("cpal stream-build error: {0}")]
    StreamBuild(#[from] cpal::BuildStreamError),

    /// `cpal` stream playback / pause error.
    #[error("cpal stream-play error: {0}")]
    StreamPlay(#[from] cpal::PlayStreamError),

    /// Mismatched sample-rate / channel-count between requested
    /// and actual stream format.
    #[error("audio config mismatch: requested {requested}, got {actual}")]
    ConfigMismatch { requested: String, actual: String },

    /// PCM data was empty when an operation required at least one
    /// sample.
    #[error("PCM buffer is empty")]
    EmptyBuffer,

    /// Recording timed out without ever seeing speech.
    #[error("recording timed out before any speech detected")]
    RecordingTimeout,
}
