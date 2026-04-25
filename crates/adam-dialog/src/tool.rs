//! Layer 3.11 — controlled tool layer, introduced in v4.0.37.
//!
//! Codex v4.0.26 roadmap "Phase 6". Pre-v4.0.37 the dialog reached
//! into [`crate::belief::BeliefState`], `extracted_facts`,
//! [`adam_retrieval::MorphemeIndex`], and `derived_facts` directly
//! from `inject_retrieval_example` / `inject_reasoning_chain` /
//! `ActionPlanner`. Each lookup was a one-off function call —
//! convenient for the v4.0.31 substrate but invisible to the trace
//! and impossible for the planner to *intend* a tool call as a
//! distinct action.
//!
//! Phase 6 introduces a uniform tool interface so:
//!
//! 1. Every internal lookup goes through one named, traced channel.
//! 2. The `ActionPlanner` (Phase 7+) can return a list of `ToolCall`s
//!    to execute, instead of inlining the lookup at the call site.
//! 3. New tools (calculator, calendar, external API, future learned
//!    rerankers) plug in without touching every consumer.
//!
//! **v4.0.37 scope** — substrate **only**. Defines types + a pure
//! dispatcher with **two** tools fully implemented (`SearchBelief`,
//! `SearchGraph`); `SearchRetrieval` and `RunLocalReasoner` are
//! reserved variants with stubs that return `success=false` so the
//! dispatcher never panics. Conversation::turn_with_trace doesn't
//! yet auto-dispatch — `tool_calls: Vec<ToolResult>` on `TurnTrace`
//! stays empty unless a caller explicitly calls `Tool::dispatch`.
//! v4.0.38 (Phase 6 part 2) will wire `ActionPlanner` so the
//! existing `inject_*` helpers go through the tool layer.
//!
//! Splits Phase 6 across two releases following the same pattern as
//! Phase 1 / 2 / 5: substrate first, behavior second, each
//! Codex-reviewable independently.

use crate::belief::{BeliefState, FactStatus};
use adam_reasoning::Fact as ReasFact;
use adam_reasoning::reasoner::DerivedFact;
use adam_retrieval::MorphemeIndex;

/// One callable internal tool. Each variant carries the inputs the
/// dispatcher needs; results come back through [`ToolResult`].
///
/// Naming follows the Codex v4.0.26 roadmap proposal verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolCall {
    /// Look up active belief facts by subject (e.g. user name,
    /// city, occupation).
    ///
    /// **v4.1.5** — accepts an optional predicate filter so the
    /// `ActionPlanner::belief_direct_answer` lookup can route through
    /// the tool layer instead of bypassing it. Two output shapes:
    /// - `predicate: None`  → audit-friendly: every active fact for
    ///   `subject` rendered as `"{subject} {predicate} {object}"`
    ///   (preserves the v4.0.37 contract).
    /// - `predicate: Some(p)` → typed-lookup-friendly: 0 or 1
    ///   findings (single-active-fact invariant, v4.0.28), each
    ///   finding is the **object string only** so the caller can
    ///   use it as a slot value without re-parsing.
    SearchBelief {
        subject: String,
        predicate: Option<String>,
    },
    /// Look up extracted (base) facts by subject and optional
    /// predicate. Proxies for "search the lexical graph" — the
    /// graph index itself isn't yet exposed, so we filter the flat
    /// `extracted_facts` Vec instead. v4.0.37 — fully implemented.
    SearchGraph {
        subject: String,
        predicate: Option<String>,
    },
    /// Corpus retrieval via [`adam_retrieval::MorphemeIndex::rank`].
    /// v4.0.38 — fully implemented; takes the morpheme list the
    /// caller would have built for `inject_retrieval_example` and
    /// returns the top-k surface texts as `findings`. When no
    /// `MorphemeIndex` is attached to the context, returns
    /// `success=false` with an explicit reason.
    SearchRetrieval { morphemes: Vec<String> },
    /// Find an existing rule-derived chain involving the topic.
    ///
    /// **v4.1.2** — full picker. Scans `derived_facts` for matches
    /// (subject or object), filters by `curated_only` (when `true`,
    /// only fully-curated derivations pass — same gate as
    /// `Conversation::curated_only_reasoning`), scores each
    /// candidate via `score_derivation`, breaks ties on IsA-chain
    /// depth (closer parent wins) then on canonical-triple ordering
    /// (deterministic), renders the top match via
    /// `render_derivation_as_kazakh`, and returns it as a single
    /// finding. Drives `Conversation::inject_reasoning_chain` —
    /// reply-text identical to the pre-v4.1.2 direct-scan path.
    ///
    /// Pre-v4.1.2 this tool returned the top 3 raw triples for
    /// audit only; the `inject_reasoning_chain` helper did its own
    /// pick + render. The two paths could disagree under tie-breaks
    /// because the audit dispatch had no IsA-depth knowledge. v4.1.2
    /// makes the tool dispatch authoritative for reasoning-chain
    /// resolution.
    RunLocalReasoner {
        topic: String,
        /// **v4.1.2** — when `true`, only derivations whose every
        /// `source_chain` entry is rooted in `world_core/` qualify
        /// (mirrors `derivation_is_fully_curated`). When `false`,
        /// any derivation involving the topic is in scope.
        curated_only: bool,
    },
}

