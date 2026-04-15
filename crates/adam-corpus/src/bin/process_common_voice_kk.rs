use std::{
    collections::HashSet,
    env,
    fs::File,
    io::{BufRead, BufReader},
    process::ExitCode,
};

use serde::Serialize;

const DEFAULT_INPUT: &str = "data/external/common_voice_kk_sentences.txt";
const DEFAULT_OUTPUT: &str = "data/curated/common_voice_kk_pack.json";

#[derive(Debug, Serialize)]
struct Sample {
    id: String,
    pack_name: String,
    source_id: String,
    domain: String,
    text: String,
}

#[derive(Debug, Serialize)]
struct Pack {
    version: String,
    name: String,
    target_language: String,
    script: String,
    source_license: String,
    source_url: String,
    attribution: String,
    lines_scanned: usize,
    sample_count: usize,
    skipped_latin: usize,
    skipped_length: usize,
    skipped_duplicate: usize,
    samples: Vec<Sample>,
}

fn main() -> ExitCode {
    let input_path = env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_INPUT.to_string());
    let output_path = env::args()
        .nth(2)
        .unwrap_or_else(|| DEFAULT_OUTPUT.to_string());

    let file = match File::open(&input_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("cannot open {input_path}: {e}");
            eprintln!(
                "hint: curl -sL \
                 https://raw.githubusercontent.com/common-voice/common-voice/main/server/data/kk/sentence-collector.txt \
                 -o {input_path}"
            );
            return ExitCode::FAILURE;
        }
    };
    let reader = BufReader::new(file);

    let mut samples: Vec<Sample> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut lines_scanned = 0usize;
    let mut skipped_latin = 0usize;
    let mut skipped_length = 0usize;
    let mut skipped_duplicate = 0usize;

    for line in reader.lines() {
        let Ok(line) = line else {
            continue;
        };
        lines_scanned += 1;
        let trimmed = line.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }
        let Some(sample_text) = accept(
            trimmed,
            &mut seen,
            &mut skipped_latin,
            &mut skipped_length,
            &mut skipped_duplicate,
        ) else {
            continue;
        };
        let idx = samples.len() + 1;
        samples.push(Sample {
            id: format!("cv_kk_{idx:05}"),
            pack_name: "adam-common-voice-kk-pack".to_string(),
            source_id: format!("common_voice_kk_line_{lines_scanned}"),
            domain: "common_voice".to_string(),
            text: sample_text,
        });
    }

    let sample_count = samples.len();
    let pack = Pack {
        version: env!("CARGO_PKG_VERSION").to_string(),
        name: "adam-common-voice-kk-pack".to_string(),
        target_language: "kazakh".to_string(),
        script: "cyrillic".to_string(),
        source_license: "CC0-1.0".to_string(),
        source_url: "https://github.com/common-voice/common-voice/blob/main/server/data/kk/sentence-collector.txt".to_string(),
        attribution: "Mozilla Common Voice contributors, Kazakh sentence collector (CC0-1.0)".to_string(),
        lines_scanned,
        sample_count,
        skipped_latin,
        skipped_length,
        skipped_duplicate,
        samples,
    };

    if let Some(parent) = std::path::Path::new(&output_path).parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            let _ = std::fs::create_dir_all(parent);
        }
    }
    if let Err(e) = std::fs::write(
        &output_path,
        serde_json::to_string_pretty(&pack).expect("serialize"),
    ) {
        eprintln!("cannot write {output_path}: {e}");
        return ExitCode::FAILURE;
    }

    eprintln!(
        "lines={} accepted={} skipped_latin={} skipped_length={} skipped_dup={}",
        lines_scanned, sample_count, skipped_latin, skipped_length, skipped_duplicate,
    );
    eprintln!("wrote {}", output_path);
    ExitCode::SUCCESS
}

fn accept(
    sentence: String,
    seen: &mut HashSet<String>,
    skipped_latin: &mut usize,
    skipped_length: &mut usize,
    skipped_duplicate: &mut usize,
) -> Option<String> {
    if sentence.chars().any(|c| c.is_ascii_alphabetic()) {
        *skipped_latin += 1;
        return None;
    }
    let word_count = sentence.split_whitespace().count();
    if !(2..=40).contains(&word_count) {
        *skipped_length += 1;
        return None;
    }
    let key = sentence.to_lowercase();
    if !seen.insert(key) {
        *skipped_duplicate += 1;
        return None;
    }
    Some(sentence)
}
