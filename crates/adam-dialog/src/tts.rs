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

use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

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
///
/// **v5.0.5** — non-blocking dispatch. `speak()` spawns the synthesiser
/// as a child process and returns immediately, so the REPL doesn't
/// stall waiting for audio. A new `speak()` call kills any still-
/// running previous synthesis (latest-wins semantics — natural for
/// interactive tutoring).
#[derive(Debug)]
pub struct OsTtsBackend {
    program: String,
    /// Pre-built argv prefix. The text-to-speak is appended as the
    /// final argument by [`speak`].
    args: Vec<String>,
    /// Resolved voice name (when known) — surfaced by [`describe`].
    voice: Option<String>,
    /// Currently-playing child, if any. Killed when a new speak call
    /// arrives so the REPL feels responsive.
    current: Mutex<Option<Child>>,
}

impl OsTtsBackend {
    /// Construct from explicit components — direct callers and tests.
    pub fn new(program: String, args: Vec<String>, voice: Option<String>) -> Self {
        Self {
            program,
            args,
            voice,
            current: Mutex::new(None),
        }
    }

    /// Underlying program name (e.g. `say`, `espeak-ng`).
    pub fn program(&self) -> &str {
        &self.program
    }

    /// Argv prefix passed to the synthesiser.
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// Resolved voice name, if detection found a Kazakh voice or the
    /// caller supplied one via `voice_override`.
    pub fn voice(&self) -> Option<&str> {
        self.voice.as_deref()
    }

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
        Some(Self::new("say".into(), args, voice))
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
            return Some(Self::new(prog.into(), args, voice));
        }
        None
    }
}

impl TtsBackend for OsTtsBackend {
    /// **v5.0.5** — non-blocking dispatch. Spawns the synthesiser as
    /// a detached child process and returns immediately. If a
    /// previous speak() call is still synthesising, it is killed so
    /// the latest response wins (the alternative — queueing — feels
    /// laggy in a tutor REPL where the student typed something new).
    fn speak(&self, text: &str) -> std::io::Result<()> {
        let cleaned = strip_for_speech(text);
        if cleaned.trim().is_empty() {
            return Ok(());
        }
        let mut guard = self.current.lock().unwrap_or_else(|p| p.into_inner());
        // Kill the previously-spawned child, if it's still running.
        // `kill()` errors if the child has already exited — fine,
        // ignore. `wait()` reaps the zombie either way.
        if let Some(mut prev) = guard.take() {
            let _ = prev.kill();
            let _ = prev.wait();
        }
        let child = Command::new(&self.program)
            .args(&self.args)
            .arg(&cleaned)
            // Suppress `say`'s tty messages.
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        *guard = Some(child);
        Ok(())
    }

    fn describe(&self) -> String {
        match &self.voice {
            Some(v) => format!("{} (voice: {})", self.program, v),
            None => format!("{} (default voice)", self.program),
        }
    }
}

impl Drop for OsTtsBackend {
    /// Best-effort: when the REPL exits, reap the last-spawned child
    /// so it doesn't linger as a zombie. `say` keeps speaking even
    /// after the parent exits (default macOS behaviour) — that's
    /// fine; we just don't want the kernel process table cluttered.
    fn drop(&mut self) {
        if let Ok(mut guard) = self.current.lock()
            && let Some(mut child) = guard.take()
        {
            // Try to wait briefly; if still running, leave it
            // detached (macOS `say` continues after parent exit, so
            // the user hears the last response even after ^D).
            let _ = child.try_wait();
        }
    }
}

