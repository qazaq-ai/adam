// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Speaker profiling from a voice sample — gender, approximate
//! age band, and a Kazakh honorific suggestion.
//!
//! The v6.3 dialog layer wants to address an unknown user
//! correctly: «Ағай» (older male), «Апай» (older female),
//! «Ақсақал» (senior male), «Әже» (senior female), «Балам»
//! (child of either gender). When the user has not yet given a
//! name, the only signal available is the voice itself.
//!
//! This module classifies on three deterministic features
//! computed by [`crate::pitch`]:
//!
//! - **F0 mean** — fundamental frequency averaged over voiced
//!   frames. Strong cue for gender (the male / female F0
//!   distributions overlap only narrowly around 160 Hz) and
//!   moderate cue for child vs. adult (children's F0 is
//!   noticeably higher than any adult's).
//! - **F0 stability (jitter proxy)** — the per-frame
//!   coefficient of variation of F0. Old age, vocal-fold ageing,
//!   and pathological voice all increase jitter. A simple
//!   "stable / unstable" partition gives a usable senior-vs-
//!   adult signal without a supervised model.
//! - **Voiced-frame ratio** — what fraction of analysed frames
//!   had a recoverable F0. Below 30 %, the input is too noisy
//!   or non-speech to classify; we return `Unknown` rather
//!   than guess.
//!
//! These features are deterministic and run in <1 ms per
//! second of audio on M2. They are not perfect — without
//! supervised training the senior / middle-aged boundary is
//! the noisiest call — but they ship a usable first-pass
//! classifier with no external model.
//!
//! Camera-based identification is on the v6.3 backlog for
//! future expansion; this voice-only module is the bootstrap.

use crate::PcmSamples;
use crate::pitch;

/// Gender classification from F0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gender {
    /// Mean F0 in the typical adult-male range (~85–165 Hz).
    Male,
    /// Mean F0 in the typical adult-female range (~165–280 Hz).
    Female,
    /// Either no F0 detected at all, or F0 outside any plausible
    /// human-speech range. Caller should fall back to other
    /// signals (name, explicit honorific request, etc.).
    Unknown,
}

/// Approximate age band. Wider categories than full ages — the
/// signal does not support finer resolution from voice alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgeBand {
    /// ~< 13. Mean F0 above 250 Hz regardless of gender.
    Child,
    /// ~13–55. The default adult range.
    Adult,
    /// ~55+. Detected from elevated F0 jitter combined with
    /// adult or near-adult F0 mean. Inherently noisy —
    /// callers should treat this as a hint, not a fact.
    Senior,
    /// Insufficient signal to classify.
    Unknown,
}

/// Combined classification result.
#[derive(Debug, Clone, PartialEq)]
pub struct SpeakerProfile {
    pub gender: Gender,
    pub age_band: AgeBand,
    /// Mean F0 over voiced frames, in Hz. `None` when fewer
    /// than 30 % of frames were voiced.
    pub f0_mean_hz: Option<f32>,
    /// Per-frame coefficient of variation of F0 (`stddev /
    /// mean`). Proxy for jitter. `None` when fewer than two
    /// voiced frames.
    pub f0_cv: Option<f32>,
    /// Fraction of analysed frames where F0 was recovered.
    pub voiced_ratio: f32,
}

/// Suggest a Kazakh honorific for addressing the speaker when
/// no name is known. Returns `None` when classification is too
/// uncertain — callers should then default to a name-less
/// address («сіз» / explicit re-introduction).
pub fn suggest_honorific(profile: &SpeakerProfile) -> Option<&'static str> {
    match (profile.gender, profile.age_band) {
        (Gender::Male, AgeBand::Senior) => Some("Ақсақал"),
        (Gender::Male, AgeBand::Adult) => Some("Ағай"),
        (Gender::Male, AgeBand::Child) => Some("Балам"),
        (Gender::Female, AgeBand::Senior) => Some("Әже"),
        (Gender::Female, AgeBand::Adult) => Some("Апай"),
        (Gender::Female, AgeBand::Child) => Some("Балам"),
        _ => None,
    }
}

