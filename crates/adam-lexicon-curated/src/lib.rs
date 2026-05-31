// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Hand-curated Kazakh lexicon.
//!
//! User directive (2026-05-28):
//!
//! > «Начиная с алфавита, создать все чистые цифровые звуки от
//! >  "А" до "Я". Потом, на базе этих звуков создать все
//! >  существительные казахского языка по алфавиту, по
//! >  количеству звуков и так далее. Например "Ай", "Ата",
//! >  "Аже", "Кун", "Жер" и так далее, по мере увеличения
//! >  звуков в словах. ... Потом перейти на глаголы, суффиксы…»
//!
//! ## Structure
//!
//! Each lexicon entry carries:
//! - `label`: stable kebab-case identifier (used as the WAV
//!   filename root and the manifest key).
//! - `cyrillic`: the canonical Cyrillic spelling.
//! - `phonemes`: phoneme decomposition over the v6.3 inventory.
//! - `pos`: part-of-speech tag.
//! - `category`: a coarse grouping used for indexing
//!   (`alphabet`, `len2`, `noun`, `verb`, `numeral`, …).
//!
//! The lexicon is organised by **growing phoneme count** within
//! each POS bucket. That makes it easy to ablate "what's the
//! recogniser's accuracy on short vs long words" or "what's the
//! per-POS error rate" without re-tagging.

#![forbid(unsafe_code)]

use adam_phoneme::Phoneme;

/// Coarse part-of-speech tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pos {
    /// One phoneme — used for the alphabet entries.
    Phoneme,
    Noun,
    Verb,
    Adjective,
    Pronoun,
    Numeral,
    Interjection,
    Adverb,
}

impl Pos {
    pub fn as_str(self) -> &'static str {
        match self {
            Pos::Phoneme => "phoneme",
            Pos::Noun => "noun",
            Pos::Verb => "verb",
            Pos::Adjective => "adjective",
            Pos::Pronoun => "pronoun",
            Pos::Numeral => "numeral",
            Pos::Interjection => "interjection",
            Pos::Adverb => "adverb",
        }
    }
}

/// One lexicon entry.
#[derive(Debug, Clone)]
pub struct LexEntry {
    pub label: &'static str,
    pub cyrillic: &'static str,
    pub phonemes: &'static [Phoneme],
    pub pos: Pos,
    pub category: &'static str,
}

impl LexEntry {
    pub const fn new(
        label: &'static str,
        cyrillic: &'static str,
        phonemes: &'static [Phoneme],
        pos: Pos,
        category: &'static str,
    ) -> Self {
        Self {
            label,
            cyrillic,
            phonemes,
            pos,
            category,
        }
    }
}

mod alphabet;
mod expanded;
mod frequency;
mod misc;
mod nouns;
mod verbs;

/// Every curated entry across every category, in stable order:
/// alphabet → nouns by length → verbs → adjectives → pronouns
/// → numerals → interjections → adverbs.
pub fn full_lexicon() -> Vec<LexEntry> {
    let mut out = Vec::new();
    out.extend_from_slice(alphabet::ALPHABET);
    out.extend_from_slice(nouns::NOUNS_LEN2);
    out.extend_from_slice(nouns::NOUNS_LEN3);
    out.extend_from_slice(nouns::NOUNS_LEN4);
    out.extend_from_slice(nouns::NOUNS_LEN5PLUS);
    out.extend_from_slice(verbs::VERBS);
    out.extend_from_slice(misc::ADJECTIVES);
    out.extend_from_slice(misc::PRONOUNS);
    out.extend_from_slice(misc::NUMERALS);
    out.extend_from_slice(misc::INTERJECTIONS);
    out.extend_from_slice(misc::ADVERBS);
    out.extend_from_slice(expanded::EXPANDED);
    out.extend_from_slice(frequency::FREQUENCY);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use adam_phoneme::cyrillic::cyrillic_to_phonemes_prayer_aware;
    use std::collections::HashSet;

    #[test]
    fn full_lexicon_is_non_empty() {
        let lex = full_lexicon();
        assert!(
            lex.len() >= 200,
            "lexicon should have ≥200 entries, has {}",
            lex.len()
        );
    }

    /// Every label is unique — labels become WAV filenames.
    #[test]
    fn labels_are_unique() {
        let lex = full_lexicon();
        let mut seen: HashSet<&str> = HashSet::new();
        for e in &lex {
            assert!(
                seen.insert(e.label),
                "duplicate label: {} ({})",
                e.label,
                e.cyrillic
            );
        }
    }

    /// Each entry's declared phoneme sequence must equal
    /// `cyrillic_to_phonemes(cyrillic, /*native*/ true)`.
    ///
    /// This is the **right direction** — it exercises the
    /// epenthetic-vowel rule from design doc §9 OQ4 (orthographic
    /// «ы» / «і» between consonants in a non-initial syllable do
    /// NOT appear in the phonemic stream). A reverse-direction
    /// test (phonemes_to_cyrillic) cannot catch a declared
    /// transcript that wrongly includes a phantom Y / Yi because
    /// it would dutifully re-emit the orthographic glyph from
    /// the phoneme — exactly the mine the user flagged.
    #[test]
    fn declared_phonemes_match_cyrillic_parse() {
        let mut bad: Vec<String> = Vec::new();
        for e in full_lexicon() {
            // Phase 11: prayer-aware so any future religious
            // lexicon entry (e.g. «бісмілләһ») produces matching
            // phonemes. Existing 312 native-KZ entries fall
            // through the early-return path and validate
            // bit-identically.
            let parsed =
                cyrillic_to_phonemes_prayer_aware(e.cyrillic, /* is_native_root */ true);
            if parsed != e.phonemes {
                bad.push(format!(
                    "  {} declared {:?} but cyrillic_to_phonemes(«{}») = {:?}",
                    e.label, e.phonemes, e.cyrillic, parsed
                ));
            }
        }
        assert!(
            bad.is_empty(),
            "{} round-trip failures (cyrillic→phonemes direction):\n{}",
            bad.len(),
            bad.join("\n")
        );
    }

    /// All categories have at least one entry — a smoke test
    /// against forgetting to push a slice into `full_lexicon()`.
    #[test]
    fn every_pos_present() {
        let lex = full_lexicon();
        for pos in [
            Pos::Phoneme,
            Pos::Noun,
            Pos::Verb,
            Pos::Adjective,
            Pos::Pronoun,
            Pos::Numeral,
            Pos::Interjection,
            Pos::Adverb,
        ] {
            assert!(
                lex.iter().any(|e| e.pos == pos),
                "no entries for POS {:?}",
                pos
            );
        }
    }
}
