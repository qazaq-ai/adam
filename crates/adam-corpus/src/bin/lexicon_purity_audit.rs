// Audit the v1 lexicon (curated + Apertium-imported) against "pure
// pre-modern Kazakh" criteria. Produces a per-POS report with:
//   - total entries
//   - entries containing any Russian-only letter (ё, ф, ц, ч, щ, ъ, ь, э)
//   - entries containing no Kazakh-specific letter (ә, ғ, қ, ң, ө, ұ, ү, һ, і)
//   - entries ending in known loanword suffix (-ция, -изм, -лог, …)
//   - entries attested in Abai's corpus (classical 19th-century Kazakh)
//   - clean entries (all filters pass)
//
// Output: data/lexicon_v1/purity_audit_report.json

use std::{
    collections::{BTreeMap, HashSet},
    env, fs,
    process::ExitCode,
};

use serde::{Deserialize, Serialize};

const CURATED: &str = "data/tokenizer/segmentation_roots.json";
const APERTIUM: &str = "data/lexicon_v1/apertium_imported_roots.json";
const ABAI_PACK: &str = "data/curated/abai_wikisource_pack.json";
const REPORT_PATH: &str = "data/lexicon_v1/purity_audit_report.json";

/// Cyrillic letters that never occur in native pre-modern Kazakh stems.
/// Their presence is a strong Russian-loanword signal.
const RUSSIAN_ONLY: &[char] = &['ё', 'ф', 'ц', 'ч', 'щ', 'ъ', 'ь', 'э'];

/// Kazakh-specific Cyrillic letters. A native Kazakh root of length >= 4
/// usually has at least one of these.
const KAZAKH_SPECIFIC: &[char] = &['ә', 'ғ', 'қ', 'ң', 'ө', 'ұ', 'ү', 'һ', 'і'];

/// Loanword suffix markers — Russian / Greek / Latin borrowings.
const LOANWORD_SUFFIXES: &[&str] = &[
    "ция",
    "цияны",
    "цияның",
    "изм",
    "измде",
    "граф",
    "графия",
    "логия",
    "лог",
    "тика",
    "тикалық",
    "ивный",
    "ильный",
    "альный",
    "альная",
    "альное",
    "онный",
    "атор",
    "итор",
    "ист",
    "изация",
    "инг",
    "инк",
    "метр",
    "фон",
    "трон",
];

