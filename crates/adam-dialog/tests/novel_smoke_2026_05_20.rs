//! **v6.0.0-rc5 pre-merge novel-query smoke test.**
//!
//! Per the `Real human-like testing per step + time + memory` memory,
//! every release should be exercised on Kazakh phrasings that were
//! *not* part of any tuned eval set. This file collects 15 such
//! prompts, runs them through a cold `Conversation`, and prints
//! the latency + response for each so a human reviewer can spot-
//! check before merging into `main`.
//!
//! NOT a hard CI gate — this is *exploratory*. Responses are
//! human-judged; the test asserts only that nothing panics and that
//! every response is non-empty. Use the printed transcript to find
//! cases that need follow-up patches (matchers, intents, data).
//!
//! Run with `--nocapture` to see the transcript:
//! ```
//! cargo test -p adam-dialog --test novel_smoke_2026_05_20 --release -- --nocapture
//! ```
//
// SPDX-License-Identifier: BUSL-1.1

use std::path::Path;
use std::time::Instant;

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

/// 15 Kazakh prompts collected for the rc5 finalisation smoke test.
/// Sampled across personas, intents, and topical breadth. None of
/// these appear verbatim in `factual_eval_100`, `live_holdout_*`,
/// or `repl_replay`. Annotated with the human reviewer's
/// expectation for the response shape.
const PROMPTS: &[(&str, &str)] = &[
    // Currency / money
    (
        "Қазақстанның ұлттық валютасы қалай аталады?",
        "expect: теңге",
    ),
    // Capital city
    ("Алматы туралы не білесіз?", "expect: city description"),
    // Duration math
    (
        "Бір аптада неше жұмыс күні бар?",
        "expect: 5 OR refusal (no fact)",
    ),
    // Statement of name (intent classification)
    (
        "Менің атым Дәулет, рахмет.",
        "expect: greeting + name-acknowledged",
    ),
    // Apology intent
    ("Кешіріңіз, ұйықтап қалдым.", "expect: apology-acknowledge"),
    // System capability question
    (
        "Сіз қандай тақырыптарда сөйлесе аласыз?",
        "expect: capabilities listing",
    ),
    // Nonsense query — expect refusal
    ("Аспан күшіне қарай ма?", "expect: refusal / clarify"),
    // Recommendation request
    (
        "Маған қандай қазақ кітабын ұсынасыз?",
        "expect: literature topic OR refusal",
    ),
    // Temporal opinion (open-ended)
    (
        "Ертең күн жылы болады ма?",
        "expect: weather-no-data refusal",
    ),
    // Self-reflection
    ("Сіз қашан жасалдыңыз?", "expect: birthdate aspect"),
    // Temperature (no data)
    (
        "Бүгін сыртта неше градус?",
        "expect: weather-temperature refusal",
    ),
    // Counting (factual)
    ("Жыл мезгілдері неше?", "expect: 4 OR refusal"),
    // Advice request
    (
        "Жұмыс іздегенде қандай кеңес бересіз?",
        "expect: refusal / clarify (no advice data)",
    ),
    // Color verification (yes/no)
    (
        "Жасыл түс — бұл негізгі түс пе?",
        "expect: yes-like answer OR refusal",
    ),
    // Definition request
    ("Ауыл деген не?", "expect: settlement definition"),
];

fn load_runtime() -> Option<(
    MorphemeIndex,
    Vec<ReasFact>,
    Vec<DerivedFact>,
    SuffixPriors,
    Option<RootAffinity>,
)> {
    if !Path::new(MORPHEME_INDEX_PATH).exists() {
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

#[test]
fn novel_smoke_rc5_finalisation() {
    let runtime = match load_runtime() {
        Some(r) => r,
        None => {
            eprintln!("novel_smoke: runtime missing — SKIP");
            return;
        }
    };
    let curated = "../../data/tokenizer/segmentation_roots.json";
    let apertium = "../../data/lexicon_v1/apertium_imported_roots.json";
    let lex = LexiconV1::load(curated, apertium).expect("lexicon load");
    let repo = TemplateRepository::load_default().expect("templates");
    let world_core_dir = Path::new("../../data/world_core");
    let domain_idx = match adam_reasoning::world_core::load_world_core_dir(world_core_dir) {
        Ok(report) => {
            let entries: Vec<_> = report.entries.into_iter().map(|(e, _)| e).collect();
            DomainIndex::build(&entries)
        }
        Err(_) => DomainIndex::default(),
    };

    println!("\n=== Novel-query smoke test for v6.0.0-rc5 finalisation ===\n");
    let mut total_latency_ms = 0u128;
    let mut max_latency_ms = 0u128;
    let mut min_latency_ms = u128::MAX;
    let mut empty_count = 0usize;

    for (q, expectation) in PROMPTS {
        let mut conv = Conversation::new()
            .with_morpheme_index(runtime.0.clone())
            .with_reasoning_chains(runtime.1.clone(), runtime.2.clone())
            .with_suffix_priors(runtime.3.clone())
            .with_priors_alpha(0.3)
            .with_domain_index(domain_idx.clone());
        if let Some(aff) = &runtime.4 {
            conv = conv.with_root_affinity(aff.clone());
        }
        let start = Instant::now();
        let response = conv.turn(q, &lex, &repo, 0);
        let elapsed = start.elapsed().as_millis();
        total_latency_ms += elapsed;
        if elapsed > max_latency_ms {
            max_latency_ms = elapsed;
        }
        if elapsed < min_latency_ms {
            min_latency_ms = elapsed;
        }
        if response.trim().is_empty() {
            empty_count += 1;
        }
        println!("Q: {q}");
        println!("  ({expectation})");
        println!("  A [{elapsed} ms]: {response}");
        println!();
    }

    let n = PROMPTS.len() as u128;
    println!("--- Latency summary ---");
    println!(
        "p_min={min_latency_ms} ms  p_avg={} ms  p_max={max_latency_ms} ms  n={n}",
        total_latency_ms / n
    );
    println!("Empty responses: {empty_count} / {n}");
    println!();

    // Soft assertion: every response non-empty. No "correctness"
    // assertion — that's the human reviewer's job from the printed
    // transcript above.
    assert_eq!(
        empty_count, 0,
        "{empty_count} novel prompt(s) produced empty responses"
    );
}
