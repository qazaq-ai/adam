// Extract root-candidate words from the committed source packs to inform
// lexicon expansion (211 → ~5000). Runs over all pack JSONs, tokenizes
// words, applies morphological suffix stripping, and produces a frequency-
// ranked list of candidate roots.
//
// Pipeline:
//   1. Load all source packs.
//   2. Split each sample into words (Cyrillic letters + dashes).
//   3. Reject words containing non-Kazakh-Cyrillic letters (ё, ъ alone,
//      Latin, digits) — these are loanword/noise signals.
//   4. Strip known Kazakh suffixes greedily to find probable root.
//   5. Count root frequencies across the whole corpus.
//   6. Emit top N to stdout as JSON.
//
// The stripped roots are CANDIDATES only — they still need automated POS
// classification + manual review before entering segmentation_roots.json.
// The purpose of this binary is to produce the candidate pool.

use std::{
    collections::HashMap,
    env,
    fs,
    path::PathBuf,
    process::ExitCode,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

const KAZAKH_SPECIFIC: &[char] = &['ә', 'ғ', 'қ', 'ң', 'ө', 'ұ', 'ү', 'һ', 'і'];
const CYR_LOWER_START: u32 = 0x0430; // а
const CYR_LOWER_END: u32 = 0x045F; // ...
const KAZAKH_ALLOWED: &[char] = &[
    'а', 'ә', 'б', 'в', 'г', 'ғ', 'д', 'е', 'ж', 'з', 'и', 'й', 'к', 'қ',
    'л', 'м', 'н', 'ң', 'о', 'ө', 'п', 'р', 'с', 'т', 'у', 'ұ', 'ү', 'ф',
    'х', 'һ', 'ц', 'ч', 'ш', 'щ', 'ы', 'і', 'э', 'ю', 'я', 'ь', 'ъ', 'ё',
    '-',
];

// Kazakh noun/verb suffixes in rough longest-first order for greedy strip.
// Stripping these recovers an approximate root; for non-obvious words the
// stripped form may be the root or a stem.
const SUFFIXES: &[&str] = &[
    // plural + case combos
    "лардың", "лердің", "дардың", "дердің", "тардың", "тердің",
    "ларым", "лерім", "дарым", "дерім", "тарым", "терім",
    "ларың", "лерің", "дарың", "дерің", "тарың", "терің",
    "ларын", "лерін", "дарын", "дерін", "тарын", "терін",
    "ларға", "лерге", "дарға", "дерге", "тарға", "терге",
    "лардан", "лерден", "дардан", "дерден", "тардан", "терден",
    "ларда", "лерде", "дарда", "дерде", "тарда", "терде",
    "ларды", "лерді", "дарды", "дерді", "тарды", "терді",
    // plural
    "лар", "лер", "дар", "дер", "тар", "тер",
    // case suffixes (any vowel harmony)
    "дегі", "тегі", "ндегі", "ндегі",
    "ның", "нің", "дың", "дің", "тың", "тің",
    "ға", "ге", "қа", "ке", "на", "не",
    "ды", "ді", "ты", "ті", "ны", "ні",
    "да", "де", "та", "те",
    "дан", "ден", "тан", "тен", "нан", "нен",
    "мен", "бен", "пен",
    // possessive
    "ым", "ім", "ың", "ің", "ысы", "ісі",
    "сы", "сі", "ы", "і",
    // verb personal endings (simple present/past)
    "йды", "йді", "ады", "еді", "ды", "ді", "ты", "ті",
    "мын", "мін", "сың", "сің", "мыз", "міз", "сыз", "сіз",
    "ған", "ген", "қан", "кен",
    "ып", "іп", "п",
    "у",
];

#[derive(Debug, Deserialize)]
struct PackSample {
    text: String,
}

#[derive(Debug, Deserialize)]
struct Pack {
    samples: Vec<PackSample>,
}

#[derive(Debug, Serialize)]
struct RootCandidate {
    root: String,
    frequency: usize,
    example_forms: Vec<String>,
}

#[derive(Debug, Serialize)]
struct Report {
    version: String,
    total_tokens_scanned: usize,
    unique_roots_found: usize,
    top_n_emitted: usize,
    kazakh_specific_letter_ratio: f32,
    candidates: Vec<RootCandidate>,
}

fn main() -> ExitCode {
    let packs: Vec<&str> = env::args()
        .skip(1)
        .filter(|a| !a.starts_with("--"))
        .collect::<Vec<_>>()
        .iter()
        .map(|s| Box::leak(s.clone().into_boxed_str()) as &str)
        .collect();

    let top_n: usize = env::args()
        .find(|a| a.starts_with("--top="))
        .and_then(|a| a["--top=".len()..].parse().ok())
        .unwrap_or(5000);

    let output_path: PathBuf = env::args()
        .find(|a| a.starts_with("--out="))
        .map(|a| PathBuf::from(&a["--out=".len()..]))
        .unwrap_or_else(|| PathBuf::from("data/curated/root_candidates_report.json"));

    let pack_paths: Vec<PathBuf> = if packs.is_empty() {
        vec![
            PathBuf::from("data/curated/abai_wikisource_pack.json"),
            PathBuf::from("data/curated/wikipedia_kz_pack.json"),
            PathBuf::from("data/curated/cc100_kk_pack.json"),
            PathBuf::from("data/curated/common_voice_kk_pack.json"),
            PathBuf::from("data/curated/tatoeba_kazakh_pack.json"),
            PathBuf::from("data/curated/kazakh_proverbs_pack.json"),
            PathBuf::from("data/curated/clean_training_corpus_pack.json"),
        ]
    } else {
        packs.iter().map(PathBuf::from).collect()
    };

    let mut root_freq: HashMap<String, usize> = HashMap::new();
    let mut root_examples: HashMap<String, Vec<String>> = HashMap::new();
    let mut total_words = 0usize;
    let mut kaz_specific_words = 0usize;

    for path in &pack_paths {
        eprintln!("scanning {:?}", path);
        let raw = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("  skip (read error: {e})");
                continue;
            }
        };
        let pack: Pack = match serde_json::from_str::<Value>(&raw).and_then(|v| serde_json::from_value(v)) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("  skip (parse error: {e})");
                continue;
            }
        };
        for sample in pack.samples {
            for word in split_words(&sample.text) {
                total_words += 1;
                if !is_pure_kazakh(&word) {
                    continue;
                }
                if word.chars().any(|c| KAZAKH_SPECIFIC.contains(&c)) {
                    kaz_specific_words += 1;
                }
                let root = strip_suffix_greedy(&word);
                if root.chars().count() < 3 {
                    continue;
                }
                *root_freq.entry(root.clone()).or_insert(0) += 1;
                let ex = root_examples.entry(root).or_insert_with(Vec::new);
                if ex.len() < 5 && !ex.contains(&word) {
                    ex.push(word);
                }
            }
        }
    }

    let mut ranked: Vec<(String, usize)> = root_freq.iter().map(|(k, v)| (k.clone(), *v)).collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ranked.truncate(top_n);

    let candidates: Vec<RootCandidate> = ranked
        .iter()
        .map(|(root, freq)| RootCandidate {
            root: root.clone(),
            frequency: *freq,
            example_forms: root_examples.get(root).cloned().unwrap_or_default(),
        })
        .collect();
    let candidates_count = candidates.len();
    let unique_roots = root_freq.len();
    let kaz_ratio = if total_words > 0 {
        kaz_specific_words as f32 / total_words as f32
    } else {
        0.0
    };

    let report = Report {
        version: env!("CARGO_PKG_VERSION").to_string(),
        total_tokens_scanned: total_words,
        unique_roots_found: unique_roots,
        top_n_emitted: candidates_count,
        kazakh_specific_letter_ratio: kaz_ratio,
        candidates,
    };

    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            let _ = fs::create_dir_all(parent);
        }
    }
    if let Err(e) = fs::write(&output_path, serde_json::to_string_pretty(&report).expect("serialize")) {
        eprintln!("write error: {e}");
        return ExitCode::FAILURE;
    }

    eprintln!(
        "scanned {} words, {} unique roots, emitted top {}, kaz-specific ratio {:.1}%",
        total_words,
        unique_roots,
        candidates_count,
        kaz_ratio * 100.0,
    );
    eprintln!("wrote {}", output_path.display());
    ExitCode::SUCCESS
}

