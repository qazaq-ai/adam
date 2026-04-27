//! Layer 3.6 — structured belief state, introduced in v4.0.27.
//!
//! Pre-v4.0.27 the dialog kept user-supplied facts in a flat
//! `HashMap<String, String>` keyed by slot name (`name`, `age`, `city`,
//! `occupation`). Good enough for a chat REPL, but a real "cognitive"
//! layer needs more than current values:
//!
//! - **provenance** — was this claim from the user, retrieval, reasoning,
//!   or a curated pack?
//! - **confidence** — is it a direct statement, a derived inference, or
//!   a tentative hypothesis?
//! - **history** — when the user contradicts themselves, we must not
//!   silently overwrite; both claims should survive with a conflict
//!   marker so the system can ask for clarification.
//! - **pending questions** — what did the user ask that we haven't yet
//!   answered?
//!
//! Codex v4.0.26 roadmap "Phase 1" directly motivates this module. We
//! keep the existing `Conversation::session` map for backwards-compat
//! and template-slot rendering; the `BeliefState` lives alongside it and
//! captures the richer semantic picture the reasoner and verifier need.

use std::collections::BTreeMap;

/// Full structured belief state for a running conversation.
///
/// Owned by [`crate::Conversation`] and mutated in lock-step with the
/// pre-existing `session` slot map so no consumer is forced to migrate
/// in one step.
#[derive(Debug, Clone, Default)]
pub struct BeliefState {
    /// Canonical-form directory of every entity introduced this
    /// session. Key is the root spelling (`"Дәулет"`, `"алматы"`,
    /// `"мұғалім"`). The `User` entity, when present, is keyed by
    /// the sentinel `"__self__"` so it doesn't collide with any
    /// real name.
    pub entities: BTreeMap<String, EntityMemory>,
    /// Ordered log of belief facts, oldest first. Append-only during
    /// a session — later claims may flag earlier facts as
    /// [`FactStatus::Superseded`] or [`FactStatus::Contested`], but
    /// we never delete evidence. The reasoner / verifier can walk
    /// back to the earliest `Active` fact when a contradiction is
    /// detected.
    pub facts: Vec<BeliefFact>,
    /// Questions that were raised but not yet resolved — either the
    /// user asked something we lack data for, or a slot was requested
    /// but not supplied, or we detected a contradiction the system
    /// should surface to the user.
    pub pending_questions: Vec<PendingQuestion>,
    /// Pairs of belief facts that directly contradict each other.
    /// Kept explicit so the `verifier` (future phase) can refuse an
    /// answer that sits on top of a conflict.
    pub contradictions: Vec<BeliefConflict>,
}

/// Self-contained record of one thing the dialog believes. The tuple
/// `(subject, predicate, object)` mirrors the world-core fact schema;
/// `provenance` + `confidence` + `status` tell the verifier how far
/// to trust it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeliefFact {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: ConfidenceBand,
    pub provenance: Provenance,
    pub status: FactStatus,
    /// Turn index (0-based) when this fact entered the belief state.
    pub recorded_at_turn: usize,
}

/// Coarse confidence banding. Not a probability — we don't have
/// calibrated scores. These are categorical trust levels consumed by
/// the epistemic-status policy (Codex Phase 5, queued).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfidenceBand {
    /// User asserted it directly this session.
    Confirmed,
    /// The reasoner derived it from other active beliefs.
    Derived,
    /// Retrieved from the corpus index — supported by a sample quote.
    Retrieved,
    /// Inferred but not derived (future: one-shot guesses).
    Hypothesized,
    /// We don't know; recorded to mark a gap.
    Unknown,
}

/// Where a fact came from. The first two variants cover the v4.0.27
/// scope (direct user statements and corpus citations). The latter
/// two are plumbed for future phases so consumers don't have to
/// change shape when reasoning / curated sources start feeding the
/// belief state directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Provenance {
    /// Statement made by the user on turn N.
    UserStatement { turn_id: usize },
    /// A corpus sample the retrieval path cited.
    Retrieval { pack: String, sample_id: String },
    /// A rule fired by the reasoner. `derived_from` records the
    /// supporting fact indices into `BeliefState::facts`.
    Reasoning {
        rule_id: String,
        derived_from: Vec<usize>,
    },
    /// A curated world_core entry (hand-reviewed by `shaman`).
    Curated { pack: String, entry_id: String },
}

