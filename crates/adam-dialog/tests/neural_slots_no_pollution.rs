// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! E2 — neural slot extractor false-positive regression test.
//!
//! Audit 2026-05-21 (manual `adam_chat` runs with
//! `ADAM_NEURAL_SLOTS=1`) found that the extractor was firing on
//! turns whose intent could not semantically carry a personal
//! slot (greetings, thanks, ice-breakers), polluting session
//! state:
//!
//!   - «сәлеметсіз бе»        → `name = "Бе"`
//!   - «рахмет»               → `occupation = "рахмет"` (later)
//!   - «сәлеметсіз бе, танысайық» → `occupation = "танысайық"`
//!
//! Fix lives in [`adam_dialog::Conversation::apply_neural_slot_rescue`]
//! plus the intent gate at the rescue call site. This test pins
//! the negative-cases contract: with the env flag on and a real
//! extractor attached, these turns must leave `session` empty.
//!
//! The test loads the actual production artefact
//! `data/slot_extractor/v1/rung_a.json`. If the file is missing
//! (developer hasn't built the dataset yet), the test skips.

use std::sync::Mutex;

use adam_dialog::{Conversation, TemplateRepository};
use adam_kernel_fst::lexicon::LexiconV1;
use adam_slot_extractor::SlotExtractor;

/// Serialise tests that mutate `ADAM_NEURAL_SLOTS` — `std::env`
/// is process-global and cargo test runs in parallel.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn load_lex() -> LexiconV1 {
    let curated = "../../data/tokenizer/segmentation_roots.json";
    let apertium = "../../data/lexicon_v1/apertium_imported_roots.json";
    LexiconV1::load(curated, apertium).expect("lexicon load failed")
}

fn load_repo() -> TemplateRepository {
    TemplateRepository::load_default().expect("templates v1.toml must exist")
}

fn load_extractor() -> Option<SlotExtractor> {
    let path = "../../data/slot_extractor/v1/rung_a.json";
    if !std::path::Path::new(path).exists() {
        eprintln!("E2 model not present at {path} — skipping");
        return None;
    }
    SlotExtractor::from_path(path).ok()
}

#[test]
fn neural_slots_do_not_pollute_on_greeting_thanks_or_ice_breaker_turns() {
    let _guard = ENV_LOCK.lock().unwrap();
    let lex = load_lex();
    let repo = load_repo();
    let extractor = match load_extractor() {
        Some(e) => e,
        None => return,
    };
    // SAFETY: in-test global env mutation, gated by ENV_LOCK.
    unsafe {
        std::env::set_var("ADAM_NEURAL_SLOTS", "1");
    }
    // Each case: (input, slot_that_must_remain_empty,
    // reason_audit_id). On audit failure we report all three so
    // a regression is diagnosable from the panic message alone.
    let cases: &[(&str, &[&str], &str)] = &[
        ("сәлеметсіз бе", &["name", "occupation"], "greeting"),
        ("рахмет", &["name", "occupation"], "thanks"),
        (
            "сәлеметсіз бе, танысайық",
            &["name", "occupation"],
            "greeting+ice-breaker",
        ),
        ("кешіріңіз", &["name", "occupation"], "apology"),
        ("сау болыңыз", &["name", "occupation"], "farewell"),
        ("жақсы", &["name", "occupation"], "affirmation"),
        ("қалайсыз?", &["name", "occupation"], "how-are-you"),
    ];
    let mut failures: Vec<String> = Vec::new();
    for (input, slots, audit_id) in cases {
        let mut conv = Conversation::new().with_neural_slot_extractor(extractor.clone());
        let _ = conv.turn(input, &lex, &repo, 0);
        for slot in *slots {
            if let Some(v) = conv.session_value(slot) {
                failures.push(format!(
                    "[{}] «{}» → session.{} = {:?} (expected: empty)",
                    audit_id, input, slot, v
                ));
            }
        }
    }
    // SAFETY: in-test global env mutation, gated by ENV_LOCK.
    unsafe {
        std::env::remove_var("ADAM_NEURAL_SLOTS");
    }
    assert!(
        failures.is_empty(),
        "neural slot extractor polluted session on non-slot-bearing turns:\n  - {}",
        failures.join("\n  - ")
    );
}

#[test]
fn neural_slots_still_fire_on_legitimate_statement_of_name() {
    // Positive control — the gate must NOT block legitimate
    // slot-bearing turns. This is the rescue case E2 was
    // designed for: cascade may or may not recognise the OOV
    // name, but the extractor should be allowed to try.
    let _guard = ENV_LOCK.lock().unwrap();
    let lex = load_lex();
    let repo = load_repo();
    let extractor = match load_extractor() {
        Some(e) => e,
        None => return,
    };
    // SAFETY: in-test global env mutation, gated by ENV_LOCK.
    unsafe {
        std::env::set_var("ADAM_NEURAL_SLOTS", "1");
    }
    let mut conv = Conversation::new().with_neural_slot_extractor(extractor);
    let _ = conv.turn("менің атым Қанат", &lex, &repo, 0);
    let name = conv.session_value("name");
    // SAFETY: in-test global env mutation, gated by ENV_LOCK.
    unsafe {
        std::env::remove_var("ADAM_NEURAL_SLOTS");
    }
    assert!(
        name.is_some() && name.as_deref().map(|s| s.to_lowercase().contains("қанат")) == Some(true),
        "positive control failed — name slot not filled for «менің атым Қанат»: got {:?}",
        name
    );
}
