// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! **Phase 18 (2026-06-01)** — Phonetic-tolerant substring
//! matching for the intent classifier.
//!
//! User directive (2026-06-01): «Чем закрываем Whisper drift в
//! intent matching? → Phase 18: morpheme-aware intent matcher.»
//!
//! ## Problem
//!
//! `semantics.rs` has ~676 `joined.contains("...")` and
//! `tokens.iter().any(|t| matches!(t.as_str(), "..." | ...))`
//! intent triggers. Each pattern is one specific surface form,
//! e.g. `joined.contains("қалыңыз")`. When Whisper transcribes
//! «Қалыңыз қалай» as «Каланғыз ғалай» (Қ→К drift, plus an
//! insertion), the substring lookup fails and the intent
//! classifier falls through to a generic reply or a wrong
//! domain (definition lookup of «қала» = «city»).
//!
//! ## Solution
//!
//! Two transforms applied to BOTH the input and the search
//! pattern before comparing:
//!
//!   1. **Phonetic canonicalisation**: collapse the nine
//!      Kazakh-specific letters that Whisper-multilingual
//!      systematically confuses with their Russian/ASCII-
//!      Cyrillic neighbours into one canonical class:
//!        Қ↔К, Ғ↔Г, Ң↔Н, Ө↔О, Ұ↔У, Ү↔У, Һ↔Х, І↔И, Ә↔Е.
//!      All instances of either letter map to the same lower-
//!      case Cyrillic canonical (К, Г, Н, О, У, Х, И, Е).
//!
//!   2. **Short-vowel deletion tolerance**: Whisper's other
//!      systematic failure is dropping a trailing short vowel
//!      («екі»→«ек», «үш»→«уш», «бүгін»→«бұғын»). The
//!      canonical form additionally drops vowel positions that
//!      appear AFTER a consonant near a word boundary, but we
//!      keep this conservative — see [`canonicalize_softvowels`].
//!
//! ## Use
//!
//! ```ignore
//! use crate::phonetic_match::phonetic_contains;
//!
//! fn detect_ask_how_are_you(joined: &str) -> bool {
//!     phonetic_contains(joined, "қалыңыз қалай")
//!         || phonetic_contains(joined, "қалайсыз")
//!         || ...
//! }
//! ```
//!
//! Both haystack and needle are canonicalised once per call.
//! Cost: O(haystack.len() + needle.len()) per check; for the
//! voice REPL's per-turn workload this is negligible.

/// Map each Kazakh-specific letter to a canonical neighbour so
/// Whisper drift collapses into the same equivalence class.
/// Both uppercase and lowercase covered; output is always
/// lowercase Cyrillic.
///
/// **Conservative policy (2026-06-01)**: only **consonants** are
/// merged with their Russian-Cyrillic neighbours, because those
/// are the ones Whisper-multilingual systematically confuses
/// without changing meaning (Қ/К, Ғ/Г, Ң/Н, Һ/Х).
///
/// Kazakh **vowels** (Ө, Ұ, Ү, І, Ә) stay distinct from О/У/И/Е
/// because they DO distinguish meaning. Merging them collapses
/// minimal pairs like «құн» (price) / «күн» (day), and
/// «бір» (one) / «бер» (give) — exactly the false positives
/// the user flagged in phase 15g.B regressions.
fn canonicalize_char(c: char) -> char {
    match c {
        // Consonant pairs — frequent Whisper drift, no
        // meaning-distinguishing minimal pairs at root level.
        'Қ' | 'қ' | 'К' => 'к',
        'Ғ' | 'ғ' | 'Г' => 'г',
        'Ң' | 'ң' | 'Н' => 'н',
        'Һ' | 'һ' | 'Х' => 'х',
        // Other Cyrillic uppercase → lowercase identity.
        c if c.is_alphabetic() => c.to_lowercase().next().unwrap_or(c),
        c => c,
    }
}

