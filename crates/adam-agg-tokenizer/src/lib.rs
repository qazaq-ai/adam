// SPDX-License-Identifier: MIT
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `adam-agg-tokenizer` — agglutinative morpheme-level tokenizer for Kazakh.
//!
//! **Phase 0 prototype** of the experimental/agglutinative-neural branch.
//! Wraps the production [`adam_kernel_fst`] FST analyser/synthesiser into a
//! tokenizer producing a sequence of typed [`MorphToken`]s instead of byte
//! pair encodings.
//!
//! **Vocabulary** (~25.6k tokens):
//!
//! - ~25 500 [`MorphToken::Root`] tokens — one per Lexicon entry.
//! - ~64 [`MorphToken::Suffix`] tokens — one per typed [`SuffixKind`]
//!   variant. NOT per surface-allomorph: a single `Suffix(Number(Plural))`
//!   token covers `-лар/-лер/-дар/-дер/-тар/-тер/-нар/-нер`; the FST
//!   chooses the correct surface form at synthesis time.
//! - 5 service tokens ([`MorphToken::Bos`], [`MorphToken::Eos`],
//!   [`MorphToken::Pad`], [`MorphToken::Space`], [`MorphToken::Unk`]).
//! - 20 punctuation tokens ([`MorphToken::Punct`]).
//!
//! **Round-trip property**: for every word `w` in the Lexicon's coverage,
//! `detokenize(tokenize(w)) == w`. See [`AggTokenizer::round_trip`].
//!
//! **What's NOT in scope for Phase 0:**
//!
//! - Multi-word phrase rendering beyond basic whitespace + BOS/EOS.
//! - OOV stemming / byte-level fallback (a single [`MorphToken::Unk`]
//!   captures the whole surface form).
//! - Vocab serialization (rebuilt in-memory from `LexiconV1` each load).
//! - Embedding tables (those belong to the training pipeline, Phase 2).
//!
//! # Relation to the legacy [`adam_tokenizer`] crate
//!
//! `adam-tokenizer` is a separate, much larger crate from the v0.1-v0.4
//! BPE/transformer era. It produces a byte/character-level token stream
//! with SentencePiece-style `▁` markers, plus experiment-tracking
//! infrastructure (TokenizerExperiment, dry-run reports). It is used
//! by [`adam_train`] binaries and exposes a `normalize_text` helper to
//! [`adam_dialog`].
//!
//! `adam-agg-tokenizer` does **not** overlap with that crate
//! architecturally — different paradigm (typed-morpheme enum vs
//! byte-level BPE), different consumer (the experimental neural
//! composition layer in Phase 2 vs the legacy transformer training).
//! See `docs/research/tokenizer_spec.md` for the design boundary.
//!
//! Future enhancements may borrow [`adam_kernel::deterministic_segment_token`]
//! as a character-level fallback for OOV words (Phase 1).

pub mod error;
pub mod vocab;

use adam_kernel_fst::lexicon::{LexiconV1, RootEntry};
use adam_kernel_fst::morphotactics::{
    Case, Derivation, NounFeatures, Number, Person, Possessive, Predicate, Tense, VerbFeatures,
    Voice, synthesise_noun, synthesise_verb,
};
use adam_kernel_fst::parser::{Analysis, analyse};

pub use error::{DetokError, RoundTripError};
pub use vocab::{Vocab, VocabBuildError};

// -----------------------------------------------------------------------------
// Token type
// -----------------------------------------------------------------------------

