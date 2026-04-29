//! v4.6.0 — Discourse-anaphora resolution.
//!
//! Kazakh (like English) uses **discourse demonstratives** to refer
//! back to a topic introduced in a previous turn. The most common
//! standalone forms:
//!
//! | Surface | Gloss | Resolves to |
//! |---|---|---|
//! | `онда` | "in it / there" | previous turn's topic, locative |
//! | `сонда` | "in that / there" | same |
//! | `осында` | "in this / here" | same |
//! | `мұнда` | "in this / here" | same |
//! | `бұнда` | "in this" | same |
//! | `одан` | "from it" | same, ablative |
//! | `содан` | "from that" | same |
//! | `бұдан` | "from this" | same |
//! | `осыдан` | "from this" | same |
//!
//! Pre-v4.6.0 these surfaces were in `NOT_A_TOPIC` to suppress the
//! FST misanalysis (e.g. `онда` parsing as `он + Locative`), which
//! prevented adam from picking up `Он` as a phantom topic. Good as
//! a defence; but it also meant `онда` carried **no semantic
//! signal** — the system silently lost the discourse anaphor.
//!
//! This module adds the missing signal by tracking the **last
//! query topic** across turns. When the user's input contains a
//! discourse anaphor (per the surface list above) and `best_noun_hint`
//! returned None for the current turn, the conversation layer
//! looks up the previous turn's topic and reuses it. So:
//!
//! ```text
//! T1: «Қазақстан туралы не білесіз?»  → topic = қазақстан, surfaced as fact
//! T2: «Ал онда қанша аймақ бар?»     → topic = (anaphora) → қазақстан
//!                                       → answers about Kazakhstan, not «он»
//! ```
//!
//! Implementation is intentionally simple — single-slot LRU. No
//! attempt to model coreference chains, multiple referents, or
//! discourse stacks. The single most-recent topic covers the
//! 80%-case observed in real REPL traces; richer modelling is
//! deferred to a future minor.

/// Discourse-anaphor surface forms (lowercased) that signal
/// "refer to the previous turn's topic". Kept aligned with the
/// `NOT_A_TOPIC` entries added in v4.3.5 (which suppress the
/// FST misanalysis side) — these are the FORMS that carry
/// anaphoric meaning.
const DISCOURSE_ANAPHORS: &[&str] = &[
    "онда",
    "сонда",
    "осында",
    "мұнда",
    "бұнда",
    "одан",
    "содан",
    "бұдан",
    "осыдан",
];

/// Returns `true` if any whitespace-separated lowercase token of
/// the input matches a known discourse anaphor. The check is
/// intentionally surface-level — we don't want to lean on the
/// FST here because the FST's analysis of these forms is exactly
/// what `NOT_A_TOPIC` suppresses.
pub fn input_contains_discourse_anaphor(input: &str) -> bool {
    let lower = input.to_lowercase();
    lower
        .split(|c: char| !c.is_alphabetic())
        .any(|word| DISCOURSE_ANAPHORS.contains(&word))
}

/// Russian function-word markers — common high-frequency Russian
/// pronouns / particles / question words that don't have an
/// orthographic homograph in Kazakh. Matching any of these is a
/// strong signal the user is typing Russian, not Kazakh. Used by
/// `input_is_likely_russian` below.
const RUSSIAN_MARKERS: &[&str] = &[
    "это",
    "что",
    "кто",
    "как",
    "где",
    "почему",
    "когда",
    "тебя",
    "себя",
    "тебе",
    "меня",
    "мне",
    "тоже",
    "очень",
    "круто",
    "спасибо",
    "пожалуйста",
    "привет",
    "пока",
    "также",
    "если",
    "потому",
    "поэтому",
    "сейчас",
    "сегодня",
];

