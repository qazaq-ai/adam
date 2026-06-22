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

/// **D.1 entry point.** Run every D.1 transform in order against
/// `raw_input`.  Returns the normalised form plus a list of
/// applied corrections (for trace logging).  When no transform
/// fires, `normalized == raw_input` and `corrections` is empty.
pub fn normalize(raw_input: &str) -> NormalizationResult {
    let mut corrections = Vec::new();
    let mut current = raw_input.to_string();

    // Pass 1: de-stuttering.  Token-level dash-prefix-onset
    // collapse.
    let destuttered = destutter(&current);
    if destuttered != current {
        corrections.push(format!("destutter: «{current}» → «{destuttered}»"));
        current = destuttered;
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