/// Default voiced-ratio floor below which we refuse to
/// classify (returns `Unknown`).
pub const MIN_VOICED_RATIO: f32 = 0.30;

/// CV-of-F0 threshold above which we classify a voice as
/// senior (in conjunction with adult F0 mean).
///
/// **This is a frame-level coefficient of variation**, not the
/// clinical "jitter local" measure (which is computed at the
/// period level and runs at well under 1 % for healthy
/// adults). Frame-level CV smooths multiple cycles per frame,
/// so the senior boundary in this proxy sits noticeably higher
/// than the clinical one. We use **0.04** as a coarse cut-off:
/// healthy stable voices land at 0.01–0.02 frame CV at the
/// 30 ms frame size, while voices with 10–15 % cycle-to-cycle
/// jitter (clinically elderly / hoarse) exceed it.
///
/// A future Phase 6 implementation may swap this proxy for a
/// proper cycle-level jitter calculation, lowering the
/// threshold to the clinical range.
pub const SENIOR_CV_THRESHOLD: f32 = 0.04;

/// F0 (Hz) above which we classify as child regardless of
/// gender. Adult females top out around 280 Hz; children's
/// voices regularly exceed 300 Hz.
pub const CHILD_F0_THRESHOLD_HZ: f32 = 280.0;

/// Profile a speaker from a PCM buffer.
///
/// The buffer must be mono ([`PcmSamples::to_mono`] is a
/// convenience for the caller). Internally the audio is split
/// into 40 ms frames at the input sample rate; F0 is estimated
/// per frame; mean and CV are computed over the voiced subset.
pub fn detect_profile(samples: &PcmSamples) -> SpeakerProfile {
    // 30 ms frames give ~3–4 voiced cycles per frame across the
    // entire 100–300 Hz speaker range. Shorter than the original
    // 40 ms (which averaged jitter out) and long enough for YIN
    // to lock onto a low-male F0 at min_f0=70 Hz.
    let frame_size = (samples.sample_rate as f32 * 0.030) as usize;
    if samples.channels != 1 || samples.data.len() < frame_size * 2 {
        return SpeakerProfile {
            gender: Gender::Unknown,
            age_band: AgeBand::Unknown,
            f0_mean_hz: None,
            f0_cv: None,
            voiced_ratio: 0.0,
        };
    }

    // Frame-level F0 estimates.
    let mut voiced_f0: Vec<f32> = Vec::new();
    let mut total_frames = 0_usize;
    let mut offset = 0;
    while offset + frame_size <= samples.data.len() {
        total_frames += 1;
        let frame = &samples.data[offset..offset + frame_size];
        if let Some(f0) = pitch::detect_f0(frame, samples.sample_rate) {
            voiced_f0.push(f0);
        }
        // Hop = 50 % overlap for stability.
        offset += frame_size / 2;
    }

    let voiced_ratio = if total_frames == 0 {
        0.0
    } else {
        voiced_f0.len() as f32 / total_frames as f32
    };

    if voiced_ratio < MIN_VOICED_RATIO || voiced_f0.is_empty() {
        return SpeakerProfile {
            gender: Gender::Unknown,
            age_band: AgeBand::Unknown,
            f0_mean_hz: None,
            f0_cv: None,
            voiced_ratio,
        };
    }

    let mean: f32 = voiced_f0.iter().sum::<f32>() / voiced_f0.len() as f32;
    let cv = if voiced_f0.len() >= 2 {
        let var: f32 =
            voiced_f0.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / voiced_f0.len() as f32;
        let stddev = var.sqrt();
        if mean > 0.0 {
            Some(stddev / mean)
        } else {
            None
        }
    } else {
        None
    };

    let gender = classify_gender(mean);
    let age_band = classify_age(mean, cv);

    SpeakerProfile {
        gender,
        age_band,
        f0_mean_hz: Some(mean),
        f0_cv: cv,
        voiced_ratio,
    }
}

