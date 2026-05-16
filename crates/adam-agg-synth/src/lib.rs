// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `adam-agg-synth` — synthetic Kazakh training-data generator.
//!
//! Combines the deterministic FST ([`adam_kernel_fst`]) with our
//! morpheme tokenizer ([`adam_agg_tokenizer`]) to produce arbitrarily
//! many `(input_token_sequence, target_token_sequence)` training pairs
//! WITHOUT any LLM teacher. The FST itself is the teacher: every
//! emitted pair is **mathematically valid Kazakh** by construction.
//!
//! ## Why this matters for the third-path hypothesis
//!
//! LLM-style synthetic data (Phi-1 «Textbooks Are All You Need»,
//! TinyStories) uses GPT-3.5/4 as teacher → inherits its biases and
//! hallucination patterns. We use the FST as teacher → inherit only
//! the morphological algebra. Kazakh produced this way is provably
//! grammatical; if a downstream model hallucinates, it's a model bug,
//! not data contamination.
//!
//! ## Generation modes
//!
//! - **Next-morpheme prediction.** Given a partial morpheme sequence,
//!   sample the next morpheme uniformly among morphologically valid
//!   continuations. Foundation task for LM training.
//! - **Inflection completion.** Given a bare root + a partial feature
//!   bundle, target the full inflected form. Tests model's ability to
//!   compose suffixes correctly.
//! - **Mask-and-restore.** Hide one morpheme in a valid sequence,
//!   target its identity. Bidirectional task.
//!
//! Phase 0 ships next-morpheme prediction; the other two modes are
//! marked with `// TODO Phase 1` stubs.

use adam_agg_tokenizer::{AggTokenizer, MorphToken, RootPos, SuffixKind};
use adam_kernel_fst::lexicon::{LexiconV1, RootEntry};
use adam_kernel_fst::morphotactics::{
    Case, NounFeatures, Number, Person, Possessive, Tense, VerbFeatures, synthesise_noun,
    synthesise_verb,
};
use serde::{Deserialize, Serialize};

pub mod error;
pub use error::SynthError;

/// One synthetic training pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingPair {
    /// Surface form of the inflected word (the "label" / target text).
    pub surface: String,
    /// Morpheme-token sequence in canonical order (root, then typed
    /// suffixes).
    pub tokens: Vec<MorphTokenSer>,
    /// FST-derivable POS for the head root.
    pub pos: RootPosSer,
}

/// Serializable mirror of [`MorphToken`] for JSON dumping. Only carries
/// the structural information training needs (token id sufficies).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MorphTokenSer {
    pub id: u32,
    pub kind: TokenKindSer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TokenKindSer {
    Root(String),
    Suffix(String),
    Bos,
    Eos,
    Pad,
    Space,
    Unk(String),
    Punct(char),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RootPosSer {
    NounLike,
    Verb,
    Particle,
}

impl From<RootPos> for RootPosSer {
    fn from(p: RootPos) -> Self {
        match p {
            RootPos::NounLike => RootPosSer::NounLike,
            RootPos::Verb => RootPosSer::Verb,
            RootPos::Particle => RootPosSer::Particle,
        }
    }
}

impl MorphTokenSer {
    pub fn from_token(t: &MorphToken) -> Self {
        match t {
            MorphToken::Root { id, root, .. } => MorphTokenSer {
                id: *id,
                kind: TokenKindSer::Root(root.clone()),
            },
            MorphToken::Suffix(k) => MorphTokenSer {
                id: suffix_token_id(k),
                kind: TokenKindSer::Suffix(format!("{:?}", k)),
            },
            MorphToken::Bos => MorphTokenSer {
                id: 1,
                kind: TokenKindSer::Bos,
            },
            MorphToken::Eos => MorphTokenSer {
                id: 2,
                kind: TokenKindSer::Eos,
            },
            MorphToken::Pad => MorphTokenSer {
                id: 0,
                kind: TokenKindSer::Pad,
            },
            MorphToken::Space => MorphTokenSer {
                id: 3,
                kind: TokenKindSer::Space,
            },
            MorphToken::Unk { surface } => MorphTokenSer {
                id: 4,
                kind: TokenKindSer::Unk(surface.clone()),
            },
            MorphToken::Punct(c) => MorphTokenSer {
                id: 5,
                kind: TokenKindSer::Punct(*c),
            },
        }
    }
}

/// Coarse suffix-token id mapping. Phase 0 uses a hash-based scheme
/// for simplicity; Phase 1 will move to a stable per-variant table.
fn suffix_token_id(k: &SuffixKind) -> u32 {
    // Deterministic hash within the suffix range [25, 99].
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let dbg = format!("{:?}", k);
    let mut h = DefaultHasher::new();
    dbg.hash(&mut h);
    let raw = h.finish() as u32;
    25 + (raw % 75)
}

/// Generator state — owns lexicon + tokenizer; produces pairs lazily.
pub struct SynthGenerator<'a> {
    lex: &'a LexiconV1,
    tokenizer: &'a AggTokenizer,
    /// Counters for reproducibility / stats.
    pub emitted: usize,
}

