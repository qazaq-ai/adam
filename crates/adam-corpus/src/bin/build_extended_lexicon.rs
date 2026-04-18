// Build an extended segmentation_roots.json from root_candidates_report.json.
//
// v3 strategy (LCP + POS-aware mutual-prefix):
//   * Keep all curated roots exactly as-is.
//   * For each new candidate:
//     1. Recompute root via longest-common-prefix of example_forms. The
//        extractor's greedy suffix-strip sometimes leaves a suffix on the
//        root (e.g. "қарай" instead of "қара" because "-й" isn't in the
//        strip list). LCP gives us the true stem.
//     2. If the FSM can segment the LCP root, skip (already derivable).
//     3. POS classification from example_forms inflection markers.
//     4. POS-aware mutual-prefix: skip only if a curated root with the SAME
//        POS has prefix overlap. Different POS can coexist (verb ал + noun
//        алғашқы do not conflict in segmentation).
//     5. Vowel harmony / final sound class / loanword / length filters.

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
        // Step 1: recompute root via longest-common-prefix of example_forms.
        // The extractor's suffix-strip can leave residual suffix characters.
        let lcp_root = longest_common_prefix(&cand.root, &cand.example_forms);
        if lcp_root.chars().count() < 3 {
            skipped_too_short += 1;
            continue;
        }
        if curated_surfaces.contains(&lcp_root) {
            skipped_already += 1;
            continue;
        }
        // Dedup new roots: multiple candidates (e.g. кел and келе) can LCP to
        // the same root (кел); keep only the first occurrence.
        if new_roots.iter().any(|r| r.root == lcp_root) {
            skipped_already += 1;
            continue;
        }
        // Filter #1: FSM already segments it — skip.
        if deterministic_segment_token(&lcp_root, &kernel_lexicon, &kernel_rules).is_some() {
            skipped_segmentable += 1;
            continue;
        }
        // Step 2: POS classification from example_forms.
        let pos = classify_pos(&lcp_root, &cand.example_forms);

        // Filter #1b (mutual-prefix over ALL curated POS): any curated root
        // that is a surface-prefix of candidate (or vice versa) can break
        // segmentation regardless of POS, because the longest-match root
        // lookup in the FSM path is POS-agnostic. POS-aware relaxation
        // regressed 442/464. Revert to "any POS" check — safer with LCP fix.
        let has_prefix_conflict = curated_surfaces.iter().any(|c| {
            c != &lcp_root && (lcp_root.starts_with(c.as_str()) || c.starts_with(lcp_root.as_str()))
        });
        if has_prefix_conflict {
            skipped_segmentable += 1;
            continue;
        }
        // Filter #2: loanword signal — no Kazakh-specific letters AND length ≥ 5.
        let has_kz_specific = lcp_root.chars().any(|c| KAZAKH_SPECIFIC.contains(&c));
        if !has_kz_specific && lcp_root.chars().count() >= 5 {
            skipped_loanword += 1;
            continue;
        }
        // Filter #3: no vowels at all = garbage.
        let (front_count, back_count) = count_vowels(&lcp_root);
        if front_count == 0 && back_count == 0 {
            skipped_no_vowel += 1;
            continue;
        }

        let harmony = if front_count >= back_count {
            "front"
        } else {
            "back"
        };
        let last_char = lcp_root.chars().last().unwrap_or('а');
        let fsc = classify_final(last_char);
        match pos {
            "verb" => classified_verb += 1,
            "adjective" => classified_adj += 1,
            _ => classified_noun += 1,
        }

        // Unique id
        let mut base_id = format!("{}_{}", pos, transliterate(&lcp_root));
        if curated_ids.contains(&base_id) || new_roots.iter().any(|r| r.id == base_id) {
            base_id.push_str(&format!("_auto{}", new_roots.len() + 1));
        }

        new_roots.push(RootEntry {
            id: base_id,
            root: lcp_root.clone(),
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

/// Compute the longest prefix common to `candidate_root` and every form in
/// `example_forms`. If the candidate is already a prefix of all forms, we keep
/// it. Otherwise, we trim it down to what actually matches the evidence. This
/// corrects the "greedy suffix strip" over-stripping (and, symmetrically, the
/// under-stripping when the extractor's suffix list doesn't cover a form).
fn longest_common_prefix(candidate_root: &str, example_forms: &[String]) -> String {
    if example_forms.is_empty() {
        return candidate_root.to_string();
    }
    // Start with the candidate as the initial guess.
    let mut prefix_chars: Vec<char> = candidate_root.chars().collect();
    for form in example_forms {
        let form_chars: Vec<char> = form.chars().collect();
        let common_len = prefix_chars
            .iter()
            .zip(form_chars.iter())
            .take_while(|(a, b)| a == b)
            .count();
        prefix_chars.truncate(common_len);
        if prefix_chars.is_empty() {
            break;
        }
    }
    prefix_chars.iter().collect()
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
