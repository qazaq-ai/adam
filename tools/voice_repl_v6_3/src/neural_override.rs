// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! **Phase 19 step G (2026-06-02)** — high-confidence neural intent
//! override.
//!
//! The substring intent layer in `adam-dialog` is the active driver,
//! but it has a few known misroutes where a strong substring match
//! shadows the actual user intent:
//!
//!   - «Қазақстан туралы не білесің» — capability handler eats
//!     «не білесің» before the topic lookup runs.
//!   - «Сен қандай бағдарламалы тілді білесің» — language
//!     definition handler eats «тіл» before Rust topic fires.
//!
//! When the neural intent classifier is **strongly confident**
//! (conf ≥ 0.85) that the utterance is `AskAboutTopic`, we rewrite
//! the surface form to nudge the substring router into the right
//! branch. No new dialog code path; just input normalisation.
//!
//! Conservative on purpose — we only rewrite when BOTH the neural
//! signal is strong AND the input has a known misrouting trigger.

/// Rewrite a high-confidence AskAboutTopic utterance so substring
/// matchers see the topic-query shape instead of the misleading
/// capability / language-definition trigger words.
///
/// Returns the original input unchanged when no known trigger is
/// present. Output is lowercase when rewritten — the dialog
/// router's substring matchers are case-insensitive for ASCII
/// punctuation routing; Kazakh content lowercased preserves
/// meaning.
pub fn rewrite_topic_query(input: &str) -> String {
    let lower = input.to_lowercase();

    // Rule 2: Rust query — Whisper hears «Rust» as «раст» / «рас»
    // and surrounds it with «тілмен / бағдарламалы тіл», which the
    // substring router catches as «Тіл — қарым-қатынас құралы».
    // Detect a Rust context (rust/раст token, or «рас» followed by
    // a language/programming word) and rewrite to a canonical
    // topic query.
    let has_rust = lower.contains("rust")
        || lower.contains("раст")
        || lower.contains("рас тіл")
        || lower.contains("рас бағдарлама")
        || lower.contains("распен");
    let has_lang_or_prog = lower.contains("тіл") || lower.contains("бағдарлама");
    if has_rust && has_lang_or_prog {
        return "Rust туралы айтшы".to_string();
    }

    // Rule 1: «X туралы не білесің» → «X туралы айтшы». The «не
    // білесің» tail collides with the capability handler; stripping
    // it lets the topic lookup fire on the «туралы» frame.
    if lower.contains("туралы")
        && (lower.contains("білесің") || lower.contains("білесіз") || lower.contains("білемін"))
    {
        let mut s = lower.clone();
        for trigger in [
            " не білесің бе?",
            " не білесің бе",
            " не білесің?",
            " не білесің",
            " не білесіз бе?",
            " не білесіз бе",
            " не білесіз?",
            " не білесіз",
            " не білемін?",
            " не білемін",
        ] {
            s = s.replace(trigger, "");
        }
        let cleaned = s.trim().trim_end_matches(['.', '!', '?']).trim();
        let mut result = cleaned.to_string();
        if !result.ends_with("айт") && !result.ends_with("айтшы") {
            result.push_str(" айтшы");
        }
        return result;
    }

    input.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_ne_bilesin_when_turaly_present() {
        let out = rewrite_topic_query("Қазақстан туралы не білесің?");
        assert!(out.contains("қазақстан"), "got: {out}");
        assert!(!out.contains("білесің"), "got: {out}");
        assert!(out.contains("туралы"), "got: {out}");
        assert!(out.ends_with("айт") || out.ends_with("айтшы"), "got: {out}");
    }

    #[test]
    fn strips_ne_bilesiz_variant() {
        let out = rewrite_topic_query("Химия туралы не білесіз?");
        assert!(out.contains("химия"), "got: {out}");
        assert!(!out.contains("білесіз"), "got: {out}");
    }

    #[test]
    fn rust_query_with_t_rewrites() {
        let out = rewrite_topic_query("Раст бағдарламалы тіл білесің?");
        assert_eq!(out, "Rust туралы айтшы");
    }

    #[test]
    fn rust_query_without_t_rewrites() {
        let out = rewrite_topic_query("Сен рас тілмен бағдарлама жазаласың ба?");
        assert_eq!(out, "Rust туралы айтшы");
    }

    #[test]
    fn generic_language_query_passes_through() {
        // No rust/раст context — left alone (will route to «тіл»
        // definition, which is a substring-router question not
        // ours to fix here).
        let input = "Сен қандай бағдарламалы тілді білесің?";
        let out = rewrite_topic_query(input);
        assert_eq!(out, input);
    }

    #[test]
    fn untouched_when_no_trigger() {
        let input = "Қазақстанның бірінші президенті кім болды?";
        let out = rewrite_topic_query(input);
        assert_eq!(out, input);

        // No «туралы» → capability handler is correct here.
        let input = "Сен не білесің?";
        let out = rewrite_topic_query(input);
        assert_eq!(out, input);
    }

    #[test]
    fn rus_aitasyn_does_not_misfire() {
        // «Рас айтасың» = «you're right» — has «рас» but no
        // тіл/бағдарлама context, so no rewrite.
        let input = "Рас айтасың, осылай.";
        let out = rewrite_topic_query(input);
        assert_eq!(out, input);
    }
}
