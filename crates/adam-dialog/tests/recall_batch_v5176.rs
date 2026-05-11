//! **v5.17.6 — adversarial D1 quick-win batch.**
//!
//! Closes 2 remaining trigger-extension cases from the v5.17.0
//! benchmark queue. After v5.17.5's anaphora paradigm batch
//! (mta_03/05/08) closed the heavy work, only two failures
//! remained that fit «extend a trigger list» pattern:
//!
//! - `mta_02`: «Жасымды білесіз бе?» — age recall via 1sg-poss-ACC
//!   + memory-probe. Two-part fix because pre-v5.17.6 even Turn 1
//!   («Маған 25 жас») wasn't recognised as StatementOfAge (the
//!   dative-experiential shape «Маған N жас» wasn't in the
//!   matcher). Fixing only the recall detector would have left
//!   session.age unset and the recall would still return the
//!   system-self template.
//!
//! - `ctt_04`: «traits деген не үшін керек?» — purpose detector
//!   was catching «не үшін арналған» (intended for) and «неге
//!   арналған» but not «не үшін керек» (needed for) / «неге
//!   керек». Semantic distinction is null in the tutor context.
//!
//! Adversarial baseline 46/50 (92%) → 48/50 (96%); five of six
//! categories now at 100%.

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
fn mta_02_age_recall_intent_routes_to_ask_age_v5176() {
    let intent = interpret_text("Жасымды білесіз бе?", &[]);
    assert!(
        matches!(intent, Intent::AskAge),
        "age memory-probe recall must route to AskAge; got: {intent:?}"
    );
}

#[test]
fn mta_02_dative_age_statement_routes_to_statement_of_age_v5176() {
    let intent = interpret_text("Маған 25 жас.", &[]);
    assert!(
        matches!(intent, Intent::StatementOfAge { years: Some(25) }),
        "«Маған N жас» dative-experiential must route to StatementOfAge {{ years: Some(N) }}; got: {intent:?}"
    );
}

#[test]
fn end_to_end_mta_02_recalls_session_age_v5176() {
    let Some(lex) = lex() else {
        eprintln!("skip: lexicon missing");
        return;
    };
    let repo = repo();
    let mut conv = Conversation::new();
    let _ = conv.turn("Маған 25 жас.", &lex, &repo, 0);
    let response = conv.turn("Жасымды білесіз бе?", &lex, &repo, 0);
    assert!(
        response.contains("25"),
        "post-Statement age recall must echo session.age; got: {response}"
    );
}

#[test]
fn ctt_04_purpose_detector_catches_не_үшін_керек_v5176() {
    let intent = interpret_text("traits деген не үшін керек?", &[]);
    match intent {
        Intent::AskPurpose { topic } => {
            assert_eq!(
                topic.as_deref(),
                Some("traits"),
                "AskPurpose must extract `traits` as topic; got: {topic:?}"
            );
        }
        other => panic!("expected AskPurpose, got {other:?}"),
    }
}

#[test]
fn ctt_04_purpose_detector_catches_неге_керек_v5176() {
    // Sibling alias: «X неге керек?» = «what is X needed for?».
    let intent = interpret_text("ownership неге керек?", &[]);
    assert!(
        matches!(intent, Intent::AskPurpose { .. }),
        "«неге керек?» must route to AskPurpose; got: {intent:?}"
    );
}

#[test]
fn factual_age_statement_unchanged_v5176() {
    // Regression guard: the pre-existing «Менің жасым 30» shape
    // must still route to StatementOfAge { years: Some(30) }.
    let intent = interpret_text("Менің жасым 30.", &[]);
    assert!(
        matches!(intent, Intent::StatementOfAge { years: Some(30) }),
        "«Менің жасым N» must still route to StatementOfAge; got: {intent:?}"
    );
}
