// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `adam-ingestion` — **typed data-ingestion pipeline
//! foundation**.
//!
//! Front-half of the «raw KZ text → world_core» pipeline the
//! 2026-06-27 strategic review named the missing piece for
//! ADAM growth: every other Kazakh-knowledge-coverage gap
//! the school-tutor / industrial-SOP eval suites surface
//! ultimately bottlenecks here.  Curated data scale is what
//! the existing deterministic kernel needs; this crate gives
//! that growth a typed, queueable, reviewable shape.
//!
//! ## What this crate is
//!
//! Three things:
//!
//! 1. **Typed candidate records.**  [`CandidateFact`] and
//!    [`CandidateProcedure`] mirror the world_core /
//!    procedure JSONL surfaces that production already
//!    consumes, plus extra fields the pipeline needs:
//!    `source` (provenance — where did this come from),
//!    `status` (where in the queue), `confidence`
//!    (extractor's self-assessment), `created_at` /
//!    `reviewed_at` (audit trail).
//!
//! 2. **Status state machine.**  [`IngestionStatus`] encodes
//!    the queue transitions every candidate goes through.
//!    Transitions are validated by [`IngestionStatus::can_transition`]
//!    so a candidate can't be marked `IntegratedIntoWorldCore`
//!    without having first been `ApprovedByHuman`.
//!
//! 3. **Persistent JSONL store.**  [`CandidateStore`] reads
//!    and writes one-record-per-line JSONL files under a
//!    configurable root, so the queue survives across
//!    pipeline-stage invocations (extractor / validator /
//!    review / integrator).
//!
//! ## What this crate is NOT (yet)
//!
//! No extractor (raw text → candidates).  No validator
//! (dup-check against world_core, contradiction detection).
//! No CLI review queue.  No integrator (approved → world_core
//! jsonl + auto-generated eval cases).  Those are subsequent
//! commits on this arc — each well-scoped, each landing
//! against this typed foundation rather than redefining its
//! own schema.
//!
//! ## Why a separate crate (not a module inside
//! `adam-reasoning`)
//!
//! Three reasons:
//!
//!   * Single-direction dependency — `adam-reasoning` is
//!     downstream of the world_core JSONL; the ingestion
//!     pipeline is upstream.  Mixing them would create
//!     either a cycle or a too-broad crate.
//!   * Different cadence — production-side world_core
//!     reading is stable; ingestion-side schema will evolve
//!     as extractors mature.  Separate crate isolates the
//!     churn.
//!   * Testable in isolation — ingestion has no runtime
//!     dependency on the cascade.  Unit tests don't need
//!     `Conversation`-level fixtures.

pub mod candidate;
pub mod extractor;
pub mod source;
pub mod status;
pub mod store;

pub use candidate::{CandidateFact, CandidateId, CandidateProcedure, ParseError};
pub use extractor::extract_facts_from_text;
pub use source::{SourceKind, SourceRef};
pub use status::{IngestionStatus, StatusTransitionError};
pub use store::{CandidateStore, StoreError};