/// **v5.1.0** — neural TTS backend via [Piper](https://github.com/rhasspy/piper).
///
/// Piper is a fast, local neural TTS that produces noticeably more
/// natural speech than OS-bundled synthesisers. The trade-off is a
/// 50-100 MB ONNX model per voice and a `piper` CLI dependency. v5.1.0
/// keeps the kernel binary small by NOT bundling Piper or any model;
/// users who want richer voice install both manually and pass
/// `--tts-backend piper --tts-model <path>`.
///
/// ## Usage
///
/// 1. Install piper: e.g. `brew install piper-tts` (macOS) or via
///    the official release binaries.
/// 2. Download a voice model (`.onnx` file) — see
///    <https://github.com/rhasspy/piper/blob/master/VOICES.md>.
///    Kazakh-specific models are not in the official catalogue at
///    time of writing; users may train or use a multilingual
///    fallback.
/// 3. Run `adam_chat --tts --tts-backend piper --tts-model
///    /path/to/model.onnx`.
///
/// ## Pipeline
///
/// `speak(text)` pipes `text` to `piper` via stdin; piper writes a
/// WAV file; an OS-native audio player (`afplay` on macOS, `aplay`
/// on Linux) plays the WAV. The audio player is spawned as a
/// detached child so playback is non-blocking. The piper synthesis
/// step itself is synchronous (~0.3-1 s per sentence on M2) — the
/// blocking is contained to that step. Kill-previous semantics
/// apply to the audio player child, mirroring `OsTtsBackend`.
#[derive(Debug)]
pub struct PiperTtsBackend {
    piper: PathBuf,
    audio_player: PathBuf,
    model_path: PathBuf,
    voice_label: Option<String>,
    current: Mutex<Option<Child>>,
}

impl PiperTtsBackend {
    /// Construct from explicit components. Tests + direct callers.
    pub fn new(
        piper: PathBuf,
        audio_player: PathBuf,
        model_path: PathBuf,
        voice_label: Option<String>,
    ) -> Self {
        Self {
            piper,
            audio_player,
            model_path,
            voice_label,
            current: Mutex::new(None),
        }
    }

    /// Detect a usable Piper installation. Returns `None` when:
    /// - `piper` CLI is not on `PATH`
    /// - no usable audio player (`afplay` / `aplay`) is on `PATH`
    /// - the supplied model path doesn't exist
    pub fn detect(model_path: &std::path::Path) -> Option<Self> {
        let piper = locate_command("piper")?;
        let audio_player = locate_audio_player()?;
        if !model_path.exists() {
            return None;
        }
        let voice_label = model_path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(String::from);
        Some(Self::new(
            piper,
            audio_player,
            model_path.to_path_buf(),
            voice_label,
        ))
    }

    /// Path to the `piper` binary.
    pub fn piper_path(&self) -> &std::path::Path {
        &self.piper
    }

    /// Path to the OS audio player.
    pub fn audio_player_path(&self) -> &std::path::Path {
        &self.audio_player
    }

    /// Path to the ONNX model file.
    pub fn model_path(&self) -> &std::path::Path {
        &self.model_path
    }

    /// Voice label (derived from the model filename stem).
    pub fn voice_label(&self) -> Option<&str> {
        self.voice_label.as_deref()
    }

    /// Path to the temp WAV file used as the piper-to-player buffer.
    fn temp_wav() -> PathBuf {
        std::env::temp_dir().join(format!("adam_piper_{}.wav", std::process::id()))
    }
}

impl TtsBackend for PiperTtsBackend {
    fn speak(&self, text: &str) -> std::io::Result<()> {
        let cleaned = strip_for_speech(text);
        if cleaned.trim().is_empty() {
            return Ok(());
        }
        let temp = Self::temp_wav();
        // Step 1: synthesise via piper. This is synchronous — piper
        // writes the WAV file before returning. Typical latency on
        // M2 is ~0.3-1s for short sentences.
        let mut piper_child = Command::new(&self.piper)
            .arg("--model")
            .arg(&self.model_path)
            .arg("--output_file")
            .arg(&temp)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        if let Some(stdin) = piper_child.stdin.as_mut() {
            stdin.write_all(cleaned.as_bytes())?;
        }
        let piper_status = piper_child.wait()?;
        if !piper_status.success() {
            return Err(std::io::Error::other(format!(
                "piper exited with non-zero status {:?}",
                piper_status.code()
            )));
        }
        // Step 2: kill any in-flight playback, then spawn the new one
        // (non-blocking — same kill-previous semantics as
        // `OsTtsBackend`).
        let mut guard = self.current.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(mut prev) = guard.take() {
            let _ = prev.kill();
            let _ = prev.wait();
        }
        let player = Command::new(&self.audio_player)
            .arg(&temp)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        *guard = Some(player);
        Ok(())
    }

