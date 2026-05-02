//! Layer 3.8 — action planner, introduced in v4.0.31.
//!
//! Codex v4.0.26 roadmap "Phase 3". Pre-v4.0.31 the dialog's `planner`
//! module picked a surface **template** for the recognised intent; it
//! had no notion of *what action the system should take*. This module
//! introduces a coarse action vocabulary and a **pure classifier**
//! that decides which action fits the current `(intent, belief,
//! task)` triple.
//!
//! **Non-breaking in v4.0.31** — the `ActionPlan` is classified but
//! not yet executed: the existing template planner still drives the
//! surface form, so reply text is byte-identical to v4.0.30. The
//! classification is stored on [`crate::task::TaskState::last_action`]
//! and surfaced in [`crate::TurnTrace`] for audit. Phase 4 (verifier)
//! and later phases will start to actually gate / reshape responses
//! based on this layer.
//!
//! The classifier is intentionally **pure**: no I/O, no randomness,
//! no side effects. Given the same inputs it always returns the same
//! plan; tests cover every branch end-to-end.
//!
//! This separation — classify now, act later — lets Phase 3 ship
//! safely as a Codex-reviewable substrate: consumers can inspect the
//! plan via `adam_chat --trace` and verify the routing is sane
//! before Phase 4 starts consuming it for gating.

use crate::belief::BeliefState;
use crate::intent::{
    Intent, UnknownAnswerMode, unknown_answer_mode, unknown_prefers_quoted_example,
};
use crate::task::{TaskState, TaskStatus};

/// Coarse vocabulary of actions the system can take on a given turn.
/// Intentionally small — we only include actions the current code
/// base can actually implement (Phase 4+ will expand as new
/// capabilities land, e.g. `CallTool`).
///
/// Every variant maps to an [`OutputKind`] via
/// [`ActionPlan::expected_output`], so downstream realisers can
/// dispatch on either the action or the output shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Answer straight from `BeliefState::active_fact` — the
    /// system already knows the answer. E.g. user asks their own
    /// name back and we have a `Confirmed` fact for it.
    AnswerDirect,
    /// Cite a verbatim corpus sample via retrieval (`example` slot
    /// already populated on the intent).
    RetrieveEvidence,
    /// Cite a rule-derived reasoning chain (`reasoning_chain` slot
    /// already populated on the intent).
    RunReasoner,
    /// Ask the user to disambiguate / fill a missing slot. Triggered
    /// when we have a goal but no evidence path to answer.
    AskClarification,
    /// Surface a belief contradiction to the user (two competing
    /// claims with the same `(subject, predicate)`).
    CheckContradiction,
    /// **v4.4.0** — User opted out of resolving a pending
    /// contradiction (replied "neither / I don't know / skip" to
    /// a `CheckContradiction` prompt). The conflict has just been
    /// dismissed by `BeliefState::dismiss_contradiction`; this
    /// action surfaces a brief acknowledgement reply ("ұқтым,
    /// екеуін де жадтан өшірдім") and clears the conversational
    /// state. Distinct from `Social` because it carries
    /// state-change semantics — belief was actually mutated.
    DismissContradiction,
    /// User asked what the system knows about them. Enumerate
    /// active belief facts. v4.0.31 reserves the action but doesn't
    /// yet emit — no intent currently triggers it.
    SummarizeBelief,
    /// Nothing actionable — intent is genuinely out of scope and no
    /// evidence exists. Signals the realiser to produce a safe
    /// fallback ("түсінбедім") rather than guess.
    RefuseOutOfScope,
    /// Social turn (greeting, farewell, thanks, affirmation, etc.).
    /// No cognitive work needed; the template planner handles it
    /// entirely via its canned templates.
    Social,
}

/// Shape of the response the action is expected to produce. Lets
/// downstream consumers dispatch on the rendering style without
/// re-classifying the action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputKind {
    /// Short factual answer sourced from belief (e.g. "Дәулет").
    DirectAnswer,
    /// Answer that includes a quoted corpus citation.
    EvidenceAnswer,
    /// Answer that includes a rule-derived reasoning chain with the
    /// «байланыс-» trust marker.
    DerivedAnswer,
    /// A question back to the user (clarification or conflict).
    ClarifyingQuestion,
    /// Safe fallback ("түсінбедім" / "білмеймін").
    SafeFallback,
    /// Social template (greeting/thanks/…).
    SocialPleasantry,
}

