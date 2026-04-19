// Find Abai word forms NOT reachable by prefix match from the pure
// Kazakh lexicon. For each such word, greedy-strip suffixes to guess its
// root, and emit a ranked candidate list.
//
// These candidates are the concrete "missing classical roots" that would
// lift our Abai coverage from 88.8% toward 100%.

use std::{
    collections::{HashMap, HashSet},
    fs,
    process::ExitCode,
};

use serde::{Deserialize, Serialize};

const LEXICON: &str = "data/lexicon_v1/pure_kazakh_roots.json";
const ABAI_PACK: &str = "data/curated/abai_wikisource_pack.json";
const OUT_PATH: &str = "data/lexicon_v1/abai_gap_candidates.json";

/// Greedy suffix list ordered longest-first.  Matches standard Kazakh
/// inflection morphemes.  Not exhaustive but covers the most frequent
/// forms we expect in Abai's poetic/philosophical corpus.
const SUFFIXES: &[&str] = &[
    // plural + case combos (longest)
    "лардың",
    "лердің",
    "дардың",
    "дердің",
    "тардың",
    "тердің",
    "ларың",
    "лерің",
    "дарың",
    "дерің",
    "тарың",
    "терің",
    "ларын",
    "лерін",
    "дарын",
    "дерін",
    "тарын",
    "терін",
    "ларға",
    "лерге",
    "дарға",
    "дерге",
    "тарға",
    "терге",
    // plural markers
    "лар",
    "лер",
    "дар",
    "дер",
    "тар",
    "тер",
    // case endings
    "дың",
    "дің",
    "тың",
    "тің",
    "ның",
    "нің",
    "ған",
    "ген",
    "қан",
    "кен",
    "ады",
    "еді",
    "ды",
    "ді",
    "ты",
    "ті",
    "ған",
    "ген",
    "ға",
    "ге",
    "қа",
    "ке",
    "дан",
    "ден",
    "тан",
    "тен",
    "нан",
    "нен",
    "да",
    "де",
    "та",
    "те",
    "нда",
    "нде",
    "нан",
    "нен",
    "мен",
    "бен",
    "пен",
    "сың",
    "сің",
    "мын",
    "мін",
    "мыз",
    "міз",
    "сыз",
    "сіз",
    "ңыз",
    "ңіз",
    "ңдар",
    "ңдер",
    "ып",
    "іп",
    "п",
    "атын",
    "етін",
    "йтын",
    "йтін",
    "тын",
    "тін",
    "ар",
    "ер",
    "ышы",
    "ішы",
    "ым",
    "ім",
    "ың",
    "ің",
    "ы",
    "і",
    "м",
    "ң",
    "қ",
    "к",
    "у",
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
struct PackSample {
    text: String,
}
#[derive(Debug, Clone, Deserialize)]
struct Pack {
    samples: Vec<PackSample>,
}

#[derive(Debug, Serialize)]
struct GapCandidate {
    root_guess: String,
    frequency: usize,
    example_forms: Vec<String>,
}

#[derive(Debug, Serialize)]
struct Report {
    abai_total_words: usize,
    abai_covered_by_lexicon: usize,
    abai_uncovered: usize,
    unique_candidate_roots: usize,
    top_candidates: Vec<GapCandidate>,
}

fn main() -> ExitCode {
    let lex: RootsFile = match fs::read_to_string(LEXICON)
        .map_err(|e| e.to_string())
        .and_then(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("lexicon: {e}");
            return ExitCode::FAILURE;
        }
    };
    let abai: Pack = match fs::read_to_string(ABAI_PACK)
        .map_err(|e| e.to_string())
        .and_then(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
    {
        Ok(p) => p,
        Err(e) => {
            eprintln!("abai: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Collect Abai word frequencies.
    let mut abai_word_freq: HashMap<String, usize> = HashMap::new();
    let mut total_tokens = 0usize;
    for s in &abai.samples {
        for w in s.text.split_whitespace() {
            total_tokens += 1;
            let c: String = w
                .chars()
                .filter(|c| c.is_alphabetic() || *c == '-')
                .collect::<String>()
                .to_lowercase();
            if c.chars().count() >= 2 {
                *abai_word_freq.entry(c).or_insert(0) += 1;
            }
        }
    }

    let lex_roots: Vec<String> = lex.roots.iter().map(|e| e.root.clone()).collect();
    let lex_set: HashSet<String> = lex_roots.iter().cloned().collect();

    let mut covered = 0usize;
    let mut uncovered_words: Vec<(String, usize)> = Vec::new();
    for (word, freq) in &abai_word_freq {
        let matched = lex_roots.iter().any(|r| word.starts_with(r));
        if matched {
            covered += 1;
        } else {
            uncovered_words.push((word.clone(), *freq));
        }
    }
    uncovered_words.sort_by(|a, b| b.1.cmp(&a.1));

    // Greedy-strip suffixes from each uncovered word to guess its root.
    let mut root_freq: HashMap<String, (usize, Vec<String>)> = HashMap::new();
    for (word, freq) in &uncovered_words {
        let root = strip_suffix_greedy(word);
        if root.chars().count() < 3 {
            continue;
        }
        if lex_set.contains(&root) {
            continue; // stripping found an existing root
        }
        let entry = root_freq.entry(root.clone()).or_insert((0, Vec::new()));
        entry.0 += freq;
        if entry.1.len() < 4 && !entry.1.contains(word) {
            entry.1.push(word.clone());
        }
    }

    let mut ranked: Vec<(String, (usize, Vec<String>))> = root_freq.into_iter().collect();
    ranked.sort_by(|a, b| b.1.0.cmp(&a.1.0));

    let top: Vec<GapCandidate> = ranked
        .iter()
        .take(500)
        .map(|(root, (freq, forms))| GapCandidate {
            root_guess: root.clone(),
            frequency: *freq,
            example_forms: forms.clone(),
        })
        .collect();

    let report = Report {
        abai_total_words: abai_word_freq.len(),
        abai_covered_by_lexicon: covered,
        abai_uncovered: uncovered_words.len(),
        unique_candidate_roots: ranked.len(),
        top_candidates: top,
    };
    fs::write(OUT_PATH, serde_json::to_string_pretty(&report).unwrap()).unwrap();

    eprintln!("=== Abai gap analysis ===");
    eprintln!(
        "Abai total tokens:     {total_tokens} (unique: {})",
        abai_word_freq.len()
    );
    eprintln!("covered by lexicon:    {}", report.abai_covered_by_lexicon);
    eprintln!("uncovered word forms:  {}", report.abai_uncovered);
    eprintln!(
        "unique candidate roots: {} (after suffix-strip + lex-overlap dedup)",
        report.unique_candidate_roots
    );
    eprintln!();
    eprintln!("top 25 candidates (ranked by corpus frequency):");
    for c in report.top_candidates.iter().take(25) {
        eprintln!(
            "  {:>5}  {:<18} [{}]",
            c.frequency,
            c.root_guess,
            c.example_forms.join(", ")
        );
    }
    eprintln!();
    eprintln!("wrote {OUT_PATH}");
    ExitCode::SUCCESS
}

fn strip_suffix_greedy(word: &str) -> String {
    for suf in SUFFIXES {
        if word.len() > suf.len() && word.ends_with(suf) {
            let stripped = &word[..word.len() - suf.len()];
            if stripped.chars().count() >= 3 {
                return stripped.to_string();
            }
        }
    }
    word.to_string()
}
