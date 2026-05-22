// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Convert `data/external/wikibooks_kk/page_*.json` into a curated
//! corpus pack (`data/curated/wikibooks_kk_pack.json`) compatible with
//! the v1.3.0+ corpus-pack schema.
//!
//! ## Why kk.wikibooks
//!
//! `wikipedia_kz_pack.json` (150 k samples, encyclopaedic) is the
//! breadth corpus. `wikibooks_kk_pack.json` is the **depth /
//! curriculum** corpus: 434 pages of Abai literature, Java tutorial in
//! Kazakh, Kazakhstan Constitution texts, language textbooks — the
//! kinds of structured content that matter for tutor-positioning, not
//! arbitrary news / biography stubs.
//!
//! ## Schema (matches `wikipedia_kz_pack.json` so the downstream
//! `extract_facts` / `mine_lexicon_gaps` consume it without changes):
//!
//! ```json
//! {
//!   "version": "1.0.0",
//!   "name": "adam-wikibooks-kk-pack",
//!   "target_language": "kazakh",
//!   "script": "cyrillic",
//!   "source_license": "CC-BY-SA-3.0",
//!   "source_url": "https://kk.wikibooks.org",
//!   "attribution": "Kazakh Wikibooks contributors (CC-BY-SA-3.0)",
//!   "pages_processed": 434,
//!   "sample_count": <N>,
//!   "samples": [
//!     { "id": "wikibooks_kk_000001", "pack_name": "adam-wikibooks-kk-pack",
//!       "source_id": "wikibooks_kk", "domain": "wikibooks", "text": "…" },
//!     …
//!   ]
//! }
//! ```
//!
//! ## Filtering applied
//!
//! - Drop sentences < 4 alphabetic chars (UI noise, headers).
//! - Drop sentences > 200 words (paragraph runs without sentence
//!   boundaries — Wikibooks editors occasionally paste long blocks).
//! - Drop sentences whose Cyrillic fraction is < 60 % (filters out
//!   programming-code listings in the Java tutorial pages).
//! - Dedupe exact duplicates within the same page (Abai poems often
//!   repeat refrains).

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

const SRC_DIR: &str = "data/external/wikibooks_kk";
const OUT_PATH: &str = "data/curated/wikibooks_kk_pack.json";

#[derive(Deserialize)]
struct RawPage {
    title: String,
    extract: String,
}

#[derive(Serialize)]
struct Sample {
    id: String,
    pack_name: String,
    source_id: String,
    domain: String,
    text: String,
    /// Original page title — preserved so downstream consumers can
    /// follow attribution back to the source page when needed.
    source_page: String,
}

#[derive(Serialize)]
struct Pack {
    version: &'static str,
    name: &'static str,
    target_language: &'static str,
    script: &'static str,
    source_license: &'static str,
    source_url: &'static str,
    attribution: &'static str,
    pages_processed: usize,
    sample_count: usize,
    samples: Vec<Sample>,
}

fn main() {
    let dir = Path::new(SRC_DIR);
    let entries: Vec<_> = match fs::read_dir(dir) {
        Ok(it) => it.filter_map(|e| e.ok()).collect(),
        Err(e) => {
            eprintln!("process_kk_wikibooks: cannot read {SRC_DIR}: {e}");
            std::process::exit(1);
        }
    };
    let mut page_files: Vec<_> = entries
        .iter()
        .filter(|e| {
            let name = e.file_name();
            let s = name.to_string_lossy();
            s.starts_with("page_") && s.ends_with(".json")
        })
        .collect();
    page_files.sort_by_key(|e| e.file_name());
    let n_pages = page_files.len();
    eprintln!("[1/3] {n_pages} page files found in {SRC_DIR}");

    let mut samples: Vec<Sample> = Vec::new();
    let mut next_id = 1usize;
    let mut dropped_short = 0usize;
    let mut dropped_long = 0usize;
    let mut dropped_non_kk = 0usize;
    let mut dropped_dup = 0usize;
    for entry in &page_files {
        let bytes = match fs::read(entry.path()) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let page: RawPage = match serde_json::from_slice(&bytes) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let mut seen: BTreeSet<String> = BTreeSet::new();
        for sentence in split_sentences(&page.extract) {
            let trimmed = sentence.trim();
            if trimmed.is_empty() {
                continue;
            }
            let alpha = trimmed.chars().filter(|c| c.is_alphabetic()).count();
            if alpha < 4 {
                dropped_short += 1;
                continue;
            }
            let words = trimmed.split_whitespace().count();
            if words > 200 {
                dropped_long += 1;
                continue;
            }
            if !is_mostly_kazakh(trimmed) {
                dropped_non_kk += 1;
                continue;
            }
            if !seen.insert(trimmed.to_string()) {
                dropped_dup += 1;
                continue;
            }
            samples.push(Sample {
                id: format!("wikibooks_kk_{:06}", next_id),
                pack_name: "adam-wikibooks-kk-pack".into(),
                source_id: "wikibooks_kk".into(),
                domain: "wikibooks".into(),
                text: trimmed.to_string(),
                source_page: page.title.clone(),
            });
            next_id += 1;
        }
    }

    eprintln!(
        "[2/3] sentences kept: {} (dropped: short={}, long={}, non-kk={}, dup={})",
        samples.len(),
        dropped_short,
        dropped_long,
        dropped_non_kk,
        dropped_dup
    );

    let pack = Pack {
        version: "1.0.0",
        name: "adam-wikibooks-kk-pack",
        target_language: "kazakh",
        script: "cyrillic",
        source_license: "CC-BY-SA-3.0",
        source_url: "https://kk.wikibooks.org",
        attribution: "Kazakh Wikibooks contributors (CC-BY-SA-3.0)",
        pages_processed: n_pages,
        sample_count: samples.len(),
        samples,
    };

    let json = serde_json::to_vec_pretty(&pack).expect("serialise");
    fs::create_dir_all(Path::new(OUT_PATH).parent().unwrap()).expect("mkdir");
    fs::write(OUT_PATH, &json).expect("write pack");
    eprintln!(
        "[3/3] wrote {OUT_PATH} — {} samples, {} KB",
        pack.sample_count,
        json.len() / 1024
    );
}

/// Naive sentence splitter on `.`, `?`, `!`, `…`. Handles abbreviations
/// crudely — Wikibooks Kazakh text has very few of them, so the simple
/// rule is good enough for v6.0.0-rc3 corpus expansion. A smarter
/// splitter is on the v7.x roadmap.
fn split_sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    for ch in text.chars() {
        buf.push(ch);
        if matches!(ch, '.' | '?' | '!' | '…' | '\n') {
            let s = buf.trim().to_string();
            if !s.is_empty() {
                out.push(s);
            }
            buf.clear();
        }
    }
    let tail = buf.trim().to_string();
    if !tail.is_empty() {
        out.push(tail);
    }
    out
}

/// Return true iff at least 60 % of the alphabetic characters are
/// Cyrillic. Filters out programming-language code listings in the
/// Java tutorial pages without dropping legitimate Kazakh prose that
/// happens to mention a Latin term.
fn is_mostly_kazakh(s: &str) -> bool {
    let mut total = 0usize;
    let mut cyrillic = 0usize;
    for c in s.chars() {
        if c.is_alphabetic() {
            total += 1;
            if matches!(c, '\u{0400}'..='\u{04FF}') {
                cyrillic += 1;
            }
        }
    }
    if total < 5 {
        return false;
    }
    cyrillic * 100 / total >= 60
}
