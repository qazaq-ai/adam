// Build an extended segmentation_roots.json from root_candidates_report.json.
//
// Strategy (minimal first pass):
//   * Keep all existing curated roots (211 entries) exactly as-is.
//   * For each new candidate (not already curated):
//     - Determine vowel_harmony from vowel classes present
//     - Determine final_sound_class from last character
//     - Default POS to Noun (conservative — to be refined in a later pass)
//     - Skip if root is too short or looks over-stripped
//     - Skip if root has no Kazakh-specific letter AND length >= 5 (loanword)
//
// Output: data/tokenizer/segmentation_roots.json (replaces existing)
//         Plus: data/curated/lexicon_expansion_report.json (audit trail)
//
// The point of this minimal pass is to MEASURE whether lexicon expansion
// moves the needle on FSM coverage. If coverage jumps significantly, we
// iterate on POS heuristics. If it doesn't, we pivot.

use std::{collections::HashSet, env, fs, process::ExitCode};

use serde::{Deserialize, Serialize};

const CANDIDATES_PATH: &str = "data/curated/root_candidates_report.json";
const EXISTING_ROOTS_PATH: &str = "data/tokenizer/segmentation_roots.json";
const OUTPUT_ROOTS_PATH: &str = "data/tokenizer/segmentation_roots.json";
const REPORT_PATH: &str = "data/curated/lexicon_expansion_report.json";

const KAZAKH_SPECIFIC: &[char] = &['ә', 'ғ', 'қ', 'ң', 'ө', 'ұ', 'ү', 'һ', 'і'];

// Vowel classification for Kazakh vowel harmony.
// Front: ә, е, ө, і, ү + borrowed и, э. Back: а, о, ы, ұ, у + borrowed я, ю.
const FRONT_VOWELS: &[char] = &['ә', 'е', 'ө', 'і', 'ү', 'и', 'э'];
const BACK_VOWELS: &[char] = &['а', 'о', 'ы', 'ұ', 'у', 'я', 'ю', 'ё'];

const NASAL_CONSONANTS: &[char] = &['м', 'н', 'ң'];
const VOICELESS_CONSONANTS: &[char] = &['к', 'қ', 'п', 'с', 'т', 'ф', 'х', 'ш', 'щ', 'ц', 'ч', 'һ'];
const VOICED_CONSONANTS: &[char] = &['б', 'в', 'г', 'ғ', 'д', 'ж', 'з', 'й', 'л', 'р'];

