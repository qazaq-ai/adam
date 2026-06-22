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
    /// **Phase 2.D — repair move.**  User retracts a prior
    /// commitment («Жоқ, X емес, Y») and replaces it.  The
    /// `rejected_value` matches a substring of the prior
    /// commitment's `claim_text`; the `replacement_value` becomes
    /// the new Proposed commitment on the same turn.  Phase 2.D
    /// detection lives in [`detect_correction_pattern`] +
    /// [`crate::Conversation::apply_correction`]; Phase 2.E will
    /// add the full intent-classifier path so adam's reply
    /// explicitly acknowledges the correction.
    Correct {
        rejected_value: String,
        replacement_value: String,
    },
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

/// **Phase 2.E.2** — typed referent for anaphora resolution.
/// Pushed onto [`DiscourseState::referents`] whenever a turn
/// surfaces a concrete topic (resolved subject in retrieval,
/// agent in `StatementOfName`, etc.).  Consulted by handlers
/// when the current input lacks an explicit subject and the
/// shape demands one («Қанша жыл өмір сүрді?» → take the most
/// recent person referent).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Referent {
    /// Canonical surface for the referent (lowercased, FST-
    /// resolvable — typically the same shape `canonical_agent_for`
    /// returns).
    pub token: String,
    /// Coarse-grained kind so callers can filter (a lifespan
    /// query wants a `Person` referent, not a `Place`).
    pub kind: ReferentKind,
    /// Turn in which this referent was last surfaced.
    pub turn_id: u64,
}

/// Coarse-grained referent type.  Phase 2.E.2 ships the minimum
/// set the migrated handlers care about; future phases add
/// `Date`, `Concept`, `Event`, etc. as new producers appear.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReferentKind {
    /// Person (historical figure, user themselves, named individual).
    Person,
    /// Place (city, country, region, building).
    Place,
    /// Organisation (institution, company, agency).
    Organisation,
    /// **v6.8.7 L4.8 C.2.** Procedure / SOP — token is the
    /// procedure id (e.g. `"kk_labor_ppe_002"`) so anaphora-
    /// aware procedure-attribute handlers (step count,
    /// authority) can fetch the full record from
    /// `procedure_loader::shared_procedures()` by id.
    Procedure,
    /// Generic / unclassified — the producer couldn't (yet) pin
    /// the type but wants the surface logged.
    Generic,
}

/// Discourse-level state carried by [`crate::Conversation`].  Phase
/// 2.B ships only the `commitments` field — the typed log of what
/// each participant has asserted, with provenance and status.
/// Phase 2.E.2 adds `referents` — the anaphora stack consulted
/// when an input lacks an explicit subject.  Future phases add
/// `register` (TY / VY politeness), `last_user_move` /
/// `last_system_move`, and `task`-state pointers.
///
/// **Why on `Conversation` and not inside `session: HashMap<String,
/// String>`:** the stringly-typed session map cannot record (a) who
/// authored a claim, (b) whether adam has confirmed it, (c) when it
/// was introduced, or (d) when it was contested.  Codex flagged the
/// failure mode where a user statement silently overwrites curated
/// truth.  Statused commitments separate «User said X» (Proposed)
/// from «adam echoes X» (Accepted) from «adam corrected X»
/// (Rejected) from «unresolved» (Contested).
///
/// Phase 2.B does NOT yet read commitments anywhere — it only
/// records them so Phase 2.C / 2.D have data to consume.  Behaviour
/// is byte-identical for every existing route.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscourseState {
    /// All commitments made by either speaker, in turn order.
    /// Most recent at the end.  The default capacity is empty;
    /// growth is unbounded for now (Phase 2 ships small dialogs).
    /// A bounded ring-buffer is a future-phase optimisation if we
    /// need it.
    pub commitments: Vec<CommitmentRecord>,
    /// **Phase 2.E.2.** Referent stack — surfaces of concrete
    /// entities recently brought into discourse, most-recent
    /// last.  Pushed when a turn resolves a subject (broad-topic
    /// retrieval, StatementOfName, etc.); consulted as anaphora
    /// fallback when an input lacks an explicit subject and the
    /// shape demands one.  Bounded growth is a future-phase
    /// optimisation; small dialogs stay cheap to walk.
    pub referents: Vec<Referent>,
}