/// Full classification for a single turn. Holds the chosen action,
/// a short rationale list (why was this action chosen?), any inputs
/// the executor will need, and the expected output shape.
///
/// The `rationale` is a list of short strings rather than a free-form
/// log so tests can assert specific entries — e.g. "belief has
/// unresolved contradiction on city" — without coupling to exact
/// wording of a full-prose explanation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionPlan {
    pub action: Action,
    pub rationale: Vec<String>,
    pub required_inputs: Vec<String>,
    pub expected_output: OutputKind,
}

impl ActionPlan {
    pub(crate) fn new(
        action: Action,
        expected_output: OutputKind,
        rationale: Vec<String>,
        required_inputs: Vec<String>,
    ) -> Self {
        Self {
            action,
            rationale,
            required_inputs,
            expected_output,
        }
    }
}

/// Pure classifier. Takes the post-injection intent (retrieval +
/// reasoning already attempted), current belief state, current task
/// state, and returns the action that best fits.
///
/// Order of checks is **precedence-significant** — conflicts
/// dominate everything else; social intents are early-exits; then we
/// look at evidence present on the intent; last-resort is
/// `RefuseOutOfScope`.
pub struct ActionPlanner;

impl ActionPlanner {
    /// **v4.4.0** — turn cap (in turns) the contradiction-priority
    /// rule (step 1 of `plan`) waits before falling through to
    /// other actions. After this many turns since
    /// `BeliefConflict.detected_at_turn`, the planner stops
    /// dominating the dialog with `CheckContradiction` and lets
    /// the user move on. The contradiction stays in
    /// `belief.contradictions` (audit trail preserved); only the
    /// planner's priority handling changes.
    ///
    /// Set to 3 turns by default — long enough to give the user
    /// two chances to resolve, short enough that a phantom
    /// contradiction (like the v4.3.2 `Жасанды` bug) doesn't
    /// permanently brick the dialog.
    pub const CONTRADICTION_PRIORITY_TURNS: usize = 3;

    /// Public entry point for production use — applies the v4.4.0
    /// contradiction-priority cap. `current_turn` is the absolute
    /// turn counter the dialog is currently on (`Conversation::turn_counter`).
    pub fn plan_with_turn(
        intent: &Intent,
        belief: &BeliefState,
        task: &TaskState,
        current_turn: usize,
    ) -> ActionPlan {
        Self::plan_inner(intent, belief, task, Some(current_turn))
    }

    /// Backwards-compatible entry point — no contradiction-priority
    /// cap. Existing test scaffolding uses this; the cap is opt-in
    /// via `plan_with_turn` so legacy callers see unchanged behaviour.
    pub fn plan(intent: &Intent, belief: &BeliefState, task: &TaskState) -> ActionPlan {
        Self::plan_inner(intent, belief, task, None)
    }

