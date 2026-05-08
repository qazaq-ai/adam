//! **v4.95.0** — SubmitSolution end-to-end integration tests.
//!
//! These tests run the full `Conversation::turn` loop against
//! Rust code submissions. The pipeline:
//!   1. Detect triple-backtick code block → `Intent::SubmitSolution`.
//!   2. Planner runs `crate::cargo_verify::verify_snippet` (real
//!      `cargo check` invocation, ~1-2 s wall time).
//!   3. Result populates `cargo_status` slot + (on failure)
//!      `error_code` / `error_explanation` / `raw_excerpt`.
//!   4. Planner sub-routes to `submit_solution.{passed,
//!      failed_known, failed_unknown, env_error}` family.
//!
//! All marked `#[ignore]` — default `cargo test` stays fast. Run
//! with `cargo test --release submit_solution -- --ignored`.

use std::path::Path;

use adam_dialog::{Conversation, DomainIndex, TemplateRepository};
use adam_kernel_fst::lexicon::LexiconV1;
use adam_kernel_fst::root_affinity::RootAffinity;
use adam_kernel_fst::suffix_priors::SuffixPriors;
use adam_reasoning::Fact as ReasFact;
use adam_reasoning::reasoner::DerivedFact;
use adam_retrieval::MorphemeIndex;
use serde::Deserialize;

const MORPHEME_INDEX_PATH: &str = "../../data/retrieval/morpheme_index.json";
const FACTS_PATH: &str = "../../data/retrieval/facts.json";
const DERIVED_FACTS_PATH: &str = "../../data/retrieval/derived_facts.json";
const PRIORS_PATH: &str = "../../data/retrieval/suffix_chain_priors.json";
const AFFINITY_PATH: &str = "../../data/retrieval/root_affinity.json";

fn load_lexicon() -> LexiconV1 {
    let curated = "../../data/tokenizer/segmentation_roots.json";
    let apertium = "../../data/lexicon_v1/apertium_imported_roots.json";
    LexiconV1::load(curated, apertium).expect("submit_solution_integration: lexicon load failed")
}

fn build_conversation() -> (Conversation, LexiconV1, TemplateRepository) {
    let lex = load_lexicon();
    let repo = TemplateRepository::load_default().expect("templates v1.toml must exist");
    #[derive(Deserialize)]
    struct FactsFile {
        facts: Vec<ReasFact>,
    }
    #[derive(Deserialize)]
    struct DerivedFile {
        derived: Vec<DerivedFact>,
    }
    let mut index: MorphemeIndex =
        serde_json::from_str(&std::fs::read_to_string(MORPHEME_INDEX_PATH).unwrap()).unwrap();
    index.refresh_stats();
    let extracted =
        serde_json::from_str::<FactsFile>(&std::fs::read_to_string(FACTS_PATH).unwrap())
            .unwrap()
            .facts;
    let derived =
        serde_json::from_str::<DerivedFile>(&std::fs::read_to_string(DERIVED_FACTS_PATH).unwrap())
            .unwrap()
            .derived;
    let priors = SuffixPriors::load(PRIORS_PATH).expect("priors load");
    let affinity = if Path::new(AFFINITY_PATH).exists() {
        RootAffinity::load(AFFINITY_PATH).ok()
    } else {
        None
    };

    let world_core_dir = Path::new("../../data/world_core");
    let domain_idx = match adam_reasoning::world_core::load_world_core_dir(world_core_dir) {
        Ok(report) => {
            let entries: Vec<_> = report.entries.into_iter().map(|(e, _)| e).collect();
            DomainIndex::build(&entries)
        }
        Err(_) => DomainIndex::default(),
    };

    let mut conv = Conversation::new()
        .with_morpheme_index(index)
        .with_reasoning_chains(extracted, derived)
        .with_suffix_priors(priors)
        .with_priors_alpha(0.3)
        .with_domain_index(domain_idx);
    if let Some(aff) = affinity {
        conv = conv.with_root_affinity(aff);
    }
    (conv, lex, repo)
}

#[test]
#[ignore]
fn submit_solution_clean_code_passes() {
    let (mut conv, lex, repo) = build_conversation();
    let input = "```rust\nfn main() { let x = 5; println!(\"{x}\"); }\n```";
    let response = conv.turn(input, &lex, &repo, 0);
    let lower = response.to_lowercase();
    assert!(
        lower.contains("жарайсыз")
            || lower.contains("компиляция")
            || lower.contains("type-checker"),
        "expected pass response, got: {response}"
    );
}

