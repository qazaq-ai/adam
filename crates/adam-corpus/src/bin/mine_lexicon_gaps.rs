//! `mine_lexicon_gaps` — v3.4.0 Lexicon expansion pipeline.
//!
//! ## What it does
//!
//! Scans the committed corpus (all source packs under `data/curated/`) for
//! tokens that **no current Lexicon root prefixes**. Aggregates across
//! every pack, ranks by frequency, picks the top-`--top` candidates,
//! extracts context sentences, and auto-tags each with:
//!
//!   - **Vowel harmony** — `back` / `front` / `mixed` inferred from the
//!     vowels present in the surface form. Kazakh back vowels:
//!     `а, о, ұ, ы` plus `и` when it surfaces after a back syllable.
//!     Front vowels: `ә, ө, ү, і, е`. A token with both → `mixed`
//!     (reviewer attention signal).
//!   - **Final-sound class** — last character → one of `vowel`,
//!     `voiceless_consonant`, `voiced_consonant`, `nasal`, `liquid`,
//!     `glide`. Matches the FST's `ConsonantClass` enum.
//!   - **POS (default `noun`)** — reviewer confirms or corrects.
//!     Auto-POS inference is deferred to a later release — wrong
//!     guesses are worse than a flagged default.
//!
//! The output `docs/lexicon_gap_candidates.md` is a **native-speaker
//! review file**: one candidate per section, checkbox to approve, slots
//! for reviewer-supplied root form + POS + harmony + final-sound
//! overrides, and a Tally section at the bottom so the reviewer can
//! record the approval rate.
//!
//! ## Why this is the v3.4.0 priority
//!
//! Per memory `project_morpheme_coverage_baseline`: current coverage is
//! 79.48 % on 3.84 M committed words. Every approved root added to the
//! Lexicon directly improves that ratio, which directly improves every
//! downstream stage (parser analysis → matcher firing → fact count →
//! scaling-law signal). The bottleneck is not tooling — it's
//! native-speaker review time. This binary converts that review from
//! "1 hour / root" into "1 hour / ~50 pre-tagged candidates".
//!
//! ## Usage
//!
//! ```
//! cargo run --release -p adam-corpus --bin mine_lexicon_gaps
//!   # default: top 200 candidates from committed corpus
//! cargo run --release -p adam-corpus --bin mine_lexicon_gaps -- \
//!   --top 500 --contexts-per-candidate 5
//! ```
//!
//! No network. No external deps beyond the existing adam-corpus tree.
//! Deterministic (same corpus + same Lexicon → byte-identical output).

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use serde::Deserialize;

const CURATED_DIR: &str = "data/curated";
const CURATED_ROOTS: &str = "data/tokenizer/segmentation_roots.json";
const APERTIUM_ROOTS: &str = "data/lexicon_v1/apertium_imported_roots.json";
const OUT_PATH: &str = "docs/lexicon_gap_candidates.md";

const DEFAULT_TOP: usize = 200;
const DEFAULT_CONTEXTS_PER_CANDIDATE: usize = 3;
const MIN_TOKEN_LEN: usize = 3;
const MIN_ROOT_LEN: usize = 3;

/// Canonical pack list — same order as `morpheme_coverage`.
const SOURCE_PACKS: &[&str] = &[
    "tatoeba_kazakh_pack.json",
    "wikipedia_kz_pack.json",
    "common_voice_kk_pack.json",
    "cc100_kk_pack.json",
    "abai_wikisource_pack.json",
    "kazakh_proverbs_pack.json",
    "synthetic_sentences_pack.json",
    "kazakh_classics_pack.json",
    "kazakh_textbooks_pack.json",
    // v4.7.1 — Rust Book Kazakh translation pack.
    "rust_book_kk_pack.json",
];

#[derive(Debug, Deserialize)]
struct PackFile {
    samples: Vec<Sample>,
}

