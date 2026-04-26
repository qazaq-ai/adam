//! Deterministic response-quality gate for user-facing dialog output.
//!
//! This module does NOT try to judge open-ended "intelligence" from
//! text alone. Its scope is deliberately narrow and auditable:
//! catch machine-visible response defects that make the answer
//! unrealistic, inadequate, or obviously non-human-looking.
//!
//! Current checks:
//! - empty / whitespace-only output
//! - leaked template placeholders (`{name}`, `{city|locative}`, …)
//! - Latin debug / internal artifacts in Kazakh-only output
//! - repeated double-space fragments
//!
//! The trace layer then adds two stricter audits:
//! - surface-vs-trace faithfulness (`audit_trace_faithfulness`)
//! - typed evidence faithfulness (`audit_typed_faithfulness`)
//!   ensuring answers are backed by the correct evidence class
//!   (graph fact vs retrieval sample vs rule-derived conclusion).

use crate::action::Action;
use crate::belief::USER_SELF_KEY;
use crate::conversation::TurnTrace;
use crate::intent::Intent;
use crate::tool::{ToolCall, ToolEvidence};
use adam_reasoning::ontology::{self, validate_derived_fact, validate_fact};
use adam_reasoning::{ConfidenceKind, Predicate as ReasPredicate};

/// Summary of a single response audit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseQualityReport {
    pub output_len: usize,
    pub issues: Vec<ResponseQualityIssue>,
}

impl ResponseQualityReport {
    pub fn is_clean(&self) -> bool {
        self.issues.is_empty()
    }
}

/// Structural response defects that should never reach the user in a
/// deterministic Kazakh-first dialog surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseQualityIssue {
    EmptyOutput,
    PlaceholderLeak,
    LatinCharactersForbidden,
    RepeatedWhitespace,
}

/// Audit a rendered dialog response for deterministic quality issues.
pub fn audit_response(output: &str) -> ResponseQualityReport {
    let mut issues = Vec::new();
    if output.trim().is_empty() {
        issues.push(ResponseQualityIssue::EmptyOutput);
    }
    if output.contains('{') || output.contains('}') {
        issues.push(ResponseQualityIssue::PlaceholderLeak);
    }
    if contains_latin(output) {
        issues.push(ResponseQualityIssue::LatinCharactersForbidden);
    }
    if !output.trim().is_empty() && output.contains("  ") {
        issues.push(ResponseQualityIssue::RepeatedWhitespace);
    }
    ResponseQualityReport {
        output_len: output.chars().count(),
        issues,
    }
}

/// Summary of a response-vs-trace faithfulness audit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceFaithfulnessReport {
    pub issues: Vec<TraceFaithfulnessIssue>,
}

impl TraceFaithfulnessReport {
    pub fn is_clean(&self) -> bool {
        self.issues.is_empty()
    }
}

/// Mismatch between the rendered user-facing answer and the runtime
/// trace that claims to support it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceFaithfulnessIssue {
    EvidenceLeakAfterVerificationFailure,
    AnswerDirectMissingStoredValue,
    DerivedAnswerMissingReasoningMarker,
    EvidenceAnswerMissingRenderedEvidence,
    AdaptedEvidenceMissingMarker,
    ConflictReplyMissingValues,
}

