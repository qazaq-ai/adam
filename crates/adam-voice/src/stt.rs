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
///
/// **v5.16.0 (V2).** Defaults to JSON-mode invocation
/// (`--output-json`) so we can extract per-segment `avg_logprob`
/// and surface a confidence score in [`Transcript::confidence`].
/// Pre-v5.16.0 the runner used text-mode (`--output-txt`) and
/// `confidence` was always `None`.
#[derive(Debug, Clone)]
pub struct WhisperCli {
    binary: PathBuf,
    model: Option<PathBuf>,
    language: String,
    threads: Option<u32>,
    json_mode: bool,
    prompt: Option<String>,
}

/// **v5.19.0 (V3).** Default Whisper `--prompt` priming for Kazakh.
/// Whisper-medium consistently mishears Kazakh-specific phonemes —
/// «Сәлем» → «Салим» (drops `қ`, adds Arabic-style `и`), «Дәулет» →
/// «дау лет» (splits ә-vowel + word boundary), «Танысайық» →
/// «Танысайыр» (drops `ң`, adds artifact). Priming the decoder with
/// a short Kazakh sentence sets the prior for these exact words.
///
/// The priming list is intentionally short: every additional token
/// in the prompt cuts into the 224-token context window Whisper
/// reserves for the actual transcription. Each word here is one
/// that we've observed Whisper mishear in real live-test sessions.
pub const KAZAKH_PRIMING_PROMPT: &str = "Қазақ тілінде сөйлеу. Сәлем. Сау бол. Менің атым Дәулет. \
     Танысайық. Сіздің атыңыз кім? Мен Алматыда тұрамын. Маған 25 жас.";

impl WhisperCli {
    /// Construct from an explicit binary path. Path is NOT validated
    /// here — that happens lazily in [`transcribe`] so test code can
    /// build a `WhisperCli` with a fake path for argv-construction
    /// assertions without touching the filesystem.
    ///
    /// **v5.19.0 (V3):** Constructor wires the Kazakh priming prompt
    /// by default. Callers that want a different prompt (or no
    /// prompt) override via [`with_prompt`].
    pub fn new(binary: impl Into<PathBuf>) -> Self {
        Self {
            binary: binary.into(),
            model: None,
            language: DEFAULT_LANGUAGE.to_string(),
            threads: None,
            json_mode: true,
            prompt: Some(KAZAKH_PRIMING_PROMPT.to_string()),
        }
    }

    /// **v5.19.0 (V3).** Override the `--prompt` text passed to
    /// whisper-cli. `None` clears the prompt entirely (use only for
    /// non-Kazakh test scenarios — the default Kazakh priming closes
    /// many of the medium-model mishearings observed in live tests).
    pub fn with_prompt(mut self, prompt: Option<String>) -> Self {
        self.prompt = prompt;
        self
    }

    /// **v5.16.0 (V2).** Force text-mode (`--output-txt`) instead of
    /// the default JSON mode. `confidence` will be `None` in the
    /// resulting [`Transcript`]; useful for testing the legacy code
    /// path or when whisper-cli's JSON output is broken on the
    /// installed binary.
    pub fn with_text_mode(mut self) -> Self {
        self.json_mode = false;
        self
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
            "--no-prints".to_string(),
        ];
        if self.json_mode {
            argv.push("--output-json".to_string());
        } else {
            argv.push("--output-txt".to_string());
        }
        if let Some(model) = &self.model {
            argv.push("-m".to_string());
            argv.push(model.display().to_string());
        }
        if let Some(n) = self.threads {
            argv.push("--threads".to_string());
            argv.push(n.to_string());
        }
        if let Some(prompt) = &self.prompt {
            argv.push("--prompt".to_string());
            argv.push(prompt.clone());
        }
        argv
    }

    /// Transcribe a WAV file. Spawns the configured binary, waits
    /// for completion, reads either the `<wav>.json` or `<wav>.txt`
    /// artifact whisper.cpp emits (depending on `json_mode`), and
    /// returns the [`Transcript`]. JSON mode populates
    /// `Transcript::confidence` from per-segment `avg_logprob`;
    /// text mode leaves it `None`.
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
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        if self.json_mode {
            let json_path = sibling_path_with_extra_ext(wav_path, "json");
            if !json_path.exists() {
                return Err(VoiceError::WhisperOutputMissing(json_path));
            }
            let raw = std::fs::read_to_string(&json_path)?;
            let (raw_text, confidence) = parse_whisper_json(&raw);
            // **v5.19.0 (V3).** Apply the Kazakh transcript normalizer
            // before returning. Only fires when language is the Kazakh
            // default — for non-Kazakh test scenarios the normalizer
            // would be a no-op for cleanly Kazakh text but might
            // mis-handle Russian/English transcripts that happen to
            // contain similar-looking Cyrillic tokens.
            let text = if self.language == DEFAULT_LANGUAGE {
                crate::normalizer::normalize_kazakh_transcript(&raw_text)
            } else {
                raw_text
            };
            Ok(Transcript {
                text,
                language: self.language.clone(),
                confidence,
                stderr,
            })
        } else {
            let txt_path = sibling_path_with_extra_ext(wav_path, "txt");
            if !txt_path.exists() {
                return Err(VoiceError::WhisperOutputMissing(txt_path));
            }
            let raw_text = std::fs::read_to_string(&txt_path)?.trim().to_string();
            let text = if self.language == DEFAULT_LANGUAGE {
                crate::normalizer::normalize_kazakh_transcript(&raw_text)
            } else {
                raw_text
            };
            Ok(Transcript {
                text,
                language: self.language.clone(),
                confidence: None,
                stderr,
            })
        }
    }
}

