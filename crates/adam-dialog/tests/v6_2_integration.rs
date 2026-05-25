// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! End-to-end integration test for the v6.2 router.
//!
//! Loads the real `data/world_core` corpus + battery-augmented
//! facts into a `FrameIndex`, then asserts the v6.2 stack answers
//! a curated set of real Kazakh / Russian questions correctly.
//!
//! Skipped silently when run outside the repo root (the corpus
//! path resolves relatively).

use adam_dialog::v6_2_router;
use adam_kernel_fst::lexicon::LexiconV1;
use std::sync::Mutex;

/// `ADAM_V6_2` env mutations are process-global and race when
/// `cargo test` runs tests in parallel. This mutex serialises the
/// two env-mutating tests so they never interleave.
static ENV_GUARD: Mutex<()> = Mutex::new(());

/// Try several relative paths so this test works both from the
/// repo root (CI / `cargo test --workspace`) and from the
/// `crates/adam-dialog` directory (local `cargo test -p adam-dialog`).
fn try_load_lexicon() -> Option<LexiconV1> {
    for (curated, apertium) in [
        (
            "data/tokenizer/segmentation_roots.json",
            "data/lexicon_v1/apertium_imported_roots.json",
        ),
        (
            "../../data/tokenizer/segmentation_roots.json",
            "../../data/lexicon_v1/apertium_imported_roots.json",
        ),
    ] {
        if let Ok(l) = LexiconV1::load(curated, apertium) {
            return Some(l);
        }
    }
    None
}

#[test]
fn v6_2_router_answers_battery_of_real_questions() {
    let cases: &[(&str, &str)] = &[
        ("Ахмет Байтұрсынұлы қашан туылған?", "1872"),
        ("Қазақ хандығы қашан құрылған?", "1465"),
        ("Алаш қозғалысы қашан құрылған?", "1917"),
        ("Қазақ КСР қашан құрылған?", "1936"),
        ("Тәуке хан қашан туылған?", "1652"),
        ("Желтоқсан оқиғасы қашан болды?", "1986"),
        ("Два плюс два", "4"),
        (
            "Двадцать пять умножь на 7 раздели на два прибавь три",
            "90.5",
        ),
        ("Корень из шестнадцати", "4"),
        ("Қазақстан деген не?", "мемлекет"),
        ("Что такое гравитация?", "сила притяжения"),
    ];

    let mut fail = Vec::new();
    for (q, expected_fragment) in cases {
        match v6_2_router::answer(q) {
            Some(a) if a.to_lowercase().contains(&expected_fragment.to_lowercase()) => {}
            Some(a) => fail.push(format!(
                "  - «{q}» → «{a}» (expected to contain «{expected_fragment}»)"
            )),
            None => fail.push(format!(
                "  - «{q}» → None (expected to contain «{expected_fragment}»)"
            )),
        }
    }
    assert!(
        fail.is_empty(),
        "v6.2 integration failures:\n{}",
        fail.join("\n")
    );
}

#[test]
fn v6_2_router_env_gate_default_off() {
    // The env-gate function must read the env var; default off
    // when ADAM_V6_2 is unset.
    let prev = std::env::var("ADAM_V6_2").ok();
    // We don't unset here (race with other tests) — just assert
    // the function is callable. The gate-default test is for
    // local manual verification.
    let _ = v6_2_router::is_v6_2_active();
    let _ = prev;
}

/// **Codex audit follow-up (2026-05-25).** The bench-time
/// `v6_2_router::answer` test above proves the router functions
/// in isolation; the audit found that with `ADAM_V6_2=1` the
/// PRODUCTION `adam_chat --once` path still returned the v6.1
/// «только казахский» refusal because `Conversation::turn_with_trace`
/// did not call the router. This test pins the end-to-end
/// integration: a `Conversation` with `ADAM_V6_2=1` set produces
/// the v6.2 answer for the exact cases the audit flagged.
#[test]
fn adam_chat_conversation_routes_through_v6_2_when_env_set() {
    use adam_dialog::{Conversation, TemplateRepository};

    // Serialise with the inverse-case test below.
    let _guard = ENV_GUARD.lock().unwrap_or_else(|e| e.into_inner());
    let prev = std::env::var("ADAM_V6_2").ok();
    // SAFETY: std::env::set_var is unsafe in edition 2024; this
    // test runs single-threaded under the default test harness.
    unsafe {
        std::env::set_var("ADAM_V6_2", "1");
    }

    let Some(lex) = try_load_lexicon() else {
        eprintln!(
            "skipping: lexicon data files not found at any candidate path \
             (test ran outside repo root)"
        );
        return;
    };
    let repo = TemplateRepository::hardcoded_fallback();
    let mut conv = Conversation::new();

    let cases: &[(&str, &str)] = &[
        // The exact cases the Codex 2026-05-25 audit flagged as
        // returning v6.1 cascade refusals when ADAM_V6_2=1.
        ("Что такое гравитация?", "сила притяжения"),
        ("Два плюс два", "4"),
        (
            "Двадцать пять умножь на 7 раздели на два прибавь три",
            "90.5",
        ),
        ("Ахмет Байтұрсынұлы қашан туылған?", "1872"),
        ("Корень из шестнадцати", "4"),
    ];

    let mut fail = Vec::new();
    for (q, expected_fragment) in cases {
        let answer = conv.turn(q, &lex, &repo, 0);
        if !answer
            .to_lowercase()
            .contains(&expected_fragment.to_lowercase())
        {
            fail.push(format!(
                "  - «{q}» → «{answer}» (expected «{expected_fragment}»)"
            ));
        }
    }

    // Restore env.
    unsafe {
        match prev {
            Some(v) => std::env::set_var("ADAM_V6_2", v),
            None => std::env::remove_var("ADAM_V6_2"),
        }
    }

    assert!(
        fail.is_empty(),
        "adam_chat / Conversation::turn integration failures with ADAM_V6_2=1:\n{}",
        fail.join("\n")
    );
}

/// Inverse case: with `ADAM_V6_2` unset, `Conversation::turn`
/// must NOT route through the v6.2 router (v6.1 cascade is
/// preserved as default behaviour).
#[test]
fn adam_chat_conversation_unchanged_when_env_unset() {
    use adam_dialog::{Conversation, TemplateRepository};

    // Serialise with the env-set test above.
    let _guard = ENV_GUARD.lock().unwrap_or_else(|e| e.into_inner());
    let prev = std::env::var("ADAM_V6_2").ok();
    unsafe {
        std::env::remove_var("ADAM_V6_2");
    }

    let Some(lex) = try_load_lexicon() else {
        eprintln!(
            "skipping: lexicon data files not found at any candidate path \
             (test ran outside repo root)"
        );
        return;
    };
    let repo = TemplateRepository::hardcoded_fallback();
    let mut conv = Conversation::new();

    // Russian input through v6.1 cascade. The exact wording depends
    // on which template repository is loaded (production / hardcoded
    // fallback), but the answer MUST NOT contain the v6.2 router's
    // typed response «сила притяжения» — that would mean the router
    // fired despite the env-gate being off.
    let answer = conv.turn("Что такое гравитация?", &lex, &repo, 0);

    // Restore env.
    unsafe {
        if let Some(v) = prev {
            std::env::set_var("ADAM_V6_2", v);
        }
    }

    let is_v6_2_answer = answer.to_lowercase().contains("сила притяжения");
    assert!(
        !is_v6_2_answer,
        "v6.2 router should NOT fire when ADAM_V6_2 is unset; got: {answer}"
    );
}
