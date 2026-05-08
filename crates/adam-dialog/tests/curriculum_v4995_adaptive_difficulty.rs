//! **v4.99.5** — adaptive-difficulty wiring (long-term roadmap step 4).
//!
//! Tests that `pedagogical::exercise_for_with_hint` consumes
//! `StageProgress::difficulty_hint` (added in v4.98.5) to scale
//! exercise selection within a curriculum stage:
//!
//! 1. Easy/Normal/Hard variants exist for the 5 canonical stages.
//! 2. Normal hint (or unknown topic) falls back to canonical
//!    `exercise_for` content.
//! 3. Conversation pre-stuffs the tailored body via `extra_slots`
//!    when AskExercise(topic-in-curriculum) fires.
//! 4. End-to-end: clean pass on ownership shifts difficulty to Hard;
//!    next AskExercise surfaces the Hard variant.
//! 5. Failure-rate: 2+ failed passes shifts difficulty to Easy.

use adam_dialog::{
    Conversation, TemplateRepository,
    curriculum::{DifficultyHint, StageProgress},
    pedagogical::{exercise_for, exercise_for_with_hint},
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

// ─── exercise_for_with_hint API ──────────────────────────────────────

#[test]
fn hint_normal_falls_back_to_canonical_exercise() {
    let canonical = exercise_for("ownership").expect("ownership has canonical");
    let normal = exercise_for_with_hint("ownership", DifficultyHint::Normal)
        .expect("ownership Normal must exist");
    assert_eq!(canonical, normal);
}

#[test]
fn hint_easy_returns_simpler_variant_for_curriculum_stages() {
    for topic in ["ownership", "borrow", "lifetime", "trait", "async"] {
        let canonical = exercise_for(topic).unwrap_or("");
        let easy = exercise_for_with_hint(topic, DifficultyHint::Easy)
            .unwrap_or_else(|| panic!("Easy variant missing for {topic}"));
        assert_ne!(
            easy, canonical,
            "Easy variant for {topic} must differ from canonical (was identical)"
        );
    }
}

#[test]
fn hint_hard_returns_harder_variant_for_curriculum_stages() {
    for topic in ["ownership", "borrow", "lifetime", "trait", "async"] {
        let canonical = exercise_for(topic).unwrap_or("");
        let hard = exercise_for_with_hint(topic, DifficultyHint::Hard)
            .unwrap_or_else(|| panic!("Hard variant missing for {topic}"));
        assert_ne!(
            hard, canonical,
            "Hard variant for {topic} must differ from canonical"
        );
    }
}

#[test]
fn hint_easy_and_hard_are_different() {
    for topic in ["ownership", "borrow", "lifetime", "trait", "async"] {
        let easy = exercise_for_with_hint(topic, DifficultyHint::Easy).unwrap_or("");
        let hard = exercise_for_with_hint(topic, DifficultyHint::Hard).unwrap_or("");
        assert_ne!(easy, hard, "Easy and Hard variants must differ for {topic}");
    }
}

#[test]
fn unknown_topic_returns_none_regardless_of_hint() {
    assert!(exercise_for_with_hint("nonexistent_xyz", DifficultyHint::Easy).is_none());
    assert!(exercise_for_with_hint("nonexistent_xyz", DifficultyHint::Hard).is_none());
    assert!(exercise_for_with_hint("nonexistent_xyz", DifficultyHint::Normal).is_none());
}

#[test]
fn non_curriculum_topic_with_hint_falls_back_to_canonical() {
    // `closure` exists in `exercise_for` but isn't a curriculum stage,
    // so any hint should return the canonical content.
    let canonical = exercise_for("closure").expect("closure exists");
    for hint in [
        DifficultyHint::Easy,
        DifficultyHint::Normal,
        DifficultyHint::Hard,
    ] {
        let got = exercise_for_with_hint("closure", hint).unwrap();
        assert_eq!(
            got, canonical,
            "non-curriculum topic must ignore hint; got different for {hint:?}"
        );
    }
}

// ─── Borrow-aliasing — ensures the topic-canonicalisation step works ─

#[test]
fn borrow_alias_қарызға_алу_resolves_to_curriculum_stage() {
    let easy = exercise_for_with_hint("қарызға алу", DifficultyHint::Easy);
    assert!(easy.is_some(), "borrow Kazakh alias should resolve");
    // And differs from the canonical `borrow` exercise.
    assert_ne!(easy.unwrap(), exercise_for("borrow").unwrap_or(""));
}

// ─── End-to-end: AskExercise after a pass surfaces Hard variant ──────

#[test]
#[ignore]
fn askexercise_after_clean_pass_on_ownership_serves_hard_variant() {
    let Some(lex) = lex() else { return };
    let repo = repo();
    let mut conv = Conversation::new();
    if conv.curriculum.is_none() {
        return;
    }

    // Turn 1: first exercise — student has 0 progress → Normal hint
    // → canonical (Medium) variant.
    let first = conv.turn(
        "Маған Rust-та ownership жаттығуын беріңізші.",
        &lex,
        &repo,
        0,
    );
    let canonical = exercise_for("ownership").unwrap();
    assert!(
        first.contains(canonical),
        "first exercise must be canonical; got: {first}"
    );

    // Turn 2: clean pass — increments ownership.passed = 1, failed = 0
    // → next hint is Hard.
    let _ = conv.turn(
        "```rust\nfn main() { println!(\"hi\"); }\n```",
        &lex,
        &repo,
        1,
    );

    // Turn 3: ask again — should now serve Hard variant.
    let second = conv.turn(
        "Маған Rust-та ownership жаттығуын беріңізші.",
        &lex,
        &repo,
        2,
    );
    let hard = exercise_for_with_hint("ownership", DifficultyHint::Hard).unwrap();
    assert!(
        second.contains(hard),
        "after one clean pass, second AskExercise must serve Hard variant; got: {second}"
    );
}

/// Synthetic preset: hand-craft progress with 2+ failures → next
/// AskExercise should surface the Easy variant.
#[test]
fn askexercise_with_two_failures_serves_easy_variant() {
    let Some(lex) = lex() else { return };
    let repo = repo();
    let mut conv = Conversation::new();
    if conv.curriculum.is_none() {
        return;
    }
    conv.curriculum_progress.insert(
        "ownership".into(),
        StageProgress {
            passed: 0,
            failed: 2,
        },
    );

    let out = conv.turn(
        "Маған Rust-та ownership жаттығуын беріңізші.",
        &lex,
        &repo,
        0,
    );
    let easy = exercise_for_with_hint("ownership", DifficultyHint::Easy).unwrap();
    assert!(
        out.contains(easy),
        "with 2 failures, AskExercise must serve Easy variant; got: {out}"
    );
}
