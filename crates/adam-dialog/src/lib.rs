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
pub mod dialog_context;
pub mod discourse;
pub mod domain_index;
pub mod intent;
pub mod language_core;
pub mod planner;
pub mod quality;
pub mod question_shape;
pub mod realiser;
pub mod semantics;
pub mod sentence_decomp;
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
pub use dialog_context::{DialogContext, TopicMention};
pub use domain_index::DomainIndex;
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
pub use question_shape::{QuestionShape, detect as detect_question_shape};
pub use realiser::realise;
pub use semantics::{interpret, interpret_text, interpret_text_with_lexicon};
pub use sentence_decomp::{
    Role, SentenceDecomposition, SentenceType, TokenRole, decompose as decompose_sentence,
};
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
    parse_input_inner(input, lexicon, None)
}

/// **v4.15.5** — priors-aware variant of `parse_input_public`.
/// Sorts each token's candidate analyses by `P(chain)` DESC before
/// picking the first. Falls back to v3.2.0 lexicographic order
/// when `priors` is `None` (caller doesn't have a trained artifact)
/// or when two parses tie on chain probability — the v3.2.0
/// determinism contract is preserved exactly in those cases.
pub(crate) fn parse_input_with_priors(
    input: &str,
    lexicon: &adam_kernel_fst::lexicon::LexiconV1,
    priors: Option<&adam_kernel_fst::suffix_priors::SuffixPriors>,
) -> Vec<adam_kernel_fst::parser::Analysis> {
    parse_input_inner(input, lexicon, priors)
}

/// Layer 1 wrapper: parse each whitespace-separated token, keep only the
/// first (highest-confidence) analysis of each.
///
/// **v4.15.5** — when `priors` is `Some`, each token's parse list
/// is sorted by `P(chain)` DESC before taking the first. The sort
/// is **stable**: parses with equal scores retain v3.2.0
/// `(root, id)` order, so empty / no-priors callers see no change.
/// `respond` / `respond_with_repo` keep calling this with `priors:
/// None` so their pre-v4.15.5 behaviour is bit-identical.
fn parse_input(
    input: &str,
    lexicon: &adam_kernel_fst::lexicon::LexiconV1,
) -> Vec<adam_kernel_fst::parser::Analysis> {
    parse_input_inner(input, lexicon, None)
}

fn parse_input_inner(
    input: &str,
    lexicon: &adam_kernel_fst::lexicon::LexiconV1,
    priors: Option<&adam_kernel_fst::suffix_priors::SuffixPriors>,
) -> Vec<adam_kernel_fst::parser::Analysis> {
    use adam_kernel_fst::parser::Analysis;
    use adam_kernel_fst::suffix_priors::{noun_chain_key, verb_chain_key};

    let mut out = Vec::new();
    // **v4.16.0** — track the previous token's selected chain key
    // for context-aware bigram re-ranking. Reset to `None` when the
    // FST returns no analyses (sentence boundary in effect).
    let mut prev_chain: Option<String> = None;
    for token in input.split_whitespace() {
        let cleaned: String = token
            .chars()
            .filter(|c| c.is_alphabetic() || *c == '-')
            .collect::<String>()
            .to_lowercase();
        if cleaned.is_empty() {
            continue;
        }
        let mut analyses = adam_kernel_fst::parser::analyse(&cleaned, lexicon);
        if let Some(p) = priors {
            // **v4.15.5 + v4.16.0** — re-rank parses by
            // P(chain | prev_chain) DESC when bigram context is
            // available; falls back to unigram when prev_chain is
            // None or unseen in the transition map. `sort_by` is
            // stable: ties preserve the v3.2.0 lexicographic
            // order, so empty / no-priors callers see no change.
            let prev = prev_chain.as_deref();
            analyses.sort_by(|a, b| {
                let score_a = score_analysis(a, p, prev);
                let score_b = score_analysis(b, p, prev);
                // Reverse: higher score wins.
                score_b
                    .partial_cmp(&score_a)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            // Helper to read a single analysis's chain score
            // under the (optional) bigram context.
            #[inline]
            fn score_analysis(
                a: &Analysis,
                p: &adam_kernel_fst::suffix_priors::SuffixPriors,
                prev: Option<&str>,
            ) -> f32 {
                match a {
                    Analysis::Noun { features, .. } => p.score_noun_given_prev(features, prev),
                    Analysis::Verb { features, .. } => p.score_verb_given_prev(features, prev),
                }
            }
        }
        // Update prev_chain to the chain key of the parse we
        // actually picked (the first one, post-sort). Done before
        // `out.push` so the closure-borrow-of-analyses dance is
        // compatible with the `into_iter().next()` consumer below.
        let chosen_key = analyses.first().map(|a| match a {
            Analysis::Noun { features, .. } => noun_chain_key(features),
            Analysis::Verb { features, .. } => verb_chain_key(features),
        });
        prev_chain = chosen_key;
        if let Some(a) = analyses.into_iter().next() {
            out.push(a);
        }
    }
    out
}