#[derive(Debug, Deserialize)]
struct Sample {
    id: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct RootsFile {
    roots: Vec<RootEntry>,
}

#[derive(Debug, Deserialize)]
struct RootEntry {
    root: String,
    /// `noun` / `verb` / `pronoun` / `particle` / `conjunction` /
    /// `numeral` / `postposition` etc. Used to special-case the
    /// closed-class short-root paradigm: pronouns like `ол` (2
    /// chars) are real Kazakh roots whose inflected surfaces are
    /// covered by the FST `pronoun_paradigm` table but would be
    /// dropped by the global `MIN_ROOT_LEN = 3` filter.
    #[serde(default)]
    part_of_speech: String,
}

/// One uncovered token candidate aggregated across all source packs.
/// `surface` + `frequency` live in the `ranked` Vec (so we can preserve
/// frequency-desc display order); this struct only tracks the per-
/// candidate contexts that are populated in pass 2.
struct Candidate {
    contexts: Vec<Context>,
}

struct Context {
    pack: String,
    sample_id: String,
    text: String,
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let top = parse_usize(&args, "--top").unwrap_or(DEFAULT_TOP);
    let contexts_per =
        parse_usize(&args, "--contexts-per-candidate").unwrap_or(DEFAULT_CONTEXTS_PER_CANDIDATE);

    let roots = match load_roots() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("cannot load Lexicon roots: {e}");
            return ExitCode::FAILURE;
        }
    };
    // load_roots() prints its own diagnostics including the
    // short-closed-class count.

    // Pass 1: count uncovered token frequencies across all packs.
    // Pass 2: collect first-N contexts for each top-frequency candidate.
    // Doing it in two passes lets us keep the contexts Vec capped at
    // contexts_per, without retaining all sample texts in memory.
    let mut freq: HashMap<String, usize> = HashMap::new();
    let mut packs_loaded = 0usize;
    let mut total_tokens = 0usize;
    let mut total_samples = 0usize;

    for pack_name in SOURCE_PACKS {
        let path = Path::new(CURATED_DIR).join(pack_name);
        let Ok(pack) = load_pack(&path) else {
            eprintln!("skipping {} (missing or malformed)", path.display());
            continue;
        };
        packs_loaded += 1;
        for s in &pack.samples {
            total_samples += 1;
            for word in s.text.split_whitespace() {
                let cleaned = normalise(word);
                if cleaned.chars().count() < MIN_TOKEN_LEN {
                    continue;
                }
                total_tokens += 1;
                if has_known_prefix(&cleaned, &roots) {
                    continue;
                }
                *freq.entry(cleaned).or_insert(0) += 1;
            }
        }
    }
    eprintln!(
        "mine_lexicon_gaps: scanned {packs_loaded} packs, {total_samples} samples, {total_tokens} tokens → {} distinct uncovered surfaces",
        freq.len()
    );

    // Rank uncovered tokens by frequency, tie-break alphabetically.
    let mut ranked: Vec<(String, usize)> = freq.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    ranked.truncate(top);

    // Pass 2: collect contexts. Build a lookup of target surfaces so
    // one corpus walk populates all candidates.
    let target: HashSet<String> = ranked.iter().map(|(s, _)| s.clone()).collect();
    let mut candidates: BTreeMap<String, Candidate> = ranked
        .iter()
        .map(|(s, _)| {
            (
                s.clone(),
                Candidate {
                    contexts: Vec::with_capacity(contexts_per),
                },
            )
        })
        .collect();

    for pack_name in SOURCE_PACKS {
        let path = Path::new(CURATED_DIR).join(pack_name);
        let Ok(pack) = load_pack(&path) else {
            continue;
        };
        for s in &pack.samples {
            let cleaned_tokens: Vec<String> = s.text.split_whitespace().map(normalise).collect();
            for tok in &cleaned_tokens {
                if !target.contains(tok) {
                    continue;
                }
                if let Some(c) = candidates.get_mut(tok) {
                    if c.contexts.len() < contexts_per {
                        c.contexts.push(Context {
                            pack: pack_name.to_string(),
                            sample_id: s.id.clone(),
                            text: s.text.clone(),
                        });
                    }
                }
            }
        }
    }

    // Render. ranked is the display order (frequency desc).
    let md = render_markdown(
        &ranked,
        &candidates,
        packs_loaded,
        total_samples,
        total_tokens,
    );
    if let Some(parent) = Path::new(OUT_PATH).parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                eprintln!("cannot create {}: {e}", parent.display());
                return ExitCode::FAILURE;
            }
        }
    }
    if let Err(e) = fs::write(OUT_PATH, md) {
        eprintln!("cannot write {OUT_PATH}: {e}");
        return ExitCode::FAILURE;
    }
    eprintln!(
        "mine_lexicon_gaps: wrote {OUT_PATH} — {} candidates, {contexts_per} contexts/candidate",
        ranked.len()
    );
    ExitCode::SUCCESS
}

