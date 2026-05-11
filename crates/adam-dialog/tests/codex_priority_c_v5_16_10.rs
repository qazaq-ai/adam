//! **v5.16.10** — Codex 2026-05-11 audit, priority C closure.
//!
//! Codex flagged: «Учебный домен пока brittle: `Rust-та borrowing
//! бойынша жаттығу бер` не вытащил topic и попросил уточнить, хотя
//! человек ожидает упражнение.»
//!
//! Diagnosis: the AskExercise intent + detector existed (v4.93.5)
//! and the canonical Kazakh aliases (иелік, қарыз, өмір кезеңі,
//! қасиет, асинхрон) all routed correctly (v5.2.5). But latin
//! surface forms — «borrowing» / «ownership» / «lifetimes» /
//! «traits» / «async» — fell through to `latin_subject_hint`,
//! which grabbed the first latin token (typically `rust` from
//! «Rust-та …») and stopped. Topic = `Some("rust")` matched no
//! curriculum stage; `exercise_body` stayed empty; planner routed
//! to `ask_exercise.no_topic` (clarification template).
//!
//! Fix: `pedagogical_topic_hint` now checks a latin_aliases list
//! AFTER Kazakh aliases but BEFORE `latin_subject_hint`. Each
//! entry maps a latin surface form to the canonical curriculum
//! stage id used by `curriculum.stage(t)`.

use adam_dialog::{Intent, semantics::interpret_text};

#[test]
fn latin_alias_borrowing_routes_to_borrow_exercise_v51610() {
    // The exact Codex scenario.
    let intent = interpret_text("Rust-та borrowing бойынша жаттығу бер", &[]);
    match intent {
        Intent::AskExercise { topic } => {
            assert_eq!(
                topic.as_deref(),
                Some("borrow"),
                "latin «borrowing» must canonicalise to `borrow` (not `rust`); got: {topic:?}"
            );
        }
        other => panic!("expected AskExercise, got {other:?}"),
    }
}

#[test]
fn latin_alias_ownership_routes_to_ownership_exercise_v51610() {
    let intent = interpret_text("Rust-та ownership бойынша жаттығу беріңізші", &[]);
    match intent {
        Intent::AskExercise { topic } => {
            assert_eq!(topic.as_deref(), Some("ownership"));
        }
        other => panic!("expected AskExercise, got {other:?}"),
    }
}

#[test]
fn latin_alias_lifetimes_routes_to_lifetime_exercise_v51610() {
    let intent = interpret_text("Маған lifetimes бойынша жаттығу керек", &[]);
    match intent {
        Intent::AskExercise { topic } => {
            assert_eq!(
                topic.as_deref(),
                Some("lifetime"),
                "plural «lifetimes» must canonicalise to singular `lifetime`"
            );
        }
        other => panic!("expected AskExercise, got {other:?}"),
    }
}

#[test]
fn latin_alias_traits_routes_to_traits_exercise_v51610() {
    let intent = interpret_text("traits бойынша жаттығу бер", &[]);
    match intent {
        Intent::AskExercise { topic } => {
            assert_eq!(topic.as_deref(), Some("traits"));
        }
        other => panic!("expected AskExercise, got {other:?}"),
    }
}

#[test]
fn latin_alias_async_routes_to_async_exercise_v51610() {
    let intent = interpret_text("async бойынша жаттығу беріңіз", &[]);
    match intent {
        Intent::AskExercise { topic } => {
            assert_eq!(topic.as_deref(), Some("async"));
        }
        other => panic!("expected AskExercise, got {other:?}"),
    }
}

#[test]
fn latin_alias_borrow_singular_also_canonicalises_v51610() {
    // The singular «borrow» (without -ing) should also map.
    let intent = interpret_text("borrow бойынша жаттығу бер", &[]);
    match intent {
        Intent::AskExercise { topic } => {
            assert_eq!(topic.as_deref(), Some("borrow"));
        }
        other => panic!("expected AskExercise, got {other:?}"),
    }
}

#[test]
fn pure_rust_token_without_stage_still_routes_to_clarification_v51610() {
    // «Rust бойынша жаттығу бер» has no specific curriculum stage —
    // topic falls back to `latin_subject_hint`'s `rust` and the
    // planner correctly routes to `ask_exercise.no_topic`. This
    // protects against an over-aggressive fix that would mis-map
    // `rust` itself onto some default stage.
    let intent = interpret_text("Rust бойынша жаттығу бер", &[]);
    match intent {
        Intent::AskExercise { topic } => {
            // Either no topic OR a non-stage topic — both are
            // acceptable; what's NOT acceptable is silently
            // pretending `rust` is a curriculum stage. The curriculum
            // resolver downstream filters out non-stage topics.
            let acceptable = topic.is_none()
                || topic
                    .as_deref()
                    .map(|t| !["ownership", "borrow", "lifetime", "traits", "async"].contains(&t))
                    .unwrap_or(true);
            assert!(
                acceptable,
                "pure «rust» must not be claimed as a curriculum stage; got: {topic:?}"
            );
        }
        other => panic!("expected AskExercise, got {other:?}"),
    }
}