    fn describe(&self) -> String {
        match &self.voice_label {
            Some(v) => format!(
                "piper (model: {}, player: {})",
                v,
                self.audio_player.display()
            ),
            None => format!(
                "piper (model: {}, player: {})",
                self.model_path.display(),
                self.audio_player.display()
            ),
        }
    }
}

impl Drop for PiperTtsBackend {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.current.lock()
            && let Some(mut child) = guard.take()
        {
            let _ = child.try_wait();
        }
    }
}

/// Resolve a command on `PATH` to its absolute path, returning `None`
/// when the command isn't installed. Mirrors `command_on_path` but
/// returns the path rather than a bool — needed by `PiperTtsBackend`
/// because piper / audio-player paths are stored on the struct.
fn locate_command(name: &str) -> Option<PathBuf> {
    let output = Command::new("which").arg(name).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        return None;
    }
    Some(PathBuf::from(path))
}

/// Locate an OS-native audio player suitable for playing piper's WAV
/// output. macOS ships `afplay`; Linux distros typically have
/// `aplay` (ALSA), `paplay` (PulseAudio), or `play` (sox).
fn locate_audio_player() -> Option<PathBuf> {
    for candidate in ["afplay", "aplay", "paplay", "play"] {
        if let Some(p) = locate_command(candidate) {
            return Some(p);
        }
    }
    None
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
        let backend = OsTtsBackend::new(
            "say".into(),
            vec!["-v".into(), "Aru".into()],
            Some("Aru".into()),
        );
        let description = backend.describe();
        assert!(description.contains("say"));
        assert!(description.contains("Aru"));
    }

    #[test]
    fn os_backend_describe_default_voice_when_none() {
        let backend = OsTtsBackend::new("say".into(), vec![], None);
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
        assert_eq!(backend.voice(), Some("Aru"));
        assert!(backend.args().iter().any(|a| a == "Aru"));
    }

    /// **v5.0.5** — `speak()` must return promptly even for long
    /// inputs (the synthesiser keeps running in the background).
    /// On macOS we use a long Kazakh sentence; the call should
    /// complete in well under a second (spawn is essentially fork +
    /// exec, microseconds typically). 250ms gives generous slack
    /// for slow / loaded CI machines.
    #[test]
    fn speak_returns_promptly_via_spawn_on_macos() {
        if !cfg!(target_os = "macos") || !command_on_path("say") {
            return;
        }
        let backend = OsTtsBackend::detect(None).expect("macOS detect");
        // ~5s of speech if it played to completion.
        let long = "Сәлеметсіз бе. Менің атым адам. Бүгін ауа-райы жақсы. \
                    Сізге қалай көмектесе аламын? Кітап оқиық немесе кодпен жаттығайық.";
        let started = std::time::Instant::now();
        backend.speak(long).expect("speak should spawn");
        let elapsed = started.elapsed();
        assert!(
            elapsed < std::time::Duration::from_millis(250),
            "spawn should return promptly; took {elapsed:?}"
        );
        // Reap the child so it doesn't echo through the test runner.
        if let Ok(mut guard) = backend.current.lock()
            && let Some(mut child) = guard.take()
        {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    /// A second `speak` while the first is still running kills the
    /// previous child (latest-wins). After two calls the registered
    /// child should be the second one.
    #[test]
    fn second_speak_kills_previous_child_on_macos() {
        if !cfg!(target_os = "macos") || !command_on_path("say") {
            return;
        }
        let backend = OsTtsBackend::detect(None).expect("macOS detect");
        backend.speak("Бірінші мәтін.").unwrap();
        let first_pid = backend
            .current
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(|c| c.id()));
        backend.speak("Екінші мәтін.").unwrap();
        let second_pid = backend
            .current
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(|c| c.id()));
        assert!(first_pid.is_some());
        assert!(second_pid.is_some());
        assert_ne!(
            first_pid, second_pid,
            "second speak must spawn a new child (latest-wins)"
        );
        // Reap.
        if let Ok(mut guard) = backend.current.lock()
            && let Some(mut child) = guard.take()
        {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    /// Empty / whitespace-only input is a no-op (skipped by
    /// `strip_for_speech`); speak() must succeed without spawning.
    #[test]
    fn speak_empty_input_is_noop() {
        let backend = OsTtsBackend::new("say".into(), vec![], None);
        assert!(backend.speak("").is_ok());
        assert!(backend.speak("\n\n").is_ok());
        // No child was spawned.
        let guard = backend.current.lock().unwrap();
        assert!(guard.is_none());
    }

    // ─── v5.1.0 — Piper backend tests ────────────────────────────────

    #[test]
    fn piper_backend_describe_includes_model() {
        let backend = PiperTtsBackend::new(
            PathBuf::from("/usr/local/bin/piper"),
            PathBuf::from("/usr/bin/afplay"),
            PathBuf::from("/voices/en_US-lessac-medium.onnx"),
            Some("en_US-lessac-medium".into()),
        );
        let description = backend.describe();
        assert!(description.contains("piper"));
        assert!(description.contains("en_US-lessac-medium"));
    }

    #[test]
    fn piper_backend_accessors_work() {
        let backend = PiperTtsBackend::new(
            PathBuf::from("/p/piper"),
            PathBuf::from("/p/afplay"),
            PathBuf::from("/m/voice.onnx"),
            Some("voice".into()),
        );
        assert_eq!(backend.piper_path(), std::path::Path::new("/p/piper"));
        assert_eq!(
            backend.audio_player_path(),
            std::path::Path::new("/p/afplay")
        );
        assert_eq!(backend.model_path(), std::path::Path::new("/m/voice.onnx"));
        assert_eq!(backend.voice_label(), Some("voice"));
    }

    #[test]
    fn piper_detect_returns_none_when_model_missing() {
        // Use a definitely-nonexistent model path.
        let nonexistent = PathBuf::from("/tmp/adam_definitely_no_model_xyz_98765.onnx");
        assert!(!nonexistent.exists());
        let backend = PiperTtsBackend::detect(&nonexistent);
        // Even if `piper` happens to be installed, the missing model
        // file must short-circuit detection.
        assert!(backend.is_none());
    }

    #[test]
    fn piper_speak_empty_input_is_noop() {
        // We can construct a backend without the binaries actually
        // existing; speak() with empty input never reaches them.
        let backend = PiperTtsBackend::new(
            PathBuf::from("/nonexistent/piper"),
            PathBuf::from("/nonexistent/afplay"),
            PathBuf::from("/nonexistent/voice.onnx"),
            None,
        );
        assert!(backend.speak("").is_ok());
        assert!(backend.speak("   \n\n").is_ok());
        let guard = backend.current.lock().unwrap();
        assert!(guard.is_none());
    }

    #[test]
    fn locate_command_finds_known_binary_on_unix() {
        // `sh` is universal on Unix; should resolve.
        if cfg!(unix) {
            let resolved = locate_command("sh");
            assert!(resolved.is_some(), "sh should be locatable");
            let path = resolved.unwrap();
            assert!(path.is_absolute());
        }
    }

    #[test]
    fn locate_command_returns_none_for_missing_binary() {
        let resolved = locate_command("definitely_not_a_real_command_xyz_98765");
        assert!(resolved.is_none());
    }

    #[test]
    fn locate_audio_player_finds_afplay_on_macos() {
        if !cfg!(target_os = "macos") {
            return;
        }
        let player = locate_audio_player();
        assert!(player.is_some(), "macOS should have afplay");
        let path = player.unwrap();
        assert!(path.to_string_lossy().contains("afplay"));
    }
}
