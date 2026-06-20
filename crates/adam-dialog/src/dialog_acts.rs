// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! # Dialog acts — typed L4.5 foundation (Phase 1)
//!
//! ## What this closes
//!
//! The 2026-06-17 external consultation surfaced a class of
//! interactive-dialog failures that all share one root cause: the
//! `v6_2_router` returns a `String`, and that string is substituted
//! into the cascade output AFTER the v6.1 verifier has already
//! checked a DIFFERENT answer.  Consequences:
//!
//! - The `proof_object` and `trace` describe one answer; the user
//!   reads another.  «Every emitted claim is verified» becomes
//!   technically false.
//! - There is no place to carry the speaker's intent (was this an
//!   Assert? a Refuse? a Clarify?), the route that produced it, or
//!   the state mutations it wants to apply.
//! - There is no place to record «the USER said X» vs «X is true»
//!   — letting a user statement silently overwrite curated truth
//!   («Алматы — астана» becomes a fact adam adopts).
//!
//! The advisor's L4.5 sketch addresses all three with one typed
//! pipeline.  Phase 1 lands the FOUNDATION types only — no
//! behavioural change yet.  Routes can opt in by returning
//! [`AnswerCandidate`] alongside their existing `String` return;
//! Phase 2+ migrate routes one at a time.
//!
//! ## Phase 1 scope
//!
//! Defines:
//! - [`AnswerCandidate`] — the typed reply: `(moves, text, proof,
//!   route, state_delta)`.  The verifier's contract: the
//!   `proof.conclusion` describes the same fact as `text`, by
//!   construction.
//! - [`DialogueMove`] — minimal enum for the migrated routes
//!   (Assert + Refuse).  Codex's full inventory (Ask / Confirm /
//!   Correct / Clarify / Warn / Suggest / Repair / Meta) lands as
//!   each variant gets its first user.
//! - [`RouteId`] — provenance attribution for arbitration and
//!   trace rendering.
//! - [`CommitmentRecord`] + [`CommitmentStatus`] — statused user
//!   beliefs so «User said X» stays distinct from «X is curated
//!   truth».  Storage and policy land in Phase 2 (when
//!   `DiscourseState` ships).
//! - [`StateDelta`] — session-slot mutations a route wants to apply
//!   atomically on win.
//! - [`PolicyReason`] — typed refusal reasons.
//!
//! Wires ONE canary route ([`crate::v6_2_router::lookup_person_lifespan`])
//! to the typed pipeline as a proof of concept.  Everything else
//! stays byte-identical.
//!
//! ## What does NOT change in Phase 1
//!
//! - `Conversation::turn` + `turn_with_trace` keep the existing
//!   `String` return; the typed pipeline runs alongside.
//! - All five production eval suites must stay green at the same
//!   scores (159 / 159 · 52 / 52 · 22 / 22 · 25 / 26 · 37 / 71).
//! - No `DiscourseState` field on `Conversation` yet — that's
//!   Phase 2.

use crate::proof_object::ProofObject;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Provenance attribution for which cascade route produced an
/// [`AnswerCandidate`].  Used by the arbitration policy to select
/// among competing candidates and to render trace output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RouteId {
    /// `v6_2_router::lookup_person_lifespan` — BornIn + DiedIn join.
    Lifespan,
    /// `v6_2_router::lookup_chemical_formula`.
    ChemistryFormula,
    /// `v6_2_router::lookup_possessive_property`.
    PossessiveProperty,
    /// `v6_2_router::needs_live_data_refusal` — crypto / weather / news.
    LiveDataRefusal,
    /// `v6_2_router::is_self_identity_query` — adam identity self-id.
    SelfIdentity,
    /// `v6_2_router::is_capabilities_query`.
    Capabilities,
    /// `v6_2_router::is_personal_experience_query` — refuses lived
    /// -experience presupposition probes.
    PersonalExperienceRefusal,
    /// `wellness::red_flags` escalation (1415 / 103 / 150 / 112).
    RedFlag,
    /// `safety_guard` refusal (Medical / Weapon / Illegal /
    /// HarmToOthers).
    SafetyGuard,
    /// `math_solver`.
    Math,
    /// `system_clock`.
    Clock,
    /// `v6.1` template cascade — covers everything not yet typed.
    V61Cascade,
    /// `v6_2_router::FrameIndex` retrieval + realiser.
    FrameRealised,
}

