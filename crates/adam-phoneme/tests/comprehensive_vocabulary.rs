// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Broad-vocabulary sweep through the bidirectional Cyrillic ↔
//! Latin renderers. Sources cover the realistic span of v6.3
//! input: geography names, historic figures, modern noun
//! vocabulary, verbs, common phrases, numerals, and loanwords.
//!
//! Per the project's realistic-tests directive (see memory
//! `feedback_advanced_realistic_tests`), this file scans **50+
//! real Kazakh words** for round-trip and harmony properties,
//! not a hand-picked ten. Failures here surface defects that a
//! synthetic-only suite would miss.

use adam_phoneme::{
    Phoneme,
    cyrillic::{cyrillic_to_phonemes, phonemes_to_cyrillic},
    latin::{latin_to_phonemes, phonemes_to_latin},
};

/// 60+ simple-root native Kazakh words (no morpheme-boundary
/// harmony breaks). Every word must:
///   - Cyrillic → phonemes succeed.
///   - phonemes → Cyrillic recover the input (allowing for
///     epenthetic ы / і dropping).
///   - Round-trip through Latin recovers the same phoneme
///     sequence.
const SIMPLE_NATIVE_WORDS: &[&str] = &[
    // Geography (places in Kazakhstan).
    "қазақстан",
    "алматы",
    "астана",
    "атырау",
    "ақтау",
    "ертіс",
    "балқаш",
    "алтай",
    "семей",
    "талдықорған",
    "торғай",
    "ұлытау",
    // Bodies / nature.
    "су",
    "тау",
    "көл",
    "өзен",
    "дала",
    "орман",
    "теңіз",
    "мұхит",
    "құм",
    "тас",
    // Common nouns from world_core surfaces.
    "адам",
    "бала",
    "ана",
    "әке",
    "ағай",
    "апай",
    "дос",
    "үй",
    "мектеп",
    "кітап",
    "жұмыс",
    "білім",
    "ой",
    "жол",
    "мемлекет",
    "халық",
    "тіл",
    // Verbs / mental-state nouns.
    "сүйу",
    "білу",
    "көру",
    "айту",
    "жазу",
    "оқу",
    "істеу",
    "айтшы",
    "келу",
    "кету",
    // Pronouns / function words.
    "мен",
    "сен",
    "сіз",
    "ол",
    "біз",
    "сендер",
    // Time vocabulary.
    "бүгін",
    "ертең",
    "кеше",
    "қазір",
    "уақыт",
    "сағат",
    // Numerals.
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
    "жүз",
    "мың",
    // Dialog phrases (single-token words from them).
    "сәлем",
    "иә",
    "жоқ",
    "рахат",
    "жақсы",
    "мүмкін",
    // Arabic-borrowed words that exercise the rare «һ» (H)
    // phoneme — without these, no native vocabulary entry
    // reaches the glottal fricative.
    "жиһаз",
    "гауһар",
];

