//! Layer 3.7 — goal + task state, introduced in v4.0.29.
//!
//! Codex v4.0.26 roadmap "Phase 2". Pre-v4.0.29 the dialog could
//! classify an utterance as `Unknown { noun_hint }` but had no
//! representation of *what the user is trying to accomplish*. A query
//! ended at template realisation; there was no continuity across turns
//! beyond the active-intent marker.
//!
//! `TaskState` adds that missing layer: a coarse goal, subgoals, a
//! status, and the last action the planner chose. It reads from
//! [`crate::belief::BeliefState`] (treating `active_fact() == None`
//! after a contradiction as a **legitimate `Blocked`** state rather
//! than an error — per Codex v4.0.28 guidance). Task-level decisions
//! live here; the [`Conversation`](crate::Conversation) just holds the
//! state and lets downstream consumers (future `ActionPlanner`,
//! `Verifier`) read it.
//!
//! v4.0.29 scope — **skeleton + coarse goal detection**. Goal
//! inference uses only intent kind + noun_hint. Later phases will
//! refine with belief-aware heuristics (e.g. "ask about the slot we
//! lack", "resolve the contradiction we logged").

use crate::belief::BeliefState;
use crate::intent::Intent;

/// Coarse taxonomy of what the user is trying to accomplish across
/// one or more turns. Intentionally narrow — we only classify what
/// the current recogniser + intent enum can distinguish. Later phases
/// may add `CompareEntities`, `DisambiguateEntity`, etc.
///
/// Every variant carries enough payload to identify whether a later
/// turn is **continuing** the same goal or **switching** to a new
/// one. Equality is structural, not by-variant, so `goal_a == goal_b`
/// means "same topic" for the router.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Goal {
    /// User wants information about `topic` (e.g. «жер туралы айтшы»
    /// → `LearnAboutTopic { topic: "жер" }`). Populated from
    /// `Intent::Unknown` with a `noun_hint`.
    LearnAboutTopic { topic: String },
    /// User asked "who/what is X" or similar — we need to identify
    /// the referent. v4.0.29 doesn't populate this yet; reserved for
    /// later recognisers.
    IdentifyEntity { entity: String },
    /// User asked the system to compare two entities. Reserved —
    /// no intent currently produces this.
    CompareEntities { left: String, right: String },
    /// Social / profile back-and-forth (name, age, location,
    /// occupation) — asking or telling. Chains across several turns
    /// as the user fills in their profile.
    ClarifyUserProfile,
    /// Carry-over marker: the user's current turn doesn't introduce
    /// a new goal and there's still something open from an earlier
    /// turn we should finish.
    ContinueOpenQuestion,
}

/// Sub-step of an active goal. v4.0.29 keeps this as a simple string
/// bucket so downstream phases can append without a schema change.
/// When Phase 3 (action planner) lands, this will grow a structured
/// enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subgoal {
    pub description: String,
    pub completed: bool,
}

/// Where the task is in its lifecycle. Ordered roughly by "progress":
/// a fresh user query typically goes `Idle → GatheringEvidence →
/// ReadyToAnswer`, and the presence of an unresolved belief conflict
/// bumps it to `Blocked`. `WaitingForUser` is for the system-asks-a-
/// clarifying-question branch Phase 3 will add.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskStatus {
    /// No active task. Default between sessions or after a social
    /// intent with no carry-over.
    #[default]
    Idle,
    /// A goal is set but we haven't yet collected the supporting
    /// evidence (retrieval / reasoning / curated fact lookup).
    GatheringEvidence,
    /// Evidence is in hand; the realiser can compose the answer.
    ReadyToAnswer,
    /// The system asked the user a question and is waiting for
    /// their reply before progressing.
    WaitingForUser,
    /// A belief conflict or missing precondition prevents answering.
    /// Codex v4.0.28 directive: `active_fact() == None` after a
    /// contradiction is a legitimate `Blocked`, not an error.
    Blocked,
}