fn split_words(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut cur = String::new();
    for ch in text.chars() {
        let lower = ch.to_lowercase().next().unwrap_or(ch);
        if lower.is_alphabetic() || lower == '-' {
            cur.push(lower);
        } else {
            if !cur.is_empty() {
                words.push(std::mem::take(&mut cur));
            }
        }
    }
    if !cur.is_empty() {
        words.push(cur);
    }
    words
}

fn is_pure_kazakh(word: &str) -> bool {
    if word.chars().count() < 3 {
        return false;
    }
    for ch in word.chars() {
        if ch == '-' {
            continue;
        }
        if !KAZAKH_ALLOWED.contains(&ch) {
            return false;
        }
    }
    // Require the word to contain at least one "real Kazakh" letter set member
    // (excluding bare ъ, ь which also appear in Russian). This filters out
    // pure-Russian words that happen to use the subset.
    word.chars().any(|c| {
        let is_cyrillic = (c as u32) >= CYR_LOWER_START && (c as u32) <= CYR_LOWER_END;
        is_cyrillic && c != 'ъ' && c != 'ь'
    })
}

fn strip_suffix_greedy(word: &str) -> String {
    let lower = word.to_lowercase();
    for suffix in SUFFIXES {
        if lower.ends_with(suffix) {
            let stripped = &lower[..lower.len() - suffix.len()];
            if stripped.chars().count() >= 2 {
                return stripped.to_string();
            }
        }
    }
    lower
}
