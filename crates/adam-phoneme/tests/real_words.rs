// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Real Kazakh words from `data/world_core` through the phoneme
//! renderer.
//!
//! As of 2026-05-29 the «ы» / «і» strict-orthographic rule
//! (user directive: «ы» / «і» are pure orthographic markers,
//! analogous to Renault's silent letters) means every
//! occurrence drops from a native-root parse — initial,
//! medial, final. These pinning tests reflect the strict rule.

use adam_phoneme::{
    Phoneme,
    Phoneme::*,
    cyrillic::{cyrillic_to_phonemes, phonemes_to_cyrillic},
    latin::{latin_to_phonemes, phonemes_to_latin},
};

/// Geography (from `geography_kz.jsonl` and `world_core` agent
/// surfaces): place names that appear in voice REPL audits.
#[test]
fn geography_round_trips() {
    // «қазақстан» — no «ы»; pure consonant + vowel sequence.
    assert_eq!(
        cyrillic_to_phonemes("қазақстан", true),
        vec![Q, A, Z, A, Q, S, T, A, N]
    );
    // «алматы» — final «ы» drops under the strict rule, leaving
    // /almat/ (final T is its own coda).
    assert_eq!(cyrillic_to_phonemes("алматы", true), vec![A, L, M, A, T]);
    // «астана» — no epenthetics.
    assert_eq!(cyrillic_to_phonemes("астана", true), vec![A, S, T, A, N, A]);
    // «балқаш» (lake) — no «ы».
    assert_eq!(
        cyrillic_to_phonemes("балқаш", true),
        vec![B, A, L, Q, A, Sh]
    );
    // «ертіс» (river) — «і» is in the second syllable between
    // consonants → epenthetic → dropped.
    assert_eq!(cyrillic_to_phonemes("ертіс", true), vec![E, R, T, S]);
    // «алтай» (mountain) — no «ы».
    assert_eq!(cyrillic_to_phonemes("алтай", true), vec![A, L, T, A, J]);
}

/// Historic names: «байтұрсынұлы» under the strict rule drops
/// every «ы» — both the medial and the final.
#[test]
fn baitursynuly_drops_all_orthographic_ы() {
    let ph = cyrillic_to_phonemes("байтұрсынұлы", true);
    assert_eq!(ph, vec![B, A, J, T, U, R, S, N, U, L]);
}

/// Common nouns from physics / state vocabulary.
#[test]
fn common_nouns_round_trip() {
    // «мысал» (example) — «ы» drops, leaving /msal/.
    assert_eq!(cyrillic_to_phonemes("мысал", true), vec![M, S, A, L]);

    // «жұмыс» (work) — «ы» in SECOND syllable between consonants
    // → DROP.
    assert_eq!(cyrillic_to_phonemes("жұмыс", true), vec![Zh, U, M, S]);

    // «жұмыссыз» (unemployed) — both «ы»s in non-initial
    // syllables between consonants → both DROP.
    assert_eq!(
        cyrillic_to_phonemes("жұмыссыз", true),
        vec![Zh, U, M, S, S, Z]
    );

    // «алты» (six) — final «ы» drops, leaving /alt/.
    assert_eq!(cyrillic_to_phonemes("алты", true), vec![A, L, T]);

    // «алтын» (gold) — «ы» between т and н in second syllable
    // → DROP.
    assert_eq!(cyrillic_to_phonemes("алтын", true), vec![A, L, T, N]);

    // «білім» (knowledge) — both «і»s drop → /blm/.
    assert_eq!(cyrillic_to_phonemes("білім", true), vec![B, L, M]);

    // «мемлекет» (state) — no «ы», no «і», no epenthetic
    // candidates. Full round-trip.
    let mml = cyrillic_to_phonemes("мемлекет", true);
    assert_eq!(mml, vec![M, E, M, L, E, K, E, T]);
    assert_eq!(phonemes_to_cyrillic(&mml), "мемлекет");
}

