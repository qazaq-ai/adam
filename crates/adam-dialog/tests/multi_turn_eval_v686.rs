// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! Integration test for the v6.8.6 L4.7 multi-turn eval framework.
//! Loads every `data/eval_multi_turn/*.jsonl` case and runs it
//! against `Conversation::turn` end-to-end, asserting each case
//! passes all of its embedded assertions.
//!
//! Skips when the curated lexicon is missing (trimmed checkout).

use adam_dialog::TemplateRepository;
use adam_dialog::multi_turn_eval::{MultiTurnCase, case_from_jsonl_line, run_case};
use adam_kernel_fst::lexicon::LexiconV1;

const FIXTURE_DIR: &str = "../../data/eval_multi_turn";

fn enable_v6_2() {
    unsafe {
        std::env::set_var("ADAM_V6_2", "1");
    }
}

fn load_lexicon() -> Option<LexiconV1> {
    let curated = "../../data/tokenizer/segmentation_roots.json";
    let apertium = "../../data/lexicon_v1/apertium_imported_roots.json";
    if !std::path::Path::new(curated).exists() || !std::path::Path::new(apertium).exists() {
        eprintln!("[multi_turn_eval_v686] lexicon not present, skipping");
        return None;
    }
    LexiconV1::load(curated, apertium).ok()
}

fn load_all_cases() -> Vec<MultiTurnCase> {
    let mut out = Vec::new();
    let dir = std::fs::read_dir(FIXTURE_DIR).expect("eval_multi_turn dir must exist");
    for entry in dir {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("read fixture");
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            out.push(case_from_jsonl_line(line).expect("parse multi-turn case"));
        }
    }
    out
}

#[test]
fn fixture_set_is_non_empty() {
    let cases = load_all_cases();
    assert!(
        !cases.is_empty(),
        "expected at least one multi-turn case under {FIXTURE_DIR}",
    );
}

/// **Production gate.** Every case tagged
/// `expected_to_pass=true` (the default) MUST pass.  A regression
/// here fails the build.
#[test]
fn every_required_case_passes_under_current_production() {
    enable_v6_2();
    let Some(lex) = load_lexicon() else { return };
    let repo = TemplateRepository::load_default().expect("templates v1.toml must exist");
    let cases = load_all_cases();
    let mut all_failures = Vec::new();
    let mut required = 0usize;
    for case in &cases {
        if !case.expected_to_pass {
            continue;
        }
        required += 1;
        let result = run_case(case, &lex, &repo);
        if !result.passed {
            all_failures.extend(result.failures);
        }
    }
    assert!(required > 0, "no required cases in fixture set");
    assert!(
        all_failures.is_empty(),
        "{} required multi-turn assertion(s) failed (of {} required cases):\n  - {}",
        all_failures.len(),
        required,
        all_failures.join("\n  - "),
    );
}

/// **Probe diagnostics — non-failing.**  Cases tagged
/// `expected_to_pass=false` capture known gaps.  This test
/// counts them and prints which still fail so future work can
/// see at-a-glance how many gaps are open without making the
/// build red.  Asserts only that the diagnostic ran — never
/// fails on a probe's failure.
#[test]
fn probes_diagnostic_count() {
    enable_v6_2();
    let Some(lex) = load_lexicon() else { return };
    let repo = TemplateRepository::load_default().expect("templates v1.toml must exist");
    let cases = load_all_cases();
    let mut probe_total = 0usize;
    let mut probe_pass = 0usize;
    for case in &cases {
        if case.expected_to_pass {
            continue;
        }
        probe_total += 1;
        let result = run_case(case, &lex, &repo);
        if result.passed {
            probe_pass += 1;
            eprintln!(
                "[probe-PASS] {} (gap may have closed — promote to required)",
                case.id
            );
        } else {
            eprintln!(
                "[probe-OPEN] {} ({} assertion(s) failing)",
                case.id,
                result.failures.len()
            );
        }
    }
    eprintln!(
        "[multi-turn probes] {probe_pass}/{probe_total} passing (gaps closing as work lands)",
    );
}
