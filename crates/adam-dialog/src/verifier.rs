//! Layer 3.9 — verifier, introduced in v4.0.32.
//!
//! Codex v4.0.26 roadmap "Phase 4". Pre-v4.0.32 the dialog would
//! render a reasoning chain or a retrieval citation any time the
//! injection passes populated them — even if `BeliefState` held an
//! unresolved contradiction, or the chosen `ActionPlan` required
//! evidence that wasn't actually present. Phase 4 adds a cheap,
//! deterministic pre-render check and, on failure, **gates** the
//! response: the evidence is stripped from the intent before the
//! template planner runs, and the realiser falls through to the
//! existing safe-fallback templates (`unknown.with_noun` →
//! «ах, X туралы айтасыз ба», or `unknown` → «түсінбедім»).
//!
//! This is the first phase that **actually changes user-visible
//! output**: prior phases (1–3) were pure substrate. Reply text
//! remains byte-identical in the *supported* path; the *unsupported*
//! path — notably "user asked about X while a contradiction sits in
//! their own profile" — now falls back instead of surfacing a
//! formally-correct but context-ignorant chain.
//!
//! Phase 5 (uncertainty policy) will add **templates** for explicit
//! conflict surfacing. Phase 4's job is just: don't render an
//! answer the system can't back.

use crate::action::{Action, ActionPlan};
use crate::belief::{BeliefState, FactStatus, USER_SELF_KEY};
use crate::intent::Intent;

/// Self-contained verdict for a single turn. `supported == true` ⇒
/// render normally; `supported == false` ⇒ the turn loop strips
/// evidence from the intent and routes to the safe-fallback template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationReport {
    /// All gating collapses to this bit. `issues.is_empty()` ⟺
    /// `supported == true`.
    pub supported: bool,
    /// Every failing check the verifier observed. Listed, not just
    /// summarised, so `adam_chat --trace` can show the auditor *why*
    /// the gate fired.
    pub issues: Vec<VerificationIssue>,
    /// Count of supporting evidence items (retrieval hits +
    /// reasoning chains + active belief facts) the verifier was
    /// able to cite for this `(intent, action)` pair. Zero is a
    /// red flag for evidence-based actions; non-zero doesn't
    /// automatically mean safe — see the per-issue checks below.
    pub evidence_count: usize,
}

/// What, specifically, went wrong. Enumerated so higher-level
/// consumers (Phase 5 uncertainty policy, future dashboards) can
/// dispatch on the failure mode instead of parsing free-form text.
///
/// The list is intentionally short — Phase 4 only implements the
/// checks that are both *cheap to evaluate* and *safe to act on*.
/// More nuanced failures (e.g. `UnsafeGeneralization`) are reserved
/// as variants so future phases can emit them without a schema
/// break.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationIssue {
    /// The `ActionPlan` promised evidence but the intent / belief
    /// carry none. E.g. `Action::RunReasoner` on an
    /// `Intent::Unknown { reasoning_chain: None, .. }`.
    MissingEvidence,
    /// Unresolved contradiction in `BeliefState` + the plan is
    /// going to render an evidence-based answer. The answer itself
    /// might be factually correct, but producing it while the
    /// user's own profile is inconsistent violates the Codex v4.0.28
    /// "conflicts dominate" invariant.
    ContradictoryBelief,
    /// Reserved — reasoner chain present but confidence is too low
    /// (e.g. R5 chain via proverb-only sources). Phase 5 will wire
    /// this once the reranker exposes a confidence band.
    WeakDerivation,
    /// Reserved — template would need slots the intent doesn't
    /// provide. Phase 5 will emit this when template-resolution
    /// diagnostics are threaded through.
    IncompleteSlots,
    /// Reserved — the answer would make a claim stronger than the
    /// evidence warrants (e.g. deriving `IsA` from a single
    /// `RelatedTo` hop). Phase 5.
    UnsafeGeneralization,
}

/// Pure deterministic verifier. Never reads I/O, never mutates
/// state. Returns a report and lets the caller decide whether to
/// gate.
pub struct Verifier;