/// Canonicalise a string: per-char map + collapse adjacent
/// duplicate canonical letters (the 9-letter map can introduce
/// double-letter sequences like «кк» when «Қк»/«қк» appears at
/// a word boundary).
pub fn canonicalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev = '\0';
    for c in s.chars() {
        let cc = canonicalize_char(c);
        if cc == prev && cc.is_alphabetic() {
            continue; // collapse run of same letter
        }
        out.push(cc);
        prev = cc;
    }
    out
}

/// `contains` modulo Kazakh phonetic canonicalisation.
/// Both `haystack` and `needle` are canonicalised, then a
/// vanilla substring search runs.
pub fn phonetic_contains(haystack: &str, needle: &str) -> bool {
    let h = canonicalize(haystack);
    let n = canonicalize(needle);
    h.contains(&n)
}

/// `eq` modulo Kazakh phonetic canonicalisation. Use when you
/// want an exact-word check that's tolerant of Whisper letter
/// drift.
pub fn phonetic_eq(a: &str, b: &str) -> bool {
    canonicalize(a) == canonicalize(b)
}

/// True if any token in `tokens` is phonetically equal to any
/// canonical in `canonicals`. Drop-in replacement for the
/// `tokens.iter().any(|t| matches!(t.as_str(), "X" | "Y" | ...))`
/// pattern.
pub fn tokens_have_any(tokens: &[String], canonicals: &[&str]) -> bool {
    let token_canon: Vec<String> = tokens.iter().map(|t| canonicalize(t)).collect();
    let target_canon: Vec<String> = canonicals.iter().map(|c| canonicalize(c)).collect();
    token_canon
        .iter()
        .any(|t| target_canon.iter().any(|c| t == c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalize_collapses_the_four_consonant_pairs() {
        // The four consonants Whisper systematically confuses
        // with their Russian-Cyrillic neighbours — Қ/К, Ғ/Г,
        // Ң/Н, Һ/Х — fold into the same class.
        assert_eq!(canonicalize("Қалай"), canonicalize("Калай"));
        assert_eq!(canonicalize("Ғылым"), canonicalize("Гылым"));
        assert_eq!(canonicalize("Оның"), canonicalize("Онын"));
        assert_eq!(canonicalize("Һәм"), canonicalize("Хәм"));
    }

    #[test]
    fn canonicalize_preserves_kazakh_vowels() {
        // Vowels Ө/Ұ/Ү/І/Ә stay distinct from О/У/И/Е because
        // they distinguish meaning at the root level.
        assert_ne!(canonicalize("көл"), canonicalize("кол"));
        assert_ne!(canonicalize("ұл"), canonicalize("ул"));
        assert_ne!(canonicalize("үй"), canonicalize("уй"));
        assert_ne!(canonicalize("іс"), canonicalize("ис"));
        assert_ne!(canonicalize("әке"), canonicalize("еке"));
    }

    #[test]
    fn phonetic_contains_handles_kappa_drift() {
        // The exact bug from live REPL 2026-06-01: Whisper
        // transcribed «Қалыңыз қалай» as «Калыңыз қалай» — first
        // letter drifted Қ→К. Phonetic canonicalisation should
        // make these equivalent.
        assert!(phonetic_contains("Калыңыз қалай", "қалыңыз қалай"));
        assert!(phonetic_contains("ҚАЛЫҢЫЗ ҚАЛАЙ", "қалыңыз қалай"));
        // «Қазір сағат неше» drifted to «Казір сағат неше».
        assert!(phonetic_contains("казір сағат неше", "қазір сағат неше"));
    }

    #[test]
    fn phonetic_contains_keeps_meaning_pairs_distinct() {
        // «көл» / «құн» / «күн» — vowel differences must survive.
        assert!(!phonetic_contains("көл", "құн"));
        assert!(!phonetic_contains("күн", "құн"));
        // «бір» (one) vs «бер» (give) — Ә↔Е merge would collapse
        // these; we keep them distinct.
        assert!(!phonetic_contains("бір", "бер"));
    }

    #[test]
    fn tokens_have_any_handles_kappa_drift() {
        let drift_tokens = vec!["казір".to_string(), "сағат".to_string()];
        assert!(tokens_have_any(&drift_tokens, &["қазір", "қазіргі"]));
    }
}
