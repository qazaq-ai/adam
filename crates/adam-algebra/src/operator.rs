// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `SuffixOp` — uniform algebraic surface over every FST operator.
//!
//! Each variant carries the FST type directly (no re-encoding), so
//! round-trip with `adam_kernel_fst::parser::Analysis` is by
//! construction byte-stable.

use adam_kernel_fst::morphotactics::{
    Case, Derivation, Number, Person, Possessive, Predicate, Tense, Voice,
};
use serde::{Deserialize, Serialize};

/// The slot a [`SuffixOp`] occupies in the canonical Kazakh
/// composition order.
///
/// Noun composition (per `morphotactics.rs::NounFeatures`):
///
/// ```text
/// ROOT + Derivation + Number + Possessive + Case + Predicate
/// ```
///
/// Verb composition (per `morphotactics.rs::VerbFeatures`):
///
/// ```text
/// ROOT + Voice + Negation + Tense + (Person × Number)
/// ```
///
/// The slot enum lets downstream code reason about composition
/// order, idempotence («at most one operator per slot»), and slot-
/// kind compatibility without inspecting individual operator values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotKind {
    // -- Noun composition slots ---------------------------------
    /// Derivational morpheme applied BEFORE inflection (occupations
    /// «-шы / -ші», collective numerals «-еу / -ау», etc.).
    Derivation,
    /// Number (Singular / Plural).
    Number,
    /// Possessive (P1Sg / P1Pl / P2 / P3, polite vs. informal).
    Possessive,
    /// Grammatical case (Nominative / Genitive / Dative / Accusative
    /// / Locative / Ablative / Instrumental / LocativeAttributive).
    Case,
    /// Predicate copula attached to a noun («мұғаліммін» = "I am a
    /// teacher"). Mutually exclusive with Possessive in practice.
    Predicate,
    // -- Verb composition slots ---------------------------------
    /// Voice (Active / Passive / Reflexive / Reciprocal / Causative).
    Voice,
    /// Negation marker «-ма / -ме» on verbs.
    Negation,
    /// Tense / mood / aspect (PastDefinite / Present / Conditional
    /// / Imperative / participle forms).
    Tense,
    /// Verb-side person agreement (the «-мын / -сың / -сыз» chain
    /// on a conjugated verb).
    VerbPerson,
    /// Verb-side number (singular vs plural agreement).
    VerbNumber,
    /// Politeness allomorph switch on a 2sg / 2pl conjugated verb
    /// («келдің» informal vs «келдіңіз» polite). Modifies the
    /// VerbPerson surface form but logically a separate slot in
    /// the algebra so it can co-exist in a well-formed composition.
    VerbPolite,
}

impl SlotKind {
    /// Stable slug for JSON / trace output.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Derivation => "derivation",
            Self::Number => "number",
            Self::Possessive => "possessive",
            Self::Case => "case",
            Self::Predicate => "predicate",
            Self::Voice => "voice",
            Self::Negation => "negation",
            Self::Tense => "tense",
            Self::VerbPerson => "verb_person",
            Self::VerbNumber => "verb_number",
            Self::VerbPolite => "verb_polite",
        }
    }

    /// True iff this slot is part of the noun composition stack.
    pub fn is_noun_slot(self) -> bool {
        matches!(
            self,
            Self::Derivation | Self::Number | Self::Possessive | Self::Case | Self::Predicate
        )
    }

    /// True iff this slot is part of the verb composition stack.
    pub fn is_verb_slot(self) -> bool {
        matches!(
            self,
            Self::Voice
                | Self::Negation
                | Self::Tense
                | Self::VerbPerson
                | Self::VerbNumber
                | Self::VerbPolite
        )
    }

    /// Canonical position in the noun composition order. `None` for
    /// non-noun slots. Used by `Composition::is_well_formed` to
    /// verify slots appear in canonical order (an artistic constraint
    /// — the FST itself doesn't care about Rust-side ordering).
    pub fn noun_order(self) -> Option<u8> {
        Some(match self {
            Self::Derivation => 0,
            Self::Number => 1,
            Self::Possessive => 2,
            Self::Case => 3,
            Self::Predicate => 4,
            _ => return None,
        })
    }

    /// Canonical position in the verb composition order.
    pub fn verb_order(self) -> Option<u8> {
        Some(match self {
            Self::Voice => 0,
            Self::Negation => 1,
            Self::Tense => 2,
            Self::VerbPerson => 3,
            Self::VerbNumber => 4,
            Self::VerbPolite => 5,
            _ => return None,
        })
    }
}

/// Uniform algebraic operator type — one variant per FST-recognised
/// suffix family. Each variant carries the FST's typed value
/// directly; round-trip with `Analysis` is byte-stable by
/// construction.
///
/// The variant name encodes the slot kind for diagnostic clarity
/// («Plural» reads at glance; «Number(Plural)» needs unpacking).
/// Slot membership is recovered via [`SuffixOp::slot_kind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "op", content = "value")]
pub enum SuffixOp {
    // -- Noun-stack operators -----------------------------------
    Derivation(Derivation),
    Number(Number),
    Possessive(Possessive),
    Case(Case),
    Predicate(Predicate),
    // -- Verb-stack operators -----------------------------------
    Voice(Voice),
    /// Verb negation is a flag («-ма / -ме» attached pre-tense).
    /// The FST models it as `VerbFeatures::negation: bool`; we lift
    /// it into a unit variant.
    Negation,
    Tense(Tense),
    VerbPerson(Person),
    VerbNumber(Number),
    /// Verb polite vs. informal ending switch (2sg / 2pl). The FST
    /// models it as `VerbFeatures::polite: bool`; lifting it into
    /// its own operator keeps the algebra unified.
    VerbPolite(bool),
}