/// Full task state for a running conversation. Held on the
/// [`Conversation`](crate::Conversation) alongside the belief state;
/// the turn loop calls [`TaskState::roll_forward`] after entity
/// absorption so the status reflects the newest intent + belief.
#[derive(Debug, Clone, Default)]
pub struct TaskState {
    /// The goal currently driving the conversation. `None` means
    /// nothing is in flight (small-talk turns, fresh session, etc.).
    pub active_goal: Option<Goal>,
    /// Ordered sub-steps toward the active goal. Most v4.0.29 goals
    /// have none; reserved for Phase 3.
    pub subgoals: Vec<Subgoal>,
    /// v4.0.31 — last `ActionPlan` chosen by
    /// [`crate::action::ActionPlanner::plan`] for the most recent
    /// turn. Pre-v4.0.31 this was a placeholder `Option<String>`;
    /// Phase 3 lands the real type. `None` before the first turn of
    /// a session or after [`Conversation::reset`].
    pub last_action: Option<crate::action::ActionPlan>,
    /// Lifecycle position — see [`TaskStatus`].
    pub status: TaskStatus,
    /// 0-based turn index when `active_goal` was installed. Drives
    /// the "has this goal been around for a while?" signal the
    /// verifier will consume later.
    pub goal_set_at_turn: Option<usize>,
}

impl TaskState {
    /// Fresh task state. Equivalent to `Default::default` but
    /// exposed so callers don't need to import the trait.
    pub fn new() -> Self {
        Self::default()
    }

    /// Derive a coarse [`Goal`] from the newest intent, independent
    /// of belief history. The **carry-over logic** that decides
    /// whether to keep the previous goal or switch lives in
    /// [`roll_forward`](Self::roll_forward).
    pub fn detect_goal(intent: &Intent) -> Option<Goal> {
        match intent {
            Intent::Unknown {
                noun_hint: Some(topic),
                ..
            } => Some(Goal::LearnAboutTopic {
                topic: topic.clone(),
            }),
            Intent::AskName
            | Intent::AskAge
            | Intent::AskLocation
            | Intent::AskOccupation
            | Intent::AskFamily
            | Intent::StatementOfName { .. }
            | Intent::StatementOfAge { .. }
            | Intent::StatementOfLocation { .. }
            | Intent::StatementOfOccupation { .. }
            | Intent::StatementOfFamily => Some(Goal::ClarifyUserProfile),
            _ => None,
        }
    }

