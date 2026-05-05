// Import Apertium-kaz vocabulary into our segmentation_roots.json schema.
//
// Source:
//   data/external/apertium/apertium-kaz/tests/vocabulary/input.csv
//   27,758 entries in tab-separated format:
//     surface_form \t lemma \t apertium_class \t ; ! comment
//
// Apertium → adam POS mapping:
//   N1, N5, N1-NAT, N-COMPOUND-PX       → Noun
//   A1, A2, A3, A4                      → Adjective
//   V-TV, V-IV, V-TD, Vinfl-AUX         → Verb
//   ADV, ADV-LANG, ADV-WITH-KI, ADV-ITG → Adverb
//   NUM                                 → Numeral
//   POST, POST-NOM, POST-DAT            → Postposition
//   DET-*, PRON-*                       → Pronoun (we use Pronoun for both)
//   CC-SOYED, CA                        → Conjunction
//   everything else                     → skip (interjection, ideophone, ...)
//
// Filters applied during import:
//   1. Skip if lemma already exists in curated segmentation_roots.json
//   2. Skip if mutual-prefix conflict with any existing curated root
//      (surface-prefix of or prefix-of) — same filter that keeps
//      segmentation tests green for v0.5.x.
//   3. Skip if no Kazakh-specific letter AND length ≥ 5 (loanword signal).

use std::{collections::HashSet, env, fs, process::ExitCode};

use adam_kernel::{SegmentationLexicon, SegmentationRuleSet, deterministic_segment_token};
use serde::{Deserialize, Serialize};

const INPUT_CSV: &str = "data/external/apertium/apertium-kaz/tests/vocabulary/input.csv";
// Reads the v0.4.0 curated base (211 entries) to seed dedup / prefix-conflict
// filtering, but writes imports to the v1.0.0 lexicon path. This keeps the
// v0.4.0 segmentation_roots.json untouched (CI segmentation tests depend on
// exactly those 211 entries and their interaction with 422 curated rules).
// The v1.0.0 FST will consume data/lexicon_v1/ directly.
const EXISTING_ROOTS: &str = "data/tokenizer/segmentation_roots.json";
const OUTPUT_ROOTS: &str = "data/lexicon_v1/apertium_imported_roots.json";
const RULES_PATH: &str = "data/tokenizer/segmentation_rules.json";
const REPORT_PATH: &str = "data/lexicon_v1/apertium_import_report.json";

const KAZAKH_SPECIFIC: &[char] = &['ә', 'ғ', 'қ', 'ң', 'ө', 'ұ', 'ү', 'һ', 'і'];
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

#[derive(Debug, Serialize)]
struct ImportReport {
    source: String,
    input_rows: usize,
    added: usize,
    skipped_duplicate_lemma: usize,
    skipped_prefix_conflict: usize,
    skipped_loanword: usize,
    skipped_fsm_derivable: usize,
    skipped_unknown_pos: usize,
    skipped_malformed: usize,
    by_pos_noun: usize,
    by_pos_verb: usize,
    by_pos_adjective: usize,
    by_pos_adverb: usize,
    by_pos_numeral: usize,
    by_pos_postposition: usize,
    by_pos_pronoun: usize,
    by_pos_conjunction: usize,
    final_root_count: usize,
}