/// Speaker attribution for a dialogue move or commitment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Speaker {
    User,
    Adam,
}

/// Provenance status for a user-introduced commitment.  Stored
/// in `DiscourseState::commitments` (Phase 2).  Phase 1 defines
/// the type so future migrations can reference it.
///
/// **Why this matters:** Codex flagged that storing user beliefs
/// as raw `Vec<Frame>` lets a user statement silently overwrite
/// curated truth.  Statused commitments separate «User said X»
/// (Proposed) from «adam echoes X» (Accepted) from «adam offered
/// the correct value» (Rejected) from «unresolved disagreement»
/// (Contested).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CommitmentStatus {
    /// User said it; adam has not confirmed.  Default for any
    /// new user-asserted fact.
    Proposed,
    /// adam confirmed it (echoed, used downstream in inference).
    Accepted,
    /// adam rejected it (offered a correction in the same turn).
    Rejected,
    /// User and adam disagree; unresolved.  Subsequent turns can
    /// resolve via clarification or repair.
    Contested,
}

/// One commitment made by a participant.  Phase 1 defines the
/// type for forward compatibility; Phase 2 wires the storage on
/// `DiscourseState`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitmentRecord {
    pub author: Speaker,
    /// Surface text of the claim as the user / adam said it.
    /// Phase 2 may add a typed `Frame` here once we know how
    /// every route serialises its claim.
    pub claim_text: String,
    pub status: CommitmentStatus,
    /// Which turn introduced this commitment.  Lets the policy
    /// reason about temporal ordering (most-recent-wins on a
    /// repair).
    pub turn_id: u64,
}

/// Typed reason for refusing to answer.  Carried by
/// [`DialogueMove::Refuse`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyReason {
    /// Closed-set harm class (`safety_guard` Medical / Weapon /
    /// Illegal / HarmToOthers).
    SafetyHarm,
    /// Crisis-class signal (`wellness::red_flags` Suicidal /
    /// AcuteMedical / ChildAbuse / DV-Immediate / Psychosis).
    Crisis,
    /// No live data feed (crypto price, weather, currency rate,
    /// news, sports score).
    NoLiveData,
    /// Question is in a domain adam does not cover (open-ended
    /// generation, creative writing, role-play).
    BeyondScope,
    /// The fact is not in the curated graph — honest «нақты
    /// дерегім жоқ».
    NoData,
    /// Presupposition refusal — user assumes adam did something
    /// (read / saw / ate / travelled) it cannot have done.
    PresuppositionFailure,
}

/// What the speaker is DOING with this turn.
///
/// Phase 1 ships only the variants the migrated routes need
/// (`Assert`, `Refuse`).  Codex's full inventory (Ask / Confirm /
/// Reject / Correct / Clarify / Warn / Suggest / Repair / Meta)
/// lands as each variant gets its first user.
///
/// **Why not ship the full enum now:** every variant defined
/// without a producer creates dead code that clippy flags and
/// invites premature design.  We add a variant when a route
/// needs it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DialogueMove {
    /// adam states a fact backed by an [`AnswerCandidate::proof`].
    /// The `claim` text matches the parent candidate's `text`.
    Assert { claim: String },
    /// adam declines to answer for a typed policy reason.  The
    /// refusal surface lives in `AnswerCandidate::text`.
    Refuse(PolicyReason),
}

