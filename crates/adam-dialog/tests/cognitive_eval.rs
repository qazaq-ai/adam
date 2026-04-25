//! v4.0.35 — narrow Cognitive Eval Harness (Codex roadmap Phase 7a).
//!
//! Loads `data/eval/cognitive_dialog_dataset.json`, runs each scenario
//! through `Conversation::turn_with_trace`, and asserts against trace
//! signals (action, task status, epistemic status, output substrings).
//! This is **not a regression test for individual fixes** — it's a
//! coarse-grained eval that establishes a baseline pass-rate across
//! the cognitive contour built up in Phases 1–5
//! (Belief → Task → Action → Verifier → Uncertainty).
//!
//! Why before Phase 6 (Tool Layer): Codex sequencing — measure first,
//! then extend. If we add tools without a baseline, we can't tell
//! whether Phase 6 improved or broke things.
//!
//! Initial baseline: every non-aspirational scenario must pass.
//! Scenarios marked `expected_failing: true` (v4.0.36) are tracked
//! separately — their PASSes are reported as "ready to promote" but
//! their FAILures don't gate CI red. This lets the dataset document
//! known gaps as concrete tests without blocking releases.
//!
//! v4.0.36 — Codex review of v4.0.35 flagged two weaknesses fixed
//! here:
//! 1. The pre-v4.0.36 harness silently `return`-ed when lexicon or
//!    dataset files were missing, so the test stayed green even
//!    when no evaluation actually ran. Fixed: missing inputs now
//!    panic with a clear message — the baseline is not skippable.
//! 2. `expected_failing` was promised in the v4.0.35 docs but not
//!    implemented. Fixed: full schema + harness support.
//!
//! The scenario JSON format is intentionally narrow — it covers what
//! the v4.0.35 trace exposes (one final-state check per scenario,
//! plus optional `with_reasoning` to attach a synthetic reasoning
//! chain for topic queries). Schema can grow as later phases add new
//! signals.

use std::collections::BTreeMap;
use std::path::Path;

use adam_dialog::{Conversation, TemplateRepository};
use adam_kernel_fst::lexicon::LexiconV1;
use adam_reasoning::reasoner::DerivedFact;
use adam_reasoning::{ConfidenceKind, FactSource, Predicate, SlotRef};
use serde::Deserialize;

const DATASET_PATH: &str = "../../data/eval/cognitive_dialog_dataset.json";

#[derive(Debug, Deserialize)]
struct Dataset {
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Deserialize)]
struct Scenario {
    id: String,
    category: String,
    #[allow(dead_code)]
    description: String,
    turns: Vec<String>,
    #[serde(default)]
    with_reasoning: bool,
    /// v4.0.36 — when `true`, this scenario is **aspirational**: its
    /// FAILures don't fail the test, but its PASSes are surfaced as
    /// "ready to promote — change `expected_failing` to false". Lets
    /// the dataset document known gaps as concrete tests without
    /// gating CI red.
    #[serde(default)]
    expected_failing: bool,
    expect: Expect,
}

/// Optional fields on the expectation block. A scenario only checks
/// what it cares about; missing fields are skipped.
#[derive(Debug, Deserialize, Default)]
struct Expect {
    epistemic_status: Option<String>,
    action: Option<String>,
    task_status: Option<String>,
    task_goal_variant: Option<String>,
    task_goal_topic: Option<String>,
    task_goal_set_at_turn: Option<usize>,
    task_subgoals_count: Option<usize>,
    belief_contradictions_count: Option<usize>,
    verification_supported: Option<bool>,
    /// Output must contain at least one of these substrings (raw).
    #[allow(dead_code)]
    output_contains_any: Option<Vec<String>>,
    /// Output (lowercased) must contain at least one of these.
    output_contains_lower_any: Option<Vec<String>>,
    /// Output (lowercased) must contain at least one of these — second slot for two independent checks.
    output_contains_lower_any_2: Option<Vec<String>>,
    /// Output must NOT contain any of these (raw match).
    output_not_contains: Option<Vec<String>>,
    /// Output (lowercased) must NOT contain any of these.
    output_not_contains_lower: Option<Vec<String>>,
}

fn load_repo() -> TemplateRepository {
    TemplateRepository::load_default().expect("templates v1.toml must exist")
}

fn load_lexicon() -> LexiconV1 {
    // v4.0.36 — Codex review of v4.0.35: pre-v4.0.36 this returned
    // Option and the test silently `return`ed when files were
    // missing. That defeated the purpose of a baseline-protecting
    // harness — the test stayed green even when no evaluation ran.
    // Now: panic with a clear message if the files aren't there.
    // CI environments must have them or the test fails red.
    let curated = "../../data/tokenizer/segmentation_roots.json";
    let apertium = "../../data/lexicon_v1/apertium_imported_roots.json";
    assert!(
        Path::new(curated).exists(),
        "cognitive_eval requires lexicon at {curated}; missing — test cannot establish baseline"
    );
    assert!(
        Path::new(apertium).exists(),
        "cognitive_eval requires apertium roots at {apertium}; missing"
    );
    LexiconV1::load(curated, apertium)
        .expect("cognitive_eval: lexicon files present but failed to parse")
}

