// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! # `slot_dataset_build`
//!
//! Builds the labelled BIO-tagged training corpus for the **E2
//! slot-extractor** experiment. Walks every `data/eval/*.json`
//! file, extracts each `query`, runs it through the existing
//! deterministic cascade
//! (`adam_dialog::semantics::interpret_text_with_lexicon`), and
//! for every turn whose Intent carries a slot value
//! (`StatementOfName { name }`, `StatementOfAge { years }`,
//! `StatementOfLocation { city }`, `StatementOfOccupation {
//! occupation }`, `StatementOfFamily`), emits one labelled
//! example.
//!
//! Each example carries:
//!   - `tokens`: whitespace-split tokens of the query (lowercased,
//!     punctuation stripped — same pre-processing as the
//!     classifier).
//!   - `tags`: per-token BIO labels. Tokens that match the
//!     cascade's extracted slot value get `B-<TYPE>` /
//!     `I-<TYPE>`; everything else gets `O`.
//!
//! Output: `data/slot_extractor/v1/dataset.jsonl`.
//!
//! See `docs/e2_slot_extractor_design.md` for the BIO inventory
//! and labelling-confidence policy.
//!
//! Usage: `cargo run -p adam-intent-dataset --bin slot_dataset_build`

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use adam_dialog::Intent;
use adam_kernel_fst::lexicon::LexiconV1;
use adam_slot_extractor::BioTag;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct EvalCase {
    #[serde(default)]
    query: Option<String>,
    #[serde(default)]
    prompt: Option<String>,
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

#[derive(Debug, Deserialize)]
struct EvalEnvelope {
    #[serde(default)]
    cases: Option<Vec<EvalCase>>,
    #[serde(default)]
    prompts: Option<Vec<EvalCase>>,
}

#[derive(Debug, Serialize)]
struct LabelledExample {
    id: String,
    tokens: Vec<String>,
    tags: Vec<String>,
    source_file: String,
}

const DATASET_OUT: &str = "data/slot_extractor/v1/dataset.jsonl";
const LEXICON_CURATED: &str = "data/tokenizer/segmentation_roots.json";
const LEXICON_APERTIUM: &str = "data/lexicon_v1/apertium_imported_roots.json";
const EVAL_DIR: &str = "data/eval";

/// Whitespace-tokenise + lowercase + strip surrounding
/// punctuation. Same pre-processing as the classifier so the two
/// trained artefacts see the same input distribution.
fn tokenise(input: &str) -> Vec<String> {
    input
        .split_whitespace()
        .map(|tok| {
            tok.chars()
                .filter(|c| c.is_alphabetic() || c.is_ascii_digit() || *c == '-')
                .flat_map(|c| c.to_lowercase())
                .collect::<String>()
        })
        .filter(|t| !t.is_empty())
        .collect()
}

/// Find the contiguous run of tokens whose lowercased forms
/// match the slot value (also lowercased + tokenised). Returns
/// `(start, end)` half-open, or `None` if no match found.
fn locate_span(tokens: &[String], value: &str) -> Option<(usize, usize)> {
    let value_tokens = tokenise(value);
    if value_tokens.is_empty() {
        return None;
    }
    // Try exact contiguous match first.
    let n = value_tokens.len();
    for start in 0..tokens.len().saturating_sub(n - 1) {
        if tokens[start..start + n] == value_tokens[..] {
            return Some((start, start + n));
        }
    }
    // Fallback: single-token match on the first content token
    // (proper-noun case: the cascade may have stripped morphology
    // off the lexical surface).
    let head = &value_tokens[0];
    if let Some(pos) = tokens.iter().position(|t| t.starts_with(head.as_str())) {
        return Some((pos, pos + 1));
    }
    None
}

/// Build the BIO-tag vector for an input.
fn build_tags(tokens: &[String], spans: &[(adam_slot_extractor::SlotType, &str)]) -> Vec<BioTag> {
    let mut tags = vec![BioTag::O; tokens.len()];
    for (slot, value) in spans {
        if let Some((start, end)) = locate_span(tokens, value) {
            let (b, i) = match slot {
                adam_slot_extractor::SlotType::Person => (BioTag::BPer, BioTag::IPer),
                adam_slot_extractor::SlotType::Location => (BioTag::BLoc, BioTag::ILoc),
                adam_slot_extractor::SlotType::Age => (BioTag::BAge, BioTag::IAge),
                adam_slot_extractor::SlotType::Occupation => (BioTag::BOcc, BioTag::IOcc),
                adam_slot_extractor::SlotType::Family => (BioTag::BFam, BioTag::IFam),
            };
            for (offset, idx) in (start..end).enumerate() {
                if idx >= tags.len() {
                    break;
                }
                // Don't overwrite a tag from an earlier span (rare —
                // would mean two slots claim the same token).
                if tags[idx] != BioTag::O {
                    continue;
                }
                tags[idx] = if offset == 0 { b } else { i };
            }
        }
    }
    tags
}

/// Extract the slot value(s) the cascade attached to this turn.
/// Returns a vec because a single Intent may carry multiple
/// slots (e.g. `StatementOfFamily` carries no slot value in v1).
fn slots_from_intent(intent: &Intent) -> Vec<(adam_slot_extractor::SlotType, String)> {
    let mut out: Vec<(adam_slot_extractor::SlotType, String)> = Vec::new();
    match intent {
        Intent::StatementOfName { name } => {
            out.push((adam_slot_extractor::SlotType::Person, name.clone()));
        }
        Intent::StatementOfAge { years } => {
            if let Some(y) = years {
                out.push((adam_slot_extractor::SlotType::Age, y.to_string()));
            }
        }
        Intent::StatementOfLocation { city } => {
            if let Some(c) = city {
                out.push((adam_slot_extractor::SlotType::Location, c.clone()));
            }
        }
        Intent::StatementOfOccupation { occupation } => {
            if let Some(o) = occupation {
                out.push((adam_slot_extractor::SlotType::Occupation, o.clone()));
            }
        }
        _ => {}
    }
    out
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let lexicon = LexiconV1::load(LEXICON_CURATED, LEXICON_APERTIUM)?;

    let eval_files: Vec<PathBuf> = fs::read_dir(EVAL_DIR)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "json").unwrap_or(false))
        .collect();

    let mut emitted: Vec<LabelledExample> = Vec::new();
    let mut per_slot_count: HashMap<String, usize> = HashMap::new();
    let mut total_seen = 0usize;
    let mut total_skipped_no_slot = 0usize;

    for path in &eval_files {
        let bytes = match fs::read(path) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let envelope: EvalEnvelope = match serde_json::from_slice(&bytes) {
            Ok(e) => e,
            Err(_) => continue,
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

            // Reuse the same minimal FST-analyse path used by the
            // intent dataset builder so analyses match production
            // pre-processing.
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
            let slot_values = slots_from_intent(&intent);
            if slot_values.is_empty() {
                total_skipped_no_slot += 1;
                continue;
            }
            let tokens = tokenise(trimmed);
            if tokens.is_empty() {
                continue;
            }
            let slots_refs: Vec<(adam_slot_extractor::SlotType, &str)> =
                slot_values.iter().map(|(s, v)| (*s, v.as_str())).collect();
            let tags = build_tags(&tokens, &slots_refs);
            // Skip examples where no slot actually landed on a
            // token (e.g. cascade emitted a slot value that
            // doesn't appear in the surface). Honest dataset
            // hygiene — we don't want to teach the model that
            // every utterance has a slot.
            if tags.iter().all(|t| *t == BioTag::O) {
                continue;
            }
            for (slot, _) in &slot_values {
                *per_slot_count.entry(slot.slug().to_string()).or_default() += 1;
            }
            let id = format!("ds_{:05}", emitted.len() + 1);
            emitted.push(LabelledExample {
                id,
                tokens,
                tags: tags.iter().map(|t| t.slug().to_string()).collect(),
                source_file: source_file.clone(),
            });
        }
    }

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

    eprintln!("=== E2 slot-dataset build summary ===");
    eprintln!("eval files scanned:    {}", eval_files.len());
    eprintln!("queries seen:           {total_seen}");
    eprintln!(
        "skipped (no slot):      {total_skipped_no_slot} ({:.1}%)",
        100.0 * total_skipped_no_slot as f64 / total_seen.max(1) as f64
    );
    eprintln!("labelled emitted:       {}", emitted.len());
    eprintln!("output:                 {DATASET_OUT}");
    eprintln!();
    eprintln!("--- per-slot example counts ---");
    let mut sorted: Vec<(&String, &usize)> = per_slot_count.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));
    for (slot, count) in sorted {
        eprintln!("  {slot:20}  {count}");
    }
    Ok(())
}