/// A single morpheme-level token.
///
/// Distinct from BPE tokens in that every variant carries explicit
/// morphological semantics. Two tokens that differ only in surface
/// allomorph (e.g. `-лар` vs `-лер`) share the **same** `MorphToken`
/// variant — the FST chooses the surface form at synthesis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MorphToken {
    /// Root morpheme. `id` is its position in
    /// [`LexiconV1::entries_ordered`].
    Root { id: u32, root: String, pos: RootPos },
    /// Typed suffix token.
    Suffix(SuffixKind),
    /// Beginning-of-sentence marker.
    Bos,
    /// End-of-sentence marker.
    Eos,
    /// Padding marker.
    Pad,
    /// Whitespace between words.
    Space,
    /// Out-of-vocabulary surface fragment. Preserves the original text
    /// verbatim so detokenize can pass it through unchanged.
    Unk { surface: String },
    /// Punctuation character.
    Punct(char),
}

/// Coarse POS tag carried by [`MorphToken::Root`]. Mirrors which
/// inflectional paradigm to apply at detokenize time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootPos {
    /// Nouns, adjectives, pronouns, numerals — share the noun-feature
    /// paradigm in our FST.
    NounLike,
    /// Verbs — verb-feature paradigm.
    Verb,
    /// Particle / closed-class word that takes no inflection. Detokenize
    /// emits the bare root.
    Particle,
}

/// Typed suffix tokens. Each variant corresponds to a single morphotactic
/// feature in the FST. Surface allomorph is determined by the FST at
/// synthesis time — the token itself is allomorph-agnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuffixKind {
    // Noun side.
    Derivation(Derivation),
    Number(Number),
    Possessive(Possessive),
    Case(Case),
    Predicate(Predicate),
    // Verb side.
    Voice(Voice),
    Negation,
    Tense(Tense),
    /// `(Person, polite)` — polite flag matters for 2nd person.
    Person(Person, bool),
}

// -----------------------------------------------------------------------------
// Tokenizer
// -----------------------------------------------------------------------------

/// Morpheme-level tokenizer for Kazakh.
///
/// Owns a [`LexiconV1`] and a derived [`Vocab`]. Construction loads the
/// lexicon (~25.5k roots) and builds the suffix-token mapping; the
/// tokenizer itself is stateless across calls.
#[derive(Debug)]
pub struct AggTokenizer {
    vocab: Vocab,
    lex: LexiconV1,
}

impl AggTokenizer {
    /// Build a tokenizer from a pre-loaded lexicon.
    pub fn build(lex: LexiconV1) -> Self {
        let vocab = Vocab::from_lexicon(&lex);
        Self { vocab, lex }
    }

    /// Vocab introspection.
    pub fn vocab(&self) -> &Vocab {
        &self.vocab
    }

    /// Tokenize a single surface word into a morpheme sequence.
    ///
    /// Picks the first [`Analysis`] returned by the FST parser (the
    /// determinism-contract gives stable ordering — see
    /// `parser::analyse` doc-comment). OOV words → single
    /// [`MorphToken::Unk`].
    pub fn tokenize_word(&self, word: &str) -> Vec<MorphToken> {
        let analyses = analyse(word, &self.lex);
        let Some(first) = analyses.into_iter().next() else {
            return vec![MorphToken::Unk {
                surface: word.to_string(),
            }];
        };
        match first {
            Analysis::Noun { root, features } => self.noun_to_tokens(&root, features),
            Analysis::Verb { root, features } => self.verb_to_tokens(&root, features),
        }
    }

    /// Tokenize a full sentence. Whitespace splits into words; punctuation
    /// is currently absorbed into [`MorphToken::Punct`] for the common
    /// characters and otherwise into [`MorphToken::Unk`].
    pub fn tokenize_sentence(&self, sentence: &str) -> Vec<MorphToken> {
        let mut out = vec![MorphToken::Bos];
        let mut prev_was_word = false;
        for piece in self.split_sentence(sentence) {
            match piece {
                SentencePiece::Word(w) => {
                    if prev_was_word {
                        out.push(MorphToken::Space);
                    }
                    out.extend(self.tokenize_word(w));
                    prev_was_word = true;
                }
                SentencePiece::Punct(c) => {
                    out.push(MorphToken::Punct(c));
                    prev_was_word = false;
                }
            }
        }
        out.push(MorphToken::Eos);
        out
    }

