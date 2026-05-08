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
