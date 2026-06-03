// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Voice typology — speakers of different age, sex and register.
//!
//! User directive (2026-05-28): "сразу создай структуру, как
//! мужского, женского, так и детский, юношеский, взрослый и
//! старческий. От сопрано до баса". The physics is simple
//! enough to expose as two main parameters plus two secondary
//! ones:
//!
//! - **`f0_hz`** — fundamental frequency (perceived pitch).
//!   Bass ~80 Hz, baritone ~110, tenor ~150, alto ~200,
//!   soprano ~250, child ~300+.
//! - **`formant_scale`** — multiplier on every formant centre
//!   frequency (`F1, F2, F3`). Vocal-tract length and formant
//!   frequency are inversely related (~17.5 cm adult-male
//!   reference → scale 1.0; ~15 cm adult-female → ~1.17;
//!   ~13.5 cm 10-year-old child → ~1.30; ~19 cm bass → ~0.92).
//!   This is the standard VTL-normalisation knob.
//! - **`jitter`** — fractional perturbation of the glottal
//!   period per cycle (0.0 = perfectly periodic; 0.02 = ±2 %
//!   ≈ healthy voice; 0.05+ = elderly / rough voice).
//! - **`breath`** — additive noise on the voiced source as a
//!   fraction of the pulse-train amplitude. 0.0 = pure
//!   harmonic; 0.3 = breathy soprano; 0.5+ = whispered /
//!   elderly.
//!
//! Together these knobs cover every preset the user listed
//! without inventing a new synthesiser per voice.

/// A speaker timbre: pitch + vocal-tract length + glottal
/// source quality.
#[derive(Debug, Clone, Copy)]
pub struct VoiceProfile {
    pub name: &'static str,
    pub f0_hz: f32,
    pub formant_scale: f32,
    pub jitter: f32,
    pub breath: f32,
}

impl VoiceProfile {
    pub const fn new(
        name: &'static str,
        f0_hz: f32,
        formant_scale: f32,
        jitter: f32,
        breath: f32,
    ) -> Self {
        Self {
            name,
            f0_hz,
            formant_scale,
            jitter,
            breath,
        }
    }
}

// ─── Adult males — bass to tenor ──────────────────────────────

/// Adult male bass — deepest male register.
pub const BASS: VoiceProfile = VoiceProfile::new("bass", 85.0, 0.92, 0.015, 0.05);

/// Adult male baritone — most common male voice.
pub const BARITONE: VoiceProfile = VoiceProfile::new("baritone", 110.0, 0.96, 0.015, 0.05);

/// Adult male tenor — highest male register.
pub const TENOR: VoiceProfile = VoiceProfile::new("tenor", 150.0, 1.00, 0.015, 0.05);

// ─── Adult females — contralto to soprano ─────────────────────

/// Adult female contralto / alto — lowest female register.
pub const CONTRALTO: VoiceProfile = VoiceProfile::new("contralto", 175.0, 1.10, 0.015, 0.10);

/// Adult female mezzo-soprano — most common female voice.
pub const MEZZO: VoiceProfile = VoiceProfile::new("mezzo", 220.0, 1.15, 0.015, 0.12);

/// Adult female soprano — highest female register.
pub const SOPRANO: VoiceProfile = VoiceProfile::new("soprano", 260.0, 1.20, 0.015, 0.15);

// ─── Teens / youth ────────────────────────────────────────────

/// Adolescent male (~14-17 y; post-puberty start, voice still
/// settling).
pub const YOUTH_MALE: VoiceProfile = VoiceProfile::new("youth_male", 150.0, 1.05, 0.025, 0.07);

/// Adolescent female (~14-17 y).
pub const YOUTH_FEMALE: VoiceProfile = VoiceProfile::new("youth_female", 230.0, 1.18, 0.020, 0.10);

// ─── Children ─────────────────────────────────────────────────

/// Young child (~5-7 y). High F0, short vocal tract.
pub const CHILD_YOUNG: VoiceProfile = VoiceProfile::new("child_young", 320.0, 1.40, 0.030, 0.10);

/// Older child (~8-11 y). Pre-puberty for both sexes.
pub const CHILD_OLDER: VoiceProfile = VoiceProfile::new("child_older", 280.0, 1.30, 0.025, 0.08);

// ─── Elderly ──────────────────────────────────────────────────

/// Elderly male (~70+ y). Lower energy, more breath + jitter
/// from atrophied vocal folds.
pub const ELDERLY_MALE: VoiceProfile = VoiceProfile::new("elderly_male", 120.0, 0.97, 0.045, 0.25);

/// Elderly female (~70+ y).
pub const ELDERLY_FEMALE: VoiceProfile =
    VoiceProfile::new("elderly_female", 195.0, 1.12, 0.045, 0.25);

/// All voice presets, in a stable order.
pub const ALL_VOICES: &[VoiceProfile] = &[
    BASS,
    BARITONE,
    TENOR,
    CONTRALTO,
    MEZZO,
    SOPRANO,
    YOUTH_MALE,
    YOUTH_FEMALE,
    CHILD_YOUNG,
    CHILD_OLDER,
    ELDERLY_MALE,
    ELDERLY_FEMALE,
];

impl Default for VoiceProfile {
    fn default() -> Self {
        BARITONE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Presets are ordered by F0 within each broad group.
    /// These are `const` assertions wrapped in tests so the
    /// linter knows the bool is statically known.
    #[test]
    fn male_pitch_ordering() {
        const { assert!(BASS.f0_hz < BARITONE.f0_hz) };
        const { assert!(BARITONE.f0_hz < TENOR.f0_hz) };
    }

    #[test]
    fn female_pitch_ordering() {
        const { assert!(CONTRALTO.f0_hz < MEZZO.f0_hz) };
        const { assert!(MEZZO.f0_hz < SOPRANO.f0_hz) };
    }

    #[test]
    fn child_above_adult() {
        const { assert!(CHILD_YOUNG.f0_hz > SOPRANO.f0_hz) };
    }

    /// Formant scale tracks vocal-tract length physics: shorter
    /// tract (children, female) → higher formants.
    #[test]
    fn formant_scale_tracks_vtl() {
        // Adult male = reference ≈ 1.0.
        const { assert!((TENOR.formant_scale - 1.0).abs() < 0.05) };
        // Female > male.
        const { assert!(SOPRANO.formant_scale > TENOR.formant_scale) };
        // Child > female.
        const { assert!(CHILD_YOUNG.formant_scale > SOPRANO.formant_scale) };
        // Bass < tenor (longer tract).
        const { assert!(BASS.formant_scale < TENOR.formant_scale) };
    }

    /// Elderly voices have more jitter and breath than healthy adults.
    #[test]
    fn elderly_has_more_jitter_and_breath() {
        const { assert!(ELDERLY_MALE.jitter > TENOR.jitter) };
        const { assert!(ELDERLY_MALE.breath > TENOR.breath) };
        const { assert!(ELDERLY_FEMALE.jitter > SOPRANO.jitter) };
        const { assert!(ELDERLY_FEMALE.breath > SOPRANO.breath) };
    }

    /// Every preset is uniquely named.
    #[test]
    fn unique_preset_names() {
        let mut names: Vec<&str> = ALL_VOICES.iter().map(|v| v.name).collect();
        names.sort_unstable();
        let n_unique = names.iter().fold((None::<&&str>, 0), |(prev, n), x| {
            if prev == Some(x) {
                (Some(x), n)
            } else {
                (Some(x), n + 1)
            }
        });
        assert_eq!(n_unique.1, ALL_VOICES.len());
    }
}