    fn plan_inner(
        intent: &Intent,
        belief: &BeliefState,
        task: &TaskState,
        current_turn: Option<usize>,
    ) -> ActionPlan {
        // 1. Belief conflict dominates. Never answer on top of an
        // unresolved contradiction — even if evidence exists. This
        // enforces the Codex v4.0.28 "active_fact() == None at
        // conflict is legitimate state" directive at the action
        // layer.
        //
        // **v4.4.0** — priority cap: only dominate while the
        // freshest contradiction is younger than
        // `CONTRADICTION_PRIORITY_TURNS`. After that, fall through
        // to normal action paths so a stale (or phantom) conflict
        // doesn't brick the dialog. The contradiction itself
        // remains in `belief.contradictions` for audit; only
        // priority handling changes.
        let active_contradiction = if let Some(ct) = current_turn {
            belief
                .contradictions
                .iter()
                .any(|c| ct.saturating_sub(c.detected_at_turn) < Self::CONTRADICTION_PRIORITY_TURNS)
        } else {
            !belief.contradictions.is_empty()
        };
        if active_contradiction {
            let rationale = belief
                .contradictions
                .iter()
                .map(|c| format!("contradiction on {} {}", c.subject, c.predicate))
                .collect();
            return ActionPlan::new(
                Action::CheckContradiction,
                OutputKind::ClarifyingQuestion,
                rationale,
                vec!["belief.contradictions".into()],
            );
        }

        // 2. Waiting for user (pending non-contradiction question).
        if task.status == TaskStatus::WaitingForUser {
            return ActionPlan::new(
                Action::AskClarification,
                OutputKind::ClarifyingQuestion,
                vec!["task status is WaitingForUser".into()],
                vec!["belief.pending_questions".into()],
            );
        }

        // 3. Social turns bypass the cognitive stack.
        if Self::is_social_intent(intent) {
            return ActionPlan::new(
                Action::Social,
                OutputKind::SocialPleasantry,
                vec!["intent is social / acknowledgement".into()],
                vec![],
            );
        }

        // **v4.3.3** — `AskAboutSystem` is the user asking adam about
        // adam's identity (`сен кімсің?`, `сіз қандай моделсіз?`).
        // Always renders adam's self-introduction from the
        // `ask_about_system` template family, never from belief
        // (belief stores facts about the USER, not the system).
        // Action::AnswerDirect carries `EpistemicStatus::Certain`
        // through the existing UncertaintyPolicy mapping. Track B
        // of `docs/intelligence_roadmap.md`.
        if matches!(intent, Intent::AskAboutSystem { .. }) {
            return ActionPlan::new(
                Action::AnswerDirect,
                OutputKind::DirectAnswer,
                vec!["intent is AskAboutSystem — render adam's self-introduction".into()],
                vec!["system_identity".into()],
            );
        }

        // 4. Profile ask/state intents with matching belief — answer
        // from belief directly. E.g. «менің атым кім» and we have a
        // Confirmed name fact.
        if let Some((slot, object)) = Self::belief_direct_answer(intent, belief) {
            return ActionPlan::new(
                Action::AnswerDirect,
                OutputKind::DirectAnswer,
                vec![format!("belief has active {slot} = {object}")],
                vec![format!("belief.active_fact(USER, {slot})")],
            );
        }

        // 5. Unknown-with-evidence — direct grounded evidence wins
        // over a reasoning chain. The user-facing chat should prefer
        // a curated fact or safe quote when available, and only fall
        // back to derivation when no grounded evidence path exists.
        if let Intent::Unknown {
            raw_tokens,
            grounded_fact,
            example,
            reasoning_chain,
            ..
        } = intent
        {
            match unknown_answer_mode(raw_tokens) {
                UnknownAnswerMode::Example if example.is_some() => {
                    return ActionPlan::new(
                        Action::RetrieveEvidence,
                        OutputKind::EvidenceAnswer,
                        vec!["intent explicitly asks for an example".into()],
                        vec!["intent.example".into()],
                    );
                }
                UnknownAnswerMode::Reasoning if reasoning_chain.is_some() => {
                    return ActionPlan::new(
                        Action::RunReasoner,
                        OutputKind::DerivedAnswer,
                        vec!["intent explicitly asks for a reason or relation".into()],
                        vec!["intent.reasoning_chain".into()],
                    );
                }
                _ => {}
            }
            if grounded_fact.is_some() {
                return ActionPlan::new(
                    Action::RetrieveEvidence,
                    OutputKind::EvidenceAnswer,
                    vec!["intent carries injected grounded_fact".into()],
                    vec!["intent.grounded_fact".into()],
                );
            }
            if example.is_some() {
                let rationale = if unknown_prefers_quoted_example(raw_tokens) {
                    "intent carries injected example"
                } else {
                    "intent carries injected grounded/retrieval example"
                };
                return ActionPlan::new(
                    Action::RetrieveEvidence,
                    OutputKind::EvidenceAnswer,
                    vec![rationale.into()],
                    vec!["intent.example".into()],
                );
            }
            if reasoning_chain.is_some() {
                return ActionPlan::new(
                    Action::RunReasoner,
                    OutputKind::DerivedAnswer,
                    vec!["intent carries injected reasoning_chain".into()],
                    vec!["intent.reasoning_chain".into()],
                );
            }
        }
        // 6. Unknown with a topic but no evidence — ask the user to
        // clarify (or narrow) rather than guess.
        if let Intent::Unknown {
            noun_hint: Some(topic),
            ..
        } = intent
        {
            return ActionPlan::new(
                Action::AskClarification,
                OutputKind::ClarifyingQuestion,
                vec![format!("noun_hint {topic} has no evidence path")],
                vec!["noun_hint".into()],
            );
        }

        // 6.5. Parse failure: user typed something but nothing
        // reducible to a topic, evidence, or chain came out.
        // Distinct from step 7 (genuinely empty input) — here the
        // user spoke, we just couldn't read it. Route to
        // AskClarification (Tentative) with a rephrase prompt
        // rather than RefuseOutOfScope (Unknown), so the dialog
        // says "I didn't understand, rephrase?" instead of "out of
        // scope". Closes aspirational scenario
        // `aspirational_unparseable_input_distinguished_from_unknown_topic`
        // (v4.0.35 finding, v4.0.40 fix).
        if let Intent::Unknown {
            raw_tokens,
            noun_hint: None,
            example: None,
            reasoning_chain: None,
            ..
        } = intent
        {
            if raw_tokens.iter().any(|t| !t.is_empty()) {
                return ActionPlan::new(
                    Action::AskClarification,
                    OutputKind::ClarifyingQuestion,
                    vec!["input present but no topic / evidence / chain extracted".into()],
                    vec!["intent.raw_tokens".into()],
                );
            }
        }

        // 7. Everything else — safe fallback.
        ActionPlan::new(
            Action::RefuseOutOfScope,
            OutputKind::SafeFallback,
            vec!["no applicable action path".into()],
            vec![],
        )
    }

