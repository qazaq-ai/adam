//! Morpheme-coverage audit (v1.5.5).
//!
//! Measures what fraction of words in each committed pack begin with a
//! known Lexicon root. This is an upper-bound proxy for true FST parse
//! coverage — fast (O(corpus × avg_word_len)) where a full FST parse of
//! every word would be O(corpus × lexicon × feature_space), i.e. billions
//! of synth calls for a multi-million-word corpus.
//!
//! Per-pack report includes:
//!   - total words, unique words
//!   - covered words (at least one prefix ∈ lexicon)
//!   - coverage ratio
//!   - top 20 most-frequent uncovered words (future Lexicon candidates)
//!
//! Output: `data/corpus_morpheme_coverage_report.json`.
//!
//! This is the first rung of the v1.6.0+ retrieval-engine ladder: once we
//! know where the Lexicon misses, we know where to expand, and we can
//! measure every Lexicon PR against a concrete coverage delta.

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use serde::{Deserialize, Serialize};

/// Minimum root length for prefix matching. Shorter roots like `мен`
/// (1sg pronoun) produce too many false positives against forms like
/// `мектеп`, so we only treat roots of ≥ 3 chars as coverage evidence.
const MIN_ROOT_LEN: usize = 3;

/// Top-K uncovered words per pack to include in the report.
const TOP_UNCOVERED: usize = 20;

const CURATED_DIR: &str = "data/curated";
const REPORT_PATH: &str = "data/corpus_morpheme_coverage_report.json";

/// Same canonical pack list as `corpus_audit`.
const SOURCE_PACKS: &[&str] = &[
    "tatoeba_kazakh_pack.json",
    "wikipedia_kz_pack.json",
    "common_voice_kk_pack.json",
    "cc100_kk_pack.json",
    "abai_wikisource_pack.json",
    "kazakh_proverbs_pack.json",
    "synthetic_sentences_pack.json",
    "kazakh_classics_pack.json",
    // v4.7.1 — Rust Book Kazakh translation pack.
    "rust_book_kk_pack.json",
];

const LEXICON_FILES: &[&str] = &[
    "data/tokenizer/segmentation_roots.json",
    "data/lexicon_v1/apertium_imported_roots.json",
];

#[derive(Debug, Deserialize)]
struct RootsFile {
    roots: Vec<RootEntryLite>,
}

#[derive(Debug, Deserialize)]
struct RootEntryLite {
    root: String,
}

#[derive(Debug, Deserialize)]
struct PackFile {
    samples: Vec<Sample>,
}

#[derive(Debug, Deserialize)]
struct Sample {
    text: String,
}

#[derive(Debug, Serialize)]
struct PackCoverage {
    pack: String,
    total_words: usize,
    unique_words: usize,
    covered_words: usize,
    coverage_ratio: f32,
    uncovered_unique: usize,
    top_uncovered: Vec<TopUncovered>,
}

#[derive(Debug, Serialize)]
struct TopUncovered {
    word: String,
    count: usize,
}

#[derive(Debug, Serialize)]
struct Report {
    version: String,
    lexicon_roots: usize,
    min_root_len: usize,
    total_words: usize,
    covered_words: usize,
    overall_coverage: f32,
    packs: Vec<PackCoverage>,
}

