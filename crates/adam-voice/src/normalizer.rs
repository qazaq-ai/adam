//! **v5.19.0 (V3).** Kazakh transcript normalizer.
//!
//! Whisper-medium consistently mishears Kazakh-specific phonemes
//! and word boundaries. Live-test 2026-05-11 caught:
//!
//! - «Сәлем» → «Салим» (drops `қ`, adds Arabic-style `и`, treats
//!   greeting as Arabic name)
//! - «Менің атым Дәулет» → «Мен аматым дау лет» (splits `Дә-улет`
//!   across word boundary; «менің» mangled to «менім»)
//! - «Танысайық» → «Танысайыр» / «Танысайыр тим» (drops `ң`,
//!   appends artifact)
//! - «Менің атым Дәулет» → «Менім атом дау лет» («атым» heard as
//!   «атом» — the chemistry word adam knows from `world_core`)
//! - «Менің есімде Дәулет» → «Менің есіңің дау лет» («есіңің»
//!   is not a real Kazakh wordform — Whisper invented it)
//!
//! This module applies a layered rule-based post-processor:
//!
//! 1. **Word-boundary mergers** — re-attach Kazakh names/words
//!    that Whisper split mid-stem («дау лет» → «Дәулет»).
//! 2. **Phoneme substitutions** — fix common phoneme drops
//!    («Салим» → «Сәлем» when in greeting context, «менім» →
//!    «менің», «атом» → «атым» when preceded by «менің»).
//! 3. **Artifact trimming** — drop nonsense suffixes Whisper
//!    sometimes appends («тим», «ыр тим»).
//!
//! Layer 2 fixes are **context-conditional** to avoid over-
//! correcting: «Салим» in isolation could be a real Arabic name,
//! but «Салим» followed by `!` or `.` in a turn-1 greeting context
//! is overwhelmingly «Сәлем». «Атом» (atom) is a real Kazakh
//! word — only rewrite to «атым» (my-name) when the prior token
//! is «Менің» / «Меним».
//!
//! The normalizer is **deterministic + inspectable** + cheap. No
//! ML, no neural net — pure pattern rewriting that fits the
//! project's «third path» commitment per
//! `project_retrieval_not_neural_v2`. Each rule has a
//! corresponding test capturing the live-observed input/output.