fn main() -> ExitCode {
    // Load existing
    let existing_raw = match fs::read_to_string(EXISTING_ROOTS) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("read {EXISTING_ROOTS}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let existing: RootsFile = match serde_json::from_str(&existing_raw) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("parse existing: {e}");
            return ExitCode::FAILURE;
        }
    };
    let curated_count = existing.roots.len();
    let curated_surfaces: HashSet<String> = existing.roots.iter().map(|r| r.root.clone()).collect();
    let curated_ids: HashSet<String> = existing.roots.iter().map(|r| r.id.clone()).collect();

    let kernel_lex: SegmentationLexicon = match serde_json::from_str(&existing_raw) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("kernel lex: {e}");
            return ExitCode::FAILURE;
        }
    };
    let rules_raw = match fs::read_to_string(RULES_PATH) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("read rules: {e}");
            return ExitCode::FAILURE;
        }
    };
    let kernel_rules: SegmentationRuleSet = match serde_json::from_str(&rules_raw) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("parse rules: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Read CSV
    let csv_raw = match fs::read_to_string(INPUT_CSV) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("read {INPUT_CSV}: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mut new_roots: Vec<RootEntry> = Vec::new();
    let mut seen_new: HashSet<String> = HashSet::new();
    let mut r = ImportReport {
        source: "apertium-kaz tests/vocabulary/input.csv".to_string(),
        input_rows: 0,
        added: 0,
        skipped_duplicate_lemma: 0,
        skipped_prefix_conflict: 0,
        skipped_loanword: 0,
        skipped_fsm_derivable: 0,
        skipped_unknown_pos: 0,
        skipped_malformed: 0,
        by_pos_noun: 0,
        by_pos_verb: 0,
        by_pos_adjective: 0,
        by_pos_adverb: 0,
        by_pos_numeral: 0,
        by_pos_postposition: 0,
        by_pos_pronoun: 0,
        by_pos_conjunction: 0,
        final_root_count: 0,
    };

    let target_n: usize = env::args()
        .find(|a| a.starts_with("--target="))
        .and_then(|a| a["--target=".len()..].parse().ok())
        .unwrap_or(usize::MAX);

    for line in csv_raw.lines() {
        r.input_rows += 1;
        if new_roots.len() >= target_n {
            break;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 3 {
            r.skipped_malformed += 1;
            continue;
        }
        let lemma = parts[1].trim();
        let pos_raw = parts[2].trim().trim_end_matches(';').trim();
        let pos = match map_pos(pos_raw) {
            Some(p) => p,
            None => {
                r.skipped_unknown_pos += 1;
                continue;
            }
        };
        if lemma.is_empty() || lemma.chars().count() < 2 {
            r.skipped_malformed += 1;
            continue;
        }
        if curated_surfaces.contains(lemma) {
            r.skipped_duplicate_lemma += 1;
            continue;
        }
        if seen_new.contains(lemma) {
            r.skipped_duplicate_lemma += 1;
            continue;
        }
        let has_kz_specific = lemma.chars().any(|c| KAZAKH_SPECIFIC.contains(&c));
        if !has_kz_specific && lemma.chars().count() >= 5 {
            r.skipped_loanword += 1;
            continue;
        }
        if deterministic_segment_token(lemma, &kernel_lex, &kernel_rules).is_some() {
            r.skipped_fsm_derivable += 1;
            continue;
        }
        let has_prefix_conflict = curated_surfaces
            .iter()
            .any(|c| c != lemma && (lemma.starts_with(c.as_str()) || c.starts_with(lemma)));
        if has_prefix_conflict {
            r.skipped_prefix_conflict += 1;
            continue;
        }

        let (harmony, fsc) = classify(lemma);
        let mut base_id = format!("{}_apt_{}", pos, transliterate(lemma));
        let mut dedup = 1;
        while curated_ids.contains(&base_id) || new_roots.iter().any(|r| r.id == base_id) {
            base_id = format!("{}_apt_{}_{}", pos, transliterate(lemma), dedup);
            dedup += 1;
        }

        match pos {
            "noun" => r.by_pos_noun += 1,
            "verb" => r.by_pos_verb += 1,
            "adjective" => r.by_pos_adjective += 1,
            "adverb" => r.by_pos_adverb += 1,
            "numeral" => r.by_pos_numeral += 1,
            "postposition" => r.by_pos_postposition += 1,
            "pronoun" => r.by_pos_pronoun += 1,
            "conjunction" => r.by_pos_conjunction += 1,
            _ => {}
        }

        seen_new.insert(lemma.to_string());
        new_roots.push(RootEntry {
            id: base_id,
            root: lemma.to_string(),
            part_of_speech: pos.to_string(),
            vowel_harmony: harmony.to_string(),
            final_sound_class: fsc.to_string(),
        });
        r.added += 1;
    }

    // Output: only the new entries. v0.4.0 curated roots stay where they are;
    // the v1.0.0 FST will union both sources at load time.
    r.final_root_count = new_roots.len();

    let out = RootsFile {
        version: existing.version.clone(),
        name: "adam-lexicon-v1-apertium-imported".to_string(),
        target_language: existing.target_language.clone(),
        script: existing.script.clone(),
        roots: new_roots,
    };
    if let Some(parent) = std::path::Path::new(OUTPUT_ROOTS).parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            let _ = fs::create_dir_all(parent);
        }
    }
    if let Err(e) = fs::write(
        OUTPUT_ROOTS,
        serde_json::to_string_pretty(&out).expect("serialize"),
    ) {
        eprintln!("write roots: {e}");
        return ExitCode::FAILURE;
    }
    if let Err(e) = fs::write(
        REPORT_PATH,
        serde_json::to_string_pretty(&r).expect("serialize report"),
    ) {
        eprintln!("write report: {e}");
        return ExitCode::FAILURE;
    }

    eprintln!("curated:              {curated_count}");
    eprintln!("input rows:           {}", r.input_rows);
    eprintln!("added:                {}", r.added);
    eprintln!("  noun:               {}", r.by_pos_noun);
    eprintln!("  verb:               {}", r.by_pos_verb);
    eprintln!("  adjective:          {}", r.by_pos_adjective);
    eprintln!("  adverb:             {}", r.by_pos_adverb);
    eprintln!("  numeral:            {}", r.by_pos_numeral);
    eprintln!("  postposition:       {}", r.by_pos_postposition);
    eprintln!("  pronoun:            {}", r.by_pos_pronoun);
    eprintln!("  conjunction:        {}", r.by_pos_conjunction);
    eprintln!("skipped dup lemma:    {}", r.skipped_duplicate_lemma);
    eprintln!("skipped prefix conf:  {}", r.skipped_prefix_conflict);
    eprintln!("skipped FSM-derivable:{}", r.skipped_fsm_derivable);
    eprintln!("skipped loanword:     {}", r.skipped_loanword);
    eprintln!("skipped unknown pos:  {}", r.skipped_unknown_pos);
    eprintln!("skipped malformed:    {}", r.skipped_malformed);
    eprintln!("final root count:     {}", r.final_root_count);
    ExitCode::SUCCESS
}