    /// Detokenize a morpheme sequence back into a surface word.
    ///
    /// Expects a single-word slice: leading [`MorphToken::Root`] (or
    /// [`MorphToken::Unk`]) followed by zero or more typed suffixes.
    /// Service tokens (Bos/Eos/Space/Pad) are rejected — use
    /// [`detokenize_sentence`](Self::detokenize_sentence) for those.
    pub fn detokenize_word(&self, tokens: &[MorphToken]) -> Result<String, DetokError> {
        let (head, rest) = tokens.split_first().ok_or(DetokError::Empty)?;
        match head {
            MorphToken::Root { root, pos, .. } => match pos {
                RootPos::NounLike | RootPos::Particle => {
                    let features = build_noun_features(rest)?;
                    Ok(synthesise_noun(root, features))
                }
                RootPos::Verb => {
                    let features = build_verb_features(rest)?;
                    Ok(synthesise_verb(root, features))
                }
            },
            MorphToken::Unk { surface } => {
                if !rest.is_empty() {
                    return Err(DetokError::UnkWithSuffix);
                }
                Ok(surface.clone())
            }
            _ => Err(DetokError::NotARoot),
        }
    }

    /// Detokenize a full sentence (word boundary = [`MorphToken::Space`]).
    pub fn detokenize_sentence(&self, tokens: &[MorphToken]) -> Result<String, DetokError> {
        let mut out = String::new();
        let mut buf: Vec<MorphToken> = Vec::new();
        for tok in tokens {
            match tok {
                MorphToken::Bos | MorphToken::Eos | MorphToken::Pad => {}
                MorphToken::Space => {
                    if !buf.is_empty() {
                        out.push_str(&self.detokenize_word(&buf)?);
                        buf.clear();
                    }
                    out.push(' ');
                }
                MorphToken::Punct(c) => {
                    if !buf.is_empty() {
                        out.push_str(&self.detokenize_word(&buf)?);
                        buf.clear();
                    }
                    out.push(*c);
                }
                _ => buf.push(tok.clone()),
            }
        }
        if !buf.is_empty() {
            out.push_str(&self.detokenize_word(&buf)?);
        }
        Ok(out)
    }

    /// Round-trip: `tokenize_word` then `detokenize_word`, compare to
    /// original input. Returns `Ok(true)` if the surface form is
    /// preserved exactly. OOV words trivially round-trip (Unk preserves
    /// surface verbatim).
    pub fn round_trip(&self, word: &str) -> Result<bool, RoundTripError> {
        let tokens = self.tokenize_word(word);
        let reconstructed = self
            .detokenize_word(&tokens)
            .map_err(RoundTripError::Detok)?;
        Ok(reconstructed == word)
    }

    // ---- internals -----------------------------------------------------------

    fn noun_to_tokens(&self, root: &RootEntry, features: NounFeatures) -> Vec<MorphToken> {
        let mut tokens = Vec::with_capacity(6);
        tokens.push(self.root_token(root));
        if let Some(d) = features.derivation {
            tokens.push(MorphToken::Suffix(SuffixKind::Derivation(d)));
        }
        if features.number == Some(Number::Plural) {
            tokens.push(MorphToken::Suffix(SuffixKind::Number(Number::Plural)));
        }
        if let Some(p) = features.possessive {
            tokens.push(MorphToken::Suffix(SuffixKind::Possessive(p)));
        }
        if let Some(c) = features.case {
            tokens.push(MorphToken::Suffix(SuffixKind::Case(c)));
        }
        if let Some(p) = features.predicate {
            tokens.push(MorphToken::Suffix(SuffixKind::Predicate(p)));
        }
        tokens
    }

