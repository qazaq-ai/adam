// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Manual sanity-check: record 3 seconds from the default mic, write
//! to a WAV file, read it back, and play it through the speaker.
//!
//! Run: `cargo run --release -p adam-audio --example record_and_play`
//!
//! Requires an actual microphone and speaker; not used in CI.

use adam_audio::{
    play::play_blocking,
    record::{RecordConfig, record_until_silence},
    wav::{read_wav, write_wav},
};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[adam-audio] speak for up to 5 seconds (or stay silent for 1.5 s to stop)...");
    let config = RecordConfig {
        max_duration: Duration::from_secs(5),
        silence_timeout: Duration::from_millis(1500),
        silence_threshold: 0.01,
    };
    let samples = record_until_silence(config)?;
    println!(
        "[adam-audio] captured {:.2} s at {} Hz mono ({} frames)",
        samples.duration_s(),
        samples.sample_rate,
        samples.data.len(),
    );

    let path = "/tmp/adam-audio-test.wav";
    write_wav(path, &samples)?;
    println!("[adam-audio] wrote {path}");

    let back = read_wav(path)?;
    println!(
        "[adam-audio] read back {:.2} s at {} Hz",
        back.duration_s(),
        back.sample_rate,
    );

    println!("[adam-audio] playing back...");
    play_blocking(&back)?;
    println!("[adam-audio] done");
    Ok(())
}
