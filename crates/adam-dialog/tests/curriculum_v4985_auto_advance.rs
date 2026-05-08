//! **v4.98.5** — auto-advance template family + adaptive-difficulty hooks.
//!
//! Tests the user-facing surface added on top of v4.98.0's curriculum
//! tracking foundation:
//!
//! 1. When a `passed` SubmitSolution closes a curriculum stage, the
//!    planner routes to `submit_solution.passed_stage_closed`
//!    surfacing the next stage label + summary (auto-advance).
//! 2. When the closing pass also closes the LAST stage, route to
//!    `submit_solution.passed_curriculum_complete` instead.
//! 3. A passed-but-not-stage-closing pass still routes to the
//!    legacy `submit_solution.passed`.
//! 4. Conversation populates the `__stage_closes__` /
//!    `next_stage_*` extra slots for the planner.
//!
//! Real `cargo check` invocations are slow, so end-to-end tests use
//! `#[ignore]`. Fast contract tests use direct API access.

use std::collections::HashMap;

use adam_dialog::{
    Conversation, TemplateRepository,
    curriculum::{Curriculum, DifficultyHint, Stage, StageProgress},
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

fn _stage(id: &str, threshold: usize, prereqs: Vec<&str>) -> Stage {
    Stage {
        id: id.into(),
        label_kk: id.into(),
        label_ru: String::new(),
        prereqs: prereqs.into_iter().map(String::from).collect(),
        exercises_to_pass: threshold,
        summary_kk: String::new(),
    }
}

#[test]
fn template_repo_has_passed_stage_closed_family() {
    let repo = repo();
    assert!(
        !repo.get("submit_solution.passed_stage_closed").is_empty(),
        "v4.98.5 must add `submit_solution.passed_stage_closed` template family"
    );
}

#[test]
fn template_repo_has_passed_curriculum_complete_family() {
    let repo = repo();
    assert!(
        !repo
            .get("submit_solution.passed_curriculum_complete")
            .is_empty(),
        "v4.98.5 must add `submit_solution.passed_curriculum_complete` family"
    );
}

#[test]
fn passed_stage_closed_templates_use_advance_slots() {
    // Every template in the family must reference the auto-advance
    // slots so the realiser can fill them.
    let repo = repo();
    let templates = repo.get("submit_solution.passed_stage_closed");
    assert!(!templates.is_empty(), "family must have templates");
    for tmpl in templates {
        assert!(
            tmpl.contains("{stage_label_kk}"),
            "every passed_stage_closed template must mention {{stage_label_kk}}; got {tmpl:?}"
        );
        assert!(
            tmpl.contains("{next_stage_label_kk}"),
            "every passed_stage_closed template must mention {{next_stage_label_kk}}; got {tmpl:?}"
        );
    }
}

/// Difficulty-hint thresholds smoke test (paired with the unit tests
/// in `curriculum::tests`).
#[test]
fn difficulty_hint_at_thresholds() {
    assert_eq!(
        StageProgress::default().difficulty_hint(),
        DifficultyHint::Normal
    );
    assert_eq!(
        StageProgress {
            passed: 0,
            failed: 2
        }
        .difficulty_hint(),
        DifficultyHint::Easy
    );
    assert_eq!(
        StageProgress {
            passed: 1,
            failed: 0
        }
        .difficulty_hint(),
        DifficultyHint::Hard
    );
}

/// End-to-end: simulate the full ownership journey — two clean
/// submissions on `ownership` should close the stage and the second
/// response should mention the next-stage label («Қарыз алу»).
#[test]
#[ignore]
fn end_to_end_two_passes_close_ownership_stage_and_announce_borrow() {
    let Some(lex) = lex() else { return };
    let repo = repo();
    let mut conv = Conversation::new();
    if conv.curriculum.is_none() {
        return;
    }

    // Lesson loop, iteration 1: ask for exercise + submit clean code.
    let _ = conv.turn(
        "Маған Rust-та ownership жаттығуын беріңізші.",
        &lex,
        &repo,
        0,
    );
    let first_verdict = conv.turn(
        "```rust\nfn main() { println!(\"hi\"); }\n```",
        &lex,
        &repo,
        1,
    );
    // First pass — stage NOT yet closed (1 / 2). Output should be a
    // standard `submit_solution.passed` response — no auto-advance
    // language.
    assert!(
        !first_verdict.contains("Қарыз алу"),
        "first pass (1/2) must NOT announce the next stage; got: {first_verdict}"
    );

    // Lesson loop, iteration 2: ask + submit again.
    let _ = conv.turn(
        "Маған Rust-та ownership жаттығуын беріңізші.",
        &lex,
        &repo,
        2,
    );
    let second_verdict = conv.turn(
        "```rust\nfn main() { let n = 42; println!(\"{}\", n); }\n```",
        &lex,
        &repo,
        3,
    );
    // Second pass — stage closed (2 / 2). Output should mention BOTH
    // the closed stage («Иелік») AND the next stage («Қарыз алу»).
    assert!(
        second_verdict.contains("Иелік"),
        "stage-closing response must mention closed stage label `Иелік`; got: {second_verdict}"
    );
    assert!(
        second_verdict.contains("Қарыз алу"),
        "stage-closing response must mention next stage label `Қарыз алу`; got: {second_verdict}"
    );
}

/// Synthetic multi-stage closure: pre-populate progress so all but the
/// final stage are closed, then submit one more clean snippet on the
/// final stage. Output should fire `passed_curriculum_complete`.
#[test]
#[ignore]
fn end_to_end_final_stage_pass_announces_curriculum_complete() {
    let Some(lex) = lex() else { return };
    let repo = repo();
    let mut conv = Conversation::new();
    let curriculum = match conv.curriculum.as_ref() {
        Some(c) => c.clone(),
        None => return,
    };

    // Pre-fill progress for every stage except the last with closure-
    // satisfying counts. For the last stage, set passed = (threshold - 1)
    // so the next pass closes it.
    let mut progress: HashMap<String, StageProgress> = HashMap::new();
    let last = curriculum.stages.last().unwrap();
    for s in &curriculum.stages {
        let target = if s.id == last.id {
            s.exercises_to_pass.saturating_sub(1)
        } else {
            s.exercises_to_pass
        };
        progress.insert(
            s.id.clone(),
            StageProgress {
                passed: target,
                failed: 0,
            },
        );
    }
    conv.curriculum_progress = progress;
    // Prime session so the planner's topic resolution falls back to
    // the last stage (typical user flow: AskExercise(<last>) just
    // happened).
    conv.session
        .insert("last_exercise_topic".into(), last.id.clone());

    // Submit a clean snippet — should close the LAST stage.
    let verdict = conv.turn(
        "```rust\nfn main() { println!(\"done\"); }\n```",
        &lex,
        &repo,
        0,
    );
    assert!(
        verdict.contains("Құттықтаймын") || verdict.contains("Тамаша!"),
        "final-stage closure must surface curriculum_complete language; got: {verdict}"
    );
    // Must NOT name a next stage (none exists).
    assert!(
        !verdict.contains("Келесі қадам"),
        "no next-stage mention when curriculum is complete; got: {verdict}"
    );
}

/// Synthetic next_unlocked check across the full real curriculum.
#[test]
fn next_unlocked_traverses_full_canonical_curriculum() {
    let curriculum = Curriculum::load_default()
        .expect("load")
        .expect("file present");
    let mut progress: HashMap<String, StageProgress> = HashMap::new();
    let mut visited = Vec::<String>::new();
    while let Some(next) = curriculum.next_unlocked(&progress) {
        visited.push(next.id.clone());
        progress.insert(
            next.id.clone(),
            StageProgress {
                passed: next.exercises_to_pass,
                failed: 0,
            },
        );
        // Safety bound: the canonical curriculum has 5 stages.
        if visited.len() > 16 {
            panic!("next_unlocked cycle detected: {visited:?}");
        }
    }
    // Should visit every stage exactly once, in dependency order.
    assert_eq!(visited.len(), curriculum.stages.len());
    assert_eq!(visited[0], "ownership");
    assert!(visited.contains(&"async".to_string()));
    assert!(curriculum.is_complete(&progress));
}
