// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! # v6.1.0 — `v6_1_predicate_eval_30` (Stage 5 decision-gate battery)
//!
//! Predeclared success-criteria suite for the v6.1.0 AnswerIR
//! research arc. Built from the predicate shapes the Codex 2026-05-22
//! audit flagged as broken under the v6.0.13 keyword hack and the
//! v6.1.0 design doc's scope list:
//!
//!   - 11 typed-predicate single-fact queries (BornIn, DiedIn,
//!     FoundedIn, EffectiveFrom, Classifies, RiskLevel, LocatedIn,
//!     Authored, NamedAfter, MemberOf, RenamedIn).
//!   - 7 IsA-shaped definitional queries that should still land
//!     the canonical IsA fact under the typed-focus path.
//!   - 8 broad-topic enumeration queries.
//!   - 4 continuation follow-ups paired with their broad-topic
//!     seed turn.
//!
//! 30 cases total. The assert: with `ADAM_ANSWER_IR=1`, ≥ 25 / 30
//! must surface at least one of the expected fragments. This is
//! the design doc's predeclared success threshold (table row 1).
//!
//! See [`docs/v6_1_answer_ir_design.md`](../../../docs/v6_1_answer_ir_design.md)
//! §"Predeclared success criteria" for the full gate.

use std::sync::Mutex;

use adam_dialog::{Conversation, TemplateRepository};
use adam_kernel_fst::lexicon::LexiconV1;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn load_lex() -> LexiconV1 {
    let curated = "../../data/tokenizer/segmentation_roots.json";
    let apertium = "../../data/lexicon_v1/apertium_imported_roots.json";
    LexiconV1::load(curated, apertium).expect("lexicon load failed")
}

fn load_repo() -> TemplateRepository {
    TemplateRepository::load_default().expect("templates v1.toml must exist")
}

fn load_world_core_facts() -> Vec<adam_reasoning::Fact> {
    let world_core_dir = std::path::Path::new("../../data/world_core");
    if !world_core_dir.exists() {
        return Vec::new();
    }
    adam_reasoning::world_core::load_world_core_facts(world_core_dir)
        .expect("world_core facts must load")
}

/// One eval case. `expected_one_of` passes iff at least one of the
/// listed lowercase fragments appears in the reply.
struct Case {
    label: &'static str,
    query: &'static str,
    expected_one_of: &'static [&'static str],
}

/// A multi-turn eval case for continuation testing. Each query in
/// `turns` runs in the same Conversation; the assertion targets the
/// LAST turn's reply.
struct MultiTurnCase {
    label: &'static str,
    turns: &'static [&'static str],
    expected_one_of: &'static [&'static str],
    /// Fragments that MUST NOT appear in the last turn's reply.
    /// Used for continuation cases where the new turn must avoid
    /// repeating the seed turn's fact.
    expected_none_of: &'static [&'static str],
}