impl Verifier {
    /// Run every applicable check for the given `(action_plan,
    /// intent, belief)` triple. Ordering: contradiction check first
    /// (matches Codex v4.0.28 precedence), then action-specific
    /// evidence presence, then per-branch content checks.
    pub fn verify(plan: &ActionPlan, intent: &Intent, belief: &BeliefState) -> VerificationReport {
        let mut issues: Vec<VerificationIssue> = Vec::new();
        let mut evidence_count: usize = 0;

        // Global check: any unresolved contradiction poisons rendering
        // of an evidence-shaped answer. The key realisation
        // (v4.0.32 iteration): even when the planner correctly
        // routes to `CheckContradiction`, the existing TEMPLATE
        // planner ignores the action and picks its template from
        // intent shape. If the intent still has `reasoning_chain` /
        // `example` attached, the template would render an answer
        // instead of a clarification.
        //
        // So the contradiction gate fires whenever
        // `belief.contradictions` is non-empty AND the intent would
        // render an answer. This applies regardless of which action
        // the planner chose — the gate ensures the realiser never
        // emits an evidence-backed answer on top of a known conflict.
        // Phase 5 will add real clarification templates and let
        // `CheckContradiction` produce aligned output without
        // stripping.
        if !belief.contradictions.is_empty() && Self::intent_has_answer_shape(intent) {
            issues.push(VerificationIssue::ContradictoryBelief);
        }

        // Per-action checks. Each branch either bumps
        // `evidence_count` (found what it needed) or pushes an
        // issue (couldn't back the promised answer).
        match plan.action {
            Action::RunReasoner => match intent {
                Intent::Unknown {
                    reasoning_chain: Some(_),
                    ..
                } => evidence_count += 1,
                _ => issues.push(VerificationIssue::MissingEvidence),
            },
            Action::RetrieveEvidence => match intent {
                Intent::Unknown {
                    grounded_fact: Some(_),
                    ..
                } => evidence_count += 1,
                Intent::Unknown {
                    example: Some(_), ..
                } => evidence_count += 1,
                _ => issues.push(VerificationIssue::MissingEvidence),
            },
            Action::AnswerDirect => {
                // **v4.3.3** — `Intent::AskAboutSystem` answers from
                // adam's hardcoded identity in the `ask_about_system`
                // template family, not from belief. The verifier
                // accepts this as self-evidence: there is no belief
                // slot to look up, but the answer is also not
                // unsupported — the system's identity is a build-time
                // contract.
                if matches!(intent, Intent::AskAboutSystem { .. }) {
                    evidence_count += 1;
                } else {
                    // Must have a matching active fact. The planner
                    // already checked this in `belief_direct_answer`,
                    // but the verifier re-checks defensively so future
                    // planner changes can't bypass the gate.
                    let slot = match intent {
                        Intent::AskName => Some("name"),
                        Intent::AskAge => Some("age"),
                        Intent::AskLocation => Some("city"),
                        Intent::AskOccupation => Some("occupation"),
                        _ => None,
                    };
                    match slot.and_then(|s| belief.active_fact(USER_SELF_KEY, s)) {
                        Some(_) => evidence_count += 1,
                        None => issues.push(VerificationIssue::MissingEvidence),
                    }
                }
            }
            Action::CheckContradiction => {
                // The planner picked this because contradictions
                // exist — so they should be present.
                if belief.contradictions.is_empty() {
                    issues.push(VerificationIssue::MissingEvidence);
                } else {
                    evidence_count += belief.contradictions.len();
                }
            }
            Action::SummarizeBelief => {
                evidence_count += belief
                    .facts
                    .iter()
                    .filter(|f| f.status == FactStatus::Active)
                    .count();
                if evidence_count == 0 {
                    issues.push(VerificationIssue::MissingEvidence);
                }
            }
            Action::AskClarification
            | Action::Social
            | Action::DismissContradiction
            | Action::RefuseOutOfScope => {
                // These actions do not need evidence by design;
                // they're the system's "out" for unclear or
                // meta-level turns.
            }
        }

        VerificationReport {
            supported: issues.is_empty(),
            issues,
            evidence_count,
        }
    }

