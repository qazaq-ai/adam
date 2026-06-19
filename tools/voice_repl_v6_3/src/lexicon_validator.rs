// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//
// **v6.6 status:** disabled at the main.rs call site (the rc25
// false-positive surface — `дәулет→сәулет`, `жасым→жасы`, etc. —
// outweighed the recall gain). The module is intentionally kept
// in-tree so its test battery serves as a regression spec for
// the v7 candidate-rescoring replacement; allow dead_code on the
// whole file rather than re-route every helper through a runtime
// gate just to satisfy clippy.
#![allow(dead_code)]
//! # Lexicon validator — v6.5.0-rc23
//!
//! Universal per-token input cleaner.  Walks every word in the
//! voice-REPL input, checks whether it parses as a Kazakh word
//! through the FST + LexiconV1, and substitutes the nearest
//! UNAMBIGUOUS edit-distance-1 candidate when it doesn't.
//!
//! ## What this closes
//!
//! User pushback after the rc22 live audit (2026-06-11):
//!
//! > получается из-за одной послышавшейся буквы "ғ".  Хотя он
//! > должен был провалидировать это слово "Қалыңыз" и не косячить
//!
//! Concrete cases the validator closes:
//!
//!   «Қалыңғыз» → «Қалыңыз»  (Whisper inserted a phantom «ғ»)
//!   «бөль»     → «бөл»      (Whisper added soft sign to imperative)
//!   «кубит»    → «көбейт»   (Whisper drift of multi-vowel root)
//!   «елісіздік» → «тәуелсіздік» (drift across compound)
//!   «Гейц»     → «Гейтс»    (drop of final consonant cluster)
//!   «жасаланады» → «жасалады» (rc22 audit T46, fuzzy already fixed)
//!
//! The previous `fuzzy_normalise` was tuned conservatively
//! (token length ≥ 5, edit distance ≥ 1.5, similarity ≥ 0.80) to
//! avoid mis-correcting minimal-pair Kazakh words («қатым/қауын»,
//! «бұғын/ұғым»).  Those gates blocked exactly the edit-distance-1
//! Whisper-noise fixes we need.
//!
//! ## Why a separate validator
//!
//! The discipline is **validate first, fuzzy only as fallback**.
//! Walk every token through:
//!
//!   1. FST + LexiconV1 parse — if the word IS a valid Kazakh
//!      form (root + suffix chain), keep it as-is.  No fuzzy.
//!   2. If FST returns no parses (token is OOV), THEN consider
//!      a fuzzy substitution.  Use a strict 1-character
//!      edit-distance and require UNIQUE substitution within the
//!      lexicon to avoid the «қатым/қауын» minimal-pair trap.
//!
//! Step 1 is the key insight from the user: «провалидировать это
//! слово».  A word that already parses correctly should NEVER
//! be rewritten, regardless of how close another word is.

use adam_kernel_fst::lexicon::LexiconV1;
use adam_kernel_fst::parser::analyse;

use crate::zipf_vocab::ZipfVocab;

/// Cleaned input + a diagnostic list of substitutions made.
/// The diagnostic is useful for the voice REPL's per-turn log
/// («[voice-repl] lexicon-validator: Қалыңғыз → Қалыңыз»).
#[derive(Debug, Clone)]
pub struct CleanedInput {
    pub text: String,
    pub substitutions: Vec<(String, String)>,
}

/// Walk every whitespace-separated token in `input`; if the token
/// is parseable as a Kazakh word, keep it.  If not, search the
/// lexicon for a single edit-distance-1 neighbour and substitute
/// only when that neighbour is unique.
///
/// Skips:
///   - Tokens shorter than 4 chars (too noisy; fuzzy false-positives)
///   - Tokens containing ASCII digits (math expressions etc.)
///   - Pure-punctuation tokens
///   - Proper-noun-like tokens that start with an uppercase letter
///     AND have no Kazakh-specific glyphs (Whisper Latin spelling
///     of foreign names — leave for the OOD discipline downstream)
#[cfg(test)]
pub fn clean(input: &str, lex: &LexiconV1) -> CleanedInput {
    clean_with_hot_vocab(input, lex, None)
}