/// **v6.8.4 L4.5 Phase 2.D.** Detect a repair / correction
/// pattern in `input`.  Returns `Some((rejected_value,
/// replacement_value))` when the input matches a recognised
/// retraction shape.
///
/// ## Supported shapes
///
/// Kazakh:
/// - «Жоқ, X емес, Y» / «жоқ, X емес, Y»
/// - «Жоқ, X емес, Y дегенмін»
/// - «Жоқ X емес Y» (no commas — Whisper STT drift)
///
/// Russian (code-switch under load):
/// - «Нет, не X, а Y»
/// - «Не X, а Y»
///
/// ## What this is NOT
///
/// Not a full intent classifier.  Returning `Some` here means the
/// SHAPE matches; the caller must still consult
/// `DiscourseState` to find a prior commitment whose claim
/// references the `rejected_value` before treating this as a true
/// repair turn.  The helper extracts the two value tokens
/// only — no normalisation, no FST round-trip.  Phase 2.E will
/// add the typed-intent classifier path.
pub fn detect_correction_pattern(input: &str) -> Option<(String, String)> {
    let lower = input.to_lowercase();

    // ── Kazakh «Жоқ, X емес, Y» / «Жоқ X емес Y» ──────────────
    // Markers: «жоқ» at start + «емес» somewhere later.  Locate
    // both in the lowercase view, then extract the X and Y slices
    // from the ORIGINAL input so the returned values preserve
    // case (`Алия` not `алия`).  Punctuation around the tokens is
    // tolerated.
    let lower_starts_zhoq =
        lower.starts_with("жоқ ") || lower.starts_with("жоқ,") || lower.starts_with("жоқ\n");
    if lower_starts_zhoq {
        // Slice past «жоқ» (5 bytes in UTF-8: ж=2 + о=1 + қ=2).
        let after_zhoq_start = "жоқ".len();
        // Find the «емес» marker (also 8 bytes: е=2+м=2+е=2+с=2).
        if let Some(emes_idx) = lower[after_zhoq_start..].find(" емес") {
            let emes_abs = after_zhoq_start + emes_idx;
            let rejected_slice = input[after_zhoq_start..emes_abs].trim();
            let rejected = rejected_slice
                .trim_matches([',', '.', '?', '!', ' '])
                .to_string();
            // After «емес» (4 chars = 8 bytes for these Cyrillic
            // letters), the replacement begins.
            let after_emes = emes_abs + " емес".len();
            let y_part = input[after_emes..].trim_start_matches([' ', ',', '\n']);
            let replacement = y_part
                .split(['.', '?', '!', ','])
                .next()
                .unwrap_or("")
                .trim()
                .trim_end_matches(" дегенмін")
                .trim_end_matches(" деймін")
                .trim()
                .to_string();
            if !rejected.is_empty() && !replacement.is_empty() {
                return Some((rejected, replacement));
            }
        }
    }

    // ── Russian «Нет, не X, а Y» / «Не X, а Y» ────────────────
    let net_ne_prefix_len = if lower.starts_with("нет, не ") {
        Some("нет, не ".len())
    } else if lower.starts_with("нет не ") {
        Some("нет не ".len())
    } else if lower.starts_with("не ") {
        Some("не ".len())
    } else {
        None
    };
    if let Some(start) = net_ne_prefix_len {
        let rest_lower = &lower[start..];
        // Find «, а » or « а » separator.
        let sep_match = rest_lower
            .find(", а ")
            .map(|i| (i, ", а ".len()))
            .or_else(|| rest_lower.find(" а ").map(|i| (i, " а ".len())));
        if let Some((sep_idx, sep_len)) = sep_match {
            let x_slice = input[start..start + sep_idx].trim();
            let rejected = x_slice.trim_matches([',', '.', '?', '!', ' ']).to_string();
            let y_start = start + sep_idx + sep_len;
            let replacement = input[y_start..]
                .split(['.', '?', '!', ','])
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            if !rejected.is_empty() && !replacement.is_empty() {
                return Some((rejected, replacement));
            }
        }
    }

    None
}