fn render_markdown(
    ranked: &[(String, usize)],
    candidates: &BTreeMap<String, Candidate>,
    packs_loaded: usize,
    total_samples: usize,
    total_tokens: usize,
) -> String {
    let mut out = String::new();
    out.push_str("# Lexicon gap candidates — v3.4.0 mining pass\n\n");
    out.push_str(&format!(
        "**Scan**: {packs_loaded} committed source packs, {total_samples} samples, {total_tokens} tokens (≥ {MIN_TOKEN_LEN} chars alphabetic). **Candidates**: top {} most-frequent surfaces that no current Lexicon root prefixes.\n\n",
        ranked.len(),
    ));
    out.push_str("## How to review\n\n");
    out.push_str("Each candidate lists the observed surface form, its frequency, 3 sample sentences, and **auto-tagged features** (vowel harmony + final-sound class). Your job:\n\n");
    out.push_str("1. Decide the canonical **root form** — the surface may be inflected; the root is the bare nominative / infinitive.\n");
    out.push_str("2. Confirm the **POS**: `noun`, `verb`, `adjective`, `adverb`, `pronoun`, `numeral`, `conjunction`, `particle`, `postposition`.\n");
    out.push_str("3. Verify or correct the auto-tags.\n");
    out.push_str("4. **Reject** loanwords, proper nouns, OCR artefacts, and anything not a real Kazakh root.\n");
    out.push_str("5. Update the **Tally** section at the bottom with approve / reject counts.\n\n");
    out.push_str("Approved roots get added via a Lexicon PR against `data/tokenizer/segmentation_roots.json`. Re-run `cargo run --release -p adam-corpus --bin morpheme_coverage` after the PR to measure the coverage delta — per memory `project_morpheme_coverage_baseline`, every Lexicon PR must do this.\n\n");
    out.push_str("---\n\n");

    for (i, (surface, freq)) in ranked.iter().enumerate() {
        let Some(c) = candidates.get(surface) else {
            continue;
        };
        let harmony = infer_vowel_harmony(surface);
        let final_sound = infer_final_sound(surface);
        out.push_str(&format!(
            "### Candidate #{} — `{surface}` (freq {freq})\n\n",
            i + 1
        ));
        out.push_str(&format!(
            "- Vowel harmony (auto): **{harmony}**\n- Final sound (auto): **{final_sound}**\n- POS (default): **noun** — confirm or correct\n",
        ));
        if !c.contexts.is_empty() {
            out.push_str("- Contexts:\n");
            for (ci, ctx) in c.contexts.iter().enumerate() {
                out.push_str(&format!(
                    "  {}. ({} / `{}`) «{}»\n",
                    ci + 1,
                    ctx.pack,
                    ctx.sample_id,
                    truncate_to_display(&ctx.text, 200),
                ));
            }
        }
        out.push_str("\n- [ ] Approved\n- Root form: __\n- POS: __\n- Harmony override: __\n- Final-sound override: __\n- Comment:\n\n---\n\n");
    }

    out.push_str("## Tally\n\n");
    out.push_str("Fill in after review. `N` = items reviewed; `A` = approved; `R` = rejected.\n\n");
    out.push_str(&format!(
        "- Approve rate: A = __ / N = {} = ___%\n",
        ranked.len()
    ));
    out.push_str("- Reject reasons (count each):\n  - loanword: __\n  - proper noun: __\n  - OCR artefact: __\n  - already in Lexicon (auto-tag miss): __\n  - not a real Kazakh word: __\n  - other: __\n\n");
    out.push_str("## Next step\n\n");
    out.push_str("Bundle approved roots into a single PR against `data/tokenizer/segmentation_roots.json`. Include for each:\n\n");
    out.push_str("```json\n{\n  \"id\": \"noun_<root>\",\n  \"root\": \"<root>\",\n  \"part_of_speech\": \"noun\",\n  \"vowel_harmony\": \"back|front\",\n  \"final_sound_class\": \"vowel|voiceless_consonant|voiced_consonant|nasal|liquid|glide\"\n}\n```\n\n");
    out.push_str("Then: `cargo run --release -p adam-corpus --bin morpheme_coverage` to measure delta. The PR description should include the before/after overall-coverage number (per memory `project_morpheme_coverage_baseline`).\n");
    out
}

