// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! Integration test for the v6.9.2 safety-briefing session engine,
//! driven against the REAL curated procedure corpus
//! (`data/procedures/labor_safety_kz.jsonl`).
//!
//! Convention (see `answer_ir_e2e_v590.rs`): tests gracefully skip
//! when the data artefact is absent so partial checkouts still build.

use adam_dialog::briefing_session::BriefingSession;
use adam_dialog::procedure_loader::shared_procedures;

/// A procedure id known to exist in the corpus and to carry enough
/// curated structure (authorization + hazards + confirmation_gates)
/// to generate the full five-question set.
const LOTO_ID: &str = "kk_metallurgy_loto_003";

fn corpus_available() -> bool {
    !shared_procedures().is_empty()
}

#[test]
fn from_id_returns_none_for_unknown_procedure() {
    if !corpus_available() {
        eprintln!("procedure corpus missing — skipping");
        return;
    }
    assert!(BriefingSession::from_id("kk_does_not_exist_999").is_none());
}

#[test]
fn reciting_curated_answers_admits_the_worker() {
    if !corpus_available() {
        eprintln!("procedure corpus missing — skipping");
        return;
    }
    let Some(mut s) = BriefingSession::from_id(LOTO_ID) else {
        panic!("{LOTO_ID} must exist in the corpus");
    };

    // Snapshot the generated questions BEFORE the quiz consumes them,
    // so we can answer each with its own curated content — this keeps
    // the test robust to future edits of the LOTO record.
    let expected: Vec<String> = s
        .questions()
        .iter()
        .map(|q| q.expected.first().cloned().unwrap_or_default())
        .collect();
    assert!(
        expected.len() >= 3,
        "a richly-curated procedure must yield ≥3 control questions, got {}",
        expected.len()
    );

    // Walk the instruction phase: one acknowledgement per step opens
    // the quiz on the final advance.
    let steps = s.step_count();
    let intro = s.begin();
    assert!(
        intro.contains("нұсқаулық сессиясы"),
        "intro must announce the session"
    );
    let mut reply = s.advance("түсінікті");
    for _ in 1..steps {
        assert!(!reply.done, "session must not finish during instruction");
        reply = s.advance("түсінікті");
    }
    assert!(
        reply.text.contains("Сұрақ 1"),
        "quiz must open after the last step; got: {}",
        reply.text
    );

    // Answer every control question by reciting its curated content.
    for e in &expected {
        assert!(!reply.done, "quiz ended before all questions were asked");
        reply = s.advance(e);
    }
    assert!(reply.done, "session must finish after the last answer");

    let proto = s.protocol().expect("protocol is ready once done");
    assert_eq!(proto.total, expected.len());
    assert_eq!(
        proto.passed_count, proto.total,
        "reciting the curated answer to every question must pass every question",
    );
    assert!(
        proto.admitted,
        "a fully-correct worker must be admitted (допущен)"
    );
    assert!(proto.render_kk().contains("ЖҰМЫСҚА ЖІБЕРІЛДІ"));
}

#[test]
fn empty_answers_deny_the_worker() {
    if !corpus_available() {
        eprintln!("procedure corpus missing — skipping");
        return;
    }
    let Some(mut s) = BriefingSession::from_id(LOTO_ID) else {
        panic!("{LOTO_ID} must exist in the corpus");
    };
    let n_questions = s.questions().len();
    let steps = s.step_count();
    let _ = s.begin();
    for _ in 0..steps {
        s.advance("түсінікті");
    }
    for _ in 0..n_questions {
        s.advance("білмеймін");
    }
    let proto = s.protocol().expect("done");
    assert_eq!(proto.passed_count, 0, "noise answers must not pass");
    assert!(!proto.admitted, "a worker who knows nothing must be denied");
    assert!(proto.render_kk().contains("ЖІБЕРІЛМЕДІ"));
}
