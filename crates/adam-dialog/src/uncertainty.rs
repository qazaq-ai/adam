//! Layer 3.10 — uncertainty / epistemic status, introduced in v4.0.33.
//!
//! Codex v4.0.26 roadmap "Phase 5" (part 1). Pre-v4.0.33 the dialog
//! knew only "this answer is either supported or it isn't" (Phase 4
//! `VerificationReport`). That's enough to gate, but it doesn't tell
//! the realiser *how* confident the answer is — "I know", "evidence
//! says", "derived through reasoning", "not sure", "unknown",
//! "conflicted" are different speech acts and deserve different
//! phrasing.
//!
//! This module introduces [`EpistemicStatus`] — a coarse band the
//! policy picks from `(ActionPlan, VerificationReport, Intent,
//! BeliefState)`. **v4.0.33 scope: classifier + trace only**. The
//! status is computed and surfaced via
//! [`crate::TurnTrace::epistemic_status`]; templates don't yet
//! consume it. v4.0.34 (Phase 5 part 2) adds the `unknown.conflicted`
//! / `unknown.tentative` template families and wires the policy into
//! rendering, at which point the reply text starts reflecting the
//! status.
//!
//! Splitting Phase 5 across two releases matches how we handled
//! Phase 1 (substrate v4.0.27 → behaviour v4.0.28 fix) and Phase 2
//! (v4.0.29 → v4.0.30). Each step is Codex-reviewable before the
//! next lands.

use crate::action::{Action, ActionPlan};
use crate::belief::BeliefState;
use crate::intent::Intent;
use crate::verifier::{VerificationIssue, VerificationReport};

/// Coarse trust / uncertainty band for the system's current answer.
/// Codex v4.0.26 roadmap wording: *«Certain → прямой ответ;
/// Supported → ответ с цитатой; Derived → ответ с «байланыс-»;
/// Tentative → мягкая формулировка + запрос уточнения;
/// Unknown → честный fallback; Conflicted → явное указание на
/// конфликт»*.
///
/// The six variants are ordered roughly **strongest to weakest** —
/// useful for eventual ranking decisions but not semantically
/// meaningful (equality is by variant tag).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpistemicStatus {
    /// System knows the answer directly — active belief fact, social
    /// pleasantry, or acknowledged user statement.
    Certain,
    /// Retrieval cited a verbatim corpus sample. The answer is
    /// backed by specific evidence the user can verify.
    Supported,
    /// Rule-derived chain rendered with the «байланыс-» marker. The
    /// system reasoned from base facts; the user can trust the
    /// individual steps but the whole chain is an inference, not a
    /// primary observation.
    Derived,
    /// Partial evidence only — or the planner chose
    /// `AskClarification`. Phase 5 part 2 will render this with a
    /// hedging marker («бәлкім…» / «анық емес») + an optional
    /// follow-up question.
    Tentative,
    /// System has nothing to say on the topic. Safe fallback —
    /// «түсінбедім» / «білмеймін».
    Unknown,
    /// `BeliefState` has an unresolved contradiction that blocks a
    /// confident answer. Phase 5 part 2 will render this with
    /// explicit conflict-surfacing templates
    /// («сіз бұрын X дедіңіз, қазір Y дейсіз…»). Until then the
    /// upstream verifier gate already strips evidence — the status
    /// just labels *why* the system is silent.
    Conflicted,
}

/// Pure-function classifier. Reads `(ActionPlan, VerificationReport,
/// Intent, BeliefState)` — NEVER mutates state — and returns the
/// band that best fits.
///
/// Order matters: `Conflicted` and `Unknown` dominate, then action-
/// specific mappings, then status falls through to
/// [`EpistemicStatus::Unknown`].
pub struct UncertaintyPolicy;

