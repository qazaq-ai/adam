// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Real Kazakh words from `data/world_core` through the
//! phonotactic validator. Pins the v6.3 §3.4 contract against
//! actual production vocabulary.
//!
//! Words are converted from Cyrillic via [`adam_phoneme::cyrillic`],
//! which already applies the §9 OQ4 epenthetic rule. The
//! phonotactic layer then validates the result.

use adam_phoneme::cyrillic::cyrillic_to_phonemes;
use adam_phonotactics::{HarmonyResult, check_harmony, syllabify, validate_native_word};

/// **Simple** (non-compound) curated Kazakh place names from
/// `world_core` — single root, pure harmony, single-word
/// validation.
#[test]
fn simple_place_names_validate() {
    for word in [
        "қазақстан",
        "алматы",
        "астана",
        "балқаш",
        "ертіс",
        "алтай",
        "атырау",
        "ақтау",
        "семей",
    ] {
        let phonemes = cyrillic_to_phonemes(word, true);
        let v = validate_native_word(&phonemes, true);
        assert!(
            v.is_ok(),
            "simple place name «{word}» failed validation: {v:?} (phonemes: {phonemes:?})"
        );
    }
}

/// Compound place names (köK-shetau, aQ-töbe, shym-kent, …) are
/// **expected** to fail single-root native validation because
/// vowel harmony does not span compound morpheme boundaries in
/// Kazakh. This is the v6.3 compound-detection backlog item —
/// the validator will get a `validate_compound_word` entry
/// point in a later phase that splits the input at the
/// morpheme boundary and validates each part independently.
///
/// Until then, these words are routed through the loan branch.
#[test]
fn compound_place_names_fail_native_pass_loan() {
    for word in ["көкшетау", "ақтөбе", "шымкент"] {
        let phonemes = cyrillic_to_phonemes(word, true);
        // Native check — expected to fail. The specific failure
        // mode depends on the word: «көкшетау» and «ақтөбе»
        // break harmony (front + back); «шымкент» now drops «ы»
        // under the strict rule, yielding [Sh, M, K, E, N, T]
        // whose first syllable has a 3-consonant onset and
        // trips `OnsetTooLarge` before reaching the harmony
        // check. Either rejection mode is correct.
        let native_check = validate_native_word(&phonemes, true);
        assert!(
            native_check.is_err(),
            "expected compound «{word}» to fail native validation; got {native_check:?}"
        );
        // Loan-mode check — passes (no harmony or shape gate).
        let loan_check = validate_native_word(&phonemes, false);
        assert!(
            loan_check.is_ok(),
            "compound «{word}» failed loan validation: {loan_check:?}"
        );
    }
}

/// «байтұрсынұлы» — the full historic name, with two «ы»
/// (one dropped, one kept) — must validate as native.
#[test]
fn baitursunuly_validates() {
    let phonemes = cyrillic_to_phonemes("байтұрсынұлы", true);
    let v = validate_native_word(&phonemes, true);
    assert!(v.is_ok(), "validation failed: {v:?}");
    // And it should be back-harmonic.
    assert!(check_harmony(&phonemes).is_consistent());
}

/// Common native dialog vocabulary — every word passes
/// validation and produces 1+ syllable.
///
/// «рахмет» is intentionally absent — it is an Arabic-origin
/// loan and mixes back-A with front-E (phonologically not
/// native, even though it is lexically integrated). See
/// [`loanword_rahmet_passes_as_loan`] below.
#[test]
fn dialog_vocabulary_validates() {
    for word in [
        "сәлем",
        "жоқ",
        "иә",
        "мен",
        "сен",
        "адам",
        "білім",
        "уақыт",
        "бүгін",
        "қазір",
        "мемлекет",
    ] {
        let phonemes = cyrillic_to_phonemes(word, true);
        let v = validate_native_word(&phonemes, true);
        assert!(
            v.is_ok(),
            "dialog word «{word}» failed: {v:?} (phonemes: {phonemes:?})"
        );
        let syls = syllabify(&phonemes);
        assert!(!syls.is_empty(), "no syllables for «{word}»");
    }
}

/// «рахмет» phonologically violates harmony (A back + E front),
/// so it MUST be validated as a loan even though Kazakh speakers
/// use it daily.
#[test]
fn loanword_rahmet_passes_as_loan() {
    let phonemes = cyrillic_to_phonemes("рахмет", false);
    assert!(
        validate_native_word(&phonemes, false).is_ok(),
        "«рахмет» failed loan-validation: phonemes {phonemes:?}"
    );
    let native_check = validate_native_word(&phonemes, true);
    assert!(
        matches!(
            native_check,
            Err(adam_phonotactics::ValidationError::HarmonyBroken { .. })
        ),
        "«рахмет» treated as native, expected HarmonyBroken: {native_check:?}"
    );
}

/// Loanwords pass when flagged as loan, fail (on harmony) when
/// flagged as native — confirming the gate behaves correctly.
#[test]
fn loanwords_gate_correctly() {
    // «гравитация» mixes back-A with front-I — must fail native
    // validation, pass loan validation.
    let phonemes = cyrillic_to_phonemes("гравитация", false);
    assert!(
        validate_native_word(&phonemes, false).is_ok(),
        "loan failed loan-validation"
    );
    let native_check = validate_native_word(&phonemes, true);
    assert!(
        matches!(
            native_check,
            Err(adam_phonotactics::ValidationError::HarmonyBroken { .. })
        ),
        "loan was treated as native, expected HarmonyBroken: {native_check:?}"
    );
}

/// Numbers (often in dialog) — «бір», «екі», «үш», «төрт»,
/// «бес», «алты», «жеті», «сегіз», «тоғыз», «он».
#[test]
fn small_numerals_validate() {
    for word in [
        "бір",
        "екі",
        "үш",
        "төрт",
        "бес",
        "алты",
        "жеті",
        "сегіз",
        "тоғыз",
        "он",
    ] {
        let phonemes = cyrillic_to_phonemes(word, true);
        let v = validate_native_word(&phonemes, true);
        assert!(
            v.is_ok(),
            "numeral «{word}» failed validation: {v:?} (phonemes: {phonemes:?})"
        );
    }
}

/// «қыз» under the strict orthographic rule → /qz/, a
/// consonant cluster. Phase 12 step 4 added sonority-tier
/// nucleus selection — Z (the last consonant) becomes the
/// nucleus and the word validates as one syllable.
#[test]
fn qyz_validates_as_consonant_cluster() {
    use adam_phoneme::Phoneme::*;
    let phonemes = cyrillic_to_phonemes("қыз", true);
    assert_eq!(phonemes, vec![Q, Z]);
    let v = validate_native_word(&phonemes, true);
    assert!(v.is_ok(), "qyz failed: {v:?}");
    let syls = syllabify(&phonemes);
    assert_eq!(syls.len(), 1);
}

/// Sanity: every harmony result on real words is `Pure` OR
/// `NoVowels` (consonant-only words under the strict rule
/// trivially have no harmony to check).
#[test]
fn all_native_words_are_pure_harmonic() {
    for word in [
        "қазақстан",
        "алматы",
        "ертіс",
        "мемлекет",
        "білім",
        "байтұрсынұлы",
        "балқаш",
    ] {
        let phonemes = cyrillic_to_phonemes(word, true);
        let h = check_harmony(&phonemes);
        assert!(
            matches!(h, HarmonyResult::Pure(_) | HarmonyResult::NoVowels),
            "non-pure / non-NoVowels harmony for «{word}»: {h:?}"
        );
    }
}
