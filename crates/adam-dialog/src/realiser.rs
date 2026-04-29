//! Layer 4 — response realiser.
//!
//! v0.8.0: plain `{slot}` placeholders, text substitution only.
//! v0.9.5: `{slot|features}` FST-backed expansion. `features` is a
//! `+`-separated spec (see `slot_syntax`) that describes the noun
//! case + number to synthesise via `adam_kernel_fst::morphotactics`.
//! v1.1.0: transliteration module removed (Kazakh-only surface
//! reverts the v0.9.6 multilingual decision). Non-Cyrillic slots are
//! now a bug and return the raw value unchanged.
//!
//! Examples:
//! ```text
//!   "сәлем {name}"                    → "сәлем Дәулет"
//!   "{city|locative} тұрасыз ба?"     → "Алматыда тұрасыз ба?"
//!   "{occupation|plural} қажет"       → "мұғалімдер қажет"
//! ```
//!
//! Unfilled placeholders stay visible as literal `{name}` / `{city|loc}`
//! — deliberately ugly so missing-slot bugs surface during QA rather
//! than silently dropping information.

use adam_kernel_fst::morphotactics::synthesise_noun;

use crate::planner::ResponsePlan;
use crate::slot_syntax::{parse_noun_features, parse_placeholder};

/// Render a response plan into the final output string. Scans the
/// template left-to-right, expanding `{...}` placeholders as they
/// appear; everything else is emitted verbatim.
///
/// **v4.6.1** — final-output orthographic pass: capitalise the
/// first letter of the first alphabetic codepoint. Templates are
/// authored lowercase by convention (so `{name|locative}` etc.
/// produce the expected lowercase morphology), but the surface
/// reply should start with a capital letter like every other
/// well-formed Kazakh sentence. Skips leading whitespace /
/// punctuation; preserves quote-led replies («...») by stepping
/// into the first alphabetic codepoint past the opening quote.
pub fn realise(plan: &ResponsePlan) -> String {
    let raw = expand_template(&plan.literal, &plan.slots);
    let cased = capitalise_first_letter(&raw);
    ensure_sentence_final(&cased)
}

/// **v4.6.5** — declarative replies should end with a sentence-
/// final punctuation mark. If the rendered reply ends with an
/// alphabetic character AND is at least 10 codepoints long
/// (filtering out short interjections like «иә», «сәлем» that
/// idiomatically don't take a period), append `.`. Any reply
/// already ending in `.`/`!`/`?`/`…`/`»`/`"`/`)` is left as-is.
/// Length-based gate is intentionally simple — Kazakh punctuation
/// conventions follow the same Cyrillic-orthographic rules as
/// Russian, and short interjections / one-word answers shouldn't
/// be forced to carry a period.
fn ensure_sentence_final(s: &str) -> String {
    let trimmed = s.trim_end();
    if trimmed.chars().count() < 10 {
        return s.to_string();
    }
    match trimmed.chars().last() {
        Some(last) if matches!(last, '.' | '!' | '?' | '…' | '»' | '"' | ')' | ']') => {
            s.to_string()
        }
        Some(last) if last.is_alphabetic() => {
            let mut out = trimmed.to_string();
            out.push('.');
            out
        }
        _ => s.to_string(),
    }
}