/// **v4.6.12** — surface-level Russian-input detection. Real-REPL
/// 2026-04-29 transcript carried «Это очень круто, а кто тебя
/// создал?» — adam parsed it partially, surfaced a half-Russian
/// half-Kazakh refusal which violates the Kazakh-only directive
/// (`project_kazakh_only_directive`). adam should refuse such
/// inputs cleanly with a "Kazakh-only" message.
///
/// The detector matches on **two signals**:
/// 1. Any of `RUSSIAN_MARKERS` appears as a whitespace-separated
///    token (high-frequency Russian function words that don't
///    overlap with common Kazakh).
/// 2. The input contains **no Kazakh-specific letters**
///    (`ә / ң / ғ / ө / ү / ұ / қ / і / һ`). A real Kazakh
///    sentence almost always carries at least one of these even
///    in short utterances; their absence + Russian-marker
///    presence is a confident "not Kazakh" signal.
///
/// Conservative by design — this short-circuits adam's normal
/// pipeline only when both signals fire. Mixed code-switching
/// inputs (Kazakh sentence with one Russian word) still flow
/// through the standard path; only obviously-Russian inputs
/// route to the dedicated refusal.
pub fn input_is_likely_russian(input: &str) -> bool {
    let lower = input.to_lowercase();
    let has_russian_marker = lower
        .split(|c: char| !c.is_alphabetic())
        .any(|word| RUSSIAN_MARKERS.contains(&word));
    if !has_russian_marker {
        return false;
    }
    let has_kazakh_specific = lower
        .chars()
        .any(|c| matches!(c, 'ә' | 'ң' | 'ғ' | 'ө' | 'ү' | 'ұ' | 'қ' | 'і' | 'һ'));
    !has_kazakh_specific
}

#[cfg(test)]
mod russian_tests {
    use super::input_is_likely_russian;

    #[test]
    fn detects_russian_only_input() {
        assert!(input_is_likely_russian("Это очень круто"));
        assert!(input_is_likely_russian("Кто тебя создал?"));
        assert!(input_is_likely_russian("Привет, как дела?"));
        assert!(input_is_likely_russian("Спасибо большое"));
    }

    #[test]
    fn does_not_match_kazakh_input() {
        // Real Kazakh — at least one ә/ң/ғ/ө/ү/ұ/қ/і/һ.
        assert!(!input_is_likely_russian("Қазақстан туралы не білесіз?"));
        assert!(!input_is_likely_russian("Сәлем"));
        assert!(!input_is_likely_russian("Менің атым Дәулет"));
        assert!(!input_is_likely_russian("Алматыда тұрамын"));
    }

    #[test]
    fn does_not_match_mixed_codeswitch() {
        // Mixed input with at least one Kazakh-specific letter
        // stays on the standard pipeline (no Russian short-circuit).
        // The Russian word appears but the sentence is still mostly
        // Kazakh per the orthographic signal.
        assert!(!input_is_likely_russian("Сәлем, как дела?"));
    }

    #[test]
    fn empty_or_no_markers_is_not_russian() {
        assert!(!input_is_likely_russian(""));
        assert!(!input_is_likely_russian("123"));
        assert!(!input_is_likely_russian("xyz abc"));
    }
}

