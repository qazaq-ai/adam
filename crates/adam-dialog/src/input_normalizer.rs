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
    let (core, punct) = split_trailing_punct(token);
    let core_chars: Vec<char> = core.chars().collect();
    // **D.2 fix.** Minimum length 6: 5-char Kazakh particles
    // («керек», «қалай», «бойынша» prefix) are too ambiguous —
    // one phonetic substitution against a world_core entry
    // («терек» tree, «балай» misc) produces a 0.92-similarity
    // false positive that corrupts perfectly clean input.  The
    // sigmatism eval loses one «Фәлем» → «сәлем» case at this
    // length floor; that's an acceptable tradeoff for keeping
    // common Kazakh particles untouched.
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
    // corrupt the cascade input.  ASCII tokens that ARE
    // misspelled English get caught elsewhere (the Rust
    // curriculum keyword matcher handles its own typos).
    if core_chars.iter().all(|c| c.is_ascii_alphabetic()) {
        return token.to_string();
    }
    let lower = core.to_lowercase();
    if vocab.iter().any(|v| v.to_lowercase() == lower) {
        return token.to_string();
    }
    if let Some((best, _score)) = crate::kazakh_fuzzy::best_match(&lower, vocab, threshold) {
        return format!("{best}{punct}");
    }
    token.to_string()
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
];

/// Split a token into its alphabetic core and trailing
/// punctuation.  «сәлем.» → («сәлем», «.»); «сәлем» → («сәлем»,
/// «»).
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
}
