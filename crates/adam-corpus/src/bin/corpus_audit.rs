//! Corpus audit (v1.1.5) — measure the starting position for the
//! v1.x expansion from ~4 M to 100 M+ Kazakh tokens.
//!
//! For every source pack in `data/curated/*_pack.json` the audit
//! reports:
//!   - sample count
//!   - total word count (whitespace-split)
//!   - unique word count
//!   - Kazakh-purity ratio (fraction of words free of Russian-only
//!     letters AND not ending in a loanword suffix)
//!   - within-pack duplicate count
//!
//! Across all packs:
//!   - total words (the "starting position" for v1.x expansion)
//!   - unique words (vocabulary)
//!   - overall purity score
//!
//! Output: `data/corpus_audit_report.json`.
//!
//! Usage:  cargo run --release -p adam-corpus --bin corpus_audit

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use serde::{Deserialize, Serialize};

/// Loanword-signal letters — native pre-modern Kazakh never uses these.
const RUSSIAN_ONLY: &[char] = &['ё', 'ф', 'ц', 'ч', 'щ', 'ъ', 'ь', 'э'];

/// Loanword suffix patterns (shared with `filter_pack.rs` / v0.5.0 era).
const LOANWORD_SUFFIXES: &[&str] = &[
    "ция",
    "логия",
    "графия",
    "тика",
    "изм",
    "ивный",
    "ильный",
    "альный",
    "альная",
    "альное",
    "ональный",
];

/// Packs to audit. Any JSON in `data/curated/` whose name ends in
/// `_pack.json` and matches one of these names. Kept explicit to
/// avoid accidentally auditing intermediate / assembled packs that
/// would double-count samples.
const SOURCE_PACKS: &[&str] = &[
    "tatoeba_kazakh_pack.json",
    "wikipedia_kz_pack.json",
    "common_voice_kk_pack.json",
    "cc100_kk_pack.json",
    "abai_wikisource_pack.json",
    "kazakh_proverbs_pack.json",
    "synthetic_sentences_pack.json",
];

const CURATED_DIR: &str = "data/curated";
const REPORT_PATH: &str = "data/corpus_audit_report.json";

#[derive(Debug, Deserialize)]
struct PackFile {
    #[allow(dead_code)]
    name: String,
    samples: Vec<Sample>,
}

#[derive(Debug, Deserialize)]
struct Sample {
    text: String,
}

#[derive(Debug, Serialize)]
struct PackAudit {
    pack: String,
    sample_count: usize,
    total_words: usize,
    unique_words: usize,
    duplicate_samples: usize,
    loanword_words: usize,
    russian_only_letter_words: usize,
    loanword_suffix_words: usize,
    purity_score: f32,
    avg_sentence_length: f32,
}

#[derive(Debug, Serialize)]
struct CorpusAuditReport {
    version: String,
    starting_position_words: usize,
    target_words: usize,
    expansion_factor: f32,
    unique_words_total: usize,
    overall_purity_score: f32,
    packs: Vec<PackAudit>,
}

fn main() -> ExitCode {
    let mut packs = Vec::new();
    let mut corpus_vocab: HashSet<String> = HashSet::new();
    let mut total_words = 0usize;
    let mut total_loanword_words = 0usize;

    for pack_name in SOURCE_PACKS {
        let path = Path::new(CURATED_DIR).join(pack_name);
        if !path.exists() {
            eprintln!("skipping missing: {}", path.display());
            continue;
        }
        let audit = match audit_pack(&path) {
            Ok(a) => a,
            Err(e) => {
                eprintln!("error auditing {}: {e}", path.display());
                return ExitCode::FAILURE;
            }
        };
        total_words += audit.total_words;
        total_loanword_words += audit.loanword_words;
        let raw: PackFile = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        for sample in &raw.samples {
            for word in sample.text.split_whitespace() {
                corpus_vocab.insert(normalise_word(word));
            }
        }
        packs.push(audit);
    }

    let overall_purity = if total_words > 0 {
        1.0 - (total_loanword_words as f32 / total_words as f32)
    } else {
        0.0
    };

    let target = 100_000_000usize;
    let report = CorpusAuditReport {
        version: "1.1.5".into(),
        starting_position_words: total_words,
        target_words: target,
        expansion_factor: target as f32 / total_words.max(1) as f32,
        unique_words_total: corpus_vocab.len(),
        overall_purity_score: overall_purity,
        packs,
    };

    let json = serde_json::to_string_pretty(&report).expect("serialise report");
    if let Err(e) = fs::write(REPORT_PATH, &json) {
        eprintln!("cannot write {REPORT_PATH}: {e}");
        return ExitCode::FAILURE;
    }

    println!("corpus audit written to {REPORT_PATH}");
    println!(
        "  starting position: {} words",
        report.starting_position_words
    );
    println!("  unique vocabulary: {} words", report.unique_words_total);
    println!(
        "  overall Kazakh purity: {:.2}%",
        report.overall_purity_score * 100.0
    );
    println!(
        "  target: {} words ({:.1}× expansion needed)",
        report.target_words, report.expansion_factor
    );
    ExitCode::SUCCESS
}

