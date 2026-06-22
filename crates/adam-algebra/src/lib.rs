// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `adam-algebra` — **Stage 1 of v6.2.0 neurosymbolic redesign**.
//!
//! Lifts the typed morphology of [`adam_kernel_fst::morphotactics`]
//! ([`Case`], [`Number`], [`Possessive`], [`Predicate`], [`Tense`],
//! [`Voice`], [`Person`], [`Derivation`]) into a uniform algebraic
//! surface: a single [`SuffixOp`] enum carrying every operator the
//! FST can apply, and a [`Composition`] type that lifts a
//! [`adam_kernel_fst::parser::Analysis`] into an ordered, slot-typed
//! sequence of operators with documented algebra rules.
//!
//! ## Why this layer exists (v6.2 thesis)
//!
//! Per [`docs/v6_2_architectural_redesign.md`](../../../docs/v6_2_architectural_redesign.md)
//! §"The unified abstraction — Agglutinative Algebra": Kazakh's
//! agglutinative morphology is **already** a typed algebra at the
//! FST level (illegal compositions are FST-rejected). What's missing
//! is a uniform Rust surface where every operator carries the same
//! type signature, so downstream layers (FrameLayer, QueryIRLayer,
//! the v6.2 learned semantic parser) can reason about composition,
//! idempotence, and inverse algebraically — not via scattered field
//! accesses on `NounFeatures` / `VerbFeatures`.
//!
//! ## Scope
//!
//! Stage 1 (shipped 4f6f0d5c):
//!
//! - [`Root`] — typed root wrapper with part-of-speech tag.
//! - [`SuffixOp`] — uniform enum subsuming every FST-known operator.
//! - [`SlotKind`] — the typed slot each operator occupies.
//! - [`Composition`] — ordered, validity-checked sequence of
//!   operators bound to a root, derived from
//!   [`Analysis::Noun`](adam_kernel_fst::parser::Analysis::Noun) /
//!   [`Analysis::Verb`](adam_kernel_fst::parser::Analysis::Verb).
//! - Algebra rules:
//!   - [`Composition::is_well_formed`] — composition validity.
//!   - [`SuffixOp::is_idempotent`] — identifies idempotent operators.
//!   - [`SuffixOp::inverse`] — operators with inverses.
//! - Round-trip: [`Composition::from_analysis`] →
//!   [`Composition::to_analysis`] is identity-preserving.
//!
//! Stage 2 (shipped bc5e0b79):
//!
//! - [`Frame`] — typed semantic record `(agent, predicate, object,
//!   modifiers, modality, polarity, evidentiality, tense, aspect)`.
//! - [`FramePredicate`] — closed-set predicate enum subsuming every
//!   v6.1 [`adam_reasoning::Predicate`] variant.
//! - [`Modality`] / [`Polarity`] / [`Evidentiality`] / [`Aspect`] /
//!   [`Modifier`] — typed dimensions of the frame.
//! - [`Frame::from_morph_lattice`] — deterministic assembler from
//!   a per-word [`Composition`] lattice.
//!
//! Stage 3 (shipped dfce4c7b):
//!
//! - [`QueryIR`] — typed query record `(agent, predicate, object,
//!   modifier_constraints, focus, form, answer_shape, sense_hints,
//!   domain_filter)`. Frame with a hole.
//! - [`QueryFocus`] / [`ModifierRole`] / [`QuestionForm`] /
//!   [`AnswerShape`] / [`ModifierConstraint`] / [`SenseHint`] /
//!   [`Domain`] — typed dimensions of the query.
//! - [`QueryIR::from_question_frame`] / [`QueryIR::from_assertion`] —
//!   bridges from a [`Frame`].
//! - [`QueryIR::match_frame`] — given a retrieved candidate
//!   [`Frame`], does it answer this query? Returns [`AnswerSlot`].
//!
//! Stage 4 (this commit):
//!
//! - [`FrameIndex`] — typed in-memory index over [`Frame`]s with
//!   secondary indexes by predicate / agent root / object root /
//!   modifier (role, value) / domain.
//! - [`FrameId`] — opaque u32 frame identifier.
//! - [`IndexedFrame`] / [`RankedFrame`] — index entry + retrieval
//!   hit types.
//! - [`FrameIndex::insert`] / [`FrameIndex::query`] /
//!   [`FrameIndex::best_match`] — insert + retrieve in
//!   O(min-applicable-index-size), then run Stage 3 `match_frame`
//!   on the survivor set for scoring.
//!
//! NOT in Stage 4 (deferred to later v6.2 stages):
//!
//! - Persistence — Stage 4 is in-memory only. Disk-backed indexes
//!   are a Stage 9 (ARM PoC) concern.
//! - Sense disambiguation policy (Stage 5) — resolves competing
//!   `SenseHint`s. Stage 4 only mechanically applies
//!   `domain_filter`.
//! - Learned components (Stage 6 — closed-set neural).
//! - Natural realiser (Stage 7) — owns the Frame ↔ surface mapping
//!   and the rewire of v6.1 NLG rule + question-routing callers.
//! - HumanDialogEval (Stage 8).
//! - ARM PoC (Stage 9).
//!
//! ## Determinism contract
//!
//! Every type in this crate is **pure-deterministic and
//! finite-set**. No probability, no learned weights, no RNG. Round-
//! trip equivalence with `adam_kernel_fst::parser::Analysis` is the
//! load-bearing invariant — broken round-trip is a unit-test
//! failure, not a warning.

pub mod composition;
pub mod corpus_loader;
pub mod dialog_battery;
pub mod frame;
pub mod index;
pub mod math_solver;
pub mod operator;
pub mod procedure;
pub mod query;
pub mod realiser;
pub mod root;
pub mod system_clock;

pub use composition::{Composition, CompositionError};
pub use frame::{
    Aspect, ContextSentenceType, Evidentiality, Frame, FramePredicate, Modality, Modifier,
    Polarity, QuestionFocus, SentenceContext, TimeAnchor,
};
pub use index::{FrameId, FrameIndex, IndexedFrame, RankedFrame};
pub use operator::{SlotKind, SuffixOp};
pub use procedure::{
    Hazard, ProcedureDomain, ProcedureIR, ProcedureParseError, ProcedureSource, ProcedureStep,
};
pub use query::{
    AnswerShape, AnswerSlot, Domain, FrameMatch, Language, ModifierConstraint, ModifierRole,
    QueryFocus, QueryIR, QuestionForm, SenseHint,
};
pub use root::{PartOfSpeech, Root};