/// **v4.6.12** — math-expression detection. Real-REPL 2026-04-29
/// transcript carried «5+5» / «7 + 3 =» / «6:2=» / «5-ті 7-ге
/// көбейткенде неше болады?» / «алтыны екіге бөліңіз» — adam
/// surfaced tangential proverbs (the system extracted whatever
/// noun leaked through). adam doesn't compute math (per
/// `limitations_summary`), so these inputs should refuse cleanly
/// with the dedicated `math_refusal` template family.
///
/// Detector matches on **two signals**:
/// 1. Arithmetic operators (`+`, `-`, `*`, `/`, `:`, `=`) appear
///    between digits or near digits.
/// 2. Kazakh math verbs / nouns (`көбейту / көбейткенде /
///    көбейтсем / бөлу / бөліңіз / қосу / қоссаңыз / алу /
///    алыңыз / есептеу`) appear alongside numeric tokens.
///
/// Conservative — fires only on clear math-input shapes. Pure
/// numerics like «17» (e.g. asking about Kazakhstan's 17 oblasts)
/// don't fire because they're not paired with operators or math
/// verbs.
pub fn input_is_math_expression(input: &str) -> bool {
    let lower = input.to_lowercase();
    // Signal 1: arithmetic operator surrounded by digit context.
    let has_arithmetic_form = {
        let bytes = input.as_bytes();
        let mut found = false;
        for (i, &b) in bytes.iter().enumerate() {
            if matches!(b, b'+' | b'-' | b'*' | b'/' | b':' | b'=') {
                // Look for a digit within 3 bytes either side.
                let near_digit_left = bytes
                    .iter()
                    .skip(i.saturating_sub(3))
                    .take(3)
                    .any(|&c| c.is_ascii_digit());
                let near_digit_right = bytes
                    .iter()
                    .skip(i + 1)
                    .take(3)
                    .any(|&c| c.is_ascii_digit());
                if near_digit_left || near_digit_right {
                    found = true;
                    break;
                }
            }
        }
        found
    };
    if has_arithmetic_form {
        return true;
    }
    // Signal 2: math verb/noun + presence of numeric tokens
    // (digit-only or Kazakh numeral words).
    const MATH_VERBS: &[&str] = &[
        "көбейту",
        "көбейтсем",
        "көбейтсеңіз",
        "көбейткенде",
        "көбейтіңіз",
        "бөлу",
        "бөлсем",
        "бөлсеңіз",
        "бөліңіз",
        "бөлгенде",
        "қосу",
        "қоссам",
        "қоссаңыз",
        "қосыңыз",
        "қосқанда",
        "алу",
        "алсам",
        "алсаңыз",
        "алыңыз",
        "алғанда",
        "есепте",
        "есептеңіз",
        "есептесеңіз",
    ];
    const KAZAKH_NUMERALS: &[&str] = &[
        "бір",
        "екі",
        "үш",
        "төрт",
        "бес",
        "алты",
        "жеті",
        "сегіз",
        "тоғыз",
        "он",
        "жиырма",
        "отыз",
        "қырық",
        "елу",
        "алпыс",
        "жетпіс",
        "сексен",
        "тоқсан",
        "жүз",
        "мың",
    ];
    let words: Vec<&str> = lower.split(|c: char| !c.is_alphabetic()).collect();
    let has_math_verb = words
        .iter()
        .any(|w| MATH_VERBS.iter().any(|v| !w.is_empty() && w.starts_with(v)));
    if !has_math_verb {
        return false;
    }
    let has_digit = input.chars().any(|c| c.is_ascii_digit());
    // **v4.6.12** — match inflected forms via prefix. Real-REPL
    // input «алтыны екіге бөліңіз» — `алтыны` is `алты` +
    // accusative, `екіге` is `екі` + dative. A pure
    // `KAZAKH_NUMERALS.contains(&w)` check misses both. Allowing
    // a numeral as a 2-4-char prefix of the surface word covers
    // case-inflected forms without false-positive matching on
    // unrelated content nouns (no Kazakh numeral overlaps with a
    // common content-noun stem of the same prefix length).
    let has_numeral_word = words.iter().any(|w| {
        if w.is_empty() {
            return false;
        }
        KAZAKH_NUMERALS
            .iter()
            .any(|n| w.starts_with(n) && w.chars().count() <= n.chars().count() + 3)
    });
    has_digit || has_numeral_word
}