    fn verb_to_tokens(&self, root: &RootEntry, features: VerbFeatures) -> Vec<MorphToken> {
        let mut tokens = Vec::with_capacity(6);
        tokens.push(self.root_token(root));
        if let Some(v) = features.voice {
            tokens.push(MorphToken::Suffix(SuffixKind::Voice(v)));
        }
        if features.negation {
            tokens.push(MorphToken::Suffix(SuffixKind::Negation));
        }
        if let Some(t) = features.tense {
            tokens.push(MorphToken::Suffix(SuffixKind::Tense(t)));
        }
        if let Some(p) = features.person {
            tokens.push(MorphToken::Suffix(SuffixKind::Person(p, features.polite)));
        }
        tokens
    }

    fn root_token(&self, root: &RootEntry) -> MorphToken {
        let id = self.vocab.root_id(&root.id).unwrap_or(u32::MAX);
        let pos = classify_root_pos(&root.part_of_speech);
        MorphToken::Root {
            id,
            root: root.root.clone(),
            pos,
        }
    }

    fn split_sentence<'a>(&self, sentence: &'a str) -> Vec<SentencePiece<'a>> {
        let mut out = Vec::new();
        let mut word_start: Option<usize> = None;
        for (i, ch) in sentence.char_indices() {
            if ch.is_whitespace() {
                if let Some(start) = word_start.take() {
                    out.push(SentencePiece::Word(&sentence[start..i]));
                }
            } else if is_kz_punct(ch) {
                if let Some(start) = word_start.take() {
                    out.push(SentencePiece::Word(&sentence[start..i]));
                }
                out.push(SentencePiece::Punct(ch));
            } else if word_start.is_none() {
                word_start = Some(i);
            }
        }
        if let Some(start) = word_start {
            out.push(SentencePiece::Word(&sentence[start..]));
        }
        out
    }
}

enum SentencePiece<'a> {
    Word(&'a str),
    Punct(char),
}

fn classify_root_pos(pos_str: &str) -> RootPos {
    match pos_str {
        "verb" => RootPos::Verb,
        "noun" | "adjective" | "pronoun" | "numeral" => RootPos::NounLike,
        _ => RootPos::Particle,
    }
}

fn is_kz_punct(ch: char) -> bool {
    matches!(
        ch,
        '.' | ',' | '?' | '!' | ';' | ':' | '«' | '»' | '"' | '\'' | '(' | ')' | '—' | '–' | '-'
    )
}

// -----------------------------------------------------------------------------
// Detokenize helpers — turn typed suffixes back into FST feature bundles.
// -----------------------------------------------------------------------------

fn build_noun_features(suffix_tokens: &[MorphToken]) -> Result<NounFeatures, DetokError> {
    let mut features = NounFeatures::default();
    for tok in suffix_tokens {
        let MorphToken::Suffix(kind) = tok else {
            return Err(DetokError::NotASuffix);
        };
        match kind {
            SuffixKind::Derivation(d) => features.derivation = Some(*d),
            SuffixKind::Number(n) => features.number = Some(*n),
            SuffixKind::Possessive(p) => features.possessive = Some(*p),
            SuffixKind::Case(c) => features.case = Some(*c),
            SuffixKind::Predicate(p) => features.predicate = Some(*p),
            SuffixKind::Voice(_)
            | SuffixKind::Negation
            | SuffixKind::Tense(_)
            | SuffixKind::Person(_, _) => return Err(DetokError::VerbSuffixOnNoun),
        }
    }
    Ok(features)
}