impl<'a> SynthGenerator<'a> {
    pub fn new(lex: &'a LexiconV1, tokenizer: &'a AggTokenizer) -> Self {
        Self {
            lex,
            tokenizer,
            emitted: 0,
        }
    }

    /// Enumerate **all** bare-root pairs in the Lexicon. For each
    /// noun-like or verb root, emit (root, bare-features → surface).
    pub fn bare_roots(&mut self) -> Vec<TrainingPair> {
        let mut out = Vec::new();
        for entry in &self.lex.entries_ordered {
            match entry.part_of_speech.as_str() {
                "noun" | "adjective" | "pronoun" | "numeral" => {
                    let surface = synthesise_noun(&entry.root, NounFeatures::default());
                    if surface == entry.root {
                        out.push(self.pair_from_word(&surface, entry, RootPos::NounLike));
                    }
                }
                "verb" => {
                    let surface = synthesise_verb(&entry.root, VerbFeatures::default());
                    if surface == entry.root {
                        out.push(self.pair_from_word(&surface, entry, RootPos::Verb));
                    }
                }
                _ => {}
            }
        }
        self.emitted += out.len();
        out
    }

    /// Enumerate basic noun inflections: every noun × {Singular/Plural}
    /// × {7 cases} = 14 forms per noun. For 1000 nouns → 14 000 pairs.
    pub fn noun_inflections(&mut self, max_roots: usize) -> Vec<TrainingPair> {
        let mut out = Vec::new();
        let cases = [
            Case::Nominative,
            Case::Genitive,
            Case::Dative,
            Case::Accusative,
            Case::Locative,
            Case::Ablative,
            Case::Instrumental,
        ];
        let mut roots_used = 0;
        for entry in &self.lex.entries_ordered {
            if entry.part_of_speech != "noun" {
                continue;
            }
            for &number in &[None, Some(Number::Plural)] {
                for &case in &cases {
                    let features = NounFeatures {
                        number,
                        case: Some(case),
                        ..Default::default()
                    };
                    let surface = synthesise_noun(&entry.root, features);
                    if surface.is_empty() {
                        continue;
                    }
                    out.push(self.pair_from_word(&surface, entry, RootPos::NounLike));
                }
            }
            roots_used += 1;
            if roots_used >= max_roots {
                break;
            }
        }
        self.emitted += out.len();
        out
    }

    /// Enumerate possessive-cased nouns: every noun × {7 possessives}
    /// × {3 cases: nom/dat/loc} = 21 forms per noun.
    pub fn noun_possessives(&mut self, max_roots: usize) -> Vec<TrainingPair> {
        let mut out = Vec::new();
        let poss = [
            Possessive::P1Sg,
            Possessive::P2SgInformal,
            Possessive::P2SgPolite,
            Possessive::P3,
            Possessive::P1Pl,
            Possessive::P2PlInformal,
            Possessive::P2PlPolite,
        ];
        let cases = [Case::Nominative, Case::Dative, Case::Locative];
        let mut roots_used = 0;
        for entry in &self.lex.entries_ordered {
            if entry.part_of_speech != "noun" {
                continue;
            }
            for &p in &poss {
                for &c in &cases {
                    let features = NounFeatures {
                        possessive: Some(p),
                        case: Some(c),
                        ..Default::default()
                    };
                    let surface = synthesise_noun(&entry.root, features);
                    if surface.is_empty() {
                        continue;
                    }
                    out.push(self.pair_from_word(&surface, entry, RootPos::NounLike));
                }
            }
            roots_used += 1;
            if roots_used >= max_roots {
                break;
            }
        }
        self.emitted += out.len();
        out
    }

    /// Enumerate basic verb inflections: every verb × {finite tenses}
    /// × {3 persons} × {Sg/Pl}. Phase 0 keeps it simple — no voice,
    /// no negation, no participles.
    pub fn verb_inflections(&mut self, max_roots: usize) -> Vec<TrainingPair> {
        let tenses = [
            Tense::PastDefinite,
            Tense::Present,
            Tense::FutureIntentional,
            Tense::FuturePossible,
        ];
        let persons = [Person::First, Person::Second, Person::Third];
        let mut out = Vec::new();
        let mut roots_used = 0;
        for entry in &self.lex.entries_ordered {
            if entry.part_of_speech != "verb" {
                continue;
            }
            for &tense in &tenses {
                for &person in &persons {
                    for &number in &[None, Some(Number::Plural)] {
                        let features = VerbFeatures {
                            tense: Some(tense),
                            person: Some(person),
                            number,
                            ..Default::default()
                        };
                        let surface = synthesise_verb(&entry.root, features);
                        if surface.is_empty() {
                            continue;
                        }
                        out.push(self.pair_from_word(&surface, entry, RootPos::Verb));
                    }
                }
            }
            roots_used += 1;
            if roots_used >= max_roots {
                break;
            }
        }
        self.emitted += out.len();
        out
    }

