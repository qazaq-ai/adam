// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `adam-phonotactics` — **Layer 0c of the v6.3 phonemic-foundation
//! arc**.
//!
//! Phonotactic rules over phoneme streams from [`adam_phoneme`]:
//!
//! - **Vowel harmony** ([`harmony`]) — within a native morpheme
//!   all vowels share a harmony class (Front or Back). The
//!   foundational determinism of Kazakh agglutination.
//! - **Syllabification** ([`syllable`]) — chunks a phoneme stream
//!   into `(C)V(C)(C)` syllables using the maximum-onset
//!   principle adapted to Kazakh.
//! - **Native-word validation** ([`validate`]) — combines the
//!   above into a single `validate_native_word(&[Phoneme])`
//!   predicate that runs every check.
//!
//! ## Why this crate exists
//!
//! Per [`docs/v6_3_phonemic_foundation.md`](../../../docs/v6_3_phonemic_foundation.md)
//! §3.4: the phonotactic FST is the **constraint surface** for
//! any constrained-decoding LM that lives above Layer 0. A neural
//! component generating phonemes one at a time will mask its
//! logits against this layer; transitions that violate harmony
//! or produce illegal clusters get `-∞`, so morphological
//! ill-formedness becomes impossible by construction.
//!
//! For now this layer is **just the validator** — the constrained
//! decoding consumer comes in Phase 6b.
//!
//! ## What this layer does NOT do (yet)
//!
//! - Voicing assimilation across morpheme boundaries (the 8-way
//!   matrix from `docs/kazakh_grammar/01_phonology.md` §3). That
//!   lives on the morpheme-suffix selector, which is a later
//!   crate.
//! - Cluster permission lists (which CC / CCC are allowed). For
//!   the first pass clusters are accepted permissively; only
//!   harmony and basic structural rules are enforced. Refined
//!   against corpus evidence in Phase 4.
//! - Loan-word exceptions to harmony. The validator's
//!   `is_native_root` flag suppresses the harmony check; the
//!   caller decides per word.

#![forbid(unsafe_code)]

pub mod harmony;
pub mod syllable;
pub mod validate;

pub use harmony::{HarmonyResult, check_harmony};
pub use syllable::{Syllable, syllabify};
pub use validate::{ValidationError, validate_native_word};
