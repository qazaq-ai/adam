// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Phoneme-native lexicon.
//!
//! Each [`PhonemeRoot`] stores a Kazakh root as a phoneme
//! sequence plus minimal metadata (POS, original Cyrillic
//! surface for human reference). No grapheme is involved in
//! morphology — Cyrillic is only carried for diagnostics.

use adam_phoneme::Phoneme;

/// Coarse part-of-speech. Keep small; finer distinctions
/// (e.g. transitive / intransitive verb) live in the
/// downstream sem-frame layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartOfSpeech {
    Noun,
    Verb,
    Adjective,
    Pronoun,
    Other,
}

/// One lexical entry: phoneme sequence + POS + Cyrillic
/// surface (for diagnostics / Cyrillic-output rendering).
#[derive(Debug, Clone)]
pub struct PhonemeRoot {
    pub phonemes: Vec<Phoneme>,
    pub pos: PartOfSpeech,
    /// Original Cyrillic form (cached for `Display`-style
    /// rendering and human-readable test output).
    pub cyrillic: &'static str,
}

/// `const`-friendly spec used to declare lexicon entries at
/// compile time; convert to owned [`PhonemeRoot`] via
/// [`From`].
pub struct RootSpec {
    pub phonemes: &'static [Phoneme],
    pub pos: PartOfSpeech,
    pub cyrillic: &'static str,
}

impl RootSpec {
    pub const fn new(
        phonemes: &'static [Phoneme],
        pos: PartOfSpeech,
        cyrillic: &'static str,
    ) -> Self {
        Self {
            phonemes,
            pos,
            cyrillic,
        }
    }
}

impl From<&RootSpec> for PhonemeRoot {
    fn from(s: &RootSpec) -> Self {
        Self {
            phonemes: s.phonemes.to_vec(),
            pos: s.pos,
            cyrillic: s.cyrillic,
        }
    }
}

/// A small curated sample lexicon for the Phase 9 POC. 15
/// roots spanning the four POS classes and the harmony +
/// coda variation needed to exercise all suffix variants.
pub fn sample_lexicon() -> Vec<PhonemeRoot> {
    use PartOfSpeech::*;
    use Phoneme::*;
    let specs: &[RootSpec] = &[
        // Vowel-final, back harmony.
        RootSpec::new(&[B, A, L, A], Noun, "бала"),
        RootSpec::new(&[A, T, A], Noun, "ата"),
        // Vowel-final, front harmony.
        RootSpec::new(&[Sh, E, Sh, E], Noun, "шеше"),
        // Voiceless-final, back harmony.
        RootSpec::new(&[T, A, S], Noun, "тас"),
        RootSpec::new(&[Q, A, Z, A, Q], Noun, "қазақ"),
        // Voiceless-final, front harmony.
        RootSpec::new(&[K, Yi, T, A, P], Noun, "кітап"),
        // Voiced-sonorant-final, back harmony.
        RootSpec::new(&[A, D, A, M], Noun, "адам"),
        RootSpec::new(&[Q, O, L], Noun, "қол"),
        // Voiced-sonorant-final, front harmony.
        RootSpec::new(&[K, Oe, L], Noun, "көл"),
        RootSpec::new(&[Ue, J], Noun, "үй"),
        // Verb roots (consonant-final, back / front).
        RootSpec::new(&[Zh, A, Z], Verb, "жаз"),
        RootSpec::new(&[K, Oe, R], Verb, "көр"),
        // Pronouns.
        RootSpec::new(&[M, E, N], Pronoun, "мен"),
        RootSpec::new(&[S, E, N], Pronoun, "сен"),
        RootSpec::new(&[O, L], Pronoun, "ол"),
    ];
    specs.iter().map(PhonemeRoot::from).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_lexicon_size_and_pos() {
        let lex = sample_lexicon();
        assert_eq!(lex.len(), 15);
        let n_noun = lex.iter().filter(|r| r.pos == PartOfSpeech::Noun).count();
        let n_verb = lex.iter().filter(|r| r.pos == PartOfSpeech::Verb).count();
        let n_pron = lex
            .iter()
            .filter(|r| r.pos == PartOfSpeech::Pronoun)
            .count();
        assert_eq!(n_noun, 10);
        assert_eq!(n_verb, 2);
        assert_eq!(n_pron, 3);
    }

    /// Round-trip a sample of roots back to Cyrillic via the
    /// renderer to prove the phoneme encoding matches the
    /// stated surface.
    #[test]
    fn roots_render_back_to_cyrillic() {
        use adam_phoneme::cyrillic::phonemes_to_cyrillic;
        for root in sample_lexicon() {
            let rendered = phonemes_to_cyrillic(&root.phonemes);
            assert_eq!(
                rendered, root.cyrillic,
                "root «{}» phonemes {:?} → «{}»",
                root.cyrillic, root.phonemes, rendered
            );
        }
    }
}