    /// True for intents that don't carry any cognitive payload —
    /// greetings, thanks, affirmations, etc. Keeping this as a
    /// private helper so the classifier has a single pattern match
    /// for "bypass cognitive stack".
    fn is_social_intent(intent: &Intent) -> bool {
        matches!(
            intent,
            Intent::Greeting { .. }
                | Intent::Farewell
                | Intent::Thanks
                | Intent::Apology
                | Intent::AskHowAreYou
                | Intent::StatementOfWellbeing
                | Intent::Affirmation
                | Intent::Negation
                | Intent::Compliment
                | Intent::WellWishes
                | Intent::Insult
                | Intent::Request
                | Intent::AskWeather
                | Intent::StatementOfWeather
                | Intent::AskTime
        )
    }

    /// Does the current `(intent, belief)` pair support a direct
    /// belief-backed answer? Returns `(slot_name, object)` when yes.
    /// v4.0.31 covers the four user-profile slots; later phases may
    /// extend to arbitrary `(subject, predicate)` queries.
    ///
    /// **v4.1.5** — third step of "tools as execution". Pre-v4.1.5
    /// this method called `BeliefState::active_fact` directly,
    /// bypassing the tool layer entirely (no audit trace, no
    /// uniform interface). Now it routes through
    /// `Tool::dispatch(SearchBelief { subject, predicate: Some(slot) })`,
    /// which honours the same single-active-fact invariant
    /// (v4.0.28) and returns the object string as the single
    /// finding. Reply text byte-identical — same lookup, same
    /// invariant, just routed through the uniform tool channel.
    fn belief_direct_answer(
        intent: &Intent,
        belief: &BeliefState,
    ) -> Option<(&'static str, String)> {
        use crate::belief::USER_SELF_KEY;
        let slot = match intent {
            Intent::AskName => "name",
            Intent::AskAge => "age",
            Intent::AskLocation => "city",
            Intent::AskOccupation => "occupation",
            _ => return None,
        };
        let ctx = crate::tool::ToolContext {
            belief,
            extracted: &[],
            derived: &[],
            retrieval: None,
            rank_config: None,
            query_input: None,
            current_domain: None,
            domain_index: None,
            previous_grounded_fact: None,
            root_affinity: None,
        };
        let result = crate::tool::Tool::dispatch(
            crate::tool::ToolCall::SearchBelief {
                subject: USER_SELF_KEY.into(),
                predicate: Some(slot.into()),
            },
            &ctx,
        );
        if !result.success {
            return None;
        }
        result
            .findings
            .into_iter()
            .next()
            .map(|object| (slot, object))
    }
}

/// Compact summary for trace output — mirrors
/// [`crate::belief::BeliefDigest`] / [`crate::task::TaskDigest`] in
/// spirit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionDigest {
    pub action: Action,
    pub expected_output: OutputKind,
    pub rationale_count: usize,
}

