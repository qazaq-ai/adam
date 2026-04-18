// Build an extended segmentation_roots.json from root_candidates_report.json.
//
// v2 strategy (POS-aware + FSM-filter):
//   * Keep all curated roots (211 entries) exactly as-is.
//   * For each new candidate:
//     1. If the existing FSM (curated lexicon + rules) can already segment the
//        candidate's root form, skip it — it's an inflected form of a known
//        root, not a new root. This is the single most important filter; it
//        prevents the regression we hit when qарай/келе/жазып were added as
//        nouns and shadowed the existing qара/кел/жаз verbs.
//     2. Classify POS from example_forms:
//          - Verb if any form ends in ды/ді/ты/ті/ған/ген/қан/кен/ып/іп/ады/еді
//          - Adjective if any form ends in лы/лі/дай/дей/рақ/рек
//          - Default Noun otherwise
//     3. Compute vowel_harmony (deterministic from vowel classes).
//     4. Compute final_sound_class (deterministic from last character).
//     5. Skip obvious garbage (length < 3, no vowels, loanword signal).

use std::{collections::HashSet, env, fs, process::ExitCode};

use adam_kernel::{SegmentationLexicon, SegmentationRuleSet, deterministic_segment_token};
use serde::{Deserialize, Serialize};

const CANDIDATES_PATH: &str = "data/curated/root_candidates_report.json";
const EXISTING_ROOTS_PATH: &str = "data/tokenizer/segmentation_roots.json";
const RULES_PATH: &str = "data/tokenizer/segmentation_rules.json";
const OUTPUT_ROOTS_PATH: &str = "data/tokenizer/segmentation_roots.json";
const REPORT_PATH: &str = "data/curated/lexicon_expansion_report.json";

const KAZAKH_SPECIFIC: &[char] = &['ә', 'ғ', 'қ', 'ң', 'ө', 'ұ', 'ү', 'һ', 'і'];

const FRONT_VOWELS: &[char] = &['ә', 'е', 'ө', 'і', 'ү', 'и', 'э'];
const BACK_VOWELS: &[char] = &['а', 'о', 'ы', 'ұ', 'у', 'я', 'ю', 'ё'];

const NASAL_CONSONANTS: &[char] = &['м', 'н', 'ң'];
const VOICELESS_CONSONANTS: &[char] = &['к', 'қ', 'п', 'с', 'т', 'ф', 'х', 'ш', 'щ', 'ц', 'ч', 'һ'];
const VOICED_CONSONANTS: &[char] = &['б', 'в', 'г', 'ғ', 'д', 'ж', 'з', 'й', 'л', 'р'];