/// A synthetic reasoning chain for «жер» — used by scenarios that
/// declare `with_reasoning: true`. We construct it locally rather
/// than load the live derived_facts.json so the harness stays
/// deterministic and doesn't depend on the corpus pipeline being
/// up-to-date in the test environment.
fn synthetic_jer_chain() -> Vec<DerivedFact> {
    vec![DerivedFact {
        subject: SlotRef {
            surface: "жер".into(),
            root: "жер".into(),
            pos: "noun".into(),
        },
        predicate: Predicate::IsA,
        object: SlotRef {
            surface: "аспан денесі".into(),
            root: "аспан денесі".into(),
            pos: "noun".into(),
        },
        rule_id: "R1_is_a_transitivity".into(),
        source_chain: vec![FactSource {
            pack: "world_core/celestial.jsonl".into(),
            sample_id: "sky_01".into(),
        }],
        confidence: ConfidenceKind::RuleInferred,
    }]
}

/// Run a single scenario; return `Ok(())` on pass or `Err(reason)`
/// on first failed assertion. Doesn't panic — the harness aggregates
/// results before deciding whether to fail the test.
fn run_scenario(s: &Scenario, lex: &LexiconV1, repo: &TemplateRepository) -> Result<(), String> {
    let mut conv = if s.with_reasoning {
        Conversation::new().with_reasoning_chains(vec![], synthetic_jer_chain())
    } else {
        Conversation::new()
    };
    let mut last_output = String::new();
    let mut last_trace_opt = None;
    for (i, input) in s.turns.iter().enumerate() {
        let (out, trace) = conv.turn_with_trace(input, lex, repo, i as u64);
        last_output = out;
        last_trace_opt = Some(trace);
    }
    let trace = last_trace_opt.ok_or_else(|| "scenario has zero turns".to_string())?;

    let exp = &s.expect;

    if let Some(want) = &exp.epistemic_status {
        let got = format!("{:?}", trace.epistemic_status);
        if &got != want {
            return Err(format!("epistemic_status: want {want}, got {got}"));
        }
    }
    if let Some(want) = &exp.action {
        let got = format!("{:?}", trace.action_digest.action);
        if &got != want {
            return Err(format!("action: want {want}, got {got}"));
        }
    }
    if let Some(want) = &exp.task_status {
        let got = format!("{:?}", trace.task_digest.status);
        if &got != want {
            return Err(format!("task_status: want {want}, got {got}"));
        }
    }
    if let Some(want) = &exp.task_goal_variant {
        match trace.task_digest.goal_variant {
            Some(v) if v == want => {}
            other => {
                return Err(format!("task_goal_variant: want {want}, got {other:?}"));
            }
        }
    }
    if let Some(want_topic) = &exp.task_goal_topic {
        let topic = match &trace.task_snapshot.active_goal {
            Some(adam_dialog::Goal::LearnAboutTopic { topic }) => Some(topic.clone()),
            Some(adam_dialog::Goal::IdentifyEntity { entity }) => Some(entity.clone()),
            _ => None,
        };
        if topic.as_deref() != Some(want_topic.as_str()) {
            return Err(format!("task_goal_topic: want {want_topic}, got {topic:?}"));
        }
    }
    if let Some(want) = exp.task_goal_set_at_turn {
        match trace.task_snapshot.goal_set_at_turn {
            Some(t) if t == want => {}
            other => {
                return Err(format!("task_goal_set_at_turn: want {want}, got {other:?}"));
            }
        }
    }
    if let Some(want) = exp.task_subgoals_count {
        let got = trace.task_snapshot.subgoals.len();
        if got != want {
            return Err(format!("task_subgoals_count: want {want}, got {got}"));
        }
    }
    if let Some(want) = exp.belief_contradictions_count {
        let got = trace.belief_digest.contradictions;
        if got != want {
            return Err(format!(
                "belief_contradictions_count: want {want}, got {got}"
            ));
        }
    }
    if let Some(want) = exp.verification_supported {
        let got = trace.verification.supported;
        if got != want {
            return Err(format!("verification_supported: want {want}, got {got}"));
        }
    }
    if let Some(any) = &exp.output_contains_lower_any {
        let lower = last_output.to_lowercase();
        if !any.iter().any(|s| lower.contains(s)) {
            return Err(format!(
                "output_contains_lower_any: none of {any:?} found in {last_output:?}"
            ));
        }
    }
    if let Some(any) = &exp.output_contains_lower_any_2 {
        let lower = last_output.to_lowercase();
        if !any.iter().any(|s| lower.contains(s)) {
            return Err(format!(
                "output_contains_lower_any_2: none of {any:?} found in {last_output:?}"
            ));
        }
    }
    if let Some(banned) = &exp.output_not_contains {
        if let Some(b) = banned.iter().find(|s| last_output.contains(s.as_str())) {
            return Err(format!(
                "output_not_contains: forbidden substring {b:?} present in {last_output:?}"
            ));
        }
    }
    if let Some(banned) = &exp.output_not_contains_lower {
        let lower = last_output.to_lowercase();
        if let Some(b) = banned.iter().find(|s| lower.contains(s.as_str())) {
            return Err(format!(
                "output_not_contains_lower: forbidden substring {b:?} present in {last_output:?}"
            ));
        }
    }

    Ok(())
}

