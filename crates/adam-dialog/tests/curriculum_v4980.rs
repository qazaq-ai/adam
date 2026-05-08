//! **v4.98.0** — lesson-state curriculum tree (long-term roadmap step 1).
//!
//! Verifies that:
//! 1. `Conversation::new()` autoloads the committed curriculum.
//! 2. After a SubmitSolution turn whose verdict is `passed`, the
//!    student's progress on the topic's stage increments by one.
//! 3. After repeated passes, the stage closes (per
//!    `exercises_to_pass`).
//! 4. A `failed` verdict increments `failed`, not `passed`.
//! 5. Topicless / non-curriculum SubmitSolution turns don't pollute
//!    progress.
//!
//! Real `cargo check` invocations are slow (~3-5 s each), so the
//! end-to-end multi-turn test uses `#[ignore]` to keep CI fast.

use adam_dialog::{Conversation, TemplateRepository, curriculum::StageProgress};
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
fn fresh_conversation_autoloads_curriculum() {
    let conv = Conversation::new();
    let curriculum = conv
        .curriculum
        .as_ref()
        .expect("curriculum file should autoload from data/dialog/curriculum/");
    assert!(curriculum.stages.len() >= 5);
    assert!(curriculum.stage("ownership").is_some());
    assert!(curriculum.stage("async").is_some());
}

#[test]
fn fresh_conversation_has_empty_progress() {
    let conv = Conversation::new();
    assert!(
        conv.curriculum_progress.is_empty(),
        "no turns yet → no progress; got {:?}",
        conv.curriculum_progress
    );
}

/// Direct contract test: simulate a `SubmitSolution(passed, ownership)`
/// outcome by manipulating progress directly. (The planner-driven
/// integration is exercised via the `#[ignore]` end-to-end test
/// below — this guards the data-shape contract independent of the
/// (slow) `cargo check` invocation.)
#[test]
fn stage_progress_record_pass_then_close() {
    let curriculum = adam_dialog::curriculum::Curriculum::load_default()
        .expect("load")
        .expect("file present");
    let mut progress = std::collections::HashMap::new();
    let stage = curriculum
        .stage("ownership")
        .expect("ownership stage exists");
    let entry = progress
        .entry(stage.id.clone())
        .or_insert(StageProgress::default());
    assert!(!entry.is_closed(stage));
    entry.record_pass();
    assert_eq!(entry.passed, 1);
    assert!(!entry.is_closed(stage)); // 1 / 2 not closed yet
    entry.record_pass();
    assert!(entry.is_closed(stage)); // 2 / 2 closed
    // Next-unlocked should now point at borrow.
    let next = curriculum.next_unlocked(&progress).expect("borrow unlocks");
    assert_eq!(next.id, "borrow");
}

/// End-to-end #[ignore] test: real cargo verify on a clean ownership
/// snippet → progress should increment. Runs cargo so it's slow.
#[test]
#[ignore]
fn submit_solution_pass_increments_curriculum_progress() {
    let Some(lex) = lex() else { return };
    let repo = repo();
    let mut conv = Conversation::new();
    if conv.curriculum.is_none() {
        // Trimmed checkout — curriculum file absent, skip.
        return;
    }

    // Turn 1: prime the lesson topic via AskExercise.
    let _ = conv.turn(
        "Маған Rust-та ownership жаттығуын беріңізші.",
        &lex,
        &repo,
        0,
    );

    // Turn 2: submit a clean snippet (passes cargo check).
    let clean_snippet = "```rust\nfn main() { println!(\"сәлем\"); }\n```";
    let _ = conv.turn(clean_snippet, &lex, &repo, 1);

    let progress = conv.curriculum_progress.get("ownership").copied();
    assert!(
        progress.is_some(),
        "expected progress entry for `ownership`; got: {:?}",
        conv.curriculum_progress
    );
    let p = progress.unwrap();
    assert!(
        p.passed >= 1,
        "expected ≥1 passed exercise on ownership; got {p:?}"
    );
}

/// End-to-end #[ignore] test: a `failed` verdict (broken snippet) must
/// increment `failed`, not `passed`.
#[test]
#[ignore]
fn submit_solution_fail_increments_failed_not_passed() {
    let Some(lex) = lex() else { return };
    let repo = repo();
    let mut conv = Conversation::new();
    if conv.curriculum.is_none() {
        return;
    }

    let _ = conv.turn(
        "Маған Rust-та ownership жаттығуын беріңізші.",
        &lex,
        &repo,
        0,
    );

    // Submit a snippet that triggers E0382 (use-of-moved-value).
    let broken =
        "```rust\nfn main() { let s = String::from(\"hi\"); let _t = s; println!(\"{s}\"); }\n```";
    let _ = conv.turn(broken, &lex, &repo, 1);

    let progress = conv.curriculum_progress.get("ownership").copied();
    assert!(progress.is_some());
    let p = progress.unwrap();
    assert_eq!(p.passed, 0, "no pass on failing snippet; got {p:?}");
    assert!(p.failed >= 1, "expected ≥1 failed; got {p:?}");
}

/// SubmitSolution on a topic NOT in the curriculum (e.g. a curriculum-
/// less submission) must not create a phantom progress entry.
#[test]
#[ignore]
fn submit_solution_with_unknown_topic_does_not_pollute_progress() {
    let Some(lex) = lex() else { return };
    let repo = repo();
    let mut conv = Conversation::new();
    if conv.curriculum.is_none() {
        return;
    }

    // Submit a clean snippet WITHOUT priming a topic via AskExercise.
    // The planner's v4.95.5 fallback finds no `last_exercise_topic`,
    // so no curriculum stage is identified.
    let clean = "```rust\nfn main() {}\n```";
    let _ = conv.turn(clean, &lex, &repo, 0);

    // Either no progress entry, or only entries for valid stages.
    for k in conv.curriculum_progress.keys() {
        assert!(
            conv.curriculum.as_ref().unwrap().stage(k).is_some(),
            "phantom progress entry for non-curriculum topic: {k:?}"
        );
    }
}
