// Take the top-ranked root candidates from `abai_gap_candidates.json`
// (high-frequency Abai words missing from the pure-Kazakh lexicon) and
// classify them by POS, vowel harmony, and final sound class. Output the
// resulting entries to `data/lexicon_v1/abai_augmented_roots.json`.
//
// The `build_pure_kazakh_lexicon` binary can then union this file with the
// existing lexicon to produce an updated production lexicon.
//
// Classification heuristics:
//   - POS: inspect the example_forms the gap analysis captured.
//       inflected forms ending in -ды/-ді/-ып/-іп/-ған/-ген/-қан/-кен
//       → verb;  otherwise → noun  (a coarse default).
//   - Vowel harmony: majority of vowels in the root.
//   - Final sound class: last character of the root.
//
// The conservative choice is fine: POS mistakes here don't block the FST
// (they just mean a verb-stem is stored as noun and no verb suffixes
// attach; still usable for roundtrip / analysis tasks).

use std::{collections::HashSet, fs, process::ExitCode};

use serde::{Deserialize, Serialize};

const GAP_PATH: &str = "data/lexicon_v1/abai_gap_candidates.json";
const PURE_LEX: &str = "data/lexicon_v1/pure_kazakh_roots.json";
const OUT_PATH: &str = "data/lexicon_v1/abai_augmented_roots.json";

const FRONT_VOWELS: &[char] = &['ә', 'е', 'ө', 'і', 'ү', 'и', 'э'];
const BACK_VOWELS: &[char] = &['а', 'о', 'ы', 'ұ', 'у', 'я', 'ю', 'ё'];
const NASAL_CONSONANTS: &[char] = &['м', 'н', 'ң'];
const VOICELESS_CONSONANTS: &[char] = &['к', 'қ', 'п', 'с', 'т', 'ф', 'х', 'ш', 'щ', 'ц', 'ч', 'һ'];
const VOICED_CONSONANTS: &[char] = &['б', 'в', 'г', 'ғ', 'д', 'ж', 'з', 'й', 'л', 'р'];

const VERB_MARKERS: &[&str] = &[
    "ды", "ді", "ты", "ті", "ған", "ген", "қан", "кен", "ып", "іп", "ады", "еді",
];

#[derive(Debug, Clone, Deserialize)]
struct GapCandidate {
    root_guess: String,
    frequency: usize,
    example_forms: Vec<String>,
}
#[derive(Debug, Clone, Deserialize)]
struct GapReport {
    top_candidates: Vec<GapCandidate>,
}

#[derive(Debug, Clone, Deserialize)]
struct RootEntry {
    id: String,
    root: String,
    #[allow(dead_code)]
    part_of_speech: String,
    #[allow(dead_code)]
    vowel_harmony: String,
    #[allow(dead_code)]
    final_sound_class: String,
}
#[derive(Debug, Clone, Deserialize)]
struct RootsFile {
    roots: Vec<RootEntry>,
}

#[derive(Debug, Clone, Serialize)]
struct OutEntry {
    id: String,
    root: String,
    part_of_speech: String,
    vowel_harmony: String,
    final_sound_class: String,
}
#[derive(Debug, Serialize)]
struct OutFile {
    version: String,
    name: String,
    target_language: String,
    script: String,
    roots: Vec<OutEntry>,
}

