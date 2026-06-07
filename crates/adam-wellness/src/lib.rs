// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! # adam-wellness — Kazakh reflective AI companion (v6.4)
//!
//! ## What this crate is
//!
//! A structured-dialog layer that helps Kazakh-speaking users
//! reflect on emotional difficulties — pain, grievance, anger,
//! fear, shame — through evidence-based therapy frameworks.
//!
//! The first framework is **IFS** (Internal Family Systems,
//! Richard Schwartz).  IFS frames each emotion as a "часть" with
//! a protective function: the goal of the dialog is to identify
//! the part, witness what it protects, and help the user move
//! toward what IFS calls "Self-energy" — calm presence from which
//! the emotional charge can release.
//!
//! Later layers (planned, not in v6.4.0): **CBT** automatic-
//! thought spotting and **ACT** acceptance + values clarification.
//!
//! ## What this crate is NOT
//!
//! - **NOT a medical treatment system.** adam does not diagnose
//!   any condition.  adam does not prescribe.  adam does not
//!   claim to cure psoriasis, diabetes, cancer, autoimmune
//!   disease, depression, anxiety, or any other diagnosis.
//! - **NOT a replacement for clinical care.**  Every wellness
//!   session opens with «алдымен дәрігерге қаралыңыз» framing
//!   when the topic involves a medical symptom.
//! - **NOT a crisis hotline.**  When red-flag signal is detected
//!   (see [`red_flags`]), the dialog HARD-ESCALATES with a
//!   scripted referral to a Kazakhstan crisis line.  adam does
//!   not negotiate, soften, or continue parts-work in that case.
//!
//! ## Hamer / GNM (German New Medicine) explicitly rejected
//!
//! The original user proposal (2026-06-04) asked for training
//! adam on Dr. Ryke Hamer's "German New Medicine" framework.
//! That proposal was rejected for cause: documented child deaths
//! (Olivia Pilhar, Susanne, Mireille) where families followed
//! GNM and abandoned standard oncology.  Translating GNM into
//! Kazakh would extend that harm to a population currently
//! shielded by language.  adam-wellness uses evidence-based
//! frameworks (IFS / CBT / ACT) only.
//!
//! ## Layered design
//!
//! ```text
//!     user utterance (KZ)
//!          │
//!          ▼
//!   ┌─────────────────────────┐
//!   │  red_flags::detect      │  ← runs FIRST on every turn
//!   └─────┬───────────────────┘
//!         │ Some(flag) → CrisisRedirect (scripted, no parts-work)
//!         │ None       → fall through
//!         ▼
//!   ┌─────────────────────────┐
//!   │  ifs::next_stage(state) │  ← state machine: 6 IFS stages
//!   └─────┬───────────────────┘
//!         │
//!         ▼
//!     template-fill (Kazakh)
//!          │
//!          ▼
//!        reply
//! ```
//!
//! The red-flag layer is checked **before** any IFS state
//! transition.  This is non-negotiable; see [`red_flags`] for
//! the detector + escalation templates.

pub mod ifs;
pub mod red_flags;