impl DiscourseState {
    /// Append a new commitment record.  Returns the index of the
    /// newly-inserted commitment for downstream cross-references
    /// (Phase 2.C arbitration will use these indices to relate
    /// candidates to the commitments they verify or contradict).
    pub fn record(&mut self, commitment: CommitmentRecord) -> usize {
        self.commitments.push(commitment);
        self.commitments.len() - 1
    }

    /// Most-recent commitment by `author`, if any.  Phase 2.D
    /// repair-act detection ( «Жоқ, X емес, Y» ) consults this to
    /// know which prior commitment to mark as Rejected.
    pub fn last_by(&self, author: Speaker) -> Option<&CommitmentRecord> {
        self.commitments.iter().rev().find(|c| c.author == author)
    }

    /// Filter view of all commitments by `author`.  Phase 2 readers
    /// will use this for stance summaries; Phase 1 has no consumers
    /// yet (this is the foundation pass).
    pub fn by_author(&self, author: Speaker) -> impl Iterator<Item = &CommitmentRecord> {
        self.commitments.iter().filter(move |c| c.author == author)
    }

    /// **v6.8.4 L4.5 Phase 2.C.** Mutable iterator over Proposed
    /// commitments authored by `User` on the given turn.  Phase 2.C
    /// commitment-promotion logic uses this to mark the
    /// just-absorbed user commitment as `Accepted` when the
    /// cascade successfully wrote the corresponding slot.
    pub fn proposed_user_commitments_for_turn(
        &mut self,
        turn_id: u64,
    ) -> impl Iterator<Item = &mut CommitmentRecord> {
        self.commitments.iter_mut().filter(move |c| {
            c.author == Speaker::User
                && c.status == CommitmentStatus::Proposed
                && c.turn_id == turn_id
        })
    }

    /// **v6.8.4 L4.5 Phase 2.E.2 (semantics revised v6.8.7
    /// L4.8 / C.1).** Push a referent onto the anaphora stack.
    /// If the most-recent entry already carries the same
    /// `token`, the existing entry's `kind` may be refreshed
    /// but its `turn_id` is LEFT UNCHANGED — `turn_id` records
    /// the turn the referent was **first introduced**, not the
    /// turn it was last mentioned.
    ///
    /// Why keep the introduction `turn_id`: the cascade's
    /// «prior-turn referent» filter (`r.turn_id < current_turn`)
    /// needs introduction-turn semantics.  If the dedupe
    /// refreshed `turn_id` to the latest mention, a topic
    /// introduced in turn 1 and re-mentioned via cascade-side
    /// topic extraction in turn 2 would carry `turn_id = 2`,
    /// and the filter would reject it as «same turn» — leaving
    /// the cascade with no anaphora hint precisely when the
    /// user is asking a follow-up.  Closes the
    /// birthplace_after_lifespan_anaphora /
    /// occupation_after_lifespan_anaphora probes that v6.8.6
    /// captured.
    pub fn push_referent(&mut self, token: String, kind: ReferentKind, turn_id: u64) {
        let normalised = token.trim().to_string();
        if normalised.is_empty() {
            return;
        }
        if let Some(last) = self.referents.last_mut() {
            if last.token == normalised {
                last.kind = kind;
                // `turn_id` intentionally NOT refreshed — see doc comment.
                return;
            }
        }
        self.referents.push(Referent {
            token: normalised,
            kind,
            turn_id,
        });
    }

    /// **Phase 2.E.2.** Most-recent referent of the requested
    /// `kind`, or `None` when the stack carries nothing matching.
    /// Used by handlers like `lookup_person_lifespan` when the
    /// current input lacks an explicit subject.
    pub fn last_referent_of_kind(&self, kind: ReferentKind) -> Option<&Referent> {
        self.referents.iter().rev().find(|r| r.kind == kind)
    }