/// Audit whether the rendered output actually matches the runtime
/// execution trace. This stays intentionally narrow and only enforces
/// hard contracts already present in the templates / verifier:
/// - `AnswerDirect` must surface the stored value.
/// - `RunReasoner` must carry the «байланыс-» marker.
/// - `RetrieveEvidence` must surface the chosen evidence text.
/// - `CheckContradiction` must surface both conflicting values.
/// - `verification.supported == false` must not leak stripped evidence.
pub fn audit_trace_faithfulness(output: &str, trace: &TurnTrace) -> TraceFaithfulnessReport {
    let mut issues = Vec::new();
    let lower = output.to_lowercase();

    if !trace.verification.supported
        && leaked_injected_evidence(&lower, &trace.intent_after_injection)
    {
        issues.push(TraceFaithfulnessIssue::EvidenceLeakAfterVerificationFailure);
    }

    match trace.action_digest.action {
        Action::AnswerDirect => {
            if let Some(expected) = expected_direct_answer_value(trace) {
                if !lower.contains(&expected.to_lowercase()) {
                    issues.push(TraceFaithfulnessIssue::AnswerDirectMissingStoredValue);
                }
            }
        }
        Action::RunReasoner if trace.verification.supported => {
            if !lower.contains("байланыс") {
                issues.push(TraceFaithfulnessIssue::DerivedAnswerMissingReasoningMarker);
            }
        }
        Action::RetrieveEvidence if trace.verification.supported => {
            if let Some(expected) = expected_evidence_text(&trace.intent_after_verification) {
                if !lower.contains(&expected.to_lowercase()) {
                    issues.push(TraceFaithfulnessIssue::EvidenceAnswerMissingRenderedEvidence);
                }
            }
            if let Intent::Unknown {
                example: Some(_),
                example_adapted: true,
                ..
            } = &trace.intent_after_verification
            {
                if !lower.contains("бейімд") {
                    issues.push(TraceFaithfulnessIssue::AdaptedEvidenceMissingMarker);
                }
            }
        }
        Action::CheckContradiction => {
            if matches!(trace.intent_after_verification, Intent::Unknown { .. }) {
                if let Some((old_value, new_value)) = latest_conflict_values(trace) {
                    let old = old_value.to_lowercase();
                    let new = new_value.to_lowercase();
                    if !lower.contains(&old) || !lower.contains(&new) {
                        issues.push(TraceFaithfulnessIssue::ConflictReplyMissingValues);
                    }
                }
            }
        }
        _ => {}
    }

    TraceFaithfulnessReport { issues }
}

/// Summary of a typed evidence audit over a single turn trace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedFaithfulnessReport {
    pub issues: Vec<TypedFaithfulnessIssue>,
}

impl TypedFaithfulnessReport {
    pub fn is_clean(&self) -> bool {
        self.issues.is_empty()
    }
}

/// Mismatch between the answer path and the typed evidence the tool
/// layer recorded for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypedFaithfulnessIssue {
    GroundedFactMissingGraphSupport,
    GroundedFactNotHumanApproved,
    ExampleMissingRetrievalSupport,
    ReasoningChainMissingDerivedSupport,
    ReasoningChainMissingRuleMetadata,
    ReasoningChainNotRuleInferred,
    ReasoningChainMissingSupportChain,
}

/// Audit whether the turn's evidence-backed answer is supported by the
/// correct typed source.
///
/// This is intentionally stricter than string-level faithfulness:
/// - grounded facts must come from `SearchGraph` and stay human-approved;
/// - examples must come from retrieval (exact match unless adapted);
/// - reasoning chains must come from a rule-derived fact with rule id
///   and `ConfidenceKind::RuleInferred`.
pub fn audit_typed_faithfulness(trace: &TurnTrace) -> TypedFaithfulnessReport {
    let mut issues = Vec::new();

    if !trace.verification.supported {
        return TypedFaithfulnessReport { issues };
    }

    match (
        &trace.action_digest.action,
        &trace.intent_after_verification,
    ) {
        (
            Action::RetrieveEvidence,
            Intent::Unknown {
                grounded_fact: Some(text),
                ..
            },
        ) => match matching_graph_fact(trace, text) {
            None => issues.push(TypedFaithfulnessIssue::GroundedFactMissingGraphSupport),
            Some(ToolEvidence::GraphFact { confidence, .. })
                if *confidence != ConfidenceKind::HumanApproved =>
            {
                issues.push(TypedFaithfulnessIssue::GroundedFactNotHumanApproved);
            }
            Some(_) => {}
        },
        (
            Action::RetrieveEvidence,
            Intent::Unknown {
                example: Some(text),
                example_adapted,
                ..
            },
        ) => {
            let supported = if *example_adapted {
                has_any_retrieval_sample(trace)
            } else {
                has_retrieval_sample(trace, text)
            };
            if !supported {
                issues.push(TypedFaithfulnessIssue::ExampleMissingRetrievalSupport);
            }
        }
        (
            Action::RunReasoner,
            Intent::Unknown {
                reasoning_chain: Some(chain),
                ..
            },
        ) => match matching_derived_fact(trace, chain) {
            None => issues.push(TypedFaithfulnessIssue::ReasoningChainMissingDerivedSupport),
            Some(ToolEvidence::DerivedFact { rule_id, .. }) if rule_id.trim().is_empty() => {
                issues.push(TypedFaithfulnessIssue::ReasoningChainMissingRuleMetadata);
            }
            Some(ToolEvidence::DerivedFact { confidence, .. })
                if *confidence != ConfidenceKind::RuleInferred =>
            {
                issues.push(TypedFaithfulnessIssue::ReasoningChainNotRuleInferred);
            }
            Some(ToolEvidence::DerivedFact { support_chain, .. }) if support_chain.is_empty() => {
                issues.push(TypedFaithfulnessIssue::ReasoningChainMissingSupportChain);
            }
            Some(_) => {}
        },
        _ => {}
    }

    TypedFaithfulnessReport { issues }
}

