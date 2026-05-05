//! `process_rust_book_kk` — build the Rust Book Kazakh translation
//! corpus pack from `data/raw/rust_book_kk/chapter_*.md`.
//!
//! For each `chapter_NN.md` file under `data/raw/rust_book_kk/`,
//! this binary:
//!
//! 1. Strips fenced code blocks (```rust / ```sh / ```toml / etc.).
//! 2. Strips heading hashes (`#`, `##`, …) and bullet markers
//!    (`- `, `* `, `+ `, `1. `).
//! 3. Splits the residual prose into sentences on `. ! ?` followed
//!    by whitespace and a Cyrillic uppercase letter — preserving
//!    backtick-quoted technical spans (so the dot inside
//!    `Cargo.toml` is not a sentence boundary).
//! 4. Emits sentence-level samples in the standard adam corpus-pack
//!    format to `data/curated/rust_book_kk_pack.json`.
//!
//! Each sample carries:
//!
//!     {
//!       "id": "rust_book_NN_MMMM",
//!       "pack_name": "adam-rust-book-kk-pack",
//!       "source_id": "chapter_NN",
//!       "domain": "programming_rust",
//!       "text": "<single Kazakh sentence>"
//!     }
//!
//! After running this, register the pack in `SOURCE_PACKS` of
//! `crates/adam-retrieval/src/bin/build_morpheme_index.rs` and
//! rebuild the morpheme index. The chapter cadence is
//! «глава = патч» on the v4.7.x minor (per user-confirmed 2026-04-29).
//!
//! Usage:
//!   cargo run --release -p adam-corpus --bin process_rust_book_kk

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde::Serialize;

const RAW_DIR: &str = "data/raw/rust_book_kk";
const OUT_PATH: &str = "data/curated/rust_book_kk_pack.json";
const PACK_NAME: &str = "adam-rust-book-kk-pack";
const DOMAIN: &str = "programming_rust";
const PACK_VERSION: &str = "1.0.0";
const SOURCE_LICENSE: &str = "MIT/Apache-2.0";
const SOURCE_URL: &str = "https://doc.rust-lang.org/book/";
const ATTRIBUTION: &str = "Translated from The Rust Programming Language by Steve Klabnik, Carol Nichols, and the Rust community. Translation maintained by adam project.";

#[derive(Serialize)]
struct Pack {
    version: String,
    name: String,
    target_language: String,
    script: String,
    source_license: String,
    source_url: String,
    attribution: String,
    chapters_translated: usize,
    sentences_total: usize,
    sample_count: usize,
    samples: Vec<Sample>,
}

#[derive(Serialize)]
struct Sample {
    id: String,
    pack_name: String,
    source_id: String,
    domain: String,
    text: String,
}

fn main() -> ExitCode {
    let raw_dir = Path::new(RAW_DIR);
    if !raw_dir.exists() {
        eprintln!("raw dir missing: {}", raw_dir.display());
        return ExitCode::FAILURE;
    }

    let mut chapters: Vec<PathBuf> = match fs::read_dir(raw_dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.file_name().is_some_and(|n| {
                    let s = n.to_string_lossy();
                    s.starts_with("chapter_") && s.ends_with(".md")
                })
            })
            .collect(),
        Err(e) => {
            eprintln!("cannot read {}: {e}", raw_dir.display());
            return ExitCode::FAILURE;
        }
    };
    chapters.sort_by_key(|p| chapter_num(p));

    if chapters.is_empty() {
        eprintln!("no chapter_*.md files in {}", raw_dir.display());
        return ExitCode::FAILURE;
    }

    let mut samples = Vec::new();
    let mut total_sentences = 0usize;
    for path in &chapters {
        let cn = chapter_num(path);
        let raw = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("cannot read {}: {e}", path.display());
                return ExitCode::FAILURE;
            }
        };
        let prose = extract_prose(&raw);
        let sentences = split_sentences(&prose);
        for (i, sentence) in sentences.iter().enumerate() {
            samples.push(Sample {
                id: format!("rust_book_{cn:02}_{:04}", i + 1),
                pack_name: PACK_NAME.into(),
                source_id: format!("chapter_{cn:02}"),
                domain: DOMAIN.into(),
                text: sentence.clone(),
            });
        }
        total_sentences += sentences.len();
        println!("  chapter_{cn:02}: {} sentences", sentences.len());
    }

    let pack = Pack {
        version: PACK_VERSION.into(),
        name: PACK_NAME.into(),
        target_language: "kk".into(),
        script: "Cyrillic".into(),
        source_license: SOURCE_LICENSE.into(),
        source_url: SOURCE_URL.into(),
        attribution: ATTRIBUTION.into(),
        chapters_translated: chapters.len(),
        sentences_total: total_sentences,
        sample_count: samples.len(),
        samples,
    };

    let json = match serde_json::to_string_pretty(&pack) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("serialize failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    let out_path = Path::new(OUT_PATH);
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).ok();
    }
    if let Err(e) = fs::write(out_path, format!("{json}\n")) {
        eprintln!("write failed: {e}");
        return ExitCode::FAILURE;
    }
    println!(
        "wrote {} ({} chapters / {} sentences / {} samples)",
        out_path.display(),
        chapters.len(),
        total_sentences,
        pack.sample_count,
    );
    ExitCode::SUCCESS
}