    /// **Phase 2.E.2.** Most-recent referent of any kind.  Used by
    /// callers that don't yet distinguish kinds (legacy fallback
    /// path); prefer `last_referent_of_kind` when the call site
    /// can express its expected type.
    pub fn last_referent(&self) -> Option<&Referent> {
        self.referents.last()
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

    // Named test-fixture constants — placeholder personal names
    // for `CommitmentRecord` round-trip tests.  Reading
    // `TEST_FIRST_NAME_*` in a test body signals «fixture», not
    // a specific person.  See
    // `memory/feedback_test_fixture_names.md` for the full rule.
    const TEST_FIRST_NAME_USER: &str = "Алия";
    const TEST_FIRST_NAME_USER_CORRECTED: &str = "Бекжан";
    const TEST_HONORIFIC: &str = "Сізді есте сақтадым.";

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
        let claim = format!("Менің атым — {TEST_FIRST_NAME_USER}.");
        let r = CommitmentRecord {
            author: Speaker::User,
            claim_text: claim.clone(),
            status: CommitmentStatus::Proposed,
            turn_id: 7,
        };
        assert_eq!(r.author, Speaker::User);
        assert_eq!(r.status, CommitmentStatus::Proposed);
        assert_eq!(r.turn_id, 7);
        assert!(r.claim_text.contains(TEST_FIRST_NAME_USER));
    }

    /// Reference SupportKind to keep the imported type live for
    /// future routes (avoids unused-import on the imports above).
    #[test]
    fn support_kind_type_referenced() {
        let _kind = SupportKind::CuratedFact;
    }

    /// **Phase 2.B — DiscourseState ops.** `record` appends and
    /// returns the new index; `last_by` finds the most-recent
    /// commitment by author; `by_author` filters the full log.
    #[test]
    fn discourse_state_record_and_query() {
        let mut state = DiscourseState::default();
        assert!(state.commitments.is_empty());
        assert!(state.last_by(Speaker::User).is_none());

        let user_c = CommitmentRecord {
            author: Speaker::User,
            claim_text: format!("Менің атым — {TEST_FIRST_NAME_USER}."),
            status: CommitmentStatus::Proposed,
            turn_id: 1,
        };
        let adam_c = CommitmentRecord {
            author: Speaker::Adam,
            claim_text: TEST_HONORIFIC.to_string(),
            status: CommitmentStatus::Accepted,
            turn_id: 1,
        };
        let user2_c = CommitmentRecord {
            author: Speaker::User,
            claim_text: format!(
                "Жоқ, {TEST_FIRST_NAME_USER} емес, {TEST_FIRST_NAME_USER_CORRECTED}.",
            ),
            status: CommitmentStatus::Proposed,
            turn_id: 2,
        };

        let i0 = state.record(user_c.clone());
        let i1 = state.record(adam_c.clone());
        let i2 = state.record(user2_c.clone());
        assert_eq!((i0, i1, i2), (0, 1, 2));
        assert_eq!(state.commitments.len(), 3);

        // Most-recent by author honours insertion order.
        assert_eq!(state.last_by(Speaker::User), Some(&user2_c));
        assert_eq!(state.last_by(Speaker::Adam), Some(&adam_c));

        // by_author filters cleanly.
        let user_log: Vec<_> = state.by_author(Speaker::User).collect();
        assert_eq!(user_log, vec![&user_c, &user2_c]);
        let adam_log: Vec<_> = state.by_author(Speaker::Adam).collect();
        assert_eq!(adam_log, vec![&adam_c]);
    }

    /// `DiscourseState::default()` produces an empty log — the
    /// canonical starting point for a fresh `Conversation`.
    #[test]
    fn discourse_state_default_is_empty() {
        let state = DiscourseState::default();
        assert!(state.commitments.is_empty());
        assert!(state.by_author(Speaker::User).next().is_none());
        assert!(state.by_author(Speaker::Adam).next().is_none());
    }

    /// **Phase 2.D — correction-pattern detector.**  Kazakh
    /// «Жоқ, X емес, Y» shape extracts both values.  Uses the
    /// `TEST_FIRST_NAME_*` constants per the test-fixture
    /// convention.
    #[test]
    fn detect_correction_kazakh_comma_form() {
        let input = format!("Жоқ, {TEST_FIRST_NAME_USER} емес, {TEST_FIRST_NAME_USER_CORRECTED}.",);
        let got = detect_correction_pattern(&input);
        assert_eq!(
            got,
            Some((
                TEST_FIRST_NAME_USER.to_string(),
                TEST_FIRST_NAME_USER_CORRECTED.to_string(),
            )),
            "expected (rejected, replacement) tuple",
        );
    }