/// **v4.6.15** — best-effort integer-arithmetic evaluator.
/// Pure deterministic computation — no novel-text generation, so
/// the no-fabrication invariant stays intact. Handles `+`, `-`,
/// `*`, `/`, `:` (Russian-style division) over signed integers
/// with operator precedence (`*` `/` `:` before `+` `-`). Returns:
///
/// - `Some(value)` for parseable pure-arithmetic input (e.g.
///   `5+5`, `7 * 3`, `100 - 25 / 5`, `7 + 3 =`, `6:2=`).
/// - `None` for unparseable input, division by zero, integer
///   overflow, or any non-integer division remainder. The planner
///   falls back to the existing `math_refusal` template family
///   when this returns `None`.
///
/// Limitations (intentional — keep the v4.6.15 scope tight):
/// - Integer-only; no fractions / decimals.
/// - No parentheses.
/// - No Kazakh-language phrasings («Алтыны екіге бөліңіз») — those
///   continue to refuse via `math_refusal`.
/// - No variables / session-bound computation.
pub fn try_evaluate_arithmetic(input: &str) -> Option<i64> {
    // Normalise: strip whitespace, drop trailing `=`, normalise
    // Russian-style division `:` to `/`.
    let cleaned: String = input
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| if c == ':' { '/' } else { c })
        .collect();
    let cleaned = cleaned.trim_end_matches('=').trim_end_matches('?');
    if cleaned.is_empty() {
        return None;
    }
    // Reject if any non-arithmetic character is present (digit /
    // operator / leading minus only).
    if !cleaned
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '+' | '-' | '*' | '/'))
    {
        return None;
    }
    // Tokenise into numbers and operators.
    let mut tokens: Vec<ArithToken> = Vec::new();
    let bytes = cleaned.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_digit() {
            // Read full number.
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            let n: i64 = cleaned[start..i].parse().ok()?;
            tokens.push(ArithToken::Num(n));
        } else if matches!(b, b'+' | b'-' | b'*' | b'/') {
            // Leading `-` (start of expression) or `-` after another
            // operator → unary minus on the next number.
            let unary = b == b'-'
                && (tokens.is_empty() || matches!(tokens.last(), Some(ArithToken::Op(_))));
            if unary {
                // Read the digits that follow as a negative number.
                i += 1;
                let start = i;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                if start == i {
                    return None;
                }
                let n: i64 = cleaned[start..i].parse().ok()?;
                tokens.push(ArithToken::Num(-n));
            } else {
                tokens.push(ArithToken::Op(b as char));
                i += 1;
            }
        } else {
            return None;
        }
    }
    // Two-pass evaluation: first pass collapses `*` `/` left-to-right,
    // second pass collapses `+` `-`.
    let mut acc: Vec<ArithToken> = Vec::new();
    let mut iter = tokens.into_iter();
    let first = iter.next()?;
    acc.push(first);
    while let Some(tok) = iter.next() {
        match tok {
            ArithToken::Op(op) if op == '*' || op == '/' => {
                let right = iter.next()?;
                let left = acc.pop()?;
                let (l, r) = match (left, right) {
                    (ArithToken::Num(l), ArithToken::Num(r)) => (l, r),
                    _ => return None,
                };
                let result = if op == '*' {
                    l.checked_mul(r)?
                } else {
                    if r == 0 || l % r != 0 {
                        return None;
                    }
                    l.checked_div(r)?
                };
                acc.push(ArithToken::Num(result));
            }
            _ => acc.push(tok),
        }
    }
    // Second pass: + / -.
    let mut iter = acc.into_iter();
    let mut value = match iter.next()? {
        ArithToken::Num(n) => n,
        ArithToken::Op(_) => return None,
    };
    while let Some(op_tok) = iter.next() {
        let op = match op_tok {
            ArithToken::Op(c) if c == '+' || c == '-' => c,
            _ => return None,
        };
        let right = match iter.next()? {
            ArithToken::Num(n) => n,
            ArithToken::Op(_) => return None,
        };
        value = if op == '+' {
            value.checked_add(right)?
        } else {
            value.checked_sub(right)?
        };
    }
    Some(value)
}

#[derive(Debug, Clone, Copy)]
enum ArithToken {
    Num(i64),
    Op(char),
}

#[cfg(test)]
mod arithmetic_tests {
    use super::try_evaluate_arithmetic;

