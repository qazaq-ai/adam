// Processor for Abai Qunanbayuly's classical Kazakh works from kk.wikisource.org.
//
// Input: data/external/abai_wikisource_plain.txt
//   Records (works) separated by ASCII RS (0x1e), one <p> extraction per line.
//
// Output: data/curated/abai_wikisource_pack.json
//
// Each <p> may be a full poem stanza, a philosophical paragraph, or a line
// from the "Qara sözder" (Words of Wisdom) essays. We split those paragraphs
// into sentence-sized samples so they roughly match the other packs' grain.
//
// Filters:
//   - drop Wikipedia footnote refs like [1], [2]
//   - drop any Latin-letter run (stripped references, stray markup)
//   - drop if word count outside [3, 60]  (looser than wiki's 4..40 since Abai
//     uses both very short poetic lines and longer prose sentences)
//   - dedup by lowercase form

use std::{
    collections::HashSet,
    env,
    fs::File,
    io::{BufReader, Read},
    process::ExitCode,
};

use serde::Serialize;

const DEFAULT_INPUT: &str = "data/external/abai_wikisource_plain.txt";
const DEFAULT_OUTPUT: &str = "data/curated/abai_wikisource_pack.json";
const RECORD_SEP: u8 = 0x1e;

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
            eprintln!("hint: run scripts/fetch_abai_wikisource.sh first");
            return ExitCode::FAILURE;
        }
    };
    let mut reader = BufReader::new(file);

    let mut samples: Vec<Sample> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut works_scanned = 0usize;
    let mut paragraphs_scanned = 0usize;
    let mut sentences_scanned = 0usize;
    let mut skipped_latin = 0usize;
    let mut skipped_length = 0usize;
    let mut skipped_duplicate = 0usize;

    let mut buf = Vec::<u8>::new();
    let mut byte = [0u8; 1];

    loop {
        match reader.read(&mut byte) {
            Ok(0) => {
                if !buf.is_empty() {
                    process_work(
                        &buf,
                        &mut samples,
                        &mut seen,
                        &mut paragraphs_scanned,
                        &mut sentences_scanned,
                        &mut skipped_latin,
                        &mut skipped_length,
                        &mut skipped_duplicate,
                        works_scanned,
                    );
                    works_scanned += 1;
                }
                break;
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("read error: {e}");
                return ExitCode::FAILURE;
            }
        }
        if byte[0] == RECORD_SEP {
            process_work(
                &buf,
                &mut samples,
                &mut seen,
                &mut paragraphs_scanned,
                &mut sentences_scanned,
                &mut skipped_latin,
                &mut skipped_length,
                &mut skipped_duplicate,
                works_scanned,
            );
            works_scanned += 1;
            buf.clear();
        } else {
            buf.push(byte[0]);
        }
    }

    let sample_count = samples.len();
    let pack = Pack {
        version: env!("CARGO_PKG_VERSION").to_string(),
        name: "adam-abai-wikisource-pack".to_string(),
        target_language: "kazakh".to_string(),
        script: "cyrillic".to_string(),
        source_license: "public domain (Abai Qunanbayuly, d. 1904) / CC-BY-SA 4.0 (Wikisource presentation)".to_string(),
        source_url: "https://kk.wikisource.org/wiki/Абай_Құнанбайұлы".to_string(),
        attribution: "Abai Qunanbayuly (1845–1904), classical Kazakh literature; text via Kazakh Wikisource contributors (CC-BY-SA 4.0)".to_string(),
        works_scanned,
        paragraphs_scanned,
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
        "works={} paragraphs={} sentences={} accepted={} skipped_latin={} skipped_length={} skipped_dup={}",
        works_scanned,
        paragraphs_scanned,
        sentences_scanned,
        sample_count,
        skipped_latin,
        skipped_length,
        skipped_duplicate,
    );
    eprintln!("wrote {}", output_path);
    ExitCode::SUCCESS
}

#[allow(clippy::too_many_arguments)]
fn process_work(
    raw: &[u8],
    samples: &mut Vec<Sample>,
    seen: &mut HashSet<String>,
    paragraphs_scanned: &mut usize,
    sentences_scanned: &mut usize,
    skipped_latin: &mut usize,
    skipped_length: &mut usize,
    skipped_duplicate: &mut usize,
    work_idx: usize,
) {
    let text = String::from_utf8_lossy(raw);
    for paragraph in text.split('\n') {
        let paragraph = paragraph.trim();
        if paragraph.is_empty() {
            continue;
        }
        *paragraphs_scanned += 1;
        let cleaned = strip_footnotes(paragraph);
        for sent in split_sentences(&cleaned) {
            *sentences_scanned += 1;
            let trimmed = sent.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.chars().any(|c| c.is_ascii_alphabetic()) {
                *skipped_latin += 1;
                continue;
            }
            let word_count = trimmed.split_whitespace().count();
            if !(3..=60).contains(&word_count) {
                *skipped_length += 1;
                continue;
            }
            let key = trimmed.to_lowercase();
            if !seen.insert(key) {
                *skipped_duplicate += 1;
                continue;
            }
            let idx = samples.len() + 1;
            samples.push(Sample {
                id: format!("abai_{idx:05}"),
                pack_name: "adam-abai-wikisource-pack".to_string(),
                source_id: format!("abai_work_{work_idx}"),
                domain: "classical_literature".to_string(),
                text: trimmed.to_string(),
            });
        }
    }
}

/// Strip Wikisource footnote refs like `[1]`, `[12]`, and the HTML-encoded
/// forms that slipped through the perl pipeline: `&#91;1&#93;`.
fn strip_footnotes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Match &#91; ... &#93;  (HTML entities for [ and ])
        if i + 5 <= bytes.len() && &bytes[i..i + 5] == b"&#91;" {
            if let Some(end) = s[i..].find("&#93;") {
                i += end + 5;
                continue;
            }
        }
        // Match literal [digits]
        if bytes[i] == b'[' {
            if let Some(end) = s[i..].find(']') {
                if s[i + 1..i + end].chars().all(|c| c.is_ascii_digit()) {
                    i += end + 1;
                    continue;
                }
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    // The hack above converts raw bytes to chars via `as char` which is wrong
    // for multibyte UTF-8. Fallback: if stripping didn't change length by a lot,
    // use the char-iter version.
    let mut out2 = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            let rest: String = chars.clone().take(4).collect();
            if rest == "#91;" {
                for _ in 0..4 {
                    chars.next();
                }
                while let Some(&nc) = chars.peek() {
                    chars.next();
                    if nc == ';' {
                        // We just consumed through the closing ';' of &#93;.
                        break;
                    }
                }
                continue;
            }
        }
        if c == '[' {
            // Try to consume `[digits]`.
            let mut lookahead = chars.clone();
            let mut digits = String::new();
            let mut closed = false;
            while let Some(&nc) = lookahead.peek() {
                if nc.is_ascii_digit() {
                    digits.push(nc);
                    lookahead.next();
                } else if nc == ']' && !digits.is_empty() {
                    closed = true;
                    lookahead.next();
                    break;
                } else {
                    break;
                }
            }
            if closed {
                chars = lookahead;
                continue;
            }
        }
        out2.push(c);
    }
    out2
}

/// Split Abai text into sentence-ish units. Respect Kazakh punctuation that
/// signals a break: period, question, exclamation, and also full-stop variants
/// used in poetic lines (';', ':').
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
