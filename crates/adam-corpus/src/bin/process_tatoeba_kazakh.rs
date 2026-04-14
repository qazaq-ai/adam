use std::{collections::HashSet, env, fs, process::ExitCode};

use serde::Serialize;

const DEFAULT_INPUT: &str = "data/external/tatoeba_kazakh_sentences.tsv";
const DEFAULT_OUTPUT: &str = "data/curated/tatoeba_kazakh_pack.json";

#[derive(Debug, Serialize)]
struct Sample {
    id: String,
    pack_name: String,
    source_id: String,
    domain: String,
    tatoeba_sentence_id: u64,
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
    sample_count: usize,
    skipped_empty: usize,
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

    let raw = match fs::read_to_string(&input_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot read {input_path}: {e}");
            eprintln!("hint: run scripts/fetch_tatoeba_kazakh.sh first");
            return ExitCode::FAILURE;
        }
    };

    let mut samples: Vec<Sample> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut skipped_empty = 0usize;
    let mut skipped_latin = 0usize;
    let mut skipped_length = 0usize;
    let mut skipped_duplicate = 0usize;

    for line in raw.lines() {
        let mut parts = line.splitn(3, '\t');
        let id_str = match parts.next() {
            Some(s) => s,
            None => continue,
        };
        let _lang = parts.next();
        let text = match parts.next() {
            Some(s) => s.trim().to_string(),
            None => continue,
        };

        if text.is_empty() {
            skipped_empty += 1;
            continue;
        }

        // Filter: allow Cyrillic + common punctuation + digits; reject any
        // Latin letters (usually means proper-noun noise or mis-tagged lines).
        if text.chars().any(|c| c.is_ascii_alphabetic()) {
            skipped_latin += 1;
            continue;
        }

        let word_count = text.split_whitespace().count();
        if !(2..=40).contains(&word_count) {
            skipped_length += 1;
            continue;
        }

        let normalized = text.to_lowercase();
        if !seen.insert(normalized) {
            skipped_duplicate += 1;
            continue;
        }

        let tid: u64 = id_str.trim().parse().unwrap_or(0);
        let idx = samples.len() + 1;
        samples.push(Sample {
            id: format!("tatoeba_kz_{idx:05}"),
            pack_name: "adam-tatoeba-kazakh-pack".to_string(),
            source_id: format!("tatoeba_kazakh_{tid}"),
            domain: "tatoeba".to_string(),
            tatoeba_sentence_id: tid,
            text,
        });
    }

    let sample_count = samples.len();
    let pack = Pack {
        version: env!("CARGO_PKG_VERSION").to_string(),
        name: "adam-tatoeba-kazakh-pack".to_string(),
        target_language: "kazakh".to_string(),
        script: "cyrillic".to_string(),
        source_license: "CC-BY 2.0 FR".to_string(),
        source_url: "https://tatoeba.org".to_string(),
        attribution: "Tatoeba.org contributors (CC-BY 2.0 FR)".to_string(),
        sample_count,
        skipped_empty,
        skipped_latin,
        skipped_length,
        skipped_duplicate,
        samples,
    };

    if let Some(parent) = std::path::Path::new(&output_path).parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            let _ = fs::create_dir_all(parent);
        }
    }
    if let Err(e) = fs::write(
        &output_path,
        serde_json::to_string_pretty(&pack).expect("serialize"),
    ) {
        eprintln!("cannot write {output_path}: {e}");
        return ExitCode::FAILURE;
    }

    eprintln!(
        "accepted={} skipped_empty={} skipped_latin={} skipped_length={} skipped_duplicate={}",
        sample_count, skipped_empty, skipped_latin, skipped_length, skipped_duplicate,
    );
    eprintln!("wrote {}", output_path);
    ExitCode::SUCCESS
}
