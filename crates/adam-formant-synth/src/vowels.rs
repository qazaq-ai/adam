// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Kazakh vowel formant tables.
//!
//! Centre frequencies and bandwidths for F1, F2, F3 of every
//! vowel in the v6.3 inventory. Values come from published
//! Turkic / Kazakh phonetic studies (averages over multiple
//! adult-male recordings); they're approximate but sufficient
//! to give each vowel a distinguishable spectral signature.
//!
//! Coordinates follow the IPA F1/F2 vowel chart:
//!
//! - F1 ↑ correlates with vowel openness (low vowels high F1).
//! - F2 ↑ correlates with vowel frontness (front vowels high F2).
//! - Rounding lowers F2 (and slightly F3).
//!
//! Bandwidths come from Klatt's typical adult-male table
//! (~80 / 90 / 120 Hz for F1 / F2 / F3) — slight variation for
//! nasal-coupled / pharyngeal vowels could be added in v2.

use adam_phoneme::Phoneme;

/// Per-formant `(centre_hz, bandwidth_hz)` pair.
pub type Formant = (f32, f32);

/// Three-formant signature of a vowel: F1, F2, F3.
#[derive(Debug, Clone, Copy)]
pub struct VowelFormants {
    pub f1: Formant,
    pub f2: Formant,
    pub f3: Formant,
}

impl VowelFormants {
    pub fn as_slice(self) -> [Formant; 3] {
        [self.f1, self.f2, self.f3]
    }
}

/// Return F1/F2/F3 for a Kazakh vowel, or `None` for non-vowel
/// phonemes. Values are adult-male averages at neutral pitch
/// (~120 Hz); the calling code can override F0 and the formant
/// pattern stays meaningful.
pub fn formants_of(phoneme: Phoneme) -> Option<VowelFormants> {
    use Phoneme::*;
    Some(match phoneme {
        // Back, open, unrounded.
        A => VowelFormants {
            f1: (750.0, 80.0),
            f2: (1200.0, 90.0),
            f3: (2600.0, 120.0),
        },
        // Front, near-open, unrounded.
        Ae => VowelFormants {
            f1: (700.0, 80.0),
            f2: (1700.0, 90.0),
            f3: (2500.0, 120.0),
        },
        // Back, mid, rounded.
        O => VowelFormants {
            f1: (500.0, 70.0),
            f2: (900.0, 90.0),
            f3: (2500.0, 120.0),
        },
        // Front, mid, rounded.
        Oe => VowelFormants {
            f1: (450.0, 70.0),
            f2: (1500.0, 90.0),
            f3: (2300.0, 120.0),
        },
        // Back, close, rounded ("ұ").
        U => VowelFormants {
            f1: (350.0, 70.0),
            f2: (800.0, 90.0),
            f3: (2400.0, 120.0),
        },
        // Front, close, rounded ("ү").
        Ue => VowelFormants {
            f1: (300.0, 70.0),
            f2: (1700.0, 90.0),
            f3: (2200.0, 120.0),
        },
        // Front, mid, unrounded.
        E => VowelFormants {
            f1: (500.0, 70.0),
            f2: (2000.0, 90.0),
            f3: (2700.0, 120.0),
        },
        // Front, close, unrounded ("и").
        I => VowelFormants {
            f1: (300.0, 70.0),
            f2: (2200.0, 90.0),
            f3: (3000.0, 120.0),
        },
        // Back, close, unrounded ("ы") — Kazakh-specific.
        Y => VowelFormants {
            f1: (400.0, 70.0),
            f2: (1100.0, 90.0),
            f3: (2500.0, 120.0),
        },
        // Front, close, unrounded ("і").
        Yi => VowelFormants {
            f1: (350.0, 70.0),
            f2: (2100.0, 90.0),
            f3: (2900.0, 120.0),
        },
        _ => return None,
    })
}

/// Default F0 (in Hz) for synthesis. 120 Hz ≈ neutral adult
/// male; the corpus generator can override per-utterance.
pub const DEFAULT_F0_HZ: f32 = 120.0;

#[cfg(test)]
mod tests {
    use super::*;

    /// Every vowel in the inventory has a formant signature.
    #[test]
    fn all_vowels_covered() {
        for p in Phoneme::ALL {
            if p.is_vowel() {
                assert!(
                    formants_of(*p).is_some(),
                    "missing formants for vowel {p:?}"
                );
            }
        }
    }

    /// Consonants and boundaries don't have formant signatures.
    #[test]
    fn consonants_have_no_formants() {
        for p in Phoneme::ALL {
            if !p.is_vowel() {
                assert!(
                    formants_of(*p).is_none(),
                    "consonant {p:?} should have no formant entry"
                );
            }
        }
    }

    /// Sanity: front vowels (Ae, E, I, Oe, Ue, Yi) have F2 > 1400 Hz;
    /// back vowels (A, O, U, Y) have F2 < 1400 Hz.
    #[test]
    fn front_back_separation_in_f2() {
        use Phoneme::*;
        let front = [Ae, E, I, Oe, Ue, Yi];
        let back = [A, O, U, Y];
        for p in front {
            let f = formants_of(p).unwrap();
            assert!(
                f.f2.0 > 1400.0,
                "front vowel {p:?} F2 {} should be > 1400",
                f.f2.0
            );
        }
        for p in back {
            let f = formants_of(p).unwrap();
            assert!(
                f.f2.0 < 1400.0,
                "back vowel {p:?} F2 {} should be < 1400",
                f.f2.0
            );
        }
    }

    /// Sanity: close vowels (I, Yi, U, Ue, Y) have F1 ≤ 400 Hz;
    /// open vowel A has F1 ≥ 700 Hz.
    #[test]
    fn openness_in_f1() {
        use Phoneme::*;
        for p in [I, Yi, U, Ue, Y] {
            let f = formants_of(p).unwrap();
            assert!(
                f.f1.0 <= 420.0,
                "close vowel {p:?} F1 {} should be ≤ 420",
                f.f1.0
            );
        }
        let a = formants_of(A).unwrap();
        assert!(a.f1.0 >= 700.0, "A F1 {} should be ≥ 700", a.f1.0);
    }
}