/// Lifecycle tag on a belief fact. A single predicate can only have
/// ONE `Active` fact per subject at a time — when the user updates a
/// slot, the old value is marked [`Superseded`](FactStatus::Superseded)
/// rather than removed, preserving history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactStatus {
    Active,
    Superseded,
    Contested,
}

/// Per-entity bucket. Tracks the canonical root plus every surface
/// form we've seen for it and which turns it appeared on. This is
/// the minimum needed to later support coreference ("I told you
/// about Дәулет before") without re-parsing history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityMemory {
    pub canonical_id: Option<String>,
    pub root: String,
    pub kind: EntityKind,
    pub first_seen_turn: usize,
    pub last_seen_turn: usize,
    pub aliases: Vec<String>,
}

/// Coarse ontology of the entities the dialog tracks. Expands as new
/// intent classes attach new entity types; v4.0.27 covers what the
/// existing slots emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    /// The human interlocutor themselves.
    User,
    /// A named person the user mentioned.
    Person,
    /// A place (city, region, country).
    Place,
    /// An occupation / profession.
    Occupation,
    /// A subject of inquiry — whatever came in as `noun_hint`.
    Topic,
    Other,
}

/// A question that's still open. Populated by contradiction
/// detection in v4.0.27; in later phases the goal layer + action
/// planner will also push [`QuestionNature::MissingSlot`] entries
/// when they decide we need more information before answering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingQuestion {
    pub raised_at_turn: usize,
    pub about: String,
    pub nature: QuestionNature,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestionNature {
    NeedsClarification,
    ContradictionToResolve {
        predicate: String,
        old_value: String,
        new_value: String,
    },
    MissingSlot {
        slot: String,
    },
}

/// Two facts the system holds about the same (subject, predicate)
/// that disagree. References carry `facts` array indices so the
/// verifier can look up provenance + render both sides to the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeliefConflict {
    pub subject: String,
    pub predicate: String,
    pub fact_a_index: usize,
    pub fact_b_index: usize,
    pub detected_at_turn: usize,
}

/// Sentinel key for the user entity. Real names never collide with
/// this because the double-underscore pattern is illegal in the
/// Kazakh orthography we accept.
pub const USER_SELF_KEY: &str = "__self__";