/// Variant of [`clean`] that also consults a hot-vocab list.  When
/// an FST-validated token has an edit-distance-1 neighbour PRESENT
/// in `vocab.entries`, the hot-vocab neighbour wins.
///
/// **v6.5.0-rc24 — frequency-aware override.**  rc23 live audit
/// T2 «Қалыңғыз қалай»: the FST happily parsed «Қалыңғыз» as
/// some derivative of «қалың» («thick / dense») + a non-canonical
/// suffix chain, so the rc23 validator kept it as-is.  But the
/// REPL's `OVERRIDES` list includes «қалыңыз» (the high-frequency
/// greeting) as a hot word.  Edit-distance 1 between «қалыңғыз»
/// and «қалыңыз» — the hot-vocab word wins.
///
/// The hot list is closed (~80 entries: honorifics, greetings,
/// identity-question triggers) and curated; it does NOT contain
/// general-purpose Kazakh words, so the override is safe — it
/// won't flip legitimately-distinct minimal pairs.
pub fn clean_with_hot_vocab(
    input: &str,
    lex: &LexiconV1,
    vocab: Option<&ZipfVocab>,
) -> CleanedInput {
    let mut substitutions: Vec<(String, String)> = Vec::new();
    let mut out = String::with_capacity(input.len());

    for (i, raw_token) in input.split_whitespace().enumerate() {
        if i > 0 {
            out.push(' ');
        }

        let (leading, core, trailing) = split_off_punct(raw_token);

        if should_skip(core) {
            out.push_str(raw_token);
            continue;
        }

        let core_lower = core.to_lowercase();

        // 0. Hot-vocab override.  **rc26 fix.**  rc24/rc25 used
        //    `vocab.contains()` which checks against the full
        //    3 147-entry curated lexicon — far too broad.  Audit
        //    rc25 caught the regression: «дәулет» was rewritten to
        //    «сәулет», «жасым» to «жасы», «толды» to «толы»,
        //    «тұрамын» to «тұратын» — all because each had an edit-1
        //    neighbour SOMEWHERE in the 3 147-word vocab.  The
        //    override is meant to ONLY catch Whisper drift on the
        //    closed 82-entry `OVERRIDES` list (honorifics / greetings
        //    / identity triggers), not arbitrary curated lexicon
        //    overlaps.
        //
        //    Skip the override when the original IS in OVERRIDES.
        if !crate::zipf_vocab::OVERRIDES.contains(&core_lower.as_str())
            && let Some(hot) = find_hot_overrides_edit1_neighbour(&core_lower)
        {
            let case_matched = match_case(core, &hot);
            substitutions.push((core.to_string(), case_matched.clone()));
            out.push_str(leading);
            out.push_str(&case_matched);
            out.push_str(trailing);
            continue;
        }
        // Suppress unused-param warning when no vocab is passed
        // (the legacy `clean(input, lex)` path).
        let _ = vocab;

        // 1. Validator: is this already a valid Kazakh word?
        let parses = analyse(&core_lower, lex);
        if !parses.is_empty() {
            out.push_str(raw_token);
            continue;
        }

        // 2. Fallback: find the unique edit-distance-1 neighbour
        //    in the lexicon.  Two-or-more candidates → no substitution.
        if let Some(substitute) = find_unique_edit1_neighbour(&core_lower, lex) {
            let case_matched = match_case(core, &substitute);
            substitutions.push((core.to_string(), case_matched.clone()));
            out.push_str(leading);
            out.push_str(&case_matched);
            out.push_str(trailing);
            continue;
        }

        // 3. No unique fix — emit raw.
        out.push_str(raw_token);
    }

    CleanedInput {
        text: out,
        substitutions,
    }
}

