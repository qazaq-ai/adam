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
//! Initial baseline: every scenario must pass. Future scenarios can
//! be marked `expected_failing: true` (not yet supported) to track
//! aspirational coverage without gating CI red.
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

fn load_lexicon() -> Option<LexiconV1> {
    let curated = "../../data/tokenizer/segmentation_roots.json";
    let apertium = "../../data/lexicon_v1/apertium_imported_roots.json";
    if !Path::new(curated).exists() || !Path::new(apertium).exists() {
        eprintln!("cognitive_eval: lexicon files not present, skipping");
        return None;
    }
    LexiconV1::load(curated, apertium).ok()
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

#[test]
fn cognitive_eval_baseline() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();

    let raw = match std::fs::read_to_string(DATASET_PATH) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cognitive_eval: dataset not present at {DATASET_PATH}: {e}");
            return;
        }
    };
    let dataset: Dataset = serde_json::from_str(&raw).expect("dataset must parse");

    let mut by_category: BTreeMap<String, (usize, usize, Vec<String>)> = BTreeMap::new();
    let mut total_passed = 0;
    let mut total = 0;

    for s in &dataset.scenarios {
        total += 1;
        let entry = by_category.entry(s.category.clone()).or_default();
        entry.1 += 1;
        match run_scenario(s, &lex, &repo) {
            Ok(()) => {
                total_passed += 1;
                entry.0 += 1;
            }
            Err(reason) => {
                entry.2.push(format!("{}: {}", s.id, reason));
            }
        }
    }

    // Print baseline report — visible in `cargo test -- --nocapture`.
    eprintln!("\n=== cognitive_eval baseline (v4.0.35) — total {total_passed}/{total} ===");
    for (cat, (passed, total, failures)) in &by_category {
        eprintln!(
            "  {cat:30} {passed:2}/{total:2}  {}",
            if failures.is_empty() {
                "OK".to_string()
            } else {
                format!("FAILED ({})", failures.len())
            }
        );
        for f in failures {
            eprintln!("      ✗ {f}");
        }
    }
    eprintln!();

    // Initial baseline: every scenario must pass.
    let failures: Vec<&String> = by_category
        .values()
        .flat_map(|(_, _, fs)| fs.iter())
        .collect();
    assert!(
        failures.is_empty(),
        "{} scenario(s) failed; see report above",
        failures.len()
    );
}