impl BeliefState {
    /// Fresh, empty belief state. Alias for `Default::default`
    /// exposed so callers don't need to import the trait.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a fact the **user** just stated, with contradiction
    /// detection, enforcing the **single-active-fact invariant**:
    /// at most one `Active` fact per `(subject, predicate)` at any
    /// time. Turn-by-turn behaviour:
    ///
    /// - **New fact** (no prior active entries): append as `Active`.
    /// - **Restatement of the same value**: mark every prior `Active`
    ///   copy as `Superseded`, append the new one as `Active`. No
    ///   conflict is logged — restatement isn't disagreement — but the
    ///   count of active facts remains exactly 1.
    /// - **Disagreement with any prior active value**: mark every
    ///   prior `Active` copy as `Contested`, append the new one as
    ///   `Contested`, log a `BeliefConflict` against the first
    ///   disagreeing prior fact, and push a
    ///   `PendingQuestion::ContradictionToResolve`. After this, there
    ///   are **zero** active facts for the `(subject, predicate)`.
    ///
    /// v4.0.28 — pre-v4.0.28 only flipped the most recent active fact
    /// (Codex v4.0.27 review #1). In the mixed sequence
    /// `value₁ → value₁ → value₂` the first copy was left `Active`
    /// even though a contradiction had been detected, so
    /// `active_fact()` could still return `Some(value₁)`. The fix
    /// flips **every** prior active fact, so the invariant holds
    /// regardless of restatement history.
    ///
    /// Returns the index of the newly-inserted fact in `self.facts`.
    pub fn record_user_fact(
        &mut self,
        subject: &str,
        predicate: &str,
        object: &str,
        turn_id: usize,
    ) -> usize {
        // Snapshot every currently-active fact for this
        // (subject, predicate). In steady state there is at most one
        // (by invariant), but legacy data or a future writer that
        // forgets the invariant could hold more.
        let prior_active_indices: Vec<usize> = self
            .facts
            .iter()
            .enumerate()
            .filter(|(_, f)| {
                f.subject == subject && f.predicate == predicate && f.status == FactStatus::Active
            })
            .map(|(i, _)| i)
            .collect();

        // First disagreeing prior fact, if any — drives conflict
        // logging. Returned by iteration order so the oldest
        // disagreement wins as `fact_a` (stable across runs).
        let disagreement_idx = prior_active_indices
            .iter()
            .copied()
            .find(|&i| self.facts[i].object != object);

        let (new_status, mark_prior_as) = if disagreement_idx.is_some() {
            (FactStatus::Contested, FactStatus::Contested)
        } else {
            (FactStatus::Active, FactStatus::Superseded)
        };

        // Flip every prior active fact in one sweep. This is the
        // v4.0.28 invariant-preserving step.
        for idx in &prior_active_indices {
            self.facts[*idx].status = mark_prior_as;
        }

        let fact = BeliefFact {
            subject: subject.to_string(),
            predicate: predicate.to_string(),
            object: object.to_string(),
            confidence: ConfidenceBand::Confirmed,
            provenance: Provenance::UserStatement { turn_id },
            status: new_status,
            recorded_at_turn: turn_id,
        };
        let new_idx = self.facts.len();
        self.facts.push(fact);

        if let Some(dis_idx) = disagreement_idx {
            self.contradictions.push(BeliefConflict {
                subject: subject.to_string(),
                predicate: predicate.to_string(),
                fact_a_index: dis_idx,
                fact_b_index: new_idx,
                detected_at_turn: turn_id,
            });
            self.pending_questions.push(PendingQuestion {
                raised_at_turn: turn_id,
                about: subject.to_string(),
                nature: QuestionNature::ContradictionToResolve {
                    predicate: predicate.to_string(),
                    old_value: self.facts[dis_idx].object.clone(),
                    new_value: self.facts[new_idx].object.clone(),
                },
            });
        }

        new_idx
    }

    /// Resolve a pending contradiction on `(subject, predicate)` by
    /// promoting `chosen_object` to `Active` and superseding every
    /// other recorded value for the same slot. Drops the matching
    /// `BeliefConflict` and `ContradictionToResolve` pending
    /// question.
    ///
    /// Returns `true` if a fact matching `chosen_object` existed and
    /// the resolution applied; `false` if no such fact exists (in
    /// which case the contradiction is left untouched — the caller
    /// is expected to record_user_fact and let the normal flow
    /// re-detect the conflict).
    ///
    /// **v4.0.41** — implements the kernel's signature feature:
    /// auditable belief revision via user choice. Closes
    /// aspirational scenario
    /// `aspirational_contradiction_resolution_via_user_choice`
    /// (v4.0.35 finding).
    pub fn resolve_contradiction(
        &mut self,
        subject: &str,
        predicate: &str,
        chosen_object: &str,
    ) -> bool {
        let candidates: Vec<usize> = self
            .facts
            .iter()
            .enumerate()
            .filter(|(_, f)| f.subject == subject && f.predicate == predicate)
            .map(|(i, _)| i)
            .collect();
        if candidates.is_empty() {
            return false;
        }
        let chosen_idx = candidates
            .iter()
            .find(|&&i| self.facts[i].object.eq_ignore_ascii_case(chosen_object))
            .copied();
        let Some(chosen_idx) = chosen_idx else {
            return false;
        };
        for &i in &candidates {
            self.facts[i].status = if i == chosen_idx {
                FactStatus::Active
            } else {
                FactStatus::Superseded
            };
        }
        self.contradictions
            .retain(|c| !(c.subject == subject && c.predicate == predicate));
        self.pending_questions.retain(|q| match &q.nature {
            QuestionNature::ContradictionToResolve { predicate: p, .. } => {
                !(q.about == subject && p == predicate)
            }
            _ => true,
        });
        true
    }