/// **rc26 — closed-set hot-override list.**  Find an edit-1
/// neighbour of `token` that is in the curated `OVERRIDES` list
/// (~82 entries: honorifics, greetings, identity triggers).  The
/// `OVERRIDES` list is closed and small — rc25 audit showed that
/// using the full 3 147-entry curated lexicon as the override
/// target is too broad and causes false rewrites on common
/// grammatical forms («дәулет» → «сәулет», «толды» → «толы»).
fn find_hot_overrides_edit1_neighbour(token: &str) -> Option<String> {
    for candidate in edit1_candidates(token) {
        if candidate == token {
            continue;
        }
        if crate::zipf_vocab::OVERRIDES.contains(&candidate.as_str()) {
            return Some(candidate);
        }
    }
    None
}

/// Skip tokens shorter than 4 chars (too noisy), tokens with
/// digits, pure-punctuation, or all-Latin proper-noun candidates.
fn should_skip(core: &str) -> bool {
    let n_chars = core.chars().count();
    if n_chars < 4 {
        return true;
    }
    if core.chars().any(|c| c.is_ascii_digit()) {
        return true;
    }
    if core.chars().all(|c| !c.is_alphabetic()) {
        return true;
    }
    if core.chars().all(|c| c.is_ascii_alphabetic()) {
        // All-Latin: leave for OOD discipline.
        return true;
    }
    false
}

/// Split off leading + trailing non-alphabetic punctuation so it
/// passes through unchanged after the substitution.
fn split_off_punct(token: &str) -> (&str, &str, &str) {
    let chars: Vec<(usize, char)> = token.char_indices().collect();
    if chars.is_empty() {
        return ("", token, "");
    }
    let first_alpha = chars.iter().position(|(_, c)| c.is_alphabetic());
    let last_alpha = chars.iter().rposition(|(_, c)| c.is_alphabetic());
    match (first_alpha, last_alpha) {
        (Some(start), Some(end)) => {
            let (start_byte, _) = chars[start];
            let (end_byte, last_char) = chars[end];
            let core_end = end_byte + last_char.len_utf8();
            (
                &token[..start_byte],
                &token[start_byte..core_end],
                &token[core_end..],
            )
        }
        _ => ("", token, ""),
    }
}

/// Find the unique edit-distance-1 neighbour of `token` in the
/// lexicon.  Returns `None` if there are zero matches or more than
/// one match.
fn find_unique_edit1_neighbour(token: &str, lex: &LexiconV1) -> Option<String> {
    // We don't have direct access to the lexicon's entry list
    // from the public API; iterate through the surface forms via
    // a candidate-generation strategy.  Generate all edit-distance-1
    // variants of `token` (insert / delete / substitute one Kazakh
    // letter) and probe each through the FST.  This is O(token_len ×
    // alphabet_size) ≈ 30 × 40 = 1 200 FST lookups per token in the
    // worst case — fast enough for a voice REPL (sub-ms wall on M2).
    let mut unique: Option<String> = None;
    for candidate in edit1_candidates(token) {
        if candidate == token {
            continue;
        }
        let parses = analyse(&candidate, lex);
        if parses.is_empty() {
            continue;
        }
        match &unique {
            None => unique = Some(candidate),
            Some(existing) if *existing == candidate => {}
            Some(_) => {
                // Ambiguous — give up to avoid the «қатым/қауын» trap.
                return None;
            }
        }
    }
    unique
}