fn map_pos(code: &str) -> Option<&'static str> {
    // Strip trailing comment chars if present
    let c = code.split_whitespace().next().unwrap_or(code);
    match c {
        "N1" | "N5" | "N1-NAT" | "N-COMPOUND-PX" => Some("noun"),
        "A1" | "A2" | "A3" | "A4" => Some("adjective"),
        "V-TV" | "V-IV" | "V-TD" | "Vinfl-AUX" => Some("verb"),
        "ADV" | "ADV-LANG" | "ADV-WITH-KI" | "ADV-ITG" => Some("adverb"),
        "NUM" => Some("numeral"),
        "POST" | "POST-NOM" | "POST-DAT" | "POST-GEN" | "POST-ABL" | "POST-LOC" => {
            Some("postposition")
        }
        "DET-QNT" | "DET-DEM" | "DET-IND" | "DET" | "PRON-IND" | "PRON-DEM" | "PRON-PERS" => {
            Some("pronoun")
        }
        "CC-SOYED" | "CA" | "CNJCOO" | "CNJSUB" | "CNJADV" => Some("conjunction"),
        _ => None,
    }
}

fn classify(word: &str) -> (&'static str, &'static str) {
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
            'ы' => "y",
            'і' => "ih",
            'э' => "e",
            'ю' => "yu",
            'я' => "ya",
            '-' => "_",
            _ => "",
        };
        out.push_str(m);
    }
    if out.is_empty() {
        out.push('x');
    }
    out
}