    /// **v4.4.0 — Dismiss a pending contradiction without choosing
    /// a value.**
    ///
    /// Symmetric counterpart to [`resolve_contradiction`]: when the
    /// user responds to a `CheckContradiction` prompt with "neither
    /// is right" / "I don't know" / "skip it" / "doesn't matter",
    /// the dialog should drop **all** contested facts on
    /// `(subject, predicate)` to `Superseded` rather than promote
    /// one of them to `Active`. The slot becomes empty in belief;
    /// future `Statement*` re-introduces a clean Active fact.
    ///
    /// Drops the matching `BeliefConflict` from `contradictions`
    /// and the matching `ContradictionToResolve` pending question
    /// (same teardown as `resolve_contradiction`). The single-
    /// active-fact invariant (v4.0.28) is preserved: zero Active
    /// facts on the slot after dismissal is a valid state.
    ///
    /// Returns `true` if at least one fact existed for the slot
    /// (so something was actually demoted); `false` if there was
    /// nothing to dismiss (no facts, no contradiction).
    ///
    /// Closes the v4.3.2 dialog-poisoning UX hazard: pre-v4.4.0,
    /// once any contradiction landed in belief — phantom or real —
    /// the planner blocked every other topic until the user
    /// resolved it. With dismissal, the user has a clean exit.
    pub fn dismiss_contradiction(&mut self, subject: &str, predicate: &str) -> bool {
        let candidates: Vec<usize> = self
            .facts
            .iter()
            .enumerate()
            .filter(|(_, f)| f.subject == subject && f.predicate == predicate)
            .map(|(i, _)| i)
            .collect();
        if candidates.is_empty() {
            return false;
        }
        for &i in &candidates {
            self.facts[i].status = FactStatus::Superseded;
        }
        self.contradictions
            .retain(|c| !(c.subject == subject && c.predicate == predicate));
        self.pending_questions.retain(|q| match &q.nature {
            QuestionNature::ContradictionToResolve { predicate: p, .. } => {
                !(q.about == subject && p == predicate)
            }
            _ => true,
        });
        true
    }

    /// Register or refresh an entity bucket. Creates on first
    /// sighting, otherwise updates `last_seen_turn` and appends a
    /// new alias when the surface form differs from the canonical
    /// root.
    pub fn touch_entity(
        &mut self,
        key: &str,
        kind: EntityKind,
        root: &str,
        surface: &str,
        canonical_id: Option<&str>,
        turn_id: usize,
    ) {
        let entry = self
            .entities
            .entry(key.to_string())
            .or_insert_with(|| EntityMemory {
                canonical_id: canonical_id.map(|id| id.to_string()),
                root: root.to_string(),
                kind,
                first_seen_turn: turn_id,
                last_seen_turn: turn_id,
                aliases: Vec::new(),
            });
        entry.last_seen_turn = turn_id;
        if entry.canonical_id.is_none() {
            entry.canonical_id = canonical_id.map(|id| id.to_string());
        }
        if surface != entry.root && !entry.aliases.iter().any(|a| a == surface) {
            entry.aliases.push(surface.to_string());
        }
    }

    /// Look up the current `Active` fact for a given `(subject,
    /// predicate)` pair. Returns `None` when nothing is recorded or
    /// every matching fact is `Superseded`/`Contested`.
    pub fn active_fact(&self, subject: &str, predicate: &str) -> Option<&BeliefFact> {
        self.facts.iter().rev().find(|f| {
            f.subject == subject && f.predicate == predicate && f.status == FactStatus::Active
        })
    }

    /// All facts ever recorded for `subject`, oldest first. Useful
    /// for a trace / debugger view.
    pub fn facts_about(&self, subject: &str) -> Vec<&BeliefFact> {
        self.facts.iter().filter(|f| f.subject == subject).collect()
    }

    /// Compact summary for trace output — counts per field without
    /// dumping every fact. Consumers who want the full picture can
    /// just format `self` directly (it's `Debug`).
    pub fn digest(&self) -> BeliefDigest {
        BeliefDigest {
            entities: self.entities.len(),
            facts_total: self.facts.len(),
            facts_active: self
                .facts
                .iter()
                .filter(|f| f.status == FactStatus::Active)
                .count(),
            facts_contested: self
                .facts
                .iter()
                .filter(|f| f.status == FactStatus::Contested)
                .count(),
            pending_questions: self.pending_questions.len(),
            contradictions: self.contradictions.len(),
        }
    }
}