#[test]
#[ignore]
fn submit_solution_e0382_use_of_moved_value_explains() {
    let (mut conv, lex, repo) = build_conversation();
    let input = "```rust\nfn main() {\n    let s = String::from(\"hello\");\n    let s2 = s;\n    println!(\"{}\", s);\n}\n```";
    let response = conv.turn(input, &lex, &repo, 0);
    let lower = response.to_lowercase();
    assert!(
        lower.contains("e0382"),
        "expected E0382 in response, got: {response}"
    );
    assert!(
        lower.contains("ownership") || lower.contains("иелен"),
        "expected ownership-explanation in response, got: {response}"
    );
}

#[test]
#[ignore]
fn submit_solution_undeclared_variable_surfaces_compiler_msg() {
    let (mut conv, lex, repo) = build_conversation();
    // Reference to undefined variable — guaranteed compile error
    // but error class varies by rustc version, so just assert
    // that adam returns a non-passing response with raw_excerpt.
    let input = "```rust\nfn main() {\n    println!(\"{}\", undefined_variable);\n}\n```";
    let response = conv.turn(input, &lex, &repo, 0);
    let lower = response.to_lowercase();
    assert!(
        !lower.contains("жарайсыз"),
        "expected NOT-pass response on undefined-var, got: {response}"
    );
    assert!(
        lower.contains("қате") || lower.contains("error"),
        "expected error mention in response, got: {response}"
    );
}

// **v4.95.5** — multi-turn lesson state. Verify that an AskExercise
// turn primes the session with `last_exercise_topic`, and that a
// subsequent SubmitSolution (with no explicit topic) inherits it
// and produces a topic-aware response.
#[test]
#[ignore]
fn multi_turn_lesson_links_exercise_to_solution() {
    let (mut conv, lex, repo) = build_conversation();

    // Turn 1: student asks for an exercise on `ownership`.
    let prompt = "Маған Rust-та ownership жаттығуын беріңізші.";
    let r1 = conv.turn(prompt, &lex, &repo, 0);
    let r1_lower = r1.to_lowercase();
    assert!(
        r1_lower.contains("ownership"),
        "exercise response should mention ownership; got: {r1}"
    );

    // Session should now carry last_exercise_topic="ownership".
    assert_eq!(
        conv.session_value("last_exercise_topic"),
        Some("ownership".to_string()),
        "session should remember the exercise topic"
    );

    // Turn 2: student submits a solution with NO explicit topic in
    // prose — just the code block. adam should fall back to the
    // session-stored exercise topic.
    let solution = "```rust\nfn main() {\n    let s = String::from(\"hello\");\n    let s2 = s;\n    println!(\"{}\", s);\n}\n```";
    let r2 = conv.turn(solution, &lex, &repo, 0);
    let r2_lower = r2.to_lowercase();

    assert!(
        r2_lower.contains("e0382"),
        "submission verdict should surface E0382; got: {r2}"
    );
    // Lesson-aware variant should mention the topic. We don't lock
    // to a specific phrasing — just require the topic word appears
    // somewhere in the verdict.
    assert!(
        r2_lower.contains("ownership"),
        "submission verdict should mention exercise topic 'ownership' since the lesson context was set; got: {r2}"
    );
}

// Multi-turn coverage with a clean (passing) solution submitted
// after an exercise prompt. Confirms the lesson-aware passed
// templates fire.
#[test]
#[ignore]
fn multi_turn_lesson_passes_clean_solution_with_topic() {
    let (mut conv, lex, repo) = build_conversation();

    let _r1 = conv.turn("Hello world коды керек.", &lex, &repo, 0);
    assert_eq!(
        conv.session_value("last_exercise_topic"),
        Some("hello world".to_string()),
        "CodeRequest should also prime the lesson context"
    );

    let r2 = conv.turn(
        "```rust\nfn main() { println!(\"hello!\"); }\n```",
        &lex,
        &repo,
        0,
    );
    let lower = r2.to_lowercase();
    assert!(
        lower.contains("жарайсыз")
            || lower.contains("компиляция")
            || lower.contains("type-checker")
            || lower.contains("дайын"),
        "passing submission should produce a positive verdict; got: {r2}"
    );
}
