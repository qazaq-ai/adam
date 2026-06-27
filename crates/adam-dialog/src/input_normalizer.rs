// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `input_normalizer` — **v6.8.8 L4.9 D.1 speech-defect
//! candidate-rescoring**.
//!
//! Codex's L4.5 consultation named «candidate-rescoring + FST
//! fuzzy match BEFORE Conversation::turn» as a v7 milestone for
//! the speech-defect eval (52 % baseline on 71 cases × 8
//! categories: rhotacism, sigmatism, lambdacism, kappacism,
//! nasalisation, stuttering, elderly, whisper).  The cascade
//! handlers themselves are robust enough for clean inputs;
//! what's missing is a pre-processor that catches deterministic
//! speech-defect transforms BEFORE the input enters the
//! cascade.
//!
//! This module is that pre-processor.  D.1 ships only the first
//! transform (`destutter`) — a purely structural fix that needs
//! no lexicon lookup.  D.2 / D.3 add phonetic substitution
//! (rhotacism / sigmatism / lambdacism / kappacism / nasalisation)
//! by reusing the existing [`crate::kazakh_fuzzy`] Levenshtein
//! infrastructure.
//!
//! ## D.1 scope: stuttering
//!
//! The eval covers stuttering uniformly as «`<onset>-<onset>-<full>`»
//! where each onset is a 1–3-character prefix matching the
//! initial letter of the final segment:
//!
//! - `Са-сә-сәлем.` → `сәлем.`
//! - `Ме-мен-менің атым Дә-Дәулет.` → `менің атым Дәулет.`
//! - `Қа-қазақтың ұлттық тағамы.` → `қазақтың ұлттық тағамы.`
//!
//! De-stuttering is deterministic and lossless — the final
//! segment IS the intended word the speaker eventually
//! produced.

/// Result of normalising an input — typed wrapper so the caller
/// can log applied corrections (e.g. for the voice REPL trace).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizationResult {
    /// Input as the cascade should see it.  Equals `raw_input`
    /// when no transformation fired.
    pub normalized: String,
    /// Human-readable list of corrections applied, newest last.
    /// Empty when the input was already clean.
    pub corrections: Vec<String>,
}

/// **Entry point.** Run every transform in order against
/// `raw_input`.  Returns the normalised form plus a list of
/// applied corrections (for trace logging).  When no transform
/// fires, `normalized == raw_input` and `corrections` is empty.
///
/// Pipeline (in order):
///   1. `destutter` (D.1) — collapse «`Са-сә-сәлем`» → «сәлем».
///   2. `phonetic_substitute` (D.2) — token-level Kazakh-aware
///      Levenshtein replacement against the shared vocabulary
///      (Алматы/Қазақстан/жүрек/...) using the extended
///      [`crate::kazakh_fuzzy`] phonetic-pair table that covers
///      rhotacism / sigmatism / lambdacism / kappacism /
///      nasalisation defect substitutions.
pub fn normalize(raw_input: &str) -> NormalizationResult {
    let mut corrections = Vec::new();
    let mut current = raw_input.to_string();

    let destuttered = destutter(&current);
    if destuttered != current {
        corrections.push(format!("destutter: «{current}» → «{destuttered}»"));
        current = destuttered;
    }

    // **v6.8.31 — kappacism start-letter correction.**  Try
    // swapping initial «Х»/«К» → «Қ» when the resulting token
    // exists in the vocab.  Targets the cluster of failing
    // shapes the v6.8.30 audit surfaced («Хазах» → «Қазақ»,
    // «Хышхыл» → «Қышқыл», «Казхстан» → «Қазақстан»,
    // «Хазір» → «Қазір») that fall under the v6.8.29
    // length-6 floor or carry too many edits to reach the
    // 0.90 similarity threshold.  Runs BEFORE
    // phonetic_substitute so the next stage sees the
    // already-canonicalised initial letter.
    let kappacism_fixed = apply_kappacism_start_correction(&current, shared_vocab());
    if kappacism_fixed != current {
        corrections.push(format!(
            "kappacism_start: «{current}» → «{kappacism_fixed}»",
        ));
        current = kappacism_fixed;
    }

    let substituted = phonetic_substitute(&current, shared_vocab(), PHONETIC_THRESHOLD);
    if substituted != current {
        corrections.push(format!(
            "phonetic_substitute: «{current}» → «{substituted}»",
        ));
        current = substituted;
    }

    NormalizationResult {
        normalized: current,
        corrections,
    }
}

