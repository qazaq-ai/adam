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
    // Locative-case anaphors (v4.6.0).
    "онда",
    "сонда",
    "осында",
    "мұнда",
    "бұнда",
    // Ablative-case anaphors (v4.6.0).
    "одан",
    "содан",
    "бұдан",
    "осыдан",
    // **v4.13.0** — Accusative/Dative/Genitive-case anaphors.
    // 2026-05-01 live REPL transcript: «Сіз Rust-ты білесіз бе?»
    // followed by «Сіз оны бағдарламалай аласыз ба?» — `оны` is
    // the accusative pronoun "it" (Rust as direct object) and
    // pre-v4.13.0 was not in the anaphor list, so the previous
    // turn's topic could not be used to resolve the reference.
    // Adding the four cases (Acc/Dat/Gen + bare) for the three
    // demonstrative stems (о-/со-/мұ-/бұ-).
    "оны",
    "соны",
    "мұны",
    "бұны",
    "оған",
    "соған",
    "мұған",
    "бұған",
    "оның",
    "соның",
    "мұның",
    "бұның",
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

/// **v4.6.20** — discourse preambles. Surface forms a Kazakh
/// speaker uses to introduce the actual question/statement that
/// follows. Pre-v4.6.20 a sentence like
/// «Айтайын дегенім, қолданыстағы жасанды интеллект модельдерінен
/// қалай жақсырақ бола аласыз?» had its first content noun
/// (`қолданыс`) grabbed by the greedy noun-hint extractor — adam
/// answered with a contract-template quote about `usage`,
/// completely missing the actual question. Stripping the preamble
/// leaves only the meaningful clause for downstream parsing.
///
/// Each entry is a lowercase preamble that, when matched at the
/// start of the input, is removed up to (and including) the next
/// clause separator (`,`, `—`, `:`, `;`). The list is checked
/// longest-first by `strip_preamble` so longer phrases like
/// `қысқаша айтқанда` always win over shorter prefixes.
const PREAMBLES: &[&str] = &[
    "айтайын дегенім",
    "айтайын дегенімді",
    "айтайын деп тұрғаным",
    "айтайын деп едім",
    "қысқаша айтқанда",
    "ашығын айтқанда",
    "шындығына келгенде",
    "сұрағым мынау",
    "сұрағым мынадай",
    "сұрақ мынадай",
    "сұрағым бар",
    "сұрағым келгені",
    "білгім келгені",
    "білгім келеді",
    "түсінсем дейтінім",
    "ойымдағы сұрақ",
    "айтпағым",
    "айтпақшы",
    "шынында",
    "шындап келгенде",
    "жалпы алғанда",
    "жалпы айтқанда",
    "иә, айтпақшы",
    "айта кетсем",
];

/// Clause-separator characters that terminate a preamble. The
/// punctuation char is consumed too so the residual starts at the
/// next non-whitespace character.
const PREAMBLE_SEPARATORS: &[char] = &[',', '—', '–', '-', ':', ';'];

/// **v4.6.20** — strip a leading discourse preamble from the
/// input, returning the residual. If the input does not start with
/// a known preamble (or has no clause separator after it), returns
/// the input unchanged. Trim preserves the user's original casing
/// of the residual; only leading whitespace is dropped.
///
/// Pure surface-level — no FST, no parsing. The preamble list is
/// closed and audited; expanding it is a v4.6.x patch.
pub fn strip_preamble(input: &str) -> &str {
    let trimmed = input.trim_start();
    let lower = trimmed.to_lowercase();
    // Sort longest-first via length comparison on the matched prefix.
    let mut best: Option<usize> = None;
    for &p in PREAMBLES {
        if lower.starts_with(p) {
            // Need a clause separator after the preamble.
            let after = &trimmed[p.len()..];
            if let Some(sep_pos) = after.find(|c: char| PREAMBLE_SEPARATORS.contains(&c)) {
                let cut = p.len() + sep_pos + after[sep_pos..].chars().next().unwrap().len_utf8();
                if best.map_or(true, |b| cut > b) {
                    best = Some(cut);
                }
            }
        }
    }
    if let Some(cut) = best {
        trimmed[cut..].trim_start()
    } else {
        input
    }
}

