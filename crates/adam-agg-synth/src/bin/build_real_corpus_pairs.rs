// SPDX-License-Identifier: MIT
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Read real Kazakh corpora — both pre-built sentence packs in
//! `data/curated/*_pack.json` and raw CSVs from
//! `data/external/huggingface_kz/` — and emit a single
//! `Vec<TrainingPair>` JSON that the adam-agg-model PoC binary can
//! consume in place of (or alongside) the FST-synthetic pairs.
//!
//! Pipeline:
//!   1. Source set is hard-coded for now. Each source contributes a
//!      `Vec<String>` of raw text (sentences from packs, or per-book
//!      cells from CSV filtered by `predicted_language == "kaz"`).
//!   2. Each text is split into alphabetic words; each word is fed
//!      through `AggTokenizer` via `SynthGenerator::pairs_from_text`.
//!   3. Resulting `TrainingPair`s are deduplicated by surface, capped
//!      at `--max-pairs`, and written to `--out`.
//!
//! Why dedup by surface: word-level training emits one pair per
//! surface; the most common Kazakh function words would dominate
//! otherwise. After dedup the distribution is closer to type-level,
//! which mirrors the FST-synth pipeline.

use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

use adam_agg_synth::{SynthGenerator, TrainingPair};
use adam_agg_tokenizer::{AggTokenizer, RootPos};
use adam_kernel_fst::lexicon::LexiconV1;
use serde::Deserialize;

/// Schema of `data/curated/*_pack.json` packs.
///
/// Two flavours seen in the tree: either each sample is a bare
/// string, or it's an object with at least a `text` field (and
/// metadata like `id`, `pack_name`, `domain`). We accept both via an
/// untagged enum and project to a `&str` downstream.
#[derive(Deserialize)]
struct CuratedPack {
    samples: Vec<Sample>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Sample {
    Plain(String),
    Object {
        #[serde(default)]
        text: String,
    },
}

impl Sample {
    fn text(&self) -> &str {
        match self {
            Sample::Plain(s) => s,
            Sample::Object { text } => text,
        }
    }
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn main() {
    let max_pairs = env_usize("MAX_PAIRS", 200_000);
    let max_books_csv = env_usize("MAX_BOOKS_CSV", 200);
    let max_chars_per_book = env_usize("MAX_CHARS_PER_BOOK", 20_000);
    let out_path: String =
        std::env::var("OUT_PATH").unwrap_or_else(|_| "data/curated/real_corpus_pairs.json".into());

    // -- Stage 1: load lexicon + tokenizer ----------------------------------
    let curated = "data/tokenizer/segmentation_roots.json";
    let apertium = "data/lexicon_v1/apertium_imported_roots.json";
    if !Path::new(curated).exists() {
        eprintln!("Lexicon files missing; run from repo root.");
        std::process::exit(1);
    }
    let lex = LexiconV1::load(curated, apertium).expect("lexicon load");
    let tokenizer = AggTokenizer::build(lex.clone());
    let mut generator = SynthGenerator::new(&lex, &tokenizer);
    eprintln!(
        "[1/4] Lexicon + tokenizer ready ({} entries)",
        lex.entries_ordered.len()
    );

    // -- Stage 2: read every committed Kazakh-language pack -----------------
    let pack_paths = [
        "data/curated/rust_book_kk_pack.json",
        "data/curated/kazakh_textbooks_pack.json",
        "data/curated/filtered_wikipedia_kz_pack.json",
        "data/curated/filtered_cc100_kk_pack.json",
        "data/curated/filtered_synthetic_sentences_pack.json",
        "data/curated/abai_wikisource_pack.json",
        "data/curated/tatoeba_kazakh_pack.json",
        "data/curated/kazakh_classics_pack.json",
        "data/curated/kazakh_proverbs_pack.json",
        "data/curated/clean_general_core_pack.json",
        "data/curated/clean_general_extension_pack.json",
        "data/curated/clean_education_core_pack.json",
        "data/curated/clean_education_extension_pack.json",
        "data/curated/clean_reference_core_pack.json",
        "data/curated/clean_reference_extension_pack.json",
    ];

    let mut all_pairs: Vec<TrainingPair> = Vec::new();
    let mut seen_surfaces: HashSet<String> = HashSet::new();

    let mut push_pair =
        |p: TrainingPair, all: &mut Vec<TrainingPair>, seen: &mut HashSet<String>| {
            if seen.insert(p.surface.clone()) {
                all.push(p);
            }
        };

    for path in &pack_paths {
        if !Path::new(path).exists() {
            eprintln!("       skip: {} (not present)", path);
            continue;
        }
        let bytes = fs::read(path).expect("read pack");
        let pack: CuratedPack = match serde_json::from_slice(&bytes) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("       skip: {} ({})", path, e);
                continue;
            }
        };
        let mut emitted_here = 0usize;
        for sample in &pack.samples {
            let sentence = sample.text();
            if sentence.is_empty() {
                continue;
            }
            let pairs = generator.pairs_from_text(sentence, RootPos::NounLike);
            for p in pairs {
                let before = all_pairs.len();
                push_pair(p, &mut all_pairs, &mut seen_surfaces);
                if all_pairs.len() > before {
                    emitted_here += 1;
                }
            }
            if all_pairs.len() >= max_pairs {
                break;
            }
        }
        eprintln!(
            "       pack {} → +{} unique words (total {} pairs)",
            path,
            emitted_here,
            all_pairs.len()
        );
        if all_pairs.len() >= max_pairs {
            break;
        }
    }
    eprintln!(
        "[2/4] Packs ingested: {} unique surface-pair samples",
        all_pairs.len()
    );