    /// Advance the task state for a new turn. Called after
    /// `absorb_entities` so `belief` reflects the just-seen intent.
    ///
    /// Decisions:
    /// 1. Compute a candidate goal from the intent.
    /// 2. If the intent yields no goal, keep `active_goal` (social
    ///    turns don't erase state) but mark
    ///    [`ContinueOpenQuestion`](Goal::ContinueOpenQuestion) when
    ///    something is still unresolved.
    /// 3. Goal change? Install the new one, reset `subgoals`, record
    ///    `goal_set_at_turn = turn_id`.
    /// 4. Status derives from (belief, intent):
    ///    - Any unresolved contradiction → `Blocked`.
    ///    - Any pending clarification question → `WaitingForUser`.
    ///    - Goal active AND the injected intent carries evidence
    ///      (v4.0.30+: `reasoning_chain.is_some()` or
    ///      `example.is_some()` or `grounded_fact.is_some()`) →
    ///      `ReadyToAnswer`.
    ///    - Goal active, no evidence yet → `GatheringEvidence`.
    ///    - No goal → `Idle`.
    ///
    /// Codex v4.0.28 invariant: `belief.active_fact() == None` after
    /// a contradiction is a **legitimate** state, not an error.
    /// `Blocked` represents that explicitly.
    ///
    /// v4.0.30 — Codex v4.0.29 review #2: pre-v4.0.30 `compute_status`
    /// never returned `ReadyToAnswer`; tests masked that by
    /// accepting either variant. Retrieval + reasoning injection run
    /// BEFORE `roll_forward` in `Conversation::turn_with_trace`, so
    /// the intent already carries any evidence by the time we reach
    /// here; checking it surfaces the status the planner is supposed
    /// to consume.
    pub fn roll_forward(&mut self, intent: &Intent, belief: &BeliefState, turn_id: usize) {
        let candidate = Self::detect_goal(intent);

        match candidate {
            Some(new_goal) => {
                let switching = self
                    .active_goal
                    .as_ref()
                    .map(|g| *g != new_goal)
                    .unwrap_or(true);
                if switching {
                    self.active_goal = Some(new_goal);
                    self.subgoals.clear();
                    self.goal_set_at_turn = Some(turn_id);
                }
                // Same goal → no-op on the structural fields; status
                // recomputes below.
            }
            None => {
                // Social turn, greeting, acknowledgement, etc.
                // Keep whatever was active. If belief has unresolved
                // state and active_goal was None, synthesise
                // ContinueOpenQuestion so the planner can pick up.
                if self.active_goal.is_none()
                    && (!belief.pending_questions.is_empty() || !belief.contradictions.is_empty())
                {
                    self.active_goal = Some(Goal::ContinueOpenQuestion);
                    self.goal_set_at_turn = Some(turn_id);
                }
            }
        }

        let has_evidence = Self::intent_has_evidence(intent);
        self.status = self.compute_status(belief, has_evidence);
    }

    /// v4.0.30 — does the given intent already carry injected
    /// evidence? Used by `compute_status` to distinguish
    /// `ReadyToAnswer` (evidence in hand) from `GatheringEvidence`
    /// (goal set but nothing to cite yet).
    ///
    /// Evidence sources for the v4.0.x dialog pipeline:
    /// - `grounded_fact` populated by curated `SearchGraph`
    /// - `reasoning_chain` populated by `inject_reasoning_chain`
    /// - `example` populated by `inject_retrieval_example`
    ///
    /// Only `Intent::Unknown` carries these slots; other intents
    /// go through their own canned-template paths.
    pub fn intent_has_evidence(intent: &Intent) -> bool {
        matches!(
            intent,
            Intent::Unknown {
                grounded_fact: Some(_),
                ..
            } | Intent::Unknown {
                reasoning_chain: Some(_),
                ..
            } | Intent::Unknown {
                example: Some(_),
                ..
            }
        )
    }

    /// Pure-function status derivation, separated so tests can
    /// exercise it without going through `roll_forward`. v4.0.30
    /// gained `has_evidence` so `ReadyToAnswer` is actually
    /// reachable.
    fn compute_status(&self, belief: &BeliefState, has_evidence: bool) -> TaskStatus {
        if !belief.contradictions.is_empty() {
            return TaskStatus::Blocked;
        }
        // A pending question that's NOT a contradiction (e.g.
        // MissingSlot) indicates we asked the user something and
        // are waiting.
        if !belief.pending_questions.is_empty() {
            return TaskStatus::WaitingForUser;
        }
        match self.active_goal {
            Some(_) if has_evidence => TaskStatus::ReadyToAnswer,
            Some(_) => TaskStatus::GatheringEvidence,
            None => TaskStatus::Idle,
        }
    }

    /// Compact summary for trace output — mirrors
    /// [`crate::belief::BeliefDigest`] in spirit.
    pub fn digest(&self) -> TaskDigest {
        TaskDigest {
            has_goal: self.active_goal.is_some(),
            goal_variant: self.active_goal.as_ref().map(|g| goal_variant_name(g)),
            subgoals: self.subgoals.len(),
            status: self.status,
            goal_age_turns: self.goal_set_at_turn,
        }
    }
}

