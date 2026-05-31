// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! **Voice REPL v6.3** — the full Whisper-free Kazakh voice
//! loop, standalone.
//!
//! ```text
//!   microphone (cpal via adam-audio::record)
//!     ↓ PCM mono 16 kHz
//!     ↓ adam-audio::mfcc
//!     ↓ adam-stt-phoneme::recognise_word + rescore
//!   Vec<Phoneme>
//!     ↓ adam-phoneme::cyrillic::phonemes_to_cyrillic
//!   Cyrillic transcript
//!     ↓ adam-tts-phoneme::synthesise_with_bank
//!     ↓ adam-audio::play
//!   speaker
//! ```
//!
//! ## Usage
//!
//! ```sh
//! # Record 3 s, recognise, print, optionally speak back:
//! cargo run --release -p adam-voice-repl-v6-3 -- --duration 3 --speak
//!
//! # Loop mode (press Enter to record, ^C to quit):
//! cargo run --release -p adam-voice-repl-v6-3 -- --loop
//! ```
//!
//! ## Banks
//!
//! Loads `data/v6_3_phoneme_bank/templates.bin` (MFCC for STT)
//! and `pcm_templates.bin` (PCM for TTS) when present. Both
//! are merged with synth fallback covering uncovered phonemes.

use adam_audio::play::play_blocking;
use adam_audio::record::{RecordConfig, record_fixed_duration, record_until_silence};
use adam_audio::wav::write_wav;
use adam_phoneme::cyrillic::phonemes_to_cyrillic;
use adam_stt_phoneme::{PhonemeBank, WordConfig, recognise_word, rescore};
use adam_tts_phoneme::{PcmBank, TtsConfig, synthesise_with_bank};
use clap::Parser;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Parser)]
#[command(
    name = "voice_repl_v6_3",
    about = "End-to-end v6.3 voice loop: mic → phoneme STT → Cyrillic → phoneme TTS → speaker.",
    version
)]
struct Args {
    /// Fixed recording duration in seconds. When set, --vad is
    /// ignored.
    #[arg(long)]
    duration: Option<u64>,
    /// Use VAD auto-stop (recording stops on 1.5 s of silence).
    /// Default 30 s hard cap.
    #[arg(long)]
    vad: bool,
    /// Loop mode — press Enter to start each recording, ^C to
    /// quit.
    #[arg(long, name = "loop")]
    loop_mode: bool,
    /// Synthesise the recognised phoneme stream back through
    /// the TTS bank and play it.
    #[arg(long)]
    speak: bool,
    /// Optional: save each recording to a WAV file (debugging).
    #[arg(long)]
    save_wav: Option<PathBuf>,
    /// Bank directory.
    #[arg(long, default_value = "data/v6_3_phoneme_bank")]
    bank_dir: PathBuf,
    /// TTS backend. `concat` = the in-tree concatenative
    /// pipeline (PcmBank + parametric fallback, Phase 7).
    /// `piper` = subprocess to the Piper neural TTS CLI using
    /// the ISSAI kk_KZ-issai-high voice from Phase 13.
    /// Piper produces single-speaker, sentence-level, natural-
    /// sounding output and is the recommended default once the
    /// model + venv are present.
    #[arg(long, default_value = "piper")]
    tts_backend: String,
    /// Piper voice .onnx path (used only when --tts-backend=piper).
    #[arg(long, default_value = "data/tts_models/kk_KZ-issai-high.onnx")]
    piper_model: PathBuf,
    /// Path to the Python venv where `piper-tts` is installed
    /// (built per tools/synthesize_piper/README.md).
    #[arg(long, default_value = "data/tts_models/.venv")]
    piper_venv: PathBuf,
    /// STT backend. `dtw` = the in-tree DTW phoneme recogniser
    /// (Phase 13 «human bank v4», FLEURS PER ≈ 76.6 %).
    /// `whisper` = subprocess to whisper.cpp's `whisper-cli`
    /// CLI using a multilingual ggml model — dramatically
    /// better word-level accuracy on natural Kazakh speech.
    /// Default `whisper` once the model is present; auto-falls
    /// back to `dtw` if model/binary missing.
    #[arg(long, default_value = "whisper")]
    stt_backend: String,
    /// Whisper ggml model path (used only when --stt-backend=whisper).
    #[arg(long, default_value = "data/stt_models/ggml-small.bin")]
    whisper_model: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Load banks (real templates + synth fallback hybrid).
    let mfcc_bank = load_mfcc_bank(&args.bank_dir, 16_000)?;
    let pcm_bank = load_pcm_bank(&args.bank_dir).ok();
    println!(
        "[voice-repl] MFCC bank covers {} phonemes; PCM bank {}",
        mfcc_bank.len(),
        pcm_bank
            .as_ref()
            .map(|b| format!("covers {} phonemes", b.len()))
            .unwrap_or_else(|| "absent (synth-only TTS)".into()),
    );

