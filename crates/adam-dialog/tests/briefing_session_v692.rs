// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! Integration test for the v6.9.2 safety-briefing session engine,
//! driven against the REAL curated procedure corpus
//! (`data/procedures/labor_safety_kz.jsonl`).
//!
//! Convention (see `answer_ir_e2e_v590.rs`): tests gracefully skip
//! when the data artefact is absent so partial checkouts still build.

use adam_dialog::briefing_session::{BriefingProtocol, BriefingSession};
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

/// Drive a session to completion: acknowledge every step, then feed
/// one answer per generated question (cycling `answers` if shorter).
fn run(id: &str, answers: &[String]) -> BriefingProtocol {
    let mut s = BriefingSession::from_id(id).expect("procedure exists");
    let nq = s.questions().len();
    let steps = s.step_count();
    let _ = s.begin();
    for _ in 0..steps {
        s.advance("түсінікті");
    }
    for i in 0..nq {
        s.advance(&answers[i % answers.len()]);
    }
    s.protocol().expect("session done")
}

/// **v6.10.2 — corpus-wide adversarial gate (Codex 2026-07-02).**
/// For EVERY curated procedure the session engine must:
///   - admit a worker who recites the curated answers, and
///   - deny (never admit) a worker who answers with noise, with the
///     question text echoed back, or with a neighbouring question's
///     answer — the leak class that let `kk_labor_work_permit_022`
///     score 4/5 on prompt-echo before the grader was hardened.
#[test]
fn adversarial_sweep_admits_curated_denies_gaming() {
    if !corpus_available() {
        eprintln!("procedure corpus missing — skipping");
        return;
    }
    let noise = vec!["иә рахмет жақсы".to_string()];
    let mut admit_failures = vec![];
    let mut gaming_admits = vec![];
    let mut echo_pass = vec![];

    for p in shared_procedures() {
        let s = BriefingSession::from_id(&p.id).unwrap();
        assert!(
            s.questions().len() >= 3,
            "{}: must generate ≥3 control questions",
            p.id
        );
        let expected: Vec<String> = s
            .questions()
            .iter()
            .map(|q| q.expected.first().cloned().unwrap_or_default())
            .collect();
        let prompts: Vec<String> = s.questions().iter().map(|q| q.prompt_kk.clone()).collect();

        // 1. Curated recitation → admitted, every question passed.
        let curated = run(&p.id, &expected);
        if !curated.admitted || curated.passed_count != curated.total {
            admit_failures.push(format!(
                "{}: curated {}/{} admitted={}",
                p.id, curated.passed_count, curated.total, curated.admitted
            ));
        }

        // 2. Noise → never admitted.
        if run(&p.id, &noise).admitted {
            gaming_admits.push(format!("{}: NOISE admitted", p.id));
        }

        // 3. Prompt echo → never admitted; expect zero question passes
        //    (all answer tokens echo the prompt → nothing novel).
        let echo = run(&p.id, &prompts);
        if echo.admitted {
            gaming_admits.push(format!("{}: PROMPT-ECHO admitted", p.id));
        }
        if echo.passed_count != 0 {
            echo_pass.push(format!(
                "{}: echo passed {} question(s)",
                p.id, echo.passed_count
            ));
        }

        // 4. Adjacent-answer contamination (shift by one) → never admitted.
        if expected.len() >= 2 {
            let mut shifted = expected.clone();
            shifted.rotate_left(1);
            if run(&p.id, &shifted).admitted {
                gaming_admits.push(format!("{}: ADJACENT(i+1) admitted", p.id));
            }
        }
    }

    assert!(
        admit_failures.is_empty(),
        "curated recitation must admit:\n{}",
        admit_failures.join("\n")
    );
    assert!(
        gaming_admits.is_empty(),
        "gaming answers must never admit:\n{}",
        gaming_admits.join("\n")
    );
    assert!(
        echo_pass.is_empty(),
        "prompt-echo must pass zero questions:\n{}",
        echo_pass.join("\n")
    );
}
