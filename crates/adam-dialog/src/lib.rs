//! adam-dialog — predictable, auditable Kazakh dialog layer.
//!
//! **Stage: v4.52.5** — 33-intent recogniser + multi-turn session +
//! FST-backed slot expansion + session-aware retrieval composition +
//! rule-derived reasoning chains (v2.7+) + World Core integration
//! (v3.9.0+) + user-activity slot extraction (v4.51.0+) +
//! transcript-driven detector extensions for continuous-form
//! activity, compound occupation, and math anaphora (v4.52.0+).
//! Every path is deterministic or samples from a finite,
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
pub mod cargo_verify;
pub mod conversation;
pub mod curriculum;
pub mod dialog_context;
pub mod discourse;
pub mod domain_index;
pub mod intent;
pub mod language_core;
pub mod nlg;
pub mod pedagogical;
pub mod planner;
pub mod quality;
pub mod question_shape;
pub mod realiser;
pub mod selection;
pub mod semantics;
pub mod sentence_decomp;
pub mod slot_syntax;
pub mod system_identity;
pub mod task;
pub mod templates;
pub mod tool;
pub mod topic_extraction;
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
    canonical_person_id, geo_entity_kind, kazakh_respectful_address,
    looks_like_named_place_candidate, looks_like_person_name, normalize_proper_noun,
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
pub use selection::{
    AuditAggregate, AuditResult, CandidateFeatures, HarvestReport, SelectionWeights,
    TrainingConfig, TrainingPair, TrainingStats, audit_compare, canonical_training_pairs_v0,
    evaluate_weights_on_pairs, extract_features, harvest_audit_traces,
    repl_derived_training_pairs_v0, score as selection_score, select_top, train_perceptron,
    trained_v0,
};
pub use semantics::{interpret, interpret_text, interpret_text_with_lexicon};
pub use sentence_decomp::{
    Role, SentenceDecomposition, SentenceType, TokenRole, decompose as decompose_sentence,
};
pub use system_identity::{SystemAspect, SystemIdentity};
pub use task::{Goal, Subgoal, TaskDigest, TaskState, TaskStatus};
pub use templates::{TemplateError, TemplateRepository};
pub use tool::{Tool, ToolCall, ToolContext, ToolEvidence, ToolResult};
// **v4.24.0** — `content_roots` was extracted from `semantics` into
// the new `topic_extraction` module as part of the Codex-review-driven
// decomposition. Re-export it here so external callers
// (`adam_chat`, `Conversation::turn_with_trace`) keep their existing
// `adam_dialog::content_roots` imports working.
pub use topic_extraction::content_roots;
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

/// **v4.15.5** — priors-aware parse path used by
/// `Conversation::turn_with_trace`. Sorts each token's candidate
/// analyses by `P(chain)` DESC before picking the first. Falls
/// back to v3.2.0 lexicographic order when `priors` is `None`
/// (caller has no trained artifact) or when two parses tie on
/// chain probability — the v3.2.0 determinism contract is
/// preserved exactly in those cases.
///
/// Replaced the v4.14.5 `parse_input_public` (removed in v4.17.0
/// after dead-code analysis confirmed no remaining callers).
///
/// **v4.16.5** — accepts an optional `alpha` interpolation weight
/// for Jelinek-Mercer smoothing. `None` = pure bigram with unigram
/// fallback (v4.16.0 behaviour); `Some(α)` interpolates
/// `α·log P(curr) + (1-α)·log P(curr|prev)`.
pub(crate) fn parse_input_with_priors(
    input: &str,
    lexicon: &adam_kernel_fst::lexicon::LexiconV1,
    priors: Option<&adam_kernel_fst::suffix_priors::SuffixPriors>,
    alpha: Option<f32>,
) -> Vec<adam_kernel_fst::parser::Analysis> {
    parse_input_inner(input, lexicon, priors, alpha)
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
    parse_input_inner(input, lexicon, None, None)
}