/// **v6.8.31 — Codex priority #5 second iteration.**  Targeted
/// start-letter kappacism correction.  Walks the input
/// token-by-token; when a token's initial letter is `Х`/`х`
/// (uvular fricative substitute) or `К`/`к` (velar fronting)
/// AND swapping it to `Қ`/`қ` produces a token in the vocab,
/// applies the swap.  Conservative — only acts when the
/// post-swap form is exactly in vocab, so no risk of
/// rewriting actual «х»-initial or «к»-initial Kazakh words
/// («хабар», «көк», «келді»).  Also probes a case-suffix-
/// stripped form so kappacism + case morphology compose
/// («Казхстанның» → strip «-ның» → «казхстан» → swap → match).
fn apply_kappacism_start_correction(input: &str, vocab: &[String]) -> String {
    input
        .split_whitespace()
        .map(|tok| apply_kappacism_start_to_token(tok, vocab))
        .collect::<Vec<_>>()
        .join(" ")
}

fn apply_kappacism_start_to_token(token: &str, vocab: &[String]) -> String {
    let (leading, core, trailing) = split_punct_around_core(token);
    let core_lower = core.to_lowercase();
    let core_chars: Vec<char> = core_lower.chars().collect();
    if core_chars.len() < 4 {
        return token.to_string();
    }
    let first = core_chars[0];
    if !matches!(first, 'х' | 'к' | 'т') {
        return token.to_string();
    }
    if vocab.iter().any(|v| v.to_lowercase() == core_lower) {
        // Already canonical or vocab-recognised — leave alone.
        return token.to_string();
    }
    // Candidate substitutes per defect direction:
    //   * `х → қ` — uvular fronting (most common Kazakh
    //     kappacism: «Хазах», «Хышхыл», «Хазір»).
    //   * `к → қ` — velar fronting that drops the uvular
    //     marker («Казхстан», «Кітап» wouldn't apply here
    //     since canonical К is correct).
    //   * `т → к` — K-class fronting to dental («Тітап»
    //     for «Кітап»).
    let candidates: &[char] = match first {
        'х' => &['қ'],
        'к' => &['қ'],
        'т' => &['к', 'қ'],
        _ => &[],
    };
    let max_len = core_chars.len() + 1;
    // **Asymmetric trust per starting letter.**  Х and К are
    // RARE as canonical Kazakh word-initial — most Х/К-start
    // tokens in defective speech are actually kappacism for
    // canonical Қ.  Т is COMMON as canonical word-initial
    // («таныс», «тұрады», «тарих», «тауық»…), so a blanket
    // best_match swap on Т-initial tokens false-positives
    // ordinary words onto distantly-related Kazakh nouns
    // («таныс» → «Сәтбаев»-derived).  Restrict the Т path
    // to EXACT vocab hits.
    let allow_fuzzy = !matches!(first, 'т');
    for &cand in candidates {
        let swapped: String = std::iter::once(cand)
            .chain(core_chars.iter().skip(1).copied())
            .collect();
        // Exact vocab hit — strongest evidence, return the
        // canonical (which may include casing fixes etc.).
        if let Some(canon) = vocab.iter().find(|v| v.to_lowercase() == swapped) {
            return format!("{leading}{canon}{trailing}");
        }
        // Phonetic-reachable from vocab — return the canonical
        // best_match directly so downstream length floors don't
        // skip short post-swap tokens.  Bound the candidate
        // length so we don't accept a much-longer canonical
        // (would over-aggressively rewrite short defective
        // tokens onto unrelated long Kazakh words).
        if allow_fuzzy
            && let Some((best, _score)) = crate::kazakh_fuzzy::best_match(&swapped, vocab, 0.85)
            && best.chars().count() <= max_len
        {
            return format!("{leading}{best}{trailing}");
        }
    }
    // Probe via the case-suffix strip path so kappacism +
    // case morphology compose: «Хазахстанның» → strip «-ның»
    // → «хазахстан» → swap to «қазахстан» — best_match against
    // «қазақстан» (1 phonetic-sub-from-vocab) clears the bar.
    if let Some((stem_lower, suffix)) = split_case_suffix(&core_lower) {
        if !stem_lower.is_empty() {
            let first_stem = stem_lower.chars().next().unwrap_or(' ');
            if matches!(first_stem, 'х' | 'к' | 'т') {
                let stem_chars: Vec<char> = stem_lower.chars().collect();
                let stem_cands: &[char] = match first_stem {
                    'х' => &['қ'],
                    'к' => &['қ'],
                    'т' => &['к', 'қ'],
                    _ => &[],
                };
                let stem_max = stem_chars.len() + 1;
                let stem_allow_fuzzy = !matches!(first_stem, 'т');
                for &cand in stem_cands {
                    let stem_swapped: String = std::iter::once(cand)
                        .chain(stem_chars.iter().skip(1).copied())
                        .collect();
                    if let Some(canon) = vocab.iter().find(|v| v.to_lowercase() == stem_swapped) {
                        let composed = format!("{canon}{suffix}");
                        return format!("{leading}{composed}{trailing}");
                    }
                    if stem_allow_fuzzy
                        && let Some((best, _score)) =
                            crate::kazakh_fuzzy::best_match(&stem_swapped, vocab, 0.85)
                        && best.chars().count() <= stem_max
                    {
                        let composed = format!("{best}{suffix}");
                        return format!("{leading}{composed}{trailing}");
                    }
                }
            }
        }
    }
    token.to_string()
}

