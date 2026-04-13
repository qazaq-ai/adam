use std::{
    collections::BTreeMap,
    env, fs,
    io::{self, Read},
    process::ExitCode,
};

use adam_kernel::{
    SegmentationLexicon, SegmentationRuleSet, deterministic_segment_token, normalize_text,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct WordFrequency {
    word: String,
    count: usize,
}

#[derive(Debug, Serialize)]
struct CoverageReport {
    lexicon_root_count: usize,
    total_word_count: usize,
    unique_word_count: usize,
    covered_word_count: usize,
    not_covered_word_count: usize,
    coverage_rate_bps: usize,
    top_unknown_words: Vec<WordFrequency>,
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(lexicon_path) = args.next() else {
        eprintln!("usage: coverage_report <lexicon.json> <rules.json> [text_file]");
        return ExitCode::FAILURE;
    };
    let Some(rules_path) = args.next() else {
        eprintln!("usage: coverage_report <lexicon.json> <rules.json> [text_file]");
        return ExitCode::FAILURE;
    };
    let text_path = args.next();

    let lexicon: SegmentationLexicon = match read_json(&lexicon_path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("failed to read lexicon: {e}");
            return ExitCode::FAILURE;
        }
    };
    let rules: SegmentationRuleSet = match read_json(&rules_path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("failed to read rules: {e}");
            return ExitCode::FAILURE;
        }
    };

    if let Err(e) = lexicon.validate() {
        eprintln!("invalid lexicon: {e}");
        return ExitCode::FAILURE;
    }
    if let Err(e) = rules.validate() {
        eprintln!("invalid rules: {e}");
        return ExitCode::FAILURE;
    }

    let raw_text = match text_path {
        Some(path) => match fs::read_to_string(&path) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("failed to read text file: {e}");
                return ExitCode::FAILURE;
            }
        },
        None => {
            let mut buf = String::new();
            if let Err(e) = io::stdin().read_to_string(&mut buf) {
                eprintln!("failed to read stdin: {e}");
                return ExitCode::FAILURE;
            }
            buf
        }
    };

    let words: Vec<String> = raw_text
        .split_whitespace()
        .map(normalize_text)
        .filter(|w| !w.is_empty())
        .collect();

    let total_word_count = words.len();
    if total_word_count == 0 {
        eprintln!("no words found in input");
        return ExitCode::FAILURE;
    }

    let mut covered_word_count = 0;
    let mut unknown_freq: BTreeMap<String, usize> = BTreeMap::new();

    for word in &words {
        if deterministic_segment_token(word, &lexicon, &rules).is_some() {
            covered_word_count += 1;
        } else {
            *unknown_freq.entry(word.clone()).or_default() += 1;
        }
    }

    let unique_word_count = {
        let mut all_words = words.clone();
        all_words.sort_unstable();
        all_words.dedup();
        all_words.len()
    };

    let not_covered_word_count = total_word_count - covered_word_count;
    let coverage_rate_bps = covered_word_count * 10_000 / total_word_count;

    let mut top_unknown_words: Vec<WordFrequency> = unknown_freq
        .into_iter()
        .map(|(word, count)| WordFrequency { word, count })
        .collect();
    top_unknown_words.sort_by(|a, b| b.count.cmp(&a.count).then(a.word.cmp(&b.word)));
    top_unknown_words.truncate(20);

    let report = CoverageReport {
        lexicon_root_count: lexicon.roots.len(),
        total_word_count,
        unique_word_count,
        covered_word_count,
        not_covered_word_count,
        coverage_rate_bps,
        top_unknown_words,
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("report serializes")
    );
    ExitCode::SUCCESS
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let value = serde_json::from_str(&contents)?;
    Ok(value)
}