/// Session-slot mutations a route wants to apply atomically when
/// its [`AnswerCandidate`] wins arbitration.  Phase 1 carries an
/// empty delta for read-only handlers (most factual lookups);
/// Phase 2+ migrates the slot-writing routes (name capture, city
/// capture, age, etc.).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateDelta {
    /// Slots to set (key → new value).
    pub session_set: BTreeMap<String, String>,
    /// Slots to remove.
    pub session_unset: Vec<String>,
}

impl StateDelta {
    /// `true` when the delta is empty — convenient for the read-
    /// only route guard.
    pub fn is_empty(&self) -> bool {
        self.session_set.is_empty() && self.session_unset.is_empty()
    }
}

/// One typed reply produced by ONE route.  The arbitration policy
/// (Phase 2) picks among competing candidates; the verifier
/// audits the WINNER's proof against its text; the cascade
/// atomically commits the [`StateDelta`].
///
/// **Phase 1 contract** for routes that emit a candidate: the
/// `proof.conclusion` describes the same fact as `text`.  This
/// closes the v6.2-overwrites-after-verification bug by
/// construction: the candidate IS the verified text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerCandidate {
    /// Dialogue acts this candidate performs.  Most v6.8 routes
    /// emit a single [`DialogueMove::Assert`] or a single
    /// [`DialogueMove::Refuse`]; complex turns (warn-then-ask)
    /// can carry several.  Order matters for surface emission.
    pub moves: Vec<DialogueMove>,
    /// Kazakh surface text the user sees.
    pub text: String,
    /// Typed proof backing this candidate.
    pub proof: ProofObject,
    /// Which route generated the candidate.
    pub route: RouteId,
    /// Session-state mutations the route wants to apply on win.
    pub state_delta: StateDelta,
}

impl AnswerCandidate {
    /// Construct a single-`Assert` candidate.  Helper for the
    /// common case where a route emits one factual claim with
    /// one proof.
    pub fn assert(text: String, proof: ProofObject, route: RouteId) -> Self {
        Self {
            moves: vec![DialogueMove::Assert {
                claim: text.clone(),
            }],
            text,
            proof,
            route,
            state_delta: StateDelta::default(),
        }
    }

    /// Construct a single-`Refuse` candidate.
    pub fn refuse(text: String, proof: ProofObject, route: RouteId, reason: PolicyReason) -> Self {
        Self {
            moves: vec![DialogueMove::Refuse(reason)],
            text,
            proof,
            route,
            state_delta: StateDelta::default(),
        }
    }

    /// Attach a [`StateDelta`] to a candidate.  Builder helper for
    /// the slot-writing routes Phase 2 migrates.
    #[must_use]
    pub fn with_state_delta(mut self, delta: StateDelta) -> Self {
        self.state_delta = delta;
        self
    }