/// Collapse stuttering onsets of the form `<onset>-<onset>-...-<full>`
/// down to just `<full>`.  Operates token by token (whitespace
/// preserves between tokens), so an input like
/// «Ме-мен-менің атым Дә-Дәулет.» becomes
/// «менің атым Дәулет.» without disturbing the inter-token
/// spaces or trailing punctuation.
pub fn destutter(input: &str) -> String {
    input
        .split_whitespace()
        .map(destutter_token)
        .collect::<Vec<_>>()
        .join(" ")
}

fn destutter_token(token: &str) -> String {
    let (core, punct) = split_trailing_punct(token);
    if !core.contains('-') {
        return token.to_string();
    }
    let segments: Vec<&str> = core.split('-').collect();
    if segments.len() < 2 {
        return token.to_string();
    }
    let last = segments.last().copied().unwrap();
    let last_chars: Vec<char> = last.chars().collect();
    let last_len = last_chars.len();
    let last_first_lower: Option<char> = last_chars.first().map(|c| c.to_ascii_lowercase());

    // Every prefix segment must be:
    //   * 1..=3 characters (typical stutter onset length);
    //   * strictly shorter than the final segment;
    //   * starting with the SAME letter (case-insensitive) as
    //     the final segment — sanity check against splitting
    //     legitimately-hyphenated tokens like «наряд-рұқсат».
    let prefixes = &segments[..segments.len() - 1];
    let all_valid = prefixes.iter().all(|seg| {
        let seg_chars: Vec<char> = seg.chars().collect();
        let seg_len = seg_chars.len();
        if !(1..=3).contains(&seg_len) || seg_len >= last_len {
            return false;
        }
        let seg_first_lower = seg_chars
            .first()
            .map(|c| c.to_lowercase().next().unwrap_or(*c))
            .map(|c| c.to_ascii_lowercase());
        let last_first = last_first_lower
            .map(|c| c.to_lowercase().next().unwrap_or(c))
            .map(|c| c.to_ascii_lowercase());
        seg_first_lower == last_first
    });
    if !all_valid {
        return token.to_string();
    }
    format!("{last}{punct}")
}

/// **D.2 phonetic substitution threshold.** A token must match
/// a vocab entry with at least this Kazakh-fuzzy similarity
/// score to be replaced.  Tuned so canonical defect patterns
/// (one phonetic substitution against a 5-8 char target) pass
/// while morphology-preserving inputs do NOT get rewritten:
///
///   * «Айматы» (6 chars) vs «Алматы» (1 phonetic sub, cost 0.4)
///     → similarity ≈ 1 - 0.4/6 ≈ 0.93.  ✓ fires.
///   * «Хазахстанның» (12 chars) vs «Қазақстанның» (2 phonetic
///     subs, cost 0.8) → similarity ≈ 1 - 0.8/12 ≈ 0.93.  ✓ fires.
///   * «Фәлем» (5) vs «сәлем» (1 phonetic sub) → 1 - 0.4/5 ≈ 0.92.
///     ✓ fires.
///   * «Жетіге» (6) vs «жетіген» (1 char insertion, cost 1.0)
///     → 1 - 1/7 ≈ 0.86.  ✗ rejected — morphology preserved.
///
/// The 0.90 floor is the difference between «one-phonetic-sub
/// defect» (cost 0.4 — always fires) and «one-char insertion or
/// random sub» (cost 1.0 — rejected).  A v6.8.9 D.2 production
/// regression (math morphology suffix «-ге» → «-ген») drove the
/// floor up from 0.85 → 0.90.
const PHONETIC_THRESHOLD: f32 = 0.90;