fn parse_input_inner(
    input: &str,
    lexicon: &adam_kernel_fst::lexicon::LexiconV1,
    priors: Option<&adam_kernel_fst::suffix_priors::SuffixPriors>,
    alpha: Option<f32>,
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
            // None or unseen in the transition map.
            //
            // **v4.22.0** — when chain scores tie within EPSILON,
            // break the tie by `score_root` (root marginal under
            // unambiguous-only attribution + closed-class boost,
            // both v4.20.0 / v4.20.5). Closes the chain-collision
            // case surfaced by v4.19.5 — for a surface like
            // «онда» where both `он + Loc` and `ол + Loc` have
            // identical chains, the root prior decides correctly
            // (P(ол) > P(он) under the closed-class boost). When
            // the FST also exposes the gold parse via the v4.21.0
            // pronoun paradigm matcher, this lifts live-dialog
            // accuracy on chain-collision cases from chain-only's
            // 91.3 % to 100 % on the curated test set.
            //
            // `sort_by` is stable: chain ties WITH equal root
            // scores preserve the v3.2.0 lexicographic order, so
            // empty / no-priors callers see no change.
            const CHAIN_TIE_EPSILON: f32 = 1e-4;
            let prev = prev_chain.as_deref();
            analyses.sort_by(|a, b| {
                let chain_a = score_analysis(a, p, prev, alpha);
                let chain_b = score_analysis(b, p, prev, alpha);
                let chain_diff = (chain_a - chain_b).abs();
                if chain_diff < CHAIN_TIE_EPSILON {
                    let root_a = p.score_root(root_of(a));
                    let root_b = p.score_root(root_of(b));
                    // Reverse: higher root score wins.
                    root_b
                        .partial_cmp(&root_a)
                        .unwrap_or(std::cmp::Ordering::Equal)
                } else {
                    // Reverse: higher chain score wins.
                    chain_b
                        .partial_cmp(&chain_a)
                        .unwrap_or(std::cmp::Ordering::Equal)
                }
            });
            // Helper to read a single analysis's chain score
            // under the (optional) bigram context.
            //
            // **v4.16.5** — when `alpha` is `Some`, uses
            // Jelinek-Mercer smoothed scoring; `None` falls
            // through to the v4.16.0 pure-bigram-with-fallback
            // path.
            #[inline]
            fn score_analysis(
                a: &Analysis,
                p: &adam_kernel_fst::suffix_priors::SuffixPriors,
                prev: Option<&str>,
                alpha: Option<f32>,
            ) -> f32 {
                match (a, alpha) {
                    (Analysis::Noun { features, .. }, Some(a)) => {
                        p.score_noun_smoothed(features, prev, a)
                    }
                    (Analysis::Verb { features, .. }, Some(a)) => {
                        p.score_verb_smoothed(features, prev, a)
                    }
                    (Analysis::Noun { features, .. }, None) => {
                        p.score_noun_given_prev(features, prev)
                    }
                    (Analysis::Verb { features, .. }, None) => {
                        p.score_verb_given_prev(features, prev)
                    }
                }
            }
            // Helper to extract the root surface from an Analysis
            // for the v4.22.0 root-tiebreaker.
            #[inline]
            fn root_of(a: &Analysis) -> &str {
                match a {
                    Analysis::Noun { root, .. } | Analysis::Verb { root, .. } => &root.root,
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

#[cfg(test)]
mod runtime_priors_tests {
    //! **v4.22.0** — runtime regression locks for the
    //! chain_tiebreak_root strategy now wired into
    //! [`parse_input_with_priors`]. Asserts that the production
    //! parse path picks the gold pronoun root for surfaces where
    //! the chain prior alone ties (онда / маған / соған etc.) —
    //! the live-dialog manifestation of the parse-disambiguation
    //! eval's 100 % closure.
    //!
    //! Tests load the real frozen artifact + lexicon. Skipped
    //! when those files aren't reachable from the cargo test
    //! working directory (e.g. CI without the data tree).
    use super::*;
    use adam_kernel_fst::lexicon::LexiconV1;
    use adam_kernel_fst::parser::Analysis;
    use adam_kernel_fst::suffix_priors::SuffixPriors;
    use std::path::Path;

    fn load_real() -> Option<(LexiconV1, SuffixPriors)> {
        let curated = "../../data/tokenizer/segmentation_roots.json";
        let apertium = "../../data/lexicon_v1/apertium_imported_roots.json";
        let priors_path = "../../data/retrieval/suffix_chain_priors.json";
        if !Path::new(curated).exists() || !Path::new(priors_path).exists() {
            eprintln!("data files not present; skipping runtime priors test");
            return None;
        }
        let lex = LexiconV1::load(curated, apertium).ok()?;
        let priors = SuffixPriors::load(priors_path).ok()?;
        Some((lex, priors))
    }

    fn root_of_first(parses: &[Analysis]) -> Option<&str> {
        parses.first().map(|a| match a {
            Analysis::Noun { root, .. } | Analysis::Verb { root, .. } => root.root.as_str(),
        })
    }

    #[test]
    fn onda_resolves_to_ol_under_runtime_priors() {
        let Some((lex, priors)) = load_real() else {
            return;
        };
        // Sentence-initial position — prev_chain is None, so the
        // chain prior alone gives он+Loc and ол+Loc identical
        // scores. The v4.22.0 root tiebreaker decides.
        let parses = parse_input_with_priors("онда", &lex, Some(&priors), Some(0.3));
        assert_eq!(
            root_of_first(&parses),
            Some("ол"),
            "runtime parse of «онда» should pick ол (anaphoric pronoun) \
             over он (digit ten) via the v4.22.0 chain_tiebreak_root path"
        );
    }

    #[test]
    fn magan_resolves_to_men_under_runtime_priors() {
        let Some((lex, priors)) = load_real() else {
            return;
        };
        let parses = parse_input_with_priors("маған", &lex, Some(&priors), Some(0.3));
        assert_eq!(
            root_of_first(&parses),
            Some("мен"),
            "runtime parse of «маған» should pick мен (1sg pronoun, Dative)"
        );
    }

    #[test]
    fn sagan_resolves_to_sen_under_runtime_priors() {
        let Some((lex, priors)) = load_real() else {
            return;
        };
        let parses = parse_input_with_priors("саған", &lex, Some(&priors), Some(0.3));
        assert_eq!(
            root_of_first(&parses),
            Some("сен"),
            "runtime parse of «саған» should pick сен (2sg-informal pronoun, Dative)"
        );
    }

    #[test]
    fn no_priors_path_unchanged_for_ambiguous_surface() {
        // Sanity check: when no priors are attached, the v3.2.0
        // deterministic order is preserved bit-for-bit (no priors
        // → no re-rank → no tiebreaker). v4.22.0 is strictly
        // additive on the priors path.
        let Some((lex, _priors)) = load_real() else {
            return;
        };
        let with_priors = parse_input_with_priors("онда", &lex, None, None);
        let without_priors = parse_input("онда", &lex);
        assert_eq!(
            with_priors, without_priors,
            "no-priors path must match the legacy parse_input output exactly"
        );
    }
}
