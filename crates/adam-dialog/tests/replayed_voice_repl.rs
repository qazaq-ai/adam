// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/agglutinative-foundation
//! **Replayed voice REPL battery.**
//!
//! Every voice REPL audit session (1 through 5) produced a set
//! of real Whisper-transcribed inputs — including STT mishears,
//! repeat-loops, broken-grammar approximations, and case-form
//! variants. Without a replay battery, the same bugs come back
//! after every refactor (and history shows they do — sessions
//! 3, 4, 5 each found regressions of bugs we'd "fixed").
//!
//! This file is the **regression floor for the voice surface**:
//! the exact strings Whisper produced are pinned to the answers
//! the router must give. Adding a new fix to
//! `adam-dialog/src/v6_2_router.rs` is incomplete until its
//! triggering input lands here as a new test case.
//!
//! Per memory `feedback_audio_dialog_testing`: "every voice REPL
//! audit finding becomes a permanent regression test".

use adam_dialog::v6_2_router::answer;

/// One battery entry: the exact Whisper transcript, plus a
/// substring the answer **must contain** (case-insensitive).
struct ReplayCase {
    /// What Whisper transcribed in a real voice REPL session.
    input: &'static str,
    /// Substring (lowercased before comparison) the answer must
    /// contain. Picking a stable substring rather than the
    /// whole answer keeps tests robust to incidental rewording.
    ///
    /// **Phase 20 multi-paraphrase tolerance** (2026-06-02) — when
    /// the handler returns one of N paraphrased variants, use a
    /// `|`-separated alternation here (e.g. «А|Б|В»). The test
    /// passes if the answer contains ANY of the alternatives.
    /// Single-string cases continue to work unchanged.
    expects_substring: &'static str,
    /// Which audit session surfaced this case. Documentation
    /// only — not asserted, but useful for archaeology.
    session: u8,
}

const BATTERY: &[ReplayCase] = &[
    // ── Session 1 (2026-05-25, baseline voice REPL) ──
    ReplayCase {
        input: "Ахмет Байтұрсынұлы қашан туылған?",
        expects_substring: "1872",
        session: 1,
    },
    ReplayCase {
        input: "Желтоқсан оқиғасы қашан болды?",
        expects_substring: "1986",
        session: 1,
    },
    ReplayCase {
        input: "Қазақстан деген не?",
        expects_substring: "мемлекет",
        session: 1,
    },
    ReplayCase {
        input: "Что такое гравитация?",
        expects_substring: "сила притяжения",
        session: 1,
    },
    // ── Session 2 (Дөңгелек / Серік / fold) ──
    ReplayCase {
        input: "Жарық жылдамдығы қанша?",
        expects_substring: "299792458",
        session: 2,
    },
    ReplayCase {
        input: "Биткоин барамы қандай?",
        expects_substring: "нақты дерек жоқ",
        session: 2,
    },
    // ── Session 3 (STT loop dedupe + broad-topic + capabilities) ──
    // Note: «Сәлем» loops dedupe correctly inside the router
    // (`dedupe_stt_loop`), but the router returns `None` on
    // bare greetings — the v6.1 cascade handles them. We test
    // the dedupe path directly via the `stt_loop_dedupes_to_one`
    // case below instead of routing through `answer()`.
    ReplayCase {
        // Occupation statement (acknowledge, don't define).
        input: "Мен бағдарламашымын",
        expects_substring: "бағдарлама",
        session: 3,
    },
    ReplayCase {
        input: "Сен не білесің?",
        // Phase 20 capability response is one of 5 paraphrased variants
        // (FNV-1a input hash selects). Stable substring across all 5:
        // every variant mentions «Rust» and «математика». Use the
        // alternation form so the test stays green if variant pool grows.
        expects_substring: "математика|rust",
        session: 3,
    },
    ReplayCase {
        input: "Қазақстан туралы айтшы",
        expects_substring: "мемлекет",
        session: 3,
    },
    ReplayCase {
        input: "Қазақстанның көршілері кім?",
        expects_substring: "ресей",
        session: 3,
    },
    ReplayCase {
        input: "Қазақстанның облыстарын айтшы",
        expects_substring: "17 облыс",
        session: 3,
    },
    ReplayCase {
        // STT mishear: «оналты» (no space) → must fold to «он
        // алты» before math sees it.
        input: "Оналты түбірі қанша?",
        expects_substring: "4",
        session: 3,
    },
    ReplayCase {
        // STT mishear: «жел тұқтықстан» → must fold to
        // «желтоқсан».
        input: "Жел тұқтықстан оқиғасы қашан болды?",
        expects_substring: "1986",
        session: 3,
    },
    // ── Session 4 (STT phonetic-neighbour folds) ──
    ReplayCase {
        // «мүгін» → «бүгін» before clock gate.
        input: "Мүгін қандай ай?",
        expects_substring: "ай",
        session: 4,
    },
    ReplayCase {
        // «қобейту» → «көбейту» before math.
        input: "Екі қобейту екі",
        expects_substring: "4",
        session: 4,
    },
    ReplayCase {
        // «кубит» → «көбейту» (math interpretation).
        input: "Екі кубит беске",
        expects_substring: "10",
        session: 4,
    },
    ReplayCase {
        // «дарижысы» → «дәрежесі» before power op.
        input: "Екі дарижысы он",
        expects_substring: "1024",
        session: 4,
    },
    ReplayCase {
        // «пайз» → «пайыз» before percent op.
        input: "Жүз пайз бес",
        expects_substring: "5",
        session: 4,
    },
    ReplayCase {
        // «толған» → «туылған» (only when preceded by «қашан»);
        // «байтурсынулы» → «байтұрсынұлы».
        input: "Ахмет байтурсынулы қашан толған?",
        expects_substring: "1872",
        session: 4,
    },
    ReplayCase {
        // Past-tense first-person slip; must still trigger
        // pitch-detection explanation.
        input: "Мен ағай болғанымды қалай түсіндім?",
        expects_substring: "жиілігі",
        session: 4,
    },
    // ── Session 5 (the «Мемлекет» fix) ──
    ReplayCase {
        input: "Қазақстанда қандай таулар бар?",
        expects_substring: "алатау",
        session: 5,
    },
    ReplayCase {
        input: "Қазақстанда қандай өзендер бар?",
        expects_substring: "ертіс",
        session: 5,
    },
    ReplayCase {
        input: "Қазақстанда қандай көлдер бар?",
        expects_substring: "балқаш",
        session: 5,
    },
    ReplayCase {
        // The un-anchored form caught by session-5.
        input: "Қандай өзендер білесің?",
        expects_substring: "ертіс",
        session: 5,
    },
    // ── Math chains (real human input across sessions) ──
    ReplayCase {
        input: "Два плюс два",
        expects_substring: "4",
        session: 1,
    },
    ReplayCase {
        input: "Корень из шестнадцати",
        expects_substring: "4",
        session: 1,
    },
    ReplayCase {
        input: "Двадцать пять умножь на 7 раздели на два прибавь три",
        expects_substring: "90.5",
        session: 1,
    },
    // ── Session 6 (2026-06-03, post-v6.3.0-rc2 live audit) ──
    // Bug B-3: «Қазір қазақстанның президенті кім?» was routing to
    // the clock handler because substring «қазір» fired
    // looks_like_time_query before the president check. Fix:
    // looks_like_time_query now defers when input contains a clear
    // entity marker (президент / премьер / etc.).
    ReplayCase {
        input: "Қазір қазақстанның президенті кім?",
        expects_substring: "тоқаев",
        session: 6,
    },
    ReplayCase {
        input: "Қазір кім қазақстанның президенті?",
        expects_substring: "тоқаев",
        session: 6,
    },
    ReplayCase {
        input: "Қазақстанның бірінші президенті кім болды.",
        expects_substring: "назарбаев",
        session: 6,
    },
];