/// Applies all Kazakh-transcript fixes to a raw Whisper output.
/// Returns a new String; original is consumed. Idempotent —
/// running the normalizer twice produces identical output.
pub fn normalize_kazakh_transcript(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let lower = trimmed.to_lowercase();

    let mut out = trimmed.to_string();

    // ── Layer 1: word-boundary mergers ──────────────────────────
    // «дау лет» / «Дау лет» → «Дәулет» (most common Whisper-medium
    // split for the Kazakh name «Дәулет»; the `ә` vowel between two
    // open syllables routinely causes a phantom space).
    out = replace_token_pair(&out, "дау", "лет", "Дәулет");
    // «Дә улет» — same name, split differently.
    out = replace_token_pair(&out, "дә", "улет", "Дәулет");
    // «Танысайыр тим» / «танысайыр тим» — Whisper appends a
    // nonsense «тим» token. Drop the appended artifact AND
    // restore «ң» (Whisper dropped it as «р»).
    out = replace_phrase(&out, "танысайыр тим", "Танысайық");
    out = replace_phrase(&out, "Танысайыр тим", "Танысайық");
    // «танысайыр» without the «тим» tail — still misheard «ң» →
    // «р». Only the standalone word, not «танысайық» which is
    // already correct.
    if lower.contains("танысайыр") && !lower.contains("танысайық") {
        out = case_aware_replace(&out, "танысайыр", "Танысайық");
    }

    // ── Layer 2: phoneme substitutions (context-conditional) ────
    // «Салим» / «салим» in a short greeting-shaped input
    // («Салим!», «Салим.») → «Сәлем». Guard against false-fire
    // on actual Arabic-name context by requiring the input to be
    // ≤ 15 codepoints (greetings are typically 4-12).
    let lowered_check = out.to_lowercase();
    if (lowered_check.contains("салим") || lowered_check.contains("салим!"))
        && out.chars().count() <= 15
    {
        out = case_aware_replace(&out, "Салим", "Сәлем");
        out = case_aware_replace(&out, "салим", "сәлем");
    }
    // «менім» (no such 1sg-poss form) → «менің» (genitive 1sg-poss).
    out = case_aware_replace(&out, "менім", "менің");
    out = case_aware_replace(&out, "Менім", "Менің");
    // «атом» (atom — real word, but in name-statement context it's
    // a mishearing of «атым»). Trigger only when preceded by
    // «менің» / «Менің» (genitive marker before noun = possessive
    // construction).
    let lc = out.to_lowercase();
    if lc.contains("менің атом") || lc.contains("меним атом") {
        out = case_aware_replace(&out, "атом", "атым");
        out = case_aware_replace(&out, "Атом", "Атым");
    }
    // «есіңің» (not a real wordform — Whisper invents this when
    // mishearing the participle «есімде»). Only safe rewrite is
    // back to «есімде» when the surrounding shape is a recall
    // question or name statement; conservative — only fix it in a
    // «есіңің … лет/Дәулет» context where the user almost certainly
    // said «есімде Дәулет».
    if lc.contains("есіңің") && (lc.contains("дау лет") || lc.contains("дәулет"))
    {
        out = case_aware_replace(&out, "есіңің", "есімде");
        out = case_aware_replace(&out, "Есіңің", "Есімде");
    }

    // Re-apply «дау лет» → «Дәулет» after layer-2 rewrites in case
    // the context fix exposed it.
    out = replace_token_pair(&out, "дау", "лет", "Дәулет");

    // ── Layer 3: stray artifact trimming ────────────────────────
    // Trailing « тим» / «тим.» / «тим!» — Whisper occasional
    // suffix it doesn't correspond to anything in Kazakh.
    for suffix in [" тим.", " тим!", " тим?", " тим"] {
        if out.ends_with(suffix) {
            out.truncate(out.len() - suffix.len());
            out.push('.');
            break;
        }
    }

    out.trim().to_string()
}

/// Replace `"<a> <b>"` with `replacement`, preserving case of the
/// first letter of `<a>` if it was uppercase.
fn replace_token_pair(input: &str, a: &str, b: &str, replacement: &str) -> String {
    let lower = input.to_lowercase();
    let target_lower = format!("{a} {b}");
    if !lower.contains(&target_lower) {
        return input.to_string();
    }
    // Find the start offset in the lowercased version, map back to
    // the original. Since lowercasing preserves byte length for the
    // Cyrillic Kazakh alphabet (each char is 2 bytes lower and
    // upper), the offset is the same.
    let mut result = String::with_capacity(input.len());
    let mut remaining: &str = input;
    while let Some(pos) = remaining.to_lowercase().find(&target_lower) {
        let (before, rest) = remaining.split_at(pos);
        result.push_str(before);
        // Use the replacement verbatim. The caller passes the
        // canonical form («Дәулет» — proper noun always capital);
        // Whisper's lowercased mishearing should be normalised UP
        // to the canonical, not preserved as lowercase.
        result.push_str(replacement);
        let consumed_bytes = a.len() + 1 + b.len();
        remaining = &rest[consumed_bytes.min(rest.len())..];
    }
    result.push_str(remaining);
    result
}

/// Replace a multi-word phrase verbatim, case-sensitive. Used for
/// trim-and-replace patterns where the casing is fixed.
fn replace_phrase(input: &str, needle: &str, replacement: &str) -> String {
    input.replace(needle, replacement)
}

