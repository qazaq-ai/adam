// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `adam-kernel-phoneme` — **Phase 9 of the v6.3 phonemic-
//! foundation arc**: phoneme-native morphological kernel.
//!
//! The thesis test: does Kazakh morphology become **simpler**
//! when the underlying representation is phonemes rather than
//! graphemes?
//!
//! ## Old (graphemic) `adam-kernel-fst`
//!
//! - Roots stored as Cyrillic `String`.
//! - Suffix templates use abstract **archiphonemes** ({Y}, {I},
//!   {S}, {N}, {G}, {M}, {L}, {D}, {K}, {A}, {E}) — placeholder
//!   symbols.
//! - Phonological rules realise each archiphoneme to a Cyrillic
//!   letter based on context (~22 rules, 650 lines of code).
//! - Voicing assimilation, vowel harmony, etc. expressed as
//!   graphemic rewrites.
//!
//! ## New (phonemic) `adam-kernel-phoneme`
//!
//! - Roots stored as `Vec<Phoneme>`. Native Kazakh phonology
//!   is the storage form.
//! - Suffix templates are concrete **phoneme sequences**, one
//!   per (harmony, voicing) variant. **No archiphonemes** —
//!   every realisation is a real phoneme.
//! - Variant selection is a typed lookup: `HarmonyClass`
//!   (Front/Back) + `CodaClass` (Vowel / Voiceless /
//!   VoicedSonorant) from [`adam_phoneme::Phoneme`] attributes.
//! - No phonological rules — the assimilations are baked into
//!   the type system.
//!
//! This is the v6.3 thesis demonstrated: **graphemes-first
//! created the rule machinery; phonemes-first removes it**.
//!
//! ## Phase 9 scope
//!
//! Proof-of-concept lexicon + 4 common suffixes (locative,
//! dative, genitive, plural). Real-data battery on
//! the inflection of 12+ Kazakh roots. Demonstrates the
//! approach; production-scale port from the 25 k-root
//! graphemic lexicon is Phase 9-plus work.

#![forbid(unsafe_code)]

pub mod inflect;
pub mod lexicon;
pub mod suffix;

pub use inflect::{InflectError, InflectedForm, inflect};
pub use lexicon::{PartOfSpeech, PhonemeRoot, sample_lexicon};
pub use suffix::{CodaClass, PhonemeSuffix, SUFFIXES, SuffixId, SuffixVariant, classify_coda};
