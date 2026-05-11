//! adam-kernel-fst — deterministic finite-state transducer for Kazakh morphology.
//!
//! The core of the adam architecture. Replaces probability-driven token
//! generation with a two-level FST that encodes Kazakh morphology
//! deterministically. Every surface form the system emits through the
//! slot path is produced by this crate; there is no generative
//! alternative.
//!
//! ## Module layout
//!
//! - [`phonology`] — phonological rules as pure Rust functions.
//!   Resolves abstract underlying forms (archiphonemes `{Y}`, `{I}`,
//!   `{S}`, `{N}`, `{G}`, `{M}`, `{L}`, `{D}`, `{K}`, `{A}`, `{E}`)
//!   to surface letters. 22+ of Apertium's 54 catalogued rules are
//!   implemented; glide-vowels `у`, `и`, `ю` are `HighSonorant`
//!   (v2.3 correction).
//! - [`morphotactics`] — state machines for noun and verb inflection
//!   chains. Given `(root, feature_bundle)` emits a sequence of
//!   archiphoneme atoms that `phonology` then realises. 36 suffix
//!   templates covering 7 inflectional cases + 1 derivational
//!   locative-attributive (v4.5.0) × 2 numbers × 7 possessives × 11
//!   derivations × 7 predicate-person copulas.
//! - [`parser`] — inverse direction. Given a surface form, enumerates
//!   every `(root, features)` analysis whose synthesis matches.
//!   Generate-and-test over every Lexicon root; O(lex × features ×
//!   word_len) per parse, ~1.2 ms / word on M2.
//! - [`lexicon`] — loader for the curated + Apertium Lexicon. Current
//!   count: **~25.5 k roots** (13.6 k pure Kazakh + 11.9 k Apertium
//!   imports); v2.2 purged 87 intervocalic-voicing duplicates that
//!   were Apertium-import artefacts.

pub mod lexicon;
pub mod morphotactics;
pub mod parser;
pub mod phonology;
pub mod pronoun_paradigm;
pub mod root_affinity;
pub mod sem_frame;
pub mod suffix_priors;

pub use phonology::{
    Archiphoneme, ConsonantClass, PhonologicalContext, VowelClass, realise_archiphoneme,
};
pub use sem_frame::{
    EvidenceKind, Modality, Polarity, PosTag, RelationKind, SemFrame, populate_ability_modality,
    populate_periphrastic_modality, populate_sentential_negation,
};
pub use suffix_priors::{
    SCHEMA_VERSION as SUFFIX_PRIORS_SCHEMA_VERSION, SuffixPriors, SuffixPriorsLoadError,
    noun_chain_key, verb_chain_key,
};