/// **v4.11.5** — leading vocative addressees (`адам`, `адамым`,
/// `адам-ау`, `адам ау`) declared longest-first. The match list
/// is closed: vocative use of "адам" addressing the system itself,
/// not the common-noun "адам" (= person/human) which must remain
/// available as a topic when not at clause-initial position.
const ADDRESSEES: &[&str] = &["адам-ау", "адам ау", "адамым", "адам"];

/// Punctuation that terminates a vocative form. Encompasses comma
/// (canonical), exclamation, em/en dash, hyphen, colon, semicolon.
const ADDRESSEE_SEPARATORS: &[char] = &[',', '!', '—', '–', '-', ':', ';'];

/// **v4.11.5** — strip a leading vocative addressee from the input,
/// returning the residual. Real-REPL 2026-04-30: «Адам, сен
/// мектептің физика бағдарламасын білесің бе?» — pre-v4.11.5 the
/// noun-hint extractor took the vocative `адам` itself as the topic
/// and answered with `адам IsA сүтқоректі`, completely missing the
/// actual subject of the question (`физика бағдарламасы`).
///
/// Recognises two clause shapes after the vocative:
/// 1. **punctuation-separated** — `Адам, …` / `Адам! …` / `Адам — …`
///    (any `ADDRESSEE_SEPARATORS` char). Strips the vocative AND the
///    separator.
/// 2. **bare-pronoun continuation** — `Адам сен …` / `Адам сіз …`
///    (vocative + space + 2nd-person pronoun). Strips just the
///    vocative; the pronoun stays so 1sg/2sg-self-recall layers
///    still see it.
///
/// **Disambiguation from definitional `Адам — сүтқоректі.`:** the
/// stripper fires only when the FULL input also carries an
/// addressee signal — a 2nd-person pronoun (`сен / сіз / сенің /
/// сізді / …`) or `?` / `!` punctuation. Definitional sentences
/// have neither and are passed through unchanged, preserving
/// `адам` as a legitimate common-noun topic.
///
/// Pure surface-level — no FST. Run AFTER `strip_preamble` so a
/// preamble + vocative combination collapses cleanly.
pub fn strip_addressee(input: &str) -> &str {
    if !has_addressee_signal(input) {
        return input;
    }
    let trimmed = input.trim_start();
    let lower = trimmed.to_lowercase();
    let mut best: Option<usize> = None;
    for &name in ADDRESSEES {
        if !lower.starts_with(name) {
            continue;
        }
        let after = &trimmed[name.len()..];
        let after_lower = after.to_lowercase();
        let cut: Option<usize> =
            if let Some(sep_pos) = after.find(|c: char| ADDRESSEE_SEPARATORS.contains(&c)) {
                // Cut after the punctuation separator so residual starts clean.
                let sep_char = after[sep_pos..].chars().next().unwrap();
                Some(name.len() + sep_pos + sep_char.len_utf8())
            } else if after_lower.starts_with(" сен") || after_lower.starts_with(" сіз") {
                // Bare-pronoun continuation: strip only the vocative.
                Some(name.len())
            } else {
                None
            };
        if let Some(c) = cut {
            if best.map_or(true, |b| c > b) {
                best = Some(c);
            }
        }
    }
    if let Some(cut) = best {
        trimmed[cut..].trim_start()
    } else {
        input
    }
}

/// **v4.11.5** — does the input carry any signal of being addressed
/// to adam (2nd-person reference or interrogative/exclamation
/// punctuation)? Used by `strip_addressee` to disambiguate the
/// vocative form `Адам, …` from the definitional `Адам — …`.
fn has_addressee_signal(input: &str) -> bool {
    if input.contains('?') || input.contains('!') {
        return true;
    }
    let lower = input.to_lowercase();
    // 2nd-person free pronouns + bound suffixes that are reliable
    // 2nd-person markers in surface text. Bare `сен` could be a
    // postposition in some contexts; the leading space prevents
    // matching word-internal occurrences (e.g. `мүсенжай`).
    [
        " сен",
        " сіз",
        "сенің",
        "сені ",
        "сіздің",
        "сізді",
        "сендей",
        "сіздей",
        "сізге",
        "сізден",
        " өзің",
        "өзіңіз",
        "өзіңді",
        "өзіңнің",
    ]
    .iter()
    .any(|m| lower.contains(m))
}