fn audit_pack(path: &PathBuf) -> Result<PackAudit, String> {
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let pack: PackFile = serde_json::from_str(&raw).map_err(|e| e.to_string())?;

    let mut vocab: HashSet<String> = HashSet::new();
    let mut seen_samples: HashSet<String> = HashSet::new();
    let mut total_words = 0usize;
    let mut russian_only_words = 0usize;
    let mut loanword_suffix_words = 0usize;
    let mut duplicate_samples = 0usize;
    let mut total_sentence_length = 0usize;

    for sample in &pack.samples {
        let normalised = sample.text.trim().to_lowercase();
        if !seen_samples.insert(normalised) {
            duplicate_samples += 1;
        }
        let words: Vec<&str> = sample.text.split_whitespace().collect();
        total_sentence_length += words.len();
        for word in words {
            let lower = normalise_word(word);
            if lower.is_empty() {
                continue;
            }
            total_words += 1;
            vocab.insert(lower.clone());
            if has_russian_only_letter(&lower) {
                russian_only_words += 1;
            } else if ends_with_loanword_suffix(&lower) {
                loanword_suffix_words += 1;
            }
        }
    }

    let loanword_words = russian_only_words + loanword_suffix_words;
    let purity_score = if total_words > 0 {
        1.0 - (loanword_words as f32 / total_words as f32)
    } else {
        0.0
    };
    let avg_sentence_length = if !pack.samples.is_empty() {
        total_sentence_length as f32 / pack.samples.len() as f32
    } else {
        0.0
    };

    Ok(PackAudit {
        pack: path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string(),
        sample_count: pack.samples.len(),
        total_words,
        unique_words: vocab.len(),
        duplicate_samples,
        loanword_words,
        russian_only_letter_words: russian_only_words,
        loanword_suffix_words,
        purity_score,
        avg_sentence_length,
    })
}

fn normalise_word(word: &str) -> String {
    word.chars()
        .filter(|c| c.is_alphabetic() || *c == '-')
        .collect::<String>()
        .to_lowercase()
}

fn has_russian_only_letter(word: &str) -> bool {
    word.chars().any(|c| RUSSIAN_ONLY.contains(&c))
}

fn ends_with_loanword_suffix(word: &str) -> bool {
    LOANWORD_SUFFIXES
        .iter()
        .any(|s| word.ends_with(s) && word.chars().count() > s.chars().count() + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn russian_only_letter_detection() {
        assert!(has_russian_only_letter("цифра"));
        assert!(has_russian_only_letter("частный"));
        assert!(!has_russian_only_letter("сәлем"));
        assert!(!has_russian_only_letter("қазақ"));
    }

    #[test]
    fn loanword_suffix_detection() {
        // -ция and -изм are both in the suffix list and should catch
        // these canonical Russified loanwords.
        assert!(ends_with_loanword_suffix("конституция"));
        assert!(ends_with_loanword_suffix("коммунизм"));
        assert!(ends_with_loanword_suffix("биология")); // -логия
        assert!(!ends_with_loanword_suffix("сөз"));
        assert!(!ends_with_loanword_suffix("ция")); // too short
    }

    #[test]
    fn normalise_strips_punctuation() {
        assert_eq!(normalise_word("сәлем,"), "сәлем");
        assert_eq!(normalise_word("Мектеп."), "мектеп");
    }
}
