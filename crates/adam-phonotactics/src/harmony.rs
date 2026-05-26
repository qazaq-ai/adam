// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Vowel harmony.
//!
//! Native Kazakh words are **harmonic** in their vowels: all
//! vowels share a single harmony class — either all Front
//! (`Ä Ö Ü E I Yi`) or all Back (`A O U Y`). Loanwords from
//! Russian and other languages frequently violate this rule
//! (e.g. «гравитация» mixes back `A` with front `I`). The
//! validator's `is_native_root` flag suppresses this check for
//! loan input.
//!
//! The harmony class is what selects the variant of every
//! agglutinative suffix Kazakh adds to a stem. A stem ending
//! in front vowels takes front suffixes; back-vowel stems take
//! back suffixes. That selection logic lives in a future
//! morpheme-suffix crate; this module only diagnoses the stem
//! itself.

use adam_phoneme::{HarmonyClass, Phoneme};

/// Result of checking vowel harmony over a phoneme stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HarmonyResult {
    /// Every vowel shares this harmony class.
    Pure(HarmonyClass),
    /// At least two vowels disagree. `first_class` is the harmony
    /// class of the first vowel seen; `first_break_at` is the
    /// (zero-based) index in the phoneme stream of the first
    /// vowel that disagrees with `first_class`.
    Mixed {
        first_class: HarmonyClass,
        first_break_at: usize,
    },
    /// No vowels at all (e.g. an all-consonant utterance — rare
    /// but possible at the phoneme-stream level after epenthetic
    /// dropping).
    NoVowels,
}

impl HarmonyResult {
    /// `true` if every vowel agrees (or there are no vowels).
    pub fn is_consistent(self) -> bool {
        !matches!(self, Self::Mixed { .. })
    }
}

/// Check vowel harmony across a phoneme stream.
///
/// Boundary markers ([`Phoneme::Glottal`]) are transparent — the
/// stream is scanned past them. Consonants are ignored. The
/// first vowel sets the expected class; subsequent vowels are
/// compared.
pub fn check_harmony(phonemes: &[Phoneme]) -> HarmonyResult {
    let mut expected: Option<HarmonyClass> = None;
    for (i, p) in phonemes.iter().enumerate() {
        let Some(class) = p.harmony_class() else {
            continue;
        };
        match expected {
            None => expected = Some(class),
            Some(prev) if prev == class => {}
            Some(prev) => {
                return HarmonyResult::Mixed {
                    first_class: prev,
                    first_break_at: i,
                };
            }
        }
    }
    match expected {
        Some(c) => HarmonyResult::Pure(c),
        None => HarmonyResult::NoVowels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Phoneme::*;

    /// Back-harmonic word: «қазақ» = Q A Z A Q — all `A` is Back.
    #[test]
    fn qazaq_is_back_harmonic() {
        assert_eq!(
            check_harmony(&[Q, A, Z, A, Q]),
            HarmonyResult::Pure(HarmonyClass::Back)
        );
    }

    /// Front-harmonic word: «білім» (post-epenthetic) = B Yi L M
    /// has one front vowel `Yi`.
    #[test]
    fn bilim_is_front_harmonic() {
        assert_eq!(
            check_harmony(&[B, Yi, L, M]),
            HarmonyResult::Pure(HarmonyClass::Front)
        );
    }

    /// «мемлекет» (state) = M E M L E K E T — all `E` is Front.
    #[test]
    fn memleket_is_front_harmonic() {
        assert_eq!(
            check_harmony(&[M, E, M, L, E, K, E, T]),
            HarmonyResult::Pure(HarmonyClass::Front)
        );
    }

    /// «жұмыс» (post-epenthetic) = Zh U M S — only one vowel `U`
    /// (Back).
    #[test]
    fn jumys_is_back_harmonic() {
        assert_eq!(
            check_harmony(&[Zh, U, M, S]),
            HarmonyResult::Pure(HarmonyClass::Back)
        );
    }

    /// Loanword example: «гравитация» with back `A` and front `I`
    /// — should be detected as mixed.
    #[test]
    fn loanword_is_mixed() {
        // G R A V I T A Ts I A — first vowel A (Back) at index 2,
        // first break is the I at index 4.
        let stream = [G, R, A, V, I, T, A, Ts, I, A];
        assert_eq!(
            check_harmony(&stream),
            HarmonyResult::Mixed {
                first_class: HarmonyClass::Back,
                first_break_at: 4
            }
        );
    }

    /// All-consonant stream: no vowels at all.
    #[test]
    fn no_vowels_recognised() {
        assert_eq!(check_harmony(&[Q, R, S]), HarmonyResult::NoVowels);
    }

    /// Empty stream → no vowels.
    #[test]
    fn empty_stream_no_vowels() {
        assert_eq!(check_harmony(&[]), HarmonyResult::NoVowels);
    }

    /// Boundary marker is transparent — same harmony class either
    /// side.
    #[test]
    fn boundary_marker_is_transparent() {
        assert_eq!(
            check_harmony(&[Q, A, Z, Glottal, A, Q]),
            HarmonyResult::Pure(HarmonyClass::Back)
        );
    }

    /// `is_consistent` returns `false` only for `Mixed`.
    #[test]
    fn is_consistent_predicate() {
        assert!(HarmonyResult::Pure(HarmonyClass::Front).is_consistent());
        assert!(HarmonyResult::Pure(HarmonyClass::Back).is_consistent());
        assert!(HarmonyResult::NoVowels.is_consistent());
        assert!(
            !HarmonyResult::Mixed {
                first_class: HarmonyClass::Back,
                first_break_at: 0
            }
            .is_consistent()
        );
    }
}