    let single = !args.loop_mode;
    loop {
        if args.loop_mode {
            println!("[voice-repl] press Enter to record, ^C to quit");
            let mut buf = String::new();
            if std::io::stdin().read_line(&mut buf).is_err() || buf.is_empty() {
                break;
            }
        }

        let pcm = if let Some(d) = args.duration {
            println!("[voice-repl] recording {d} s...");
            record_fixed_duration(Duration::from_secs(d))?
        } else if args.vad {
            println!("[voice-repl] recording (auto-stop on 1.5 s of silence, 30 s max)...");
            record_until_silence(RecordConfig::default())?
        } else {
            println!("[voice-repl] recording 3 s (default)...");
            record_fixed_duration(Duration::from_secs(3))?
        };
        println!(
            "[voice-repl] captured {:.2} s at {} Hz",
            pcm.duration_s(),
            pcm.sample_rate
        );

        if let Some(path) = &args.save_wav {
            write_wav(path, &pcm)?;
            println!("[voice-repl] saved {}", path.display());
        }

        // STT: recognise. Whisper backend takes the WAV directly and
        // returns Cyrillic text; DTW backend produces a phoneme
        // stream that's rendered via phonemes_to_cyrillic.
        // `rescored` is also needed below for the concat-TTS fallback
        // path, so we always run the DTW recogniser for that pathway
        // and use whisper's Cyrillic when --stt-backend=whisper.
        let raw = recognise_word(
            &pcm.data,
            pcm.sample_rate,
            &mfcc_bank,
            &WordConfig::default(),
        );
        let rescored = rescore(&raw);
        let dtw_cyrillic = phonemes_to_cyrillic(&rescored);

        let cyrillic = match args.stt_backend.as_str() {
            "whisper" => match transcribe_via_whisper(&pcm, &args.whisper_model) {
                Ok(text) => {
                    println!("[voice-repl] whisper transcribed: «{text}»");
                    text
                }
                Err(e) => {
                    eprintln!("[voice-repl] whisper backend failed: {e} — falling back to DTW",);
                    println!("[voice-repl] dtw phonemes (raw):       {raw:?}");
                    println!("[voice-repl] dtw phonemes (rescored):  {rescored:?}");
                    println!("[voice-repl] dtw cyrillic:             «{dtw_cyrillic}»");
                    dtw_cyrillic.clone()
                }
            },
            _ => {
                println!("[voice-repl] dtw phonemes (raw):       {raw:?}");
                println!("[voice-repl] dtw phonemes (rescored):  {rescored:?}");
                println!("[voice-repl] dtw cyrillic:             «{dtw_cyrillic}»");
                dtw_cyrillic.clone()
            }
        };

        // Optional TTS playback.
        if args.speak {
            let tts_out = match args.tts_backend.as_str() {
                "piper" => {
                    match synthesise_via_piper(&cyrillic, &args.piper_model, &args.piper_venv) {
                        Ok(pcm) => {
                            println!(
                                "[voice-repl] piper synthesised {:.2} s @ {} Hz, playing back",
                                pcm.duration_s(),
                                pcm.sample_rate,
                            );
                            pcm
                        }
                        Err(e) => {
                            eprintln!(
                                "[voice-repl] piper backend failed: {e} — falling back to concat",
                            );
                            let pcm = synthesise_with_bank(
                                &rescored,
                                pcm_bank.as_ref(),
                                &TtsConfig::default(),
                            );
                            println!(
                                "[voice-repl] concat synthesised {:.2} s, playing back",
                                pcm.duration_s(),
                            );
                            pcm
                        }
                    }
                }
                _ => {
                    let pcm =
                        synthesise_with_bank(&rescored, pcm_bank.as_ref(), &TtsConfig::default());
                    println!(
                        "[voice-repl] concat synthesised {:.2} s, playing back",
                        pcm.duration_s(),
                    );
                    pcm
                }
            };
            play_blocking(&tts_out)?;
        }

        if single {
            break;
        }
        println!();
    }

