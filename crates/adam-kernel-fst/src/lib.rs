//! adam-kernel-fst — deterministic finite-state transducer for Kazakh morphology.
//!
//! Stage: v1.0.0 scaffold (week 1 day 1 — skeleton only).
//!
//! This crate is the new core of the adam v1.0.0 line. It replaces the
//! probability-driven transformer with a two-level finite-state transducer
//! that encodes Kazakh morphology deterministically. See
//! `docs/kazakh_grammar/00_architecture_v1.md` for the rationale.
//!
//! ## Module layout
//!
//! - [`phonology`] — 54 phonological rules as pure Rust functions. Resolves
//!   abstract underlying forms (archiphonemes) to surface letters.
//! - [`morphotactics`] — state machines for noun and verb inflection chains.
//!   Given `(root, feature_bundle)` emits a sequence of archiphoneme strings
//!   that `phonology` then realises.
//! - [`lexicon`] — loader for the v1 lexicon (211 curated + 11,919 Apertium
//!   imports at current count).
//!
//! Week 1 deliverable is **types + red tests**. No implementation yet.

pub mod lexicon;
pub mod morphotactics;
pub mod parser;
pub mod phonology;

pub use phonology::{
    Archiphoneme, ConsonantClass, PhonologicalContext, VowelClass, realise_archiphoneme,
};