/// Kazakh back vowels — all syllables containing these pull the word
/// into back harmony.
const BACK_VOWELS: &[char] = &['а', 'о', 'ұ', 'ы'];
/// Kazakh front vowels.
const FRONT_VOWELS: &[char] = &['ә', 'ө', 'ү', 'і', 'е'];
/// Glide-vowels that adopt the harmony of their surrounding syllable.
/// Not used for harmony inference — just flagged as "mixed" if alone.
const NEUTRAL_VOWELS: &[char] = &['и', 'у', 'ю'];

pub fn infer_vowel_harmony(surface: &str) -> &'static str {
    let mut has_back = false;
    let mut has_front = false;
    let mut has_neutral = false;
    for c in surface.chars() {
        if BACK_VOWELS.contains(&c) {
            has_back = true;
        } else if FRONT_VOWELS.contains(&c) {
            has_front = true;
        } else if NEUTRAL_VOWELS.contains(&c) {
            has_neutral = true;
        }
    }
    match (has_back, has_front, has_neutral) {
        (true, false, _) => "back",
        (false, true, _) => "front",
        (true, true, _) => "mixed",
        (false, false, true) => "neutral (only и/у/ю — needs context)",
        _ => "unknown (no vowels)",
    }
}

pub fn infer_final_sound(surface: &str) -> &'static str {
    let Some(last) = surface.chars().last() else {
        return "unknown";
    };
    if BACK_VOWELS.contains(&last) || FRONT_VOWELS.contains(&last) || NEUTRAL_VOWELS.contains(&last)
    {
        return "vowel";
    }
    match last {
        'п' | 'т' | 'с' | 'к' | 'қ' | 'ш' | 'ф' | 'х' | 'ц' | 'ч' | 'щ' => {
            "voiceless_consonant"
        }
        'б' | 'д' | 'г' | 'ғ' | 'в' | 'ж' | 'з' | 'ь' | 'ъ' => "voiced_consonant",
        'м' | 'н' | 'ң' => "nasal",
        'л' | 'р' => "liquid",
        'й' => "glide",
        _ => "unknown",
    }
}

fn truncate_to_display(s: &str, max_chars: usize) -> String {
    let collapsed: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let count = collapsed.chars().count();
    if count <= max_chars {
        return collapsed;
    }
    let mut out: String = collapsed.chars().take(max_chars).collect();
    out.push_str(" …");
    out
}

fn normalise(word: &str) -> String {
    word.chars()
        .filter(|c| c.is_alphabetic())
        .collect::<String>()
        .to_lowercase()
}

fn load_pack(path: &PathBuf) -> Result<PackFile, String> {
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}

/// Closed-class POS strings whose roots are accepted regardless of
/// length. These items have well-known irregular paradigms in
/// `adam-kernel-fst::pronoun_paradigm` (and forthcoming
/// closed-class extensions); the gap-miner must not classify their
/// inflected surfaces as "uncovered" just because the bare root is
/// two characters long.
const CLOSED_CLASS_POS: &[&str] = &[
    "pronoun",
    "particle",
    "conjunction",
    "postposition",
    "interjection",
    "numeral",
];

fn is_closed_class(pos: &str) -> bool {
    CLOSED_CLASS_POS.iter().any(|p| p.eq_ignore_ascii_case(pos))
}

fn load_roots() -> Result<HashSet<String>, String> {
    let mut set = HashSet::new();
    let mut short_closed_class_kept = 0usize;
    for path in [CURATED_ROOTS, APERTIUM_ROOTS] {
        let raw = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        let file: RootsFile = serde_json::from_str(&raw).map_err(|e| format!("{path}: {e}"))?;
        for entry in &file.roots {
            let r = entry.root.trim().to_lowercase();
            let len = r.chars().count();
            if len >= MIN_ROOT_LEN {
                set.insert(r);
            } else if is_closed_class(&entry.part_of_speech) {
                // Short closed-class roots — keep regardless of length.
                short_closed_class_kept += 1;
                set.insert(r);
            }
        }
    }
    eprintln!(
        "mine_lexicon_gaps: loaded {} Lexicon roots ({} short closed-class kept)",
        set.len(),
        short_closed_class_kept
    );
    Ok(set)
}

