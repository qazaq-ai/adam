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
    /// detection. If a previous `Active` fact for
    /// `(subject, predicate)` exists with a different object, the
    /// old fact is flagged [`FactStatus::Contested`], the new fact
    /// is recorded as [`FactStatus::Contested`] too, and a
    /// [`BeliefConflict`] + [`PendingQuestion`] are pushed. When the
    /// new object matches the previous one, the old fact is
    /// refreshed (still `Active`) and no conflict is logged —
    /// repeated restatement isn't a disagreement.
    ///
    /// Returns the index of the newly-inserted fact in `self.facts`.
    pub fn record_user_fact(
        &mut self,
        subject: &str,
        predicate: &str,
        object: &str,
        turn_id: usize,
    ) -> usize {
        let prior_idx = self.facts.iter().enumerate().rposition(|(_, f)| {
            f.subject == subject && f.predicate == predicate && f.status == FactStatus::Active
        });

        let new_status = match prior_idx {
            Some(idx) if self.facts[idx].object == object => FactStatus::Active,
            Some(idx) => {
                // Repeated statement with a different value — flag
                // both as contested. The old fact stays in place so
                // provenance survives; the new one is stored next
                // so consumers can inspect both sides.
                self.facts[idx].status = FactStatus::Contested;
                FactStatus::Contested
            }
            None => FactStatus::Active,
        };

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

        if let Some(old_idx) = prior_idx {
            if self.facts[old_idx].object != self.facts[new_idx].object {
                self.contradictions.push(BeliefConflict {
                    subject: subject.to_string(),
                    predicate: predicate.to_string(),
                    fact_a_index: old_idx,
                    fact_b_index: new_idx,
                    detected_at_turn: turn_id,
                });
                self.pending_questions.push(PendingQuestion {
                    raised_at_turn: turn_id,
                    about: subject.to_string(),
                    nature: QuestionNature::ContradictionToResolve {
                        predicate: predicate.to_string(),
                        old_value: self.facts[old_idx].object.clone(),
                        new_value: self.facts[new_idx].object.clone(),
                    },
                });
            }
        }

        new_idx
    }

    /// Register or refresh an entity bucket. Creates on first
    /// sighting, otherwise updates `last_seen_turn` and appends a
    /// new alias when the surface form differs from the canonical
    /// root.
    pub fn touch_entity(&mut self, key: &str, kind: EntityKind, surface: &str, turn_id: usize) {
        let entry = self
            .entities
            .entry(key.to_string())
            .or_insert_with(|| EntityMemory {
                root: key.to_string(),
                kind,
                first_seen_turn: turn_id,
                last_seen_turn: turn_id,
                aliases: Vec::new(),
            });
        entry.last_seen_turn = turn_id;
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

    #[test]
    fn repeated_same_value_does_not_create_conflict() {
        let mut b = BeliefState::new();
        b.record_user_fact(USER_SELF_KEY, "city", "алматы", 0);
        b.record_user_fact(USER_SELF_KEY, "city", "алматы", 3);
        assert_eq!(b.facts.len(), 2);
        // Restatement stays Active on both sides.
        assert!(b.facts.iter().all(|f| f.status == FactStatus::Active));
        assert!(b.contradictions.is_empty());
        assert!(b.pending_questions.is_empty());
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
    fn touch_entity_tracks_first_and_last_seen() {
        let mut b = BeliefState::new();
        b.touch_entity(USER_SELF_KEY, EntityKind::User, "Дәулет", 0);
        b.touch_entity(USER_SELF_KEY, EntityKind::User, "Дәуі", 4);
        let e = b.entities.get(USER_SELF_KEY).unwrap();
        assert_eq!(e.first_seen_turn, 0);
        assert_eq!(e.last_seen_turn, 4);
        assert_eq!(e.aliases, vec!["Дәулет".to_string(), "Дәуі".to_string()]);
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