/// **D.2 — token-level phonetic substitution.** Walk the input
/// token by token; for any token NOT already in `vocab`,
/// consult [`crate::kazakh_fuzzy::best_match`] for the best
/// vocab entry above `threshold`; replace when found.  Existing
/// punctuation is preserved.
///
/// Skipped categories (the substitution never fires):
///   * pure-digit or punctuation tokens (math expressions,
///     numbers, dates);
///   * tokens shorter than 4 characters (too ambiguous —
///     short Kazakh particles like «не», «ма», «де» would get
///     incorrectly rewritten);
///   * tokens that ARE in the vocab (no need to substitute).
pub fn phonetic_substitute(input: &str, vocab: &[String], threshold: f32) -> String {
    input
        .split_whitespace()
        .map(|tok| phonetic_substitute_token(tok, vocab, threshold))
        .collect::<Vec<_>>()
        .join(" ")
}

fn phonetic_substitute_token(token: &str, vocab: &[String], threshold: f32) -> String {
    let (leading, core, trailing) = split_punct_around_core(token);
    let core_chars: Vec<char> = core.chars().collect();
    // **v6.8.29 — Codex priority #5 speech-defect v7.**  The
    // min-length-6 floor stays — at length 5 a single phonetic-
    // sub (cost 0.4) clears the 0.90 threshold and false-
    // positives common Kazakh words against same-length nouns
    // («алдын» → «алтын», «керек» → «терек»).  The v6.8.29 win
    // for kappacism / elderly tokens comes from the suffix-strip
    // FALLBACK below (handles long-form «Хазахстанның» (12) →
    // «Қазақстанның» via stem stripping), NOT from a lower
    // length floor.
    if core_chars.len() < 6 {
        return token.to_string();
    }
    if core_chars
        .iter()
        .all(|c| c.is_ascii_digit() || matches!(*c, '.' | ',' | '+' | '-' | '*' | '/' | '=' | '%'))
    {
        return token.to_string();
    }
    // **D.2 fix.** Skip ASCII-only tokens — English loanwords
    // («lifetimes», «traits», «ownership») are first-class in
    // the code-tutor cascade; vocab is exclusively Kazakh, so a
    // best-match against vocab would inevitably rewrite a real
    // English term to a phonetically-similar Kazakh word and
    // corrupt the cascade input.  Rust attributes like
    // `#[instrument]` are caught here too: the
    // `split_punct_around_core` helper strips the leading `#[`
    // and trailing `]` so the bracket-wrapped core («instrument»)
    // reaches the all-ASCII-alphabetic check unobscured.
    if core_chars.iter().all(|c| c.is_ascii_alphabetic()) {
        return token.to_string();
    }
    // **D.2 hotfix v3.** Skip tokens whose core contains INNER
    // punctuation — generic-parameterised Rust types like
    // «`Arc<Mutex>`» / «`Vec<T>`» / «`Result<T,E>`» have
    // alphanumeric chars on both ends, so split_punct_around_core
    // keeps the inner `<` `>` `,` inside `core`.  A phonetic-defect
    // pattern is by definition a single-word phoneme substitution
    // and never spans punctuation, so the safe move is to skip
    // any token whose core isn't alphanumeric-plus-hyphen.
    if core_chars.iter().any(|c| !c.is_alphanumeric() && *c != '-') {
        return token.to_string();
    }
    let lower = core.to_lowercase();
    if vocab.iter().any(|v| v.to_lowercase() == lower) {
        return token.to_string();
    }
    if let Some((best, _score)) = crate::kazakh_fuzzy::best_match(&lower, vocab, threshold) {
        // **D.2 hotfix v3.** Candidate length must be ≥ input
        // length.  Speech defects substitute or drop characters
        // — the canonical form is always EQUAL or LONGER than
        // the defective form.  Without this guard, agglutinative
        // morphology suffixes («университет-і» → «университет»)
        // get treated as defect-fixes and stripped, corrupting
        // any genitive / possessive construction.  Saw it caught
        // by `v6_0_6_audit_regression::round4b_kru_university_
        // definition_question_surfaces_grounded_fact`.
        let input_len = lower.chars().count();
        let cand_len = best.chars().count();
        if cand_len < input_len {
            return token.to_string();
        }
        return format!("{leading}{best}{trailing}");
    }
    // **v6.8.29 — speech-defect v7 fallback.**  When the direct
    // lower-against-vocab match misses (typically because the
    // input carries a case suffix not present in the vocab
    // lemmas), try splitting the token into stem + suffix and
    // matching just the stem.  On a hit, re-append the suffix
    // so morphology is preserved.  Closes the kappacism cases
    // «Хазахстанның» → «Қазақстанның» / «Тмірдің» → «Темірдің»
    // / «Кмістің» → «Күмістің» that fall through the direct
    // path because the world_core vocab indexes lemmas only.
    if let Some((stem, suffix)) = split_case_suffix(&lower) {
        if stem.chars().count() >= 4 {
            if let Some((best_stem, _score)) =
                crate::kazakh_fuzzy::best_match(&stem, vocab, threshold)
            {
                let best_len = best_stem.chars().count();
                let stem_len = stem.chars().count();
                if best_len >= stem_len {
                    return format!("{leading}{best_stem}{suffix}{trailing}");
                }
            }
        }
    }
    token.to_string()
}

