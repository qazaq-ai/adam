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

/// One callable internal tool. Each variant carries the inputs the
/// dispatcher needs; results come back through [`ToolResult`].
///
/// Naming follows the Codex v4.0.26 roadmap proposal verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolCall {
    /// Look up active belief facts by subject (e.g. user name,
    /// city, occupation). v4.0.37 — fully implemented.
    SearchBelief { subject: String },
    /// Look up extracted (base) facts by subject and optional
    /// predicate. Proxies for "search the lexical graph" — the
    /// graph index itself isn't yet exposed, so we filter the flat
    /// `extracted_facts` Vec instead. v4.0.37 — fully implemented.
    SearchGraph {
        subject: String,
        predicate: Option<String>,
    },
    /// Reserved — corpus retrieval via `MorphemeIndex::rank`.
    /// v4.0.37 returns `success=false` with reason recorded in
    /// `trace`; v4.0.38 will wire `inject_retrieval_example`
    /// through this dispatcher.
    SearchRetrieval { morphemes: Vec<String> },
    /// Reserved — invoke the offline reasoner on a specific topic
    /// to derive a fresh chain on demand. v4.0.37 stub.
    RunLocalReasoner { topic: String },
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

    fn empty(call: ToolCall, reason: &str) -> Self {
        Self {
            call,
            success: false,
            findings: Vec::new(),
            trace: vec![reason.to_string()],
        }
    }

    fn stubbed(call: ToolCall, why: &str) -> Self {
        Self {
            call,
            success: false,
            findings: Vec::new(),
            trace: vec![format!("v4.0.37 stub — {why}; v4.0.38 will wire it")],
        }
    }
}

/// Pure-function dispatcher. Reads belief / extracted_facts (and
/// later, retrieval index + derived_facts), never mutates them.
/// Future tools that need write access (e.g. updating a learned
/// scorer) must take `&mut` references explicitly.
pub struct Tool;

impl Tool {
    /// Execute a single `ToolCall` against the available stores.
    /// `extracted` is the flat fact list (Phase 1+2 substrate);
    /// when the planner wires this in v4.0.38, callers will pass
    /// the same `&self.extracted_facts` they already keep on the
    /// `Conversation`.
    pub fn dispatch(call: ToolCall, belief: &BeliefState, extracted: &[ReasFact]) -> ToolResult {
        let mut trace = Vec::new();
        trace.push(format!("tool: dispatch {call:?}"));
        match &call {
            ToolCall::SearchBelief { subject } => {
                let active: Vec<String> = belief
                    .facts
                    .iter()
                    .filter(|f| f.subject == *subject && f.status == FactStatus::Active)
                    .map(|f| format!("{} {} {}", f.subject, f.predicate, f.object))
                    .collect();
                trace.push(format!(
                    "search_belief: subject={subject} active_matches={}",
                    active.len()
                ));
                if active.is_empty() {
                    ToolResult::empty(call, "search_belief: no active facts")
                } else {
                    ToolResult::ok(call, active, trace)
                }
            }
            ToolCall::SearchGraph { subject, predicate } => {
                let matches: Vec<String> = extracted
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
            ToolCall::SearchRetrieval { .. } => {
                ToolResult::stubbed(call, "SearchRetrieval not yet wired to MorphemeIndex")
            }
            ToolCall::RunLocalReasoner { .. } => {
                ToolResult::stubbed(call, "RunLocalReasoner not yet wired to reasoner")
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

    #[test]
    fn search_belief_finds_active_fact() {
        let mut belief = BeliefState::new();
        belief.record_user_fact(USER_SELF_KEY, "name", "Дәулет", 0);
        let r = Tool::dispatch(
            ToolCall::SearchBelief {
                subject: USER_SELF_KEY.into(),
            },
            &belief,
            &[],
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
            },
            &belief,
            &[],
        );
        assert!(!r.success);
        assert!(r.findings.is_empty());
    }

    #[test]
    fn search_belief_skips_contested_facts() {
        // Single-active-fact invariant from v4.0.28: contested
        // facts are NOT Active, so SearchBelief must not return
        // them.
        let mut belief = BeliefState::new();
        belief.record_user_fact(USER_SELF_KEY, "city", "алматы", 0);
        belief.record_user_fact(USER_SELF_KEY, "city", "астана", 1);
        let r = Tool::dispatch(
            ToolCall::SearchBelief {
                subject: USER_SELF_KEY.into(),
            },
            &belief,
            &[],
        );
        assert!(
            !r.success,
            "all city facts contested → no Active → SearchBelief must return empty, got {r:?}"
        );
    }

    #[test]
    fn search_graph_filters_by_subject() {
        let extracted = vec![
            fact("жер", Predicate::IsA, "аспан денесі"),
            fact("күн", Predicate::IsA, "жұлдыз"),
            fact("жер", Predicate::Has, "ауа"),
        ];
        let r = Tool::dispatch(
            ToolCall::SearchGraph {
                subject: "жер".into(),
                predicate: None,
            },
            &BeliefState::new(),
            &extracted,
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
        let r = Tool::dispatch(
            ToolCall::SearchGraph {
                subject: "жер".into(),
                predicate: Some("isa".into()),
            },
            &BeliefState::new(),
            &extracted,
        );
        assert!(r.success);
        assert_eq!(r.findings.len(), 1);
        assert!(r.findings[0].contains("аспан денесі"));
    }

    #[test]
    fn search_retrieval_is_stubbed_in_v4_0_37() {
        let r = Tool::dispatch(
            ToolCall::SearchRetrieval {
                morphemes: vec!["жер".into()],
            },
            &BeliefState::new(),
            &[],
        );
        assert!(!r.success);
        assert!(r.trace.iter().any(|t| t.contains("v4.0.37 stub")));
    }

    #[test]
    fn run_local_reasoner_is_stubbed_in_v4_0_37() {
        let r = Tool::dispatch(
            ToolCall::RunLocalReasoner {
                topic: "жер".into(),
            },
            &BeliefState::new(),
            &[],
        );
        assert!(!r.success);
        assert!(r.trace.iter().any(|t| t.contains("v4.0.37 stub")));
    }

    #[test]
    fn dispatch_records_call_in_result() {
        let belief = BeliefState::new();
        let call = ToolCall::SearchBelief {
            subject: "x".into(),
        };
        let r = Tool::dispatch(call.clone(), &belief, &[]);
        assert_eq!(r.call, call);
    }
}
