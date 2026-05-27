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

        // STT: recognise phoneme stream + rescore.
        let raw = recognise_word(
            &pcm.data,
            pcm.sample_rate,
            &mfcc_bank,
            &WordConfig::default(),
        );
        let rescored = rescore(&raw);
        let cyrillic = phonemes_to_cyrillic(&rescored);
        println!("[voice-repl] phonemes (raw):       {raw:?}");
        println!("[voice-repl] phonemes (rescored):  {rescored:?}");
        println!("[voice-repl] cyrillic:             «{cyrillic}»");

        // Optional TTS playback.
        if args.speak {
            let tts_out = synthesise_with_bank(&rescored, pcm_bank.as_ref(), &TtsConfig::default());
            println!(
                "[voice-repl] synthesised {:.2} s, playing back",
                tts_out.duration_s()
            );
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