impl UncertaintyPolicy {
    pub fn derive(
        plan: &ActionPlan,
        verification: &VerificationReport,
        intent: &Intent,
        belief: &BeliefState,
    ) -> EpistemicStatus {
        // 1. Contradictions always win. Even if the realiser will
        // fall through to a safe fallback (because the Phase 4 gate
        // stripped evidence), the epistemic truth is that the
        // system *knows it has a conflict*. Phase 5 part 2 will
        // produce a conflict-surfacing template; v4.0.33 just
        // records the status.
        if !belief.contradictions.is_empty() {
            return EpistemicStatus::Conflicted;
        }

        // 2. Verifier flagged `ContradictoryBelief` without the
        // belief actually holding one (e.g. intermediate states).
        // Defensive: treat as Conflicted.
        if verification
            .issues
            .contains(&VerificationIssue::ContradictoryBelief)
        {
            return EpistemicStatus::Conflicted;
        }

        // 3. Explicit refusal from the planner. The system
        // acknowledged it has nothing to say.
        if plan.action == Action::RefuseOutOfScope {
            return EpistemicStatus::Unknown;
        }

        // 4. Missing evidence → Tentative. The planner aimed for an
        // evidence-based answer but the intent/belief couldn't back
        // it, so the Phase 4 gate stripped it. The realiser will
        // produce a noun-echo in v4.0.33; v4.0.34 will soften it.
        if verification
            .issues
            .contains(&VerificationIssue::MissingEvidence)
        {
            return EpistemicStatus::Tentative;
        }

        // 5. Action-specific confidence.
        match plan.action {
            Action::Social => EpistemicStatus::Certain,
            Action::AnswerDirect => EpistemicStatus::Certain,
            Action::RetrieveEvidence => EpistemicStatus::Supported,
            Action::RunReasoner => EpistemicStatus::Derived,
            Action::AskClarification => EpistemicStatus::Tentative,
            Action::CheckContradiction => EpistemicStatus::Conflicted,
            // **v4.4.0** — Dismissal is a deliberate user-driven
            // belief-state change; the system has just performed
            // exactly what was asked. `Certain` matches `Social`
            // and `AnswerDirect`: high confidence in the action,
            // not in any factual claim.
            Action::DismissContradiction => EpistemicStatus::Certain,
            Action::SummarizeBelief => EpistemicStatus::Supported,
            Action::RefuseOutOfScope => EpistemicStatus::Unknown,
        }
        // intent / belief args are available for future refinement
        // (e.g. `Tentative` when retrieval score is low or when the
        // active_fact confidence band is non-`Confirmed`). Not used
        // in v4.0.33; kept in the signature so Phase 5 part 2 /
        // Phase 6 don't break the call site.
        .and_refine(intent, belief)
    }
}

trait StatusRefine: Sized {
    fn and_refine(self, intent: &Intent, belief: &BeliefState) -> Self;
}