const SINGLE_TURN_CASES: &[Case] = &[
    // -- Typed-predicate single-fact (11 cases) ---------------
    Case {
        label: "BornIn — Ахмет",
        query: "Ахмет Байтұрсынұлы қашан туылған?",
        expected_one_of: &["1872"],
    },
    Case {
        label: "DiedIn — Ахмет",
        query: "Ахмет Байтұрсынұлы қашан қайтыс болды?",
        expected_one_of: &["1937"],
    },
    Case {
        label: "FoundedIn — КРУ",
        query: "Қостанай өңірлік университеті қашан құрылған?",
        expected_one_of: &["1939"],
    },
    Case {
        label: "RenamedIn — КРУ 2021",
        query: "Қостанай өңірлік университеті қашан атауы өзгерген?",
        expected_one_of: &["2021", "ахмет байтұрсынұлы"],
    },
    Case {
        label: "EffectiveFrom — ЖИ заң",
        query: "Жасанды интеллект туралы заң қашан күшіне енді?",
        expected_one_of: &["2026", "қаңтар"],
    },
    Case {
        label: "Classifies — ЖИ заң",
        query: "Жасанды интеллект туралы заң қандай санаттарға жіктейді?",
        expected_one_of: &["тәуекел"],
    },
    Case {
        label: "RiskLevel — қорғаныс ИИ",
        query: "Қорғаныс саласындағы жасанды интеллект қандай тәуекелді?",
        expected_one_of: &["жоғары"],
    },
    Case {
        label: "LocatedIn — КРУ",
        query: "Қостанай өңірлік университеті қайда орналасқан?",
        expected_one_of: &["қостанай"],
    },
    Case {
        label: "Authored — Төте жазу",
        query: "Төте жазуды кім жасады?",
        expected_one_of: &["ахмет байтұрсынұлы", "1912"],
    },
    Case {
        label: "NamedAfter — КРУ",
        query: "Қостанай өңірлік университеті кімнің атымен аталған?",
        expected_one_of: &["ахмет байтұрсынұлы"],
    },
    Case {
        label: "MemberOf — Ахмет",
        query: "Ахмет Байтұрсынұлы кімнің мүшесі?",
        expected_one_of: &["алаш"],
    },
    // -- IsA / definitional (7 cases) -------------------------
    Case {
        label: "IsA — Ахмет",
        query: "Ахмет Байтұрсынұлы кім?",
        expected_one_of: &["ағартушы", "тілтанушы", "ғалым"],
    },
    Case {
        label: "IsA — КРУ",
        query: "КРУ деген не?",
        expected_one_of: &["қостанай өңірлік университеті", "жоғары оқу орны"],
    },
    Case {
        label: "IsA — Төте жазу",
        query: "Төте жазу деген не?",
        expected_one_of: &["әліпби", "араб"],
    },
    Case {
        label: "IsA — ЖИ заң",
        query: "Жасанды интеллект туралы заң деген не?",
        expected_one_of: &["қазақстан заңы", "жасанды интеллект"],
    },
    Case {
        label: "IsA — Кибер-қорғаныс",
        query: "Кибер-қорғаныс деген не?",
        expected_one_of: &["қорғау шарасы", "ақпараттық"],
    },
    Case {
        label: "IsA — Кибершабуыл",
        query: "Кибершабуыл деген не?",
        expected_one_of: &["заңсыз ену", "бұзу әрекеті", "пайдаланушы деректер"],
    },
    Case {
        label: "IsA — Алаш қозғалысы (related row)",
        query: "Алаш қозғалысы туралы не білесіз?",
        expected_one_of: &["ахмет байтұрсынұлы", "жетекші"],
    },
    // -- Broad-topic enumeration (8 cases) --------------------
    Case {
        label: "BroadTopic — Ахмет",
        query: "Ахмет Байтұрсынұлы туралы айтыңыз.",
        expected_one_of: &["ағартушы", "1872"],
    },
    Case {
        label: "BroadTopic — КРУ",
        query: "Қостанай өңірлік университеті туралы айтыңыз.",
        expected_one_of: &["жоғары оқу орны", "1939", "қостанай"],
    },
    Case {
        label: "BroadTopic — ЖИ заң",
        query: "Жасанды интеллект туралы заң туралы айтыңыз.",
        expected_one_of: &["қазақстан заңы", "тәуекел", "2026"],
    },
    Case {
        label: "BroadTopic — Төте жазу",
        query: "Төте жазу туралы айтыңыз.",
        expected_one_of: &["әліпби", "ахмет байтұрсынұлы", "1912"],
    },
    Case {
        label: "BroadTopic — Кибер-қорғаныс",
        query: "Кибер-қорғаныс туралы айтып беріңізші.",
        expected_one_of: &["қорғау шарасы", "ақпараттық"],
    },
    Case {
        label: "BroadTopic — Кибершабуыл",
        query: "Кибершабуыл туралы айтшы.",
        expected_one_of: &["заңсыз ену", "бұзу әрекеті", "пайдаланушы деректер"],
    },
    Case {
        label: "BroadTopic — Алаш",
        query: "Алаш қозғалысы туралы айтыңыз.",
        expected_one_of: &["ахмет байтұрсынұлы", "жетекші"],
    },
    Case {
        label: "BroadTopic — Қорғаныс ИИ",
        query: "Қорғаныс саласындағы жасанды интеллект туралы айтыңыз.",
        expected_one_of: &["жоғары тәуекелді", "жасанды интеллект туралы заң"],
    },
];

