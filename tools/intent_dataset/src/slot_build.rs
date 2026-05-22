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

#[derive(Debug, Serialize, Deserialize)]
struct LabelledExample {
    id: String,
    tokens: Vec<String>,
    tags: Vec<String>,
    #[serde(default)]
    source_file: String,
}

const DATASET_OUT: &str = "data/slot_extractor/v1/dataset.jsonl";
const LEXICON_CURATED: &str = "data/tokenizer/segmentation_roots.json";
const LEXICON_APERTIUM: &str = "data/lexicon_v1/apertium_imported_roots.json";
const EVAL_DIR: &str = "data/eval";
/// **E2 round 2 finding** — eval corpora are 99.7 % factual
/// queries, so cascade-on-eval yields only ~3 labelled rows.
/// E1's `seed_examples.jsonl` already carries ~ 50 self-
/// introduction rows tagged `StatementOf{Name, Age, Location,
/// Occupation, Family}` — those are the natural primary source
/// for E2. Build path: scan the seed file, isolate the
/// slot-bearing rows, run the cascade to extract the slot
/// value, then BIO-tag.
const SEED_IN: &str = "data/intent_classifier/v1/seed_examples.jsonl";
const SYNTH_IN: &str = "data/slot_extractor/v1/dataset_synth.jsonl";

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

    // **E2 round 2 — seed-row merge.** Read every row from the
    // E1 seed file whose intent is a `StatementOf*` variant,
    // re-run the cascade on it to extract the slot value, and
    // emit a BIO-tagged training row. This lifts the dataset
    // from the cascade-on-eval floor (3 rows) to a usable size
    // without any new hand-authored data.
    let mut seed_loaded = 0usize;
    let mut seed_skipped_no_slot = 0usize;
    if let Ok(seed_raw) = fs::read_to_string(SEED_IN) {
        for line in seed_raw.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            // Lenient parse — we only care about the `input` and
            // `intent` fields; the rest of the row is irrelevant
            // to E2.
            let row: serde_json::Value = match serde_json::from_str(trimmed) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let input = match row.get("input").and_then(|v| v.as_str()) {
                Some(s) => s.trim(),
                None => continue,
            };
            let intent_label = match row.get("intent").and_then(|v| v.as_str()) {
                Some(s) => s,
                None => continue,
            };
            // Only consume slot-bearing intent labels — the
            // greeting / farewell / etc. rows have no slot value
            // to extract.
            const SLOT_INTENTS: &[&str] = &[
                "StatementOfName",
                "StatementOfAge",
                "StatementOfLocation",
                "StatementOfOccupation",
                "StatementOfFamily",
            ];
            if !SLOT_INTENTS.contains(&intent_label) {
                continue;
            }
            seed_loaded += 1;
            total_seen += 1;
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
                        adam_kernel_fst::parser::analyse(&cleaned, &lexicon)
                    }
                })
                .collect();
            let intent: Intent =
                adam_dialog::semantics::interpret_text_with_lexicon(input, &parses, Some(&lexicon));
            let slot_values = slots_from_intent(&intent);
            if slot_values.is_empty() {
                seed_skipped_no_slot += 1;
                continue;
            }
            let tokens = tokenise(input);
            if tokens.is_empty() {
                continue;
            }
            let slots_refs: Vec<(adam_slot_extractor::SlotType, &str)> =
                slot_values.iter().map(|(s, v)| (*s, v.as_str())).collect();
            let tags = build_tags(&tokens, &slots_refs);
            if tags.iter().all(|t| *t == BioTag::O) {
                seed_skipped_no_slot += 1;
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
                source_file: "seed".to_string(),
            });
        }
    }

    // **Synth merge.** Same shape as the E1 build: cascade →
    // seed → synth, so the trainer sees a mixed distribution.
    let mut synth_count = 0usize;
    if let Ok(synth_raw) = fs::read_to_string(SYNTH_IN) {
        for line in synth_raw.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            match serde_json::from_str::<LabelledExample>(trimmed) {
                Ok(ex) => {
                    // Per-slot count from tags.
                    for tag in &ex.tags {
                        if let Some(slot) = match tag.as_str() {
                            "B-PER" => Some("person"),
                            "B-LOC" => Some("location"),
                            "B-AGE" => Some("age"),
                            "B-OCC" => Some("occupation"),
                            "B-FAM" => Some("family"),
                            _ => None,
                        } {
                            *per_slot_count.entry(slot.to_string()).or_default() += 1;
                        }
                    }
                    emitted.push(ex);
                    synth_count += 1;
                }
                Err(_) => {}
            }
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
    eprintln!("eval files scanned:     {}", eval_files.len());
    eprintln!("queries seen total:     {total_seen}");
    eprintln!(
        "  · cascade-on-eval:    {} kept, {} skipped (no slot)",
        emitted.len() - (seed_loaded - seed_skipped_no_slot),
        total_skipped_no_slot
    );
    eprintln!(
        "  · seed (slot-bearing): {seed_loaded} considered, {} kept, {seed_skipped_no_slot} skipped",
        seed_loaded - seed_skipped_no_slot,
    );
    eprintln!("  · synth merged:       {synth_count}");
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
