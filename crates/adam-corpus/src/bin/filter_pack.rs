// Filter a source pack down to pure-Kazakh, morphologically-dense samples.
//
// For every sample in the input pack:
//   1. Require at least MIN_WORDS (default 4) whitespace-separated tokens.
//   2. Require per-sample FSM coverage ≥ FSM_RATIO (default 0.70) — i.e.,
//      70% of words must be segmentable by the curated + extended lexicon.
//   3. Reject if any word ends in a hardcoded loanword suffix pattern
//      (-ция, -изм, -лог, -графия, -тика, -ивный, -ильный, -альный).
//   4. Reject if any word is in the hardcoded loanword blocklist.
//
// Input:  data/curated/<name>_pack.json
// Output: data/curated/filtered_<name>_pack.json with same schema.
//
// This implements the user's v0.5.0 ask: drop 2-/3-word fragments, drop
// loanwords, keep pure Kazakh agglutinative signal.

use std::{env, fs, path::PathBuf, process::ExitCode};

use adam_kernel::{SegmentationLexicon, SegmentationRuleSet, deterministic_segment_token};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const ROOTS_PATH: &str = "data/tokenizer/segmentation_roots.json";
const RULES_PATH: &str = "data/tokenizer/segmentation_rules.json";
const DEFAULT_MIN_WORDS: usize = 4;
const DEFAULT_FSM_RATIO: f32 = 0.70;

/// Suffix patterns that signal Russian/Greek/Latin loanwords. Any word ending
/// in one of these (and longer than the suffix itself) fails the sample.
const LOANWORD_SUFFIXES: &[&str] = &[
    "ция",
    "ция,",
    "ция.",
    "ция)",
    "изм",
    "изм.",
    "изм,",
    "логия",
    "логии",
    "графия",
    "графии",
    "тика",
    "тикалық",
    "ивный",
    "ильный",
    "альный",
    "альная",
    "альное",
    "ональный",
];

/// Explicit loanword blocklist (common Russian/English borrowings observed
/// in CC-100 + Wikipedia that bypass the suffix heuristics).
const LOANWORD_BLOCKLIST: &[&str] = &[
    "интернет",
    "компьютер",
    "онлайн",
    "сайт",
    "телефон",
    "автобус",
    "трамвай",
    "автомобиль",
    "метро",
    "радио",
    "телевизор",
    "видео",
    "аудио",
    "маркетинг",
    "менеджмент",
    "бизнес",
    "офис",
    "банкомат",
    "компания",
    "компаниясы",
    "корпорация",
    "министрлік",
    "министрлігі",
    "департамент",
    "процент",
    "процесс",
    "система",
    "системасы",
    "проект",
    "проектіні",
    "университет",
    "университеті",
    "институт",
    "институты",
    "академия",
    "академиясы",
    "технология",
    "технологиясы",
    "энергия",
    "энергиясы",
    "программа",
    "программасы",
    "информация",
    "информациясы",
    "организация",
    "организациясы",
    "республика",
    "республикасы",
];

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Sample {
    #[serde(flatten)]
    extra: serde_json::Map<String, Value>,
}