/// What the tool returned. `findings` are short opaque strings the
/// caller can render or further process; `trace` is a per-call audit
/// log mirroring `ResponsePlan.trace`. `success` is the binary
/// outcome — useful for the `ActionPlanner` to decide whether to
/// fall back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResult {
    pub call: ToolCall,
    pub success: bool,
    pub findings: Vec<String>,
    pub trace: Vec<String>,
}

impl ToolResult {
    fn ok(call: ToolCall, findings: Vec<String>, trace: Vec<String>) -> Self {
        Self {
            call,
            success: true,
            findings,
            trace,
        }
    }

    /// Dispatch ran successfully, but the tool found nothing —
    /// e.g. `SearchBelief` on a subject with no Active facts, or
    /// `RunLocalReasoner` on a topic that has no derivations.
    /// Distinct from [`unsupported`](Self::unsupported), which means
    /// the tool *couldn't* run for lack of context.
    fn empty(call: ToolCall, reason: &str) -> Self {
        Self {
            call,
            success: false,
            findings: Vec::new(),
            trace: vec![reason.to_string()],
        }
    }

    /// Dispatch couldn't run — the `ToolContext` lacks the store
    /// the tool needs (e.g. `SearchRetrieval` with no
    /// `MorphemeIndex`). Distinct from [`empty`](Self::empty),
    /// which means the tool ran fine and produced no findings.
    fn unsupported(call: ToolCall, why: &str) -> Self {
        Self {
            call,
            success: false,
            findings: Vec::new(),
            trace: vec![why.to_string()],
        }
    }
}

/// v4.0.38 — bundle of read-only references the dispatcher needs.
/// Adding a tool that needs a new store (e.g. retrieval ranker
/// config, future calculator state) means adding a field here, not
/// changing the `Tool::dispatch` signature.
pub struct ToolContext<'a> {
    pub belief: &'a BeliefState,
    pub extracted: &'a [ReasFact],
    pub derived: &'a [DerivedFact],
    pub retrieval: Option<&'a MorphemeIndex>,
    /// **v4.1.1** — caller-supplied retrieval ranker config. `None`
    /// means "use [`adam_retrieval::RankConfig::default`]". Threaded
    /// through `ToolContext` rather than the `ToolCall::SearchRetrieval`
    /// payload because `RankConfig` is a sizeable struct (weights +
    /// per-pack purity prior map) and would otherwise be cloned into
    /// every tool call. Required for `inject_retrieval_example` to
    /// route through `Tool::dispatch(SearchRetrieval)` while still
    /// honouring `Conversation::rank_config` (the per-conversation
    /// override).
    pub rank_config: Option<&'a adam_retrieval::RankConfig>,
}

/// Pure-function dispatcher. Reads `ToolContext` references, never
/// mutates them. Future tools that need write access must take a
/// `&mut` field explicitly.
pub struct Tool;

