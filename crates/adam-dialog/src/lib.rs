//! adam-dialog — predictable, auditable Kazakh dialog layer.
//!
//! **Stage: v3.9.5** — 26-intent recogniser + multi-turn session +
//! FST-backed slot expansion + session-aware retrieval composition +
//! rule-derived reasoning chains (v2.7+) + World Core integration
//! (v3.9.0+). Every path is deterministic or samples from a finite,
//! inspectable set.
//!
//! See [`docs/architecture_v3.md`](../../../docs/architecture_v3.md) for the
//! current canonical architecture; [`docs/kazakh_grammar/07_dialog_architecture.md`](../../../docs/kazakh_grammar/07_dialog_architecture.md)
//! is the original v1.0 MVP reference kept as a historical snapshot.
//!
//! Five-layer pipeline:
//!
//! 1. Morphological parser (`adam_kernel_fst::parser`)
//! 2. Semantic interpreter ([`semantics`]) — intent recognition +
//!    entity extraction + `NOT_A_TOPIC` closed-class filter (v3.9.5
//!    synced with `adam_reasoning::patterns::is_closed_class`)
//! 3. Dialog planner ([`planner`]) — template selection (v2.7+
//!    routes `Intent::Unknown` with `reasoning_chain: Some(...)` to
//!    the `unknown.with_derived_chain` family for «байланыс-» marked
//!    responses)
//! 4. Response realiser ([`realiser`])
//! 5. Morphological synthesiser (`adam_kernel_fst::morphotactics::synthesise_*`)
//!
//! The whole chain is pure-function except for [`planner::choose_template`],
//! which picks uniformly from ≤ 5 applicable templates for the recognised
//! intent. That is the ONLY source of randomness in the system.

pub mod action;
pub mod belief;
pub mod conversation;
pub mod intent;
pub mod language_core;
pub mod planner;
pub mod quality;
pub mod realiser;
pub mod semantics;
pub mod slot_syntax;
pub mod system_identity;
pub mod task;
pub mod templates;
pub mod tool;
pub mod uncertainty;
pub mod verifier;

pub use action::{Action, ActionDigest, ActionPlan, ActionPlanner, OutputKind};
pub use belief::{
    BeliefConflict, BeliefDigest, BeliefFact, BeliefState, ConfidenceBand, EntityKind,
    EntityMemory, FactStatus, PendingQuestion, Provenance, QuestionNature, USER_SELF_KEY,
};
pub use conversation::{ComposeMode, Conversation, IntentKind, TurnTrace};
pub use intent::{GreetingKind, Intent, SubjectPerson};
pub use language_core::{
    GeoEntity, PersonEntity, canonical_geo_entity, canonical_geo_id, canonical_person_entity,
    canonical_person_id, geo_entity_kind, looks_like_named_place_candidate, looks_like_person_name,
    normalize_proper_noun,
};
pub use planner::{
    ResponsePlan, intent_key, plan_response, plan_response_with_epistemic, plan_response_with_repo,
    plan_response_with_session,
};
pub use quality::{
    GraphAdmissibilityIssue, GraphAdmissibilityReport, ResponseQualityIssue, ResponseQualityReport,
    TraceFaithfulnessIssue, TraceFaithfulnessReport, TypedFaithfulnessIssue,
    TypedFaithfulnessReport, audit_graph_admissibility, audit_response, audit_trace_faithfulness,
    audit_typed_faithfulness,
};
pub use realiser::realise;
pub use semantics::{interpret, interpret_text, interpret_text_with_lexicon};
pub use system_identity::{SystemAspect, SystemIdentity};
pub use task::{Goal, Subgoal, TaskDigest, TaskState, TaskStatus};
pub use templates::{TemplateError, TemplateRepository};
pub use tool::{Tool, ToolCall, ToolContext, ToolEvidence, ToolResult};
pub use uncertainty::{EpistemicStatus, UncertaintyPolicy};
pub use verifier::{VerificationIssue, VerificationReport, Verifier, strip_evidence};

/// End-to-end entry point: Kazakh text in, Kazakh text out.
///
/// Uses the hardcoded-fallback template repository — convenient but
/// limited. Production uses should call [`respond_with_repo`] with a
/// `TemplateRepository` loaded from `data/dialog/templates/v1.toml`
/// for the full 10-intent template coverage.
pub fn respond(
    input: &str,
    lexicon: &adam_kernel_fst::lexicon::LexiconV1,
    rng_seed: u64,
) -> String {
    let parses = parse_input(input, lexicon);
    let intent = interpret_text_with_lexicon(input, &parses, Some(lexicon));
    let plan = plan_response(&intent, rng_seed);
    realise(&plan)
}

/// End-to-end entry point with an explicit template repository.
pub fn respond_with_repo(
    input: &str,
    lexicon: &adam_kernel_fst::lexicon::LexiconV1,
    repo: &TemplateRepository,
    rng_seed: u64,
) -> String {
    let parses = parse_input(input, lexicon);
    let intent = interpret_text_with_lexicon(input, &parses, Some(lexicon));
    let plan = plan_response_with_repo(&intent, rng_seed, repo);
    realise(&plan)
}

/// Crate-public alias so [`Conversation::turn`] can share the same
/// parser path without duplicating the token-cleaning logic. Not
/// intended for external callers — use [`respond`] / [`respond_with_repo`]
/// or the `Conversation` API instead.
pub(crate) fn parse_input_public(
    input: &str,
    lexicon: &adam_kernel_fst::lexicon::LexiconV1,
) -> Vec<adam_kernel_fst::parser::Analysis> {
    parse_input(input, lexicon)
}

/// Layer 1 wrapper: parse each whitespace-separated token, keep only the
/// first (highest-confidence) analysis of each. Full disambiguation is
/// a future refinement; for MVP we proceed with the single-best parse.
fn parse_input(
    input: &str,
    lexicon: &adam_kernel_fst::lexicon::LexiconV1,
) -> Vec<adam_kernel_fst::parser::Analysis> {
    let mut out = Vec::new();
    for token in input.split_whitespace() {
        let cleaned: String = token
            .chars()
            .filter(|c| c.is_alphabetic() || *c == '-')
            .collect::<String>()
            .to_lowercase();
        if cleaned.is_empty() {
            continue;
        }
        let analyses = adam_kernel_fst::parser::analyse(&cleaned, lexicon);
        if let Some(a) = analyses.into_iter().next() {
            out.push(a);
        }
    }
    out
}
