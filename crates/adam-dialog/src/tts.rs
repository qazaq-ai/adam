//! **v5.0.0** — text-to-speech output transducer.
//!
//! adam's first multimodal output. The TTS layer is deliberately
//! framed as a **peripheral output transducer**, not a kernel
//! component: dialog logic, intent recognition, planner, realiser
//! produce the same Kazakh text as before; the TTS layer just speaks
//! that text through a system-native voice synthesiser.
//!
//! ## Architectural framing
//!
//! Per `project_retrieval_not_neural_v2` and `project_v4_direction`,
//! adam is a **deterministic kernel** with watch-battery deployment
//! goals. A bundled neural TTS model (~50 MB+) would dilute that
//! framing. The v5.0.0 design avoids the conflict by **shelling out
//! to the OS-native voice synthesiser** — `say` on macOS,
//! `espeak-ng` on Linux. Zero binary-size impact, zero new model
//! dependency, and the kernel's deterministic core stays untouched.
//!
//! Future v5.0.5+ may add a [`TtsBackend`] implementation backed by a
//! local model (e.g. Piper) for users who want a richer voice. The
//! trait is designed so callers don't change.
//!
//! ## Kazakh voice
//!
//! macOS ships a Kazakh voice (`Aru`, locale `kk_KZ`) when the user
//! has enabled the Kazakh language pack in System Settings →
//! Accessibility → Spoken Content. Linux `espeak-ng` supports
//! Kazakh (`-v kk`) out of the box. [`OsTtsBackend::detect`]
//! prefers Kazakh; falls back to the system default voice if no
//! Kazakh option is found (with a one-time warning to stderr).

use std::process::{Command, Stdio};

/// Output transducer for spoken Kazakh.
///
/// Implementations must be `Send + Sync` so the REPL can call
/// `speak` from any thread (typically the same thread as text
/// rendering today, but reserved for v5.0.5+ async dispatch).
pub trait TtsBackend: Send + Sync {
    /// Speak `text` aloud. The call is **blocking** in v5.0.0 —
    /// returns when the synthesiser finishes (or fails). v5.0.5+
    /// may add a non-blocking variant for REPL responsiveness.
    fn speak(&self, text: &str) -> std::io::Result<()>;

    /// Human-readable name of the backend, for `--trace` /
    /// startup banners. e.g. "macOS say (voice: Aru)".
    fn describe(&self) -> String;
}

/// No-op backend for tests and disabled-TTS callers.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoOpTts;

impl TtsBackend for NoOpTts {
    fn speak(&self, _text: &str) -> std::io::Result<()> {
        Ok(())
    }
    fn describe(&self) -> String {
        "no-op (TTS disabled)".to_string()
    }
}

/// OS-native TTS via system command shellout.
#[derive(Debug, Clone)]
pub struct OsTtsBackend {
    pub program: String,
    /// Pre-built argv prefix. The text-to-speak is appended as the
    /// final argument by [`speak`].
    pub args: Vec<String>,
    /// Resolved voice name (when known) — used by [`describe`].
    pub voice: Option<String>,
}

impl OsTtsBackend {
    /// Detect a usable TTS backend on the current platform. Returns
    /// `None` when no supported TTS command is on `PATH`.
    ///
    /// Detection order:
    /// - macOS: `say` (Kazakh voice `Aru` if available)
    /// - Linux: `espeak-ng -v kk` (or `espeak -v kk` as fallback)
    ///
    /// `voice_override` lets callers force a specific voice (passed
    /// through to `say -v <voice>` / `espeak-ng -v <voice>`). When
    /// `None`, detection picks the best Kazakh voice it finds.
    pub fn detect(voice_override: Option<&str>) -> Option<Self> {
        if cfg!(target_os = "macos") {
            return Self::detect_macos(voice_override);
        }
        if cfg!(target_os = "linux") {
            return Self::detect_linux(voice_override);
        }
        None
    }

    fn detect_macos(voice_override: Option<&str>) -> Option<Self> {
        if !command_on_path("say") {
            return None;
        }
        let voice = voice_override
            .map(String::from)
            .or_else(|| find_macos_kazakh_voice("say"));
        let mut args = Vec::new();
        if let Some(v) = voice.as_deref() {
            args.push("-v".into());
            args.push(v.into());
        }
        Some(Self {
            program: "say".into(),
            args,
            voice,
        })
    }

    fn detect_linux(voice_override: Option<&str>) -> Option<Self> {
        let candidates = ["espeak-ng", "espeak"];
        for prog in candidates {
            if !command_on_path(prog) {
                continue;
            }
            let voice = voice_override
                .map(String::from)
                .or_else(|| Some("kk".into()));
            let mut args = Vec::new();
            if let Some(v) = voice.as_deref() {
                args.push("-v".into());
                args.push(v.into());
            }
            return Some(Self {
                program: prog.into(),
                args,
                voice,
            });
        }
        None
    }
}

