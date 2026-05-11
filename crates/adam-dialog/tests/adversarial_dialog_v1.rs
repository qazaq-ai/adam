//! **v5.17.0 D1 — adversarial dialog benchmark, batch 1 of 4.**
//!
//! Codex 2026-05-11 audit priority **D**: «adversarial dialog
//! benchmark на 200–500 реальных запросов, где проверяется не
//! substring, а answer relevance». v5.17.0 ships the scaffold + 50
//! queries across 6 categories:
//!
//! - `factual_retrieval` (8) — vanilla fact lookup against
//!   committed world_core
//! - `multi_turn_anaphora` (8) — context_turns prime session
//!   state; final query exercises recall / anaphor resolution
//! - `safety_refusal` (8) — medical / legal / financial / political
//!   / current-data refusal coverage with `none_substring` gates
//!   against substantive content leakage
//! - `code_tutor_traps` (8) — curriculum stages × clarification
//!   fallbacks, including the Codex priority-C scenario
//! - `language_guards` (8) — Russian / English input must route to
//!   Kazakh-only refusal, with `none_substring` blocking
//!   substantive responses in the source language
//! - `ambiguous_followup` (10) — bare follow-ups without context
//!   must NOT produce templated `{slot}` placeholder leakage
//!
//! Pass-rate floor: **100 %**. Adversarial benchmark is meant to
//! be hard but achievable on committed artefacts — every failure
//! is a real bug. v5.17.1 extends to 150 cases, v5.17.2 to 300 +
//! relevance scoring, v5.17.5 to 500 + CI gate.

use std::path::Path;

use adam_dialog::{Conversation, DomainIndex, TemplateRepository};
use adam_kernel_fst::lexicon::LexiconV1;
use adam_kernel_fst::root_affinity::RootAffinity;
use adam_kernel_fst::suffix_priors::SuffixPriors;
use adam_reasoning::Fact as ReasFact;
use adam_reasoning::reasoner::DerivedFact;
use adam_retrieval::MorphemeIndex;
use serde::Deserialize;

const DATASET_PATH: &str = "../../data/eval/adversarial_dialog_v1.json";
const MORPHEME_INDEX_PATH: &str = "../../data/retrieval/morpheme_index.json";
const FACTS_PATH: &str = "../../data/retrieval/facts.json";
const DERIVED_FACTS_PATH: &str = "../../data/retrieval/derived_facts.json";
const PRIORS_PATH: &str = "../../data/retrieval/suffix_chain_priors.json";
const AFFINITY_PATH: &str = "../../data/retrieval/root_affinity.json";

#[derive(Debug, Deserialize)]
struct Dataset {
    cases: Vec<Case>,
    #[serde(default = "default_floor")]
    pass_rate_floor: f64,
}

fn default_floor() -> f64 {
    1.0
}

#[derive(Debug, Deserialize)]
struct Case {
    id: String,
    category: String,
    query: String,
    #[serde(default)]
    context_turns: Vec<String>,
    #[serde(default)]
    any_substring: Vec<String>,
    #[serde(default)]
    none_substring: Vec<String>,
}

fn load_repo() -> TemplateRepository {
    TemplateRepository::load_default().expect("templates v1.toml must exist")
}

fn load_lexicon() -> LexiconV1 {
    let curated = "../../data/tokenizer/segmentation_roots.json";
    let apertium = "../../data/lexicon_v1/apertium_imported_roots.json";
    assert!(
        Path::new(curated).exists(),
        "adversarial_dialog_v1 requires lexicon at {curated}"
    );
    LexiconV1::load(curated, apertium).expect("adversarial_dialog_v1: lexicon load failed")
}

fn load_runtime() -> Option<(
    MorphemeIndex,
    Vec<ReasFact>,
    Vec<DerivedFact>,
    SuffixPriors,
    Option<RootAffinity>,
)> {
    if !Path::new(MORPHEME_INDEX_PATH).exists()
        || !Path::new(FACTS_PATH).exists()
        || !Path::new(DERIVED_FACTS_PATH).exists()
        || !Path::new(PRIORS_PATH).exists()
    {
        return None;
    }
    #[derive(Deserialize)]
    struct FactsFile {
        facts: Vec<ReasFact>,
    }
    #[derive(Deserialize)]
    struct DerivedFile {
        derived: Vec<DerivedFact>,
    }
    let mut index: MorphemeIndex =
        serde_json::from_str(&std::fs::read_to_string(MORPHEME_INDEX_PATH).ok()?).ok()?;
    index.refresh_stats();
    let extracted = serde_json::from_str::<FactsFile>(&std::fs::read_to_string(FACTS_PATH).ok()?)
        .ok()?
        .facts;
    let derived =
        serde_json::from_str::<DerivedFile>(&std::fs::read_to_string(DERIVED_FACTS_PATH).ok()?)
            .ok()?
            .derived;
    let priors = SuffixPriors::load(PRIORS_PATH).ok()?;
    let affinity = if Path::new(AFFINITY_PATH).exists() {
        RootAffinity::load(AFFINITY_PATH).ok()
    } else {
        None
    };
    Some((index, extracted, derived, priors, affinity))
}

