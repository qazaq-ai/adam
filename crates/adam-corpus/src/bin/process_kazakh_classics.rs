// Processor for classical Kazakh authors (non-Abai) from kk.wikisource.org.
//
// Input: data/external/kazakh_classics_plain.txt
//   Records (works) separated by ASCII RS (0x1e), one <p> per line.
//
// Output: data/curated/kazakh_classics_pack.json
//
// Filters (same as abai_wikisource processor, plus v1.x purity gate):
//   - drop Wikipedia footnote refs [1], [2]
//   - drop any Latin-letter run
//   - drop if word count outside [3, 60]
//   - dedup by lowercase form
//   - drop if sample's loanword-density is > 10 %
//     (Russian-only letters OR loanword suffix)
//
// v1.2.0: first significant post-v1.0 corpus addition. Covers
// Ыбырай Алтынсарин + Мағжан Жұмабаев (Abai stays in its own pack).
// Shakarim, Zhambyl, Saken, Mirzhakyp don't yet have pages on
// kk.wikisource — they'll be added when sources exist.

use std::{
    collections::HashSet,
    env,
    fs::File,
    io::{BufReader, Read},
    process::ExitCode,
};

use serde::Serialize;

const DEFAULT_INPUT: &str = "data/external/kazakh_classics_plain.txt";
const DEFAULT_OUTPUT: &str = "data/curated/kazakh_classics_pack.json";
const RECORD_SEP: u8 = 0x1e;

const MIN_WORDS: usize = 3;
const MAX_WORDS: usize = 60;
const LOANWORD_DENSITY_CAP: f32 = 0.10;

/// Russian-only letters — strong loanword signal for pre-modern Kazakh.
const RUSSIAN_ONLY: &[char] = &['ё', 'ф', 'ц', 'ч', 'щ', 'ъ', 'ь', 'э'];

/// Suffix patterns that mark Russian / Greek / Latin loanwords.
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
    works_scanned: usize,
    paragraphs_scanned: usize,
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

    let file = match File::open(&input_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("cannot open {input_path}: {e}");
            eprintln!("hint: run scripts/fetch_kazakh_classics.sh first");
            return ExitCode::FAILURE;
        }
    };
    let mut reader = BufReader::new(file);
    let mut buf: Vec<u8> = Vec::new();
    if let Err(e) = reader.read_to_end(&mut buf) {
        eprintln!("read error: {e}");
        return ExitCode::FAILURE;
    }

    let mut samples: Vec<Sample> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut works_scanned = 0usize;
    let mut paragraphs_scanned = 0usize;
    let mut sentences_scanned = 0usize;
    let mut skipped_latin = 0usize;
    let mut skipped_length = 0usize;
    let mut skipped_duplicate = 0usize;
    let mut skipped_loanword = 0usize;

    for work_bytes in buf.split(|&b| b == RECORD_SEP) {
        if work_bytes.is_empty() {
            continue;
        }
        works_scanned += 1;
        let work_text = match std::str::from_utf8(work_bytes) {
            Ok(s) => s,
            Err(_) => continue,
        };

        for paragraph in work_text.lines() {
            let p = paragraph.trim();
            if p.is_empty() {
                continue;
            }
            paragraphs_scanned += 1;

            for raw_sentence in split_sentences(p) {
                sentences_scanned += 1;
                let text = strip_footnote_refs(raw_sentence).trim().to_string();
                if text.is_empty() {
                    continue;
                }

                if has_latin(&text) {
                    skipped_latin += 1;
                    continue;
                }

                let word_count = text.split_whitespace().count();
                if !(MIN_WORDS..=MAX_WORDS).contains(&word_count) {
                    skipped_length += 1;
                    continue;
                }

                if loanword_density(&text) > LOANWORD_DENSITY_CAP {
                    skipped_loanword += 1;
                    continue;
                }

                let key = text.to_lowercase();
                if !seen.insert(key) {
                    skipped_duplicate += 1;
                    continue;
                }

                let id = format!("kz_classics_{:05}", samples.len() + 1);
                samples.push(Sample {
                    id,
                    pack_name: "adam-kazakh-classics-pack".into(),
                    source_id: "kk.wikisource.org".into(),
                    domain: "literature".into(),
                    text,
                });
            }
        }
    }

    let pack = Pack {
        version: "1.2.0".into(),
        name: "adam-kazakh-classics-pack".into(),
        target_language: "kazakh".into(),
        script: "cyrillic".into(),
        source_license: "CC-BY-SA 4.0".into(),
        source_url: "https://kk.wikisource.org".into(),
        attribution: "Kazakh Wikisource contributors; works by Ыбырай Алтынсарин \
            (1841–1889) and Мағжан Жұмабаев (1893–1938), public domain"
            .into(),
        works_scanned,
        paragraphs_scanned,
        sentences_scanned,
        sample_count: samples.len(),
        skipped_latin,
        skipped_length,
        skipped_duplicate,
        skipped_loanword_heavy: skipped_loanword,
        samples,
    };

    let json = match serde_json::to_string_pretty(&pack) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("serialise: {e}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = std::fs::write(&output_path, &json) {
        eprintln!("cannot write {output_path}: {e}");
        return ExitCode::FAILURE;
    }

    println!(
        "wrote {} samples to {output_path} (works={}, paragraphs={}, sentences={})",
        pack.sample_count, pack.works_scanned, pack.paragraphs_scanned, pack.sentences_scanned
    );
    println!(
        "skipped: latin={} length={} duplicate={} loanword-heavy={}",
        pack.skipped_latin,
        pack.skipped_length,
        pack.skipped_duplicate,
        pack.skipped_loanword_heavy
    );
    ExitCode::SUCCESS
}

fn split_sentences(paragraph: &str) -> Vec<&str> {
    // Same grain as abai_wikisource: naive period/exclaim/question splits.
    paragraph.split(['.', '!', '?']).collect()
}

fn strip_footnote_refs(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '[' {
            // Skip through the next ']'.
            let mut inner = String::new();
            for ic in chars.by_ref() {
                if ic == ']' {
                    break;
                }
                inner.push(ic);
            }
            // Only skip if inner was all digits/whitespace (footnote).
            if inner
                .chars()
                .all(|ch| ch.is_ascii_digit() || ch.is_whitespace())
            {
                continue;
            }
            // Otherwise put back.
            out.push('[');
            out.push_str(&inner);
            out.push(']');
        } else {
            out.push(c);
        }
    }
    out
}

fn has_latin(text: &str) -> bool {
    text.chars().any(|c| c.is_ascii_alphabetic())
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
