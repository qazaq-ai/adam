// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `Composition` — root + ordered, slot-typed sequence of operators.
//!
//! A composition is the algebraic lift of
//! [`adam_kernel_fst::parser::Analysis`]: it carries the same
//! information (root + per-slot feature) but in a uniform
//! ordered-list shape that the v6.2 OperatorLayer / FrameLayer can
//! traverse algebraically.
//!
//! ## Round-trip invariant (load-bearing)
//!
//! ```text
//! Composition::from_analysis(&a) → c
//! c.to_analysis()                → a
//! ```
//!
//! Round-trip equivalence is the v6.2 Stage 1 success criterion. A
//! broken round-trip is a unit-test failure — see the
//! `round_trip_*` tests at the bottom of this module.

use adam_kernel_fst::lexicon::RootEntry;
use adam_kernel_fst::morphotactics::{NounFeatures, VerbFeatures};
use adam_kernel_fst::parser::Analysis;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::operator::{SlotKind, SuffixOp};
use crate::root::{PartOfSpeech, Root};

/// Errors a composition construction or validation can raise. None
/// of them are runtime failures — they all signal a contract
/// violation that should be caught at the test or callsite level.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CompositionError {
    /// The lexicon entry's `part_of_speech` is not recognised by
    /// [`PartOfSpeech::from_lexicon_str`]. Lexicon migration issue,
    /// not algebra issue.
    #[error("unknown POS in lexicon entry: {0}")]
    UnknownPos(String),
    /// A noun-slot operator was applied to a verb root, or vice
    /// versa.
    #[error("slot {slot:?} requires {expected:?} composition stack but root is {actual:?}")]
    SlotRootMismatch {
        slot: SlotKind,
        expected: &'static str,
        actual: PartOfSpeech,
    },
    /// Two operators occupied the same slot in the composition.
    /// The agglutinative algebra enforces slot uniqueness (the FST
    /// likewise: `NounFeatures::case` is `Option<Case>`, not
    /// `Vec<Case>`).
    #[error("slot {slot:?} is filled twice in the composition")]
    DuplicateSlot { slot: SlotKind },
    /// Operators appeared out of canonical order. Composition is
    /// ordered in Kazakh: ROOT + derivation + number + possessive
    /// + case + predicate (nouns) / voice + negation + tense +
    /// person + number (verbs).
    #[error(
        "operator in slot {late:?} appears before operator in slot {early:?} but canonical \
         Kazakh ordering requires {early:?} first"
    )]
    OutOfOrder { late: SlotKind, early: SlotKind },
}

/// Root + ordered operator sequence. Conceptually:
///
/// ```text
/// surface = Composition.apply_in_order(root)
/// ```
///
/// At Stage 1 the composition does NOT itself produce the surface
/// form — that's the FST's job (`synthesise_noun` / `synthesise_verb`).
/// What the composition guarantees is that the slot bookkeeping is
/// well-formed: every slot is filled at most once, operators
/// appear in canonical order, and the root POS matches the
/// composition stack the operators want.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Composition {
    pub root: Root,
    /// Operators in canonical Kazakh order (noun or verb stack
    /// depending on `root.pos`).
    pub operators: Vec<SuffixOp>,
}

impl Composition {
    /// The identity composition: root with no operators applied.
    /// Surface == root.surface.
    pub fn identity(root: Root) -> Self {
        Self {
            root,
            operators: Vec::new(),
        }
    }

    /// Lift a kernel-fst [`Analysis`] into a Composition. The
    /// operator list is produced in canonical Kazakh order.
    pub fn from_analysis(analysis: &Analysis) -> Result<Self, CompositionError> {
        match analysis {
            Analysis::Noun { root, features } => Self::from_noun(root, features),
            Analysis::Verb { root, features } => Self::from_verb(root, features),
        }
    }

    fn from_noun(root: &RootEntry, features: &NounFeatures) -> Result<Self, CompositionError> {
        let pos = PartOfSpeech::from_lexicon_str(&root.part_of_speech)
            .ok_or_else(|| CompositionError::UnknownPos(root.part_of_speech.clone()))?;
        let mut ops = Vec::new();
        if let Some(d) = features.derivation {
            ops.push(SuffixOp::Derivation(d));
        }
        if let Some(n) = features.number {
            ops.push(SuffixOp::Number(n));
        }
        if let Some(p) = features.possessive {
            ops.push(SuffixOp::Possessive(p));
        }
        if let Some(c) = features.case {
            ops.push(SuffixOp::Case(c));
        }
        if let Some(pr) = features.predicate {
            ops.push(SuffixOp::Predicate(pr));
        }
        Ok(Self {
            root: Root::new(&root.root, pos),
            operators: ops,
        })
    }