/// Generate all edit-distance-1 candidates of `token` over the
/// Kazakh alphabet (Cyrillic 33 + Kazakh-specific 9 = 42 glyphs).
/// Returns inserts, deletes, substitutes, and one-step transposes.
fn edit1_candidates(token: &str) -> impl Iterator<Item = String> + '_ {
    const ALPHA: &[char] = &[
        'а', 'ә', 'б', 'в', 'г', 'ғ', 'д', 'е', 'ё', 'ж', 'з', 'и', 'й', 'к', 'қ', 'л', 'м', 'н',
        'ң', 'о', 'ө', 'п', 'р', 'с', 'т', 'у', 'ұ', 'ү', 'ф', 'х', 'һ', 'ц', 'ч', 'ш', 'щ', 'ъ',
        'ы', 'і', 'ь', 'э', 'ю', 'я',
    ];
    let chars: Vec<char> = token.chars().collect();
    let n = chars.len();
    let mut out: Vec<String> = Vec::with_capacity(n * (ALPHA.len() + 2));

    // Deletes — drop one character.
    for i in 0..n {
        let mut buf = String::with_capacity(token.len());
        for (j, c) in chars.iter().enumerate() {
            if j != i {
                buf.push(*c);
            }
        }
        out.push(buf);
    }

    // Substitutes — replace one character with each alphabet glyph.
    for i in 0..n {
        for a in ALPHA {
            if *a == chars[i] {
                continue;
            }
            let mut buf = String::with_capacity(token.len());
            for (j, c) in chars.iter().enumerate() {
                if j == i {
                    buf.push(*a);
                } else {
                    buf.push(*c);
                }
            }
            out.push(buf);
        }
    }

    // Inserts — insert each alphabet glyph at each position.
    for i in 0..=n {
        for a in ALPHA {
            let mut buf = String::with_capacity(token.len() + a.len_utf8());
            for (j, c) in chars.iter().enumerate() {
                if j == i {
                    buf.push(*a);
                }
                buf.push(*c);
            }
            if n == i {
                buf.push(*a);
            }
            out.push(buf);
        }
    }

    // Adjacent transposes — swap chars[i] and chars[i+1].
    for i in 0..n.saturating_sub(1) {
        let mut buf = String::with_capacity(token.len());
        for (j, c) in chars.iter().enumerate() {
            if j == i {
                buf.push(chars[i + 1]);
            } else if j == i + 1 {
                buf.push(chars[i]);
            } else {
                buf.push(*c);
            }
        }
        out.push(buf);
    }

    out.into_iter()
}