#[derive(Debug, Clone, Deserialize, Serialize)]
struct RootEntry {
    id: String,
    root: String,
    part_of_speech: String,
    vowel_harmony: String,
    final_sound_class: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RootsFile {
    roots: Vec<RootEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct Sample {
    text: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Pack {
    samples: Vec<Sample>,
}

#[derive(Debug, Serialize, Default, Clone)]
struct PosStats {
    total: usize,
    has_russian_only_letter: usize,
    no_kazakh_specific: usize,
    loanword_suffix: usize,
    attested_in_abai: usize,
    clean: usize,
}

#[derive(Debug, Serialize)]
struct Report {
    total_entries: usize,
    by_pos: BTreeMap<String, PosStats>,
    abai_vocabulary_size: usize,
    clean_entries_total: usize,
    examples_dropped_russian: Vec<String>,
    examples_dropped_loanword_suffix: Vec<String>,
    examples_attested_in_abai: Vec<String>,
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let report_path = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| REPORT_PATH.to_string());

    let curated: RootsFile = match read_json(CURATED) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("read curated: {e}");
            return ExitCode::FAILURE;
        }
    };
    let apertium: RootsFile = match read_json(APERTIUM) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("read apertium: {e}");
            return ExitCode::FAILURE;
        }
    };
    let abai: Pack = match read_json(ABAI_PACK) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("read abai: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Build Abai vocabulary: all whitespace-separated words from the Abai
    // corpus, lowercased, with leading/trailing punctuation stripped.
    let mut abai_vocab: HashSet<String> = HashSet::new();
    for s in &abai.samples {
        for w in s.text.split_whitespace() {
            let cleaned: String = w
                .chars()
                .filter(|c| c.is_alphabetic() || *c == '-')
                .collect::<String>()
                .to_lowercase();
            if !cleaned.is_empty() {
                abai_vocab.insert(cleaned);
            }
        }
    }

    let mut by_pos: BTreeMap<String, PosStats> = BTreeMap::new();
    let mut examples_dropped_russian: Vec<String> = Vec::new();
    let mut examples_dropped_loanword: Vec<String> = Vec::new();
    let mut examples_attested: Vec<String> = Vec::new();
    let mut total_entries = 0usize;
    let mut clean_total = 0usize;

    let mut all_entries = curated.roots.clone();
    all_entries.extend(apertium.roots.clone());

    for e in &all_entries {
        total_entries += 1;
        let pos = e.part_of_speech.clone();
        let stats = by_pos.entry(pos.clone()).or_default();
        stats.total += 1;

        let has_russian = e.root.chars().any(|c| RUSSIAN_ONLY.contains(&c));
        let has_specific = e.root.chars().any(|c| KAZAKH_SPECIFIC.contains(&c));
        let loanword_suffix = LOANWORD_SUFFIXES
            .iter()
            .any(|suf| e.root.len() > suf.len() && e.root.ends_with(suf));
        let attested = abai_vocab
            .iter()
            .any(|w| w == &e.root || w.starts_with(&e.root));

        if has_russian {
            stats.has_russian_only_letter += 1;
            if examples_dropped_russian.len() < 15 {
                examples_dropped_russian.push(e.root.clone());
            }
        }
        if !has_specific && e.root.chars().count() >= 4 {
            stats.no_kazakh_specific += 1;
        }
        if loanword_suffix {
            stats.loanword_suffix += 1;
            if examples_dropped_loanword.len() < 15 {
                examples_dropped_loanword.push(e.root.clone());
            }
        }
        if attested {
            stats.attested_in_abai += 1;
            if examples_attested.len() < 15 {
                examples_attested.push(e.root.clone());
            }
        }

        // A "clean" entry: no Russian-only letter, no loanword suffix, and
        // either has a Kazakh-specific letter or is attested in Abai.
        if !has_russian && !loanword_suffix && (has_specific || attested) {
            stats.clean += 1;
            clean_total += 1;
        }
    }

    let report = Report {
        total_entries,
        by_pos,
        abai_vocabulary_size: abai_vocab.len(),
        clean_entries_total: clean_total,
        examples_dropped_russian,
        examples_dropped_loanword_suffix: examples_dropped_loanword,
        examples_attested_in_abai: examples_attested,
    };

    if let Err(e) = fs::write(&report_path, serde_json::to_string_pretty(&report).unwrap()) {
        eprintln!("write report: {e}");
        return ExitCode::FAILURE;
    }

    // Human-readable summary to stderr.
    eprintln!("=== v1 lexicon purity audit ===");
    eprintln!("total entries:          {}", report.total_entries);
    eprintln!("clean entries (all OK): {}", report.clean_entries_total);
    eprintln!("Abai vocabulary size:   {}", report.abai_vocabulary_size);
    eprintln!();
    eprintln!(
        "{:<14} {:>6} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "POS", "total", "clean", "russ.let", "no-kaz", "loanw.suf", "attest"
    );
    for (pos, stats) in &report.by_pos {
        eprintln!(
            "{:<14} {:>6} {:>8} {:>8} {:>8} {:>8} {:>8}",
            pos,
            stats.total,
            stats.clean,
            stats.has_russian_only_letter,
            stats.no_kazakh_specific,
            stats.loanword_suffix,
            stats.attested_in_abai,
        );
    }
    eprintln!();
    eprintln!(
        "dropped (Russian letter):  {:?}",
        report.examples_dropped_russian
    );
    eprintln!(
        "dropped (loanword suffix): {:?}",
        report.examples_dropped_loanword_suffix
    );
    eprintln!();
    eprintln!("wrote {}", report_path);
    ExitCode::SUCCESS
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let raw = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}