/// Summary of graph admissibility checks for one turn trace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphAdmissibilityReport {
    pub issues: Vec<GraphAdmissibilityIssue>,
}

impl GraphAdmissibilityReport {
    pub fn is_clean(&self) -> bool {
        self.issues.is_empty()
    }
}

/// Hard graph-schema violations. These do not ask whether the answer
/// is faithful to its source; they ask whether the source itself is
/// admissible for the tool and predicate class that produced it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphAdmissibilityIssue {
    SearchGraphReturnedWrongEvidenceType,
    SearchGraphSubjectMismatch,
    SearchGraphPredicateMismatch,
    ReasonerReturnedWrongEvidenceType,
    DerivedFactTopicMismatch,
    DerivedFactRulePredicateMismatch,
    DerivedFactMissingSupportChain,
    DerivedFactSupportPatternMismatch,
    SupportingFactInvalid,
    PlacePredicateRequiresPlaceObject,
    AfterPredicateRequiresTimeLikeEntities,
}

/// Audit whether graph and reasoner evidence respects the repository's
/// typed admissibility contract.
///
/// Current enforced invariants:
/// - `SearchGraph` may only surface `GraphFact` evidence for its own
///   requested subject and optional predicate filter.
/// - `RunLocalReasoner` may only surface `DerivedFact` evidence that
///   mentions the requested topic.
/// - `rule_id -> predicate` must match the hand-coded reasoner schema.
/// - `LivesIn` / `GoesTo` require a place-like object.
/// - `After` requires time-like subject and object.
pub fn audit_graph_admissibility(trace: &TurnTrace) -> GraphAdmissibilityReport {
    let mut issues = Vec::new();

    for result in trace.tool_calls.iter().filter(|result| result.success) {
        match &result.call {
            ToolCall::SearchGraph { subject, predicate } => {
                for evidence in &result.evidence {
                    match evidence {
                        ToolEvidence::GraphFact {
                            subject: fact_subject,
                            predicate: fact_predicate,
                            object,
                            ..
                        } => {
                            if fact_subject != subject {
                                issues.push(GraphAdmissibilityIssue::SearchGraphSubjectMismatch);
                            }
                            if let Some(expected) = predicate {
                                if !predicate_name_matches(*fact_predicate, expected) {
                                    issues.push(
                                        GraphAdmissibilityIssue::SearchGraphPredicateMismatch,
                                    );
                                }
                            }
                            append_ontology_issues(
                                validate_fact(fact_subject, *fact_predicate, object),
                                &mut issues,
                            );
                        }
                        _ => {
                            issues.push(
                                GraphAdmissibilityIssue::SearchGraphReturnedWrongEvidenceType,
                            );
                        }
                    }
                }
            }
            ToolCall::RunLocalReasoner { topic, .. } => {
                for evidence in &result.evidence {
                    match evidence {
                        ToolEvidence::DerivedFact {
                            subject,
                            predicate,
                            object,
                            rule_id,
                            support_chain,
                            ..
                        } => {
                            if subject != topic && object != topic {
                                issues.push(GraphAdmissibilityIssue::DerivedFactTopicMismatch);
                            }
                            append_ontology_issues(
                                validate_derived_fact(rule_id, subject, *predicate, object),
                                &mut issues,
                            );
                            if support_chain.is_empty() {
                                issues
                                    .push(GraphAdmissibilityIssue::DerivedFactMissingSupportChain);
                            }
                            for support in support_chain {
                                if validate_fact(
                                    &support.subject,
                                    support.predicate,
                                    &support.object,
                                )
                                .is_err()
                                {
                                    issues.push(GraphAdmissibilityIssue::SupportingFactInvalid);
                                }
                            }
                            if !support_chain_matches(rule_id, subject, object, support_chain) {
                                issues.push(
                                    GraphAdmissibilityIssue::DerivedFactSupportPatternMismatch,
                                );
                            }
                        }
                        _ => {
                            issues.push(GraphAdmissibilityIssue::ReasonerReturnedWrongEvidenceType);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    GraphAdmissibilityReport { issues }
}

fn leaked_injected_evidence(output_lower: &str, intent: &Intent) -> bool {
    match intent {
        Intent::Unknown {
            example,
            grounded_fact,
            reasoning_chain,
            ..
        } => [example, grounded_fact, reasoning_chain]
            .into_iter()
            .flatten()
            .any(|value| output_lower.contains(&value.to_lowercase())),
        _ => false,
    }
}

fn expected_direct_answer_value(trace: &TurnTrace) -> Option<String> {
    let predicate = match trace.intent_after_verification {
        Intent::AskName => "name",
        Intent::AskAge => "age",
        Intent::AskLocation => "city",
        Intent::AskOccupation => "occupation",
        _ => return None,
    };
    trace
        .belief_snapshot
        .active_fact(USER_SELF_KEY, predicate)
        .map(|fact| fact.object.clone())
        .or_else(|| trace.session_snapshot.get(predicate).cloned())
}

fn expected_evidence_text(intent: &Intent) -> Option<String> {
    match intent {
        Intent::Unknown {
            grounded_fact: Some(fact),
            ..
        } => Some(fact.clone()),
        Intent::Unknown {
            example: Some(example),
            ..
        } => Some(example.clone()),
        _ => None,
    }
}

fn latest_conflict_values(trace: &TurnTrace) -> Option<(String, String)> {
    let conflict = trace.belief_snapshot.contradictions.last()?;
    let old_value = trace
        .belief_snapshot
        .facts
        .get(conflict.fact_a_index)?
        .object
        .clone();
    let new_value = trace
        .belief_snapshot
        .facts
        .get(conflict.fact_b_index)?
        .object
        .clone();
    Some((old_value, new_value))
}

fn contains_latin(value: &str) -> bool {
    value.chars().any(|ch| ch.is_ascii_alphabetic())
}

fn matching_graph_fact<'a>(trace: &'a TurnTrace, text: &str) -> Option<&'a ToolEvidence> {
    trace
        .tool_calls
        .iter()
        .filter(|result| result.success)
        .flat_map(|result| result.evidence.iter())
        .find(|evidence| {
            matches!(
                evidence,
                ToolEvidence::GraphFact { rendered, .. } if rendered == text
            )
        })
}

fn has_any_retrieval_sample(trace: &TurnTrace) -> bool {
    trace
        .tool_calls
        .iter()
        .filter(|result| result.success)
        .flat_map(|result| result.evidence.iter())
        .any(|evidence| matches!(evidence, ToolEvidence::RetrievalSample { .. }))
}

fn has_retrieval_sample(trace: &TurnTrace, text: &str) -> bool {
    trace
        .tool_calls
        .iter()
        .filter(|result| result.success)
        .flat_map(|result| result.evidence.iter())
        .any(|evidence| {
            matches!(
                evidence,
                ToolEvidence::RetrievalSample { text: sample } if sample == text
            )
        })
}

fn matching_derived_fact<'a>(trace: &'a TurnTrace, text: &str) -> Option<&'a ToolEvidence> {
    trace
        .tool_calls
        .iter()
        .filter(|result| result.success)
        .flat_map(|result| result.evidence.iter())
        .find(|evidence| {
            matches!(
                evidence,
                ToolEvidence::DerivedFact { rendered, .. } if rendered == text
            )
        })
}

fn predicate_name_matches(predicate: ReasPredicate, needle: &str) -> bool {
    predicate.as_str().replace('_', "") == needle.to_lowercase().replace('_', "")
}

fn support_chain_matches(
    rule_id: &str,
    subject: &str,
    object: &str,
    support_chain: &[crate::tool::SupportFactEvidence],
) -> bool {
    if support_chain.len() == 1 {
        return true;
    }
    let [first, second] = support_chain else {
        return false;
    };
    match rule_id {
        "R1_is_a_transitivity" => {
            first.predicate == ReasPredicate::IsA
                && second.predicate == ReasPredicate::IsA
                && first.object == second.subject
                && subject == first.subject
                && object == second.object
        }
        "R2_has_inheritance" => {
            first.predicate == ReasPredicate::IsA
                && second.predicate == ReasPredicate::Has
                && first.object == second.subject
                && subject == first.subject
                && object == second.object
        }
        "R3_has_inheritance_via_part_of" => {
            first.predicate == ReasPredicate::Has
                && second.predicate == ReasPredicate::PartOf
                && first.object == second.subject
                && subject == first.subject
                && object == second.object
        }
        "R5_shared_is_a_target" => {
            first.predicate == ReasPredicate::IsA
                && second.predicate == ReasPredicate::IsA
                && first.object == second.object
                && canonical_pair(&first.subject, &second.subject)
                    == (subject.to_string(), object.to_string())
        }
        "R6_lives_in_via_part_of" => {
            first.predicate == ReasPredicate::LivesIn
                && second.predicate == ReasPredicate::PartOf
                && first.object == second.subject
                && subject == first.subject
                && object == second.object
        }
        "R7_goes_to_via_part_of" => {
            first.predicate == ReasPredicate::GoesTo
                && second.predicate == ReasPredicate::PartOf
                && first.object == second.subject
                && subject == first.subject
                && object == second.object
        }
        "R8_after_transitivity" => {
            first.predicate == ReasPredicate::After
                && second.predicate == ReasPredicate::After
                && first.object == second.subject
                && subject == first.subject
                && object == second.object
        }
        "R9_part_of_transitivity" => {
            first.predicate == ReasPredicate::PartOf
                && second.predicate == ReasPredicate::PartOf
                && first.object == second.subject
                && subject == first.subject
                && object == second.object
        }
        "R10_in_domain_inheritance" => {
            first.predicate == ReasPredicate::IsA
                && second.predicate == ReasPredicate::InDomain
                && first.object == second.subject
                && subject == first.subject
                && object == second.object
        }
        "R11_in_domain_shared_target" => {
            first.predicate == ReasPredicate::InDomain
                && second.predicate == ReasPredicate::InDomain
                && first.object == second.object
                && canonical_pair(&first.subject, &second.subject)
                    == (subject.to_string(), object.to_string())
        }
        _ => false,
    }
}

fn canonical_pair(a: &str, b: &str) -> (String, String) {
    if a <= b {
        (a.to_string(), b.to_string())
    } else {
        (b.to_string(), a.to_string())
    }
}

fn append_ontology_issues(
    result: Result<(), ontology::OntologyIssue>,
    issues: &mut Vec<GraphAdmissibilityIssue>,
) {
    match result {
        Ok(()) => {}
        Err(ontology::OntologyIssue::RulePredicateMismatch { .. }) => {
            issues.push(GraphAdmissibilityIssue::DerivedFactRulePredicateMismatch);
        }
        Err(ontology::OntologyIssue::PlaceObjectRequired { .. }) => {
            issues.push(GraphAdmissibilityIssue::PlacePredicateRequiresPlaceObject);
        }
        Err(ontology::OntologyIssue::TimeLikeRequired { .. }) => {
            issues.push(GraphAdmissibilityIssue::AfterPredicateRequiresTimeLikeEntities);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_kazakh_output_passes() {
        let report = audit_response("сіз Алматыда тұрасыз");
        assert!(report.is_clean(), "unexpected issues: {:?}", report.issues);
    }

    #[test]
    fn empty_output_fails() {
        let report = audit_response("   ");
        assert_eq!(report.issues, vec![ResponseQualityIssue::EmptyOutput]);
    }

    #[test]
    fn placeholder_leak_fails() {
        let report = audit_response("сіздің атыңыз {name}");
        assert!(
            report
                .issues
                .contains(&ResponseQualityIssue::PlaceholderLeak)
        );
    }

    #[test]
    fn latin_debug_artifact_fails() {
        let report = audit_response("planner: city=Алматы");
        assert!(
            report
                .issues
                .contains(&ResponseQualityIssue::LatinCharactersForbidden)
        );
    }

    #[test]
    fn repeated_whitespace_fails() {
        let report = audit_response("сіз  Алматыда тұрасыз");
        assert!(
            report
                .issues
                .contains(&ResponseQualityIssue::RepeatedWhitespace)
        );
    }
}