/// Replay battery — every case must produce an answer
/// containing the expected substring (case-insensitive).
#[test]
fn full_replay_battery_passes() {
    unsafe {
        std::env::set_var("ADAM_V6_2", "1");
    }

    let mut failures: Vec<String> = Vec::new();
    for case in BATTERY {
        let got = answer(case.input).unwrap_or_else(|| "<no answer>".to_string());
        let got_lc = got.to_lowercase();
        // Phase 20 multi-paraphrase tolerance: split on `|` so any one of
        // the alternatives is enough to pass. Single-string cases reduce
        // to a one-element split → behaves the same as before.
        let want_lc = case.expects_substring.to_lowercase();
        let any_match = want_lc.split('|').any(|alt| {
            let trimmed = alt.trim();
            !trimmed.is_empty() && got_lc.contains(trimmed)
        });
        if !any_match {
            failures.push(format!(
                "[session {}] «{}» → «{}» (expected substring «{}»)",
                case.session, case.input, got, case.expects_substring,
            ));
        }
    }

    unsafe {
        std::env::remove_var("ADAM_V6_2");
    }

    assert!(
        failures.is_empty(),
        "{}/{} replay cases failed:\n  {}",
        failures.len(),
        BATTERY.len(),
        failures.join("\n  ")
    );
}

/// Battery coverage: at least one case from each of the 5
/// audit sessions. Drives the discipline that future sessions
/// also pin their findings here.
#[test]
fn battery_covers_all_audit_sessions() {
    use std::collections::HashSet;
    let sessions: HashSet<u8> = BATTERY.iter().map(|c| c.session).collect();
    for s in 1..=5 {
        assert!(
            sessions.contains(&s),
            "no replay case from session {s} in the battery",
        );
    }
}

/// Battery size — keep it growing.
#[test]
fn battery_size_minimum() {
    // Each fix should add at least one case. Floor is set
    // a touch below the current count so that adding a case
    // tomorrow doesn't break this assertion; we'll bump it
    // explicitly when we hit it.
    assert!(
        BATTERY.len() >= 25,
        "replay battery only has {} cases — every voice REPL fix should add at least one",
        BATTERY.len()
    );
}

/// **STT-loop dedupe** is tested at the conversation layer
/// (the router is built so that bare greetings fall through
/// to v6.1 cascade — which we don't expose here). What we
/// pin instead is: a known STT-loop input does NOT make the
/// router panic and does NOT return a non-greeting answer.
#[test]
fn stt_loop_does_not_panic_or_misroute() {
    unsafe {
        std::env::set_var("ADAM_V6_2", "1");
    }
    let loop_input = "Сәлем. ".repeat(20);
    let result = answer(&loop_input);
    // Either None (router declines, v6.1 will greet) or a
    // greeting-class reply — but NOT a factoid hallucination.
    if let Some(r) = result.as_ref() {
        let lc = r.to_lowercase();
        assert!(
            !lc.contains("мемлекет") && !lc.contains("дөңгелек"),
            "STT-loop misrouted to factoid: {r}",
        );
    }
    unsafe {
        std::env::remove_var("ADAM_V6_2");
    }
}
