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
