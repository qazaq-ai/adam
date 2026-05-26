// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Top-level phonotactic validator for native Kazakh words.
//!
//! Composes the harmony and syllable layers into a single
//! `validate_native_word` predicate. A word passes iff:
//!
//! 1. It has at least one vowel nucleus.
//! 2. Vowel harmony is pure (all vowels share a class).
//! 3. Onset / coda sizes stay within the Kazakh `(C)V(CC)` shape.
//!
//! Cluster *permission* (which specific CC combinations are
//! allowed) is not enforced at this first pass — Kazakh has too
//! many corpus-evidenced clusters to enumerate from first
//! principles, and over-rejecting kills the gate. Phase 4
//! adds corpus-evidenced cluster lists.
//!
//! Loanwords are validated with a separate caller-supplied
//! `is_native_root: false` (the harmony check is skipped); the
//! shape check still runs because Kazakh's loan adaptations
//! preserve syllable structure even when violating harmony.

use crate::harmony::{HarmonyResult, check_harmony};
use crate::syllable::syllabify;
use adam_phoneme::Phoneme;

/// Reason a phoneme stream is not a well-formed Kazakh word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// The stream has no vowels — cannot form a syllable.
    NoNucleus,
    /// Vowel harmony broken at the indicated stream offset
    /// (`first_break_at`). Only reported when `is_native_root`
    /// is `true`.
    HarmonyBroken { first_break_at: usize },
    /// A syllable's onset has more consonants than Kazakh allows
    /// (max 1). `syllable_index` is the zero-based syllable
    /// position; `onset_len` is the offending size.
    OnsetTooLarge {
        syllable_index: usize,
        onset_len: usize,
    },
    /// A syllable's coda has more consonants than Kazakh allows
    /// (max 2). `syllable_index` is the zero-based syllable
    /// position; `coda_len` is the offending size.
    CodaTooLarge {
        syllable_index: usize,
        coda_len: usize,
    },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoNucleus => write!(f, "no vowel nucleus in stream"),
            Self::HarmonyBroken { first_break_at } => {
                write!(f, "vowel harmony broken at offset {first_break_at}")
            }
            Self::OnsetTooLarge {
                syllable_index,
                onset_len,
            } => write!(
                f,
                "syllable {syllable_index}: onset has {onset_len} consonants (max 1 in Kazakh)"
            ),
            Self::CodaTooLarge {
                syllable_index,
                coda_len,
            } => write!(
                f,
                "syllable {syllable_index}: coda has {coda_len} consonants (max 2 in Kazakh)"
            ),
        }
    }
}

impl std::error::Error for ValidationError {}

/// Maximum onset size in a single Kazakh syllable for native
/// roots. Kazakh forbids initial consonant clusters; loanwords
/// from Russian routinely violate this (e.g. «спорт») and are
/// not checked.
pub const MAX_ONSET: usize = 1;

/// Maximum coda size in a single Kazakh syllable, **at the
/// phonemic level**. Three consonants is the realistic upper
/// bound *after* the epenthetic dropping rule from
/// [`adam_phoneme::cyrillic`] §9 OQ4 has run: «ертіс» becomes
/// `[E, R, T, S]` (one vowel, three-consonant coda); the
/// orthography re-inserts the epenthetic «і» at the surface.
/// First-pass permissive bound; corpus evidence in Phase 4 may
/// tighten it (e.g. forbidding specific 3-segment finals like
/// -ртс as too coronal-heavy).
pub const MAX_CODA: usize = 3;