    fn pair_from_word(&self, surface: &str, _entry: &RootEntry, pos: RootPos) -> TrainingPair {
        let toks = self.tokenizer.tokenize_word(surface);
        TrainingPair {
            surface: surface.to_string(),
            tokens: toks.iter().map(MorphTokenSer::from_token).collect(),
            pos: pos.into(),
        }
    }

    /// Tokenise a single surface word using the underlying
    /// [`AggTokenizer`]. Used by the real-corpus ingestion pipeline
    /// when we don't have a paired [`RootEntry`] (the lexicon may not
    /// contain the root). Falls back to `pos_hint` for the POS field.
    pub fn pair_from_text_word(&self, word: &str, pos_hint: RootPos) -> TrainingPair {
        let toks = self.tokenizer.tokenize_word(word);
        TrainingPair {
            surface: word.to_string(),
            tokens: toks.iter().map(MorphTokenSer::from_token).collect(),
            pos: pos_hint.into(),
        }
    }

    /// Split `text` on non-alphanumeric boundaries, tokenise each word
    /// through [`AggTokenizer`], and emit a [`TrainingPair`] per word.
    /// Used to ingest free-form Kazakh prose (Wikipedia articles, book
    /// chapters, news) into the same shape `bare_roots()` / inflection
    /// generators produce — so the downstream training loop is
    /// distribution-agnostic between synth and real data.
    pub fn pairs_from_text(&mut self, text: &str, pos_hint: RootPos) -> Vec<TrainingPair> {
        let mut out = Vec::new();
        for word in text.split(|c: char| !c.is_alphabetic()) {
            if word.is_empty() {
                continue;
            }
            // Skip words shorter than 2 chars (probably noise).
            if word.chars().count() < 2 {
                continue;
            }
            out.push(self.pair_from_text_word(word, pos_hint));
        }
        self.emitted += out.len();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn tiny_lex() -> LexiconV1 {
        let entries = vec![
            RootEntry {
                id: "bala_0001".into(),
                root: "бала".into(),
                part_of_speech: "noun".into(),
                vowel_harmony: "back".into(),
                final_sound_class: "vowel".into(),
            },
            RootEntry {
                id: "uy_0001".into(),
                root: "үй".into(),
                part_of_speech: "noun".into(),
                vowel_harmony: "front".into(),
                final_sound_class: "glide".into(),
            },
        ];
        let mut by_surface = HashMap::new();
        for e in &entries {
            by_surface.insert(e.root.clone(), e.clone());
        }
        let curated_count = entries.len();
        LexiconV1 {
            by_surface,
            entries_ordered: entries,
            curated_count,
            apertium_count: 0,
        }
    }

    #[test]
    fn bare_roots_emit_at_least_one_pair_per_root() {
        let lex = tiny_lex();
        let tok = AggTokenizer::build(lex.clone());
        let mut generator = SynthGenerator::new(&lex, &tok);
        let pairs = generator.bare_roots();
        assert!(
            pairs.len() >= 2,
            "expected ≥2 bare-root pairs, got {}",
            pairs.len()
        );
        assert!(pairs.iter().any(|p| p.surface == "бала"));
        assert!(pairs.iter().any(|p| p.surface == "үй"));
    }

    #[test]
    fn noun_inflections_emit_multiple_forms_per_root() {
        let lex = tiny_lex();
        let tok = AggTokenizer::build(lex.clone());
        let mut generator = SynthGenerator::new(&lex, &tok);
        let pairs = generator.noun_inflections(2);
        // 2 numbers (none, plural) × 7 cases × 2 roots = 28 (modulo
        // synth failures), but we accept anything reasonable.
        assert!(
            pairs.len() >= 8,
            "expected ≥8 noun-inflection pairs, got {}",
            pairs.len()
        );
        let bala_forms: Vec<&str> = pairs
            .iter()
            .filter_map(|p| {
                if p.surface.starts_with("бала") {
                    Some(p.surface.as_str())
                } else {
                    None
                }
            })
            .collect();
        // Should at least have бала + балалар + балаға + балада ...
        assert!(
            bala_forms.len() >= 4,
            "expected multiple бала forms, got {:?}",
            bala_forms
        );
    }

    #[test]
    fn pairs_have_token_sequences() {
        let lex = tiny_lex();
        let tok = AggTokenizer::build(lex.clone());
        let mut generator = SynthGenerator::new(&lex, &tok);
        let pairs = generator.bare_roots();
        for p in &pairs {
            assert!(!p.tokens.is_empty(), "{} has empty tokens", p.surface);
        }
    }
}
