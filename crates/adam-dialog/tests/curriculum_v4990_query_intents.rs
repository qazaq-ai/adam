//! **v4.99.0** — student-side curriculum-query intents.
//!
//! Tests `Intent::AskNextTopic` («Келесі қандай тақырыпты үйренсем?»)
//! and `Intent::AskCurrentProgress` («Менің прогресім қандай?»):
//!
//! 1. Detector positive cases (variety of natural Kazakh phrasings).
//! 2. Detector negative cases (statements / unrelated questions).
//! 3. Conversation slot population — `next_stage_*` for AskNextTopic;
//!    `progress_recap` (or `__progress_empty__`) for AskCurrentProgress.
//! 4. End-to-end multi-turn: empty-state recap → exercise+pass →
//!    partial recap with mixed statuses.
//! 5. Template-family presence + slot references.

use std::collections::HashMap;

use adam_dialog::{
    Conversation, Intent, TemplateRepository,
    curriculum::StageProgress,
    semantics::{detect_ask_current_progress, detect_ask_next_topic},
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

// ─── Detector positive / negative cases ──────────────────────────────

#[test]
fn detect_ask_next_topic_natural_phrasings() {
    assert!(detect_ask_next_topic("Келесі қандай тақырыпты үйренсем?"));
    assert!(detect_ask_next_topic("Енді нені үйренсем болады?"));
    assert!(detect_ask_next_topic("Әрі қарай не үйрену керек?"));
    assert!(detect_ask_next_topic("Келесі тақырып не?"));
    assert!(detect_ask_next_topic("Кейін не үйрену керек?"));
}

#[test]
fn detect_ask_next_topic_negatives() {
    // No question marker → not a query.
    assert!(!detect_ask_next_topic("Келесі тақырыпты үйрендім."));
    // No "advance" marker.
    assert!(!detect_ask_next_topic("Қандай тақырып бар?"));
    // No "learn" / "topic" marker.
    assert!(!detect_ask_next_topic("Келесі күн қалай болады?"));
}

#[test]
fn detect_ask_current_progress_natural_phrasings() {
    assert!(detect_ask_current_progress("Мен қай жерде тұрмын?"));
    assert!(detect_ask_current_progress("Менің прогресім қандай?"));
    assert!(detect_ask_current_progress("Қай тақырыпты бітірдім?"));
    assert!(detect_ask_current_progress("Қанша тақырыпты үйрендім?"));
    assert!(detect_ask_current_progress("Жалпы қалай тұр?"));
}

#[test]
fn detect_ask_current_progress_negatives() {
    // Unrelated question about `қай`.
    assert!(!detect_ask_current_progress("Қай күн бүгін?"));
    // Statement, not query.
    assert!(!detect_ask_current_progress("Прогресім жақсы."));
}

// ─── Intent dispatch — interpret_text routes to new intents ──────────

#[test]
fn interpret_routes_ask_next_topic() {
    use adam_dialog::semantics::interpret_text;
    let intent = interpret_text("Келесі қандай тақырыпты үйренсем?", &[]);
    assert!(
        matches!(intent, Intent::AskNextTopic),
        "expected AskNextTopic, got {intent:?}"
    );
}

#[test]
fn interpret_routes_ask_current_progress() {
    use adam_dialog::semantics::interpret_text;
    let intent = interpret_text("Менің прогресім қандай?", &[]);
    assert!(
        matches!(intent, Intent::AskCurrentProgress),
        "expected AskCurrentProgress, got {intent:?}"
    );
}

// ─── Template families ───────────────────────────────────────────────

#[test]
fn template_repo_has_next_topic_families() {
    let repo = repo();
    assert!(!repo.get("next_topic.suggestion").is_empty());
    assert!(!repo.get("next_topic.complete").is_empty());
}

#[test]
fn template_repo_has_current_progress_families() {
    let repo = repo();
    assert!(!repo.get("current_progress.recap").is_empty());
    assert!(!repo.get("current_progress.empty").is_empty());
}

#[test]
fn next_topic_suggestion_templates_use_next_stage_slots() {
    let repo = repo();
    for tmpl in repo.get("next_topic.suggestion") {
        assert!(
            tmpl.contains("{next_stage_label_kk}"),
            "every next_topic.suggestion template must reference {{next_stage_label_kk}}; got {tmpl:?}"
        );
    }
}

#[test]
fn current_progress_recap_templates_use_progress_recap_slot() {
    let repo = repo();
    for tmpl in repo.get("current_progress.recap") {
        assert!(
            tmpl.contains("{progress_recap}"),
            "every current_progress.recap template must reference {{progress_recap}}; got {tmpl:?}"
        );
    }
}

// ─── End-to-end: full conversation flow ──────────────────────────────

/// Empty-state — fresh conversation, AskCurrentProgress should hit
/// the empty branch and surface a "first stage" hint.
#[test]
fn fresh_conversation_progress_query_routes_to_empty() {
    let Some(lex) = lex() else { return };
    let repo = repo();
    let mut conv = Conversation::new();
    if conv.curriculum.is_none() {
        return;
    }
    let out = conv.turn("Менің прогресім қандай?", &lex, &repo, 0);
    assert!(
        out.contains("Әзірге бір де жаттығу шешілмеген") || out.contains("Курсты әлі бастамадыңыз"),
        "empty progress query must surface empty template; got: {out}"
    );
    assert!(
        out.contains("Иелік"),
        "empty template must mention the first stage label; got: {out}"
    );
}

#[test]
fn fresh_conversation_next_topic_query_recommends_first_stage() {
    let Some(lex) = lex() else { return };
    let repo = repo();
    let mut conv = Conversation::new();
    if conv.curriculum.is_none() {
        return;
    }
    let out = conv.turn("Келесі қандай тақырыпты үйренсем?", &lex, &repo, 0);
    assert!(
        out.contains("Иелік"),
        "fresh-state next-topic query must recommend ownership (first stage); got: {out}"
    );
}

/// After partial progress — recap must list every stage with mixed
/// statuses (closed / current / locked).
#[test]
fn partial_progress_recap_shows_mixed_statuses() {
    let Some(lex) = lex() else { return };
    let repo = repo();
    let mut conv = Conversation::new();
    if conv.curriculum.is_none() {
        return;
    }
    // Hand-craft progress: ownership closed (2/2), borrow halfway (1/2).
    let mut progress = HashMap::new();
    progress.insert(
        "ownership".into(),
        StageProgress {
            passed: 2,
            failed: 0,
        },
    );
    progress.insert(
        "borrow".into(),
        StageProgress {
            passed: 1,
            failed: 0,
        },
    );
    conv.curriculum_progress = progress;

    let out = conv.turn("Менің прогресім қандай?", &lex, &repo, 0);
    assert!(
        out.contains("**Иелік** ✓"),
        "ownership must show closed; got: {out}"
    );
    assert!(
        out.contains("**Қарыз алу** (1/2"),
        "borrow must show in-progress; got: {out}"
    );
    assert!(
        out.contains("**Өмір кезеңі** ⊘"),
        "lifetime must show locked; got: {out}"
    );
}

/// After ownership closed, AskNextTopic should advance to borrow.
#[test]
fn next_topic_query_after_ownership_closed_recommends_borrow() {
    let Some(lex) = lex() else { return };
    let repo = repo();
    let mut conv = Conversation::new();
    if conv.curriculum.is_none() {
        return;
    }
    let mut progress = HashMap::new();
    progress.insert(
        "ownership".into(),
        StageProgress {
            passed: 2,
            failed: 0,
        },
    );
    conv.curriculum_progress = progress;
    let out = conv.turn("Келесі қандай тақырыпты үйренсем?", &lex, &repo, 0);
    assert!(
        out.contains("Қарыз алу"),
        "next-topic after ownership closed must recommend borrow; got: {out}"
    );
}

/// When the entire curriculum is complete, AskNextTopic must surface
/// the `next_topic.complete` celebratory template.
#[test]
fn next_topic_query_when_curriculum_complete_celebrates() {
    let Some(lex) = lex() else { return };
    let repo = repo();
    let mut conv = Conversation::new();
    let curriculum = match conv.curriculum.as_ref() {
        Some(c) => c.clone(),
        None => return,
    };
    let mut progress = HashMap::new();
    for s in &curriculum.stages {
        progress.insert(
            s.id.clone(),
            StageProgress {
                passed: s.exercises_to_pass,
                failed: 0,
            },
        );
    }
    conv.curriculum_progress = progress;

    let out = conv.turn("Келесі қандай тақырыпты үйренсем?", &lex, &repo, 0);
    assert!(
        out.contains("барлық тақырыптарды бітіріп") || out.contains("Жалпы курс бітті"),
        "completed curriculum must surface celebration template; got: {out}"
    );
}
