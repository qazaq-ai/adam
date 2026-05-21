// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! # `intent_dataset_build`
//!
//! Walks every `data/eval/*.json` evaluation pack, extracts the
//! `query` field of each case, and runs it through the existing
//! deterministic cascade (`adam_dialog::interpret_text_with_lexicon`).
//! For each turn that ends in a known [`IntentKind`] **other than
//! `Unknown`**, emits one labelled training example.
//!
//! Output: `data/intent_classifier/v1/dataset.jsonl` — one JSON
//! line per labelled pair.
//!
//! See `docs/e1_intent_classifier_design.md` § "Training data /
//! Sources" for the labelling-confidence policy and the schema
//! contract.
//!
//! Usage: `cargo run -p adam-intent-dataset --bin intent_dataset_build`

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use adam_dialog::Intent;
use adam_dialog::conversation::IntentKind;
use adam_kernel_fst::lexicon::LexiconV1;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct EvalCase {
    /// The query the eval pack tests. Field is named `query` in
    /// every pack we ship as of 2026-05-21.
    #[serde(default)]
    query: Option<String>,
    /// Some packs use `prompt` instead.
    #[serde(default)]
    prompt: Option<String>,
    /// Some packs use `input`.
    #[serde(default)]
    input: Option<String>,
}

impl EvalCase {
    fn surface(&self) -> Option<&str> {
        self.query
            .as_deref()
            .or(self.prompt.as_deref())
            .or(self.input.as_deref())
    }
}

/// Lenient envelope — the eval packs vary in top-level shape;
/// we only care about a `cases` array or `prompts` array.
#[derive(Debug, Deserialize)]
struct EvalEnvelope {
    #[serde(default)]
    cases: Option<Vec<EvalCase>>,
    #[serde(default)]
    prompts: Option<Vec<EvalCase>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LabelledExample {
    id: String,
    input: String,
    intent: String,
    #[serde(default, rename = "source_file")]
    source_file: String,
    /// Per-class confidence: `high` for a non-Unknown label,
    /// `low` for Unknown / no match. We **skip** low rows in the
    /// emitted dataset — they would teach the classifier
    /// "Unknown" as a positive class, which is not what we want.
    confidence: String,
}

const DATASET_OUT: &str = "data/intent_classifier/v1/dataset.jsonl";
const SEED_IN: &str = "data/intent_classifier/v1/seed_examples.jsonl";
const LEXICON_CURATED: &str = "data/tokenizer/segmentation_roots.json";
const LEXICON_APERTIUM: &str = "data/lexicon_v1/apertium_imported_roots.json";
const EVAL_DIR: &str = "data/eval";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let lexicon = LexiconV1::load(LEXICON_CURATED, LEXICON_APERTIUM)?;

