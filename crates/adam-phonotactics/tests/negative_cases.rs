// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! **Negative tests** for the phonotactic validator.
//!
//! Per `feedback_advanced_realistic_tests`: for every positive
//! contract (this word passes), we also pin a counter-example
//! (this synthetic stream with a known defect must fail with
//! a specific error). Positive-only tests under-constrain.

use adam_phoneme::Phoneme::*;
use adam_phonotactics::{ValidationError, validate_native_word};

/// Empty stream → `NoNucleus`.
#[test]
fn empty_stream_rejected() {
    assert_eq!(
        validate_native_word(&[], true),
        Err(ValidationError::NoNucleus)
    );
}

/// Consonant-only streams no longer auto-reject after the
/// 2026-05-29 strict-orthographic rule support: sonorants (and
/// failing that, the last consonant) serve as syllabic nuclei.
/// They may still fail OTHER checks (onset cluster, harmony,
/// etc.) but not `NoNucleus`.
#[test]
fn consonant_streams_dont_fail_no_nucleus() {
    let cases: &[Vec<adam_phoneme::Phoneme>] = &[
        vec![Q],                   // Q → fallback last-consonant nucleus
        vec![Q, R],                // R sonorant nucleus
        vec![S, T, R],             // R sonorant nucleus
        vec![Q, Z, Q, S, T, N],    // N sonorant nucleus
        vec![B, J, T, R, S, N, L], // J/R/N/L sonorant nuclei
    ];
    for c in cases {
        let v = validate_native_word(c, true);
        assert!(
            !matches!(v, Err(ValidationError::NoNucleus)),
            "{c:?} unexpectedly returned NoNucleus (got {v:?})"
        );
    }
}

/// Word-initial consonant clusters → `OnsetTooLarge`. Native
/// Kazakh forbids any onset with > 1 consonant; Russian loans
/// import such clusters but must be validated with
/// `is_native_root=false`.
#[test]
fn initial_clusters_rejected_in_native_mode() {
    let cases: &[Vec<adam_phoneme::Phoneme>] = &[
        vec![S, T, A],          // "st-" cluster
        vec![P, R, O],          // "pr-"
        vec![K, R, A, J],       // "kr-"
        vec![Z, D, R, A],       // "zdr-" (3-segment cluster)
        vec![S, T, R, E, S, S], // "str-"
    ];
    for c in cases {
        match validate_native_word(c, true) {
            Err(ValidationError::OnsetTooLarge {
                syllable_index: 0, ..
            }) => {}
            other => panic!("expected OnsetTooLarge for {c:?}, got {other:?}"),
        }
    }
}

/// Same clusters accepted in loan mode (no shape check).
#[test]
fn initial_clusters_accepted_in_loan_mode() {
    let cases: &[Vec<adam_phoneme::Phoneme>] = &[vec![S, T, A], vec![P, R, O], vec![K, R, A, J]];
    for c in cases {
        assert!(
            validate_native_word(c, false).is_ok(),
            "loan mode rejected {c:?}"
        );
    }
}

/// Codas > 3 → `CodaTooLarge`.
#[test]
fn coda_over_three_rejected() {
    // Synthetic single-vowel word with 4 consonants after.
    let stream = vec![T, A, R, S, N, D];
    match validate_native_word(&stream, true) {
        Err(ValidationError::CodaTooLarge { coda_len, .. }) => {
            assert!(coda_len >= 4, "coda was {coda_len}");
        }
        other => panic!("expected CodaTooLarge, got {other:?}"),
    }
}

/// Coda = 3 accepted (post-epenthetic-drop reality, e.g. ертіс).
#[test]
fn coda_three_accepted() {
    assert!(validate_native_word(&[E, R, T, S], true).is_ok());
}

/// Mixed harmony — front + back vowels in native mode → fail.
#[test]
fn mixed_harmony_rejected() {
    let cases: &[Vec<adam_phoneme::Phoneme>] = &[
        vec![Q, A, Z, E],  // A (back) + E (front)
        vec![B, I, Z, A],  // I (front) + A (back)
        vec![M, A, M, Yi], // A (back) + Yi (front)
    ];
    for c in cases {
        match validate_native_word(c, true) {
            Err(ValidationError::HarmonyBroken { .. }) => {}
            other => panic!("expected HarmonyBroken for {c:?}, got {other:?}"),
        }
    }
}

/// Each error class produces a non-empty Display message
/// (useful for diagnostics in REPL / logs).
#[test]
fn all_error_classes_display() {
    let cases: &[ValidationError] = &[
        ValidationError::NoNucleus,
        ValidationError::HarmonyBroken { first_break_at: 5 },
        ValidationError::OnsetTooLarge {
            syllable_index: 0,
            onset_len: 3,
        },
        ValidationError::CodaTooLarge {
            syllable_index: 1,
            coda_len: 4,
        },
    ];
    for e in cases {
        let s = format!("{e}");
        assert!(!s.is_empty(), "empty Display for {e:?}");
        assert!(s.len() < 200, "Display too long for {e:?}: {s}");
    }
}

/// **Boundary**: single-vowel word — onset 0, coda 0 — must
/// pass (e.g. interjections like «А!»).
#[test]
fn single_vowel_word_accepted() {
    assert!(validate_native_word(&[A], true).is_ok());
    assert!(validate_native_word(&[Yi], true).is_ok());
}

/// **Boundary**: CV (single consonant + vowel) — must pass.
#[test]
fn cv_shape_accepted() {
    assert!(validate_native_word(&[Q, A], true).is_ok());
    assert!(validate_native_word(&[M, E], true).is_ok());
}

/// **Boundary**: V onset = 0, coda = 0 even with 2 syllables.
#[test]
fn vowel_hiatus_accepted() {
    // [A, A] — two syllables, both with empty onset / coda.
    assert!(validate_native_word(&[A, A], true).is_ok());
}

/// **Boundary**: maximally complex valid native word.
/// Onset 1 + Vowel + Coda 3 — synthetic but within bounds.
#[test]
fn max_complexity_native_word_accepted() {
    assert!(validate_native_word(&[Q, A, R, S, T], true).is_ok());
}

/// **Counter-example to `OnsetTooLarge`**: a CCV at a non-
/// initial syllable boundary is detected by the syllabifier
/// as a coda-onset split, not a cluster onset. Confirms the
/// onset-too-large error is reserved for word-initial CCs.
#[test]
fn medial_cc_resolves_via_syllabification_not_onset_error() {
    // V C C V — the medial CC splits as coda+onset, neither
    // exceeds size 1, so no error.
    let r = validate_native_word(&[A, L, T, A], true);
    assert!(
        r.is_ok(),
        "medial CC should split via MOP, not trip OnsetTooLarge: {r:?}",
    );
}

/// Loan harmony violations pass loan validation.
#[test]
fn loan_harmony_violations_pass_in_loan_mode() {
    let stream = vec![B, I, Z, A]; // mixed front/back
    assert!(validate_native_word(&stream, false).is_ok());
}