fn main() -> ExitCode {
    let roots = match load_roots() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("cannot load lexicon roots: {e}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!("loaded {} roots (≥ {MIN_ROOT_LEN} chars)", roots.len());

    let mut packs = Vec::new();
    let mut total_words = 0usize;
    let mut covered_total = 0usize;

    for pack_name in SOURCE_PACKS {
        let path = Path::new(CURATED_DIR).join(pack_name);
        if !path.exists() {
            eprintln!("skipping missing: {}", path.display());
            continue;
        }
        match audit_pack(&path, &roots) {
            Ok(c) => {
                total_words += c.total_words;
                covered_total += c.covered_words;
                packs.push(c);
            }
            Err(e) => {
                eprintln!("error on {}: {e}", path.display());
                return ExitCode::FAILURE;
            }
        }
    }

    let overall = if total_words > 0 {
        covered_total as f32 / total_words as f32
    } else {
        0.0
    };

    let report = Report {
        version: env!("CARGO_PKG_VERSION").to_string(),
        lexicon_roots: roots.len(),
        min_root_len: MIN_ROOT_LEN,
        total_words,
        covered_words: covered_total,
        overall_coverage: overall,
        packs,
    };

    let json = serde_json::to_string_pretty(&report).expect("serialise");
    if let Err(e) = fs::write(REPORT_PATH, &json) {
        eprintln!("cannot write {REPORT_PATH}: {e}");
        return ExitCode::FAILURE;
    }

    println!("morpheme coverage written to {REPORT_PATH}");
    println!(
        "  lexicon: {} roots (≥ {} chars)",
        report.lexicon_roots, report.min_root_len
    );
    println!("  corpus: {} words total", report.total_words);
    println!(
        "  prefix-match coverage: {:.2}%",
        report.overall_coverage * 100.0
    );
    ExitCode::SUCCESS
}

fn load_roots() -> Result<HashSet<String>, String> {
    let mut set = HashSet::new();
    for path in LEXICON_FILES {
        let raw = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        let file: RootsFile = serde_json::from_str(&raw).map_err(|e| format!("{path}: {e}"))?;
        for entry in file.roots {
            let r = entry.root.trim().to_lowercase();
            if r.chars().count() >= MIN_ROOT_LEN {
                set.insert(r);
            }
        }
    }
    Ok(set)
}

fn audit_pack(path: &PathBuf, roots: &HashSet<String>) -> Result<PackCoverage, String> {
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let pack: PackFile = serde_json::from_str(&raw).map_err(|e| e.to_string())?;

    let mut word_counts: HashMap<String, usize> = HashMap::new();
    for sample in &pack.samples {
        for word in sample.text.split_whitespace() {
            let cleaned = normalise(word);
            if cleaned.chars().count() < MIN_ROOT_LEN {
                continue;
            }
            *word_counts.entry(cleaned).or_insert(0) += 1;
        }
    }

    let mut total_words = 0usize;
    let mut covered_words = 0usize;
    let mut uncovered_counts: HashMap<String, usize> = HashMap::new();

    for (word, count) in &word_counts {
        total_words += count;
        if has_known_prefix(word, roots) {
            covered_words += count;
        } else {
            *uncovered_counts.entry(word.clone()).or_insert(0) += count;
        }
    }

    let mut top: Vec<(String, usize)> = uncovered_counts.into_iter().collect();
    top.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    top.truncate(TOP_UNCOVERED);
    let top_uncovered = top
        .into_iter()
        .map(|(word, count)| TopUncovered { word, count })
        .collect();

    let coverage_ratio = if total_words > 0 {
        covered_words as f32 / total_words as f32
    } else {
        0.0
    };

    Ok(PackCoverage {
        pack: path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string(),
        total_words,
        unique_words: word_counts.len(),
        covered_words,
        coverage_ratio,
        uncovered_unique: total_words - covered_words,
        top_uncovered,
    })
}

fn normalise(word: &str) -> String {
    word.chars()
        .filter(|c| c.is_alphabetic() || *c == '-')
        .collect::<String>()
        .to_lowercase()
}

/// Returns true if any prefix of `word` (length ≥ MIN_ROOT_LEN, up to the
/// whole word) matches a known root. The char-boundary walk matters for
/// Cyrillic — byte-slicing mid-codepoint would panic.
fn has_known_prefix(word: &str, roots: &HashSet<String>) -> bool {
    let mut prefix = String::new();
    for (i, ch) in word.chars().enumerate() {
        prefix.push(ch);
        if i + 1 >= MIN_ROOT_LEN && roots.contains(&prefix) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_matches_full_root() {
        let mut roots = HashSet::new();
        roots.insert("бала".to_string());
        assert!(has_known_prefix("балалар", &roots));
    }

    #[test]
    fn prefix_matches_exact() {
        let mut roots = HashSet::new();
        roots.insert("мектеп".to_string());
        assert!(has_known_prefix("мектеп", &roots));
    }

    #[test]
    fn short_prefix_ignored() {
        let mut roots = HashSet::new();
        roots.insert("ба".to_string()); // too short to be a root anyway
        assert!(!has_known_prefix("балалар", &roots));
    }

    #[test]
    fn no_match_on_unrelated() {
        let mut roots = HashSet::new();
        roots.insert("мектеп".to_string());
        assert!(!has_known_prefix("бала", &roots));
    }

    #[test]
    fn normalise_strips_punct_and_lowercases() {
        assert_eq!(normalise("Балалар,"), "балалар");
    }
}
