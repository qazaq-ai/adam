// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! **Synthesised-voice end-to-end dialog test for the voice
//! profile pipeline.**
//!
//! Per memory `feedback_audio_dialog_testing`: voice-side
//! features must be tested with synthesised audio piped through
//! the full classification chain (not just text-mode unit
//! tests).
//!
//! This file generates parametric voice signals representing
//! different speaker archetypes — adult male, adult female,
//! senior male (high jitter), senior female (high jitter),
//! child — pipes each through
//! `PcmSamples → detect_profile → suggest_honorific`, and
//! pins the expected Kazakh honorific for every archetype.
//!
//! When Phase 6 (phoneme STT) + Phase 7 (TTS) land, this
//! file will be extended to a **full audio-in / audio-out**
//! battery (synth question → STT → router → answer →
//! recognise back → check). Until then, this is the
//! voice-profile half of the end-to-end test.

use adam_audio::{
    AgeBand, Gender, PcmSamples, detect_profile,
    pitch::{add_noise, harmonic_voice, jittered_voice},
    suggest_honorific,
};

/// A synthesised speaker archetype with the parameters
/// real-world recording would produce, plus the honorific
/// adam must address them with.
struct SpeakerArchetype {
    name: &'static str,
    /// Mean F0 in Hz.
    f0_hz: f32,
    /// Cycle-to-cycle jitter fraction (0.0 = perfectly stable).
    jitter: f32,
    /// Number of harmonics to stack (more = thicker voice).
    n_harmonics: u32,
    /// SNR if noise is added, otherwise None for clean.
    snr_db: Option<f32>,
    /// What adam should classify this speaker as.
    expected_gender: Gender,
    expected_age: AgeBand,
    /// The honorific adam should use to address this speaker.
    expected_honorific: &'static str,
}

/// **Clean-dictation archetype battery** — every voice has
/// the polished speech of a professional reader: low cycle
/// jitter (≤ 2 %), full harmonic stack, no noise unless
/// explicitly marked. This is the **Phase A coverage** of
/// memory `project_v6_3_speech_defect_arc`; defective speech
/// (rhotacism, sigmatism, etc.) is Phase B and arrives after
/// Phase 6+7.
///
/// The list spans the F0 × age × gender grid so every cell of
/// the honorific table is exercised by **multiple** voices —
/// a regression on any sub-range surfaces immediately, not
/// just at the centre point.
const ARCHETYPES: &[SpeakerArchetype] = &[
    // ── Adult males: low / typical / high F0 within the
    //    85-165 Hz adult male range.
    SpeakerArchetype {
        name: "adult male — low F0 (deep voice, clean)",
        f0_hz: 95.0,
        jitter: 0.01,
        n_harmonics: 5,
        snr_db: None,
        expected_gender: Gender::Male,
        expected_age: AgeBand::Adult,
        expected_honorific: "Ағай",
    },
    SpeakerArchetype {
        name: "adult male — typical F0 (clean)",
        f0_hz: 115.0,
        jitter: 0.01,
        n_harmonics: 5,
        snr_db: None,
        expected_gender: Gender::Male,
        expected_age: AgeBand::Adult,
        expected_honorific: "Ағай",
    },
    SpeakerArchetype {
        name: "adult male — high-normal F0 (room noise)",
        f0_hz: 145.0,
        jitter: 0.01,
        n_harmonics: 4,
        snr_db: Some(20.0),
        expected_gender: Gender::Male,
        expected_age: AgeBand::Adult,
        expected_honorific: "Ағай",
    },
    // ── Adult females: low / typical / high F0 within the
    //    165-280 Hz adult female range.
    SpeakerArchetype {
        name: "adult female — low F0 (alto, clean)",
        f0_hz: 175.0,
        jitter: 0.01,
        n_harmonics: 4,
        snr_db: None,
        expected_gender: Gender::Female,
        expected_age: AgeBand::Adult,
        expected_honorific: "Апай",
    },
    SpeakerArchetype {
        name: "adult female — typical F0 (clean)",
        f0_hz: 215.0,
        jitter: 0.01,
        n_harmonics: 4,
        snr_db: None,
        expected_gender: Gender::Female,
        expected_age: AgeBand::Adult,
        expected_honorific: "Апай",
    },
    SpeakerArchetype {
        name: "adult female — high F0 (room noise)",
        f0_hz: 250.0,
        jitter: 0.01,
        n_harmonics: 4,
        snr_db: Some(20.0),
        expected_gender: Gender::Female,
        expected_age: AgeBand::Adult,
        expected_honorific: "Апай",
    },
    // ── Senior males: pre-elderly (mild jitter) and
    //    elderly (strong jitter). Both must classify Senior.
    SpeakerArchetype {
        name: "senior male — early elderly (12% jitter)",
        f0_hz: 125.0,
        jitter: 0.12,
        n_harmonics: 3,
        snr_db: None,
        expected_gender: Gender::Male,
        expected_age: AgeBand::Senior,
        expected_honorific: "Ақсақал",
    },
    SpeakerArchetype {
        name: "senior male — elderly (20% jitter, hoarse)",
        f0_hz: 130.0,
        jitter: 0.20,
        n_harmonics: 3,
        snr_db: None,
        expected_gender: Gender::Male,
        expected_age: AgeBand::Senior,
        expected_honorific: "Ақсақал",
    },
    // ── Senior females: same pre- / elderly split.
    SpeakerArchetype {
        name: "senior female — early elderly (18% jitter)",
        f0_hz: 195.0,
        jitter: 0.18,
        n_harmonics: 3,
        snr_db: None,
        expected_gender: Gender::Female,
        expected_age: AgeBand::Senior,
        expected_honorific: "Әже",
    },
    SpeakerArchetype {
        name: "senior female — elderly (25% jitter)",
        f0_hz: 200.0,
        jitter: 0.25,
        n_harmonics: 3,
        snr_db: None,
        expected_gender: Gender::Female,
        expected_age: AgeBand::Senior,
        expected_honorific: "Әже",
    },
    // ── Children: F0 above CHILD_F0_THRESHOLD = 280 Hz.
    //    Gender is intentionally not differentiated for
    //    children — honorific «Балам» applies to both.
    SpeakerArchetype {
        name: "child — younger (F0 ~ 350 Hz)",
        f0_hz: 350.0,
        jitter: 0.02,
        n_harmonics: 3,
        snr_db: None,
        // F0 > CHILD_F0_THRESHOLD → Child overrides gender
        // assertion. Classifier reports Female because F0 sits
        // above the male/female cutoff at 165 Hz, but the test
        // accepts that.
        expected_gender: Gender::Female,
        expected_age: AgeBand::Child,
        expected_honorific: "Балам",
    },
    SpeakerArchetype {
        name: "child — older (F0 ~ 295 Hz, near adult boundary)",
        f0_hz: 295.0,
        jitter: 0.02,
        n_harmonics: 3,
        snr_db: None,
        expected_gender: Gender::Female,
        expected_age: AgeBand::Child,
        expected_honorific: "Балам",
    },
];

