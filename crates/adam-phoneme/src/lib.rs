// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `adam-phoneme` — **Layer 0a of the v6.3 phonemic-foundation arc**.
//!
//! The typed phoneme alphabet of Kazakh, defined in
//! [`docs/v6_3_phonemic_foundation.md`](../../../docs/v6_3_phonemic_foundation.md)
//! §3.2: 10 vowels (8 full + 2 epenthetic) plus 27 consonants
//! (21 native + 5 loan-only + 1 boundary marker) = **37 units**.
//!
//! Each [`Phoneme`] variant carries every articulatory and
//! phonological attribute the rest of the v6.3 stack needs:
//! place / manner / voicing for consonants, harmony class /
//! height / rounding / length for vowels, the
//! native-vs-loan flag, and — critically — the
//! [`Phoneme::is_epenthetic`] predicate that flags the two
//! Cyrillic letters «ы» / «і» whose acoustic realisation in
//! most positions is null or minimal.
//!
//! ## Why this crate exists (v6.3 thesis)
//!
//! Per [`docs/v6_3_phonemic_foundation.md`](../../../docs/v6_3_phonemic_foundation.md)
//! §2: the Cyrillic / Latin orthographies of Kazakh **promote
//! phonetic transitions into full graphemes**, so any layer
//! keyed on `&str` graphemes inherits that orthographic noise.
//! Phonemes are the math-correct canonical form. Graphemes
//! become bidirectional renderers at the periphery.
//!
//! This crate is the foundation: every higher layer (lexicon,
//! morpheme FST, frame index, dialog router) will eventually
//! key on `&[Phoneme]` instead of `&str`.
//!
//! ## Surface
//!
//! - [`Phoneme`] — the 37-variant enum.
//! - [`PhonemeClass`] — Vowel | Consonant | Boundary.
//! - [`Place`], [`Manner`], [`Voicing`] — consonant attributes.
//! - [`HarmonyClass`], [`Height`], [`Rounding`], [`Length`] —
//!   vowel attributes.
//! - [`cyrillic`] module — `phonemes_to_cyrillic` and
//!   `cyrillic_to_phonemes` (bidirectional renderer with the
//!   «ы»/«і» epenthetic rule).

#![forbid(unsafe_code)]

pub mod cyrillic;
mod phoneme;

pub use phoneme::{
    HarmonyClass, Height, Length, Manner, Phoneme, PhonemeClass, Place, Rounding, Voicing,
};