/// **v4.6.1** — uppercase the first alphabetic codepoint of the
/// reply. Cyrillic-Kazakh-aware: `қ`/`ң`/`ғ`/`ө`/`ү`/`ұ`/`һ` map to
/// `Қ`/`Ң`/`Ғ`/`Ө`/`Ү`/`Ұ`/`Һ` via standard Unicode case mapping
/// (Rust's `char::to_uppercase` handles all Cyrillic correctly).
/// Iterates past leading non-alphabetic characters (whitespace,
/// punctuation, quotation marks) so a quote-led reply still
/// capitalises the first letter of the actual word, not the quote
/// glyph. No-op on empty strings or all-non-alphabetic strings.
fn capitalise_first_letter(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 1);
    let mut chars = s.chars();
    let mut capitalised = false;
    for ch in chars.by_ref() {
        if !capitalised && ch.is_alphabetic() {
            // Push the uppercase version (may expand to multiple
            // chars in some scripts; for Kazakh Cyrillic always 1).
            for u in ch.to_uppercase() {
                out.push(u);
            }
            capitalised = true;
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod realiser_tests {
    use super::capitalise_first_letter;

    #[test]
    fn capitalises_kazakh_first_letter() {
        assert_eq!(capitalise_first_letter("сәлем"), "Сәлем");
        assert_eq!(capitalise_first_letter("қазақстан"), "Қазақстан");
        assert_eq!(capitalise_first_letter("ңайып"), "Ңайып");
        assert_eq!(capitalise_first_letter("өзгеше"), "Өзгеше");
        assert_eq!(capitalise_first_letter("үлгі"), "Үлгі");
    }

    #[test]
    fn capitalises_past_leading_punctuation() {
        assert_eq!(
            capitalise_first_letter("«қазақстан туралы»"),
            "«Қазақстан туралы»"
        );
        assert_eq!(capitalise_first_letter("  сәлем"), "  Сәлем");
        assert_eq!(capitalise_first_letter("— иә"), "— Иә");
    }

    #[test]
    fn already_capitalised_stays_capitalised() {
        assert_eq!(capitalise_first_letter("Сәлем"), "Сәлем");
    }

    #[test]
    fn no_op_on_empty_or_non_alphabetic() {
        assert_eq!(capitalise_first_letter(""), "");
        assert_eq!(capitalise_first_letter("   "), "   ");
        assert_eq!(capitalise_first_letter("!?."), "!?.");
    }

    use super::ensure_sentence_final;

    #[test]
    fn appends_period_to_declarative_reply() {
        assert_eq!(
            ensure_sentence_final("Қазақстан — Орталық Азиядағы ел"),
            "Қазақстан — Орталық Азиядағы ел."
        );
        assert_eq!(
            ensure_sentence_final("Маркпен танысқаныма қуаныштымын"),
            "Маркпен танысқаныма қуаныштымын."
        );
    }

    #[test]
    fn leaves_existing_punctuation_alone() {
        assert_eq!(ensure_sentence_final("Қазақстан — ел."), "Қазақстан — ел.");
        assert_eq!(
            ensure_sentence_final("Сіз қайда тұрасыз?"),
            "Сіз қайда тұрасыз?"
        );
        assert_eq!(ensure_sentence_final("Иә!"), "Иә!");
    }

    #[test]
    fn skips_short_interjections() {
        assert_eq!(ensure_sentence_final("Сәлем"), "Сәлем");
        assert_eq!(ensure_sentence_final("Иә"), "Иә");
        assert_eq!(ensure_sentence_final("Жақсы"), "Жақсы");
    }

    #[test]
    fn handles_quoted_or_paren_endings() {
        assert_eq!(
            ensure_sentence_final("Кітап «Абай жолы» дейді»"),
            "Кітап «Абай жолы» дейді»"
        );
    }
}

fn expand_template(template: &str, slots: &std::collections::HashMap<String, String>) -> String {
    let mut out = String::with_capacity(template.len());
    let mut i = 0;
    let bytes = template.as_bytes();
    while i < bytes.len() {
        // `{` is a single byte; safe to compare at a byte offset since
        // the preceding iterations are char-aligned.
        if bytes[i] == b'{' {
            if let Some(end_rel) = template[i + 1..].find('}') {
                let inner = &template[i + 1..i + 1 + end_rel];
                out.push_str(&expand_placeholder(inner, slots));
                i += 1 + end_rel + 1;
                continue;
            }
        }
        let ch = template[i..].chars().next().expect("valid utf-8 boundary");
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn expand_placeholder(inner: &str, slots: &std::collections::HashMap<String, String>) -> String {
    let (slot_name, feature_spec) = parse_placeholder(inner);
    let Some(root) = slots.get(slot_name) else {
        // Unfilled — leave the raw placeholder visible.
        return format!("{{{inner}}}");
    };
    match feature_spec {
        None => root.clone(),
        Some(spec) => {
            // Kazakh-only surface (v1.1.0): non-Cyrillic roots aren't
            // expected. FST phonology operates directly on whatever
            // glyphs are passed in; if a Latin-only root leaks through
            // from an upstream bug it's better surfaced as visibly
            // wrong output than silently transliterated.
            let features = parse_noun_features(spec);
            synthesise_noun(root, features)
        }
    }
}