/// Verb-signature suffixes on example_forms. Presence of any of these on a
/// non-root form is strong evidence the candidate is a verb.
const VERB_SUFFIX_MARKERS: &[&str] = &[
    "ды",
    "ді",
    "ты",
    "ті",
    "ған",
    "ген",
    "қан",
    "кен",
    "ып",
    "іп",
    "ады",
    "еді",
    "майды",
    "мейді",
];

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
    example_forms: Vec<String>,
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
    skipped_already_segmentable: usize,
    skipped_loanword: usize,
    skipped_no_vowel: usize,
    skipped_too_short: usize,
    classified_as_noun: usize,
    classified_as_verb: usize,
    classified_as_adjective: usize,
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

    // Load lexicon + rules through kernel types for FSM-based filter
    let kernel_lexicon: SegmentationLexicon = match serde_json::from_str(&existing_raw) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("cannot parse lexicon as kernel SegmentationLexicon: {e}");
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
    let kernel_rules: SegmentationRuleSet = match serde_json::from_str(&rules_raw) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("cannot parse rules: {e}");
            return ExitCode::FAILURE;
        }
    };

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
    let mut skipped_segmentable = 0usize;
    let mut skipped_loanword = 0usize;
    let mut skipped_no_vowel = 0usize;
    let mut skipped_too_short = 0usize;
    let mut classified_noun = 0usize;
    let mut classified_verb = 0usize;
    let mut classified_adj = 0usize;
    let mut min_freq = usize::MAX;
    let mut max_freq = 0usize;
    let candidate_pool_size = cand_report.candidates.len();

    for cand in cand_report.candidates.iter() {
        if new_roots.len() >= target_n {
            break;
        }
        if curated_surfaces.contains(&cand.root) {
            skipped_already += 1;
            continue;
        }
        if cand.root.chars().count() < 3 {
            skipped_too_short += 1;
            continue;
        }
        // Filter #1a (critical): if the existing FSM can already segment this
        // candidate, it's not a new root — it's an inflected form.
        if deterministic_segment_token(&cand.root, &kernel_lexicon, &kernel_rules).is_some() {
            skipped_segmentable += 1;
            continue;
        }
        // Filter #1b (mutual-prefix): even if FSM can't segment candidate, a
        // curated root may still conflict via surface-prefix overlap. Two
        // failure modes observed:
        //   - "алд" candidate (over-stripped from ал+ды) shadows "ал" verb on
        //     "алды" segmentation: curated root is prefix of candidate.
        //   - "арқы" candidate (over-stripped from арқылы) shadows "арқылы"
        //     postposition: candidate is prefix of curated root.
        // Skip both directions.
        let candidate_conflicts = curated_surfaces.iter().any(|c| {
            c != &cand.root
                && (cand.root.starts_with(c.as_str()) || c.starts_with(cand.root.as_str()))
        });
        if candidate_conflicts {
            skipped_segmentable += 1;
            continue;
        }
        // Filter #2: loanword signal — no Kazakh-specific letters AND length ≥ 5.
        let has_kz_specific = cand.root.chars().any(|c| KAZAKH_SPECIFIC.contains(&c));
        if !has_kz_specific && cand.root.chars().count() >= 5 {
            skipped_loanword += 1;
            continue;
        }
        // Filter #3: no vowels at all = garbage.
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
        let last_char = cand.root.chars().last().unwrap_or('а');
        let fsc = classify_final(last_char);
        let pos = classify_pos(&cand.root, &cand.example_forms);
        match pos {
            "verb" => classified_verb += 1,
            "adjective" => classified_adj += 1,
            _ => classified_noun += 1,
        }

        // Unique id
        let mut base_id = format!("{}_{}", pos, transliterate(&cand.root));
        if curated_ids.contains(&base_id) || new_roots.iter().any(|r| r.id == base_id) {
            base_id.push_str(&format!("_auto{}", new_roots.len() + 1));
        }

        new_roots.push(RootEntry {
            id: base_id,
            root: cand.root.clone(),
            part_of_speech: pos.to_string(),
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
        skipped_already_segmentable: skipped_segmentable,
        skipped_loanword,
        skipped_no_vowel,
        skipped_too_short,
        classified_as_noun: classified_noun,
        classified_as_verb: classified_verb,
        classified_as_adjective: classified_adj,
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

    eprintln!("curated:                {curated_count}");
    eprintln!("candidate pool:         {candidate_pool_size}");
    eprintln!("added:                  {added_count}");
    eprintln!("  as noun:              {classified_noun}");
    eprintln!("  as verb:              {classified_verb}");
    eprintln!("  as adjective:         {classified_adj}");
    eprintln!("skipped already curated: {skipped_already}");
    eprintln!("skipped already segmentable (FSM-derivable): {skipped_segmentable}");
    eprintln!("skipped loanword:       {skipped_loanword}");
    eprintln!("skipped no-vowel:       {skipped_no_vowel}");
    eprintln!("skipped too short:      {skipped_too_short}");
    eprintln!("final roots:            {final_count}");
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
        "voiced_consonant"
    }
}

/// Classify POS from example_forms.
///
/// Rule order:
/// 1. Strong verb signal: a non-root form ends in a verb inflection suffix
///    (-ды/-ді/-ған/-ген/-ып/-іп/-ады/-еді).
/// 2. Strong adjective signal: a non-root form ends in a derivational adj
///    suffix (-дай/-дей/-рақ/-рек).
/// 3. Fall through to noun.
fn classify_pos(root: &str, example_forms: &[String]) -> &'static str {
    // Collect forms that are strictly longer than the root (inflected).
    let inflected: Vec<&String> = example_forms.iter().filter(|f| *f != root).collect();

    if inflected.is_empty() {
        // No observed inflection — defaulting noun is safe enough.
        return "noun";
    }

    // Verb check first (strongest signal, more distinctive suffixes).
    for form in &inflected {
        for marker in VERB_SUFFIX_MARKERS {
            if form.ends_with(marker) {
                // But guard: -ды/-ді are also noun accusative / ablative.
                // We need a stronger test. If root doesn't end in vowel AND form
                // ends -ып/-іп/-ған/-ген/-қан/-кен — those are unambiguously
                // verb converb/past-participle markers.
                if *marker == "ып"
                    || *marker == "іп"
                    || *marker == "ған"
                    || *marker == "ген"
                    || *marker == "қан"
                    || *marker == "кен"
                    || *marker == "ады"
                    || *marker == "еді"
                    || *marker == "майды"
                    || *marker == "мейді"
                {
                    return "verb";
                }
                // Weaker markers (-ды/-ді/-ты/-ті) need corroborating evidence.
                // If ANOTHER form shows a strong verb marker, we already returned.
                // If the only evidence is -ды, keep looking for stronger marker
                // among other forms.
            }
        }
    }
    // Second pass: strong adj markers.
    for form in &inflected {
        for marker in &["дай", "дей", "рақ", "рек"] {
            if form.ends_with(marker) {
                return "adjective";
            }
        }
    }
    // Weak adj: -лы/-лі is derivational ("with-X"). Check this separately from
    // nouns where -лы could mark a derived adjective form.
    for form in &inflected {
        if (form.ends_with("лы") || form.ends_with("лі"))
            && form.chars().count() == root.chars().count() + 2
        {
            return "adjective";
        }
    }

    "noun"
}

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