/// **v6.8.29.**  Split a Kazakh token into (stem, case_suffix) on
/// recognised genitive / locative / ablative / dative endings.
/// Returns `None` when no recognised suffix is present.  Local
/// to this module — keeps the speech-defect fallback path
/// self-contained without a cross-crate dependency on
/// `v6_2_router::strip_kazakh_case_suffixes`.
fn split_case_suffix(lower: &str) -> Option<(String, &'static str)> {
    // Longest-first so «ның» strips before «ы».
    const SUFFIXES: &[&str] = &[
        "ның",
        "нің",
        "дың",
        "дің",
        "тың",
        "тің",
        "сынан",
        "сінен",
        "сына",
        "сіне",
        "ына",
        "іне",
        "нан",
        "нен",
        "дан",
        "ден",
        "тан",
        "тен",
        "ға",
        "ге",
        "қа",
        "ке",
        "да",
        "де",
        "та",
        "те",
    ];
    for suf in SUFFIXES {
        if lower.ends_with(suf) {
            let stem_len = lower.len() - suf.len();
            let stem = lower[..stem_len].to_string();
            if !stem.is_empty() {
                return Some((stem, suf));
            }
        }
    }
    None
}

/// Shared vocabulary loaded once per process from the curated
/// world_core fact graph plus a small set of high-frequency
/// interjections / particles the eval covers but world_core
/// does not (greetings, acknowledgements).  Lower-cased.
///
/// Vocab is intentionally limited to the world_core surface set
/// + curated greetings; we do NOT pull in every random word from
/// the lexicon, because doing so dilutes the best-match
/// signal — most random Kazakh nouns would score similarly to
/// the intended canonical, and the wrong one would win.
fn shared_vocab() -> &'static [String] {
    use std::sync::OnceLock;
    static VOCAB: OnceLock<Vec<String>> = OnceLock::new();
    VOCAB.get_or_init(build_vocab)
}

