// Process the full Kazakh Wikipedia dump into a curated pack.
//
// Input:  data/external/wikipedia_kz_plain.txt (~638 MB as of v1.3.0)
//         Articles separated by ASCII RS (0x1e).
// Output: data/curated/wikipedia_kz_pack.json
//
// v1.3.0 rewrite:
//   - stream in 64 KB chunks (old version read 1 byte at a time, ~hours on
//     638 MB; chunked version does it in seconds)
//   - optional target-cap argument retained for dev / sanity runs, but the
//     default is unbounded — we want the full Wikipedia yield.
//   - adds loanword-density filter (drop samples > 10 % loanword tokens)
//   - drops the 2-word lesson's residue: sentences below 4 words still rejected
//
// Filters (per v1.x corpus purity directive):
//   - reject if any Latin letter (stray English / wiki markup)
//   - reject if word count outside [4, 40]
//   - reject if sample's loanword density > 10 %
//     (Russian-only letter OR loanword suffix)
//   - dedup cross-article by lowercased full text

use std::{
    collections::HashSet,
    env,
    fs::File,
    io::{BufReader, Read},
    process::ExitCode,
};

use serde::Serialize;

const DEFAULT_INPUT: &str = "data/external/wikipedia_kz_plain.txt";
const DEFAULT_OUTPUT: &str = "data/curated/wikipedia_kz_pack.json";

const ARTICLE_SEP: u8 = 0x1e;
const READ_CHUNK: usize = 64 * 1024;

const MIN_WORDS: usize = 4;
const MAX_WORDS: usize = 40;
const LOANWORD_DENSITY_CAP: f32 = 0.10;

/// Russian-only letters that never appear in native pre-modern Kazakh.
const RUSSIAN_ONLY: &[char] = &['ё', 'ф', 'ц', 'ч', 'щ', 'ъ', 'ь', 'э'];

/// Loanword suffix patterns shared across processors.
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
    skipped_loanword_heavy: usize,
    samples: Vec<Sample>,
}

fn main() -> ExitCode {
    let input_path = env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_INPUT.to_string());
    let output_path = env::args()
        .nth(2)
        .unwrap_or_else(|| DEFAULT_OUTPUT.to_string());
    // Optional third arg = dev target cap; default unbounded.
    let target_cap: Option<usize> = env::args().nth(3).and_then(|s| s.parse().ok());

    let file = match File::open(&input_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("cannot open {input_path}: {e}");
            eprintln!("hint: run scripts/fetch_wikipedia_kz.sh first");
            return ExitCode::FAILURE;
        }
    };
    let mut reader = BufReader::with_capacity(READ_CHUNK, file);

    let mut samples: Vec<Sample> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut articles_scanned = 0usize;
    let mut sentences_scanned = 0usize;
    let mut skipped_latin = 0usize;
    let mut skipped_length = 0usize;
    let mut skipped_duplicate = 0usize;
    let mut skipped_loanword = 0usize;

    // Buffered streaming by article. Accumulate bytes into `article`; on
    // each ARTICLE_SEP flush and process.
    let mut article: Vec<u8> = Vec::with_capacity(16 * 1024);
    let mut chunk = [0u8; READ_CHUNK];

    'outer: loop {
        let n = match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                eprintln!("read error: {e}");
                return ExitCode::FAILURE;
            }
        };
        for &b in &chunk[..n] {
            if b == ARTICLE_SEP {
                articles_scanned += 1;
                process_article(
                    &article,
                    &mut samples,
                    &mut seen,
                    &mut sentences_scanned,
                    &mut skipped_latin,
                    &mut skipped_length,
                    &mut skipped_duplicate,
                    &mut skipped_loanword,
                );
                article.clear();
                if let Some(cap) = target_cap {
                    if samples.len() >= cap {
                        break 'outer;
                    }
                }
            } else {
                article.push(b);
            }
        }
    }
    // Flush any trailing article without a terminator.
    if !article.is_empty() {
        articles_scanned += 1;
        process_article(
            &article,
            &mut samples,
            &mut seen,
            &mut sentences_scanned,
            &mut skipped_latin,
            &mut skipped_length,
            &mut skipped_duplicate,
            &mut skipped_loanword,
        );
    }

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
        sample_count: samples.len(),
        skipped_latin,
        skipped_length,
        skipped_duplicate,
        skipped_loanword_heavy: skipped_loanword,
        samples,
    };

    if let Some(parent) = std::path::Path::new(&output_path).parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            let _ = std::fs::create_dir_all(parent);
        }
    }
    match serde_json::to_string_pretty(&pack) {
        Ok(s) => {
            if let Err(e) = std::fs::write(&output_path, s) {
                eprintln!("cannot write {output_path}: {e}");
                return ExitCode::FAILURE;
            }
        }
        Err(e) => {
            eprintln!("serialise: {e}");
            return ExitCode::FAILURE;
        }
    }

    eprintln!(
        "articles={} sentences_scanned={} accepted={} skipped_latin={} skipped_length={} skipped_dup={} skipped_loanword={}",
        pack.articles_scanned,
        pack.sentences_scanned,
        pack.sample_count,
        pack.skipped_latin,
        pack.skipped_length,
        pack.skipped_duplicate,
        pack.skipped_loanword_heavy,
    );
    eprintln!("wrote {output_path}");
    ExitCode::SUCCESS
}