/// `<wav>.txt` / `<wav>.json` — whisper.cpp appends the artifact
/// extension to the *full* input file name, not to its stem. So
/// `/tmp/foo.wav` produces `/tmp/foo.wav.txt`, not `/tmp/foo.txt`.
fn sibling_path_with_extra_ext(wav_path: &Path, ext: &str) -> PathBuf {
    let mut p = wav_path.to_path_buf();
    let mut name = p.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(".");
    name.push(ext);
    p.set_file_name(name);
    p
}

/// **v5.16.0 (V2).** Parse whisper.cpp `--output-json` artefact and
/// extract `(text, confidence)`. `confidence` is the duration-
/// weighted geometric mean of per-segment `exp(avg_logprob)` —
/// approximates the average probability per token across the whole
/// utterance. Returns `(text, None)` when no segments carry
/// `avg_logprob` (older whisper.cpp builds, or extreme edge cases).
///
/// Pure / no I/O — kept crate-public so tests can inject synthetic
/// JSON without spawning whisper.
pub(crate) fn parse_whisper_json(raw: &str) -> (String, Option<f32>) {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(raw) else {
        return (String::new(), None);
    };
    let segments = root
        .get("transcription")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut texts: Vec<String> = Vec::new();
    let mut logprob_sum: f64 = 0.0;
    let mut weight_sum: f64 = 0.0;
    for seg in &segments {
        if let Some(t) = seg.get("text").and_then(|v| v.as_str()) {
            texts.push(t.trim().to_string());
        }
        let logp = seg.get("avg_logprob").and_then(|v| v.as_f64());
        let from = seg
            .get("offsets")
            .and_then(|o| o.get("from"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let to = seg
            .get("offsets")
            .and_then(|o| o.get("to"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let duration = (to - from).max(1) as f64;
        if let Some(lp) = logp {
            logprob_sum += lp * duration;
            weight_sum += duration;
        }
    }
    let text = texts
        .iter()
        .filter(|s| !s.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    let confidence = if weight_sum > 0.0 {
        let avg_logprob = logprob_sum / weight_sum;
        Some(avg_logprob.exp() as f32)
    } else {
        None
    };
    (text.trim().to_string(), confidence)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn build_argv_minimal_v5140() {
        // **v5.16.0** — default mode is now JSON (was text in V0/V1).
        // **v5.19.0 (V3)** — Kazakh priming prompt added by default.
        let cli = WhisperCli::new("/opt/whisper/whisper-cli");
        let argv = cli.build_argv(&PathBuf::from("/tmp/in.wav"));
        assert_eq!(
            argv,
            vec![
                "-f",
                "/tmp/in.wav",
                "--language",
                "kk",
                "--no-prints",
                "--output-json",
                "--prompt",
                KAZAKH_PRIMING_PROMPT,
            ]
        );
    }

    #[test]
    fn build_argv_with_prompt_none_drops_prompt_arg_v5190() {
        let cli = WhisperCli::new("/opt/whisper/whisper-cli").with_prompt(None);
        let argv = cli.build_argv(&PathBuf::from("/tmp/in.wav"));
        assert!(!argv.iter().any(|a| a == "--prompt"));
    }

    #[test]
    fn build_argv_with_custom_prompt_v5190() {
        let cli = WhisperCli::new("/opt/whisper/whisper-cli")
            .with_prompt(Some("custom test prompt".to_string()));
        let argv = cli.build_argv(&PathBuf::from("/tmp/in.wav"));
        assert!(argv.iter().any(|a| a == "custom test prompt"));
    }

    #[test]
    fn build_argv_text_mode_v5160() {
        let cli = WhisperCli::new("/opt/whisper/whisper-cli").with_text_mode();
        let argv = cli.build_argv(&PathBuf::from("/tmp/in.wav"));
        assert!(argv.iter().any(|a| a == "--output-txt"));
        assert!(!argv.iter().any(|a| a == "--output-json"));
    }

    #[test]
    fn parse_whisper_json_extracts_text_and_confidence_v5160() {
        // Synthetic JSON shaped after whisper.cpp 1.8.4 output.
        let raw = r#"{
            "result": {"language": "kk"},
            "transcription": [
                {
                    "text": "Сәлеметсіз",
                    "avg_logprob": -0.1,
                    "offsets": {"from": 0, "to": 1000}
                },
                {
                    "text": "бе",
                    "avg_logprob": -0.2,
                    "offsets": {"from": 1000, "to": 1500}
                }
            ]
        }"#;
        let (text, conf) = parse_whisper_json(raw);
        assert_eq!(text, "Сәлеметсіз бе");
        // Weighted avg logprob = (-0.1*1000 + -0.2*500) / 1500 ≈ -0.133
        // exp(-0.133) ≈ 0.875
        let c = conf.expect("confidence populated");
        assert!(c > 0.85 && c < 0.90, "confidence ≈ 0.87, got {c}");
    }

    #[test]
    fn parse_whisper_json_missing_logprobs_yields_none_confidence_v5160() {
        let raw = r#"{
            "transcription": [
                {"text": "Сәлем", "offsets": {"from": 0, "to": 500}}
            ]
        }"#;
        let (text, conf) = parse_whisper_json(raw);
        assert_eq!(text, "Сәлем");
        assert_eq!(conf, None);
    }

    #[test]
    fn parse_whisper_json_malformed_returns_empty_v5160() {
        let raw = "this is not json";
        let (text, conf) = parse_whisper_json(raw);
        assert_eq!(text, "");
        assert_eq!(conf, None);
    }

    #[test]
    fn parse_whisper_json_empty_transcription_v5160() {
        let raw = r#"{"transcription": []}"#;
        let (text, conf) = parse_whisper_json(raw);
        assert_eq!(text, "");
        assert_eq!(conf, None);
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
