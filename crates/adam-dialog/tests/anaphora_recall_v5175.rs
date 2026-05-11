//! **v5.17.5 — adversarial D1 anaphora batch closure.**
//!
//! Codex 2026-05-11 audit priority D, adversarial benchmark cases
//! mta_03 / mta_05 / mta_08 — all three failed with the same
//! paradigm: turn 1 establishes a session slot via Statement* intent;
//! turn 2 asks the system to recall it using a 1sg-PART-ACC verb
//! form («тұратынымды», «келгенімді», «істейтінімді») paired with a
//! memory-probe verb («білесіз бе», «есіңізде ме»). The recall-
//! question detectors (`detect_ask_location` / `detect_ask_occupation`)
//! covered direct 1sg-PRES verbs (`тұрамын`, `қайдамын`) but not
//! the nominalised + accusative + memory-probe shape.
//!
//! Fix: extend the two detectors with a memory-probe branch, exact
//! mirror of the v4.54.5 `detect_ask_name` memory-probe extension
//! that closed «менің атымды есіңізде ме?». Adversarial baseline
//! 43/50 → 46/50 (86% → 92%); multi_turn_anaphora 3/8 → 6/8.

use adam_dialog::{Conversation, Intent, TemplateRepository, semantics::interpret_text};
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

#[test]
fn mta_03_city_anaphora_routes_to_ask_location_v5175() {
    let intent = interpret_text("Қай қалада тұратынымды білесіз бе?", &[]);
    assert!(
        matches!(intent, Intent::AskLocation),
        "city memory-probe recall must route to AskLocation; got: {intent:?}"
    );
}

#[test]
fn mta_05_occupation_anaphora_routes_to_ask_occupation_v5175() {
    let intent = interpret_text("Кім болып жұмыс істейтінімді есіңізде ме?", &[]);
    assert!(
        matches!(intent, Intent::AskOccupation),
        "occupation memory-probe recall must route to AskOccupation; got: {intent:?}"
    );
}

#[test]
fn mta_08_origin_anaphora_routes_to_ask_location_v5175() {
    // «Қайдан келгенімді білесіз бе?» — origin recall. Same target
    // session slot (session.city) as residence; AskLocation handles
    // both via the same recall path.
    let intent = interpret_text("Қайдан келгенімді білесіз бе?", &[]);
    assert!(
        matches!(intent, Intent::AskLocation),
        "origin memory-probe recall must route to AskLocation; got: {intent:?}"
    );
}

#[test]
fn end_to_end_mta_03_recalls_session_city_v5175() {
    let Some(lex) = lex() else {
        eprintln!("skip: lexicon missing");
        return;
    };
    let repo = repo();
    let mut conv = Conversation::new();
    let _ = conv.turn("Мен Алматыда тұрамын.", &lex, &repo, 0);
    let response = conv.turn("Қай қалада тұратынымды білесіз бе?", &lex, &repo, 0);
    let lower = response.to_lowercase();
    assert!(
        lower.contains("алматы"),
        "post-Statement city recall must surface session.city; got: {response}"
    );
}

#[test]
fn end_to_end_mta_05_recalls_session_occupation_v5175() {
    let Some(lex) = lex() else {
        eprintln!("skip: lexicon missing");
        return;
    };
    let repo = repo();
    let mut conv = Conversation::new();
    let _ = conv.turn("Мен бағдарламашымын.", &lex, &repo, 0);
    let response = conv.turn("Кім болып жұмыс істейтінімді есіңізде ме?", &lex, &repo, 0);
    let lower = response.to_lowercase();
    assert!(
        lower.contains("бағдарламашы"),
        "post-Statement occupation recall must surface session.occupation; got: {response}"
    );
}

#[test]
fn end_to_end_mta_08_recalls_session_origin_v5175() {
    let Some(lex) = lex() else {
        eprintln!("skip: lexicon missing");
        return;
    };
    let repo = repo();
    let mut conv = Conversation::new();
    let _ = conv.turn("Мен Астанадан келдім.", &lex, &repo, 0);
    let response = conv.turn("Қайдан келгенімді білесіз бе?", &lex, &repo, 0);
    let lower = response.to_lowercase();
    assert!(
        lower.contains("астана"),
        "post-Statement origin recall must surface session.city; got: {response}"
    );
}

#[test]
fn factual_city_question_still_routes_factually_v5175() {
    // Regression guard: a non-recall factual question about a city
    // («Алматы қай елде орналасқан?» — not a self-recall) must NOT
    // be misclassified as AskLocation. The memory-probe extension
    // requires BOTH the nominalised 1sg verb AND a memory-probe
    // verb; this query has neither, so it should stay Unknown.
    let intent = interpret_text("Алматы қай елде орналасқан?", &[]);
    assert!(
        !matches!(intent, Intent::AskLocation),
        "factual city question must NOT route to AskLocation; got: {intent:?}"
    );
}
