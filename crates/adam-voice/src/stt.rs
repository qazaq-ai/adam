//! **v5.14.0 (V0).** STT shell-out to whisper.cpp.
//!
//! ## Why shell-out, not FFI
//!
//! 1. Zero `unsafe` in adam-voice.
//! 2. whisper.cpp's C API churns across versions; pinning a binary
//!    that the user installs / builds insulates us from upstream
//!    surface changes.
//! 3. Build-time complexity stays low — no `cc::Build` /
//!    `bindgen` / link-search-path puzzle. `cargo build` works on a
//!    fresh checkout without a model file or a compiled whisper.
//! 4. The transducer boundary is text. As long as we get a
//!    transcript back, the kernel doesn't care whether STT was
//!    whisper or vosk or a remote service.
//!
//! ## Configuration
//!
//! - **Binary:** `WhisperCli::new(path)` accepts an explicit path.
//!   `WhisperCli::from_env()` reads `ADAM_WHISPER_BIN`. If the path
//!   doesn't exist on disk, `start()` returns
//!   `VoiceError::WhisperBinaryMissing` so the user sees an
//!   actionable error rather than a cryptic `ENOENT`.
//! - **Model:** the GGML model file (`ggml-base.bin` /
//!   `ggml-medium.bin` / etc.) — pass with `with_model()` or rely on
//!   the binary's default model search.
//! - **Language:** `with_language("kk")` — Whisper's Kazakh code.
//!   v5.14.0 defaults to `kk`; v5.17.0 may add auto-detect for
//!   code-switched input but the kernel-only-Kazakh directive
//!   (project_kazakh_only_directive memory) keeps `kk` as the
//!   default for now.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::error::{Result, VoiceError};

/// Default env var holding the whisper-cli binary path.
pub const ADAM_WHISPER_BIN_ENV: &str = "ADAM_WHISPER_BIN";

/// Default Whisper language code for the kernel.
pub const DEFAULT_LANGUAGE: &str = "kk";

/// Result of a Whisper invocation. Carries the transcript text plus
/// the audit metadata the v5.16.0 confidence-gate milestone will
/// consume. v5.14.0 fills in `language` and `text`; `confidence` is
/// reserved (whisper-cli's text-mode output doesn't expose per-token
/// logprobs, so v5.14.0 leaves it at `None` and v5.16.0 will switch
/// to JSON-mode invocation to populate it).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcript {
    /// The recognised text. Whitespace-trimmed; line breaks within
    /// the transcript are preserved so multi-utterance recordings
    /// stay readable.
    pub text: String,
    /// Whisper's detected (or forced) language code — typically
    /// `"kk"` for adam.
    pub language: String,
    /// Per-utterance confidence, when the engine exposes it.
    /// `None` in v5.14.0 (text-mode output); set by v5.16.0 once we
    /// move to JSON-mode invocation.
    pub confidence: Option<f32>,
    /// Raw stderr from the binary, captured for `--trace` audit.
    /// Empty in the success path on most systems but kept for
    /// diagnostics.
    pub stderr: String,
}

/// Shell-out runner for whisper.cpp's `whisper-cli` (or the legacy
/// `main` binary, both work). Configured by builder methods.
#[derive(Debug, Clone)]
pub struct WhisperCli {
    binary: PathBuf,
    model: Option<PathBuf>,
    language: String,
    threads: Option<u32>,
}

impl WhisperCli {
    /// Construct from an explicit binary path. Path is NOT validated
    /// here — that happens lazily in [`transcribe`] so test code can
    /// build a `WhisperCli` with a fake path for argv-construction
    /// assertions without touching the filesystem.
    pub fn new(binary: impl Into<PathBuf>) -> Self {
        Self {
            binary: binary.into(),
            model: None,
            language: DEFAULT_LANGUAGE.to_string(),
            threads: None,
        }
    }

    /// Construct from the `ADAM_WHISPER_BIN` env var.
    pub fn from_env() -> Option<Self> {
        std::env::var_os(ADAM_WHISPER_BIN_ENV).map(Self::new)
    }

    pub fn with_model(mut self, path: impl Into<PathBuf>) -> Self {
        self.model = Some(path.into());
        self
    }

    pub fn with_language(mut self, lang: impl Into<String>) -> Self {
        self.language = lang.into();
        self
    }

    pub fn with_threads(mut self, n: u32) -> Self {
        self.threads = Some(n);
        self
    }