impl StatusRefine for EpistemicStatus {
    /// Reserved hook. v4.0.33 is a no-op; Phase 5 part 2 /
    /// Phase 6 refinements (low retrieval score, non-Confirmed
    /// confidence, etc.) will plug in here without the call site in
    /// `UncertaintyPolicy::derive` changing.
    fn and_refine(self, _intent: &Intent, _belief: &BeliefState) -> Self {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{ActionPlan, OutputKind};
    use crate::belief::USER_SELF_KEY;
    use crate::verifier::{VerificationIssue, VerificationReport};

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
            input_evidence: None,
            input_is_inversion_question: false,
        }
    }

    fn plan(action: Action) -> ActionPlan {
        let kind = match action {
            Action::RunReasoner => OutputKind::DerivedAnswer,
            Action::RetrieveEvidence => OutputKind::EvidenceAnswer,
            Action::AnswerDirect => OutputKind::DirectAnswer,
            Action::CheckContradiction | Action::AskClarification => OutputKind::ClarifyingQuestion,
            Action::Social | Action::DismissContradiction => OutputKind::SocialPleasantry,
            Action::RefuseOutOfScope | Action::SummarizeBelief => OutputKind::SafeFallback,
        };
        ActionPlan {
            action,
            rationale: vec!["test".into()],
            required_inputs: vec![],
            expected_output: kind,
        }
    }

    fn report(issues: Vec<VerificationIssue>, supported: bool) -> VerificationReport {
        VerificationReport {
            supported,
            issues,
            evidence_count: if supported { 1 } else { 0 },
        }
    }

    #[test]
    fn belief_contradiction_dominates_to_conflicted() {
        let mut belief = BeliefState::new();
        belief.record_user_fact(USER_SELF_KEY, "city", "алматы", 0);
        belief.record_user_fact(USER_SELF_KEY, "city", "астана", 1);
        // Even if action is RunReasoner and verifier somehow passes,
        // the live conflict in belief must win.
        let status = UncertaintyPolicy::derive(
            &plan(Action::RunReasoner),
            &report(vec![], true),
            &unknown(Some("жер"), Some("chain"), None),
            &belief,
        );
        assert_eq!(status, EpistemicStatus::Conflicted);
    }

    #[test]
    fn contradictory_belief_issue_without_live_conflict_still_conflicted() {
        let belief = BeliefState::new(); // no live conflict
        let status = UncertaintyPolicy::derive(
            &plan(Action::RunReasoner),
            &report(vec![VerificationIssue::ContradictoryBelief], false),
            &unknown(Some("жер"), Some("chain"), None),
            &belief,
        );
        assert_eq!(status, EpistemicStatus::Conflicted);
    }

    #[test]
    fn missing_evidence_routes_to_tentative() {
        let belief = BeliefState::new();
        let status = UncertaintyPolicy::derive(
            &plan(Action::RunReasoner),
            &report(vec![VerificationIssue::MissingEvidence], false),
            &unknown(Some("жер"), None, None),
            &belief,
        );
        assert_eq!(status, EpistemicStatus::Tentative);
    }

    #[test]
    fn refuse_out_of_scope_is_unknown() {
        let belief = BeliefState::new();
        let status = UncertaintyPolicy::derive(
            &plan(Action::RefuseOutOfScope),
            &report(vec![], true),
            &unknown(None, None, None),
            &belief,
        );
        assert_eq!(status, EpistemicStatus::Unknown);
    }

    #[test]
    fn social_is_certain() {
        let belief = BeliefState::new();
        let status = UncertaintyPolicy::derive(
            &plan(Action::Social),
            &report(vec![], true),
            &Intent::Thanks,
            &belief,
        );
        assert_eq!(status, EpistemicStatus::Certain);
    }

    #[test]
    fn answer_direct_is_certain() {
        let mut belief = BeliefState::new();
        belief.record_user_fact(USER_SELF_KEY, "name", "Дәулет", 0);
        let status = UncertaintyPolicy::derive(
            &plan(Action::AnswerDirect),
            &report(vec![], true),
            &Intent::AskName,
            &belief,
        );
        assert_eq!(status, EpistemicStatus::Certain);
    }

    #[test]
    fn run_reasoner_is_derived() {
        let belief = BeliefState::new();
        let status = UncertaintyPolicy::derive(
            &plan(Action::RunReasoner),
            &report(vec![], true),
            &unknown(Some("жер"), Some("chain"), None),
            &belief,
        );
        assert_eq!(status, EpistemicStatus::Derived);
    }

    #[test]
    fn retrieve_evidence_is_supported() {
        let belief = BeliefState::new();
        let status = UncertaintyPolicy::derive(
            &plan(Action::RetrieveEvidence),
            &report(vec![], true),
            &unknown(Some("жер"), None, Some("sample")),
            &belief,
        );
        assert_eq!(status, EpistemicStatus::Supported);
    }

    #[test]
    fn ask_clarification_is_tentative() {
        let belief = BeliefState::new();
        let status = UncertaintyPolicy::derive(
            &plan(Action::AskClarification),
            &report(vec![], true),
            &unknown(Some("жер"), None, None),
            &belief,
        );
        assert_eq!(status, EpistemicStatus::Tentative);
    }

    #[test]
    fn check_contradiction_action_is_conflicted_status() {
        let mut belief = BeliefState::new();
        belief.record_user_fact(USER_SELF_KEY, "city", "алматы", 0);
        belief.record_user_fact(USER_SELF_KEY, "city", "астана", 1);
        let status = UncertaintyPolicy::derive(
            &plan(Action::CheckContradiction),
            &report(vec![], true),
            &unknown(None, None, None),
            &belief,
        );
        // Belief contradiction wins at step 1 before we even check
        // action.
        assert_eq!(status, EpistemicStatus::Conflicted);
    }
}