/// Classify gender from a single F0 mean.
pub fn classify_gender(f0_mean_hz: f32) -> Gender {
    if !(70.0..=400.0).contains(&f0_mean_hz) {
        return Gender::Unknown;
    }
    if f0_mean_hz < 165.0 {
        Gender::Male
    } else {
        Gender::Female
    }
}

/// Classify age band from F0 mean and (optional) CV.
pub fn classify_age(f0_mean_hz: f32, f0_cv: Option<f32>) -> AgeBand {
    if !(70.0..=400.0).contains(&f0_mean_hz) {
        return AgeBand::Unknown;
    }
    if f0_mean_hz > CHILD_F0_THRESHOLD_HZ {
        return AgeBand::Child;
    }
    match f0_cv {
        Some(cv) if cv > SENIOR_CV_THRESHOLD => AgeBand::Senior,
        _ => AgeBand::Adult,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::{add_noise, harmonic_voice, jittered_voice};

    /// 120 Hz **harmonic voice** (F0 + 4 overtones) — realistic
    /// adult-male timbre, not a pure sine. Profile must classify
    /// Male + Adult, honorific «Ағай».
    #[test]
    fn male_adult_realistic_voice() {
        let mono = PcmSamples::from_mono(16_000, harmonic_voice(120.0, 1.5, 16_000, 0.4, 4));
        let p = detect_profile(&mono);
        assert_eq!(p.gender, Gender::Male);
        assert_eq!(p.age_band, AgeBand::Adult);
        assert_eq!(suggest_honorific(&p), Some("Ағай"));
        // Sanity on F0 estimate.
        let f0 = p.f0_mean_hz.unwrap();
        assert!((f0 - 120.0).abs() < 5.0, "{f0}");
    }

    /// 220 Hz harmonic voice — adult female. Honorific «Апай».
    #[test]
    fn female_adult_realistic_voice() {
        let mono = PcmSamples::from_mono(16_000, harmonic_voice(220.0, 1.5, 16_000, 0.4, 4));
        let p = detect_profile(&mono);
        assert_eq!(p.gender, Gender::Female);
        assert_eq!(p.age_band, AgeBand::Adult);
        assert_eq!(suggest_honorific(&p), Some("Апай"));
    }

    /// 320 Hz harmonic voice — child. Honorific «Балам».
    #[test]
    fn child_realistic_voice() {
        let mono = PcmSamples::from_mono(16_000, harmonic_voice(320.0, 1.5, 16_000, 0.4, 3));
        let p = detect_profile(&mono);
        assert_eq!(p.age_band, AgeBand::Child);
        assert_eq!(suggest_honorific(&p), Some("Балам"));
    }

    /// Real adult-male voice **with 20 dB SNR background noise**.
    /// Honorific must still resolve to «Ағай». This is the
    /// realistic indoor-microphone case.
    #[test]
    fn male_voice_with_room_noise_still_classifies() {
        let clean = harmonic_voice(125.0, 1.5, 16_000, 0.4, 4);
        let noisy = add_noise(&clean, 20.0);
        let mono = PcmSamples::from_mono(16_000, noisy);
        let p = detect_profile(&mono);
        assert_eq!(p.gender, Gender::Male);
        // Adult vs Senior may flip under noise — assert only
        // the gender + honorific class (Ағай OR Ақсақал
        // both acceptable for the noisy case).
        let h = suggest_honorific(&p);
        assert!(
            matches!(h, Some("Ағай") | Some("Ақсақал")),
            "expected male honorific, got {h:?}"
        );
    }

    /// **Jittered male voice (15 % cycle-to-cycle jitter)** —
    /// the upper end of pathological / elderly speech. Must
    /// classify Senior; honorific «Ақсақал».
    #[test]
    fn senior_male_via_jittered_voice() {
        let mono = PcmSamples::from_mono(16_000, jittered_voice(130.0, 2.0, 16_000, 0.4, 0.15));
        let p = detect_profile(&mono);
        assert_eq!(p.gender, Gender::Male);
        // Pin the CV is actually high.
        let cv = p.f0_cv.unwrap();
        assert!(cv > SENIOR_CV_THRESHOLD, "expected high CV, got {cv}");
        assert_eq!(p.age_band, AgeBand::Senior);
        assert_eq!(suggest_honorific(&p), Some("Ақсақал"));
    }

    /// **Jittered female voice (20 % cycle-to-cycle jitter)** —
    /// senior female. Higher jitter than the male test because
    /// 200 Hz F0 fits more cycles per frame than 130 Hz, so the
    /// frame averaging smooths more aggressively — to surface
    /// senior-level frame CV we need a stronger underlying
    /// cycle perturbation. This is the proxy limitation
    /// documented on [`SENIOR_CV_THRESHOLD`].
    #[test]
    fn senior_female_via_jittered_voice() {
        let mono = PcmSamples::from_mono(16_000, jittered_voice(200.0, 2.0, 16_000, 0.4, 0.20));
        let p = detect_profile(&mono);
        assert_eq!(p.gender, Gender::Female);
        let cv = p.f0_cv.unwrap();
        assert!(
            cv > SENIOR_CV_THRESHOLD,
            "expected senior-level CV, got {cv}"
        );
        assert_eq!(p.age_band, AgeBand::Senior);
        assert_eq!(suggest_honorific(&p), Some("Әже"));
    }

    /// **Healthy adult voice (1 % jitter)** — should NOT be
    /// classified Senior. Pins the lower boundary.
    #[test]
    fn healthy_adult_low_jitter_not_senior() {
        let mono = PcmSamples::from_mono(16_000, jittered_voice(120.0, 2.0, 16_000, 0.4, 0.01));
        let p = detect_profile(&mono);
        assert_eq!(p.gender, Gender::Male);
        let cv = p.f0_cv.unwrap();
        assert!(
            cv < SENIOR_CV_THRESHOLD,
            "low-jitter voice has high CV: {cv}"
        );
        assert_eq!(p.age_band, AgeBand::Adult);
        assert_eq!(suggest_honorific(&p), Some("Ағай"));
    }

    /// **48 kHz sample rate** path — classifier is sample-rate
    /// agnostic.
    #[test]
    fn classification_at_48khz() {
        let mono = PcmSamples::from_mono(48_000, harmonic_voice(140.0, 1.5, 48_000, 0.4, 4));
        let p = detect_profile(&mono);
        assert_eq!(p.gender, Gender::Male);
        assert_eq!(p.age_band, AgeBand::Adult);
    }

    /// **Stereo input** is rejected at the PCM-channel check
    /// (caller responsibility to downmix first). Returns
    /// Unknown profile.
    #[test]
    fn stereo_input_rejected() {
        let stereo = PcmSamples {
            sample_rate: 16_000,
            channels: 2,
            data: vec![0.1; 32_000],
        };
        let p = detect_profile(&stereo);
        assert_eq!(p.gender, Gender::Unknown);
        assert_eq!(p.age_band, AgeBand::Unknown);
        assert!(p.f0_mean_hz.is_none());
    }

    /// **Mixed signal** (voice fading to silence): the
    /// voiced-ratio gate should still permit classification if
    /// at least 30 % of frames are voiced.
    #[test]
    fn partial_voice_classifies_when_above_min_ratio() {
        let mut voiced = harmonic_voice(115.0, 1.0, 16_000, 0.4, 4);
        let silent = vec![0.0_f32; 16_000]; // 1 s of silence
        voiced.extend(silent);
        let mono = PcmSamples::from_mono(16_000, voiced);
        let p = detect_profile(&mono);
        assert_eq!(p.gender, Gender::Male);
        assert!(p.voiced_ratio >= MIN_VOICED_RATIO);
    }

    /// **Mostly silence** — voiced ratio falls below the gate;
    /// no classification.
    #[test]
    fn mostly_silence_yields_unknown_via_voiced_ratio() {
        let mut signal = harmonic_voice(120.0, 0.1, 16_000, 0.4, 4); // 0.1 s voice
        signal.extend(vec![0.0_f32; 16_000 * 3]); // 3 s silence
        let mono = PcmSamples::from_mono(16_000, signal);
        let p = detect_profile(&mono);
        // voiced ratio should be ~3 % — well below MIN_VOICED_RATIO.
        assert!(p.voiced_ratio < MIN_VOICED_RATIO);
        assert_eq!(p.gender, Gender::Unknown);
    }

    /// Silent input → Unknown both ways; no honorific.
    #[test]
    fn silence_yields_unknown() {
        let mono = PcmSamples::from_mono(16_000, vec![0.0; 16_000]);
        let p = detect_profile(&mono);
        assert_eq!(p.gender, Gender::Unknown);
        assert_eq!(p.age_band, AgeBand::Unknown);
        assert!(suggest_honorific(&p).is_none());
    }

    /// CV threshold: high jitter at adult-male F0 → Senior;
    /// honorific «Ақсақал».
    #[test]
    fn high_jitter_at_male_f0_classifies_senior() {
        let g = classify_gender(125.0);
        assert_eq!(g, Gender::Male);
        let a = classify_age(125.0, Some(0.08));
        assert_eq!(a, AgeBand::Senior);
        let p = SpeakerProfile {
            gender: g,
            age_band: a,
            f0_mean_hz: Some(125.0),
            f0_cv: Some(0.08),
            voiced_ratio: 0.95,
        };
        assert_eq!(suggest_honorific(&p), Some("Ақсақал"));
    }

    /// CV threshold: high jitter at adult-female F0 → Senior;
    /// honorific «Әже».
    #[test]
    fn high_jitter_at_female_f0_classifies_senior() {
        let p = SpeakerProfile {
            gender: classify_gender(195.0),
            age_band: classify_age(195.0, Some(0.07)),
            f0_mean_hz: Some(195.0),
            f0_cv: Some(0.07),
            voiced_ratio: 0.95,
        };
        assert_eq!(p.gender, Gender::Female);
        assert_eq!(p.age_band, AgeBand::Senior);
        assert_eq!(suggest_honorific(&p), Some("Әже"));
    }

    /// Boundary: F0 exactly at male/female threshold goes Female.
    #[test]
    fn gender_boundary_male_female() {
        assert_eq!(classify_gender(164.9), Gender::Male);
        assert_eq!(classify_gender(165.0), Gender::Female);
    }

    /// Boundary: F0 just above adult ceiling → Child.
    #[test]
    fn age_boundary_adult_child() {
        assert_eq!(classify_age(280.1, None), AgeBand::Child);
        assert_eq!(classify_age(279.9, None), AgeBand::Adult);
    }

    /// F0 outside human range → Unknown.
    #[test]
    fn extreme_f0_unknown() {
        assert_eq!(classify_gender(50.0), Gender::Unknown);
        assert_eq!(classify_gender(500.0), Gender::Unknown);
        assert_eq!(classify_age(50.0, Some(0.01)), AgeBand::Unknown);
        assert_eq!(classify_age(500.0, Some(0.01)), AgeBand::Unknown);
    }

    /// Honorific table covers every (Gender, AgeBand) of
    /// interest; (Unknown, *) and (*, Unknown) return None.
    #[test]
    fn honorific_table_completeness() {
        let cases: &[(Gender, AgeBand, Option<&str>)] = &[
            (Gender::Male, AgeBand::Senior, Some("Ақсақал")),
            (Gender::Male, AgeBand::Adult, Some("Ағай")),
            (Gender::Male, AgeBand::Child, Some("Балам")),
            (Gender::Female, AgeBand::Senior, Some("Әже")),
            (Gender::Female, AgeBand::Adult, Some("Апай")),
            (Gender::Female, AgeBand::Child, Some("Балам")),
            (Gender::Unknown, AgeBand::Adult, None),
            (Gender::Male, AgeBand::Unknown, None),
            (Gender::Unknown, AgeBand::Unknown, None),
        ];
        for &(g, a, expected) in cases {
            let p = SpeakerProfile {
                gender: g,
                age_band: a,
                f0_mean_hz: None,
                f0_cv: None,
                voiced_ratio: 0.0,
            };
            assert_eq!(suggest_honorific(&p), expected, "for ({g:?}, {a:?})");
        }
    }
}