impl Tool {
    /// Execute a single `ToolCall` against the bundled context.
    pub fn dispatch(call: ToolCall, ctx: &ToolContext) -> ToolResult {
        let mut trace = Vec::new();
        trace.push(format!("tool: dispatch {call:?}"));
        match &call {
            ToolCall::SearchBelief { subject, predicate } => {
                let active: Vec<&crate::belief::BeliefFact> = ctx
                    .belief
                    .facts
                    .iter()
                    .filter(|f| f.subject == *subject && f.status == FactStatus::Active)
                    .filter(|f| match predicate {
                        Some(p) => f.predicate == *p,
                        None => true,
                    })
                    .collect();
                trace.push(format!(
                    "search_belief: subject={subject} predicate={:?} active_matches={}",
                    predicate,
                    active.len()
                ));
                if active.is_empty() {
                    return ToolResult::empty(call, "search_belief: no active facts");
                }
                let findings: Vec<String> = match predicate {
                    Some(_) => active.iter().map(|f| f.object.clone()).collect(),
                    None => active
                        .iter()
                        .map(|f| format!("{} {} {}", f.subject, f.predicate, f.object))
                        .collect(),
                };
                ToolResult::ok(call, findings, trace)
            }
            ToolCall::SearchGraph { subject, predicate } => {
                let matches: Vec<String> = ctx
                    .extracted
                    .iter()
                    .filter(|f| f.subject.root == *subject)
                    .filter(|f| match predicate {
                        Some(p) => format!("{:?}", f.predicate).to_lowercase() == p.to_lowercase(),
                        None => true,
                    })
                    .map(|f| format!("{} {:?} {}", f.subject.root, f.predicate, f.object.root))
                    .collect();
                trace.push(format!(
                    "search_graph: subject={subject} predicate={predicate:?} matches={}",
                    matches.len()
                ));
                if matches.is_empty() {
                    ToolResult::empty(call, "search_graph: no matching facts")
                } else {
                    ToolResult::ok(call, matches, trace)
                }
            }
            ToolCall::SearchRetrieval { morphemes } => {
                let Some(index) = ctx.retrieval else {
                    return ToolResult::unsupported(
                        call,
                        "search_retrieval: no MorphemeIndex attached to context",
                    );
                };
                let refs: Vec<&str> = morphemes.iter().map(|s| s.as_str()).collect();
                let default_cfg;
                let cfg = match ctx.rank_config {
                    Some(c) => c,
                    None => {
                        default_cfg = adam_retrieval::RankConfig::default();
                        &default_cfg
                    }
                };
                let hits = index.rank(&refs, cfg);
                let top: Vec<String> = hits
                    .iter()
                    .take(3)
                    .filter_map(|h| index.sample_text(&h.sref).map(String::from))
                    .collect();
                trace.push(format!(
                    "search_retrieval: morphemes={} hits={}",
                    morphemes.len(),
                    hits.len()
                ));
                if top.is_empty() {
                    ToolResult::empty(call, "search_retrieval: no hits for given morphemes")
                } else {
                    ToolResult::ok(call, top, trace)
                }
            }
            ToolCall::RunLocalReasoner {
                topic,
                curated_only,
            } => {
                let passes_safety = |d: &&DerivedFact| -> bool {
                    !curated_only || adam_reasoning::reasoner::derivation_is_fully_curated(d)
                };
                let candidates: Vec<&DerivedFact> = ctx
                    .derived
                    .iter()
                    .filter(|d| {
                        (d.subject.root == *topic || d.object.root == *topic) && passes_safety(d)
                    })
                    .collect();
                trace.push(format!(
                    "run_local_reasoner: topic={topic} curated_only={curated_only} candidates={}",
                    candidates.len()
                ));
                let picked = candidates.iter().copied().max_by(|a, b| {
                    crate::conversation::score_derivation(a, topic)
                        .cmp(&crate::conversation::score_derivation(b, topic))
                        .then_with(|| {
                            if a.predicate == adam_reasoning::Predicate::IsA
                                && b.predicate == adam_reasoning::Predicate::IsA
                            {
                                let da = crate::conversation::isa_chain_depth(
                                    ctx.extracted,
                                    &a.subject.root,
                                    &a.object.root,
                                );
                                let db = crate::conversation::isa_chain_depth(
                                    ctx.extracted,
                                    &b.subject.root,
                                    &b.object.root,
                                );
                                da.cmp(&db).reverse()
                            } else {
                                std::cmp::Ordering::Equal
                            }
                        })
                        .then_with(|| {
                            (
                                a.subject.root.as_str(),
                                a.predicate.as_str(),
                                a.object.root.as_str(),
                            )
                                .cmp(&(
                                    b.subject.root.as_str(),
                                    b.predicate.as_str(),
                                    b.object.root.as_str(),
                                ))
                                .reverse()
                        })
                });
                match picked {
                    None => {
                        ToolResult::empty(call, "run_local_reasoner: no derivation found for topic")
                    }
                    Some(d) => {
                        let rendered = crate::conversation::render_derivation_as_kazakh(d);
                        ToolResult::ok(call, vec![rendered], trace)
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::belief::{BeliefState, USER_SELF_KEY};
    use adam_reasoning::{ConfidenceKind, FactSource, Predicate, SlotRef};

    fn fact(subject: &str, pred: Predicate, object: &str) -> ReasFact {
        ReasFact {
            subject: SlotRef {
                surface: subject.into(),
                root: subject.into(),
                pos: "noun".into(),
            },
            predicate: pred,
            object: SlotRef {
                surface: object.into(),
                root: object.into(),
                pos: "noun".into(),
            },
            pattern: "test".into(),
            source: FactSource {
                pack: "test".into(),
                sample_id: "t1".into(),
            },
            confidence: ConfidenceKind::Grammar,
            raw_text: "test".into(),
        }
    }

    fn derived(subject: &str, pred: Predicate, object: &str) -> DerivedFact {
        DerivedFact {
            subject: SlotRef {
                surface: subject.into(),
                root: subject.into(),
                pos: "noun".into(),
            },
            predicate: pred,
            object: SlotRef {
                surface: object.into(),
                root: object.into(),
                pos: "noun".into(),
            },
            rule_id: "R1_is_a_transitivity".into(),
            source_chain: vec![FactSource {
                pack: "world_core/test.jsonl".into(),
                sample_id: "t1".into(),
            }],
            confidence: ConfidenceKind::RuleInferred,
        }
    }

    fn ctx<'a>(belief: &'a BeliefState, extracted: &'a [ReasFact]) -> ToolContext<'a> {
        ToolContext {
            belief,
            extracted,
            derived: &[],
            retrieval: None,
            rank_config: None,
        }
    }

    #[test]
    fn search_belief_finds_active_fact() {
        let mut belief = BeliefState::new();
        belief.record_user_fact(USER_SELF_KEY, "name", "Дәулет", 0);
        let r = Tool::dispatch(
            ToolCall::SearchBelief {
                subject: USER_SELF_KEY.into(),
                predicate: None,
            },
            &ctx(&belief, &[]),
        );
        assert!(r.success);
        assert_eq!(r.findings.len(), 1);
        assert!(r.findings[0].contains("Дәулет"));
    }

    #[test]
    fn search_belief_empty_on_no_match() {
        let belief = BeliefState::new();
        let r = Tool::dispatch(
            ToolCall::SearchBelief {
                subject: "nonexistent".into(),
                predicate: None,
            },
            &ctx(&belief, &[]),
        );
        assert!(!r.success);
        assert!(r.findings.is_empty());
    }

    #[test]
    fn search_belief_skips_contested_facts() {
        let mut belief = BeliefState::new();
        belief.record_user_fact(USER_SELF_KEY, "city", "алматы", 0);
        belief.record_user_fact(USER_SELF_KEY, "city", "астана", 1);
        let r = Tool::dispatch(
            ToolCall::SearchBelief {
                subject: USER_SELF_KEY.into(),
                predicate: None,
            },
            &ctx(&belief, &[]),
        );
        assert!(!r.success, "contested facts must not surface as Active");
    }

    /// **v4.1.5** — when `predicate` is `Some(p)`, findings are the
    /// raw object strings only (no `subject {predicate} object`
    /// triple) so callers like `ActionPlanner::belief_direct_answer`
    /// can use the value directly as a slot fill.
    #[test]
    fn search_belief_with_predicate_returns_object_only() {
        let mut belief = BeliefState::new();
        belief.record_user_fact(USER_SELF_KEY, "name", "Дәулет", 0);
        belief.record_user_fact(USER_SELF_KEY, "city", "алматы", 1);
        let r = Tool::dispatch(
            ToolCall::SearchBelief {
                subject: USER_SELF_KEY.into(),
                predicate: Some("city".into()),
            },
            &ctx(&belief, &[]),
        );
        assert!(r.success);
        assert_eq!(r.findings, vec!["алматы".to_string()]);
    }

    /// **v4.1.5** — `predicate` filter respects single-active-fact
    /// invariant: at most one finding for a given `(subject, predicate)`
    /// because the same gate as `BeliefState::active_fact` applies.
    #[test]
    fn search_belief_with_predicate_returns_empty_on_no_active() {
        let mut belief = BeliefState::new();
        // Both contested → no Active fact for `city`.
        belief.record_user_fact(USER_SELF_KEY, "city", "алматы", 0);
        belief.record_user_fact(USER_SELF_KEY, "city", "астана", 1);
        let r = Tool::dispatch(
            ToolCall::SearchBelief {
                subject: USER_SELF_KEY.into(),
                predicate: Some("city".into()),
            },
            &ctx(&belief, &[]),
        );
        assert!(!r.success);
        assert!(r.findings.is_empty());
    }

    #[test]
    fn search_graph_filters_by_subject() {
        let extracted = vec![
            fact("жер", Predicate::IsA, "аспан денесі"),
            fact("күн", Predicate::IsA, "жұлдыз"),
            fact("жер", Predicate::Has, "ауа"),
        ];
        let belief = BeliefState::new();
        let r = Tool::dispatch(
            ToolCall::SearchGraph {
                subject: "жер".into(),
                predicate: None,
            },
            &ctx(&belief, &extracted),
        );
        assert!(r.success);
        assert_eq!(r.findings.len(), 2);
    }

    #[test]
    fn search_graph_filters_by_subject_and_predicate() {
        let extracted = vec![
            fact("жер", Predicate::IsA, "аспан денесі"),
            fact("жер", Predicate::Has, "ауа"),
        ];
        let belief = BeliefState::new();
        let r = Tool::dispatch(
            ToolCall::SearchGraph {
                subject: "жер".into(),
                predicate: Some("isa".into()),
            },
            &ctx(&belief, &extracted),
        );
        assert!(r.success);
        assert_eq!(r.findings.len(), 1);
        assert!(r.findings[0].contains("аспан денесі"));
    }

    /// v4.0.38 — SearchRetrieval without a MorphemeIndex returns
    /// `success=false` with explicit reason. Useful for callers
    /// that try to dispatch unconditionally.
    #[test]
    fn search_retrieval_unsupported_without_index() {
        let belief = BeliefState::new();
        let r = Tool::dispatch(
            ToolCall::SearchRetrieval {
                morphemes: vec!["жер".into()],
            },
            &ctx(&belief, &[]),
        );
        assert!(!r.success);
        assert!(
            r.trace.iter().any(|t| t.contains("no MorphemeIndex")),
            "must explain why dispatch failed, got {r:?}"
        );
    }

    /// v4.0.38 — RunLocalReasoner finds derivations whose subject
    /// or object matches the topic. Up to 3 matches returned.
    #[test]
    fn run_local_reasoner_finds_matching_derivations() {
        let derived_facts = vec![
            derived("жер", Predicate::IsA, "аспан денесі"),
            derived("күн", Predicate::IsA, "жұлдыз"),
        ];
        let belief = BeliefState::new();
        let context = ToolContext {
            belief: &belief,
            extracted: &[],
            derived: &derived_facts,
            retrieval: None,
            rank_config: None,
        };
        let r = Tool::dispatch(
            ToolCall::RunLocalReasoner {
                topic: "жер".into(),
                curated_only: false,
            },
            &context,
        );
        assert!(r.success);
        assert_eq!(r.findings.len(), 1);
        assert!(r.findings[0].contains("аспан денесі"));
    }

    #[test]
    fn run_local_reasoner_empty_when_no_match() {
        let belief = BeliefState::new();
        let r = Tool::dispatch(
            ToolCall::RunLocalReasoner {
                topic: "nonexistent".into(),
                curated_only: false,
            },
            &ctx(&belief, &[]),
        );
        assert!(!r.success);
    }

    #[test]
    fn dispatch_records_call_in_result() {
        let belief = BeliefState::new();
        let call = ToolCall::SearchBelief {
            subject: "x".into(),
            predicate: None,
        };
        let r = Tool::dispatch(call.clone(), &ctx(&belief, &[]));
        assert_eq!(r.call, call);
    }
}