impl SuffixOp {
    /// Which slot does this operator occupy?
    pub fn slot_kind(self) -> SlotKind {
        match self {
            Self::Derivation(_) => SlotKind::Derivation,
            Self::Number(_) => SlotKind::Number,
            Self::Possessive(_) => SlotKind::Possessive,
            Self::Case(_) => SlotKind::Case,
            Self::Predicate(_) => SlotKind::Predicate,
            Self::Voice(_) => SlotKind::Voice,
            Self::Negation => SlotKind::Negation,
            Self::Tense(_) => SlotKind::Tense,
            Self::VerbPerson(_) => SlotKind::VerbPerson,
            Self::VerbNumber(_) => SlotKind::VerbNumber,
            Self::VerbPolite(_) => SlotKind::VerbPolite,
        }
    }

    /// Is this operator **idempotent** in the algebraic sense —
    /// applying it twice has the same effect as applying it once?
    ///
    /// In the agglutinative algebra, slot uniqueness (at most one
    /// operator per slot per composition) makes literal double-
    /// application unrepresentable; this method documents the
    /// semantic statement for the OperatorLayer's reasoning needs.
    ///
    /// - `Plural ∘ Plural = Plural` — applying plural twice doesn't
    ///   produce «plural-plural»; Kazakh has no recursive number.
    /// - `Negation ∘ Negation` is **not** idempotent — it cancels
    ///   (see [`SuffixOp::inverse`]).
    /// - `Possessive` is not idempotent — re-applying a different
    ///   person changes meaning.
    ///
    /// **Default: false** unless the operator is structurally a
    /// flag (number, polite-switch) where re-application is a no-op.
    pub fn is_idempotent(self) -> bool {
        matches!(
            self,
            Self::Number(_) | Self::VerbNumber(_) | Self::VerbPolite(_)
        )
    }

    /// Does this operator have an **algebraic inverse** — an
    /// operator that, applied after it, yields the identity (modulo
    /// surface variation)?
    ///
    /// - `Negation` is its own inverse (`Negation ∘ Negation` cancels
    ///   to affirmative; the discourse-prosody connotation is
    ///   ignored at this algebraic layer).
    /// - Case / Number / Tense have NO inverse — there's no «un-
    ///   dative» suffix.
    ///
    /// Returns `Some(self)` when the operator is its own inverse,
    /// `None` otherwise.
    pub fn inverse(self) -> Option<Self> {
        match self {
            Self::Negation => Some(Self::Negation),
            _ => None,
        }
    }

    /// True iff applying this operator to a [`crate::Root`] requires
    /// the root's POS to use the **noun** composition stack.
    pub fn requires_noun_root(self) -> bool {
        self.slot_kind().is_noun_slot()
    }

    /// True iff applying this operator to a [`crate::Root`] requires
    /// the root's POS to use the **verb** composition stack.
    pub fn requires_verb_root(self) -> bool {
        self.slot_kind().is_verb_slot()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_kinds_partition_into_noun_or_verb() {
        for slot in [
            SlotKind::Derivation,
            SlotKind::Number,
            SlotKind::Possessive,
            SlotKind::Case,
            SlotKind::Predicate,
            SlotKind::Voice,
            SlotKind::Negation,
            SlotKind::Tense,
            SlotKind::VerbPerson,
            SlotKind::VerbNumber,
            SlotKind::VerbPolite,
        ] {
            assert!(
                slot.is_noun_slot() ^ slot.is_verb_slot(),
                "{slot:?} must be exactly one of noun/verb slot"
            );
        }
    }

    #[test]
    fn noun_order_covers_all_noun_slots() {
        for slot in [
            SlotKind::Derivation,
            SlotKind::Number,
            SlotKind::Possessive,
            SlotKind::Case,
            SlotKind::Predicate,
        ] {
            assert!(slot.noun_order().is_some());
            assert!(slot.verb_order().is_none());
        }
    }

    #[test]
    fn verb_order_covers_all_verb_slots() {
        for slot in [
            SlotKind::Voice,
            SlotKind::Negation,
            SlotKind::Tense,
            SlotKind::VerbPerson,
            SlotKind::VerbNumber,
            SlotKind::VerbPolite,
        ] {
            assert!(slot.verb_order().is_some());
            assert!(slot.noun_order().is_none());
        }
    }

    #[test]
    fn number_is_idempotent() {
        // Plural ∘ Plural = Plural (no recursive plural in Kazakh).
        assert!(SuffixOp::Number(Number::Plural).is_idempotent());
        assert!(SuffixOp::Number(Number::Singular).is_idempotent());
    }

    #[test]
    fn case_is_not_idempotent() {
        assert!(!SuffixOp::Case(Case::Dative).is_idempotent());
        assert!(!SuffixOp::Case(Case::Locative).is_idempotent());
    }

    #[test]
    fn negation_is_its_own_inverse() {
        assert_eq!(SuffixOp::Negation.inverse(), Some(SuffixOp::Negation));
    }

    #[test]
    fn case_has_no_inverse() {
        assert_eq!(SuffixOp::Case(Case::Dative).inverse(), None);
    }

    #[test]
    fn verb_polite_is_separate_slot_from_person() {
        // Politeness has its own slot so (VerbPerson, VerbPolite)
        // can co-exist in a well-formed composition («келдіңіз» =
        // Verb + Tense + Person(2) + Polite(true)).
        assert_eq!(SuffixOp::VerbPolite(true).slot_kind(), SlotKind::VerbPolite);
        assert_eq!(
            SuffixOp::VerbPerson(Person::Second).slot_kind(),
            SlotKind::VerbPerson
        );
    }
}
