//! **v5.12.5 — Codex follow-up review (B5.3) integration tests.**
//!
//! Confirms the corrected-code surface: per-error-code template
//! families ship a runnable repaired snippet; session counter
//! `fix_example_idx` rotates between two complementary repair
//! strategies on follow-up turns («Тағы бір мысал бер»); generic
//! E0107 (no specialised family) still falls back to with_data.

use std::path::Path;

use adam_dialog::{Conversation, TemplateRepository};
use adam_kernel_fst::lexicon::LexiconV1;

fn load_lexicon() -> LexiconV1 {
    let curated = "../../data/tokenizer/segmentation_roots.json";
    let apertium = "../../data/lexicon_v1/apertium_imported_roots.json";
    assert!(
        Path::new(curated).exists(),
        "ask_fix_corrected_code_v5125 requires lexicon at {curated}"
    );
    LexiconV1::load(curated, apertium).expect("ask_fix_corrected_code_v5125: lexicon load failed")
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
    conv.session.insert(
        "last_code_snippet".into(),
        "let s = String::from(\"hi\"); let t = s; println!(\"{}\", s);".into(),
    );
    conv.session.insert("fix_example_idx".into(), "0".into());
    conv
}

#[test]
fn e0382_first_request_yields_clone_snippet_v5125() {
    let lex = load_lexicon();
    let repo = load_repo();
    let mut conv = seeded_conversation_with_error("E0382", "borrow of moved value", "ownership");
    let response = conv.turn("Түзетілген кодты көрсет.", &lex, &repo, 0);
    let lower = response.to_lowercase();
    // Variant 0 is the clone-based repair.
    assert!(
        lower.contains("e0382"),
        "expected E0382 in response, got: {response}"
    );
    assert!(
        lower.contains("clone"),
        "expected clone-based repair on first request, got: {response}"
    );
    assert!(
        lower.contains("```rust"),
        "expected Rust code fence in response, got: {response}"
    );
}

#[test]
fn e0382_pre_seeded_idx_1_renders_reference_snippet_v5125() {
    // Pre-seed counter at 1 (simulating a session where the user
    // already saw the clone variant). A non-rotating request like
    // «Қалай түзетемін?» keeps the counter stable, so the planner
    // picks variant 1 (reference-based repair).
    let lex = load_lexicon();
    let repo = load_repo();
    let mut conv = seeded_conversation_with_error("E0382", "borrow of moved value", "ownership");
    conv.session.insert("fix_example_idx".into(), "1".into());
    let response = conv.turn("Қалай түзетемін?", &lex, &repo, 0);
    let lower = response.to_lowercase();
    assert!(
        lower.contains("e0382"),
        "expected E0382 in second-variant response, got: {response}"
    );
    assert!(
        lower.contains("&s") || lower.contains("сілтеме"),
        "expected reference-based repair on second variant, got: {response}"
    );
}

#[test]
fn e0596_corrected_code_carries_let_mut_v5125() {
    let lex = load_lexicon();
    let repo = load_repo();
    let mut conv = seeded_conversation_with_error("E0596", "cannot borrow as mutable", "borrow");
    let response = conv.turn("Түзетілген кодты бер.", &lex, &repo, 0);
    let lower = response.to_lowercase();
    assert!(
        lower.contains("e0596"),
        "expected E0596 in response, got: {response}"
    );
    assert!(
        lower.contains("let mut"),
        "expected `let mut` repair on first variant, got: {response}"
    );
}

#[test]
fn unspecialised_error_still_falls_back_to_with_data_v5125() {
    // E0107 has no `with_corrected_code.E0107` family; planner should
    // fall through to `ask_fix_previous_error.with_data`.
    let lex = load_lexicon();
    let repo = load_repo();
    let mut conv =
        seeded_conversation_with_error("E0107", "wrong number of generic args", "generics");
    let response = conv.turn("Қалай түзетемін?", &lex, &repo, 0);
    let lower = response.to_lowercase();
    assert!(
        lower.contains("e0107"),
        "expected E0107 in fallback response, got: {response}"
    );
    // with_data templates point at the Rust error-index docs.
    assert!(
        lower.contains("rust-lang.org") || lower.contains("түзетілген"),
        "expected error-index link or repair phrasing, got: {response}"
    );
}

#[test]
fn rotation_via_taghy_increments_counter_v5125() {
    // End-to-end repeat-detection: «Тағы бір мысал бер» on a fresh
    // session must increment the counter and rotate to the second
    // variant — even though we start at idx=0.
    let lex = load_lexicon();
    let repo = load_repo();
    let mut conv = seeded_conversation_with_error("E0382", "borrow of moved value", "ownership");

    // First: «Қалай түзетемін?» — counter stays at 0, clone variant.
    let r1 = conv.turn("Қалай түзетемін?", &lex, &repo, 0);
    assert!(
        r1.to_lowercase().contains("clone"),
        "expected clone variant on first turn, got: {r1}"
    );
    assert_eq!(
        conv.session.get("fix_example_idx").map(String::as_str),
        Some("0"),
        "counter should stay at 0 on first non-rotation turn"
    );

    // Second: «Тағы бір мысал бер» — counter increments, reference variant.
    let r2 = conv.turn("Тағы бір мысал бер.", &lex, &repo, 0);
    assert_eq!(
        conv.session.get("fix_example_idx").map(String::as_str),
        Some("1"),
        "counter should increment on rotation request"
    );
    assert!(
        r2.to_lowercase().contains("&s") || r2.to_lowercase().contains("сілтеме"),
        "expected reference variant after rotation, got: {r2}"
    );
}