#[derive(Debug, Deserialize, Serialize, Clone)]
struct RootEntry {
    id: String,
    root: String,
    part_of_speech: String,
    vowel_harmony: String,
    final_sound_class: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct RootsFile {
    version: String,
    name: String,
    target_language: String,
    script: String,
    roots: Vec<RootEntry>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    root: String,
    frequency: usize,
}

#[derive(Debug, Deserialize)]
struct CandidatesReport {
    candidates: Vec<Candidate>,
}

#[derive(Debug, Serialize)]
struct ExpansionReport {
    version: String,
    curated_root_count: usize,
    candidate_pool_size: usize,
    added_count: usize,
    skipped_already_curated: usize,
    skipped_over_stripped: usize,
    skipped_loanword: usize,
    skipped_no_vowel: usize,
    final_root_count: usize,
    min_candidate_frequency: usize,
    max_candidate_frequency: usize,
}

fn main() -> ExitCode {
    let target_n: usize = env::args()
        .find(|a| a.starts_with("--target="))
        .and_then(|a| a["--target=".len()..].parse().ok())
        .unwrap_or(3000);

    // Load existing curated lexicon
    let existing_raw = match fs::read_to_string(EXISTING_ROOTS_PATH) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot read {EXISTING_ROOTS_PATH}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let existing: RootsFile = match serde_json::from_str(&existing_raw) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("cannot parse existing roots: {e}");
            return ExitCode::FAILURE;
        }
    };
    let curated_count = existing.roots.len();
    let curated_surfaces: HashSet<String> = existing.roots.iter().map(|r| r.root.clone()).collect();
    let curated_ids: HashSet<String> = existing.roots.iter().map(|r| r.id.clone()).collect();

    // Load candidates
    let cand_raw = match fs::read_to_string(CANDIDATES_PATH) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot read {CANDIDATES_PATH}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let cand_report: CandidatesReport = match serde_json::from_str(&cand_raw) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("cannot parse candidates: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mut new_roots: Vec<RootEntry> = Vec::new();
    let mut skipped_already = 0usize;
    let mut skipped_over_strip = 0usize;
    let mut skipped_loanword = 0usize;
    let mut skipped_no_vowel = 0usize;
    let mut min_freq = usize::MAX;
    let mut max_freq = 0usize;
    let candidate_pool_size = cand_report.candidates.len();

    for cand in cand_report.candidates.iter().take(target_n * 2) {
        // try up to 2× target to compensate for skips
        if new_roots.len() >= target_n {
            break;
        }
        if curated_surfaces.contains(&cand.root) {
            skipped_already += 1;
            continue;
        }
        if cand.root.chars().count() < 3 {
            skipped_over_strip += 1;
            continue;
        }
        // Loanword heuristic: no Kazakh-specific letter AND length >= 5.
        // Short common words (кел, бар) often lack specific letters but are native.
        let has_kz_specific = cand.root.chars().any(|c| KAZAKH_SPECIFIC.contains(&c));
        if !has_kz_specific && cand.root.chars().count() >= 5 {
            skipped_loanword += 1;
            continue;
        }
        // Determine vowel harmony. If no vowels at all, skip (likely garbage).
        let (front_count, back_count) = count_vowels(&cand.root);
        if front_count == 0 && back_count == 0 {
            skipped_no_vowel += 1;
            continue;
        }
        let harmony = if front_count >= back_count {
            "front"
        } else {
            "back"
        };
        // Determine final sound class from last char.
        let last_char = cand.root.chars().last().unwrap_or('а');
        let fsc = classify_final(last_char);

        // Build a unique ID. Some roots produce colliding IDs (e.g. identical
        // transliteration after diacritic strip); append freq-rank suffix.
        let mut base_id = format!("noun_{}", transliterate(&cand.root));
        if curated_ids.contains(&base_id) {
            base_id.push_str(&format!("_auto{}", new_roots.len() + 1));
        }
        // Also guard against collisions with previously-added auto entries
        while new_roots.iter().any(|r| r.id == base_id) {
            base_id.push('_');
            base_id.push_str(&(new_roots.len() + 1).to_string());
        }

        new_roots.push(RootEntry {
            id: base_id,
            root: cand.root.clone(),
            part_of_speech: "noun".to_string(),
            vowel_harmony: harmony.to_string(),
            final_sound_class: fsc.to_string(),
        });

        if cand.frequency < min_freq {
            min_freq = cand.frequency;
        }
        if cand.frequency > max_freq {
            max_freq = cand.frequency;
        }
    }

    // Merge: preserve curated order, then append new
    let added_count = new_roots.len();
    let mut all_roots = existing.roots.clone();
    all_roots.extend(new_roots);
    let final_count = all_roots.len();

    let output = RootsFile {
        version: existing.version.clone(),
        name: existing.name.clone(),
        target_language: existing.target_language.clone(),
        script: existing.script.clone(),
        roots: all_roots,
    };

    if let Err(e) = fs::write(
        OUTPUT_ROOTS_PATH,
        serde_json::to_string_pretty(&output).expect("serialize roots"),
    ) {
        eprintln!("write roots error: {e}");
        return ExitCode::FAILURE;
    }

    let report = ExpansionReport {
        version: env!("CARGO_PKG_VERSION").to_string(),
        curated_root_count: curated_count,
        candidate_pool_size,
        added_count,
        skipped_already_curated: skipped_already,
        skipped_over_stripped: skipped_over_strip,
        skipped_loanword,
        skipped_no_vowel,
        final_root_count: final_count,
        min_candidate_frequency: if min_freq == usize::MAX { 0 } else { min_freq },
        max_candidate_frequency: max_freq,
    };
    if let Err(e) = fs::write(
        REPORT_PATH,
        serde_json::to_string_pretty(&report).expect("serialize report"),
    ) {
        eprintln!("write report error: {e}");
        return ExitCode::FAILURE;
    }

    eprintln!("curated: {curated_count}");
    eprintln!("candidate pool: {candidate_pool_size}");
    eprintln!("added: {added_count}");
    eprintln!("  skipped already curated: {skipped_already}");
    eprintln!("  skipped over-stripped: {skipped_over_strip}");
    eprintln!("  skipped loanword: {skipped_loanword}");
    eprintln!("  skipped no vowel: {skipped_no_vowel}");
    eprintln!("final: {final_count}");
    eprintln!("wrote {OUTPUT_ROOTS_PATH}");
    eprintln!("wrote {REPORT_PATH}");
    ExitCode::SUCCESS
}

fn count_vowels(word: &str) -> (usize, usize) {
    let mut front = 0usize;
    let mut back = 0usize;
    for c in word.chars() {
        let lower = c.to_lowercase().next().unwrap_or(c);
        if FRONT_VOWELS.contains(&lower) {
            front += 1;
        } else if BACK_VOWELS.contains(&lower) {
            back += 1;
        }
    }
    (front, back)
}

fn classify_final(c: char) -> &'static str {
    let lower = c.to_lowercase().next().unwrap_or(c);
    if FRONT_VOWELS.contains(&lower) || BACK_VOWELS.contains(&lower) {
        "vowel"
    } else if NASAL_CONSONANTS.contains(&lower) {
        "nasal"
    } else if VOICELESS_CONSONANTS.contains(&lower) {
        "voiceless_consonant"
    } else if VOICED_CONSONANTS.contains(&lower) {
        "voiced_consonant"
    } else {
        // Fallback: ь, ъ, or unusual chars — treat as voiced so FSM still tries.
        "voiced_consonant"
    }
}

/// Best-effort ASCII transliteration for use in IDs. Only needs to be stable
/// and mostly unique across the candidate pool.
fn transliterate(word: &str) -> String {
    let mut out = String::with_capacity(word.len());
    for c in word.chars() {
        let lower = c.to_lowercase().next().unwrap_or(c);
        let mapped: &str = match lower {
            'а' => "a",
            'ә' => "ae",
            'б' => "b",
            'в' => "v",
            'г' => "g",
            'ғ' => "gh",
            'д' => "d",
            'е' => "e",
            'ё' => "yo",
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
            'ф' => "f",
            'х' => "kh",
            'һ' => "h",
            'ц' => "ts",
            'ч' => "ch",
            'ш' => "sh",
            'щ' => "sch",
            'ъ' => "",
            'ы' => "y",
            'і' => "ih",
            'ь' => "",
            'э' => "e",
            'ю' => "yu",
            'я' => "ya",
            '-' => "_",
            _ => "",
        };
        out.push_str(mapped);
    }
    if out.is_empty() {
        out.push_str("x");
    }
    out
}