    /// Whisper-drift shape with no commas — same extraction.
    #[test]
    fn detect_correction_kazakh_no_commas() {
        let input = format!("жоқ {TEST_FIRST_NAME_USER} емес {TEST_FIRST_NAME_USER_CORRECTED}",);
        let got = detect_correction_pattern(&input);
        assert_eq!(
            got,
            Some((
                TEST_FIRST_NAME_USER.to_string(),
                TEST_FIRST_NAME_USER_CORRECTED.to_string(),
            )),
        );
    }

    /// Kazakh «Жоқ, X емес, Y дегенмін» trailing-tag — same
    /// extraction (the «дегенмін» suffix is trimmed).
    #[test]
    fn detect_correction_kazakh_with_dеgenmin_suffix() {
        let input = format!(
            "Жоқ, {TEST_FIRST_NAME_USER} емес, {TEST_FIRST_NAME_USER_CORRECTED} дегенмін.",
        );
        let got = detect_correction_pattern(&input);
        assert_eq!(
            got.as_ref().map(|t| t.1.as_str()),
            Some(TEST_FIRST_NAME_USER_CORRECTED),
            "replacement must strip the «дегенмін» tag",
        );
    }

    /// Russian «Нет, не X, а Y» shape.  The detector preserves
    /// the case of the extracted values so downstream consumers
    /// can do canonical name lookup against the curated DB.
    #[test]
    fn detect_correction_russian_full_form() {
        let got = detect_correction_pattern("Нет, не Иван, а Пётр.");
        assert_eq!(got, Some(("Иван".to_string(), "Пётр".to_string())));
    }

    /// Negative control: a non-correction input returns `None`.
    #[test]
    fn detect_correction_negative_controls() {
        assert!(detect_correction_pattern("").is_none());
        assert!(detect_correction_pattern("Сәлем!").is_none());
        assert!(detect_correction_pattern("Менің атым — Айгүл.").is_none());
        // «Жоқ» alone without the «емес» marker is just a no, not
        // a repair.
        assert!(detect_correction_pattern("Жоқ.").is_none());
        assert!(detect_correction_pattern("Жоқ, рахмет.").is_none());
    }

    /// **Phase 2.E.2 — referent stack** (semantics revised v6.8.7
    /// L4.8 / C.1: dedupe keeps INTRODUCTION turn_id).
    /// `push_referent` records the surface + kind + turn_id, and
    /// `last_referent_of_kind` returns the most-recent matching
    /// entry.  Re-pushing the same token deduplicates but
    /// preserves the original introduction `turn_id` — the
    /// cascade's «prior-turn referent» filter depends on
    /// introduction-turn semantics (see push_referent doc).
    #[test]
    fn referent_stack_push_and_query() {
        let mut state = DiscourseState::default();
        assert!(state.last_referent().is_none());

        state.push_referent("ахмет байтұрсынұлы".into(), ReferentKind::Person, 0);
        state.push_referent("қостанай".into(), ReferentKind::Place, 1);
        state.push_referent("абай".into(), ReferentKind::Person, 2);

        assert_eq!(state.referents.len(), 3);
        // Last-of-kind walks newest-first.
        assert_eq!(
            state
                .last_referent_of_kind(ReferentKind::Person)
                .map(|r| r.token.as_str()),
            Some("абай"),
        );
        assert_eq!(
            state
                .last_referent_of_kind(ReferentKind::Place)
                .map(|r| r.token.as_str()),
            Some("қостанай"),
        );
        assert!(
            state
                .last_referent_of_kind(ReferentKind::Organisation)
                .is_none()
        );

        // Re-pushing the same token deduplicates without creating
        // a fresh entry, AND preserves the original
        // introduction turn_id (was 2, not the new 3) — see
        // push_referent doc for the «prior-turn filter» reason.
        state.push_referent("абай".into(), ReferentKind::Person, 3);
        assert_eq!(state.referents.len(), 3);
        assert_eq!(state.referents.last().map(|r| r.turn_id), Some(2));
    }

    /// `push_referent` ignores empty / whitespace-only tokens.
    #[test]
    fn referent_stack_rejects_empty_token() {
        let mut state = DiscourseState::default();
        state.push_referent(String::new(), ReferentKind::Person, 0);
        state.push_referent("   ".into(), ReferentKind::Place, 1);
        assert!(state.referents.is_empty());
    }
}
