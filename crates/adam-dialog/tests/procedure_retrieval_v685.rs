// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! Integration test for the v6.8.5 L4.6 procedure retrieval
//! layer: drives `Conversation::turn` through realistic pilot-
//! style Kazakh + Russian procedure queries and asserts the
//! cascade routes each to its expected fixture.
//!
//! Also verifies unrelated inputs (math, factual lookup) fall
//! through to the existing cascade unchanged.

use adam_dialog::{Conversation, TemplateRepository};
use adam_kernel_fst::lexicon::LexiconV1;

fn load_lexicon() -> Option<LexiconV1> {
    let curated = "../../data/tokenizer/segmentation_roots.json";
    let apertium = "../../data/lexicon_v1/apertium_imported_roots.json";
    if !std::path::Path::new(curated).exists() || !std::path::Path::new(apertium).exists() {
        eprintln!("[procedure_retrieval_v685] lexicon not present, skipping");
        return None;
    }
    LexiconV1::load(curated, apertium).ok()
}

fn load_repo() -> TemplateRepository {
    TemplateRepository::load_default().expect("templates v1.toml must exist")
}

fn enable_v6_2() {
    unsafe {
        std::env::set_var("ADAM_V6_2", "1");
    }
}

#[test]
fn ppe_issuance_query_routes_to_kk_labor_ppe_002() {
    enable_v6_2();
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let reply = conv.turn("СИЗ беру тәртібі қандай?", &lex, &repo, 0);
    assert!(
        reply.contains("Жеке қорғаныс құралдарын беру тәртібі")
            && reply.contains("Қадамдар:")
            && reply.contains("Дереккөз:"),
        "expected the PPE-issuance procedure response, got: {reply}",
    );
}

#[test]
fn primary_briefing_query_routes_to_kk_labor_intro_001() {
    enable_v6_2();
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let reply = conv.turn("Бастапқы нұсқаулықты қалай жүргізеді?", &lex, &repo, 0);
    assert!(
        reply.contains("Жұмысқа алу кезіндегі бастапқы нұсқаулық"),
        "expected the primary-briefing procedure response, got: {reply}",
    );
}

#[test]
fn height_work_query_routes_to_kk_construction_height_005() {
    enable_v6_2();
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let reply = conv.turn("Биіктікте жұмыс жүргізу тәртібі қандай?", &lex, &repo, 0);
    assert!(
        reply.contains("Биіктікте жұмыс жүргізу тәртібі") && reply.contains("Қауіптер:"),
        "expected the height-work procedure (with hazards block), got: {reply}",
    );
}

#[test]
fn math_query_falls_through_unchanged() {
    enable_v6_2();
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let reply = conv.turn("2+2 қанша?", &lex, &repo, 0);
    // Math handler still owns this — must NOT have been hijacked
    // by the procedure layer.
    assert!(
        !reply.contains("Рәсім:"),
        "math query must not route to the procedure handler, got: {reply}",
    );
    assert!(
        reply.contains("4"),
        "math handler should still answer, got: {reply}",
    );
}

#[test]
fn factual_query_falls_through_unchanged() {
    enable_v6_2();
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let reply = conv.turn("Қазақстанның астанасы қай қала?", &lex, &repo, 0);
    assert!(
        !reply.contains("Рәсім:"),
        "factual capital query must not route to procedure handler, got: {reply}",
    );
    assert!(
        reply.contains("Астана"),
        "capital answer should still surface, got: {reply}",
    );
}