fn main() -> ExitCode {
    let gap: GapReport = match fs::read_to_string(GAP_PATH)
        .map_err(|e| e.to_string())
        .and_then(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("gap: {e}");
            return ExitCode::FAILURE;
        }
    };
    let lex: RootsFile = match fs::read_to_string(PURE_LEX)
        .map_err(|e| e.to_string())
        .and_then(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("lex: {e}");
            return ExitCode::FAILURE;
        }
    };

    let existing: HashSet<String> = lex.roots.iter().map(|e| e.root.clone()).collect();
    let existing_ids: HashSet<String> = lex.roots.iter().map(|e| e.id.clone()).collect();

    let mut out_roots: Vec<OutEntry> = Vec::new();
    let mut seen_new: HashSet<String> = HashSet::new();

    let mut noun_count = 0;
    let mut verb_count = 0;

    for c in &gap.top_candidates {
        let root = c.root_guess.clone();
        if existing.contains(&root) {
            continue;
        }
        if seen_new.contains(&root) {
            continue;
        }
        if root.chars().count() < 3 {
            continue;
        }
        let pos = classify_pos(&c.example_forms);
        let (harmony, fsc) = classify_sounds(&root);

        let base_id = format!("{}_abai_{}", pos, transliterate(&root));
        let mut final_id = base_id.clone();
        let mut counter = 1;
        while existing_ids.contains(&final_id) || out_roots.iter().any(|e| e.id == final_id) {
            final_id = format!("{base_id}_{counter}");
            counter += 1;
        }

        match pos {
            "verb" => verb_count += 1,
            _ => noun_count += 1,
        }

        seen_new.insert(root.clone());
        out_roots.push(OutEntry {
            id: final_id,
            root,
            part_of_speech: pos.to_string(),
            vowel_harmony: harmony.to_string(),
            final_sound_class: fsc.to_string(),
        });
    }

    let out = OutFile {
        version: env!("CARGO_PKG_VERSION").to_string(),
        name: "adam-lexicon-v1-abai-augmented".to_string(),
        target_language: "kazakh".to_string(),
        script: "cyrillic".to_string(),
        roots: out_roots.clone(),
    };
    fs::write(OUT_PATH, serde_json::to_string_pretty(&out).unwrap()).unwrap();

    eprintln!("=== Abai-augmented root build ===");
    eprintln!("input candidates:    {}", gap.top_candidates.len());
    eprintln!("accepted (new):      {}", out_roots.len());
    eprintln!("  classified noun:   {noun_count}");
    eprintln!("  classified verb:   {verb_count}");
    eprintln!("wrote {OUT_PATH}");
    ExitCode::SUCCESS
}

fn classify_pos(example_forms: &[String]) -> &'static str {
    for f in example_forms {
        for m in VERB_MARKERS {
            if f.ends_with(m) {
                return "verb";
            }
        }
    }
    "noun"
}

fn classify_sounds(word: &str) -> (&'static str, &'static str) {
    let mut front = 0usize;
    let mut back = 0usize;
    for c in word.chars() {
        let l = c.to_lowercase().next().unwrap_or(c);
        if FRONT_VOWELS.contains(&l) {
            front += 1;
        } else if BACK_VOWELS.contains(&l) {
            back += 1;
        }
    }
    let harmony = if front >= back { "front" } else { "back" };
    let last = word.chars().last().unwrap_or('а');
    let fsc = if FRONT_VOWELS.contains(&last) || BACK_VOWELS.contains(&last) {
        "vowel"
    } else if NASAL_CONSONANTS.contains(&last) {
        "nasal"
    } else if VOICELESS_CONSONANTS.contains(&last) {
        "voiceless_consonant"
    } else if VOICED_CONSONANTS.contains(&last) {
        "voiced_consonant"
    } else {
        "voiced_consonant"
    };
    (harmony, fsc)
}

fn transliterate(word: &str) -> String {
    let mut out = String::with_capacity(word.len());
    for c in word.chars() {
        let l = c.to_lowercase().next().unwrap_or(c);
        let m: &str = match l {
            'а' => "a",
            'ә' => "ae",
            'б' => "b",
            'в' => "v",
            'г' => "g",
            'ғ' => "gh",
            'д' => "d",
            'е' => "e",
            'ж' => "j",
            'з' => "z",
            'и' => "i",
            'й' => "y",
            'к' => "k",
            'қ' => "q",
            'л' => "l",
            'м' => "m",
            'н' => "n",
            'ң' => "ng",
            'о' => "o",
            'ө' => "oe",
            'п' => "p",
            'р' => "r",
            'с' => "s",
            'т' => "t",
            'у' => "u",
            'ұ' => "uu",
            'ү' => "ue",
            'х' => "kh",
            'һ' => "h",
            'ш' => "sh",
            'ы' => "y",
            'і' => "ih",
            '-' => "_",
            _ => "",
        };
        out.push_str(m);
    }
    if out.is_empty() {
        out.push_str("x");
    }
    out
}
