//! **v5.10.0 — Codex follow-up review (B3) integration tests.**
//!
//! Confirms the AskFixPreviousError follow-up route: after a failed
//! SubmitSolution caches `last_cargo_error_code` /
//! `last_error_explanation` / `last_submission_topic` in session, the
//! user can ask «Оны қалай түзетемін?» / «Қалай шешемін?» / «Түзетілген
//! кодты бер.» across multiple turns and adam carries the error
//! context — instead of falling to retrieval on the literal token
//! «оны» or «болд» (the v5.10.0 regression Codex caught).
//!
//! Pre-seeded session state — the cargo verify path is exercised by
//! `submit_solution_integration` (and is slow / ignored by default).
//! Here we set session keys directly to test the **dialog handler**
//! without recompiling Rust on every run.

use std::path::Path;

use adam_dialog::{Conversation, TemplateRepository};
use adam_kernel_fst::lexicon::LexiconV1;

fn load_lexicon() -> LexiconV1 {
    let curated = "../../data/tokenizer/segmentation_roots.json";
    let apertium = "../../data/lexicon_v1/apertium_imported_roots.json";
    assert!(
        Path::new(curated).exists(),
        "ask_fix_previous_error_v5100 requires lexicon at {curated}"
    );
    LexiconV1::load(curated, apertium).expect("ask_fix_previous_error_v5100: lexicon load failed")
}

fn load_repo() -> TemplateRepository {
    TemplateRepository::load_default().expect("templates v1.toml must exist")
}

fn seeded_conversation_with_error(code: &str, explanation: &str, topic: &str) -> Conversation {
    let mut conv = Conversation::new();
    conv.session
        .insert("last_cargo_error_code".into(), code.into());
    conv.session
        .insert("last_error_explanation".into(), explanation.into());
    conv.session
        .insert("last_submission_topic".into(), topic.into());
    conv
}

#[test]
fn ask_fix_previous_error_e0382_carries_clone_or_reference_hint_v5100() {
    let lex = load_lexicon();
    let repo = load_repo();
    let mut conv = seeded_conversation_with_error("E0382", "borrow of moved value", "ownership");
    let response = conv.turn("Оны қалай түзетемін?", &lex, &repo, 0);
    let lower = response.to_lowercase();
    assert!(
        lower.contains("e0382"),
        "expected E0382 in fix-error response, got: {response}"
    );
    assert!(
        lower.contains("clone") || lower.contains("reference") || lower.contains("сілтеме"),
        "expected repair hint (clone / reference / сілтеме), got: {response}"
    );
}

#[test]
fn ask_fix_previous_error_carries_error_index_link_v5100() {
    let lex = load_lexicon();
    let repo = load_repo();
    let mut conv =
        seeded_conversation_with_error("E0107", "wrong number of generic args", "generics");
    // E0107 has no specialised family — should fall through to
    // `ask_fix_previous_error.with_data` and surface the cached
    // explanation + Rust error-index pointer.
    let response = conv.turn("Түзетілген кодты бер.", &lex, &repo, 0);
    let lower = response.to_lowercase();
    assert!(
        lower.contains("e0107"),
        "expected E0107 in fix-error response, got: {response}"
    );
    assert!(
        lower.contains("error-index")
            || lower.contains("rust-lang.org")
            || lower.contains("түзетілген"),
        "expected error-index pointer or repair phrasing, got: {response}"
    );
}

#[test]
fn ask_fix_previous_error_empty_session_returns_honest_refusal_v5100() {
    let lex = load_lexicon();
    let repo = load_repo();
    let mut conv = Conversation::new();
    let response = conv.turn("Қалай түзетемін?", &lex, &repo, 0);
    let lower = response.to_lowercase();
    // Empty-session response must be honest about the missing
    // context — not a retrieval surface or a generic curriculum reply.
    assert!(
        lower.contains("сақтаулы қате жоқ")
            || lower.contains("кодты ұсынсаңыз")
            || lower.contains("сәтсіз тапсырыс жоқ")
            || lower.contains("кодты бере салыңыз"),
        "expected empty-session honest refusal, got: {response}"
    );
}

#[test]
fn two_consecutive_followups_keep_error_context_v5100() {
    // Codex's regression scenario: two follow-ups after the same
    // failed submission must BOTH retain the error context.
    let lex = load_lexicon();
    let repo = load_repo();
    let mut conv = seeded_conversation_with_error("E0382", "borrow of moved value", "ownership");

    // First follow-up — recall question.
    let r1 = conv.turn("Ал алдыңғы қате неде болды?", &lex, &repo, 0);
    let r1_lower = r1.to_lowercase();
    assert!(
        r1_lower.contains("e0382"),
        "first follow-up must surface E0382, got: {r1}"
    );

    // Second follow-up — fix question. Pre-v5.10.0 fell to retrieval
    // on the anaphor «оны» / fix verb «түзет-» without carrying the
    // cached error context.
    let r2 = conv.turn("Оны қалай түзетемін?", &lex, &repo, 0);
    let r2_lower = r2.to_lowercase();
    assert!(
        r2_lower.contains("e0382"),
        "second follow-up must STILL carry E0382 (persistence bug), got: {r2}"
    );
    assert!(
        r2_lower.contains("clone")
            || r2_lower.contains("reference")
            || r2_lower.contains("сілтеме"),
        "second follow-up must include repair hint, got: {r2}"
    );
}
