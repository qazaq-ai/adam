// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `multi_turn_eval` — **v6.8.6 / L4.7 multi-turn evaluation
//! framework**.
//!
//! Codex's L4.5 consultation explicitly named multi-turn eval as
//! the gate before any LM-consideration work.  The single-turn
//! eval format (`data/eval/*.json` consumed by `respond_full`)
//! covers «one input → one expected output» perfectly, but it
//! can't observe state that lives across turns:
//!
//!   * referent persistence — does «Қанша жыл өмір сүрді?»
//!     after «X туралы айтшы.» actually resolve?
//!   * commitment lifecycle — does a `Proposed` claim survive
//!     the next turn and promote to `Accepted`?
//!   * correction flows — does «Жоқ, X емес, Y.» mark X
//!     `Rejected` AND record Y `Proposed` AND emit the
//!     acknowledgement template?
//!   * cross-turn anaphora for procedures — does «А қанша
//!     қадам бар?» after a procedure query resolve to the
//!     just-emitted procedure?
//!
//! The integration tests in `tests/discourse_state_commitments_
//! v684.rs` and `tests/procedure_retrieval_v685.rs` already
//! exercise this in Rust.  This module lifts the same pattern
//! into a **data-driven** runner so curators (and pilot
//! supervisors who don't write Rust) can author multi-turn
//! cases as JSON.
//!
//! ## Schema
//!
//! Each line of `data/eval_multi_turn/*.jsonl` is one
//! [`MultiTurnCase`]:
//!
//! ```jsonc
//! {
//!   "id": "correction_lifecycle_v684",
//!   "description": "User states name → corrects it → ack template",
//!   "turns": [
//!     { "input": "Менің атым — Бекжан.", "assertions": [
//!         { "kind": "session_slot_equals", "slot": "name", "value": "бекжан" }
//!     ]},
//!     { "input": "Жоқ, Бекжан емес, Болат.", "assertions": [
//!         { "kind": "response_contains", "text": "Түзеттім" },
//!         { "kind": "session_slot_equals", "slot": "name", "value": "болат" }
//!     ]}
//!   ]
//! }
//! ```

use serde::{Deserialize, Serialize};

/// One end-to-end multi-turn dialog case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultiTurnCase {
    /// Stable case identifier — included in failure messages.
    pub id: String,
    /// Free-form description.  Authors should explain *what
    /// architectural property* this case exercises (not just
    /// what the input looks like).
    pub description: String,
    /// Ordered sequence of dialog turns.
    pub turns: Vec<EvalTurn>,
}

/// One turn of an authored multi-turn case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalTurn {
    /// User input for this turn (passed verbatim to
    /// [`crate::Conversation::turn`]).
    pub input: String,
    /// Optional exact-match expected response.  Most multi-turn
    /// cases prefer fine-grained assertions over surface match;
    /// `expected_response` is here for the simple cases where
    /// the response IS the property being asserted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_response: Option<String>,
    /// Fine-grained assertions checked AFTER the turn runs.
    /// Empty list means "no assertions for this turn" (useful
    /// for set-up turns that just seed state).
    #[serde(default)]
    pub assertions: Vec<TurnAssertion>,
}

/// A typed assertion checked after a turn completes.  Each
/// variant captures one observable property that the cascade is
/// supposed to produce or update.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TurnAssertion {
    /// The turn's response must contain this substring
    /// (case-sensitive — for the most common cases authors
    /// write Kazakh substrings exactly as they expect them).
    ResponseContains { text: String },
    /// The turn's response must NOT contain this substring.
    ResponseDoesNotContain { text: String },
    /// `Conversation::session[slot]` must equal `value`
    /// (case-insensitive comparison — the cascade sometimes
    /// stores canonical-lowered forms, sometimes original
    /// case).
    SessionSlotEquals { slot: String, value: String },
    /// A referent whose token contains `token_contains`
    /// (case-insensitive) must be present on
    /// `discourse_state.referents`.
    ReferentPresent { token_contains: String },
    /// At least one commitment whose claim text contains
    /// `claim_contains` must have status equal to
    /// `expected_status` (snake_case: `proposed` / `accepted` /
    /// `rejected` / `contested`).
    CommitmentStatus {
        claim_contains: String,
        expected_status: String,
    },
}

/// Outcome of running one case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseResult {
    pub id: String,
    /// `true` iff every turn's `expected_response` (if any) and
    /// every assertion held.
    pub passed: bool,
    /// Human-readable failure messages.  Empty when `passed` is
    /// `true`.
    pub failures: Vec<String>,
}

impl CaseResult {
    pub fn ok(id: String) -> Self {
        Self {
            id,
            passed: true,
            failures: Vec::new(),
        }
    }
    pub fn fail(id: String, failures: Vec<String>) -> Self {
        Self {
            id,
            passed: false,
            failures,
        }
    }
}