    Ok(())
}

fn load_mfcc_bank(bank_dir: &std::path::Path, sample_rate: u32) -> std::io::Result<PhonemeBank> {
    let path = bank_dir.join("templates.bin");
    let synth = PhonemeBank::synthetic(sample_rate);
    if path.exists() {
        match PhonemeBank::load_from_file(&path) {
            Ok(real) => Ok(real.merged_with_fallback(&synth)),
            Err(e) => {
                eprintln!(
                    "[voice-repl] WARN: failed to load {}: {} — using synth-only bank",
                    path.display(),
                    e,
                );
                Ok(synth)
            }
        }
    } else {
        eprintln!(
            "[voice-repl] note: no real MFCC bank at {} — using synth-only",
            path.display()
        );
        Ok(synth)
    }
}

fn load_pcm_bank(bank_dir: &std::path::Path) -> Result<PcmBank, Box<dyn std::error::Error>> {
    let path = bank_dir.join("pcm_templates.bin");
    Ok(PcmBank::load_from_file(path)?)
}

/// Phase 13 (2026-05-31) Piper neural TTS backend.
///
/// Shells out to the Piper CLI installed in the Python venv at
/// `venv_path`. Input is the recognised Cyrillic text with the
/// canonical formatting we tuned during the listening session:
/// **capitalise first letter + append "."** so the model gets a
/// proper sentence shape (otherwise short utterances lose their
/// initial consonant). Output is a 22 050 Hz mono WAV which
/// `play_blocking` accepts directly.
///
/// This is the «integration bridge» implementation. Phase 13b
/// will replace it with a pure-Rust `tract-onnx` adapter that
/// runs the same model in-process, eliminating the Python +
/// onnxruntime C++ dependencies at runtime.
fn synthesise_via_piper(
    cyrillic_text: &str,
    model_path: &std::path::Path,
    venv_path: &std::path::Path,
) -> Result<adam_audio::PcmSamples, Box<dyn std::error::Error>> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    if !model_path.exists() {
        return Err(format!(
            "piper model not found at {} — fetch via the URL in .gitignore",
            model_path.display()
        )
        .into());
    }
    let piper_bin = venv_path.join("bin/piper");
    if !piper_bin.exists() {
        return Err(format!(
            "piper CLI not found at {} — set up the venv per tools/synthesize_piper/README.md",
            piper_bin.display()
        )
        .into());
    }

    // Canonical sentence shape: capitalise first char + trailing period.
    let trimmed = cyrillic_text.trim();
    if trimmed.is_empty() {
        return Err("piper backend: empty input text".into());
    }
    let mut chars = trimmed.chars();
    let head = chars.next().unwrap();
    let head_upper: String = head.to_uppercase().collect();
    let cap = format!("{head_upper}{}", chars.as_str());
    let needs_period = !cap.ends_with(['.', '!', '?']);
    let prompt = if needs_period { format!("{cap}.") } else { cap };

    let tmp_dir = std::env::temp_dir();
    let pid = std::process::id();
    let tmp_out = tmp_dir.join(format!("voice_repl_piper_{pid}.wav"));

    let mut child = Command::new(&piper_bin)
        .arg("--model")
        .arg(model_path)
        .arg("--length-scale")
        .arg("1.0")
        .arg("--sentence-silence")
        .arg("0.2")
        .arg("--output-file")
        .arg(&tmp_out)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(prompt.as_bytes())?;
    }
    let output = child.wait_with_output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "piper exited with {:?}: {}",
            output.status.code(),
            stderr.trim()
        )
        .into());
    }
    if !tmp_out.exists() {
        return Err(format!("piper produced no output at {}", tmp_out.display()).into());
    }
    let pcm = adam_audio::wav::read_wav(&tmp_out)?;
    let _ = std::fs::remove_file(&tmp_out);
    Ok(pcm)
}