    /// Build the argv as the runner would pass it to `Command::new`.
    /// Pure: no I/O, no env access — kept public-crate so unit tests
    /// can assert the exact CLI we'll invoke without spawning a
    /// process.
    pub(crate) fn build_argv(&self, wav_path: &Path) -> Vec<String> {
        let mut argv = vec![
            "-f".to_string(),
            wav_path.display().to_string(),
            "--language".to_string(),
            self.language.clone(),
            "--output-txt".to_string(),
            "--no-prints".to_string(),
        ];
        if let Some(model) = &self.model {
            argv.push("-m".to_string());
            argv.push(model.display().to_string());
        }
        if let Some(n) = self.threads {
            argv.push("--threads".to_string());
            argv.push(n.to_string());
        }
        argv
    }

    /// Transcribe a WAV file. Spawns the configured binary, waits
    /// for completion, reads the `<wav>.txt` artifact whisper.cpp
    /// emits when `--output-txt` is set, returns the [`Transcript`].
    pub fn transcribe(&self, wav_path: &Path) -> Result<Transcript> {
        if !self.binary.exists() {
            return Err(VoiceError::WhisperBinaryMissing(self.binary.clone()));
        }
        let argv = self.build_argv(wav_path);
        let output = Command::new(&self.binary).args(&argv).output()?;
        if !output.status.success() {
            return Err(VoiceError::WhisperFailed {
                exit_code: output.status.code().unwrap_or(-1),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        // whisper.cpp writes `<wav>.txt` next to the input WAV when
        // --output-txt is passed. Read it.
        let txt_path = {
            let mut p = wav_path.to_path_buf();
            let mut name = p.file_name().map(|n| n.to_os_string()).unwrap_or_default();
            name.push(".txt");
            p.set_file_name(name);
            p
        };
        if !txt_path.exists() {
            return Err(VoiceError::WhisperOutputMissing(txt_path));
        }
        let text = std::fs::read_to_string(&txt_path)?.trim().to_string();
        Ok(Transcript {
            text,
            language: self.language.clone(),
            confidence: None,
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn build_argv_minimal_v5140() {
        let cli = WhisperCli::new("/opt/whisper/whisper-cli");
        let argv = cli.build_argv(&PathBuf::from("/tmp/in.wav"));
        assert_eq!(
            argv,
            vec![
                "-f",
                "/tmp/in.wav",
                "--language",
                "kk",
                "--output-txt",
                "--no-prints",
            ]
        );
    }

    #[test]
    fn build_argv_with_model_and_threads_v5140() {
        let cli = WhisperCli::new("/opt/whisper/whisper-cli")
            .with_model("/data/models/ggml-base.bin")
            .with_threads(4);
        let argv = cli.build_argv(&PathBuf::from("/tmp/in.wav"));
        assert!(
            argv.windows(2)
                .any(|w| w == ["-m", "/data/models/ggml-base.bin"])
        );
        assert!(argv.windows(2).any(|w| w == ["--threads", "4"]));
    }

    #[test]
    fn transcribe_returns_missing_binary_error_v5140() {
        let cli = WhisperCli::new("/nonexistent/path/to/whisper");
        let r = cli.transcribe(&PathBuf::from("/tmp/in.wav"));
        match r {
            Err(VoiceError::WhisperBinaryMissing(path)) => {
                assert_eq!(path, PathBuf::from("/nonexistent/path/to/whisper"));
            }
            other => panic!("expected WhisperBinaryMissing, got {other:?}"),
        }
    }

    #[test]
    fn from_env_returns_none_when_unset_v5140() {
        // Save and clear so the test is deterministic.
        let saved = std::env::var_os(ADAM_WHISPER_BIN_ENV);
        // SAFETY: single-threaded test; restored before return.
        unsafe { std::env::remove_var(ADAM_WHISPER_BIN_ENV) };
        assert!(WhisperCli::from_env().is_none());
        if let Some(v) = saved {
            unsafe { std::env::set_var(ADAM_WHISPER_BIN_ENV, v) };
        }
    }

    #[test]
    fn with_language_overrides_default_v5140() {
        let cli = WhisperCli::new("/opt/whisper").with_language("en");
        let argv = cli.build_argv(&PathBuf::from("/tmp/in.wav"));
        assert!(argv.windows(2).any(|w| w == ["--language", "en"]));
    }
}