fn build_vocab() -> Vec<String> {
    use std::collections::HashSet;
    let mut set: HashSet<String> = HashSet::new();

    // 1. High-frequency Kazakh greetings / interjections /
    //    particles the eval probes.  Each is a stand-alone
    //    canonical form a defect-form would map to.
    for w in CURATED_HIGH_FREQ {
        set.insert((*w).to_string());
    }

    // 2. Every distinct agent + object surface from world_core.
    //    Walk all jsonl files in `data/world_core/*.jsonl` and
    //    extract `facts[].subject` and `facts[].object`.  Each
    //    surface is added in lowercase to match the caller's
    //    case-insensitive lookup.
    for candidate in [
        "data/world_core",
        "../data/world_core",
        "../../data/world_core",
        "../../../data/world_core",
    ] {
        if let Ok(read_dir) = std::fs::read_dir(candidate) {
            for entry in read_dir.flatten() {
                let p = entry.path();
                if p.extension().and_then(|s| s.to_str()) != Some("jsonl") {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(&p) else {
                    continue;
                };
                for line in text.lines() {
                    let Ok(val) = serde_json::from_str::<serde_json::Value>(line) else {
                        continue;
                    };
                    let Some(facts) = val.get("facts").and_then(|v| v.as_array()) else {
                        continue;
                    };
                    for fact in facts {
                        for key in ["subject", "object"] {
                            if let Some(s) = fact.get(key).and_then(|v| v.as_str()) {
                                let trimmed = s.trim();
                                if !trimmed.is_empty() {
                                    set.insert(trimmed.to_lowercase());
                                }
                            }
                        }
                    }
                }
            }
            // First directory that worked wins; don't double-load.
            break;
        }
    }

    set.into_iter().collect()
}

/// High-frequency canonical surfaces that the eval probes but
/// world_core does not list as facts (greetings, particles,
/// short interjections).  Lower-case throughout.
const CURATED_HIGH_FREQ: &[&str] = &[
    "сәлем",
    "рақмет",
    "оқасы жоқ",
    "сау бол",
    "бар бол",
    "иә",
    "жоқ",
    "мен",
    "сен",
    "сіз",
    "ассалаумағалейкум",
    "уағалайкум",
    "уағалайкум-ас-салам",
    "қош",
    "хош",
    // **v6.8.29 — Codex #5 speech-defect v7.**  High-frequency
    // Kazakh function / content words that are NOT noun
    // subjects/objects in world_core, so the world_core-derived
    // vocab misses them.  Without these in the early-exit
    // vocab the v6.8.29 min-length-4 phonetic-substitute path
    // false-positives them onto similar-looking nouns
    // («керек» → «терек»: poplar tree).  These are the closed
    // set of 4-7 char tokens caught by adversarial probe.
    "керек",
    "деген",
    "менің",
    "сенің",
    "сіздің",
    "бұл",
    "осы",
    "сол",
    "бірі",
    "бірге",
    "онда",
    "сонда",
    "өзің",
    "өзім",
];

/// Split a token into its alphabetic core and trailing
/// punctuation.  «сәлем.» → («сәлем», «.»); «сәлем» → («сәлем»,
/// «»).  Used by destutter (whose stutter pattern lives in the
/// core and dash-segments don't have leading punct).
fn split_trailing_punct(token: &str) -> (&str, &str) {
    let mut split_at = token.len();
    for (i, ch) in token.char_indices().rev() {
        if ch.is_alphanumeric() || ch == '-' {
            split_at = i + ch.len_utf8();
            break;
        }
    }
    if split_at == token.len() {
        return (token, "");
    }
    token.split_at(split_at)
}

/// Split a token into `(leading_punct, core, trailing_punct)`
/// where `core` carries only alphanumeric characters (plus
/// internal hyphens).  «`#[instrument]`» → («`#[`», «instrument»,
/// «`]`»); «сәлем.» → («», «сәлем», «.»).  Used by
/// `phonetic_substitute_token` so a Rust attribute or bracketed
/// English term doesn't fall through the ASCII-alpha guard
/// just because its leading bracket isn't alphanumeric.
fn split_punct_around_core(token: &str) -> (&str, &str, &str) {
    // Find first alphanumeric — leading punct ends there.
    let first_alpha = token
        .char_indices()
        .find(|(_, c)| c.is_alphanumeric())
        .map(|(i, _)| i);
    let Some(start) = first_alpha else {
        return ("", "", token);
    };
    // Find last alphanumeric — trailing punct starts after it.
    let last_alpha_end = token
        .char_indices()
        .rev()
        .find(|(_, c)| c.is_alphanumeric())
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(token.len());
    let leading = &token[..start];
    let core = &token[start..last_alpha_end];
    let trailing = &token[last_alpha_end..];
    (leading, core, trailing)
}

#[cfg(test)]
mod v6_8_31_kappacism_start_tests {
    //! **v6.8.31 — Codex #5 second iteration.**  Lock the
    //! start-letter kappacism preprocessor.  Recoveries should
    //! be canonical Kazakh forms; canonical-Х / canonical-Т
    //! Kazakh words must NOT be rewritten.
    use super::normalize;

    #[test]
    fn x_initial_kappacism_recovers() {
        let r = normalize("Хышхыл деген не?");
        assert!(
            r.normalized.to_lowercase().contains("қышқыл"),
            "got: {}",
            r.normalized
        );
    }

    #[test]
    fn k_initial_kappacism_recovers_with_suffix() {
        // Казхстанның → suffix strip → казхстан → swap к→қ →
        // best_match against canonical «қазақстан» → return
        // canonical + re-append «-ның».
        let r = normalize("Хазахстанның");
        assert!(
            r.normalized.to_lowercase().contains("қазақстан"),
            "got: {}",
            r.normalized
        );
    }

    #[test]
    fn t_initial_exact_vocab_match_recovers() {
        // Тітап → swap т→к → «кітап» exact vocab hit → swap.
        let r = normalize("Тітап деген не?");
        assert!(
            r.normalized.to_lowercase().contains("кітап"),
            "got: {}",
            r.normalized
        );
    }

    #[test]
    fn t_initial_canonical_word_not_rewritten() {
        // «таныс» (acquainted) is a real Kazakh word — must
        // NOT get rewritten via the kappacism path even
        // though it starts with Т.  Fuzzy path is gated off
        // for Т-initial tokens to prevent this.
        let r = normalize("Алдымен таныс болайық.");
        assert!(
            r.normalized.to_lowercase().contains("таныс"),
            "must preserve canonical «таныс», got: {}",
            r.normalized
        );
    }

    #[test]
    fn x_initial_canonical_word_not_rewritten() {
        // «хабар» (news) is canonical Kazakh — exact-vocab
        // check skips already-canonical tokens.
        let r = normalize("Хабар жоқ па?");
        assert!(
            r.normalized.to_lowercase().contains("хабар"),
            "must preserve canonical «хабар», got: {}",
            r.normalized
        );
    }
}

#[cfg(test)]
mod v6_8_29_speech_defect_tests {
    //! **v6.8.29 — Codex #5 speech-defect v7.**  Lock the
    //! suffix-stripping fallback path so kappacism / elderly
    //! tokens with case suffixes recover via the world_core
    //! lemma vocab.
    use super::{normalize, split_case_suffix};

    #[test]
    fn split_case_suffix_genitive() {
        assert_eq!(
            split_case_suffix("хазахстанның"),
            Some(("хазахстан".into(), "ның"))
        );
        assert_eq!(split_case_suffix("темірдің"), Some(("темір".into(), "дің")));
    }

    #[test]
    fn split_case_suffix_locative() {
        assert_eq!(
            split_case_suffix("қазақстанда"),
            Some(("қазақстан".into(), "да"))
        );
    }

    #[test]
    fn split_case_suffix_returns_none_for_lemma() {
        // Stem with no recognised suffix shouldn't split.  The
        // test relies on a token whose trailing chars don't
        // match any of the curated case suffixes.
        assert_eq!(split_case_suffix("кітап"), None);
    }

    #[test]
    fn kappacism_with_genitive_recovers() {
        let r = normalize("Хазахстанның");
        assert!(
            r.normalized.to_lowercase().contains("қазақстан"),
            "expected normalisation to produce Қазақстанның, got «{}»",
            r.normalized
        );
    }

    #[test]
    fn elderly_dropped_vowel_with_genitive_recovers() {
        // Кмістің ← Күмістің (drop ү).  Fallback path strips
        // «тің», stems are «кміс» → vocab «күміс» via cheap-
        // vowel-insertion edit cost; suffix re-appended.
        let r = normalize("Кмістің");
        assert!(
            r.normalized.to_lowercase().contains("күміс"),
            "expected Күмістің recovery, got «{}»",
            r.normalized
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn destutter_simple_three_segment() {
        assert_eq!(destutter("Са-сә-сәлем."), "сәлем.");
        assert_eq!(destutter("Жү-жү-жүрек"), "жүрек");
    }

    #[test]
    fn destutter_two_segment() {
        assert_eq!(destutter("Дә-Дәулет."), "Дәулет.");
        assert_eq!(destutter("Қа-қазақтың"), "қазақтың");
    }

    #[test]
    fn destutter_mixed_token_sentence() {
        assert_eq!(
            destutter("Ме-мен-менің атым Дә-Дәулет."),
            "менің атым Дәулет.",
        );
    }

    #[test]
    fn destutter_full_eval_sample() {
        // All ten stuttering cases from data/eval/speech_defect_eval.json.
        assert_eq!(destutter("Са-сә-сәлем."), "сәлем.");
        assert_eq!(
            destutter("Ме-мен-менің атым Дә-Дәулет."),
            "менің атым Дәулет.",
        );
        assert_eq!(
            destutter("Жү-жү-жүрек не үшін керек?"),
            "жүрек не үшін керек?",
        );
        assert_eq!(
            destutter("Қа-қа-қазақстанның астанасы."),
            "қазақстанның астанасы.",
        );
        assert_eq!(destutter("Ал-ал-алты түбірі."), "алты түбірі.");
        assert_eq!(
            destutter("Бі-бі-бір байтта неше бит бар?"),
            "бір байтта неше бит бар?",
        );
        assert_eq!(
            destutter("Кү-кү-күмістің формуласы."),
            "күмістің формуласы."
        );
        assert_eq!(destutter("А-а-атом дегеніміз не?"), "атом дегеніміз не?");
        assert_eq!(
            destutter("Қа-қазақтың ұлттық тағамы."),
            "қазақтың ұлттық тағамы.",
        );
        assert_eq!(destutter("Бе-бе-бесті жетіге қос."), "бесті жетіге қос.");
    }

    /// Hyphenated multi-word terms must NOT be collapsed —
    /// «наряд-рұқсат», «техникалық-экономикалық» are real
    /// compounds, not stutters.  Our sanity gate (first-letter
    /// match) catches them.
    #[test]
    fn destutter_preserves_legitimate_compounds() {
        assert_eq!(destutter("наряд-рұқсат"), "наряд-рұқсат");
        assert_eq!(
            destutter("техникалық-экономикалық"),
            "техникалық-экономикалық"
        );
    }

    /// Clean input (no hyphens, no stuttering) passes through
    /// unchanged byte-for-byte.
    #[test]
    fn destutter_clean_input_passthrough() {
        assert_eq!(destutter("Сәлем!"), "Сәлем!");
        assert_eq!(destutter("Менің атым — Дәулет."), "Менің атым — Дәулет.");
        assert_eq!(destutter(""), "");
    }

    /// `normalize` wraps `destutter` with the corrections trace.
    #[test]
    fn normalize_records_corrections() {
        let r = normalize("Са-сә-сәлем.");
        assert_eq!(r.normalized, "сәлем.");
        assert_eq!(r.corrections.len(), 1);
        assert!(r.corrections[0].contains("destutter"));
    }

    #[test]
    fn normalize_clean_input_no_corrections() {
        let r = normalize("Сәлем!");
        assert_eq!(r.normalized, "Сәлем!");
        assert!(r.corrections.is_empty());
    }

    /// **Regression coverage (D.2 hotfix v2).** Rust attributes
    /// like `#[instrument]` carry brackets around an English-
    /// alphabetic core.  Before `split_punct_around_core`, the
    /// leading `#[` was retained as part of `core` and the
    /// ASCII-alpha guard didn't fire — best_match then returned
    /// a punctuated form that, with the trailing bracket
    /// re-appended, produced «`#[instrument]]`» (extra bracket
    /// at the end).  This caused
    /// `rust_async_book_chapter_08_holdout::async8_instrument`
    /// to drop to 17/18.  Fix: strip leading + trailing punct
    /// around the core, then guard on the core's ASCII shape.
    #[test]
    fn normalize_preserves_rust_attribute_with_brackets() {
        let r = normalize("#[instrument] деген не?");
        assert_eq!(r.normalized, "#[instrument] деген не?");
        assert!(
            r.corrections.is_empty(),
            "Rust attribute must pass through; corrections: {:?}",
            r.corrections,
        );
    }

    #[test]
    fn normalize_preserves_bracketed_english_term() {
        let r = normalize("[ownership] қалай жұмыс істейді?");
        assert_eq!(r.normalized, "[ownership] қалай жұмыс істейді?");
    }

    /// **Regression coverage (D.2 hotfix v3).** Generic-parameterised
    /// Rust types like «`Arc<Mutex>`» / «`Vec<T>`» have alphanumeric
    /// chars on both ends so split_punct_around_core keeps the
    /// inner `<` / `>` inside `core`.  Before the inner-punct
    /// guard, best_match would run against `arc<mutex` and return
    /// a punctuated form that produced «`arc<mutex>>`» (extra `>`).
    /// This caused `rust_book_chapter_16_holdout::ch16_arc_mutex`
    /// to drop to 17/18.
    #[test]
    fn normalize_preserves_generic_type_with_angle_brackets() {
        let r = normalize("Arc<Mutex> деген не?");
        assert_eq!(r.normalized, "Arc<Mutex> деген не?");
        assert!(r.corrections.is_empty());
    }

    #[test]
    fn normalize_preserves_result_with_comma_inside() {
        let r = normalize("Result<T,E> қалай жұмыс істейді?");
        assert_eq!(r.normalized, "Result<T,E> қалай жұмыс істейді?");
    }

    /// **Regression coverage (D.2 hotfix v3 — morphology-preserving
    /// length guard).** Kazakh agglutinative morphology suffixes
    /// (possessive `-і`, locative `-те`, dative `-ге`, etc.) lengthen
    /// the root.  Before the candidate-length guard, the
    /// possessive «университет-і» would score ≈ 0.92 against the
    /// shorter root «университет» (1-char edit / 12 chars) and
    /// get stripped down to the bare root, breaking the cascade's
    /// understanding of the genitive construction.
    /// `v6_0_6_audit_regression::round4b_kru_university_definition_
    /// question_surfaces_grounded_fact` caught this.
    #[test]
    fn normalize_preserves_morphology_suffix() {
        let r = normalize("Қостанай өңірлік университеті деген не?");
        assert_eq!(
            r.normalized, "Қостанай өңірлік университеті деген не?",
            "morphology suffix (-і possessive) must be preserved",
        );
    }
}