    // -- Stage 3: ingest a slice of huggingface_kz/kazakhBooks.csv ---------
    let csv_path = "data/external/huggingface_kz/kazakhBooks.csv";
    if Path::new(csv_path).exists() && all_pairs.len() < max_pairs {
        eprintln!(
            "[3/4] Reading first ≤{} kaz books × ≤{} chars/book from {}",
            max_books_csv, max_chars_per_book, csv_path
        );
        let file = fs::File::open(csv_path).expect("open csv");
        let reader = BufReader::new(file);
        let mut books_read = 0usize;
        let mut in_quoted_record = false;
        let mut current_record = String::new();
        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };
            // Naive multi-line CSV merge: a record may span many lines
            // because book bodies contain unescaped newlines inside the
            // quoted `text` cell. Track parity of unescaped `"` chars.
            current_record.push_str(&line);
            current_record.push('\n');
            let mut quotes = 0usize;
            let mut i = 0usize;
            let bytes = current_record.as_bytes();
            while i < bytes.len() {
                if bytes[i] == b'"' {
                    quotes += 1;
                }
                i += 1;
            }
            in_quoted_record = quotes % 2 != 0;
            if in_quoted_record {
                continue;
            }
            // We have a complete record. Skip the header.
            if current_record.starts_with("text,") {
                current_record.clear();
                continue;
            }
            // The record fields are: text, predicted_language, contains_kaz_symbols, id.
            // Find the LAST three commas to split out the three trailing fields.
            let record = current_record.trim_end_matches('\n').to_string();
            current_record.clear();

            let parts: Vec<&str> = record.rsplitn(4, ',').collect();
            if parts.len() < 4 {
                continue;
            }
            // parts is reversed: [id, contains_kaz, predicted_language, text]
            let predicted = parts[2].trim();
            if predicted != "kaz" {
                continue;
            }
            // Strip surrounding double quotes from text field.
            let text_field = parts[3]
                .trim()
                .trim_start_matches('"')
                .trim_end_matches('"');
            let snippet: String = text_field.chars().take(max_chars_per_book).collect();
            let pairs = generator.pairs_from_text(&snippet, RootPos::NounLike);
            for p in pairs {
                push_pair(p, &mut all_pairs, &mut seen_surfaces);
                if all_pairs.len() >= max_pairs {
                    break;
                }
            }
            books_read += 1;
            if books_read >= max_books_csv || all_pairs.len() >= max_pairs {
                break;
            }
        }
        eprintln!(
            "       CSV: {} kaz books read; total pairs now {}",
            books_read,
            all_pairs.len()
        );
    } else if !Path::new(csv_path).exists() {
        eprintln!("[3/4] CSV not present; skipping: {}", csv_path);
    }

    // -- Stage 4: write out -------------------------------------------------
    let out_bytes = serde_json::to_vec(&all_pairs).expect("serialise pairs");
    fs::write(&out_path, &out_bytes).expect("write out");
    eprintln!(
        "[4/4] Wrote {} unique-surface pairs to {} ({} bytes)",
        all_pairs.len(),
        out_path,
        out_bytes.len(),
    );
}
