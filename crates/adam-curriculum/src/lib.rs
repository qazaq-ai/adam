// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `adam-curriculum` — L7-L10-edu layers for the «Qazaq AI Ұстаз»
//! educational product.
//!
//! Layered on top of the v6.0 deterministic kernel + neural
//! composition layer. Four modules map 1:1 to the four pedagogical
//! layers specified in
//! [`docs/product/qazaq_ai_ustaz_v1.md`](../../../docs/product/qazaq_ai_ustaz_v1.md):
//!
//! - **`concept`** (L7-edu) — curriculum graph: concept nodes,
//!   prerequisite DAG, validation.
//! - **`diagnostic`** (L8-edu) — given a target concept the learner
//!   failed, find the deepest unmastered prerequisite. The
//!   "name the specific misconception" step.
//! - **`planner`** (L9-edu) — deterministic next-lesson selector.
//!   No stochastic spaced-repetition; SRS is a discrete schedule.
//! - **`outcome`** (L10-edu) — measurable mastery verifier. Sole
//!   writer of the «mastered» flag on the learner record. The
//!   single source of truth for the product's «measurable mastery
//!   per concept» rubric.
//!
//! Supporting modules:
//!
//! - **`learner`** — learner state: `LearnerRecord`, attempt history,
//!   per-concept mastery view. Read-only to L9 and L10 except
//!   through the typed `record_attempt` / `mark_mastered` writers.
//! - **`error`** — typed `CurriculumError` and `Result`.
//!
//! ## Design invariants
//!
//! 1. **Determinism.** Every public function in this crate is a
//!    pure function of its inputs. Same `(graph, learner, …)` →
//!    byte-identical result.
//! 2. **Only outcome verifier may declare mastery.** L9-edu reads
//!    the mastered flag but cannot set it. This invariant lets us
//!    audit «why is concept X marked mastered» with a one-line
//!    answer.
//! 3. **Append-only attempt history.** Re-testing updates the
//!    mastered flag but never erases the historical attempt
//!    records. The L10 outcome verifier is the only writer of the
//!    flag; attempts themselves are written by callers via
//!    `LearnerRecord::record_attempt`.
//! 4. **Graph is a DAG.** `ConceptGraph::insert` enforces this via
//!    cycle detection. Authoring tools must satisfy this; the
//!    runtime trusts the loaded graph.
//!
//! See the per-module docs for further detail.

pub mod concept;
pub mod diagnostic;
pub mod error;
pub mod learner;
pub mod outcome;
pub mod planner;

pub use concept::{Concept, ConceptGraph, ConceptId, Pillar};
pub use diagnostic::{Diagnosis, diagnose};
pub use error::{CurriculumError, Result};
pub use learner::{AttemptRecord, ConceptMastery, LearnerRecord};
pub use outcome::{NotYetReason, VerifyOutcome, verify_concept};
pub use planner::{LessonRationale, NextLesson, plan_next};
