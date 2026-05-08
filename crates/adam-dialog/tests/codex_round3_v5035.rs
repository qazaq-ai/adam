//! **v5.3.5** — Codex round-3 audit fixes (pass 3 — compound-statement
//! absorption + occupation self-recall).
//!
//! Closes the post-v5.3.0 user-reported gap: «Менің атым Дәулет,
//! мамандығым бағдарламашы.» — pre-fix only the name was absorbed,
//! occupation was lost; «Менің мамандығым есіңізде ме?» pre-fix fell
//! through to Unknown surface with a generic «Мамандық — адамның
//! кәсібі» definition.

use adam_dialog::{Conversation, TemplateRepository, semantics::extract_secondary_profile_facts};
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

// ─── extract_secondary_profile_facts ───────────────────────────────────

#[test]
fn secondary_extraction_finds_occupation_after_name() {
    let facts = extract_secondary_profile_facts("Менің атым Дәулет, мамандығым бағдарламашы.");
    assert!(facts.contains(&("occupation".into(), "бағдарламашы".into())));
}

#[test]
fn secondary_extraction_finds_кәсібім_variant() {
    let facts = extract_secondary_profile_facts("атым Дәулет, кәсібім инженер.");
    assert!(facts.contains(&("occupation".into(), "инженер".into())));
}

#[test]
fn secondary_extraction_finds_age() {
    let facts = extract_secondary_profile_facts("Менің атым Дәулет, жасым 30.");
    assert!(facts.contains(&("age".into(), "30".into())));
}

#[test]
fn secondary_extraction_skips_filler_words() {
    // «мамандығым болып бағдарламашы» should NOT capture «болып» as
    // value (filler word); should skip to «бағдарламашы».
    let facts = extract_secondary_profile_facts("мамандығым болып бағдарламашы істеймін.");
    assert!(
        !facts.iter().any(|(_, v)| v == "болып"),
        "filler «болып» must not be captured as value; got: {facts:?}"
    );
}

#[test]
fn secondary_extraction_empty_when_no_pattern() {
    assert!(extract_secondary_profile_facts("").is_empty());
    assert!(extract_secondary_profile_facts("Сәлем!").is_empty());
    assert!(extract_secondary_profile_facts("Менің атым Дәулет.").is_empty());
}

// ─── End-to-end: compound profile statement absorbs both facts ─────────

#[test]
fn compound_name_and_occupation_absorbs_both() {
    let Some(lex) = lex() else { return };
    let repo = repo();
    let mut conv = Conversation::new();

    let _ = conv.turn(
        "Менің атым Дәулет, мамандығым бағдарламашы.",
        &lex,
        &repo,
        0,
    );

    assert_eq!(
        conv.session.get("name").map(String::as_str),
        Some("Дәулет"),
        "name must be absorbed"
    );
    assert_eq!(
        conv.session.get("occupation").map(String::as_str),
        Some("бағдарламашы"),
        "occupation must be absorbed from the second clause"
    );
}

#[test]
fn occupation_self_recall_after_compound_surfaces_value() {
    let Some(lex) = lex() else { return };
    let repo = repo();
    let mut conv = Conversation::new();

    let _ = conv.turn(
        "Менің атым Дәулет, мамандығым бағдарламашы.",
        &lex,
        &repo,
        0,
    );
    let out = conv.turn("Менің мамандығым есіңізде ме?", &lex, &repo, 1);

    assert!(
        out.contains("бағдарламашы"),
        "occupation self-recall must surface stored value; got: {out}"
    );
    // Must NOT surface the generic definition «Мамандық — адамның кәсібі».
    assert!(
        !out.contains("Мамандық — адамның кәсібі"),
        "self-recall must NOT fall through to definition; got: {out}"
    );
}

#[test]
fn occupation_self_recall_with_full_compound_dialog() {
    // Full reproduction of the user's reported scenario.
    let Some(lex) = lex() else { return };
    let repo = repo();
    let mut conv = Conversation::new();

    let _ = conv.turn("Сәлем!", &lex, &repo, 0);
    let _ = conv.turn(
        "Менің атым Дәулет, мамандығым бағдарламашы.",
        &lex,
        &repo,
        1,
    );
    // Several turns later (simulate the user's longer dialog).
    let _ = conv.turn("Қалыңыз қалай?", &lex, &repo, 2);
    let out = conv.turn("Менің мамандығым есіңізде ме?", &lex, &repo, 3);

    assert!(
        out.contains("бағдарламашы"),
        "occupation must persist + recall after intermediate turns; got: {out}"
    );
}

#[test]
fn name_only_input_does_not_extract_occupation() {
    let Some(lex) = lex() else { return };
    let repo = repo();
    let mut conv = Conversation::new();

    let _ = conv.turn("Менің атым Дәулет.", &lex, &repo, 0);
    assert_eq!(conv.session.get("name").map(String::as_str), Some("Дәулет"));
    // No occupation in input → no occupation in session.
    assert!(
        conv.session.get("occupation").is_none(),
        "no occupation in input → session must not have one; got: {:?}",
        conv.session.get("occupation")
    );
}