    fn from_verb(root: &RootEntry, features: &VerbFeatures) -> Result<Self, CompositionError> {
        let pos = PartOfSpeech::from_lexicon_str(&root.part_of_speech)
            .ok_or_else(|| CompositionError::UnknownPos(root.part_of_speech.clone()))?;
        let mut ops = Vec::new();
        if let Some(v) = features.voice {
            ops.push(SuffixOp::Voice(v));
        }
        if features.negation {
            ops.push(SuffixOp::Negation);
        }
        if let Some(t) = features.tense {
            ops.push(SuffixOp::Tense(t));
        }
        if let Some(p) = features.person {
            ops.push(SuffixOp::VerbPerson(p));
        }
        if let Some(n) = features.number {
            ops.push(SuffixOp::VerbNumber(n));
        }
        if features.polite {
            ops.push(SuffixOp::VerbPolite(true));
        }
        Ok(Self {
            root: Root::new(&root.root, pos),
            operators: ops,
        })
    }

    /// Project the composition back to a kernel-fst [`NounFeatures`]
    /// bundle. Round-trip equivalent of [`from_noun`].
    /// Returns `None` if `self.root.pos` is on the verb stack.
    pub fn to_noun_features(&self) -> Option<NounFeatures> {
        if !self.root.pos.uses_noun_composition() {
            return None;
        }
        let mut f = NounFeatures::default();
        for op in &self.operators {
            match op {
                SuffixOp::Derivation(d) => f.derivation = Some(*d),
                SuffixOp::Number(n) => f.number = Some(*n),
                SuffixOp::Possessive(p) => f.possessive = Some(*p),
                SuffixOp::Case(c) => f.case = Some(*c),
                SuffixOp::Predicate(pr) => f.predicate = Some(*pr),
                // Verb-slot ops on a noun composition are
                // structurally invalid — `is_well_formed` catches
                // this. Silently skip here to keep the projection
                // total; the test suite verifies that valid
                // compositions never trigger this branch.
                _ => {}
            }
        }
        Some(f)
    }

    /// Project the composition back to a kernel-fst [`VerbFeatures`]
    /// bundle. Returns `None` if `self.root.pos` is not Verb.
    pub fn to_verb_features(&self) -> Option<VerbFeatures> {
        if !self.root.pos.uses_verb_composition() {
            return None;
        }
        let mut f = VerbFeatures::default();
        for op in &self.operators {
            match op {
                SuffixOp::Voice(v) => f.voice = Some(*v),
                SuffixOp::Negation => f.negation = true,
                SuffixOp::Tense(t) => f.tense = Some(*t),
                SuffixOp::VerbPerson(p) => f.person = Some(*p),
                SuffixOp::VerbNumber(n) => f.number = Some(*n),
                SuffixOp::VerbPolite(p) => f.polite = *p,
                _ => {}
            }
        }
        Some(f)
    }

    /// Check that the composition obeys the algebra rules:
    ///   1. Every operator's slot matches the root's composition
    ///      stack (noun ops on noun root, verb ops on verb root).
    ///   2. No slot is filled twice.
    ///   3. Operators appear in canonical Kazakh order.
    ///
    /// Identity compositions (zero operators) are well-formed.
    pub fn is_well_formed(&self) -> Result<(), CompositionError> {
        let mut seen: Vec<SlotKind> = Vec::with_capacity(self.operators.len());
        let mut last_order: Option<u8> = None;
        let is_noun_root = self.root.pos.uses_noun_composition();
        for op in &self.operators {
            let slot = op.slot_kind();
            // Rule 1: slot ↔ root POS.
            if is_noun_root && !slot.is_noun_slot() {
                return Err(CompositionError::SlotRootMismatch {
                    slot,
                    expected: "noun",
                    actual: self.root.pos,
                });
            }
            if !is_noun_root && !slot.is_verb_slot() {
                return Err(CompositionError::SlotRootMismatch {
                    slot,
                    expected: "verb",
                    actual: self.root.pos,
                });
            }
            // Rule 2: slot uniqueness.
            if seen.contains(&slot) {
                return Err(CompositionError::DuplicateSlot { slot });
            }
            // Rule 3: canonical order.
            let order = if is_noun_root {
                slot.noun_order()
            } else {
                slot.verb_order()
            };
            if let (Some(prev), Some(cur)) = (last_order, order) {
                if cur < prev {
                    let prev_slot = *seen.last().expect("seen.last exists since prev is Some");
                    return Err(CompositionError::OutOfOrder {
                        late: slot,
                        early: prev_slot,
                    });
                }
            }
            seen.push(slot);
            if let Some(o) = order {
                last_order = Some(o);
            }
        }
        Ok(())
    }

    /// Number of operators applied (`0` for the identity composition).
    pub fn arity(&self) -> usize {
        self.operators.len()
    }

    /// Iterate the slot kinds in composition order (skipping the
    /// root). Useful for diagnostic / trace output.
    pub fn slot_sequence(&self) -> impl Iterator<Item = SlotKind> + '_ {
        self.operators.iter().map(|op| op.slot_kind())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adam_kernel_fst::morphotactics::{Case, Number, Person, Tense, Voice};