/// Small summary struct used by `adam_chat --trace`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BeliefDigest {
    pub entities: usize,
    pub facts_total: usize,
    pub facts_active: usize,
    pub facts_contested: usize,
    pub pending_questions: usize,
    pub contradictions: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_user_fact_creates_active_fact() {
        let mut b = BeliefState::new();
        let idx = b.record_user_fact(USER_SELF_KEY, "name", "Дәулет", 0);
        assert_eq!(idx, 0);
        assert_eq!(b.facts.len(), 1);
        assert_eq!(b.facts[0].status, FactStatus::Active);
        assert_eq!(b.facts[0].confidence, ConfidenceBand::Confirmed);
        assert!(matches!(
            b.facts[0].provenance,
            Provenance::UserStatement { turn_id: 0 }
        ));
        assert!(b.contradictions.is_empty());
        assert!(b.pending_questions.is_empty());
    }

    /// v4.0.28 — restatement of the same value must not create a
    /// conflict, but the single-active-fact invariant means the
    /// earlier copy is marked `Superseded` rather than kept `Active`.
    #[test]
    fn repeated_same_value_preserves_single_active_invariant() {
        let mut b = BeliefState::new();
        b.record_user_fact(USER_SELF_KEY, "city", "алматы", 0);
        b.record_user_fact(USER_SELF_KEY, "city", "алматы", 3);
        assert_eq!(b.facts.len(), 2);
        assert_eq!(b.facts[0].status, FactStatus::Superseded);
        assert_eq!(b.facts[1].status, FactStatus::Active);
        assert!(b.contradictions.is_empty());
        assert!(b.pending_questions.is_empty());
        assert_eq!(
            b.facts
                .iter()
                .filter(|f| f.status == FactStatus::Active)
                .count(),
            1
        );
    }

    /// v4.0.28 — Codex v4.0.27 review #1 regression. Mixed sequence
    /// `value → same value → different value` must end with ZERO
    /// active facts for the `(subject, predicate)` pair. Pre-v4.0.28
    /// the first copy was left `Active` because `record_user_fact`
    /// only contested the most recent active entry — downstream
    /// `active_fact()` could return a stale winner even after a
    /// contradiction was logged, which would break Phase 2+ (Task,
    /// Action, Verifier) that trust `active_fact()` as authoritative.
    #[test]
    fn same_same_different_leaves_no_active_fact() {
        let mut b = BeliefState::new();
        b.record_user_fact(USER_SELF_KEY, "city", "алматы", 0);
        b.record_user_fact(USER_SELF_KEY, "city", "алматы", 1);
        b.record_user_fact(USER_SELF_KEY, "city", "астана", 2);

        assert_eq!(b.facts.len(), 3);
        assert!(
            b.active_fact(USER_SELF_KEY, "city").is_none(),
            "after contradiction, active_fact() must be None — got {:?}",
            b.active_fact(USER_SELF_KEY, "city")
        );
        let active_count = b
            .facts
            .iter()
            .filter(|f| f.status == FactStatus::Active)
            .count();
        assert_eq!(
            active_count,
            0,
            "active count must be 0, got {active_count}; statuses={:?}",
            b.facts.iter().map(|f| f.status).collect::<Vec<_>>()
        );
        assert_eq!(b.contradictions.len(), 1);
        assert_eq!(b.pending_questions.len(), 1);
    }

    #[test]
    fn contradictory_statements_flag_both_facts_and_log_conflict() {
        let mut b = BeliefState::new();
        b.record_user_fact(USER_SELF_KEY, "city", "алматы", 0);
        b.record_user_fact(USER_SELF_KEY, "city", "астана", 2);
        assert_eq!(b.facts.len(), 2);
        assert!(b.facts.iter().all(|f| f.status == FactStatus::Contested));
        assert_eq!(b.contradictions.len(), 1);
        let c = &b.contradictions[0];
        assert_eq!(c.subject, USER_SELF_KEY);
        assert_eq!(c.predicate, "city");
        assert_eq!(c.detected_at_turn, 2);
        assert_eq!(b.pending_questions.len(), 1);
        match &b.pending_questions[0].nature {
            QuestionNature::ContradictionToResolve {
                predicate,
                old_value,
                new_value,
            } => {
                assert_eq!(predicate, "city");
                assert_eq!(old_value, "алматы");
                assert_eq!(new_value, "астана");
            }
            other => panic!("expected ContradictionToResolve, got {other:?}"),
        }
    }

    #[test]
    fn resolve_contradiction_picks_chosen_and_supersedes_others() {
        let mut b = BeliefState::new();
        b.record_user_fact(USER_SELF_KEY, "city", "алматы", 0);
        b.record_user_fact(USER_SELF_KEY, "city", "астана", 2);
        assert_eq!(b.contradictions.len(), 1);
        assert_eq!(b.pending_questions.len(), 1);

        let resolved = b.resolve_contradiction(USER_SELF_KEY, "city", "астана");
        assert!(resolved);

        // Astana now Active, Almaty Superseded
        let astana = b
            .facts
            .iter()
            .find(|f| f.object == "астана")
            .expect("astana fact present");
        let almaty = b
            .facts
            .iter()
            .find(|f| f.object == "алматы")
            .expect("almaty fact present");
        assert_eq!(astana.status, FactStatus::Active);
        assert_eq!(almaty.status, FactStatus::Superseded);

        // Conflict and ContradictionToResolve question both gone
        assert!(b.contradictions.is_empty());
        assert!(b.pending_questions.is_empty());

        // active_fact() now returns the chosen value (single-active invariant)
        assert_eq!(
            b.active_fact(USER_SELF_KEY, "city")
                .map(|f| f.object.as_str()),
            Some("астана")
        );
    }

    #[test]
    fn resolve_contradiction_returns_false_when_chosen_value_unknown() {
        let mut b = BeliefState::new();
        b.record_user_fact(USER_SELF_KEY, "city", "алматы", 0);
        b.record_user_fact(USER_SELF_KEY, "city", "астана", 2);

        let resolved = b.resolve_contradiction(USER_SELF_KEY, "city", "шымкент");
        assert!(!resolved);
        // State untouched
        assert_eq!(b.contradictions.len(), 1);
        assert!(b.facts.iter().all(|f| f.status == FactStatus::Contested));
    }

    /// **v4.4.0** — `dismiss_contradiction` drops all contested
    /// facts to `Superseded` and clears the matching conflict +
    /// pending question. The slot ends up with zero `Active` facts
    /// — a valid state under the single-active-fact invariant.
    #[test]
    fn dismiss_contradiction_supersedes_all_contested_facts() {
        let mut b = BeliefState::new();
        b.record_user_fact(USER_SELF_KEY, "city", "алматы", 0);
        b.record_user_fact(USER_SELF_KEY, "city", "астана", 2);
        assert_eq!(b.contradictions.len(), 1);
        assert_eq!(b.pending_questions.len(), 1);

        let dismissed = b.dismiss_contradiction(USER_SELF_KEY, "city");
        assert!(dismissed);

        // Both city facts now Superseded; no Active fact for `city`.
        let city_facts: Vec<_> = b
            .facts
            .iter()
            .filter(|f| f.subject == USER_SELF_KEY && f.predicate == "city")
            .collect();
        assert_eq!(city_facts.len(), 2);
        assert!(
            city_facts
                .iter()
                .all(|f| f.status == FactStatus::Superseded),
            "all city facts must be demoted to Superseded after dismissal"
        );
        assert!(
            b.active_fact(USER_SELF_KEY, "city").is_none(),
            "no Active city fact may survive a dismissal"
        );

        // Conflict and pending question both cleared.
        assert!(b.contradictions.is_empty());
        assert!(b.pending_questions.is_empty());
    }

    /// **v4.4.0** — `dismiss_contradiction` returns `false` for an
    /// empty slot. Prevents callers from confusing "nothing to
    /// dismiss" with "successfully dismissed".
    #[test]
    fn dismiss_contradiction_returns_false_when_no_facts_recorded() {
        let mut b = BeliefState::new();
        let dismissed = b.dismiss_contradiction(USER_SELF_KEY, "city");
        assert!(!dismissed);
        assert!(b.facts.is_empty());
        assert!(b.contradictions.is_empty());
    }

    /// **v4.4.0** — Dismissal is idempotent: calling it on an
    /// already-empty slot (e.g. after a previous dismissal) is a
    /// no-op that returns `false`. Future `Statement*` on the slot
    /// re-creates a clean Active fact, so dismissal does NOT lock
    /// the slot.
    #[test]
    fn dismiss_contradiction_does_not_lock_slot_for_future_statements() {
        let mut b = BeliefState::new();
        b.record_user_fact(USER_SELF_KEY, "city", "алматы", 0);
        b.record_user_fact(USER_SELF_KEY, "city", "астана", 1);
        b.dismiss_contradiction(USER_SELF_KEY, "city");

        // After dismissal: no Active. New Statement on turn 4 must
        // create a fresh Active fact (no contradiction, since both
        // priors were already Superseded).
        b.record_user_fact(USER_SELF_KEY, "city", "шымкент", 4);
        assert_eq!(
            b.active_fact(USER_SELF_KEY, "city")
                .map(|f| f.object.as_str()),
            Some("шымкент"),
            "post-dismissal Statement creates a fresh Active fact"
        );
        assert_eq!(
            b.contradictions.len(),
            0,
            "no new contradiction is logged because both priors were Superseded"
        );
    }

    #[test]
    fn touch_entity_tracks_first_and_last_seen() {
        let mut b = BeliefState::new();
        b.touch_entity(
            USER_SELF_KEY,
            EntityKind::User,
            "__self__",
            "Дәулет",
            None,
            0,
        );
        b.touch_entity(USER_SELF_KEY, EntityKind::User, "__self__", "Дәуі", None, 4);
        let e = b.entities.get(USER_SELF_KEY).unwrap();
        assert_eq!(e.canonical_id, None);
        assert_eq!(e.first_seen_turn, 0);
        assert_eq!(e.last_seen_turn, 4);
        assert_eq!(e.aliases, vec!["Дәулет".to_string(), "Дәуі".to_string()]);
    }

    #[test]
    fn touch_entity_preserves_canonical_id_for_places() {
        let mut b = BeliefState::new();
        b.touch_entity(
            "geo_kz_004",
            EntityKind::Place,
            "Алматы",
            "Алматы",
            Some("geo_kz_004"),
            0,
        );
        let e = b.entities.get("geo_kz_004").expect("place entity");
        assert_eq!(e.canonical_id.as_deref(), Some("geo_kz_004"));
        assert_eq!(e.root, "Алматы");
    }

    /// **v4.3.1** — symmetric to the place test above. The user's
    /// `EntityMemory` is keyed by `USER_SELF_KEY` (sentinel), but
    /// `canonical_id` carries the `person:<canonical>` resolved form
    /// so the planner can branch on canonical identity instead of
    /// the (possibly drifted) surface alias.
    #[test]
    fn touch_entity_preserves_canonical_person_id() {
        let mut b = BeliefState::new();
        b.touch_entity(
            USER_SELF_KEY,
            EntityKind::User,
            USER_SELF_KEY,
            "Дәулет",
            Some("person:Дәулет"),
            0,
        );
        let e = b.entities.get(USER_SELF_KEY).expect("user entity");
        assert_eq!(e.canonical_id.as_deref(), Some("person:Дәулет"));
        assert_eq!(e.root, USER_SELF_KEY);
    }

    #[test]
    fn active_fact_returns_latest_non_superseded() {
        let mut b = BeliefState::new();
        b.record_user_fact(USER_SELF_KEY, "occupation", "мұғалім", 0);
        assert_eq!(
            b.active_fact(USER_SELF_KEY, "occupation").unwrap().object,
            "мұғалім"
        );
        // Contradiction — the getter should return None because
        // both entries are Contested, not Active.
        b.record_user_fact(USER_SELF_KEY, "occupation", "дәрігер", 2);
        assert!(b.active_fact(USER_SELF_KEY, "occupation").is_none());
    }

    #[test]
    fn digest_counts_facts_by_status() {
        let mut b = BeliefState::new();
        b.record_user_fact(USER_SELF_KEY, "name", "Дәулет", 0);
        b.record_user_fact(USER_SELF_KEY, "city", "алматы", 1);
        b.record_user_fact(USER_SELF_KEY, "city", "астана", 2); // conflict
        let d = b.digest();
        assert_eq!(d.facts_total, 3);
        assert_eq!(d.facts_active, 1); // only name
        assert_eq!(d.facts_contested, 2); // both city entries
        assert_eq!(d.contradictions, 1);
        assert_eq!(d.pending_questions, 1);
    }
}