/// Aggregated results for one category. Tracks the canonical /
/// must-pass slice and the aspirational / `expected_failing` slice
/// separately so the report can show both without conflating them.
#[derive(Default)]
struct CategoryReport {
    canonical_passed: usize,
    canonical_total: usize,
    canonical_failures: Vec<String>,
    aspirational_total: usize,
    /// Aspirational scenarios that unexpectedly **passed** —
    /// candidates to flip `expected_failing` off.
    aspirational_promotions: Vec<String>,
}

#[test]
fn cognitive_eval_baseline() {
    // v4.0.36 — both loaders are now hard-required (Codex review of
    // v4.0.35). If the workspace is in a state where lexicon or
    // dataset is missing, the baseline harness must fail red, not
    // silently pass.
    let lex = load_lexicon();
    let repo = load_repo();

    let raw = std::fs::read_to_string(DATASET_PATH).unwrap_or_else(|e| {
        panic!(
            "cognitive_eval: dataset must exist at {DATASET_PATH} for the baseline gate — got {e}"
        );
    });
    let dataset: Dataset = serde_json::from_str(&raw).expect("dataset must parse");
    assert!(
        !dataset.scenarios.is_empty(),
        "cognitive_eval dataset must contain at least one scenario"
    );

    let mut by_category: BTreeMap<String, CategoryReport> = BTreeMap::new();
    let mut canonical_passed_total = 0usize;
    let mut canonical_total = 0usize;
    let mut aspirational_total = 0usize;
    let mut aspirational_promoted_total = 0usize;

    for s in &dataset.scenarios {
        let entry = by_category.entry(s.category.clone()).or_default();
        let outcome = run_scenario(s, &lex, &repo);
        if s.expected_failing {
            entry.aspirational_total += 1;
            aspirational_total += 1;
            if outcome.is_ok() {
                aspirational_promoted_total += 1;
                entry.aspirational_promotions.push(s.id.clone());
            }
        } else {
            entry.canonical_total += 1;
            canonical_total += 1;
            match outcome {
                Ok(()) => {
                    entry.canonical_passed += 1;
                    canonical_passed_total += 1;
                }
                Err(reason) => {
                    entry
                        .canonical_failures
                        .push(format!("{}: {}", s.id, reason));
                }
            }
        }
    }

    // Print baseline report — visible in `cargo test -- --nocapture`.
    eprintln!(
        "\n=== cognitive_eval baseline (v{}) — canonical {canonical_passed_total}/{canonical_total}, aspirational promotions {aspirational_promoted_total}/{aspirational_total} ===",
        env!("CARGO_PKG_VERSION")
    );
    for (cat, r) in &by_category {
        eprintln!(
            "  {cat:30} canonical {:2}/{:2}  {}",
            r.canonical_passed,
            r.canonical_total,
            if r.canonical_failures.is_empty() {
                "OK".to_string()
            } else {
                format!("FAILED ({})", r.canonical_failures.len())
            }
        );
        for f in &r.canonical_failures {
            eprintln!("      ✗ {f}");
        }
        if r.aspirational_total > 0 {
            eprintln!(
                "  {cat:30} aspirational {}/{} ready-to-promote",
                r.aspirational_promotions.len(),
                r.aspirational_total
            );
            for id in &r.aspirational_promotions {
                eprintln!("      ⤴ {id} — flip `expected_failing` to false");
            }
        }
    }
    eprintln!();

    // Hard gate: every canonical scenario must pass. Aspirational
    // failures are OK — that's their declared contract.
    let canonical_failures: Vec<&String> = by_category
        .values()
        .flat_map(|r| r.canonical_failures.iter())
        .collect();
    assert!(
        canonical_failures.is_empty(),
        "{} canonical scenario(s) failed; see report above",
        canonical_failures.len()
    );
}