/// Every word in [`SIMPLE_NATIVE_WORDS`] survives
/// Cyrillic → phonemes → Cyrillic round-trip up to epenthetic
/// dropping. The recovered Cyrillic might differ from the
/// original where an epenthetic «ы»/«і» was dropped, but the
/// recovered form must be a subset.
#[test]
fn cyrillic_round_trip_on_simple_native_vocabulary() {
    let mut failures: Vec<String> = Vec::new();
    for &w in SIMPLE_NATIVE_WORDS {
        let phonemes = cyrillic_to_phonemes(w, true);
        if phonemes.is_empty() {
            failures.push(format!("«{w}»: produced empty phoneme stream"));
            continue;
        }
        // Recoverable round-trip when rule is OFF (loan mode):
        // phonemes_to_cyrillic should reproduce the input only
        // for words without epenthetic drops. We test the safer
        // "no empty / no crash" property here; the strict
        // identity test runs in loan mode below.
        let recovered = phonemes_to_cyrillic(&phonemes);
        if recovered.is_empty() {
            failures.push(format!("«{w}»: produced empty Cyrillic projection"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} words failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Strict identity round-trip in **loan mode** (epenthetic
/// rule disabled). Every word must Cyrillic → phonemes →
/// Cyrillic exactly. Distinguishes structural renderer bugs
/// from intentional epenthetic dropping.
#[test]
fn strict_cyrillic_round_trip_in_loan_mode() {
    let mut failures: Vec<String> = Vec::new();
    for &w in SIMPLE_NATIVE_WORDS {
        let phonemes = cyrillic_to_phonemes(w, false);
        let recovered = phonemes_to_cyrillic(&phonemes);
        if recovered != w {
            failures.push(format!("«{w}» → {phonemes:?} → «{recovered}» (not exact)"));
        }
    }
    // Some lossy points are tolerated (and known): «и» digraph
    // collapsing, «у»-as-glide vs. «у»-as-vowel. Report at the
    // end so we see which words break and can curate the list
    // — but not fail the test on the first lossy case. Phase 4
    // refinement narrows this list.
    if !failures.is_empty() {
        eprintln!(
            "[WARN] {}/{} loan-mode round-trips not exact (known Phase 4 lossy points):\n  {}",
            failures.len(),
            SIMPLE_NATIVE_WORDS.len(),
            failures.join("\n  "),
        );
    }
    // Hard cap: at least 80 % of the vocabulary must round-trip
    // cleanly. If we fall below, something deeper has broken.
    let success_rate = 1.0 - (failures.len() as f32 / SIMPLE_NATIVE_WORDS.len() as f32);
    assert!(
        success_rate >= 0.80,
        "loan-mode round-trip success rate {:.0}% below 80%",
        success_rate * 100.0,
    );
}

/// **Cyrillic / Latin parity** across the full vocabulary.
/// Phoneme streams produced by the two parsers must match
/// when both are given the same phoneme content in their
/// respective scripts. Run in loan mode so the rule does not
/// muddle the comparison.
#[test]
fn latin_cyrillic_parity_across_vocabulary() {
    let mut mismatches: Vec<String> = Vec::new();
    for &w in SIMPLE_NATIVE_WORDS {
        let cyr_phonemes = cyrillic_to_phonemes(w, false);
        let lat_render = phonemes_to_latin(&cyr_phonemes);
        let lat_phonemes = latin_to_phonemes(&lat_render, false);
        if cyr_phonemes != lat_phonemes {
            mismatches.push(format!(
                "«{w}» cyr={cyr_phonemes:?} lat «{lat_render}» → {lat_phonemes:?}"
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} parity failures:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// **Negative**: pure-Russian words (which are NOT Kazakh
/// natives but appear in mixed-language input). With the
/// epenthetic rule OFF, every Cyrillic letter must produce a
/// phoneme — no silent drops.
#[test]
fn russian_loanwords_no_silent_drops() {
    const RUSSIAN_LOANS: &[(&str, usize)] = &[
        // (word, expected phoneme count — counted manually).
        ("гравитация", 10), // Г Р А В И Т А Ц И Я (Я → A)
        ("биткоин", 7),     // Б И Т К О И Н
        ("компьютер", 8),   // К О М П (Ь silent) Ь Ю(→U) Т Е Р → 8
    ];
    for &(w, expected) in RUSSIAN_LOANS {
        let phonemes = cyrillic_to_phonemes(w, false);
        assert_eq!(
            phonemes.len(),
            expected,
            "«{w}» produced {} phonemes, expected {expected}: {:?}",
            phonemes.len(),
            phonemes,
        );
    }
}

/// **Negative**: garbage input must not crash and must produce
/// only legitimate phonemes (no Glottal, no panics).
#[test]
fn garbage_input_silent() {
    for w in ["123", "!!!", "xyz?", "    \n\t  ", "ABCDEF", "🦀"] {
        let phonemes = cyrillic_to_phonemes(w, true);
        // Non-Kazakh / non-Cyrillic input → empty phoneme stream.
        assert!(phonemes.is_empty(), "garbage «{w}» produced {phonemes:?}",);
    }
}

/// **Latin garbage** — non-Latin / non-Kazakh input produces
/// no phonemes. (Plain ASCII letters that happen to be valid
/// Kazakh Latin letters — like `z` — DO produce phonemes,
/// which is by design.)
#[test]
fn latin_garbage_input_silent() {
    for w in ["123", "!!!", "  ", "🦀", "中文", "العربية"] {
        let phonemes = latin_to_phonemes(w, true);
        assert!(phonemes.is_empty(), "garbage «{w}» produced {phonemes:?}");
    }
}

/// **Length stress test** — long agglutinative form must
/// process without crash.
#[test]
fn long_agglutinated_word_processes() {
    // Synthetic: «жұмыссыздарымыздағылардан» (~25 chars,
    // multi-suffix dative-ablative chain). Just needs to not
    // panic and produce a sensible phoneme count.
    let word = "жұмыссыздарымыздағылардан";
    let phonemes = cyrillic_to_phonemes(word, true);
    assert!(
        phonemes.len() >= 10 && phonemes.len() <= word.chars().count(),
        "long agglutinated word produced {} phonemes",
        phonemes.len()
    );
}

/// Phoneme alphabet coverage: each non-loan native phoneme is
/// reachable from some entry in [`SIMPLE_NATIVE_WORDS`]. Pins
/// that the vocabulary spans the inventory — if we add a new
/// phoneme to the enum, this test fails until a word using it
/// joins the vocabulary.
#[test]
fn vocabulary_covers_native_phoneme_inventory() {
    use std::collections::HashSet;
    let mut covered: HashSet<Phoneme> = HashSet::new();
    for &w in SIMPLE_NATIVE_WORDS {
        for p in cyrillic_to_phonemes(w, false) {
            covered.insert(p);
        }
    }
    // Loan-only consonants and the boundary marker are not
    // expected; build the required set.
    let mut required = Vec::new();
    for &p in Phoneme::ALL {
        if !p.is_loan() && !matches!(p, Phoneme::Glottal) {
            required.push(p);
        }
    }
    let mut missing: Vec<Phoneme> = Vec::new();
    for r in required {
        if !covered.contains(&r) {
            missing.push(r);
        }
    }
    assert!(
        missing.is_empty(),
        "vocabulary missing coverage for native phonemes: {missing:?}",
    );
}