    let eval_files: Vec<PathBuf> = fs::read_dir(EVAL_DIR)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "json").unwrap_or(false))
        .collect();

    let mut emitted: Vec<LabelledExample> = Vec::new();
    let mut per_intent_count: HashMap<String, usize> = HashMap::new();
    let mut total_seen = 0usize;
    let mut total_unknown = 0usize;

    for path in &eval_files {
        let bytes = match fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("skip {}: {}", path.display(), e);
                continue;
            }
        };
        let envelope: EvalEnvelope = match serde_json::from_slice(&bytes) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("skip {}: parse failure {}", path.display(), e);
                continue;
            }
        };
        let cases: Vec<EvalCase> = envelope
            .cases
            .into_iter()
            .flatten()
            .chain(envelope.prompts.into_iter().flatten())
            .collect();
        let source_file = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "?".to_string());

        for case in cases {
            let Some(surface) = case.surface() else {
                continue;
            };
            let trimmed = surface.trim();
            if trimmed.is_empty() {
                continue;
            }
            total_seen += 1;
            // Whitespace-tokenise + per-token FST analyse — the
            // minimal subset of `adam_dialog::parse_input_inner`
            // we need without touching its priors / alpha plumbing.
            // Cleaning rules mirror that function: keep alphabetic,
            // digit, and hyphen; lowercase. Empty cleaned tokens
            // are skipped so they don't pollute the parse stream.
            let parses: Vec<adam_kernel_fst::parser::Analysis> = trimmed
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
                        adam_kernel_fst::parser::analyse(&cleaned, &lexicon)
                    }
                })
                .collect();
            let intent: Intent = adam_dialog::semantics::interpret_text_with_lexicon(
                trimmed,
                &parses,
                Some(&lexicon),
            );
            // **E1 dataset findung 2026-05-21.** The cascade buckets
            // most evaluation queries as `Unknown` even when they
            // carry a clear topic noun. Splitting that bucket into
            // two positive classes lets the classifier learn the
            // dominant input type instead of dropping 91 % of the
            // dataset:
            //   - `Unknown { noun_hint: Some(_) }` → "FactualQuery"
            //     (the user asked about a named topic; the cascade
            //     just doesn't have a tight enum slot for it).
            //   - `Unknown { noun_hint: None }`   → still dropped
            //     (genuine "I have no idea what this is" cases).
            let label = match &intent {
                Intent::Unknown { noun_hint, .. } => {
                    if noun_hint.is_some() {
                        "FactualQuery".to_string()
                    } else {
                        total_unknown += 1;
                        continue;
                    }
                }
                _ => {
                    let kind: IntentKind = (&intent).into();
                    format!("{kind:?}")
                }
            };
            *per_intent_count.entry(label.clone()).or_default() += 1;
            let id = format!("ds_{:05}", emitted.len() + 1);
            emitted.push(LabelledExample {
                id,
                input: trimmed.to_string(),
                intent: label,
                source_file: source_file.clone(),
                confidence: "high".to_string(),
            });
        }
    }

    // **Seed examples merge** — hand-crafted minority-class
    // examples written directly to `data/intent_classifier/v1/
    // seed_examples.jsonl`. Closes the class imbalance the first
    // dataset build (84 cascade-labelled examples) surfaced:
    // FactualQuery dominates everything else. Seed rows are
    // appended *after* the cascade-derived rows so the
    // classifier's training loop sees a more balanced mix per
    // class. Format: same `LabelledExample` schema, deserialised
    // tolerantly so we don't break on schema drift.
    let mut seed_count = 0usize;
    if let Ok(seed_raw) = fs::read_to_string(SEED_IN) {
        for line in seed_raw.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            match serde_json::from_str::<LabelledExample>(trimmed) {
                Ok(ex) => {
                    *per_intent_count.entry(ex.intent.clone()).or_default() += 1;
                    emitted.push(ex);
                    seed_count += 1;
                }
                Err(e) => {
                    eprintln!("skip seed row: {e}");
                }
            }
        }
    } else {
        eprintln!("note: no seed file at {SEED_IN} — skipping seed merge");
    }

    // Write dataset.
    let out_path = Path::new(DATASET_OUT);
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut buf = String::new();
    for ex in &emitted {
        buf.push_str(&serde_json::to_string(ex)?);
        buf.push('\n');
    }
    fs::write(out_path, buf)?;

    // Summary to stderr — design-doc reporting contract.
    eprintln!("=== E1 dataset build summary ===");
    eprintln!("eval files scanned: {}", eval_files.len());
    eprintln!("queries seen:        {total_seen}");
    eprintln!(
        "cascade-labelled:    {} ({:.1}%)",
        emitted.len() - seed_count,
        100.0 * (emitted.len() - seed_count) as f64 / total_seen.max(1) as f64
    );
    eprintln!("seed rows merged:    {seed_count}");
    eprintln!("total emitted:       {}", emitted.len());
    eprintln!(
        "dropped (Unknown):   {total_unknown} ({:.1}%)",
        100.0 * total_unknown as f64 / total_seen.max(1) as f64
    );
    eprintln!("output: {DATASET_OUT}");
    eprintln!();
    eprintln!("--- per-intent class counts ---");
    let mut classes: Vec<(&String, &usize)> = per_intent_count.iter().collect();
    classes.sort_by(|a, b| b.1.cmp(a.1));
    for (label, count) in classes {
        eprintln!("  {label:30}  {count}");
    }

    Ok(())
}
