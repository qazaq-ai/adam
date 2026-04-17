// Streaming processor for CC-100 Kazakh.
//
// Input: stdin (one line per sentence; feed via `xzcat cc100_kk.txt.xz | ...`)
// Output: data/curated/cc100_kk_pack.json
//
// CC-100 is Common Crawl filtered by language. The Kazakh slice is ~5-8 GB
// decompressed, heterogeneous quality. We stream, filter, and stop early at
// a target sample count — we don't need the whole corpus.
//
// Filters (rejection reasons, in order):
//   - length: word count outside [4, 40]
//   - latin:  any run of >= 3 consecutive ASCII letters (indicates English/code)
//   - cyrillic_ratio: < 80% of alphabetic chars are Cyrillic
//   - url_email: line contains "http", "www.", "@", or ".com/.org/.net/.ru/.kz"
//   - repetitive: contains a 3-gram repeated >= 3 times (boilerplate detector)
//   - punct_heavy: non-alphabetic char ratio > 30%
//   - duplicate: lowercase text seen already
//
// Stops after MAX_SAMPLES accepted or EOF. Reports stats to stderr.

use std::{
    collections::HashSet,
    env,
    io::{self, BufRead},
    process::ExitCode,
};

use serde::Serialize;

const DEFAULT_OUTPUT: &str = "data/curated/cc100_kk_pack.json";
const DEFAULT_MAX_SAMPLES: usize = 50_000;

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
    skipped_length: usize,
    skipped_latin: usize,
    skipped_cyrillic_ratio: usize,
    skipped_url_email: usize,
    skipped_repetitive: usize,
    skipped_punct_heavy: usize,
    skipped_duplicate: usize,
    samples: Vec<Sample>,
}

fn main() -> ExitCode {
    let output_path = env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_OUTPUT.to_string());
    let max_samples: usize = env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_MAX_SAMPLES);

    let stdin = io::stdin();
    let reader = stdin.lock();

    let mut samples: Vec<Sample> = Vec::with_capacity(max_samples);
    let mut seen: HashSet<String> = HashSet::with_capacity(max_samples * 2);
    let mut lines_scanned = 0usize;
    let mut skipped_length = 0usize;
    let mut skipped_latin = 0usize;
    let mut skipped_cyr_ratio = 0usize;
    let mut skipped_url = 0usize;
    let mut skipped_repetitive = 0usize;
    let mut skipped_punct = 0usize;
    let mut skipped_duplicate = 0usize;

    for line in reader.lines() {
        let Ok(line) = line else {
            continue;
        };
        lines_scanned += 1;
        if samples.len() >= max_samples {
            break;
        }
        if lines_scanned % 100_000 == 0 {
            eprintln!(
                "progress: scanned={} accepted={}",
                lines_scanned,
                samples.len()
            );
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let word_count = trimmed.split_whitespace().count();
        if !(4..=40).contains(&word_count) {
            skipped_length += 1;
            continue;
        }
        if has_latin_run(trimmed, 3) {
            skipped_latin += 1;
            continue;
        }
        if cyrillic_alpha_ratio(trimmed) < 0.80 {
            skipped_cyr_ratio += 1;
            continue;
        }
        if contains_url_or_email(trimmed) {
            skipped_url += 1;
            continue;
        }
        if has_repetitive_trigram(trimmed) {
            skipped_repetitive += 1;
            continue;
        }
        if non_alpha_ratio(trimmed) > 0.30 {
            skipped_punct += 1;
            continue;
        }
        let key = trimmed.to_lowercase();
        if !seen.insert(key) {
            skipped_duplicate += 1;
            continue;
        }

        let idx = samples.len() + 1;
        samples.push(Sample {
            id: format!("cc100_kk_{idx:06}"),
            pack_name: "adam-cc100-kk-pack".to_string(),
            source_id: format!("cc100_kk_line_{lines_scanned}"),
            domain: "web_crawl".to_string(),
            text: trimmed.to_string(),
        });
    }

    let sample_count = samples.len();
    let pack = Pack {
        version: env!("CARGO_PKG_VERSION").to_string(),
        name: "adam-cc100-kk-pack".to_string(),
        target_language: "kazakh".to_string(),
        script: "cyrillic".to_string(),
        source_license: "Common Crawl terms of use".to_string(),
        source_url: "https://data.statmt.org/cc-100/kk.txt.xz".to_string(),
        attribution: "Conneau et al. 2020, Unsupervised Cross-lingual Representation Learning at Scale (XLM-R)".to_string(),
        lines_scanned,
        sample_count,
        skipped_length,
        skipped_latin,
        skipped_cyrillic_ratio: skipped_cyr_ratio,
        skipped_url_email: skipped_url,
        skipped_repetitive,
        skipped_punct_heavy: skipped_punct,
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
        "lines={} accepted={} skipped_length={} skipped_latin={} skipped_cyr={} skipped_url={} skipped_rep={} skipped_punct={} skipped_dup={}",
        lines_scanned,
        sample_count,
        skipped_length,
        skipped_latin,
        skipped_cyr_ratio,
        skipped_url,
        skipped_repetitive,
        skipped_punct,
        skipped_duplicate,
    );
    eprintln!("wrote {}", output_path);
    ExitCode::SUCCESS
}

fn has_latin_run(s: &str, min_run: usize) -> bool {
    let mut run = 0usize;
    for c in s.chars() {
        if c.is_ascii_alphabetic() {
            run += 1;
            if run >= min_run {
                return true;
            }
        } else {
            run = 0;
        }
    }
    false
}

fn cyrillic_alpha_ratio(s: &str) -> f32 {
    let mut alpha = 0usize;
    let mut cyr = 0usize;
    for c in s.chars() {
        if c.is_alphabetic() {
            alpha += 1;
            if ('\u{0400}'..='\u{04FF}').contains(&c) {
                cyr += 1;
            }
        }
    }
    if alpha == 0 {
        return 0.0;
    }
    cyr as f32 / alpha as f32
}

fn contains_url_or_email(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    lower.contains("http")
        || lower.contains("www.")
        || lower.contains('@')
        || lower.contains(".com")
        || lower.contains(".org")
        || lower.contains(".net")
        || lower.contains(".ru")
        || lower.contains(".kz")
}

fn has_repetitive_trigram(s: &str) -> bool {
    let words: Vec<&str> = s.split_whitespace().collect();
    if words.len() < 9 {
        return false;
    }
    let mut counts: std::collections::HashMap<(&str, &str, &str), usize> =
        std::collections::HashMap::new();
    for w in words.windows(3) {
        let key = (w[0], w[1], w[2]);
        let c = counts.entry(key).or_insert(0);
        *c += 1;
        if *c >= 3 {
            return true;
        }
    }
    false
}

fn non_alpha_ratio(s: &str) -> f32 {
    let mut total = 0usize;
    let mut non_alpha = 0usize;
    for c in s.chars() {
        if c.is_whitespace() {
            continue;
        }
        total += 1;
        if !c.is_alphabetic() {
            non_alpha += 1;
        }
    }
    if total == 0 {
        return 0.0;
    }
    non_alpha as f32 / total as f32
}