    fn noun_root_entry(root: &str) -> RootEntry {
        RootEntry {
            id: format!("test_{root}"),
            root: root.to_string(),
            part_of_speech: "noun".to_string(),
            vowel_harmony: "back".to_string(),
            final_sound_class: "vowel".to_string(),
        }
    }

    fn verb_root_entry(root: &str) -> RootEntry {
        RootEntry {
            id: format!("test_{root}"),
            root: root.to_string(),
            part_of_speech: "verb".to_string(),
            vowel_harmony: "back".to_string(),
            final_sound_class: "vowel".to_string(),
        }
    }

    #[test]
    fn identity_composition_is_well_formed() {
        let c = Composition::identity(Root::noun("адам"));
        assert!(c.is_well_formed().is_ok());
        assert_eq!(c.arity(), 0);
    }

    #[test]
    fn round_trip_noun_plural_dative() {
        let mut features = NounFeatures::default();
        features.number = Some(Number::Plural);
        features.case = Some(Case::Dative);
        let analysis = Analysis::Noun {
            root: noun_root_entry("адам"),
            features,
        };
        let comp = Composition::from_analysis(&analysis).expect("composition");
        assert_eq!(
            comp.operators,
            vec![
                SuffixOp::Number(Number::Plural),
                SuffixOp::Case(Case::Dative)
            ],
        );
        assert!(comp.is_well_formed().is_ok());
        let recovered = comp.to_noun_features().expect("noun features");
        assert_eq!(recovered, features);
    }

    #[test]
    fn round_trip_verb_present_first_singular() {
        let mut features = VerbFeatures::default();
        features.voice = Some(Voice::Active);
        features.tense = Some(Tense::Present);
        features.person = Some(Person::First);
        features.number = Some(Number::Singular);
        let analysis = Analysis::Verb {
            root: verb_root_entry("бар"),
            features,
        };
        let comp = Composition::from_analysis(&analysis).expect("composition");
        assert!(comp.is_well_formed().is_ok());
        let recovered = comp.to_verb_features().expect("verb features");
        assert_eq!(recovered, features);
    }

    #[test]
    fn round_trip_verb_negation_and_polite() {
        let mut features = VerbFeatures::default();
        features.tense = Some(Tense::PastDefinite);
        features.negation = true;
        features.person = Some(Person::Second);
        features.polite = true;
        let analysis = Analysis::Verb {
            root: verb_root_entry("кел"),
            features,
        };
        let comp = Composition::from_analysis(&analysis).expect("composition");
        assert!(comp.operators.contains(&SuffixOp::Negation));
        assert!(comp.operators.contains(&SuffixOp::VerbPolite(true)));
        assert!(comp.is_well_formed().is_ok());
        let recovered = comp.to_verb_features().expect("verb features");
        assert_eq!(recovered, features);
    }

    #[test]
    fn slot_root_mismatch_rejects_verb_op_on_noun_root() {
        let comp = Composition {
            root: Root::noun("адам"),
            operators: vec![SuffixOp::Tense(Tense::Present)],
        };
        let err = comp.is_well_formed().unwrap_err();
        assert!(matches!(err, CompositionError::SlotRootMismatch { .. }));
    }

    #[test]
    fn duplicate_slot_rejected() {
        let comp = Composition {
            root: Root::noun("адам"),
            operators: vec![SuffixOp::Case(Case::Dative), SuffixOp::Case(Case::Locative)],
        };
        let err = comp.is_well_formed().unwrap_err();
        assert!(matches!(err, CompositionError::DuplicateSlot { slot } if slot == SlotKind::Case));
    }

    #[test]
    fn out_of_order_rejected() {
        // Canonical noun order: Number(1) → Case(3). Reverse them
        // and the validator should fire.
        let comp = Composition {
            root: Root::noun("адам"),
            operators: vec![
                SuffixOp::Case(Case::Dative),
                SuffixOp::Number(Number::Plural),
            ],
        };
        let err = comp.is_well_formed().unwrap_err();
        assert!(matches!(err, CompositionError::OutOfOrder { .. }));
    }

    #[test]
    fn unknown_pos_returns_error() {
        let bogus = RootEntry {
            id: "test_bogus".to_string(),
            root: "x".to_string(),
            part_of_speech: "unicorn".to_string(),
            vowel_harmony: "back".to_string(),
            final_sound_class: "vowel".to_string(),
        };
        let analysis = Analysis::Noun {
            root: bogus,
            features: NounFeatures::default(),
        };
        let err = Composition::from_analysis(&analysis).unwrap_err();
        assert!(matches!(err, CompositionError::UnknownPos(_)));
    }

    #[test]
    fn slot_sequence_iteration() {
        let comp = Composition {
            root: Root::noun("адам"),
            operators: vec![
                SuffixOp::Number(Number::Plural),
                SuffixOp::Case(Case::Dative),
            ],
        };
        let kinds: Vec<SlotKind> = comp.slot_sequence().collect();
        assert_eq!(kinds, vec![SlotKind::Number, SlotKind::Case]);
    }
}
