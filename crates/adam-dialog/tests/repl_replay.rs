//! v4.4.6 — REPL replay battery.
//!
//! Reads `data/eval/repl_dialogs.json` and runs each dialog through
//! `Conversation::turn` with deterministic seeds, asserting the
//! user-facing reply text against `output_contains_lower_any` /
//! `output_not_contains_lower` substring expectations on each turn.
//!
//! **Why it exists.** Codex's 2026-04-27 external review caught two
//! v4.4.0 defects (CheckContradiction renderer leak, AskAge self-recall
//! misclassification) that the cognitive_eval harness missed because
//! the trace signals were correct — only the rendered reply text was
//! wrong. The cognitive eval asserts on `(action, intent, epistemic,
//! belief)`; this asserts on what the user actually sees. The two
//! harnesses are deliberately complementary.
//!
//! Per `CONTRIBUTING.md`, every surface-text defect ships with at
//! least one new dialog here.

use std::collections::BTreeMap;
use std::path::Path;

use adam_dialog::{Conversation, TemplateRepository};
use adam_kernel_fst::lexicon::LexiconV1;
use serde::Deserialize;

const DATASET_PATH: &str = "../../data/eval/repl_dialogs.json";

#[derive(Debug, Deserialize)]
struct Dataset {
    dialogs: Vec<Dialog>,
}

#[derive(Debug, Deserialize)]
struct Dialog {
    id: String,
    category: String,
    #[allow(dead_code)]
    description: String,
    /// When `true`, this dialog is **aspirational**: its FAILures
    /// don't fail the test, but PASSes are surfaced as
    /// "ready to promote — change `expected_failing` to false".
    /// Mirrors the v4.0.36 cognitive_eval contract.
    #[serde(default)]
    expected_failing: bool,
    turns: Vec<Turn>,
}

#[derive(Debug, Deserialize)]
struct Turn {
    user: String,
    /// Output (lowercased) must contain at least one of these.
    output_contains_lower_any: Option<Vec<String>>,
    /// Output (lowercased) must NOT contain any of these.
    output_not_contains_lower: Option<Vec<String>>,
}

fn load_repo() -> TemplateRepository {
    TemplateRepository::load_default().expect("templates v1.toml must exist")
}

fn load_lexicon() -> LexiconV1 {
    let curated = "../../data/tokenizer/segmentation_roots.json";
    let apertium = "../../data/lexicon_v1/apertium_imported_roots.json";
    assert!(
        Path::new(curated).exists(),
        "repl_replay requires lexicon at {curated}; missing — test cannot establish baseline"
    );
    assert!(
        Path::new(apertium).exists(),
        "repl_replay requires apertium roots at {apertium}; missing"
    );
    LexiconV1::load(curated, apertium)
        .expect("repl_replay: lexicon files present but failed to parse")
}

/// Run a single dialog. Returns `Ok(())` on pass or `Err(reason)` on
/// the first failed assertion. Doesn't panic — the harness aggregates
/// before deciding whether to fail the test.
fn run_dialog(d: &Dialog, lex: &LexiconV1, repo: &TemplateRepository) -> Result<(), String> {
    let mut conv = Conversation::new();
    for (i, t) in d.turns.iter().enumerate() {
        let out = conv.turn(&t.user, lex, repo, i as u64);
        let lower = out.to_lowercase();
        if let Some(any) = &t.output_contains_lower_any {
            if !any.iter().any(|s| lower.contains(s)) {
                return Err(format!(
                    "turn {} ({:?}): output_contains_lower_any: none of {:?} found in {:?}",
                    i, t.user, any, out
                ));
            }
        }
        if let Some(banned) = &t.output_not_contains_lower {
            if let Some(b) = banned.iter().find(|s| lower.contains(s.as_str())) {
                return Err(format!(
                    "turn {} ({:?}): output_not_contains_lower: forbidden substring {:?} present in {:?}",
                    i, t.user, b, out
                ));
            }
        }
    }
    Ok(())
}

#[derive(Default)]
struct CategoryReport {
    canonical_passed: usize,
    canonical_total: usize,
    canonical_failures: Vec<String>,
    aspirational_total: usize,
    aspirational_promotions: Vec<String>,
}

#[test]
fn repl_replay_baseline() {
    let lex = load_lexicon();
    let repo = load_repo();

    let raw = std::fs::read_to_string(DATASET_PATH).unwrap_or_else(|e| {
        panic!("repl_replay: dataset must exist at {DATASET_PATH} for the baseline gate — got {e}")
    });
    let dataset: Dataset = serde_json::from_str(&raw).expect("repl_dialogs.json must parse");
    assert!(
        !dataset.dialogs.is_empty(),
        "repl_dialogs.json must contain at least one dialog"
    );

    let mut by_category: BTreeMap<String, CategoryReport> = BTreeMap::new();
    let mut canonical_passed_total = 0usize;
    let mut canonical_total = 0usize;
    let mut aspirational_total = 0usize;
    let mut aspirational_promoted_total = 0usize;

    for d in &dataset.dialogs {
        let entry = by_category.entry(d.category.clone()).or_default();
        let outcome = run_dialog(d, &lex, &repo);
        if d.expected_failing {
            entry.aspirational_total += 1;
            aspirational_total += 1;
            if outcome.is_ok() {
                aspirational_promoted_total += 1;
                entry.aspirational_promotions.push(d.id.clone());
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
                        .push(format!("{}: {}", d.id, reason));
                }
            }
        }
    }

    eprintln!(
        "\n=== repl_replay baseline (v{}) — canonical {canonical_passed_total}/{canonical_total}, aspirational promotions {aspirational_promoted_total}/{aspirational_total} ===",
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

    let canonical_failures: Vec<&String> = by_category
        .values()
        .flat_map(|r| r.canonical_failures.iter())
        .collect();
    assert!(
        canonical_failures.is_empty(),
        "{} canonical REPL replay dialog(s) failed; see report above",
        canonical_failures.len()
    );
}