    /// Actions whose rendered output is an **answer** supported by
    /// evidence. Retained for potential per-action gating by future
    /// phases; the v4.0.32 contradiction gate is intent-shape-based
    /// and doesn't rely on this — see `verify`.
    #[allow(dead_code)]
    fn is_evidence_action(action: Action) -> bool {
        matches!(
            action,
            Action::RunReasoner
                | Action::RetrieveEvidence
                | Action::AnswerDirect
                | Action::SummarizeBelief
        )
    }

    /// True when the intent carries a slot (`reasoning_chain` or
    /// `example`) that would drive the existing template planner
    /// to render an answer rather than a question. Gate decisions
    /// key on this because the template layer is blind to
    /// `ActionPlan`.
    fn intent_has_answer_shape(intent: &Intent) -> bool {
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
}

/// Strip every injected evidence slot from an `Intent::Unknown` so
/// the template planner falls through to the safe-fallback branch
/// (`unknown.with_noun` → «ах, X туралы айтасыз ба» or `unknown` →
/// «түсінбедім»). Leaves other intent variants untouched — they
/// carry no evidence slots to strip.
///
/// Called by the turn loop when
/// [`VerificationReport::supported`] is `false`.
pub fn strip_evidence(intent: Intent) -> Intent {
    match intent {
        Intent::Unknown {
            raw_tokens,
            noun_hint,
            question_shape,
            temporal_scope,
            compositional_function,
            ..
        } => Intent::Unknown {
            raw_tokens,
            noun_hint,
            example: None,
            grounded_fact: None,
            example_adapted: false,
            reasoning_chain: None,
            // **v4.12.0** — preserve question_shape across the
            // strip; it's an analytical signal, not evidence.
            question_shape,
            // **v4.23.0** — same: temporal_scope is an analytical
            // signal about query structure, not evidence; preserve
            // it through strip_evidence so the planner still routes
            // to the temporal-no-data template family even after
            // verifier rejects injected evidence.
            temporal_scope,
            // **v4.23.5** — same: compositional_function is an
            // analytical signal about query structure.
            compositional_function,
        },
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{ActionPlan, OutputKind};

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
        }
    }