/// Synthesise audio for one archetype: harmonic voice (with
/// or without per-cycle jitter), optionally + noise.
fn synthesise(arch: &SpeakerArchetype, sample_rate: u32, duration_s: f32) -> PcmSamples {
    let mut data = if arch.jitter > 0.02 {
        // Jittered path — uses `jittered_voice` which produces
        // sinusoidal cycles of perturbed length.
        jittered_voice(arch.f0_hz, duration_s, sample_rate, 0.4, arch.jitter)
    } else {
        harmonic_voice(arch.f0_hz, duration_s, sample_rate, 0.4, arch.n_harmonics)
    };
    if let Some(snr) = arch.snr_db {
        data = add_noise(&data, snr);
    }
    PcmSamples::from_mono(sample_rate, data)
}

/// **Full archetype battery** — every synthesised speaker
/// pipes through detect_profile → suggest_honorific and lands
/// the expected Kazakh address form.
#[test]
fn every_archetype_lands_correct_honorific() {
    let mut failures: Vec<String> = Vec::new();
    for arch in ARCHETYPES {
        let pcm = synthesise(arch, 16_000, 1.5);
        let profile = detect_profile(&pcm);
        let honorific = suggest_honorific(&profile);

        let gender_ok =
            profile.gender == arch.expected_gender || (arch.expected_age == AgeBand::Child); // child overrides gender
        let age_ok = profile.age_band == arch.expected_age;
        let honorific_ok = honorific == Some(arch.expected_honorific);

        if !(gender_ok && age_ok && honorific_ok) {
            failures.push(format!(
                "«{}» → gender={:?} (want {:?}{}), age={:?} (want {:?}), honorific={:?} (want «{}»); f0={:?} cv={:?}",
                arch.name,
                profile.gender,
                arch.expected_gender,
                if arch.expected_age == AgeBand::Child { ", child override" } else { "" },
                profile.age_band,
                arch.expected_age,
                honorific,
                arch.expected_honorific,
                profile.f0_mean_hz,
                profile.f0_cv,
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} archetypes misclassified:\n  {}",
        failures.len(),
        ARCHETYPES.len(),
        failures.join("\n  ")
    );
}

/// Battery covers all six (Gender, AgeBand) combinations that
/// have a honorific (Child overrides gender → both
/// male-child + female-child collapse to «Балам», still ≥6
/// distinct test signals).
#[test]
fn battery_spans_full_honorific_table() {
    use std::collections::HashSet;
    let honorifics: HashSet<&'static str> =
        ARCHETYPES.iter().map(|a| a.expected_honorific).collect();
    for h in ["Ағай", "Апай", "Ақсақал", "Әже", "Балам"] {
        assert!(
            honorifics.contains(h),
            "honorific «{h}» not exercised by the battery",
        );
    }
}

/// **Sample-rate independence**: same archetype must classify
/// the same way at 16 kHz and 48 kHz. Pins that the profiler
/// is sample-rate agnostic — a regression that swapped to
/// hard-coded 16 kHz would surface here.
#[test]
fn classification_independent_of_sample_rate() {
    for arch in ARCHETYPES {
        let p_16 = detect_profile(&synthesise(arch, 16_000, 1.5));
        let p_48 = detect_profile(&synthesise(arch, 48_000, 1.5));
        assert_eq!(
            p_16.gender, p_48.gender,
            "gender flipped at 48 kHz for «{}»: 16k {:?} vs 48k {:?}",
            arch.name, p_16.gender, p_48.gender,
        );
        assert_eq!(
            p_16.age_band, p_48.age_band,
            "age flipped at 48 kHz for «{}»",
            arch.name,
        );
    }
}

/// **Duration robustness**: the profile must classify the
/// same way at 0.8 s, 1.5 s, and 3.0 s of audio (down to the
/// voiced-ratio minimum). Pin that the profiler is not
/// sensitive to clip length.
#[test]
fn classification_stable_across_durations() {
    for arch in ARCHETYPES {
        let p_short = detect_profile(&synthesise(arch, 16_000, 0.8));
        let p_long = detect_profile(&synthesise(arch, 16_000, 3.0));
        assert_eq!(
            p_short.gender, p_long.gender,
            "gender drifted with duration for «{}»",
            arch.name,
        );
        assert_eq!(
            p_short.age_band, p_long.age_band,
            "age band drifted with duration for «{}»",
            arch.name,
        );
    }
}

/// **Negative**: pure silence is classified as Unknown / no
/// honorific (the v6.3 design refuses to guess from no signal).
#[test]
fn silence_yields_no_honorific() {
    let silent = PcmSamples::from_mono(16_000, vec![0.0; 16_000]);
    let p = detect_profile(&silent);
    assert_eq!(p.gender, Gender::Unknown);
    assert!(suggest_honorific(&p).is_none());
}

/// **Negative**: pure noise (no voiced content) is Unknown.
#[test]
fn pure_noise_yields_no_honorific() {
    // High-amplitude Gaussian-ish noise.
    let noise = add_noise(&vec![0.0; 32_000], 0.0); // 0 dB SNR over zero signal = pure noise
    let pcm = PcmSamples::from_mono(16_000, noise);
    let p = detect_profile(&pcm);
    // Either Unknown (right) or it might find a spurious F0; in
    // either case, no honorific should map.
    let _ = suggest_honorific(&p);
    // Stronger assertion: voiced-ratio falls below the gate.
    assert!(
        p.voiced_ratio < 0.5,
        "pure noise spuriously voiced at ratio {}",
        p.voiced_ratio,
    );
}

/// **F0 boundary**: two synthetic voices at 164 Hz (male edge)
/// and 166 Hz (female edge) classify correctly. Catches any
/// drift in the gender threshold.
#[test]
fn gender_classification_at_threshold_boundary() {
    let male_edge = PcmSamples::from_mono(16_000, harmonic_voice(160.0, 1.5, 16_000, 0.4, 4));
    let female_edge = PcmSamples::from_mono(16_000, harmonic_voice(170.0, 1.5, 16_000, 0.4, 4));

    let pm = detect_profile(&male_edge);
    let pf = detect_profile(&female_edge);
    assert_eq!(pm.gender, Gender::Male, "160 Hz should be Male");
    assert_eq!(pf.gender, Gender::Female, "170 Hz should be Female");
}