fn chapter_num(path: &Path) -> usize {
    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    name.strip_prefix("chapter_")
        .and_then(|n| n.parse::<usize>().ok())
        .unwrap_or(0)
}

/// Strip fenced code blocks and markdown decoration (heading hashes,
/// list markers) so only Kazakh prose remains.
fn extract_prose(md: &str) -> String {
    let mut out = String::new();
    let mut in_fence = false;
    for line in md.split('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let mut cleaned = trimmed.to_string();
        // Strip leading hashes (any heading level).
        cleaned = cleaned.trim_start_matches('#').trim_start().to_string();
        // Strip bullet markers.
        if let Some(rest) = cleaned.strip_prefix("- ") {
            cleaned = rest.to_string();
        } else if let Some(rest) = cleaned.strip_prefix("* ") {
            cleaned = rest.to_string();
        } else if let Some(rest) = cleaned.strip_prefix("+ ") {
            cleaned = rest.to_string();
        } else if let Some(rest) = strip_numbered_list(&cleaned) {
            cleaned = rest;
        }
        out.push_str(&cleaned);
        out.push('\n');
    }
    out
}

fn strip_numbered_list(s: &str) -> Option<String> {
    // Match `<digits>. ` prefix (e.g. `1. `, `12. `).
    let mut chars = s.chars();
    let mut buf = String::new();
    while let Some(ch) = chars.next() {
        if ch.is_ascii_digit() {
            buf.push(ch);
        } else if ch == '.' && !buf.is_empty() {
            // Need a following space.
            if chars.next() == Some(' ') {
                return Some(chars.collect());
            }
            return None;
        } else {
            return None;
        }
    }
    None
}

/// Split text into sentences. Boundary: `. ! ?` followed by
/// whitespace and a Cyrillic uppercase letter. Preserves backtick-
/// quoted technical spans (the dot inside `Cargo.toml` is not a
/// boundary).
fn split_sentences(text: &str) -> Vec<String> {
    // Replace newlines with spaces and collapse whitespace.
    let normalised: String = text
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();
    let chars: Vec<char> = normalised.chars().collect();
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut in_code = false;
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '`' {
            in_code = !in_code;
            buf.push(ch);
            i += 1;
            continue;
        }
        buf.push(ch);
        if !in_code && (ch == '.' || ch == '!' || ch == '?') {
            // Look ahead: whitespace + Cyrillic uppercase, or end-of-input.
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            let boundary = j >= chars.len()
                || chars
                    .get(j)
                    .map(|c| is_cyrillic_uppercase(*c))
                    .unwrap_or(false);
            if boundary {
                let sentence = buf.trim().to_string();
                if accept_sentence(&sentence) {
                    out.push(collapse_whitespace(&sentence));
                }
                buf.clear();
                i = j;
                continue;
            }
        }
        i += 1;
    }
    let tail = buf.trim().to_string();
    if accept_sentence(&tail) {
        out.push(collapse_whitespace(&tail));
    }
    out
}

fn is_cyrillic_uppercase(ch: char) -> bool {
    // Russian + Kazakh-specific uppercase letters.
    matches!(ch, 'А'..='Я' | 'Ё')
        || matches!(ch, 'Ә' | 'Ғ' | 'Қ' | 'Ң' | 'Ө' | 'Ұ' | 'Ү' | 'Һ' | 'І')
}

fn accept_sentence(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let word_count = s.split_whitespace().count();
    if word_count < 3 {
        return false;
    }
    // Skip pure-punctuation noise.
    if s.chars().all(|c| !c.is_alphabetic()) {
        return false;
    }
    true
}

fn collapse_whitespace(s: &str) -> String {
    let mut out = String::new();
    let mut prev_space = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                out.push(' ');
            }
            prev_space = true;
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_fenced_code_blocks() {
        let md = "Бірінші жол.\n```rust\nfn main() {}\n```\nЕкінші жол.";
        let prose = extract_prose(md);
        assert!(prose.contains("Бірінші"));
        assert!(prose.contains("Екінші"));
        assert!(!prose.contains("fn main"));
    }

    #[test]
    fn splits_sentences_on_capital_kazakh_uppercase() {
        let text = "Бірінші сөйлем мынадай. Екінші сөйлем — бұл.";
        let sentences = split_sentences(text);
        assert_eq!(sentences.len(), 2);
        assert!(sentences[0].starts_with("Бірінші"));
        assert!(sentences[1].starts_with("Екінші"));
    }

    #[test]
    fn preserves_backtick_spans_at_sentence_boundary() {
        // Two ≥3-word sentences (the accept_sentence threshold);
        // the period inside `Cargo.toml` must not split.
        let text = "Файлды `Cargo.toml` деп атайды осы жобада. Бұл өте маңызды файл.";
        let sentences = split_sentences(text);
        assert_eq!(sentences.len(), 2, "got: {sentences:?}");
        assert!(sentences[0].contains("`Cargo.toml`"));
        assert!(sentences[1].starts_with("Бұл"));
    }

    #[test]
    fn rejects_short_fragments() {
        assert!(!accept_sentence("ок."));
        assert!(!accept_sentence("Қысқа сөйлем")); // 2 words
        assert!(accept_sentence("Бұл жеткілікті ұзын сөйлем."));
    }
}
