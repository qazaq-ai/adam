//! **v5.2.5** — Codex 2026-05-08 round-3 audit fixes.
//!
//! Closes 5 P0/P1 UX bugs flagged in the round-3 review:
//!
//! - Bug 1 — placeholder leak (`{Occupation} екен`) on interrogative
//!   «Мен кім болып жұмыс істеймін?»
//! - Bug 3 — Kazakh-first tutor UX. «Иелік бойынша жаттығу беріңізші»
//!   was not recognised as ownership exercise.
//! - Bug 6 — Kazakh-only policy inconsistency. English «What is
//!   Rust ownership?» got a substantive answer.
//! - Stale banner (v5.1 → v5.2).
//! - cargo fmt diff.
//!
//! Deferred to v5.3.0 (architectural):
//! - Bug 2 — contradiction resolution (Алматы/Астана dance).
//! - Bug 4 — anaphora over-carry (Аспан after Қазақстан).
//! - Bug 5 — shallow domain answers («Алматыдағы таулар»).

use adam_dialog::{
    Conversation, Intent, TemplateRepository,
    discourse::{input_is_likely_english, input_is_likely_russian},
    semantics::interpret_text,
};
use adam_kernel_fst::lexicon::LexiconV1;

fn lex() -> Option<LexiconV1> {
    let curated = "../../data/tokenizer/segmentation_roots.json";
    let apertium = "../../data/lexicon_v1/apertium_imported_roots.json";
    if !std::path::Path::new(curated).exists() || !std::path::Path::new(apertium).exists() {
        return None;
    }
    LexiconV1::load(curated, apertium).ok()
}

fn repo() -> TemplateRepository {
    TemplateRepository::load_default().expect("templates v1.toml")
}

// ─── Bug 3: Kazakh curriculum aliases ─────────────────────────────────

#[test]
fn kazakh_alias_иелік_routes_to_ownership_exercise() {
    let intent = interpret_text("Иелік бойынша жаттығу беріңізші.", &[]);
    match intent {
        Intent::AskExercise { topic } => {
            assert_eq!(
                topic.as_deref(),
                Some("ownership"),
                "Kazakh «Иелік» must canonicalise to `ownership`; got: {topic:?}"
            );
        }
        other => panic!("expected AskExercise, got {other:?}"),
    }
}

#[test]
fn kazakh_alias_қарыз_алу_routes_to_borrow_exercise() {
    let intent = interpret_text("Қарыз алу жаттығуы керек.", &[]);
    match intent {
        Intent::AskExercise { topic } => {
            assert_eq!(topic.as_deref(), Some("borrow"));
        }
        other => panic!("expected AskExercise, got {other:?}"),
    }
}

#[test]
fn kazakh_alias_өмір_кезеңі_routes_to_lifetime_exercise() {
    let intent = interpret_text("Өмір кезеңі жаттығуын ұсыныңыз.", &[]);
    match intent {
        Intent::AskExercise { topic } => {
            assert_eq!(topic.as_deref(), Some("lifetime"));
        }
        other => panic!("expected AskExercise, got {other:?}"),
    }
}

#[test]
fn kazakh_alias_қасиеттер_routes_to_traits_exercise() {
    let intent = interpret_text("Қасиеттер бойынша жаттығу бер.", &[]);
    match intent {
        Intent::AskExercise { topic } => {
            assert_eq!(topic.as_deref(), Some("traits"));
        }
        other => panic!("expected AskExercise, got {other:?}"),
    }
}

#[test]
fn kazakh_alias_асинхронды_routes_to_async_exercise() {
    let intent = interpret_text("Асинхронды жаттығу беріңізші.", &[]);
    match intent {
        Intent::AskExercise { topic } => {
            assert_eq!(topic.as_deref(), Some("async"));
        }
        other => panic!("expected AskExercise, got {other:?}"),
    }
}

// ─── Bug 1: placeholder leak — interrogative not StatementOfOccupation ──

#[test]
fn occupation_question_does_not_route_to_statement_of_occupation_none() {
    // «Мен кім болып жұмыс істеймін?» is a QUESTION about who one
    // works as, NOT a statement. Pre-fix it routed to
    // StatementOfOccupation { occupation: None } which surfaced the
    // literal `{occupation} екен, түсіндім.` placeholder. Post-fix
    // it must NOT route to StatementOfOccupation.
    let intent = interpret_text("Мен кім болып жұмыс істеймін?", &[]);
    assert!(
        !matches!(intent, Intent::StatementOfOccupation { occupation: None }),
        "interrogative must NOT route to StatementOfOccupation None; got: {intent:?}"
    );
}

#[test]
fn occupation_statement_still_routes_correctly() {
    // Sanity check — non-interrogative «Жұмыс істеймін» should
    // still route to StatementOfOccupation None as before.
    let intent = interpret_text("Жұмыс істеймін.", &[]);
    assert!(
        matches!(intent, Intent::StatementOfOccupation { occupation: None }),
        "non-question «жұмыс істеймін» must still route to StatementOfOccupation; got: {intent:?}"
    );
}

#[test]
fn occupation_question_response_does_not_leak_placeholder() {
    let Some(lex) = lex() else { return };
    let repo = repo();
    let mut conv = Conversation::new();
    let out = conv.turn("Мен кім болып жұмыс істеймін?", &lex, &repo, 0);
    // The exact response shape varies (depends on routing), but
    // the literal `{occupation}` MUST NOT appear in user-visible
    // output.
    assert!(
        !out.contains("{occupation}"),
        "response must not contain unfilled {{occupation}} placeholder; got: {out}"
    );
    assert!(
        !out.contains("{Occupation}"),
        "response must not contain unfilled {{Occupation}} placeholder; got: {out}"
    );
}

// ─── Bug 6: English-input refusal ─────────────────────────────────────

#[test]
fn english_input_what_is_rust_ownership_detected() {
    assert!(input_is_likely_english("What is Rust ownership?"));
    assert!(input_is_likely_english("How do you explain async/await?"));
    assert!(input_is_likely_english("Tell me about lifetimes."));
}

#[test]
fn english_detector_negatives() {
    // Pure Kazakh — no English markers.
    assert!(!input_is_likely_english("Маған Rust туралы айтыңыз."));
    // Bare Latin technical token inside Kazakh sentence — should NOT
    // trigger (we want adam to handle «ownership туралы айт» as a
    // mixed-script Kazakh query).
    assert!(!input_is_likely_english("ownership туралы айт"));
    // Russian, not English — handled by the Russian detector.
    assert!(!input_is_likely_english("Расскажи про Rust"));
    // Empty / whitespace.
    assert!(!input_is_likely_english(""));
    assert!(!input_is_likely_english("   "));
}

#[test]
fn english_input_routes_to_kazakh_only_refusal() {
    let Some(lex) = lex() else { return };
    let repo = repo();
    let mut conv = Conversation::new();
    let out = conv.turn("What is Rust ownership?", &lex, &repo, 0);
    // Kazakh-only refusal mentions «қазақ» (Kazakh) and the «only»
    // / «not understand other languages» framing.
    assert!(
        out.contains("қазақша") || out.contains("қазақ тілінде"),
        "English-input refusal must surface Kazakh-only language; got: {out}"
    );
}

// ─── Russian detector remains intact (regression guard) ───────────────

#[test]
fn russian_detector_still_fires() {
    assert!(input_is_likely_russian("Расскажи про Rust"));
    assert!(input_is_likely_russian("Объясните мне ownership"));
}