    /// Phase 1 invariant check — verify the candidate's first
    /// `Assert` move (if any) carries the same surface as the
    /// emitted `text`.  Catches obvious shape mismatches before
    /// the candidate reaches the verifier.  Returns the reason
    /// the candidate is malformed, or `Ok(())` when consistent.
    pub fn invariant_check(&self) -> Result<(), &'static str> {
        if self.text.is_empty() {
            return Err("AnswerCandidate.text must not be empty");
        }
        for m in &self.moves {
            if let DialogueMove::Assert { claim } = m {
                if claim != &self.text {
                    return Err("DialogueMove::Assert.claim must match AnswerCandidate.text");
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proof_object::{Claim, ClaimPredicate, Polarity, ProofObject, SupportKind};

    fn make_test_proof() -> ProofObject {
        ProofObject {
            conclusion: Claim {
                subject: "test".into(),
                predicate: ClaimPredicate::IsA,
                object: "test".into(),
                polarity: Polarity::Affirmative,
            },
            support: vec![],
            derivation: None,
            hedges: vec![],
            unsupported_claims: vec![],
        }
    }

    /// `AnswerCandidate::assert` produces a single-move candidate
    /// where the Assert claim equals the emitted text.
    #[test]
    fn assert_builder_produces_consistent_candidate() {
        let proof = make_test_proof();
        let c = AnswerCandidate::assert("Hello.".into(), proof, RouteId::Lifespan);
        assert_eq!(c.text, "Hello.");
        assert_eq!(c.route, RouteId::Lifespan);
        assert_eq!(c.moves.len(), 1);
        match &c.moves[0] {
            DialogueMove::Assert { claim } => assert_eq!(claim, "Hello."),
            other => panic!("expected Assert, got {other:?}"),
        }
        assert!(c.state_delta.is_empty());
        assert!(c.invariant_check().is_ok());
    }

    /// `AnswerCandidate::refuse` produces a single-Refuse with
    /// the typed reason carried through.
    #[test]
    fn refuse_builder_carries_typed_reason() {
        let proof = ProofObject::safety_refusal(
            "test input".into(),
            "noliv".into(),
            crate::proof_object::SafetyDomain::CurrentData,
        );
        let c = AnswerCandidate::refuse(
            "Дерегім жоқ.".into(),
            proof,
            RouteId::LiveDataRefusal,
            PolicyReason::NoLiveData,
        );
        assert_eq!(c.route, RouteId::LiveDataRefusal);
        match &c.moves[0] {
            DialogueMove::Refuse(reason) => assert_eq!(*reason, PolicyReason::NoLiveData),
            other => panic!("expected Refuse, got {other:?}"),
        }
        assert!(c.invariant_check().is_ok());
    }

    /// Invariant check catches the case where an Assert's claim
    /// drifts from the candidate's text — the bug Codex flagged
    /// at the cascade level.
    #[test]
    fn invariant_check_rejects_claim_text_mismatch() {
        let proof = make_test_proof();
        let bad = AnswerCandidate {
            moves: vec![DialogueMove::Assert {
                claim: "Answer A".into(),
            }],
            text: "Answer B".into(),
            proof,
            route: RouteId::FrameRealised,
            state_delta: StateDelta::default(),
        };
        assert!(bad.invariant_check().is_err());
    }

    /// Empty text is rejected — a candidate without surface is
    /// meaningless and must not flow into the cascade.
    #[test]
    fn invariant_check_rejects_empty_text() {
        let proof = make_test_proof();
        let bad = AnswerCandidate {
            moves: vec![],
            text: String::new(),
            proof,
            route: RouteId::Math,
            state_delta: StateDelta::default(),
        };
        assert!(bad.invariant_check().is_err());
    }

    /// `with_state_delta` attaches a delta and is observable via
    /// `state_delta.is_empty()`.
    #[test]
    fn with_state_delta_attaches_mutations() {
        let proof = make_test_proof();
        let mut delta = StateDelta::default();
        delta.session_set.insert("city".into(), "Қостанай".into());
        let c =
            AnswerCandidate::assert("ok".into(), proof, RouteId::Lifespan).with_state_delta(delta);
        assert!(!c.state_delta.is_empty());
        assert_eq!(
            c.state_delta.session_set.get("city").map(String::as_str),
            Some("Қостанай"),
        );
    }

    /// CommitmentRecord round-trip — Phase 1 sanity that the type
    /// can be constructed and inspected; Phase 2 wires storage.
    #[test]
    fn commitment_record_roundtrip() {
        let r = CommitmentRecord {
            author: Speaker::User,
            claim_text: "Менің атым — Дәулет.".into(),
            status: CommitmentStatus::Proposed,
            turn_id: 7,
        };
        assert_eq!(r.author, Speaker::User);
        assert_eq!(r.status, CommitmentStatus::Proposed);
        assert_eq!(r.turn_id, 7);
        assert!(r.claim_text.contains("Дәулет"));
    }

    /// Reference SupportKind to keep the imported type live for
    /// future routes (avoids unused-import on the imports above).
    #[test]
    fn support_kind_type_referenced() {
        let _kind = SupportKind::CuratedFact;
    }
}