/// Validate a phoneme stream as a well-formed Kazakh word.
///
/// `is_native_root` controls **both** the harmony and the shape
/// checks:
///
/// - Harmony: native roots must have pure vowel harmony; loans
///   may mix.
/// - Shape: native roots must obey Kazakh's `(C)V(CC(C))` syllable
///   shape; loans may violate (Russian «гравитация» enters with
///   initial CC `[G, R, A, ...]` and is accepted as-is).
///
/// The nucleus-presence check applies unconditionally — a phoneme
/// stream with no vowels is ill-formed regardless of provenance.
pub fn validate_native_word(
    phonemes: &[Phoneme],
    is_native_root: bool,
) -> Result<(), ValidationError> {
    // 1. Nucleus presence — unconditional.
    if !phonemes.iter().any(|p| p.is_vowel()) {
        return Err(ValidationError::NoNucleus);
    }

    // 2 & 3 are native-only.
    if !is_native_root {
        return Ok(());
    }

    // 2. Harmony.
    if let HarmonyResult::Mixed { first_break_at, .. } = check_harmony(phonemes) {
        return Err(ValidationError::HarmonyBroken { first_break_at });
    }

    // 3. Syllable shape.
    for (i, s) in syllabify(phonemes).iter().enumerate() {
        if s.onset.len() > MAX_ONSET {
            return Err(ValidationError::OnsetTooLarge {
                syllable_index: i,
                onset_len: s.onset.len(),
            });
        }
        if s.coda.len() > MAX_CODA {
            return Err(ValidationError::CodaTooLarge {
                syllable_index: i,
                coda_len: s.coda.len(),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use Phoneme::*;
    use adam_phoneme::HarmonyClass;

    /// «қазақ» — passes every check.
    #[test]
    fn qazaq_validates_native() {
        assert_eq!(validate_native_word(&[Q, A, Z, A, Q], true), Ok(()));
    }

    /// «мемлекет» — front-harmonic native, three clean syllables.
    #[test]
    fn memleket_validates_native() {
        assert_eq!(
            validate_native_word(&[M, E, M, L, E, K, E, T], true),
            Ok(())
        );
    }

    /// «жұмыс» post-epenthetic = Zh U M S — single syllable with
    /// 2-consonant coda. Passes (MAX_CODA = 2).
    #[test]
    fn jumys_passes_with_two_coda_consonants() {
        assert_eq!(validate_native_word(&[Zh, U, M, S], true), Ok(()));
    }

    /// No vowels at all → NoNucleus.
    #[test]
    fn empty_or_all_consonants_no_nucleus() {
        assert_eq!(
            validate_native_word(&[], true),
            Err(ValidationError::NoNucleus)
        );
        assert_eq!(
            validate_native_word(&[Q, R, S], true),
            Err(ValidationError::NoNucleus)
        );
    }

    /// Loanword with harmony violation, validated as loan — OK.
    #[test]
    fn loan_with_mixed_harmony_passes_when_loan() {
        // Synthetic shape that has mixed harmony: Front-Back-Front
        // syllable shape is fine, only harmony breaks. With
        // is_native_root=false the harmony check is skipped.
        let v = validate_native_word(&[B, I, Z, A], false);
        assert_eq!(v, Ok(()));
    }

    /// Same loan stream validated as native → harmony break.
    #[test]
    fn loan_with_mixed_harmony_fails_when_native() {
        let v = validate_native_word(&[B, I, Z, A], true);
        assert_eq!(v, Err(ValidationError::HarmonyBroken { first_break_at: 3 }));
    }

    /// Onset too large: synthetic CCV input (no initial clusters
    /// in Kazakh) is rejected when checked.
    #[test]
    fn initial_cluster_rejected_via_onset_size() {
        // S T A — initial CC cluster ends up as 2-segment onset.
        let v = validate_native_word(&[S, T, A], true);
        assert!(matches!(
            v,
            Err(ValidationError::OnsetTooLarge {
                syllable_index: 0,
                onset_len: 2
            })
        ));
    }

    /// Onset 1 + coda 2 is the canonical Kazakh max — accepted.
    #[test]
    fn canonical_cv_cc_shape_accepted() {
        let v = validate_native_word(&[T, U, R, S], true);
        assert_eq!(v, Ok(()));
    }

    /// Coda 3 accepted (post-epenthetic-drop reality).
    #[test]
    fn coda_three_accepted() {
        // E R T S — «ертіс» post-epenthetic, one syllable with
        // coda [R, T, S]. Used to fail; now passes because
        // MAX_CODA = 3.
        assert_eq!(validate_native_word(&[E, R, T, S], true), Ok(()));
    }

    /// Coda 4+ rejected. Synthetic stream `T A R S N D` puts
    /// R S N D all in one coda.
    #[test]
    fn coda_over_three_rejected() {
        let v = validate_native_word(&[T, A, R, S, N, D], true);
        assert!(matches!(
            v,
            Err(ValidationError::CodaTooLarge {
                syllable_index: 0,
                coda_len: 4
            })
        ));
    }

    /// Display format produces a usable message.
    #[test]
    fn error_display_messages() {
        assert_eq!(
            format!("{}", ValidationError::NoNucleus),
            "no vowel nucleus in stream"
        );
        assert_eq!(
            format!("{}", ValidationError::HarmonyBroken { first_break_at: 4 }),
            "vowel harmony broken at offset 4"
        );
    }

    /// Validate consistency with the harmony module.
    #[test]
    fn harmony_pure_means_no_break() {
        let v = validate_native_word(&[Q, A, Z, A, Q], true);
        assert_eq!(v, Ok(()));
        assert_eq!(
            check_harmony(&[Q, A, Z, A, Q]),
            HarmonyResult::Pure(HarmonyClass::Back)
        );
    }
}