/// Phase 15 (2026-05-31) whisper.cpp STT backend.
///
/// Shells out to the `whisper-cli` binary (brew install
/// whisper-cpp) with a ggml-format multilingual model and the
/// Kazakh language hint. Input is the recorded `PcmSamples`;
/// we write it as a 16 kHz mono WAV in `/tmp/` (whisper-cli
/// reads from disk), invoke whisper-cli, strip its progress
/// banner from the stdout, and return the recognised Cyrillic
/// text.
///
/// Pairs symmetrically with `synthesise_via_piper`: both are
/// subprocess wrappers around well-maintained C/C++ neural
/// runtimes that ship as `brew` binaries. Phase 13b will
/// replace BOTH with pure-Rust `tract-onnx` adapters that run
/// the same models in-process.
fn transcribe_via_whisper(
    pcm: &adam_audio::PcmSamples,
    model_path: &std::path::Path,
) -> Result<String, Box<dyn std::error::Error>> {
    use std::process::Command;

    if !model_path.exists() {
        return Err(format!(
            "whisper ggml model not found at {} — fetch with:\n  \
             curl -L -o {} https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
            model_path.display(),
            model_path.display()
        )
        .into());
    }
    // whisper-cli expects 16 kHz mono WAV on disk.
    let tmp_dir = std::env::temp_dir();
    let pid = std::process::id();
    let tmp_wav = tmp_dir.join(format!("voice_repl_whisper_in_{pid}.wav"));
    adam_audio::wav::write_wav(&tmp_wav, pcm)?;

    let output = Command::new("whisper-cli")
        .arg("-m")
        .arg(model_path)
        .arg("-l")
        .arg("kk")
        .arg("-f")
        .arg(&tmp_wav)
        .arg("-nt") // no timestamps
        .arg("--print-progress")
        .arg("false")
        .stderr(std::process::Stdio::piped())
        .output()?;

    let _ = std::fs::remove_file(&tmp_wav);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "whisper-cli exited with {:?}: {}",
            output.status.code(),
            stderr.trim()
        )
        .into());
    }

    // Parse stdout: whisper-cli prints lots of init noise before
    // the transcription. The transcription itself is on lines
    // that aren't bracket/log-prefixed. Keep the longest non-log
    // line as the transcript.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut best: String = String::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty()
            || line.starts_with('[')
            || line.starts_with("whisper_")
            || line.starts_with("ggml_")
            || line.starts_with("load_")
            || line.starts_with("main")
            || line.starts_with("system_info")
        {
            continue;
        }
        if line.len() > best.len() {
            best = line.to_string();
        }
    }
    if best.is_empty() {
        return Err(format!("whisper-cli produced no recognisable output:\n{stdout}").into());
    }
    Ok(best)
}