fn build_verb_features(suffix_tokens: &[MorphToken]) -> Result<VerbFeatures, DetokError> {
    let mut features = VerbFeatures::default();
    for tok in suffix_tokens {
        let MorphToken::Suffix(kind) = tok else {
            return Err(DetokError::NotASuffix);
        };
        match kind {
            SuffixKind::Voice(v) => features.voice = Some(*v),
            SuffixKind::Negation => features.negation = true,
            SuffixKind::Tense(t) => features.tense = Some(*t),
            SuffixKind::Person(p, polite) => {
                features.person = Some(*p);
                features.polite = *polite;
            }
            SuffixKind::Derivation(_)
            | SuffixKind::Number(_)
            | SuffixKind::Possessive(_)
            | SuffixKind::Case(_)
            | SuffixKind::Predicate(_) => return Err(DetokError::NounSuffixOnVerb),
        }
    }
    Ok(features)
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Tiny in-memory lexicon for unit tests that don't need the full
    /// committed Lexicon files. Two roots: `бала` (noun) and `жаз` (verb).
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
                id: "zhaz_0001".into(),
                root: "жаз".into(),
                part_of_speech: "verb".into(),
                vowel_harmony: "back".into(),
                final_sound_class: "voiced".into(),
            },
        ];
        let mut by_surface = std::collections::HashMap::new();
        for e in &entries {
            by_surface.insert(e.root.clone(), e.clone());
        }
        let mut entries_ordered = entries.clone();
        entries_ordered.sort_by(|a, b| a.root.cmp(&b.root).then_with(|| a.id.cmp(&b.id)));
        let curated_count = entries.len();
        LexiconV1 {
            by_surface,
            entries_ordered,
            curated_count,
            apertium_count: 0,
        }
    }

    #[test]
    fn empty_word_tokenizes_to_unk() {
        let lex = tiny_lex();
        let tok = AggTokenizer::build(lex);
        let tokens = tok.tokenize_word("xxxxx");
        assert_eq!(tokens.len(), 1);
        assert!(matches!(&tokens[0], MorphToken::Unk { surface } if surface == "xxxxx"));
    }

    #[test]
    fn bare_root_round_trips() {
        let lex = tiny_lex();
        let tok = AggTokenizer::build(lex);
        let tokens = tok.tokenize_word("бала");
        assert!(matches!(&tokens[0], MorphToken::Root { root, .. } if root == "бала"));
        assert_eq!(tok.detokenize_word(&tokens).unwrap(), "бала");
    }

    #[test]
    fn unk_round_trips_verbatim() {
        let lex = tiny_lex();
        let tok = AggTokenizer::build(lex);
        let tokens = tok.tokenize_word("foreign");
        assert_eq!(tok.detokenize_word(&tokens).unwrap(), "foreign");
        assert!(tok.round_trip("foreign").unwrap());
    }

    #[test]
    fn tokenize_is_deterministic_across_calls() {
        let lex = tiny_lex();
        let tok = AggTokenizer::build(lex);
        let runs: Vec<Vec<MorphToken>> = (0..5).map(|_| tok.tokenize_word("бала")).collect();
        for w in runs.windows(2) {
            assert_eq!(w[0], w[1]);
        }
    }

    #[test]
    fn sentence_emits_bos_and_eos() {
        let lex = tiny_lex();
        let tok = AggTokenizer::build(lex);
        let tokens = tok.tokenize_sentence("бала");
        assert!(matches!(tokens.first(), Some(MorphToken::Bos)));
        assert!(matches!(tokens.last(), Some(MorphToken::Eos)));
    }

    #[test]
    fn punctuation_is_emitted() {
        let lex = tiny_lex();
        let tok = AggTokenizer::build(lex);
        let tokens = tok.tokenize_sentence("бала.");
        assert!(tokens.iter().any(|t| matches!(t, MorphToken::Punct('.'))));
    }

    #[test]
    fn empty_token_slice_rejected_on_detokenize() {
        let lex = tiny_lex();
        let tok = AggTokenizer::build(lex);
        let err = tok.detokenize_word(&[]).unwrap_err();
        assert!(matches!(err, DetokError::Empty));
    }

    #[test]
    fn verb_root_round_trips_bare() {
        let lex = tiny_lex();
        let tok = AggTokenizer::build(lex);
        let tokens = tok.tokenize_word("жаз");
        assert!(matches!(
            &tokens[0],
            MorphToken::Root {
                pos: RootPos::Verb,
                ..
            }
        ));
        assert_eq!(tok.detokenize_word(&tokens).unwrap(), "жаз");
    }
}
