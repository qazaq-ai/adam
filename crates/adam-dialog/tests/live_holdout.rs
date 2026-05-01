//! **v4.24.5 — live holdout regression baseline.**
//!
//! Independent of the curated v4.x test fixtures (`cognitive_eval`,
//! `repl_replay`, `parse_disambiguation`), this harness runs every
//! query in `data/eval/live_holdout_YYYYMMDD.json` through a
//! production-shaped `Conversation::turn_with_trace` and asserts
//! response shape via substring presence/absence rules.
//!
//! **Why a separate eval.** The curated regression suites have been
//! hand-tuned over ~20 releases to lock specific expected behaviour;
//! they're definitionally not "blind". Codex review (v4.22.5)
//! flagged this as the highest-leverage missing signal: an
//! independent dataset captured from a real session, never edited
//! after capture, run with no template tuning to make any specific
//! case pass. Pass rate on a fresh holdout is the closest thing
//! the project has to "is dialog quality actually getting better
//! across releases?".
//!
//! **Schema (per case):**
//! - `id` — unique within the file.
//! - `category` — coarse class (identity / world_core_science /
//!   temporal_no_data / honest_unknown / …) for per-category
//!   pass-rate breakdowns.
//! - `query` — verbatim user input.
//! - `any_substring` — pass iff at least one listed substring is in
//!   the (lowercased) response.
//! - `none_substring` — pass iff none of the listed substrings is
//!   in the response (negative test, e.g. "should NOT surface this
//!   proverb"). Both checks combine with logical AND.
//! - `note` — human-readable rationale for what the case verifies.
//!
//! The test prints a pass/fail summary, per-category breakdown, and
//! per-failure detail. `assert!` on overall pass rate is gated to
//! the current baseline so future regressions go red but
//! incremental improvements don't immediately flip the threshold.

use std::path::Path;

use adam_dialog::{Conversation, DomainIndex, TemplateRepository};
use adam_kernel_fst::lexicon::LexiconV1;
use adam_kernel_fst::suffix_priors::SuffixPriors;
use adam_reasoning::Fact as ReasFact;
use adam_reasoning::reasoner::DerivedFact;
use adam_retrieval::MorphemeIndex;
use serde::Deserialize;

const DATASET_PATH: &str = "../../data/eval/live_holdout_2026_05_01.json";
const MORPHEME_INDEX_PATH: &str = "../../data/retrieval/morpheme_index.json";
const FACTS_PATH: &str = "../../data/retrieval/facts.json";
const DERIVED_FACTS_PATH: &str = "../../data/retrieval/derived_facts.json";
const PRIORS_PATH: &str = "../../data/retrieval/suffix_chain_priors.json";

#[derive(Debug, Deserialize)]
struct Dataset {
    cases: Vec<Case>,
}

#[derive(Debug, Deserialize)]
struct Case {
    id: String,
    category: String,
    query: String,
    #[serde(default)]
    any_substring: Vec<String>,
    #[serde(default)]
    none_substring: Vec<String>,
    #[allow(dead_code)]
    #[serde(default)]
    note: String,
}

fn load_repo() -> TemplateRepository {
    TemplateRepository::load_default().expect("templates v1.toml must exist")
}

fn load_lexicon() -> LexiconV1 {
    let curated = "../../data/tokenizer/segmentation_roots.json";
    let apertium = "../../data/lexicon_v1/apertium_imported_roots.json";
    assert!(
        Path::new(curated).exists(),
        "live_holdout requires lexicon at {curated}"
    );
    LexiconV1::load(curated, apertium).expect("live_holdout: lexicon load failed")
}

fn load_runtime() -> Option<(MorphemeIndex, Vec<ReasFact>, Vec<DerivedFact>, SuffixPriors)> {
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

    Some((index, extracted, derived, priors))
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
    runtime: Option<&(MorphemeIndex, Vec<ReasFact>, Vec<DerivedFact>, SuffixPriors)>,
    domain_idx: Option<&DomainIndex>,
) -> Result<String, String> {
    let mut conv = Conversation::new();
    if let Some((index, extracted, derived, priors)) = runtime {
        conv = conv
            .with_morpheme_index(index.clone())
            .with_reasoning_chains(extracted.clone(), derived.clone())
            .with_suffix_priors(priors.clone())
            .with_priors_alpha(0.3);
        if let Some(idx) = domain_idx {
            conv = conv.with_domain_index(idx.clone());
        }
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
fn live_holdout_2026_05_01() {
    let lex = load_lexicon();
    let repo = load_repo();
    let runtime = load_runtime();
    let domain_idx = if runtime.is_some() {
        Some(build_domain_index())
    } else {
        None
    };

    let raw = std::fs::read_to_string(DATASET_PATH)
        .unwrap_or_else(|e| panic!("live_holdout: dataset must exist at {DATASET_PATH} — got {e}"));
    let dataset: Dataset = serde_json::from_str(&raw).expect("live_holdout JSON must parse");

    let mut by_category: std::collections::BTreeMap<String, (usize, usize)> = Default::default();
    let mut failures: Vec<(String, String, String)> = Vec::new(); // (id, category, reason)
    let total = dataset.cases.len();
    let mut passed = 0;

    for case in &dataset.cases {
        let entry = by_category.entry(case.category.clone()).or_insert((0, 0));
        entry.1 += 1; // total++
        match run_case(&case, &lex, &repo, runtime.as_ref(), domain_idx.as_ref()) {
            Ok(_) => {
                entry.0 += 1; // passed++
                passed += 1;
            }
            Err(reason) => {
                failures.push((case.id.clone(), case.category.clone(), reason));
            }
        }
    }

    println!("\n=== live_holdout_2026_05_01 ===");
    println!(
        "Overall: {} / {} passed ({:.1}%)",
        passed,
        total,
        100.0 * passed as f64 / total as f64
    );
    println!("\nPer-category:");
    for (cat, (p, t)) in &by_category {
        println!(
            "  {cat:<25} {p}/{t}  ({:.0}%)",
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

    // Baseline gate. v4.24.5 establishes the first holdout baseline;
    // future captures may have lower pass rates and that's expected
    // (a fresh holdout is supposed to expose new failures). The
    // current baseline is documented in the v4.24.5 CHANGELOG.
    //
    // The threshold below is the floor — failures below it indicate
    // a regression and should fail CI. The v4.24.5 baseline is set
    // conservatively at 70% to leave room for the known carry-
    // forwards (Latin technical names, multi-turn anaphora,
    // composition imperfections) without going red.
    let pass_rate = passed as f64 / total as f64;
    assert!(
        pass_rate >= 0.70,
        "live_holdout pass rate {:.1}% below v4.24.5 baseline floor of 70.0%",
        pass_rate * 100.0
    );
}
