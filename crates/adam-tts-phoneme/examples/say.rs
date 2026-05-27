// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Manual example: synthesise a Kazakh word and write it to
//! WAV (and play it through the speaker).
//!
//! ```sh
//! cargo run --release -p adam-tts-phoneme --example say -- "сәлем"
//! # → /tmp/adam-tts.wav written; played via cpal.
//! ```
//!
//! Requires a working speaker; not used in CI.

use adam_audio::play::play_blocking;
use adam_audio::wav::write_wav;
use adam_phoneme::cyrillic::cyrillic_to_phonemes;
use adam_tts_phoneme::{TtsConfig, synthesise};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let word = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "сәлем".to_string());
    println!("[say] input: «{word}»");
    let phonemes = cyrillic_to_phonemes(&word, true);
    println!("[say] phonemes: {phonemes:?}");

    let pcm = synthesise(&phonemes, &TtsConfig::default());
    println!(
        "[say] synthesised {:.2} s at {} Hz",
        pcm.duration_s(),
        pcm.sample_rate
    );

    let path = "/tmp/adam-tts.wav";
    write_wav(path, &pcm)?;
    println!("[say] wrote {path}");

    println!("[say] playing...");
    play_blocking(&pcm)?;
    println!("[say] done");
    Ok(())
}
