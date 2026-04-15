use std::{
    collections::HashSet,
    env,
    fs::File,
    io::{BufRead, BufReader, Read},
    process::ExitCode,
};

use serde::Serialize;

const DEFAULT_INPUT: &str = "data/external/wikipedia_kz_plain.txt";
const DEFAULT_OUTPUT: &str = "data/curated/wikipedia_kz_pack.json";
const DEFAULT_TARGET: usize = 15_000;

// ASCII record separator used by the fetch script to delimit articles.
const ARTICLE_SEP: u8 = 0x1e;

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
    articles_scanned: usize,
    sentences_scanned: usize,
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
    let target_samples: usize = env::args()
        .nth(3)
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_TARGET);

    let file = match File::open(&input_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("cannot open {input_path}: {e}");
            eprintln!("hint: run scripts/fetch_wikipedia_kz.sh first");
            return ExitCode::FAILURE;
        }
    };
    let mut reader = BufReader::new(file);

    let mut samples: Vec<Sample> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut articles_scanned = 0usize;
    let mut sentences_scanned = 0usize;
    let mut skipped_latin = 0usize;
    let mut skipped_length = 0usize;
    let mut skipped_duplicate = 0usize;

    let mut article = Vec::<u8>::new();
    let mut byte = [0u8; 1];

    loop {
        if samples.len() >= target_samples {
            break;
        }
        match reader.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(e) => {
                eprintln!("read error: {e}");
                return ExitCode::FAILURE;
            }
        }
        if byte[0] == ARTICLE_SEP {
            articles_scanned += 1;
            let text = String::from_utf8_lossy(&article);
            for sent in split_sentences(&text) {
                sentences_scanned += 1;
                let result = accept(
                    sent,
                    &mut seen,
                    &mut skipped_latin,
                    &mut skipped_length,
                    &mut skipped_duplicate,
                );
                if let Some(s) = result {
                    let idx = samples.len() + 1;
                    samples.push(Sample {
                        id: format!("wiki_kz_{idx:05}"),
                        pack_name: "adam-wikipedia-kz-pack".to_string(),
                        source_id: format!("wikipedia_kz_article_{articles_scanned}"),
                        domain: "wikipedia".to_string(),
                        text: s,
                    });
                    if samples.len() >= target_samples {
                        break;
                    }
                }
            }
            article.clear();
        } else {
            article.push(byte[0]);
        }
    }

    let sample_count = samples.len();
    let pack = Pack {
        version: env!("CARGO_PKG_VERSION").to_string(),
        name: "adam-wikipedia-kz-pack".to_string(),
        target_language: "kazakh".to_string(),
        script: "cyrillic".to_string(),
        source_license: "CC-BY-SA 4.0".to_string(),
        source_url: "https://kk.wikipedia.org".to_string(),
        attribution: "Wikipedia contributors, Kazakh Wikipedia (CC-BY-SA 4.0)".to_string(),
        articles_scanned,
        sentences_scanned,
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
        "articles={} sentences_scanned={} accepted={} skipped_latin={} skipped_length={} skipped_dup={}",
        articles_scanned,
        sentences_scanned,
        sample_count,
        skipped_latin,
        skipped_length,
        skipped_duplicate,
    );
    eprintln!("wrote {}", output_path);
    ExitCode::SUCCESS
}

/// Split a chunk of Wikipedia article text into candidate sentences.
fn split_sentences(text: &str) -> Vec<String> {
    let mut sents = Vec::new();
    let mut cur = String::new();
    for ch in text.chars() {
        cur.push(ch);
        if matches!(ch, '.' | '!' | '?') {
            // Sentence boundary after terminator.
            let trimmed = cur.trim().to_string();
            if !trimmed.is_empty() {
                sents.push(trimmed);
            }
            cur.clear();
        }
    }
    if !cur.trim().is_empty() {
        sents.push(cur.trim().to_string());
    }
    sents
}

fn accept(
    sentence: String,
    seen: &mut HashSet<String>,
    skipped_latin: &mut usize,
    skipped_length: &mut usize,
    skipped_duplicate: &mut usize,
) -> Option<String> {
    // Reject any Latin letter: signals noise (English tech terms, stray markup, etc).
    if sentence.chars().any(|c| c.is_ascii_alphabetic()) {
        *skipped_latin += 1;
        return None;
    }
    let word_count = sentence.split_whitespace().count();
    if !(4..=40).contains(&word_count) {
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

#[allow(dead_code)]
fn _unused_bufread(_: &dyn BufRead) {}