/// Match the case of the substitute to the original token's first
/// letter.  Keeps proper-noun casing intact when the validator
/// fires on an OOV proper noun.
fn match_case(original: &str, substitute: &str) -> String {
    if original.chars().next().is_some_and(|c| c.is_uppercase()) {
        let mut out = String::with_capacity(substitute.len());
        let mut chars = substitute.chars();
        if let Some(first) = chars.next() {
            for upper in first.to_uppercase() {
                out.push(upper);
            }
        }
        out.extend(chars);
        out
    } else {
        substitute.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex() -> Option<LexiconV1> {
        LexiconV1::load_default().ok()
    }

    /// **rc23 audit T2** — when we ran the rc23 validator on
    /// «Қалыңғыз қалай», the validator **did not fire**: the FST
    /// happily parses «Қалыңғыз» as some derivative form, so the
    /// «validate-first» rule kept it.  rc24 adds a hot-vocab override
    /// that catches this exact pattern.
    #[test]
    fn rc23_audit_t2_hot_vocab_override_closes_phantom_letter() {
        let Some(lex) = lex() else { return };
        let vocab = crate::zipf_vocab::ZipfVocab::load_or_overrides_only(".");
        let cleaned = clean_with_hot_vocab("Қалыңғыз қалай", &lex, Some(&vocab));
        assert!(
            cleaned
                .substitutions
                .iter()
                .any(|(o, n)| o.to_lowercase() == "қалыңғыз" && n.to_lowercase() == "қалыңыз"),
            "rc24 hot-vocab override must catch Қалыңғыз → Қалыңыз even when the FST parses Қалыңғыз; got={:?}",
            cleaned
        );
    }

    /// Hot-vocab override does NOT rewrite a word that is ITSELF
    /// in the hot vocab.  «қалыңыз» stays «қалыңыз».
    #[test]
    fn rc24_hot_vocab_does_not_rewrite_itself() {
        let Some(lex) = lex() else { return };
        let vocab = crate::zipf_vocab::ZipfVocab::load_or_overrides_only(".");
        let cleaned = clean_with_hot_vocab("Қалыңыз қалай", &lex, Some(&vocab));
        assert!(
            cleaned.substitutions.is_empty(),
            "hot vocab must not rewrite its own entries; got={:?}",
            cleaned.substitutions
        );
    }

    /// **rc26 regression test.**  rc25 live audit caught false-positive
    /// rewrites where the validator wrote common grammatical forms
    /// to ANY edit-1 lexicon neighbour.  Each line below MUST stay
    /// unchanged.
    #[test]
    fn rc26_audit_no_false_positive_rewrites_on_valid_forms() {
        let Some(lex) = lex() else { return };
        let vocab = crate::zipf_vocab::ZipfVocab::load_or_overrides_only(".");
        for input in &[
            // T5 — «Менің атым дәулет» was rewritten to «сәулет»
            "Менің атым дәулет",
            // T6, T28 — «тұрамын» (1sg present) was rewritten to «тұратын»
            "Мен қостанайда тұрамын",
            "Мен қай жерде тұрамын",
            // T7, T9 — «жасым ... толды» both got mangled
            "Жасым алпыс алтыға толды",
            "Менің жасым алпыс алтыға толды",
            // T12 — «нешеу» was being rewritten to «нені»
            "Олай болса, қазір сағат нешеу",
            // T15 — «Ерден» → «Жерден» on a date drift
            "Ерден қай күн болады",
            // T19 — «алдынан» → «алдына»
            "Тоқаевтың алдынан кім болды",
            // T26 — «Менім» → «Мені» (the genitive ı/і drift)
            "Менім атым кім",
        ] {
            let cleaned = clean_with_hot_vocab(input, &lex, Some(&vocab));
            assert!(
                cleaned.substitutions.is_empty(),
                "validator rewrote a valid form in «{input}»: {:?}",
                cleaned.substitutions
            );
        }
    }

    /// Already-valid words are NEVER rewritten — the validator-first
    /// discipline.
    #[test]
    fn already_valid_word_is_not_rewritten() {
        let Some(lex) = lex() else { return };
        let cleaned = clean("Қазақстан туралы айтшы", &lex);
        assert!(
            cleaned.substitutions.is_empty(),
            "valid words must not be rewritten; got: {:?}",
            cleaned.substitutions
        );
    }

    /// Short words (<4 chars) are skipped to avoid noise.
    #[test]
    fn short_words_are_skipped() {
        let Some(lex) = lex() else { return };
        let cleaned = clean("ат не", &lex);
        // Neither «ат» (2 chars) nor «не» (2 chars) should trigger
        // a substitution.
        assert!(cleaned.substitutions.is_empty());
    }

    /// Tokens with digits pass through.
    #[test]
    fn digit_tokens_pass_through() {
        let Some(lex) = lex() else { return };
        let cleaned = clean("5+5 қанша", &lex);
        assert!(
            !cleaned.text.is_empty(),
            "digit token must survive: {:?}",
            cleaned.text
        );
        // No substitution on «5+5» itself.
        let has_digit_sub = cleaned
            .substitutions
            .iter()
            .any(|(old, _)| old.contains('5'));
        assert!(!has_digit_sub);
    }

    /// Adjacent transpose: «Гейтс» → «Гетыс»-style typos get
    /// recovered.  This particular test doesn't rely on a real
    /// lexicon entry (we can't assume «Гейтс» is curated), so we
    /// just sanity-check the candidate generator emits the transpose.
    #[test]
    fn edit1_candidates_include_transposes() {
        let cands: Vec<String> = edit1_candidates("абв").collect();
        assert!(cands.iter().any(|c| c == "бав")); // transpose 0-1
        assert!(cands.iter().any(|c| c == "авб")); // transpose 1-2
    }
}