/// Run one [`MultiTurnCase`] against a freshly-constructed
/// [`crate::Conversation`].  Returns a [`CaseResult`] that
/// describes every observed failure (we keep going past the
/// first failure so a single run surfaces all problems, not just
/// the first).
pub fn run_case(
    case: &MultiTurnCase,
    lex: &adam_kernel_fst::lexicon::LexiconV1,
    repo: &crate::TemplateRepository,
) -> CaseResult {
    let mut conv = crate::Conversation::new();
    let mut failures = Vec::new();

    for (turn_idx, turn) in case.turns.iter().enumerate() {
        let reply = conv.turn(&turn.input, lex, repo, turn_idx as u64);

        if let Some(expected) = &turn.expected_response
            && reply != *expected
        {
            failures.push(format!(
                "[{}] turn {}: response mismatch — expected «{}», got «{}»",
                case.id, turn_idx, expected, reply,
            ));
        }

        for (a_idx, assertion) in turn.assertions.iter().enumerate() {
            if let Some(msg) = check_assertion(assertion, &reply, &conv) {
                failures.push(format!(
                    "[{}] turn {} assertion {}: {}",
                    case.id, turn_idx, a_idx, msg,
                ));
            }
        }
    }

    if failures.is_empty() {
        CaseResult::ok(case.id.clone())
    } else {
        CaseResult::fail(case.id.clone(), failures)
    }
}

/// Check one assertion.  Returns `None` on success, `Some(msg)`
/// describing the failure otherwise.
fn check_assertion(
    assertion: &TurnAssertion,
    reply: &str,
    conv: &crate::Conversation,
) -> Option<String> {
    match assertion {
        TurnAssertion::ResponseContains { text } => {
            if reply.contains(text) {
                None
            } else {
                Some(format!(
                    "response_contains failed — expected «{text}», got «{reply}»",
                ))
            }
        }
        TurnAssertion::ResponseDoesNotContain { text } => {
            if !reply.contains(text) {
                None
            } else {
                Some(format!(
                    "response_does_not_contain failed — «{text}» was present in «{reply}»",
                ))
            }
        }
        TurnAssertion::SessionSlotEquals { slot, value } => {
            let actual = conv.session.get(slot).cloned().unwrap_or_default();
            if actual.to_lowercase() == value.to_lowercase() {
                None
            } else {
                Some(format!(
                    "session_slot_equals failed — slot «{slot}»: expected «{value}», got «{actual}»",
                ))
            }
        }
        TurnAssertion::ReferentPresent { token_contains } => {
            let needle = token_contains.to_lowercase();
            let hit = conv
                .discourse_state
                .referents
                .iter()
                .any(|r| r.token.to_lowercase().contains(&needle));
            if hit {
                None
            } else {
                Some(format!(
                    "referent_present failed — no referent containing «{token_contains}» on stack {:?}",
                    conv.discourse_state.referents,
                ))
            }
        }
        TurnAssertion::CommitmentStatus {
            claim_contains,
            expected_status,
        } => {
            let needle = claim_contains.to_lowercase();
            let want_status = expected_status.to_lowercase();
            let hit = conv.discourse_state.commitments.iter().any(|c| {
                let status_str = format!("{:?}", c.status).to_lowercase();
                c.claim_text.to_lowercase().contains(&needle) && status_str == want_status
            });
            if hit {
                None
            } else {
                Some(format!(
                    "commitment_status failed — no commitment matching «{claim_contains}» \
                     with status {expected_status}; current commitments: {:?}",
                    conv.discourse_state.commitments,
                ))
            }
        }
    }
}

/// Parse a single JSONL line into a [`MultiTurnCase`].
pub fn case_from_jsonl_line(line: &str) -> Result<MultiTurnCase, serde_json::Error> {
    serde_json::from_str(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_round_trip() {
        let case = MultiTurnCase {
            id: "round_trip".into(),
            description: "Smoke test that all assertion variants serialise cleanly.".into(),
            turns: vec![EvalTurn {
                input: "тест".into(),
                expected_response: Some("ок".into()),
                assertions: vec![
                    TurnAssertion::ResponseContains {
                        text: "ок".into()
                    },
                    TurnAssertion::ResponseDoesNotContain {
                        text: "ошибка".into(),
                    },
                    TurnAssertion::SessionSlotEquals {
                        slot: "name".into(),
                        value: "x".into(),
                    },
                    TurnAssertion::ReferentPresent {
                        token_contains: "x".into(),
                    },
                    TurnAssertion::CommitmentStatus {
                        claim_contains: "x".into(),
                        expected_status: "accepted".into(),
                    },
                ],
            }],
        };
        let line = serde_json::to_string(&case).expect("serialize");
        let parsed = case_from_jsonl_line(&line).expect("round-trip parse");
        assert_eq!(parsed, case);
    }

    #[test]
    fn empty_assertions_turn_is_valid() {
        // Set-up turns with no assertions are explicitly allowed.
        let line = r#"{"id":"x","description":"y","turns":[{"input":"тест"}]}"#;
        let parsed = case_from_jsonl_line(line).expect("parse");
        assert_eq!(parsed.turns.len(), 1);
        assert!(parsed.turns[0].assertions.is_empty());
        assert!(parsed.turns[0].expected_response.is_none());
    }
}