fn has_known_prefix(word: &str, roots: &HashSet<String>) -> bool {
    // Take prefixes of length 2..=word.len(), check if any is a known
    // root. The minimum 2 lets short closed-class roots ("ол", "не",
    // "мен", "сен") match their oblique surfaces ("оның", "неге",
    // "маған", "саған"). Long-root behaviour is unchanged because
    // any 2-char prefix of a noun/verb stem rarely matches a Lexicon
    // entry (those are length ≥ 3 by construction). Short-circuits
    // on first match.
    let chars: Vec<char> = word.chars().collect();
    let n = chars.len();
    for take in 2..=n {
        let prefix: String = chars.iter().take(take).collect();
        if roots.contains(&prefix) {
            return true;
        }
    }
    false
}

fn parse_usize(args: &[String], name: &str) -> Option<usize> {
    let idx = args.iter().position(|a| a == name)?;
    args.get(idx + 1).and_then(|s| s.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vowel_harmony_back_for_all_back_vowels() {
        assert_eq!(infer_vowel_harmony("бала"), "back");
        assert_eq!(infer_vowel_harmony("қазақ"), "back");
        assert_eq!(infer_vowel_harmony("оқу"), "back");
    }

    #[test]
    fn vowel_harmony_front_for_all_front_vowels() {
        assert_eq!(infer_vowel_harmony("мектеп"), "front");
        assert_eq!(infer_vowel_harmony("әке"), "front");
        assert_eq!(infer_vowel_harmony("өмір"), "front");
    }

    #[test]
    fn vowel_harmony_mixed_when_both_classes_present() {
        // Hypothetical loanword / compound: front ә + back а.
        assert_eq!(infer_vowel_harmony("әлам"), "mixed");
    }

    #[test]
    fn vowel_harmony_neutral_when_only_neutral_vowels() {
        assert_eq!(
            infer_vowel_harmony("ии"),
            "neutral (only и/у/ю — needs context)"
        );
    }

    #[test]
    fn final_sound_vowel() {
        assert_eq!(infer_final_sound("бала"), "vowel");
        assert_eq!(infer_final_sound("ата"), "vowel");
    }

    #[test]
    fn final_sound_voiceless_consonant() {
        assert_eq!(infer_final_sound("мектеп"), "voiceless_consonant");
        assert_eq!(infer_final_sound("қазақ"), "voiceless_consonant");
    }

    #[test]
    fn final_sound_nasal() {
        assert_eq!(infer_final_sound("адам"), "nasal");
        assert_eq!(infer_final_sound("жан"), "nasal");
    }

    #[test]
    fn final_sound_liquid() {
        assert_eq!(infer_final_sound("бел"), "liquid");
        assert_eq!(infer_final_sound("әр"), "liquid");
    }

    #[test]
    fn final_sound_glide() {
        assert_eq!(infer_final_sound("бай"), "glide");
    }

    #[test]
    fn final_sound_voiced_consonant() {
        assert_eq!(infer_final_sound("жүз"), "voiced_consonant");
        assert_eq!(infer_final_sound("дос"), "voiceless_consonant"); // с is voiceless
    }

    #[test]
    fn truncate_preserves_short_strings() {
        assert_eq!(truncate_to_display("короткое", 200), "короткое");
    }

    #[test]
    fn truncate_collapses_whitespace() {
        assert_eq!(
            truncate_to_display("one\n\ttwo  three", 200),
            "one two three"
        );
    }

    #[test]
    fn truncate_caps_long_strings_with_ellipsis() {
        let long: String = "а".repeat(300);
        let out = truncate_to_display(&long, 200);
        assert!(out.ends_with("…"));
        assert!(out.chars().count() <= 202); // 200 + " …"
    }

    #[test]
    fn normalise_strips_punct_and_lowercases() {
        assert_eq!(normalise("«Қазақ»"), "қазақ");
        assert_eq!(normalise("Word!"), "word");
    }

    #[test]
    fn has_known_prefix_matches_root_exact() {
        let mut roots = HashSet::new();
        roots.insert("бала".to_string());
        assert!(has_known_prefix("бала", &roots));
        assert!(has_known_prefix("балалар", &roots)); // prefix match
        assert!(!has_known_prefix("бұл", &roots));
    }

    #[test]
    fn has_known_prefix_ignores_sub_min_length() {
        // MIN_ROOT_LEN=3; a 2-char root shouldn't match.
        let mut roots = HashSet::new();
        roots.insert("ба".to_string()); // normally filtered at load
        assert!(!has_known_prefix("бала", &roots));
    }
}