    #[test]
    fn evaluates_pure_arithmetic() {
        assert_eq!(try_evaluate_arithmetic("5+5"), Some(10));
        assert_eq!(try_evaluate_arithmetic("7 + 3 ="), Some(10));
        assert_eq!(try_evaluate_arithmetic("6:2="), Some(3));
        assert_eq!(try_evaluate_arithmetic("100-25"), Some(75));
        assert_eq!(try_evaluate_arithmetic("12*4"), Some(48));
    }

    #[test]
    fn respects_operator_precedence() {
        // 2 + 3 * 4 = 2 + 12 = 14 (multiplication first)
        assert_eq!(try_evaluate_arithmetic("2+3*4"), Some(14));
        // 100 - 25 / 5 = 100 - 5 = 95
        assert_eq!(try_evaluate_arithmetic("100-25/5"), Some(95));
    }

    #[test]
    fn handles_division_zero_and_remainder() {
        assert_eq!(try_evaluate_arithmetic("5/0"), None);
        assert_eq!(try_evaluate_arithmetic("7/2"), None); // non-integer
    }

    #[test]
    fn handles_unary_minus() {
        assert_eq!(try_evaluate_arithmetic("-5"), Some(-5));
        assert_eq!(try_evaluate_arithmetic("10+-3"), Some(7));
        assert_eq!(try_evaluate_arithmetic("10*-3"), Some(-30));
    }

    #[test]
    fn rejects_non_arithmetic_input() {
        assert_eq!(try_evaluate_arithmetic("5-ті 7-ге көбейткенде"), None);
        assert_eq!(try_evaluate_arithmetic("Алтыны екіге бөліңіз"), None);
        assert_eq!(try_evaluate_arithmetic("hello"), None);
        assert_eq!(try_evaluate_arithmetic(""), None);
    }
}

#[cfg(test)]
mod math_tests {
    use super::input_is_math_expression;

    #[test]
    fn detects_pure_arithmetic() {
        assert!(input_is_math_expression("5+5"));
        assert!(input_is_math_expression("7 + 3 ="));
        assert!(input_is_math_expression("6:2="));
        assert!(input_is_math_expression("12 * 4"));
    }

    #[test]
    fn detects_kazakh_math_verb_with_numerals() {
        assert!(input_is_math_expression(
            "5-ті 7-ге көбейткенде неше болады?"
        ));
        assert!(input_is_math_expression("Алтыны екіге бөліңіз"));
        assert!(input_is_math_expression("Үшке төртті қоссаңыз"));
    }

    #[test]
    fn does_not_match_non_math_kazakh() {
        assert!(!input_is_math_expression("Қазақстанда 17 облыс бар."));
        assert!(!input_is_math_expression("Менің жасым 30"));
        assert!(!input_is_math_expression("Алты қаласы Қазақстанда"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_standalone_discourse_anaphors() {
        for input in [
            "Ал онда қанша аймақ бар?",
            "Сонда не бар?",
            "Осында тұрамын",
            "Мұнда барлығы жақсы",
            "Бұнда да солай",
            "Одан кейін не?",
            "Содан соң айт",
            "Бұдан шықпайды",
            "Осыдан бастаймыз",
        ] {
            assert!(
                input_contains_discourse_anaphor(input),
                "input {input:?} must register as carrying a discourse anaphor"
            );
        }
    }

    #[test]
    fn does_not_match_unrelated_words() {
        for input in ["Қазақстан туралы не білесіз?", "Алматы — қала", "Сәлем"]
        {
            assert!(
                !input_contains_discourse_anaphor(input),
                "input {input:?} must NOT register as discourse anaphor"
            );
        }
    }

    #[test]
    fn handles_punctuation_around_anaphor() {
        assert!(input_contains_discourse_anaphor("Ал онда?"));
        assert!(input_contains_discourse_anaphor("онда, иә"));
        assert!(input_contains_discourse_anaphor("онда."));
    }
}