/// Cheap trace payload. Six scalars + a lifted variant tag —
/// enough to render a single `├─ task: ...` line in
/// `adam_chat --trace` without dumping the whole state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskDigest {
    pub has_goal: bool,
    pub goal_variant: Option<&'static str>,
    pub subgoals: usize,
    pub status: TaskStatus,
    pub goal_age_turns: Option<usize>,
}

fn goal_variant_name(g: &Goal) -> &'static str {
    match g {
        Goal::LearnAboutTopic { .. } => "LearnAboutTopic",
        Goal::IdentifyEntity { .. } => "IdentifyEntity",
        Goal::CompareEntities { .. } => "CompareEntities",
        Goal::ClarifyUserProfile => "ClarifyUserProfile",
        Goal::ContinueOpenQuestion => "ContinueOpenQuestion",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::belief::{BeliefState, USER_SELF_KEY};
    use crate::intent::Intent;

    fn unknown_with_noun(topic: &str) -> Intent {
        Intent::Unknown {
            raw_tokens: vec![topic.into()],
            noun_hint: Some(topic.into()),
            example: None,
            grounded_fact: None,
            example_adapted: false,
            reasoning_chain: None,
            question_shape: None,
            temporal_scope: false,
            compositional_function: false,
            noun_hint_polarity: adam_kernel_fst::Polarity::Affirmative,
        }
    }

    #[test]
    fn detect_goal_maps_unknown_topic_to_learn() {
        let g = TaskState::detect_goal(&unknown_with_noun("жер")).unwrap();
        assert_eq!(
            g,
            Goal::LearnAboutTopic {
                topic: "жер".into()
            }
        );
    }

    #[test]
    fn detect_goal_maps_profile_intents_to_clarify_user_profile() {
        for intent in [
            Intent::AskName,
            Intent::AskAge,
            Intent::AskLocation,
            Intent::AskOccupation,
            Intent::AskFamily,
            Intent::StatementOfName {
                name: "Дәулет".into(),
            },
            Intent::StatementOfAge { years: Some(30) },
            Intent::StatementOfLocation {
                city: Some("алматы".into()),
            },
            Intent::StatementOfOccupation {
                occupation: Some("мұғалім".into()),
            },
            Intent::StatementOfFamily,
        ] {
            assert_eq!(
                TaskState::detect_goal(&intent),
                Some(Goal::ClarifyUserProfile),
                "profile intent {intent:?} must map to ClarifyUserProfile"
            );
        }
    }

    #[test]
    fn detect_goal_returns_none_for_social_and_unknown_without_topic() {
        let social = [
            Intent::Greeting {
                kind: crate::intent::GreetingKind::Casual,
            },
            Intent::Farewell,
            Intent::Affirmation,
            Intent::Negation,
            Intent::Thanks,
            Intent::Unknown {
                raw_tokens: vec!["ммм".into()],
                noun_hint: None,
                example: None,
                grounded_fact: None,
                example_adapted: false,
                reasoning_chain: None,
                question_shape: None,
                temporal_scope: false,
                compositional_function: false,
                noun_hint_polarity: adam_kernel_fst::Polarity::Affirmative,
            },
        ];
        for intent in social {
            assert_eq!(
                TaskState::detect_goal(&intent),
                None,
                "social intent {intent:?} should not derive a goal"
            );
        }
    }

    #[test]
    fn roll_forward_installs_goal_on_first_unknown_topic() {
        let mut t = TaskState::new();
        let belief = BeliefState::new();
        t.roll_forward(&unknown_with_noun("жер"), &belief, 0);
        assert_eq!(
            t.active_goal,
            Some(Goal::LearnAboutTopic {
                topic: "жер".into()
            })
        );
        assert_eq!(t.goal_set_at_turn, Some(0));
        assert_eq!(t.status, TaskStatus::GatheringEvidence);
    }

    #[test]
    fn roll_forward_keeps_goal_across_same_topic() {
        let mut t = TaskState::new();
        let belief = BeliefState::new();
        t.roll_forward(&unknown_with_noun("жер"), &belief, 0);
        let first_set = t.goal_set_at_turn;
        t.roll_forward(&unknown_with_noun("жер"), &belief, 1);
        assert_eq!(t.goal_set_at_turn, first_set, "goal age must not reset");
        assert_eq!(
            t.active_goal,
            Some(Goal::LearnAboutTopic {
                topic: "жер".into()
            })
        );
    }

    #[test]
    fn roll_forward_switches_goal_on_topic_change() {
        let mut t = TaskState::new();
        let belief = BeliefState::new();
        t.roll_forward(&unknown_with_noun("жер"), &belief, 0);
        t.roll_forward(&unknown_with_noun("күн"), &belief, 2);
        assert_eq!(
            t.active_goal,
            Some(Goal::LearnAboutTopic {
                topic: "күн".into()
            })
        );
        assert_eq!(t.goal_set_at_turn, Some(2));
        assert_eq!(t.subgoals.len(), 0);
    }

    #[test]
    fn roll_forward_preserves_goal_on_social_turn() {
        let mut t = TaskState::new();
        let belief = BeliefState::new();
        t.roll_forward(&unknown_with_noun("жер"), &belief, 0);
        t.roll_forward(&Intent::Thanks, &belief, 1);
        // Thanks is social → no new goal → keep жер.
        assert_eq!(
            t.active_goal,
            Some(Goal::LearnAboutTopic {
                topic: "жер".into()
            })
        );
    }

    #[test]
    fn roll_forward_marks_blocked_on_belief_contradiction() {
        let mut t = TaskState::new();
        let mut belief = BeliefState::new();
        belief.record_user_fact(USER_SELF_KEY, "city", "алматы", 0);
        belief.record_user_fact(USER_SELF_KEY, "city", "астана", 1);
        assert!(!belief.contradictions.is_empty());

        t.roll_forward(
            &Intent::StatementOfLocation {
                city: Some("астана".into()),
            },
            &belief,
            1,
        );
        assert_eq!(t.status, TaskStatus::Blocked);
        // Codex v4.0.28 invariant check: active_fact() is None and
        // that's expected state — the task status reflects it.
        assert!(belief.active_fact(USER_SELF_KEY, "city").is_none());
    }

    #[test]
    fn roll_forward_synthesises_continue_open_question_when_belief_has_pending() {
        let mut t = TaskState::new();
        let mut belief = BeliefState::new();
        belief.record_user_fact(USER_SELF_KEY, "city", "алматы", 0);
        belief.record_user_fact(USER_SELF_KEY, "city", "астана", 1);
        // Fresh task, social intent — active_goal was None, but
        // belief has unresolved state → task synthesises
        // ContinueOpenQuestion so downstream phases know to circle
        // back.
        t.roll_forward(&Intent::Affirmation, &belief, 2);
        assert_eq!(t.active_goal, Some(Goal::ContinueOpenQuestion));
        assert_eq!(t.status, TaskStatus::Blocked);
    }

    #[test]
    fn digest_captures_variant_tag_and_status() {
        let mut t = TaskState::new();
        let belief = BeliefState::new();
        t.roll_forward(&unknown_with_noun("жер"), &belief, 0);
        let d = t.digest();
        assert!(d.has_goal);
        assert_eq!(d.goal_variant, Some("LearnAboutTopic"));
        assert_eq!(d.status, TaskStatus::GatheringEvidence);
        assert_eq!(d.goal_age_turns, Some(0));
    }

    /// v4.0.30 — `intent_has_evidence` returns true iff the Unknown
    /// intent has `reasoning_chain` OR `example` populated by the
    /// retrieval / reasoning injection passes.
    #[test]
    fn intent_has_evidence_detects_injected_slots() {
        let bare = unknown_with_noun("жер");
        assert!(!TaskState::intent_has_evidence(&bare));

        let with_chain = Intent::Unknown {
            raw_tokens: vec!["жер".into()],
            noun_hint: Some("жер".into()),
            example: None,
            grounded_fact: None,
            example_adapted: false,
            reasoning_chain: Some("ой-тізбек: жер — аспан денесі".into()),
            question_shape: None,
            temporal_scope: false,
            compositional_function: false,
            noun_hint_polarity: adam_kernel_fst::Polarity::Affirmative,
        };
        assert!(TaskState::intent_has_evidence(&with_chain));

        let with_example = Intent::Unknown {
            raw_tokens: vec!["жер".into()],
            noun_hint: Some("жер".into()),
            example: Some("Жер — біздің планета.".into()),
            grounded_fact: None,
            example_adapted: false,
            reasoning_chain: None,
            question_shape: None,
            temporal_scope: false,
            compositional_function: false,
            noun_hint_polarity: adam_kernel_fst::Polarity::Affirmative,
        };
        assert!(TaskState::intent_has_evidence(&with_example));

        let with_grounded = Intent::Unknown {
            raw_tokens: vec!["жер".into()],
            noun_hint: Some("жер".into()),
            example: None,
            grounded_fact: Some("Жер — Күн жүйесіндегі планета.".into()),
            example_adapted: false,
            reasoning_chain: None,
            question_shape: None,
            temporal_scope: false,
            compositional_function: false,
            noun_hint_polarity: adam_kernel_fst::Polarity::Affirmative,
        };
        assert!(TaskState::intent_has_evidence(&with_grounded));

        // Non-Unknown intents never carry evidence slots.
        assert!(!TaskState::intent_has_evidence(&Intent::Thanks));
        assert!(!TaskState::intent_has_evidence(&Intent::AskName));
    }

    /// v4.0.30 — Codex v4.0.29 review #2 regression. Goal + evidence
    /// in intent MUST produce `ReadyToAnswer`, not the catch-all
    /// `GatheringEvidence` pre-v4.0.30 always returned.
    #[test]
    fn roll_forward_reaches_ready_to_answer_with_injected_chain() {
        let mut t = TaskState::new();
        let belief = BeliefState::new();
        let intent = Intent::Unknown {
            raw_tokens: vec!["жер".into()],
            noun_hint: Some("жер".into()),
            example: None,
            grounded_fact: None,
            example_adapted: false,
            reasoning_chain: Some("ой-тізбек: жер — аспан денесі".into()),
            question_shape: None,
            temporal_scope: false,
            compositional_function: false,
            noun_hint_polarity: adam_kernel_fst::Polarity::Affirmative,
        };
        t.roll_forward(&intent, &belief, 0);
        assert_eq!(t.status, TaskStatus::ReadyToAnswer);
    }

    /// v4.0.30 — contradictions still dominate even when evidence
    /// is present — `Blocked` trumps `ReadyToAnswer`.
    #[test]
    fn blocked_beats_ready_to_answer() {
        let mut t = TaskState::new();
        let mut belief = BeliefState::new();
        belief.record_user_fact(USER_SELF_KEY, "city", "алматы", 0);
        belief.record_user_fact(USER_SELF_KEY, "city", "астана", 1);
        assert!(!belief.contradictions.is_empty());

        let intent = Intent::Unknown {
            raw_tokens: vec!["жер".into()],
            noun_hint: Some("жер".into()),
            example: Some("sample".into()),
            grounded_fact: None,
            example_adapted: false,
            reasoning_chain: Some("chain".into()),
            question_shape: None,
            temporal_scope: false,
            compositional_function: false,
            noun_hint_polarity: adam_kernel_fst::Polarity::Affirmative,
        };
        t.roll_forward(&intent, &belief, 2);
        assert_eq!(t.status, TaskStatus::Blocked);
    }
}
