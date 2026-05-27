// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! **End-to-end v6.3 pipeline test.**
//!
//! This test exercises the full v6.3 stack without Whisper or
//! any external STT:
//!
//! ```text
//!   synthesised PCM audio (one phoneme at a time)
//!     ↓ adam_audio::mfcc + adam_stt_phoneme::recognise_word
//!   recognised Phoneme stream
//!     ↓ adam_phoneme::cyrillic::phonemes_to_cyrillic
//!   Cyrillic string
//! ```
//!
//! It's the **regression floor for the audio-to-text path**
//! that v6.2 currently delegates to Whisper. As Phase 2d
//! lands real corpus templates, this test's expected output
//! tightens (currently the synth bank only handles a subset
//! of phonemes cleanly).

use adam_audio::pitch::harmonic_voice;
use adam_phoneme::Phoneme;
use adam_phoneme::cyrillic::phonemes_to_cyrillic;
use adam_stt_phoneme::{PhonemeBank, WordConfig, recognise_word};

/// Synthesise one phoneme's audio matching the synthetic bank.
fn synth_phoneme(p: Phoneme, sample_rate: u32, duration_s: f32) -> Vec<f32> {
    let f0 = match p {
        Phoneme::A => 100.0,
        Phoneme::Ae => 120.0,
        Phoneme::O => 140.0,
        Phoneme::Oe => 160.0,
        Phoneme::U => 180.0,
        Phoneme::Ue => 200.0,
        Phoneme::E => 220.0,
        Phoneme::I => 240.0,
        Phoneme::Y => 260.0,
        Phoneme::Yi => 280.0,
        _ => 150.0,
    };
    if matches!(p.class(), adam_phoneme::PhonemeClass::Vowel) {
        harmonic_voice(f0, duration_s, sample_rate, 0.4, 4)
    } else {
        vec![0.0; (duration_s * sample_rate as f32) as usize]
    }
}

/// Synthesise audio for a phoneme sequence (concatenated).
fn synth_sequence(seq: &[Phoneme], sample_rate: u32, per_phoneme_s: f32) -> Vec<f32> {
    let mut out = Vec::new();
    for &p in seq {
        out.extend(synth_phoneme(p, sample_rate, per_phoneme_s));
    }
    out
}

/// **Vowel-only "word"**: synthesise `[A, E, I]`, recognise,
/// render to Cyrillic. The result must contain every input
/// vowel's glyph in the correct order.
#[test]
fn vowel_word_audio_to_cyrillic() {
    let bank = PhonemeBank::synthetic(16_000);
    let seq = [Phoneme::A, Phoneme::E, Phoneme::I];
    let audio = synth_sequence(&seq, 16_000, 0.20);

    // Recognise.
    let recognised = recognise_word(&audio, 16_000, &bank, &WordConfig::default());

    // Render.
    let cyrillic = phonemes_to_cyrillic(&recognised);

    // The recognised stream must contain every input vowel in
    // monotone order. (We don't assert exact equality because
    // the synth bank may insert adjacent neighbours at
    // boundaries.)
    let pos_a = cyrillic.chars().position(|c| c == 'а');
    let pos_e = cyrillic.chars().position(|c| c == 'е');
    let pos_i = cyrillic.chars().position(|c| c == 'и');
    assert!(pos_a.is_some(), "«а» missing from «{cyrillic}»");
    assert!(pos_e.is_some(), "«е» missing from «{cyrillic}»");
    assert!(pos_i.is_some(), "«и» missing from «{cyrillic}»");
    assert!(
        pos_a.unwrap() < pos_e.unwrap() && pos_e.unwrap() < pos_i.unwrap(),
        "order wrong in «{cyrillic}»",
    );
}

/// **Single-vowel utterance**: 300 ms of `Phoneme::U` → must
/// recognise as `U` → render to «ұ».
#[test]
fn single_vowel_renders_to_correct_glyph() {
    let bank = PhonemeBank::synthetic(16_000);
    let audio = synth_phoneme(Phoneme::U, 16_000, 0.30);
    let recognised = recognise_word(&audio, 16_000, &bank, &WordConfig::default());
    let cyrillic = phonemes_to_cyrillic(&recognised);
    assert!(
        cyrillic.contains('ұ'),
        "«ұ» missing from «{cyrillic}» for synth U input",
    );
}

/// **Front-vowel word**: `[E, I, Yi]` — all front. Recognised
/// stream must have pure-Front harmony (per `adam-phonotactics`
/// `check_harmony`).
#[test]
fn front_vowel_word_has_pure_front_harmony() {
    use adam_phoneme::HarmonyClass;
    use adam_phonotactics::{HarmonyResult, check_harmony};

    let bank = PhonemeBank::synthetic(16_000);
    let seq = [Phoneme::E, Phoneme::I, Phoneme::Yi];
    let audio = synth_sequence(&seq, 16_000, 0.20);
    let recognised = recognise_word(&audio, 16_000, &bank, &WordConfig::default());

    let h = check_harmony(&recognised);
    assert!(
        matches!(h, HarmonyResult::Pure(HarmonyClass::Front)),
        "expected pure-front harmony, got {h:?} from {:?}",
        recognised,
    );
}

/// **Silence audio → no recognised phonemes.**
#[test]
fn silence_yields_empty_recognition() {
    let bank = PhonemeBank::synthetic(16_000);
    let audio = vec![0.0_f32; 16_000];
    let recognised = recognise_word(&audio, 16_000, &bank, &WordConfig::default());
    // The recogniser may pick *some* phoneme by minimum cost
    // (DTW always finds a best match), but it must be a tiny
    // result — not multiple labels.
    assert!(recognised.len() <= 1, "silence produced {:?}", recognised);
}

/// **48 kHz path**: same vowel synthesis at 48 kHz must work
/// just as well as 16 kHz.
#[test]
fn end_to_end_at_48khz() {
    let bank = PhonemeBank::synthetic(48_000);
    let audio = synth_phoneme(Phoneme::A, 48_000, 0.30);
    let recognised = recognise_word(&audio, 48_000, &bank, &WordConfig::default());
    let cyrillic = phonemes_to_cyrillic(&recognised);
    assert!(
        cyrillic.contains('а'),
        "«а» missing at 48 kHz from «{cyrillic}»",
    );
}