    fn plan(action: Action) -> ActionPlan {
        // Output kind doesn't affect verifier — just pick a
        // plausible one per action.
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

    #[test]
    fn run_reasoner_with_chain_is_supported() {
        let belief = BeliefState::new();
        let intent = unknown(Some("жер"), Some("chain"), None);
        let r = Verifier::verify(&plan(Action::RunReasoner), &intent, &belief);
        assert!(r.supported);
        assert_eq!(r.evidence_count, 1);
        assert!(r.issues.is_empty());
    }

    #[test]
    fn run_reasoner_without_chain_flags_missing_evidence() {
        let belief = BeliefState::new();
        let intent = unknown(Some("жер"), None, None);
        let r = Verifier::verify(&plan(Action::RunReasoner), &intent, &belief);
        assert!(!r.supported);
        assert!(r.issues.contains(&VerificationIssue::MissingEvidence));
    }

    #[test]
    fn retrieve_evidence_without_example_flags_missing_evidence() {
        let belief = BeliefState::new();
        let intent = unknown(Some("жер"), None, None);
        let r = Verifier::verify(&plan(Action::RetrieveEvidence), &intent, &belief);
        assert!(!r.supported);
        assert!(r.issues.contains(&VerificationIssue::MissingEvidence));
    }

    #[test]
    fn contradiction_in_belief_blocks_evidence_based_answers() {
        let mut belief = BeliefState::new();
        belief.record_user_fact(USER_SELF_KEY, "city", "алматы", 0);
        belief.record_user_fact(USER_SELF_KEY, "city", "астана", 1);
        let intent = unknown(Some("жер"), Some("chain"), None);
        let r = Verifier::verify(&plan(Action::RunReasoner), &intent, &belief);
        assert!(!r.supported);
        assert!(r.issues.contains(&VerificationIssue::ContradictoryBelief));
    }

    /// v4.0.32 iteration — the gate is now intent-shape-based, not
    /// action-based. `CheckContradiction` + an intent that still
    /// carries answer-shaped evidence IS rejected, because the
    /// existing template planner would render the evidence and miss
    /// the contradiction. Phase 5 will add clarification templates
    /// so CheckContradiction can produce aligned output; until then
    /// the gate strips evidence for this case.
    #[test]
    fn check_contradiction_with_answer_shape_intent_is_gated() {
        let mut belief = BeliefState::new();
        belief.record_user_fact(USER_SELF_KEY, "city", "алматы", 0);
        belief.record_user_fact(USER_SELF_KEY, "city", "астана", 1);
        let intent = unknown(Some("жер"), Some("chain"), None);
        let r = Verifier::verify(&plan(Action::CheckContradiction), &intent, &belief);
        assert!(
            !r.supported,
            "intent with answer shape under a conflict must be gated regardless of action, got {r:?}"
        );
        assert!(r.issues.contains(&VerificationIssue::ContradictoryBelief));
    }

    /// v4.0.32 — `CheckContradiction` on a question-shape intent
    /// (no reasoning_chain, no example) remains supported: the gate
    /// is only firing when rendering would produce a mismatched
    /// answer.
    #[test]
    fn check_contradiction_with_question_shape_intent_passes() {
        let mut belief = BeliefState::new();
        belief.record_user_fact(USER_SELF_KEY, "city", "алматы", 0);
        belief.record_user_fact(USER_SELF_KEY, "city", "астана", 1);
        // Question-shape: no evidence attached.
        let intent = unknown(Some("жер"), None, None);
        let r = Verifier::verify(&plan(Action::CheckContradiction), &intent, &belief);
        assert!(
            r.supported,
            "question-shape intent with CheckContradiction action must pass, got {r:?}"
        );
    }

    #[test]
    fn social_and_refuse_actions_are_always_supported() {
        let belief = BeliefState::new();
        let intent = Intent::Thanks;
        for action in [
            Action::Social,
            Action::RefuseOutOfScope,
            Action::AskClarification,
        ] {
            let r = Verifier::verify(&plan(action), &intent, &belief);
            assert!(r.supported, "action {action:?} must pass verification");
        }
    }

    #[test]
    fn answer_direct_without_matching_belief_flags_missing_evidence() {
        let belief = BeliefState::new();
        let r = Verifier::verify(&plan(Action::AnswerDirect), &Intent::AskName, &belief);
        assert!(!r.supported);
        assert!(r.issues.contains(&VerificationIssue::MissingEvidence));
    }

    #[test]
    fn answer_direct_with_matching_belief_is_supported() {
        let mut belief = BeliefState::new();
        belief.record_user_fact(USER_SELF_KEY, "name", "Дәулет", 0);
        let r = Verifier::verify(&plan(Action::AnswerDirect), &Intent::AskName, &belief);
        assert!(r.supported);
        assert_eq!(r.evidence_count, 1);
    }

    #[test]
    fn strip_evidence_clears_unknown_slots() {
        let intent = unknown(Some("жер"), Some("chain"), Some("example"));
        let stripped = strip_evidence(intent);
        match stripped {
            Intent::Unknown {
                noun_hint,
                example,
                reasoning_chain,
                example_adapted,
                ..
            } => {
                assert_eq!(noun_hint, Some("жер".into()));
                assert!(example.is_none());
                assert!(reasoning_chain.is_none());
                assert!(!example_adapted);
            }
            other => panic!("expected Intent::Unknown, got {other:?}"),
        }
    }

    #[test]
    fn strip_evidence_passes_through_non_unknown_intents() {
        let stripped = strip_evidence(Intent::Thanks);
        assert_eq!(stripped, Intent::Thanks);
    }
}