/// **v4.6.20** — detect a long, gracious user-acknowledgement.
/// Real-REPL: «Мен сенің әлі бәрін білмейтініңді және әлі де көп
/// жаттығу керек екенін түсіндім» — adam grabbed `әлі` and quoted
/// poetry. The input is a multi-clause statement *about* adam,
/// usually empathetic, ending in a 1sg perfective verb of
/// understanding/realisation.
///
/// Two-signal detector:
/// - Contains addressee marker `сенің / сені / сізді / сіздің`.
/// - Contains 1sg perfective realisation verb: `түсіндім / білдім
///   / көрдім / байқадым / ұқтым / аңғардым / сезіндім`.
/// Plus the sentence is not a question (no `?`, no question
/// pronoun) — questions like «Сені кім жасады?» also contain
/// `сені` but are not acknowledgements.
pub fn input_is_user_acknowledgement(input: &str) -> bool {
    let lower = input.to_lowercase();
    let has_addressee = lower.contains("сенің")
        || lower.contains("сені")
        || lower.contains("сіздің")
        || lower.contains("сізді");
    let has_realisation_verb = lower.contains("түсіндім")
        || lower.contains("білдім")
        || lower.contains("көрдім")
        || lower.contains("байқадым")
        || lower.contains("ұқтым")
        || lower.contains("аңғардым")
        || lower.contains("сезіндім");
    let is_question = lower.contains('?')
        || lower.contains("қалай")
        || lower.contains("неге")
        || lower.contains("кім")
        || lower.contains("қашан")
        || lower.contains("қайда");
    has_addressee && has_realisation_verb && !is_question
}

/// **v4.6.20** — detect "how are you better than other AI
/// models?" style questions. Routes to
/// `SystemAspect::SelfComparison`. Two-signal detector:
/// - Contains a "comparison" marker: `артық / артықсың / артықсыз
///   / жақсырақ / жақсырақсың / жақсырақсыз / озасың / озасыз /
///   айырмашылық`.
/// - Contains an addressee anchor (any of `сен / сіз / сені /
///   сізді / -сың/-сыз verb suffix already in marker).
/// The list is closed; mainstream "ерекшелік" stays under
/// Architecture (which v4.3.4 already routes correctly).
pub fn input_is_self_comparison_question(input: &str) -> bool {
    let lower = input.to_lowercase();
    let has_comparison = lower.contains("артық")
        || lower.contains("жақсырақ")
        || lower.contains("озасың")
        || lower.contains("озасыз")
        || lower.contains("қалай үстем")
        || lower.contains("несімен бөлек")
        || lower.contains("неге сенемін")
        || lower.contains("несімен ерекше")
        || lower.contains("неге таңдау керек");
    if !has_comparison {
        return false;
    }
    // Must reference adam (the addressee) — a comparison between
    // two third parties shouldn't trigger. Either a free-standing
    // 2nd-person pronoun (`сен / сіз / сені / сізді`) or a 2nd-
    // person verb ending (`-сың / -сыз` on a copula or modal,
    // including the `аласың / аласыз` ability form). The `сың/сыз`
    // suffix is itself the addressee marker even without a free-
    // standing pronoun.
    let has_pronoun = lower.contains(" сен")
        || lower.contains(" сіз")
        || lower.starts_with("сен")
        || lower.starts_with("сіз")
        || lower.contains("сені")
        || lower.contains("сізді");
    let has_addressee_suffix = lower.contains("аласың")
        || lower.contains("аласыз")
        || lower.contains("артықсың")
        || lower.contains("артықсыз")
        || lower.contains("жақсырақсың")
        || lower.contains("жақсырақсыз")
        || lower.contains("озасың")
        || lower.contains("озасыз");
    has_pronoun || has_addressee_suffix
}