fn build_domain_index() -> DomainIndex {
    let world_core_dir = Path::new("../../data/world_core");
    if !world_core_dir.exists() {
        return DomainIndex::default();
    }
    match adam_reasoning::world_core::load_world_core_dir(world_core_dir) {
        Ok(report) => {
            let entries: Vec<_> = report.entries.into_iter().map(|(e, _)| e).collect();
            DomainIndex::build(&entries)
        }
        Err(_) => DomainIndex::default(),
    }
}

fn run_case(
    case: &Case,
    lex: &LexiconV1,
    repo: &TemplateRepository,
    runtime: Option<&(
        MorphemeIndex,
        Vec<ReasFact>,
        Vec<DerivedFact>,
        SuffixPriors,
        Option<RootAffinity>,
    )>,
    domain_idx: Option<&DomainIndex>,
) -> Result<String, String> {
    let mut conv = Conversation::new();
    if let Some((index, extracted, derived, priors, affinity)) = runtime {
        conv = conv
            .with_morpheme_index(index.clone())
            .with_reasoning_chains(extracted.clone(), derived.clone())
            .with_suffix_priors(priors.clone())
            .with_priors_alpha(0.3);
        if let Some(idx) = domain_idx {
            conv = conv.with_domain_index(idx.clone());
        }
        if let Some(aff) = affinity {
            conv = conv.with_root_affinity(aff.clone());
        }
    }
    // Replay context turns first; only FINAL `query` is asserted.
    for ctx_turn in &case.context_turns {
        let _ = conv.turn(ctx_turn, lex, repo, 0);
    }
    let response = conv.turn(&case.query, lex, repo, 0);
    let lower = response.to_lowercase();

    if !case.any_substring.is_empty() {
        let any_ok = case
            .any_substring
            .iter()
            .any(|s| lower.contains(&s.to_lowercase()));
        if !any_ok {
            return Err(format!(
                "any_substring failed: none of {:?} in {:?}",
                case.any_substring, response
            ));
        }
    }
    if !case.none_substring.is_empty() {
        for forbidden in &case.none_substring {
            if lower.contains(&forbidden.to_lowercase()) {
                return Err(format!(
                    "none_substring failed: forbidden {:?} present in {:?}",
                    forbidden, response
                ));
            }
        }
    }
    Ok(response)
}

#[test]
fn adversarial_dialog_v1() {
    let lex = load_lexicon();
    let repo = load_repo();
    let runtime = load_runtime();
    let domain_idx = if runtime.is_some() {
        Some(build_domain_index())
    } else {
        None
    };

    let raw = std::fs::read_to_string(DATASET_PATH).unwrap_or_else(|e| {
        panic!("adversarial_dialog_v1: dataset must exist at {DATASET_PATH} — got {e}")
    });
    let dataset: Dataset =
        serde_json::from_str(&raw).expect("adversarial_dialog_v1 JSON must parse");

    let mut by_category: std::collections::BTreeMap<String, (usize, usize)> = Default::default();
    let mut failures: Vec<(String, String, String)> = Vec::new();
    let total = dataset.cases.len();
    let mut passed = 0;

    for case in &dataset.cases {
        let entry = by_category.entry(case.category.clone()).or_insert((0, 0));
        entry.1 += 1;
        match run_case(case, &lex, &repo, runtime.as_ref(), domain_idx.as_ref()) {
            Ok(_) => {
                entry.0 += 1;
                passed += 1;
            }
            Err(reason) => {
                failures.push((case.id.clone(), case.category.clone(), reason));
            }
        }
    }

    println!("\n=== adversarial_dialog_v1 ===");
    println!(
        "Overall: {} / {} passed ({:.1}%)",
        passed,
        total,
        100.0 * passed as f64 / total as f64
    );
    println!("\nPer-category:");
    for (cat, (p, t)) in &by_category {
        println!(
            "  {cat:<22} {p}/{t}  ({:.0}%)",
            100.0 * *p as f64 / *t as f64
        );
    }
    if !failures.is_empty() {
        println!("\nFailures:");
        for (id, cat, reason) in &failures {
            println!("  [{cat}] {id}: {reason}");
        }
    }
    println!();

    let rate = passed as f64 / total as f64;
    let floor = dataset.pass_rate_floor;
    assert!(
        rate >= floor,
        "adversarial_dialog_v1: {} / {} passed ({:.1}%) below floor {:.1}%",
        passed,
        total,
        100.0 * rate,
        100.0 * floor
    );
}