/// Replace `needle` with `replacement` everywhere, preserving the
/// case of the FIRST character of each match. («Менім» → «Менің»,
/// «менім» → «менің».)
fn case_aware_replace(input: &str, needle: &str, replacement: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut remaining = input;
    let needle_lower = needle.to_lowercase();
    while let Some(pos) = remaining.to_lowercase().find(&needle_lower) {
        let (before, rest) = remaining.split_at(pos);
        out.push_str(before);
        let matched_start_upper = rest.chars().next().is_some_and(|c| c.is_uppercase());
        if matched_start_upper {
            // Capitalise first char of replacement.
            let mut chars = replacement.chars();
            if let Some(first) = chars.next() {
                for c in first.to_uppercase() {
                    out.push(c);
                }
                out.push_str(chars.as_str());
            }
        } else {
            out.push_str(&replacement.to_lowercase());
        }
        // Advance past the matched needle. needle bytes count varies
        // for Cyrillic; use the needle's byte length.
        let consumed = needle.len().min(rest.len());
        remaining = &rest[consumed..];
    }
    out.push_str(remaining);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_дау_лет_into_дәулет() {
        assert_eq!(
            normalize_kazakh_transcript("Менің атым дау лет."),
            "Менің атым Дәулет."
        );
    }

    #[test]
    fn fixes_салим_in_short_greeting() {
        assert_eq!(normalize_kazakh_transcript("Салим!"), "Сәлем!");
        assert_eq!(normalize_kazakh_transcript("салим"), "сәлем");
    }

    #[test]
    fn preserves_салим_in_long_context() {
        // Long context: probably an actual Arabic name — don't rewrite.
        let input = "Менің досымның есімі Салим, ол маған кеше қоңырау шалды.";
        assert_eq!(normalize_kazakh_transcript(input), input);
    }

    #[test]
    fn fixes_танысайыр_тим_to_танысайық() {
        assert_eq!(normalize_kazakh_transcript("Танысайыр тим"), "Танысайық");
    }

    #[test]
    fn fixes_танысайыр_alone_to_танысайық() {
        assert_eq!(normalize_kazakh_transcript("Танысайыр!"), "Танысайық!");
    }

    #[test]
    fn fixes_менім_to_менің() {
        assert_eq!(
            normalize_kazakh_transcript("Менім атым Дәулет"),
            "Менің атым Дәулет"
        );
    }

    #[test]
    fn fixes_атом_when_preceded_by_менің() {
        // Codex live-test: «Менім атом дау лет» → «Менің атым Дәулет».
        assert_eq!(
            normalize_kazakh_transcript("Менім атом дау лет."),
            "Менің атым Дәулет."
        );
    }

    #[test]
    fn preserves_атом_in_chemistry_context() {
        // «Атом — заттың ең кіші бөлшегі» (atom — smallest particle
        // of matter). Real word; don't rewrite to «атым».
        let input = "Атом — заттың ең кіші бөлшегі.";
        assert_eq!(normalize_kazakh_transcript(input), input);
    }

    #[test]
    fn fixes_есіңің_in_name_statement_context() {
        // Codex live-test: «Менің есіңің дау лет» — clearly meant
        // «Менің есімде Дәулет» (my name is Daulet, with Whisper
        // collapsing «есімде» into «есіңің»).
        assert_eq!(
            normalize_kazakh_transcript("Менің есіңің дау лет."),
            "Менің есімде Дәулет."
        );
    }

    #[test]
    fn idempotent_on_clean_input() {
        let clean = "Сәлем! Менің атым Дәулет. Танысайық.";
        assert_eq!(normalize_kazakh_transcript(clean), clean);
    }

    #[test]
    fn idempotent_double_application() {
        let raw = "Менім атом дау лет.";
        let once = normalize_kazakh_transcript(raw);
        let twice = normalize_kazakh_transcript(&once);
        assert_eq!(once, twice);
    }

    #[test]
    fn trims_artifact_тим_suffix() {
        // Whisper sometimes emits stray «тим» as a token. Drop it.
        let out = normalize_kazakh_transcript("Сәлем тим");
        assert!(!out.contains("тим"), "got: {out}");
    }
}
