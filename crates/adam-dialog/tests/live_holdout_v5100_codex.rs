//! **v5.10.0 — Codex follow-up review fix (B3) live holdout.**
//!
//! Loads `data/eval/live_holdout_v5100_codex.json` and asserts the
//! AskFixPreviousError follow-up route works end-to-end across
//! per-error-code specialisations + the generic `with_data` fallback
//! + the `empty` honest refusal + a regression case (unrelated help
//! request must NOT trigger the override).
//!
//! Pre-seeded session simulates the post-failed-SubmitSolution state
//! (`last_cargo_error_code` / `last_error_explanation` /
//! `last_submission_topic`). Pass-rate floor: 100 %.

use std::path::Path;

use adam_dialog::{Conversation, TemplateRepository};
use adam_kernel_fst::lexicon::LexiconV1;
use serde::Deserialize;

const DATASET_PATH: &str = "../../data/eval/live_holdout_v5100_codex.json";

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
    preseed_session: std::collections::HashMap<String, String>,
    #[serde(default)]
    any_substring: Vec<String>,
    #[serde(default)]
    none_substring: Vec<String>,
}

fn load_lexicon() -> LexiconV1 {
    let curated = "../../data/tokenizer/segmentation_roots.json";
    let apertium = "../../data/lexicon_v1/apertium_imported_roots.json";
    assert!(
        Path::new(curated).exists(),
        "live_holdout_v5100_codex requires lexicon at {curated}"
    );
    LexiconV1::load(curated, apertium).expect("live_holdout_v5100_codex: lexicon load failed")
}

fn run_case(case: &Case, lex: &LexiconV1, repo: &TemplateRepository) -> Result<String, String> {
    let mut conv = Conversation::new();
    for (k, v) in &case.preseed_session {
        conv.session.insert(k.clone(), v.clone());
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
    for forbidden in &case.none_substring {
        if lower.contains(&forbidden.to_lowercase()) {
            return Err(format!(
                "none_substring failed: forbidden {:?} present in {:?}",
                forbidden, response
            ));
        }
    }
    Ok(response)
}

#[test]
fn live_holdout_v5100_codex() {
    let lex = load_lexicon();
    let repo = TemplateRepository::load_default().expect("templates v1.toml must exist");

    let raw = std::fs::read_to_string(DATASET_PATH).unwrap_or_else(|e| {
        panic!("live_holdout_v5100_codex: dataset must exist at {DATASET_PATH} — got {e}")
    });
    let dataset: Dataset =
        serde_json::from_str(&raw).expect("live_holdout_v5100_codex JSON must parse");

    let mut by_category: std::collections::BTreeMap<String, (usize, usize)> = Default::default();
    let mut failures: Vec<(String, String, String)> = Vec::new();
    let total = dataset.cases.len();
    let mut passed = 0;

    for case in &dataset.cases {
        let entry = by_category.entry(case.category.clone()).or_insert((0, 0));
        entry.1 += 1;
        match run_case(case, &lex, &repo) {
            Ok(_) => {
                entry.0 += 1;
                passed += 1;
            }
            Err(reason) => {
                failures.push((case.id.clone(), case.category.clone(), reason));
            }
        }
    }

    eprintln!(
        "live_holdout_v5100_codex: {passed}/{total} cases passed across {} categories",
        by_category.len()
    );
    for (cat, (ok, n)) in &by_category {
        eprintln!("  {cat}: {ok}/{n}");
    }

    assert_eq!(
        failures.len(),
        0,
        "live_holdout_v5100_codex: {} cases failed:\n{}",
        failures.len(),
        failures
            .iter()
            .map(|(id, cat, msg)| format!("  [{cat}/{id}] {msg}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert_eq!(
        passed, total,
        "live_holdout_v5100_codex: pass-rate floor is 100 %"
    );
}