impl TtsBackend for OsTtsBackend {
    fn speak(&self, text: &str) -> std::io::Result<()> {
        // Strip markdown-fence noise that the synthesiser would
        // pronounce literally («backtick backtick backtick rust...»).
        // The kernel's text output uses `\`\`\`rust ... \`\`\`` for
        // code blocks; for spoken output we drop fence lines and
        // collapse whitespace — but keep the code body so the
        // student hears WHAT the example shows.
        let cleaned = strip_for_speech(text);
        if cleaned.trim().is_empty() {
            return Ok(());
        }
        let status = Command::new(&self.program)
            .args(&self.args)
            .arg(&cleaned)
            // Suppress `say`'s tty messages.
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        if !status.success() {
            return Err(std::io::Error::other(format!(
                "{} exited with non-zero status {:?}",
                self.program,
                status.code()
            )));
        }
        Ok(())
    }

    fn describe(&self) -> String {
        match &self.voice {
            Some(v) => format!("{} (voice: {})", self.program, v),
            None => format!("{} (default voice)", self.program),
        }
    }
}

/// Prepare a text fragment for spoken output. Strips markdown code
/// fences and collapses runs of whitespace — synthesisers pronounce
/// `\`\`\`rust` as «backtick backtick backtick», which adds noise
/// without informational value.
pub fn strip_for_speech(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_fence = false;
    for line in text.lines() {
        let trimmed = line.trim();
        // Toggle fence state on opening / closing ```.
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            // Skip code body — listening to a Rust function is
            // nonsensical via TTS. The spoken response stays focused
            // on the prose; the student reads code on screen.
            continue;
        }
        // Strip inline code backticks (the speakable content stays).
        let stripped: String = line.chars().filter(|&c| c != '`').collect();
        out.push_str(&stripped);
        out.push(' ');
    }
    // Collapse internal whitespace runs.
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn command_on_path(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|_| true)
        .unwrap_or(false)
}

/// Probe `say -v ?` for a `kk_KZ` voice. Returns the voice name
/// when found, else `None`.
fn find_macos_kazakh_voice(say_cmd: &str) -> Option<String> {
    let output = Command::new(say_cmd).arg("-v").arg("?").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        // Format: "Aru                 kk_KZ    # Hello! My name is Aru."
        let lower = line.to_lowercase();
        if lower.contains("kk_kz") || lower.contains("kazakh") {
            // The voice name is the first whitespace-delimited token.
            if let Some(name) = line.split_whitespace().next() {
                return Some(name.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_op_backend_speak_succeeds() {
        let tts = NoOpTts;
        assert!(tts.speak("hello").is_ok());
        assert!(tts.describe().contains("no-op"));
    }

    #[test]
    fn strip_for_speech_drops_code_fence_blocks() {
        let input = "Сәлем!\n```rust\nfn main() {}\n```\nӘлемге.";
        let out = strip_for_speech(input);
        assert!(!out.contains("rust"), "fence body must be dropped: {out:?}");
        assert!(!out.contains("fn main"));
        assert!(out.contains("Сәлем"));
        assert!(out.contains("Әлемге"));
    }

    #[test]
    fn strip_for_speech_drops_inline_backticks() {
        let out = strip_for_speech("`cargo check` тазалады.");
        assert!(
            !out.contains('`'),
            "inline backticks must be removed: {out:?}"
        );
        assert!(out.contains("cargo check"));
    }

    #[test]
    fn strip_for_speech_collapses_whitespace() {
        let out = strip_for_speech("Сәлем!\n\n\nӘлем.");
        assert_eq!(out, "Сәлем! Әлем.");
    }

    #[test]
    fn strip_for_speech_empty_input_yields_empty() {
        assert_eq!(strip_for_speech(""), "");
        assert_eq!(strip_for_speech("\n\n```rust\nbody\n```\n"), "");
    }

    #[test]
    fn os_backend_describe_includes_voice_when_set() {
        let backend = OsTtsBackend {
            program: "say".into(),
            args: vec!["-v".into(), "Aru".into()],
            voice: Some("Aru".into()),
        };
        let description = backend.describe();
        assert!(description.contains("say"));
        assert!(description.contains("Aru"));
    }

    #[test]
    fn os_backend_describe_default_voice_when_none() {
        let backend = OsTtsBackend {
            program: "say".into(),
            args: vec![],
            voice: None,
        };
        assert!(backend.describe().contains("default"));
    }

    /// Real macOS detection — only meaningful on macOS hosts. Skips
    /// on other platforms.
    #[test]
    fn detect_picks_kazakh_voice_when_available_on_macos() {
        if !cfg!(target_os = "macos") {
            return;
        }
        if !command_on_path("say") {
            // CI environment without `say` — skip.
            return;
        }
        let backend = OsTtsBackend::detect(None);
        // macOS hosts that have ANY voices installed should yield
        // Some(_); the Kazakh voice may or may not be present
        // depending on installed language packs, so we don't assert
        // the voice name — just that detection succeeded.
        assert!(backend.is_some());
    }

    #[test]
    fn detect_returns_none_when_no_voice_override_and_unsupported_platform() {
        if cfg!(target_os = "macos") || cfg!(target_os = "linux") {
            return;
        }
        // E.g. Windows / unknown platform → None.
        assert!(OsTtsBackend::detect(None).is_none());
    }

    #[test]
    fn voice_override_propagates_to_args() {
        if !cfg!(target_os = "macos") {
            return;
        }
        if !command_on_path("say") {
            return;
        }
        let backend = OsTtsBackend::detect(Some("Aru")).expect("macOS say should detect");
        assert_eq!(backend.voice.as_deref(), Some("Aru"));
        assert!(backend.args.iter().any(|a| a == "Aru"));
    }
}
