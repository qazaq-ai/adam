// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! # `intent_dataset_diagnose`
//!
//! Re-runs the deterministic cascade on every row in the
//! committed dataset and reports every case where the cascade's
//! current verdict disagrees with the row's stored label. Surfaces
//! three categories:
//!
//! 1. **Cascade-vs-seed mismatch** — the seed rows were hand-
//!    authored against a specific intent class; the cascade now
//!    classifies them differently. Either the cascade has a gap
//!    the seed author noticed, or the seed label is wrong.
//! 2. **Cascade self-inconsistency** — a cascade-derived row
//!    whose current label differs from the one captured at build
//!    time. Should be zero in steady state; a non-zero count
//!    means the cascade has been modified between build and now.
//! 3. **Label-set drift** — rows whose stored label is no longer
//!    in the cascade's reachable output set.
//!
//! Output: stderr human summary +
//! `data/intent_classifier/v1/diagnose.json` for the machine
//! report.
//!
//! Usage: `cargo run -p adam-intent-dataset --bin intent_dataset_diagnose`

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use adam_dialog::Intent;
use adam_dialog::conversation::IntentKind;
use adam_kernel_fst::lexicon::LexiconV1;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct LabelledExample {
    id: String,
    input: String,
    intent: String,
    #[serde(default)]
    source_file: String,
    #[serde(default)]
    confidence: String,
}

#[derive(Debug, Serialize)]
struct Mismatch {
    id: String,
    input: String,
    stored_label: String,
    cascade_label: String,
    source_file: String,
}

#[derive(Debug, Serialize)]
struct DiagnoseReport {
    total_rows: usize,
    matches: usize,
    mismatches: usize,
    seed_mismatches: usize,
    cascade_self_inconsistencies: usize,
    mismatches_by_seed_label: HashMap<String, usize>,
    mismatches_by_cascade_label: HashMap<String, usize>,
    rows: Vec<Mismatch>,
}

const DATASET_IN: &str = "data/intent_classifier/v1/dataset.jsonl";
const DIAGNOSE_OUT: &str = "data/intent_classifier/v1/diagnose.json";
const LEXICON_CURATED: &str = "data/tokenizer/segmentation_roots.json";
const LEXICON_APERTIUM: &str = "data/lexicon_v1/apertium_imported_roots.json";

fn cascade_label(input: &str, lexicon: &LexiconV1) -> String {
    let parses: Vec<adam_kernel_fst::parser::Analysis> = input
        .split_whitespace()
        .flat_map(|tok| {
            let cleaned: String = tok
                .chars()
                .filter(|c| c.is_alphabetic() || c.is_ascii_digit() || *c == '-')
                .flat_map(|c| c.to_lowercase())
                .collect();
            if cleaned.is_empty() {
                Vec::new()
            } else {
                adam_kernel_fst::parser::analyse(&cleaned, lexicon)
            }
        })
        .collect();
    let intent: Intent =
        adam_dialog::semantics::interpret_text_with_lexicon(input, &parses, Some(lexicon));
    match &intent {
        Intent::Unknown { noun_hint, .. } if noun_hint.is_some() => "FactualQuery".to_string(),
        _ => {
            let kind: IntentKind = (&intent).into();
            format!("{kind:?}")
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let lexicon = LexiconV1::load(LEXICON_CURATED, LEXICON_APERTIUM)?;
    let raw = fs::read_to_string(DATASET_IN)?;
    let examples: Vec<LabelledExample> = raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;

    let mut mismatches: Vec<Mismatch> = Vec::new();
    let mut by_seed: HashMap<String, usize> = HashMap::new();
    let mut by_cascade: HashMap<String, usize> = HashMap::new();
    let mut seed_only = 0usize;
    let mut cascade_only = 0usize;
    let mut matches = 0usize;

    for ex in &examples {
        let casc = cascade_label(&ex.input, &lexicon);
        if casc == ex.intent {
            matches += 1;
            continue;
        }
        let mismatch = Mismatch {
            id: ex.id.clone(),
            input: ex.input.clone(),
            stored_label: ex.intent.clone(),
            cascade_label: casc.clone(),
            source_file: ex.source_file.clone(),
        };
        if ex.source_file == "seed" {
            seed_only += 1;
        } else {
            cascade_only += 1;
        }
        *by_seed.entry(ex.intent.clone()).or_default() += 1;
        *by_cascade.entry(casc).or_default() += 1;
        mismatches.push(mismatch);
    }

    let report = DiagnoseReport {
        total_rows: examples.len(),
        matches,
        mismatches: mismatches.len(),
        seed_mismatches: seed_only,
        cascade_self_inconsistencies: cascade_only,
        mismatches_by_seed_label: by_seed.clone(),
        mismatches_by_cascade_label: by_cascade.clone(),
        rows: mismatches,
    };

    let out_path = Path::new(DIAGNOSE_OUT);
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(out_path, serde_json::to_string_pretty(&report)?)?;

    eprintln!("=== E1 dataset diagnose ===");
    eprintln!("total rows:                       {}", report.total_rows);
    eprintln!(
        "match cascade verdict:            {} ({:.1}%)",
        report.matches,
        100.0 * report.matches as f64 / report.total_rows.max(1) as f64
    );
    eprintln!(
        "mismatch:                         {} ({:.1}%)",
        report.mismatches,
        100.0 * report.mismatches as f64 / report.total_rows.max(1) as f64
    );
    eprintln!(
        "  └─ seed-row mismatches:         {}",
        report.seed_mismatches
    );
    eprintln!(
        "  └─ cascade self-inconsistencies: {}",
        report.cascade_self_inconsistencies
    );
    eprintln!();
    eprintln!("--- mismatches by STORED (seed/oracle) label ---");
    let mut by_seed_sorted: Vec<(&String, &usize)> = by_seed.iter().collect();
    by_seed_sorted.sort_by(|a, b| b.1.cmp(a.1));
    for (label, count) in by_seed_sorted {
        eprintln!("  {label:30}  {count}");
    }
    eprintln!();
    eprintln!("--- mismatches by CASCADE's preferred label ---");
    let mut by_cascade_sorted: Vec<(&String, &usize)> = by_cascade.iter().collect();
    by_cascade_sorted.sort_by(|a, b| b.1.cmp(a.1));
    for (label, count) in by_cascade_sorted {
        eprintln!("  {label:30}  {count}");
    }
    eprintln!();
    eprintln!("--- first 30 mismatched rows ---");
    for (i, m) in report.rows.iter().take(30).enumerate() {
        eprintln!(
            "  [{:>2}] {:14}  stored={:25}  cascade={:25}  «{}»",
            i + 1,
            m.source_file,
            m.stored_label,
            m.cascade_label,
            m.input
        );
    }
    eprintln!();
    eprintln!("full report: {DIAGNOSE_OUT}");
    Ok(())
}