/// **v4.6.20** — Adjective+noun compound noun-hint extraction.
/// Real-REPL: «Машиналық оқыту туралы айтып беріңізші» — pre-v4.6.20
/// the noun-hint extractor returned `оқыту` (the second word) and
/// dropped the modifier `машиналық`, then retrieved a generic quote
/// about education. v4.6.20 detects the `<adj> <noun>` shape via a
/// closed, audited list of compound topics seen in real REPL traces
/// and returns the joined compound as the hint.
///
/// The list is intentionally narrow — broader compound recognition
/// belongs in `MULTIWORD_ENTITIES` (semantics.rs) which is already
/// the canonical home for multi-token recognised topics. This
/// function exists for the specific case where the adj+noun form is
/// a topical compound (a *kind* of thing) rather than a named
/// entity.
const ADJ_NOUN_COMPOUND_HINTS: &[&str] = &[
    "машиналық оқыту",
    "терең оқыту",
    "жасанды интеллект",
    "табиғи тіл",
    "компьютерлік ғылым",
    "ақпараттық технология",
    "сандық технология",
    "сандық экономика",
    "жасанды нейрон",
    "жасанды нейрондық желі",
    "нейрондық желі",
];

/// **v4.6.20** — return the longest matching adj+noun compound
/// hint contained in the lowercased input, if any. Used by the
/// noun-hint extractor in `semantics.rs` to override the
/// FST-derived first-noun pick.
pub fn find_adj_noun_compound(input: &str) -> Option<&'static str> {
    let lower = input.to_lowercase();
    let mut best: Option<&'static str> = None;
    for &c in ADJ_NOUN_COMPOUND_HINTS {
        if lower.contains(c) && best.map_or(true, |b| c.len() > b.len()) {
            best = Some(c);
        }
    }
    best
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

    #[test]
    fn strips_leading_addressee_with_comma() {
        // Real-REPL 2026-04-30: vocative `Адам,` masked the actual
        // topic «физика бағдарламасы» behind retrieval of
        // `адам IsA сүтқоректі`.
        assert_eq!(
            strip_addressee("Адам, сен мектептің физика бағдарламасын білесің бе?"),
            "сен мектептің физика бағдарламасын білесің бе?"
        );
        assert_eq!(strip_addressee("Адам! Қалайсың?"), "Қалайсың?");
        assert_eq!(strip_addressee("Адам — қаласың?"), "қаласың?");
    }

    #[test]
    fn strips_addressee_variants_longest_first() {
        assert_eq!(strip_addressee("Адам-ау, не білесің?"), "не білесің?");
        assert_eq!(
            strip_addressee("Адамым, өзің туралы айтшы"),
            "өзің туралы айтшы"
        );
    }

    #[test]
    fn strips_addressee_before_bare_pronoun() {
        // No punctuation between vocative and `сен/сіз` — strip just
        // the vocative; the pronoun stays so 1sg-self-recall still
        // sees it.
        assert_eq!(
            strip_addressee("Адам сен мектеп пәндерін білесің бе?"),
            "сен мектеп пәндерін білесің бе?"
        );
        assert_eq!(
            strip_addressee("Адам сіз қандай тілде жазылғансыз?"),
            "сіз қандай тілде жазылғансыз?"
        );
    }

    #[test]
    fn preserves_input_when_addressee_not_leading() {
        // Bare common-noun "адам" at non-initial position is a
        // legitimate topic and must NOT be stripped.
        assert_eq!(strip_addressee("Адам — сүтқоректі."), "Адам — сүтқоректі.");
        assert_eq!(
            strip_addressee("Қазақстандағы адам туралы не білесіз?"),
            "Қазақстандағы адам туралы не білесіз?"
        );
        assert_eq!(strip_addressee("Сәлем"), "Сәлем");
    }
}