const MULTI_TURN_CASES: &[MultiTurnCase] = &[
    MultiTurnCase {
        label: "Continuation — Ахмет ал тағы айт",
        turns: &["Ахмет Байтұрсынұлы туралы айтыңыз.", "Ал тағы айт."],
        expected_one_of: &["1872", "1937", "төте жазу", "алаш", "әліпби"],
        // The seed turn opens with the IsA row (kru_001), so the
        // continuation MUST NOT lead with «ағартушы» — the exact
        // raw_text fragment that opened the previous turn.
        expected_none_of: &["қазақ ағартушысы, ғалым, тілтанушы"],
    },
    MultiTurnCase {
        label: "Continuation — КРУ тағы не білесіз",
        turns: &[
            "Қостанай өңірлік университеті туралы айтыңыз.",
            "Тағы не білесіз?",
        ],
        expected_one_of: &["1939", "2021", "ахмет байтұрсынұлы", "педагогика"],
        expected_none_of: &[],
    },
    MultiTurnCase {
        label: "Subject-switch — Ахмет → КРУ resets seen",
        turns: &[
            "Ахмет Байтұрсынұлы туралы айтыңыз.",
            "Қостанай өңірлік университеті туралы айтыңыз.",
        ],
        expected_one_of: &["жоғары оқу орны", "қостанай", "1939"],
        expected_none_of: &[],
    },
    MultiTurnCase {
        label: "Two continuations — drains the seen list",
        turns: &[
            "Ахмет Байтұрсынұлы туралы айтыңыз.",
            "Ал тағы айт.",
            "Тағы не білесіз?",
        ],
        expected_one_of: &["1937", "1912", "төте жазу", "алаш", "әліпби"],
        expected_none_of: &[],
    },
];

fn pass_case(reply: &str, case: &Case) -> bool {
    let lc = reply.to_lowercase();
    case.expected_one_of
        .iter()
        .any(|frag| lc.contains(&frag.to_lowercase()))
}

fn pass_multi(reply: &str, case: &MultiTurnCase) -> bool {
    let lc = reply.to_lowercase();
    let one_of_hit = case
        .expected_one_of
        .iter()
        .any(|frag| lc.contains(&frag.to_lowercase()));
    let none_of_violated = case
        .expected_none_of
        .iter()
        .any(|frag| lc.contains(&frag.to_lowercase()));
    one_of_hit && !none_of_violated
}

struct EnvGuard;

impl EnvGuard {
    fn set() -> Self {
        unsafe { std::env::set_var("ADAM_ANSWER_IR", "1") };
        Self
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe { std::env::remove_var("ADAM_ANSWER_IR") };
    }
}

#[test]
fn v6_1_predicate_eval_30() {
    let _lock = ENV_LOCK.lock().expect("env lock poisoned");
    let _flag = EnvGuard::set();

    let facts = load_world_core_facts();
    if facts.is_empty() {
        eprintln!("world_core facts not present; skipping v6_1_predicate_eval_30");
        return;
    }
    let lex = load_lex();
    let repo = load_repo();

    let mut passes = 0usize;
    let mut failures: Vec<(String, String, String)> = Vec::new();

    for case in SINGLE_TURN_CASES {
        let mut conv = Conversation::new().with_reasoning_chains(facts.clone(), Vec::new());
        let reply = conv.turn(case.query, &lex, &repo, 0);
        if pass_case(&reply, case) {
            passes += 1;
        } else {
            failures.push((case.label.to_string(), case.query.to_string(), reply));
        }
    }

    for case in MULTI_TURN_CASES {
        let mut conv = Conversation::new().with_reasoning_chains(facts.clone(), Vec::new());
        let mut last_reply = String::new();
        for (i, q) in case.turns.iter().enumerate() {
            last_reply = conv.turn(q, &lex, &repo, i as u64);
        }
        if pass_multi(&last_reply, case) {
            passes += 1;
        } else {
            failures.push((
                case.label.to_string(),
                case.turns.last().unwrap().to_string(),
                last_reply,
            ));
        }
    }

    let total = SINGLE_TURN_CASES.len() + MULTI_TURN_CASES.len();
    println!("\nv6_1_predicate_eval_30: {passes}/{total} pass");
    if !failures.is_empty() {
        println!("Failures:");
        for (label, query, reply) in &failures {
            println!("  - [{label}] «{query}» → {reply}");
        }
    }

    // Predeclared design-doc threshold: ≥ 25 / 30.
    assert!(
        total == 30,
        "v6_1_predicate_eval_30 must contain exactly 30 cases (got {total})"
    );
    assert!(
        passes >= 25,
        "v6_1_predicate_eval_30: only {passes}/30 pass — below the predeclared ≥25/30 success threshold from docs/v6_1_answer_ir_design.md §Predeclared success criteria. See failure list above."
    );
}
