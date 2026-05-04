//! **v4.48.5** — Stage B bundle 8: test-harness integration of the
//! `selection_audit` harvest pipeline.
//!
//! Runs every query in `data/eval/live_holdout_2026_05_01.json`
//! through a production-shaped `Conversation::turn_with_trace`,
//! collects every `ToolResult.trace` line emitted across all turns,
//! and feeds the flattened `Vec<Vec<String>>` into
//! `harvest_audit_traces`. Prints the resulting `HarvestReport` so
//! the offline operator can read it from CI logs and (when
//! disagreements are surfaced) inspect raw lines for hand-curation
//! into new `TrainingPair` entries.
//!
//! **No assertions on disagreement count** — the v4.46.5 wiring with
//! `default_v0()` weights is calibrated to match the v4.38.0
//! heuristic ranker, so disagreements should be rare. This test is
//! diagnostic, not gating: it surfaces what the audit sees, prints
//! the report, and asserts only that the harness ran without panic.
//! When v4.48.5+ patches re-train the weights, the test will start
//! reporting actual disagreement counts that operators can act on.
//!
//! Skipped silently when runtime artifacts (`facts.json`,
//! `morpheme_index.json`, etc.) are absent — same convention as
//! `tests/live_holdout.rs`.

use std::path::Path;

use adam_dialog::{
    Conversation, DomainIndex, HarvestReport, TemplateRepository, harvest_audit_traces,
};
use adam_kernel_fst::lexicon::LexiconV1;
use adam_kernel_fst::root_affinity::RootAffinity;
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
const AFFINITY_PATH: &str = "../../data/retrieval/root_affinity.json";

#[derive(Debug, Deserialize)]
struct Dataset {
    cases: Vec<Case>,
}

#[derive(Debug, Deserialize)]
struct Case {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    category: String,
    query: String,
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

fn collect_trace_lines(
    case_query: &str,
    conv: &mut Conversation,
    lex: &LexiconV1,
    repo: &TemplateRepository,
) -> Vec<String> {
    // turn_with_trace returns a (response, TurnTrace) tuple. The
    // selection_audit lines emitted by `tool::search_graph` live in
    // `tool_calls[].trace`, so flatten across all tool calls.
    let (_response, trace) = conv.turn_with_trace(case_query, lex, repo, 0);
    let mut lines: Vec<String> = Vec::new();
    for tool_result in &trace.tool_calls {
        for line in &tool_result.trace {
            lines.push(line.clone());
        }
    }
    // Also include plan_trace for completeness — the planner might
    // surface audit-related diagnostics in the future even though
    // v4.48.0's wiring lives in tool::search_graph only.
    for line in &trace.plan_trace {
        lines.push(line.clone());
    }
    lines
}

fn print_report(label: &str, report: &HarvestReport) {
    eprintln!("\n=== selection_audit harvest report — {label} ===");
    eprintln!("  total_turns:           {}", report.total_turns);
    eprintln!(
        "  multi_candidate_turns: {} ({} of total)",
        report.multi_candidate_turns,
        if report.total_turns > 0 {
            format!(
                "{:.1}%",
                100.0 * report.multi_candidate_turns as f32 / report.total_turns as f32
            )
        } else {
            "n/a".to_string()
        }
    );
    eprintln!("  audit_lines_found:     {}", report.audit_lines_found);
    eprintln!("  disagreement_count:    {}", report.disagreement_count);
    eprintln!("  disagreement_rate:     {:.4}", report.disagreement_rate());
    if !report.disagreement_traces.is_empty() {
        eprintln!("  disagreement_traces:");
        for (i, line) in report.disagreement_traces.iter().enumerate() {
            eprintln!("    [{}] {}", i, line);
        }
    } else {
        eprintln!("  disagreement_traces:   (none)");
    }
}

#[test]
fn selection_audit_harvest_live_holdout_2026_05_01() {
    if !Path::new(DATASET_PATH).exists() {
        eprintln!("skipping selection_audit_harvest: {DATASET_PATH} not present");
        return;
    }
    let runtime = match load_runtime() {
        Some(r) => r,
        None => {
            eprintln!("skipping selection_audit_harvest: runtime artifacts absent");
            return;
        }
    };
    let lex = {
        let curated = "../../data/tokenizer/segmentation_roots.json";
        let apertium = "../../data/lexicon_v1/apertium_imported_roots.json";
        assert!(Path::new(curated).exists(), "lexicon at {curated} required");
        LexiconV1::load(curated, apertium).expect("lexicon load failed")
    };
    let repo = TemplateRepository::load_default().expect("templates v1.toml must exist");
    let domain_idx = build_domain_index();
    let dataset: Dataset =
        serde_json::from_str(&std::fs::read_to_string(DATASET_PATH).expect("dataset readable"))
            .expect("dataset parses as JSON");

    let mut all_traces: Vec<Vec<String>> = Vec::with_capacity(dataset.cases.len());
    for case in &dataset.cases {
        let (index, extracted, derived, priors, affinity) = &runtime;
        let mut conv = Conversation::new()
            .with_morpheme_index(index.clone())
            .with_reasoning_chains(extracted.clone(), derived.clone())
            .with_suffix_priors(priors.clone())
            .with_priors_alpha(0.3)
            .with_domain_index(domain_idx.clone());
        if let Some(aff) = affinity {
            conv = conv.with_root_affinity(aff.clone());
        }
        let lines = collect_trace_lines(&case.query, &mut conv, &lex, &repo);
        all_traces.push(lines);
    }
    let report = harvest_audit_traces(&all_traces);
    print_report("live_holdout_2026_05_01", &report);

    // Diagnostic, not gating: assert only that the harness ran
    // without panic and the report shape is internally consistent.
    assert_eq!(report.total_turns, dataset.cases.len());
    assert!(report.disagreement_count <= report.audit_lines_found);
    assert!(report.multi_candidate_turns <= report.total_turns);
}