impl ActionPlan {
    pub fn digest(&self) -> ActionDigest {
        ActionDigest {
            action: self.action,
            expected_output: self.expected_output,
            rationale_count: self.rationale.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::belief::{BeliefState, USER_SELF_KEY};
    use crate::intent::{GreetingKind, Intent};
    use crate::task::TaskState;

    fn unknown(topic: Option<&str>, chain: Option<&str>, example: Option<&str>) -> Intent {
        Intent::Unknown {
            raw_tokens: vec![topic.unwrap_or("").into()],
            noun_hint: topic.map(|s| s.into()),
            example: example.map(|s| s.into()),
            grounded_fact: None,
            example_adapted: false,
            reasoning_chain: chain.map(|s| s.into()),
            question_shape: None,
            temporal_scope: false,
            compositional_function: false,
            noun_hint_polarity: adam_kernel_fst::Polarity::Affirmative,
            input_modality: None,
        }
    }

    #[test]
    fn contradiction_always_dominates() {
        let mut belief = BeliefState::new();
        belief.record_user_fact(USER_SELF_KEY, "city", "алматы", 0);
        belief.record_user_fact(USER_SELF_KEY, "city", "астана", 1);
        let task = TaskState::new();
        let intent = unknown(Some("жер"), Some("chain"), None);
        let plan = ActionPlanner::plan(&intent, &belief, &task);
        assert_eq!(plan.action, Action::CheckContradiction);
        assert_eq!(plan.expected_output, OutputKind::ClarifyingQuestion);
        assert!(!plan.rationale.is_empty());
    }

    #[test]
    fn social_intent_routes_to_social() {
        let belief = BeliefState::new();
        let task = TaskState::new();
        for intent in [
            Intent::Greeting {
                kind: GreetingKind::Casual,
            },
            Intent::Thanks,
            Intent::Farewell,
            Intent::Affirmation,
            Intent::Negation,
        ] {
            let plan = ActionPlanner::plan(&intent, &belief, &task);
            assert_eq!(
                plan.action,
                Action::Social,
                "intent {intent:?} must route to Social"
            );
            assert_eq!(plan.expected_output, OutputKind::SocialPleasantry);
        }
    }

    #[test]
    fn unknown_with_reasoning_chain_routes_to_run_reasoner() {
        let belief = BeliefState::new();
        let task = TaskState::new();
        let intent = unknown(Some("жер"), Some("ой-тізбек..."), None);
        let plan = ActionPlanner::plan(&intent, &belief, &task);
        assert_eq!(plan.action, Action::RunReasoner);
        assert_eq!(plan.expected_output, OutputKind::DerivedAnswer);
    }

    #[test]
    fn unknown_with_example_only_routes_to_retrieve_evidence() {
        let belief = BeliefState::new();
        let task = TaskState::new();
        let intent = unknown(Some("жер"), None, Some("Жер — біздің планета."));
        let plan = ActionPlanner::plan(&intent, &belief, &task);
        assert_eq!(plan.action, Action::RetrieveEvidence);
        assert_eq!(plan.expected_output, OutputKind::EvidenceAnswer);
    }

    #[test]
    fn grounded_example_wins_over_reasoning_chain_when_both_present() {
        let belief = BeliefState::new();
        let task = TaskState::new();
        let intent = unknown(Some("жер"), Some("chain"), Some("example"));
        let plan = ActionPlanner::plan(&intent, &belief, &task);
        assert_eq!(
            plan.action,
            Action::RetrieveEvidence,
            "grounded evidence should beat a derived chain when both exist"
        );
    }

    #[test]
    fn explicit_example_request_prefers_quote_over_grounded_fact() {
        let belief = BeliefState::new();
        let task = TaskState::new();
        let intent = Intent::Unknown {
            raw_tokens: vec![
                "әке".into(),
                "туралы".into(),
                "мысал".into(),
                "айтшы".into(),
            ],
            noun_hint: Some("әке".into()),
            example: Some("Әке баласына ақыл айтты.".into()),
            grounded_fact: Some("Әке — отбасының мүшесі.".into()),
            example_adapted: false,
            reasoning_chain: None,
            question_shape: None,
            temporal_scope: false,
            compositional_function: false,
            noun_hint_polarity: adam_kernel_fst::Polarity::Affirmative,
            input_modality: None,
        };
        let plan = ActionPlanner::plan(&intent, &belief, &task);
        assert_eq!(plan.action, Action::RetrieveEvidence);
        assert!(
            plan.rationale
                .iter()
                .any(|r| r.contains("asks for an example"))
        );
    }

    #[test]
    fn explicit_reasoning_request_prefers_chain_over_grounded_fact() {
        let belief = BeliefState::new();
        let task = TaskState::new();
        let intent = Intent::Unknown {
            raw_tokens: vec!["жер".into(), "неге".into(), "байланысты".into()],
            noun_hint: Some("жер".into()),
            example: Some("Жер Күнді айналады.".into()),
            grounded_fact: Some("Жер — Күн жүйесіндегі планета.".into()),
            example_adapted: false,
            reasoning_chain: Some("байланыс бойынша, жер аспан денесіне жатады.".into()),
            question_shape: None,
            temporal_scope: false,
            compositional_function: false,
            noun_hint_polarity: adam_kernel_fst::Polarity::Affirmative,
            input_modality: None,
        };
        let plan = ActionPlanner::plan(&intent, &belief, &task);
        assert_eq!(plan.action, Action::RunReasoner);
        assert!(
            plan.rationale
                .iter()
                .any(|r| r.contains("asks for a reason or relation"))
        );
    }

    #[test]
    fn unknown_with_topic_no_evidence_routes_to_clarification() {
        let belief = BeliefState::new();
        let task = TaskState::new();
        let intent = unknown(Some("обфускаторий"), None, None);
        let plan = ActionPlanner::plan(&intent, &belief, &task);
        assert_eq!(plan.action, Action::AskClarification);
        assert_eq!(plan.expected_output, OutputKind::ClarifyingQuestion);
    }

    #[test]
    fn ask_name_with_belief_routes_to_answer_direct() {
        let mut belief = BeliefState::new();
        belief.record_user_fact(USER_SELF_KEY, "name", "Дәулет", 0);
        let task = TaskState::new();
        let plan = ActionPlanner::plan(&Intent::AskName, &belief, &task);
        assert_eq!(plan.action, Action::AnswerDirect);
        assert_eq!(plan.expected_output, OutputKind::DirectAnswer);
        assert!(
            plan.rationale
                .iter()
                .any(|r| r.contains("name") && r.contains("Дәулет"))
        );
    }

    #[test]
    fn ask_name_without_belief_does_not_route_to_answer_direct() {
        let belief = BeliefState::new();
        let task = TaskState::new();
        let plan = ActionPlanner::plan(&Intent::AskName, &belief, &task);
        // AskName is in `is_social_intent`'s **not**-list, and with
        // no belief, we skip AnswerDirect; no evidence → fall all
        // the way to RefuseOutOfScope.
        assert_eq!(plan.action, Action::RefuseOutOfScope);
    }

    #[test]
    fn no_actionable_path_returns_refuse_out_of_scope() {
        let belief = BeliefState::new();
        let task = TaskState::new();
        let intent = unknown(None, None, None);
        let plan = ActionPlanner::plan(&intent, &belief, &task);
        assert_eq!(plan.action, Action::RefuseOutOfScope);
        assert_eq!(plan.expected_output, OutputKind::SafeFallback);
    }

    #[test]
    fn rationale_is_non_empty_for_every_non_refusal_branch() {
        // Quick coverage check — every decision branch should log
        // SOMETHING the auditor can read.
        let belief = BeliefState::new();
        let task = TaskState::new();
        for intent in [
            Intent::Thanks,
            unknown(Some("жер"), Some("chain"), None),
            unknown(Some("жер"), None, Some("ex")),
            unknown(Some("жер"), None, None),
        ] {
            let plan = ActionPlanner::plan(&intent, &belief, &task);
            assert!(
                !plan.rationale.is_empty(),
                "plan for {intent:?} must carry rationale, got {plan:?}"
            );
        }
    }

    #[test]
    fn digest_preserves_action_and_output_kind() {
        let belief = BeliefState::new();
        let task = TaskState::new();
        let intent = unknown(Some("жер"), Some("chain"), None);
        let plan = ActionPlanner::plan(&intent, &belief, &task);
        let d = plan.digest();
        assert_eq!(d.action, Action::RunReasoner);
        assert_eq!(d.expected_output, OutputKind::DerivedAnswer);
        assert_eq!(d.rationale_count, plan.rationale.len());
    }
}