#[allow(clippy::too_many_arguments)]
fn process_article(
    bytes: &[u8],
    samples: &mut Vec<Sample>,
    seen: &mut HashSet<String>,
    sentences_scanned: &mut usize,
    skipped_latin: &mut usize,
    skipped_length: &mut usize,
    skipped_duplicate: &mut usize,
    skipped_loanword: &mut usize,
) {
    let text = match std::str::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => return, // malformed UTF-8 slice, skip
    };
    for sent in split_sentences(text) {
        *sentences_scanned += 1;
        match accept(
            sent,
            seen,
            skipped_latin,
            skipped_length,
            skipped_duplicate,
            skipped_loanword,
        ) {
            Some(clean) => {
                let idx = samples.len() + 1;
                samples.push(Sample {
                    id: format!("wiki_kz_{idx:07}"),
                    pack_name: "adam-wikipedia-kz-pack".to_string(),
                    source_id: "wikipedia_kz".to_string(),
                    domain: "wikipedia".to_string(),
                    text: clean,
                });
            }
            None => {}
        }
    }
}

/// Split article text into candidate sentences on `.?!`.
fn split_sentences(text: &str) -> Vec<String> {
    let mut sents = Vec::new();
    let mut cur = String::new();
    for ch in text.chars() {
        cur.push(ch);
        if matches!(ch, '.' | '!' | '?') {
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
    skipped_loanword: &mut usize,
) -> Option<String> {
    // Reject any Latin letter — signals stray English / wiki markup.
    if sentence.chars().any(|c| c.is_ascii_alphabetic()) {
        *skipped_latin += 1;
        return None;
    }
    let word_count = sentence.split_whitespace().count();
    if !(MIN_WORDS..=MAX_WORDS).contains(&word_count) {
        *skipped_length += 1;
        return None;
    }
    if loanword_density(&sentence) > LOANWORD_DENSITY_CAP {
        *skipped_loanword += 1;
        return None;
    }
    let key = sentence.to_lowercase();
    if !seen.insert(key) {
        *skipped_duplicate += 1;
        return None;
    }
    Some(sentence)
}

fn loanword_density(text: &str) -> f32 {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return 0.0;
    }
    let flagged = words.iter().filter(|w| is_loanword_token(w)).count();
    flagged as f32 / words.len() as f32
}

fn is_loanword_token(word: &str) -> bool {
    let cleaned: String = word
        .chars()
        .filter(|c| c.is_alphabetic() || *c == '-')
        .collect::<String>()
        .to_lowercase();
    if cleaned.chars().any(|c| RUSSIAN_ONLY.contains(&c)) {
        return true;
    }
    LOANWORD_SUFFIXES
        .iter()
        .any(|s| cleaned.ends_with(s) && cleaned.chars().count() > s.chars().count() + 1)
}
