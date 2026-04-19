// Produce a "pure pre-modern Kazakh" lexicon by filtering v0.4.0 curated
// and v0.4.5 Apertium-imported entries through strict purity rules.
//
// An entry is ACCEPTED when all of these hold:
//   1. No Russian-only letter (ё, ф, ц, ч, щ, ъ, ь, э)
//   2. No loanword suffix (-ция, -изм, -лог, -тика, -метр, …)
//   3. Either a Kazakh-specific letter (ә, ғ, қ, ң, ө, ұ, ү, һ, і)
//      is present OR the word is attested as a prefix in the Abai corpus
//      (classical 19th-century Kazakh).
//
// Output:
//   data/lexicon_v1/pure_kazakh_roots.json  — the accepted entries
//   data/lexicon_v1/dropped_loanwords.json  — the rejected entries (audit)
//
// Abai coverage report (how many Abai word forms are reachable from
// the filtered lexicon via prefix match) is printed to stderr.

use std::{
    collections::{HashMap, HashSet},
    fs,
    process::ExitCode,
};

use serde::{Deserialize, Serialize};

const CURATED: &str = "data/tokenizer/segmentation_roots.json";
const APERTIUM: &str = "data/lexicon_v1/apertium_imported_roots.json";
const ABAI_PACK: &str = "data/curated/abai_wikisource_pack.json";
const OUT_ROOTS: &str = "data/lexicon_v1/pure_kazakh_roots.json";
const OUT_DROPPED: &str = "data/lexicon_v1/dropped_loanwords.json";

const RUSSIAN_ONLY: &[char] = &['ё', 'ф', 'ц', 'ч', 'щ', 'ъ', 'ь', 'э'];
const KAZAKH_SPECIFIC: &[char] = &['ә', 'ғ', 'қ', 'ң', 'ө', 'ұ', 'ү', 'һ', 'і'];

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

#[derive(Debug, Clone, Deserialize, Serialize)]
struct RootsFile {
    version: String,
    name: String,
    target_language: String,
    script: String,
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

#[derive(Debug, Clone, Serialize)]
struct FilterReason {
    root: String,
    part_of_speech: String,
    reasons: Vec<String>,
}

#[derive(Debug, Serialize)]
struct DroppedFile {
    total_dropped: usize,
    by_reason: HashMap<String, usize>,
    entries: Vec<FilterReason>,
}

fn main() -> ExitCode {
    let curated: RootsFile = match read_json(CURATED) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let apertium: RootsFile = match read_json(APERTIUM) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let abai: Pack = match read_json(ABAI_PACK) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    // Abai vocabulary (word forms)
    let mut abai_vocab: HashSet<String> = HashSet::new();
    for s in &abai.samples {
        for w in s.text.split_whitespace() {
            let c: String = w
                .chars()
                .filter(|c| c.is_alphabetic() || *c == '-')
                .collect::<String>()
                .to_lowercase();
            if !c.is_empty() {
                abai_vocab.insert(c);
            }
        }
    }

    let mut all = curated.roots.clone();
    all.extend(apertium.roots.clone());

    let mut kept: Vec<RootEntry> = Vec::new();
    let mut dropped: Vec<FilterReason> = Vec::new();
    let mut by_reason: HashMap<String, usize> = HashMap::new();
    let mut by_pos_kept: HashMap<String, usize> = HashMap::new();

    for e in &all {
        let mut reasons: Vec<String> = Vec::new();
        if e.root.chars().any(|c| RUSSIAN_ONLY.contains(&c)) {
            reasons.push("russian_only_letter".to_string());
        }
        if LOANWORD_SUFFIXES
            .iter()
            .any(|s| e.root.len() > s.len() && e.root.ends_with(s))
        {
            reasons.push("loanword_suffix".to_string());
        }
        let has_specific = e.root.chars().any(|c| KAZAKH_SPECIFIC.contains(&c));
        let attested = abai_vocab
            .iter()
            .any(|w| w == &e.root || w.starts_with(&e.root));
        if !has_specific && !attested && e.root.chars().count() >= 4 {
            reasons.push("no_kazakh_signal".to_string());
        }

        if reasons.is_empty() {
            kept.push(e.clone());
            *by_pos_kept.entry(e.part_of_speech.clone()).or_insert(0) += 1;
        } else {
            for r in &reasons {
                *by_reason.entry(r.clone()).or_insert(0) += 1;
            }
            dropped.push(FilterReason {
                root: e.root.clone(),
                part_of_speech: e.part_of_speech.clone(),
                reasons,
            });
        }
    }

    // Dedupe kept on (surface, POS).
    let mut seen: HashSet<(String, String)> = HashSet::new();
    kept.retain(|e| seen.insert((e.root.clone(), e.part_of_speech.clone())));

    let out = RootsFile {
        version: env!("CARGO_PKG_VERSION").to_string(),
        name: "adam-lexicon-v1-pure-kazakh".to_string(),
        target_language: "kazakh".to_string(),
        script: "cyrillic".to_string(),
        roots: kept.clone(),
    };

    fs::write(OUT_ROOTS, serde_json::to_string_pretty(&out).unwrap()).unwrap();
    fs::write(
        OUT_DROPPED,
        serde_json::to_string_pretty(&DroppedFile {
            total_dropped: dropped.len(),
            by_reason: by_reason.clone(),
            entries: dropped.clone(),
        })
        .unwrap(),
    )
    .unwrap();

    // Coverage: how many unique Abai word forms have a prefix match in the
    // kept lexicon? Gives us a concrete missing-vocabulary signal.
    let kept_surfaces: Vec<String> = kept.iter().map(|e| e.root.clone()).collect();
    let mut abai_hits = 0usize;
    for w in &abai_vocab {
        if kept_surfaces.iter().any(|r| w.starts_with(r)) {
            abai_hits += 1;
        }
    }

    eprintln!("=== pure Kazakh lexicon build ===");
    eprintln!("input entries:   {}", all.len());
    eprintln!("accepted:        {}", kept.len());
    eprintln!("dropped:         {}", dropped.len());
    for (r, n) in &by_reason {
        eprintln!("  reason {r:<24}: {n}");
    }
    eprintln!();
    eprintln!("by POS (accepted):");
    let mut pos_rows: Vec<(String, usize)> = by_pos_kept.into_iter().collect();
    pos_rows.sort_by(|a, b| b.1.cmp(&a.1));
    for (pos, n) in pos_rows {
        eprintln!("  {pos:<14} {n}");
    }
    eprintln!();
    eprintln!(
        "Abai corpus coverage: {}/{} word forms reachable via prefix match ({:.1}%)",
        abai_hits,
        abai_vocab.len(),
        100.0 * abai_hits as f64 / abai_vocab.len() as f64
    );
    eprintln!("wrote {OUT_ROOTS}");
    eprintln!("wrote {OUT_DROPPED}");
    ExitCode::SUCCESS
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let raw = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}