/// Loanwords: the rule is suppressed, every vowel realises.
#[test]
fn loanwords_keep_all_vowels() {
    // «гравитация» — has 'я' which is approximated to A in
    // Phase 1; «и» → I. Mostly we're checking the rule
    // suppression: nothing is dropped.
    let g = cyrillic_to_phonemes("гравитация", false);
    // Г Р А В И Т А Ц И Я → G R A V I T A Ts I A (after lossy
    // 'я' → A approximation).
    assert_eq!(g, vec![G, R, A, V, I, T, A, Ts, I, A]);

    // «биткоин» — «и» appears twice in a loanword; the rule
    // never applies; every «и» realises as I.
    let bit = cyrillic_to_phonemes("биткоин", false);
    assert_eq!(bit, vec![B, I, T, K, O, I, N]);
}

/// Whitespace / punctuation tolerated.
#[test]
fn dialog_phrases() {
    let ph = cyrillic_to_phonemes("Сәлем, Дәке!", true);
    // С Ә Л Е М , (space) Д Ә К Е !
    assert_eq!(ph, vec![S, Ae, L, E, M, D, Ae, K, E]);
}

/// Forward render covers every phoneme in `ALL` (sanity:
/// `cyrillic_glyph` returns Some for every non-boundary).
#[test]
fn all_phonemes_have_a_cyrillic_projection_except_boundary() {
    use adam_phoneme::{Phoneme, PhonemeClass};
    for &p in Phoneme::ALL {
        let g = p.cyrillic_glyph();
        match p.class() {
            PhonemeClass::Boundary => assert!(g.is_none()),
            _ => assert!(g.is_some(), "no glyph for {p:?}"),
        }
    }
}

/// Latin renderer parity with Cyrillic on real words. Both
/// projections of a Kazakh phoneme stream must round-trip in
/// loan mode (rule disabled).
#[test]
fn latin_cyrillic_parity_on_real_words() {
    let test_words: &[&[Phoneme]] = &[
        &[Q, A, Z, A, Q, S, T, A, N],    // қазақстан / qazaqstan
        &[A, L, M, A, T],                // алматы → /almat/ under strict rule
        &[B, A, J, T, U, R, S, N, U, L], // байтұрсынұлы → all «ы» drop
        &[M, E, M, L, E, K, E, T],       // мемлекет / memleket
        &[T, U, R, S],                   // тұрс (synthetic test)
    ];
    for ph in test_words {
        let cyr = phonemes_to_cyrillic(ph);
        let lat = phonemes_to_latin(ph);
        // Both rendered back through their inverse renderer with
        // is_native_root=false (rule off) recover the original.
        assert_eq!(
            cyrillic_to_phonemes(&cyr, false),
            ph.to_vec(),
            "cyr round-trip"
        );
        assert_eq!(
            latin_to_phonemes(&lat, false),
            ph.to_vec(),
            "lat round-trip"
        );
    }
}

/// «байтұрсынұлы» Latin: epenthetic «y» drops mid-word, final
/// «y» stays. Symmetric to the Cyrillic test.
#[test]
fn baitursunuly_latin_drops_all_orthographic_y() {
    // Under the strict v6.3 rule every orthographic «ы» drops
    // regardless of position, so both the medial and final «ы»
    // disappear from the parsed phoneme stream.
    let cyr_phonemes = cyrillic_to_phonemes("байтұрсынұлы", true);
    let lat = phonemes_to_latin(&cyr_phonemes);
    let lat_phonemes = latin_to_phonemes(&lat, true);
    assert_eq!(cyr_phonemes, lat_phonemes);
    assert_eq!(cyr_phonemes, vec![B, A, J, T, U, R, S, N, U, L]);
}

/// Forward render covers every phoneme in `ALL` for Latin too.
#[test]
fn all_phonemes_have_a_latin_projection_except_boundary() {
    use adam_phoneme::{Phoneme, PhonemeClass};
    for &p in Phoneme::ALL {
        let g = p.latin_glyph();
        match p.class() {
            PhonemeClass::Boundary => assert!(g.is_none()),
            _ => assert!(g.is_some(), "no latin glyph for {p:?}"),
        }
    }
}