fn main() -> ExitCode {
    let input_path: PathBuf = match env::args().nth(1) {
        Some(p) => PathBuf::from(p),
        None => {
            eprintln!(
                "usage: filter_pack <input_pack.json> [output_pack.json] [--min-words=N] [--fsm-ratio=R]"
            );
            return ExitCode::FAILURE;
        }
    };
    let output_path: PathBuf = env::args()
        .nth(2)
        .filter(|s| !s.starts_with("--"))
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let dir = input_path.parent().unwrap_or(std::path::Path::new("."));
            let fname = input_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("pack.json");
            dir.join(format!("filtered_{fname}"))
        });
    let min_words: usize = env::args()
        .find(|a| a.starts_with("--min-words="))
        .and_then(|a| a["--min-words=".len()..].parse().ok())
        .unwrap_or(DEFAULT_MIN_WORDS);
    let fsm_ratio: f32 = env::args()
        .find(|a| a.starts_with("--fsm-ratio="))
        .and_then(|a| a["--fsm-ratio=".len()..].parse().ok())
        .unwrap_or(DEFAULT_FSM_RATIO);

    // Load lexicon + rules
    let roots_raw = match fs::read_to_string(ROOTS_PATH) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot read {ROOTS_PATH}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let lexicon: SegmentationLexicon = match serde_json::from_str(&roots_raw) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("cannot parse lexicon: {e}");
            return ExitCode::FAILURE;
        }
    };
    let rules_raw = match fs::read_to_string(RULES_PATH) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot read {RULES_PATH}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let rules: SegmentationRuleSet = match serde_json::from_str(&rules_raw) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("cannot parse rules: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Load pack
    let pack_raw = match fs::read_to_string(&input_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot read {input_path:?}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let mut pack: Value = match serde_json::from_str(&pack_raw) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("cannot parse pack: {e}");
            return ExitCode::FAILURE;
        }
    };

    let samples_value = match pack.get_mut("samples").and_then(|v| v.as_array_mut()) {
        Some(arr) => arr,
        None => {
            eprintln!("pack has no `samples` array");
            return ExitCode::FAILURE;
        }
    };

    let original_count = samples_value.len();
    let mut skipped_length = 0usize;
    let mut skipped_coverage = 0usize;
    let mut skipped_loanword_suffix = 0usize;
    let mut skipped_loanword_blocklist = 0usize;
    let mut kept: Vec<Value> = Vec::with_capacity(original_count);

    for sample in samples_value.iter() {
        let text = match sample.get("text").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => continue,
        };

        let words: Vec<String> = text
            .split_whitespace()
            .map(|w| {
                w.trim_matches(|c: char| !c.is_alphabetic() && c != '-')
                    .to_lowercase()
            })
            .filter(|w| !w.is_empty())
            .collect();

        if words.len() < min_words {
            skipped_length += 1;
            continue;
        }

        // Loanword suffix check
        let has_loanword_suffix = words.iter().any(|w| {
            LOANWORD_SUFFIXES
                .iter()
                .any(|suf| w.len() > suf.len() && w.ends_with(suf))
        });
        if has_loanword_suffix {
            skipped_loanword_suffix += 1;
            continue;
        }

        // Blocklist check
        let has_blocklisted = words
            .iter()
            .any(|w| LOANWORD_BLOCKLIST.contains(&w.as_str()));
        if has_blocklisted {
            skipped_loanword_blocklist += 1;
            continue;
        }

        // FSM coverage check
        let segmentable = words
            .iter()
            .filter(|w| deterministic_segment_token(w, &lexicon, &rules).is_some())
            .count();
        let ratio = segmentable as f32 / words.len() as f32;
        if ratio < fsm_ratio {
            skipped_coverage += 1;
            continue;
        }

        kept.push(sample.clone());
    }

    // Replace samples and write.
    let kept_count = kept.len();
    pack["samples"] = Value::Array(kept);

    // Touch the name to signal it's a filtered pack (best-effort; preserves
    // original if missing). Update counts if present.
    if let Some(name_val) = pack.get_mut("name") {
        if let Some(original_name) = name_val.as_str() {
            if !original_name.starts_with("filtered-") {
                *name_val = Value::String(format!("filtered-{original_name}"));
            }
        }
    }
    if let Some(n) = pack.get_mut("sample_count") {
        *n = Value::Number(kept_count.into());
    }

    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            let _ = fs::create_dir_all(parent);
        }
    }
    if let Err(e) = fs::write(
        &output_path,
        serde_json::to_string_pretty(&pack).expect("serialize"),
    ) {
        eprintln!("cannot write {output_path:?}: {e}");
        return ExitCode::FAILURE;
    }

    let kept_ratio = kept_count as f32 / original_count.max(1) as f32;
    eprintln!(
        "{:?}: {} → {} ({:.1}%)  [length: -{}  coverage: -{}  loan-suf: -{}  loan-block: -{}]",
        input_path.file_name().unwrap_or_default(),
        original_count,
        kept_count,
        kept_ratio * 100.0,
        skipped_length,
        skipped_coverage,
        skipped_loanword_suffix,
        skipped_loanword_blocklist,
    );
    eprintln!("wrote {}", output_path.display());
    ExitCode::SUCCESS
}
