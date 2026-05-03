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
    // **v4.17.5** — plural anaphors. Live REPL 2026-05-01 turn 14:
    // «Оларды тізімдей аласыз ба?» (after a turn mentioning 17
    // regions) — `оларды` is the 3rd-plural accusative anaphor
    // ("them"). v4.13.0 added the singular forms but missed the
    // plural paradigm. Adding both Acc/Dat/Gen plural forms here
    // for completeness.
    "оларды",
    "соларды",
    "мұларды",
    "бұларды",
    "оларға",
    "соларға",
    "мұларға",
    "бұларға",
    "олардың",
    "солардың",
    "мұлардың",
    "бұлардың",
];

/// Returns `true` if any whitespace-separated lowercase token of
/// the input matches a known discourse anaphor. The check is
/// intentionally surface-level — we don't want to lean on the
/// FST here because the FST's analysis of these forms is exactly
/// what `NOT_A_TOPIC` suppresses.
pub fn input_contains_discourse_anaphor(input: &str) -> bool {
    let lower = input.to_lowercase();
    if lower
        .split(|c: char| !c.is_alphabetic())
        .any(|word| DISCOURSE_ANAPHORS.contains(&word))
    {
        return true;
    }
    // **v4.30.0** — adnominal-demonstrative coreference. Live REPL
    // 2026-05-02 turn 11: «Бұл тілдегі кілт сөздер қандай?» — the
    // intended referent is the language being discussed in prior
    // turns (Rust). The bare-pronoun anaphor list (v4.13.0) doesn't
    // catch this because «бұл» here is a determiner modifying a
    // generic head noun («тіл / тілдегі») — not a standalone Acc/
    // Loc/Dat pronoun. The pattern is a strong coreference signal:
    // demonstrative determiner («бұл / осы / сол») + generic head
    // («тіл / нәрсе / зат / тақырып / сала / ұғым / бағыт / жүйе»)
    // means "the X we just discussed". Routes to the same
    // `dialog_context.resolve_anaphor()` substitution path as the
    // bare-pronoun anaphors.
    input_contains_adnominal_demonstrative(input)
}

/// **v4.30.0** — Detects the adnominal-demonstrative coreference
/// pattern: a demonstrative determiner («бұл / осы / сол / о /
/// мұ») followed (with up to 1 token gap for an adjective like
/// «жаңа / ескі») by a generic head noun in any inflection.
///
/// Generic heads cover the lemmas that — in adnominal-anaphora
/// position — almost always mean "the topic we are discussing":
/// `тіл / нәрсе / зат / тақырып / сала / ұғым / бағыт / жүйе`.
///
/// Returns `true` when the input contains such a phrase, telling
/// the caller to substitute `dialog_context.resolve_anaphor()`
/// for the topic. Conservative on false positives: the head must
/// match a fixed list of generic referent nouns; mentions like
/// «бұл кітап» (this book — likely a NEW topic, not anaphoric)
/// don't trigger.
pub fn input_contains_adnominal_demonstrative(input: &str) -> bool {
    const DETERMINERS: &[&str] = &["бұл", "осы", "сол"];
    // Generic-head prefixes — anything that starts with one of
    // these in lowercase is treated as the head. Covers all case
    // inflections (Loc, Acc, Dat, Gen, Abl, P3): тіл / тілі /
    // тілдегі / тілді / тілге / тілдің / тілден / тілдер ...
    const HEAD_PREFIXES: &[&str] = &[
        "тіл",
        "нәрсе",
        "зат",
        "тақырып",
        "сала",
        "ұғым",
        "бағыт",
        "жүйе",
    ];
    // Optional intervening adjective stems (allow 1 token between
    // determiner and head). Empty by default — keep as bare match
    // for now; widening to allow «бұл жаңа тілде» can come later
    // with evidence from real REPL.
    let lower = input.to_lowercase();
    let tokens: Vec<&str> = lower
        .split(|c: char| !c.is_alphabetic())
        .filter(|t| !t.is_empty())
        .collect();
    for window in tokens.windows(2) {
        if !DETERMINERS.contains(&window[0]) {
            continue;
        }
        let head = window[1];
        if HEAD_PREFIXES.iter().any(|p| head.starts_with(p)) {
            return true;
        }
    }
    false
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
    // **v4.41.0** — match by stem-prefix («көбейт*», «бөл*»,
    // «қос*») so naked imperative («бөл», «қос», «көбейт») and
    // converb forms («көбейтсек», «бөлсе») fire too. Pre-v4.41.0
    // the explicit form list missed «Жүзді онға бөл» (bare
    // imperative) — fell through to clarify path. The
    // stem-prefix is short but preceded by the `has_numeral_word`
    // gate so an incidental noun starting with «көб»/«бөл»/«қос»
    // can't trigger math-mode by itself.
    // **v4.42.0** — added `азайт*` (decrease / subtract) as a
    // fifth math-verb stem; pairs with the new sequel-clause
    // multi-step evaluator.
    const MATH_VERB_STEMS: &[&str] = &["көбейт", "бөл", "қос", "есепте", "азайт"];
    // `ал` (subtract / take) is too short to use as a prefix —
    // checked below as a closed set of inflected forms.
    // **v4.41.0** — closed set of `ал` (subtract / take) inflected
    // forms recognised as math-verb. Bare imperative «ал» is
    // intentionally OMITTED — it doubles as the Kazakh sentence-
    // initial conjunction "and / but" («Ал онда қанша аймақ
    // бар?» — pre-cognitive_eval bare «ал» in SUB_FORMS triggered
    // math mode here on the «он» numeral prefix in «онда»,
    // breaking the v4.6.0 anaphora test). For subtraction prefer
    // the explicit imperative «алыңыз» / verbal noun «алу» /
    // converb «алып» / conditional «алсам / алсаң / алсаңыз».
    const SUB_FORMS: &[&str] = &[
        "алу",
        "алса",
        "алсам",
        "алсаң",
        "алсаңыз",
        "алыңыз",
        "алғанда",
        "алып",
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
    let non_empty_words: Vec<&&str> = words.iter().filter(|w| !w.is_empty()).collect();
    let has_math_verb = words.iter().any(|w| {
        if w.is_empty() {
            return false;
        }
        MATH_VERB_STEMS.iter().any(|stem| w.starts_with(stem)) || SUB_FORMS.contains(w)
    });
    // **v4.41.0** — bare imperative «ал» is the standalone form of
    // «to subtract» but doubles as the sentence-initial Kazakh
    // conjunction "and / but". Position disambiguates: math-«ал»
    // is sentence-final («Жүзден елуді ал»); conjunction-«ал» is
    // sentence-initial («Ал онда қанша аймақ бар?»). Accept ONLY
    // the sentence-final position.
    let has_bare_al_imperative = non_empty_words.last().map(|w| **w == "ал").unwrap_or(false);
    if !has_math_verb && !has_bare_al_imperative {
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

/// **v4.41.0** — Kazakh word-form math evaluator. Returns
/// `Some(value)` for parseable Kazakh-language arithmetic input
/// (e.g. «бесті отызға көбейту» = 5×30 = 150; «жүз пен елуді
/// қосу» = 100+50 = 150; «жүзді онға бөл» = 100/10 = 10).
/// Returns `None` when the input doesn't have the
/// `<num-word> <num-word> <math-verb>` shape, when number parsing
/// fails, or when computation overflows / divides non-evenly.
///
/// Pipeline:
/// 1. Strip case suffixes from each token (бесті → бес, отызға → отыз).
/// 2. Detect math operation by stem-prefix match against verb roots
///    (қос / көбейт / бөл / ал) — handles inflected forms like
///    «көбейтіңіз», «көбейткенде», «көбейтсек», «қоссаңыз», etc.
/// 3. Group consecutive number-word tokens into operands using
///    additive composition («жиырма бес» → 20+5 = 25) and
///    multiplicative composition with `жүз / мың / миллион` («екі
///    мың» → 2×1000 = 2000; «бір жүз елу» → 100+50 = 150).
/// 4. Apply the operation; return integer result. Non-integer
///    division falls back to `None` (planner picks math_refusal).
///
/// Why a separate evaluator from `try_evaluate_arithmetic`:
/// the digit-form version normalises whitespace and walks ASCII
/// arithmetic; the word-form version needs lexical recognition of
/// number words AND verb stems with Kazakh case morphology. Sharing
/// internals would mix two unrelated parse strategies.
pub fn try_evaluate_kazakh_word_math(input: &str) -> Option<i64> {
    // **v4.42.0** — multi-clause support. Split input by commas /
    // sequencing connectives («және» — "and", «содан кейін» — "then",
    // «соңында» — "at the end") so chained operations like
    // «Беске жетіні қоссақ, екіге көбейтеміз, үшке бөлеміз және
    // бесті азайтамыз» — ((5+7)*2)/3-5 = 3 — evaluate left-to-right
    // with the running accumulator carried between clauses.
    //
    // Pipeline:
    //   - First clause: must provide BOTH operands (2 numbers with
    //     explicit case morphology) and one math verb. Result is
    //     the seed accumulator.
    //   - Each subsequent clause: one operand + one verb; the
    //     accumulator becomes the left operand, the new operand
    //     the right.
    //   - Any clause failing to satisfy this shape → return None
    //     and let the planner pick math_refusal.
    let lower = input.to_lowercase();
    let normalized: String = lower
        .replace(',', " __CLAUSE_SEP__ ")
        .replace(" және ", " __CLAUSE_SEP__ ")
        .replace(" содан кейін ", " __CLAUSE_SEP__ ")
        .replace(" соңында ", " __CLAUSE_SEP__ ");
    let clauses: Vec<&str> = normalized
        .split("__CLAUSE_SEP__")
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .collect();
    if clauses.is_empty() {
        return None;
    }
    let mut iter = clauses.iter();
    // First clause — bootstrap with two operands.
    let first = iter.next()?;
    let mut accumulator = single_clause_kazakh_math(first)?;
    // Subsequent clauses — apply one op against running accumulator.
    // Trailing non-math clauses («нәтижесі қандай болады») are
    // skipped (return None from sequel parser → treat as appendage,
    // not failure).
    for clause in iter {
        if let Some(next) = sequel_clause_kazakh_math(clause, accumulator) {
            accumulator = next;
        } else if clause_has_math_verb(clause) {
            // Math verb present but couldn't parse → real failure.
            return None;
        }
        // No math verb → trailing rhetorical appendage; ignore.
    }
    Some(accumulator)
}

fn clause_has_math_verb(clause: &str) -> bool {
    let lowered = clause.to_lowercase();
    lowered.split(|c: char| !c.is_alphabetic()).any(|w| {
        !w.is_empty()
            && (w.starts_with("көбейт")
                || w.starts_with("бөл")
                || w.starts_with("қос")
                || w.starts_with("есепте")
                || w.starts_with("азайт")
                || matches!(
                    w,
                    "алу"
                        | "алса"
                        | "алсам"
                        | "алсаң"
                        | "алсаңыз"
                        | "алыңыз"
                        | "алғанда"
                        | "алып"
                ))
    })
}

fn split_kazakh_math_clause(clause: &str) -> Vec<&str> {
    // **v4.41.7** — split on whitespace + non-alphanumeric chars
    // EXCEPT '-'. Pre-v4.41.7 the predicate was `!c.is_alphabetic()`
    // which dropped digits entirely (chars '3' and '0' aren't
    // alphabetic, so the splitter cut between them, leaving "30"
    // as a sequence of empty strings). Real-REPL transcript
    // 2026-05-03 typed «30-ды азайтыңыз» (digit form with Kazakh
    // case suffix); pre-v4.41.7 the «30» was lost entirely and the
    // chunk had no operand. Keeping `'-'` as part of tokens lets
    // «30-ды» survive as one token, parsed by
    // `parse_kazakh_number_token`'s digit-prefix branch.
    clause
        .split(|c: char| !c.is_alphanumeric() && c != '-')
        .filter(|t| !t.is_empty())
        .collect()
}

fn single_clause_kazakh_math(clause: &str) -> Option<i64> {
    let raw_tokens = split_kazakh_math_clause(clause);
    if raw_tokens.len() < 3 {
        return None;
    }
    let op = detect_kazakh_math_op(&raw_tokens)?;
    let operands = extract_kazakh_number_operands(&raw_tokens);
    if operands.len() != 2 {
        return None;
    }
    apply_kazakh_math_op(op, operands[0], operands[1])
}

fn sequel_clause_kazakh_math(clause: &str, accumulator: i64) -> Option<i64> {
    let raw_tokens = split_kazakh_math_clause(clause);
    let op = detect_kazakh_math_op(&raw_tokens)?;
    let operands = extract_kazakh_number_operands(&raw_tokens);
    // Sequel clauses provide exactly ONE additional operand —
    // the accumulator from previous clauses serves as the left
    // operand. «Екіге көбейтеміз» = «running × 2».
    if operands.len() != 1 {
        return None;
    }
    apply_kazakh_math_op(op, accumulator, operands[0])
}

fn apply_kazakh_math_op(op: KazakhMathOp, a: i64, b: i64) -> Option<i64> {
    match op {
        KazakhMathOp::Add => a.checked_add(b),
        KazakhMathOp::Sub => a.checked_sub(b),
        KazakhMathOp::Mul => a.checked_mul(b),
        KazakhMathOp::Div => {
            if b == 0 || a % b != 0 {
                None
            } else {
                Some(a / b)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KazakhMathOp {
    Add,
    Sub,
    Mul,
    Div,
}

fn detect_kazakh_math_op(tokens: &[&str]) -> Option<KazakhMathOp> {
    // Stem-prefix match against verb roots. Order matters: longer
    // / more-specific stems first so «көбейту» wins over a
    // hypothetical «көб» (shorter prefix). All four ops covered;
    // `ал` (subtract / take) is intentionally checked LAST because
    // it's the shortest stem and would otherwise eat plenty of
    // unrelated tokens.
    for t in tokens {
        if t.starts_with("көбейт") {
            return Some(KazakhMathOp::Mul);
        }
        if t.starts_with("бөл") {
            return Some(KazakhMathOp::Div);
        }
        if t.starts_with("қос") {
            return Some(KazakhMathOp::Add);
        }
        // **v4.42.0** — `азайт*` (decrease / subtract) — alternate
        // verb stem for subtraction. «Бесті азайт» = «subtract 5».
        if t.starts_with("азайт") {
            return Some(KazakhMathOp::Sub);
        }
    }
    // `ал` separately because of its short length — accept only when
    // the stem is exactly one of the known math-verb forms (not a
    // prefix of unrelated nouns like `алма` "apple" / `алмас` "won't
    // take" / `алыс` "far"). Conservative whitelist.
    // **v4.41.0** — closed set of `ал` (subtract / take) inflected
    // forms recognised as math-verb. Bare imperative «ал» is
    // intentionally OMITTED — it doubles as the Kazakh sentence-
    // initial conjunction "and / but" («Ал онда қанша аймақ
    // бар?» — pre-cognitive_eval bare «ал» in SUB_FORMS triggered
    // math mode here on the «он» numeral prefix in «онда»,
    // breaking the v4.6.0 anaphora test). For subtraction prefer
    // the explicit imperative «алыңыз» / verbal noun «алу» /
    // converb «алып» / conditional «алсам / алсаң / алсаңыз».
    const SUB_FORMS: &[&str] = &[
        "алу",
        "алса",
        "алсам",
        "алсаң",
        "алсаңыз",
        "алыңыз",
        "алғанда",
        "алып",
    ];
    for t in tokens {
        if SUB_FORMS.contains(t) {
            return Some(KazakhMathOp::Sub);
        }
    }
    // **v4.41.0** — bare imperative «ал» accepted ONLY when
    // sentence-final (canonical imperative position; sentence-
    // initial «ал» is the conjunction "and / but" — closes the
    // false-positive on «Ал онда қанша аймақ бар?»).
    if tokens.last().is_some_and(|t| *t == "ал") {
        return Some(KazakhMathOp::Sub);
    }
    None
}

fn extract_kazakh_number_operands(tokens: &[&str]) -> Vec<i64> {
    // **v4.41.0** — case-morphology-aware operand extraction.
    //
    // Algorithm: a "number chunk" accumulates consecutive number-
    // word tokens via standard number-composition (additive for
    // descending magnitude — «жиырма бес» = 20+5 = 25;
    // multiplicative for `жүз`/`мың`/`миллион` — «бір мың
    // тоғыз жүз» = 1*1000 + 9*100 = 1900). A chunk CLOSES when:
    //  - the token has an explicit case suffix («бесті» Acc /
    //    «отызға» Dat / «жүзден» Abl) — the case marker is the
    //    user's signal that this number is a complete operand
    //    serving a syntactic role;
    //  - OR a non-number token (verb / separator) appears.
    //
    // This handles the canonical Kazakh math phrasings:
    //   «Бесті отызға көбейту»          → [5, 30] × → 150
    //   «Жиырма бесті отызға көбейту»   → [25, 30] × → 750
    //   «Жүз елуді бір мыңға қос»       → [150, 1000] + → 1150
    //
    // Without case morphology, two consecutive number words
    // compose normally (so «жиырма бес есепте» means «count up to
    // 25» — single operand 25). The case suffix is the
    // disambiguator between «25» and «20-and-5-as-separate-args».
    // **v4.41.7** — pre-scan: count case-marked numbers in this
    // clause. Determines whether `пен/мен/бен` between numbers
    // should MERGE additively into a single chunk (count ≥ 2) or
    // SPLIT into separate operands (count == 1).
    //
    // Reasoning:
    // - «Қырық пен бесті қосып» (count = 1, only `бесті`) →
    //   user means "add 40 AND 5" → operands [40, 5], op=Add → 45.
    //   The single case-marked operand signals a binary operation
    //   on the two pen-conjoined numbers.
    // - «Қырық пен бесті екіге көбейтіп» (count = 2, `бесті` and
    //   `екіге`) → user means "(40+5) × 2" → operands [45, 2],
    //   op=Mul → 90. The two case-marked positions ARE the
    //   binary operands; пен-conjoined numbers are a compound
    //   first operand.
    let case_marked_count = tokens
        .iter()
        .filter(|t| parse_kazakh_number_token(t).map_or(false, |(_, has_case)| has_case))
        .count();
    let pen_merges = case_marked_count >= 2;

    let mut operands = Vec::new();
    let mut chunk_total: i64 = 0;
    let mut chunk_inflight = false;
    for t in tokens {
        if let Some((value, has_case)) = parse_kazakh_number_token(t) {
            // Compose into the current chunk.
            chunk_total = compose_number_in_chunk(chunk_total, value);
            chunk_inflight = true;
            if has_case {
                operands.push(chunk_total);
                chunk_total = 0;
                chunk_inflight = false;
            }
        } else if pen_merges && matches!(*t, "пен" | "мен" | "бен") && chunk_inflight {
            // Pen-conjoined merge — chunk stays open, next number
            // adds to chunk_total. Active only when 2+ case-marked
            // operands are present in the clause.
        } else if chunk_inflight {
            operands.push(chunk_total);
            chunk_total = 0;
            chunk_inflight = false;
        }
    }
    if chunk_inflight {
        operands.push(chunk_total);
    }
    operands
}

fn compose_number_in_chunk(current: i64, next: i64) -> i64 {
    // Standard cardinal-number composition for descending-magnitude
    // languages.
    //   - `жүз` (100) multiplies the < 1000 remainder of `current`
    //     (or 1 if remainder is 0); preserves any thousands
    //     accumulated already. «бес жүз» = 5*100 = 500;
    //     «бір мың тоғыз жүз» = 1000 + 9*100 = 1900.
    //   - `мың` / `миллион` multiply the entire accumulator (or 1).
    //   - Other digits add.
    if next == 100 {
        let thousands = current / 1000 * 1000;
        let units = current % 1000;
        let multiplier = if units == 0 { 1 } else { units };
        thousands + multiplier * 100
    } else if next == 1000 || next == 1_000_000 {
        let multiplier = if current == 0 { 1 } else { current };
        multiplier * next
    } else {
        current + next
    }
}

fn parse_kazakh_number_token(token: &str) -> Option<(i64, bool)> {
    // Returns (value, has_explicit_case_suffix).
    // First try the bare token (no case).
    if let Some(v) = bare_kazakh_number(token) {
        return Some((v, false));
    }
    // **v4.41.7** — digit form: «30», «100», «5» bare digits AND
    // digit + Kazakh case suffix («30-ды», «100-ге», «5-ке»).
    // Real-REPL transcript «Бес санын үшке көбейтіп, 30-ды
    // азайтыңыз» typed digit «30» with Acc «-ды»; pre-v4.41.7
    // the splitter dropped the digits entirely. Now both bare
    // digits and digit + dash + suffix forms are recognised.
    if token.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        let digits: String = token.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(v) = digits.parse::<i64>() {
            let rest = &token[digits.len()..];
            // Strip optional leading '-' (Kazakh writes «30-ды»).
            let rest = rest.strip_prefix('-').unwrap_or(rest);
            if rest.is_empty() {
                return Some((v, false));
            }
            // Trailing chars present → treat as case-marked
            // operand. Conservative: any non-empty suffix on a
            // digit-prefix is read as case morphology even if
            // the suffix isn't a known Kazakh case form (covers
            // dialectal / colloquial inflections without an
            // exhaustive whitelist).
            return Some((v, true));
        }
    }
    // Try each case-suffix, longest first, and check that the
    // remaining stem IS a recognised bare number.
    for suff in CASE_SUFFIXES {
        if let Some(stem_len) = token.len().checked_sub(suff.len()) {
            if token.ends_with(suff) {
                let stem = &token[..stem_len];
                if let Some(v) = bare_kazakh_number(stem) {
                    return Some((v, true));
                }
            }
        }
    }
    None
}

const CASE_SUFFIXES: &[&str] = &[
    // Ordered longest-first so longest-match wins.
    "тардың",
    "тердің",
    "лардың",
    "лердің",
    "дардың",
    "дердің",
    "ынан",
    "інен",
    "ының",
    "інің",
    "ында",
    "інде",
    "тың",
    "тің",
    "дың",
    "дің",
    "ның",
    "нің",
    "пен",
    "бен",
    "ден",
    "тен",
    "нен",
    "дан",
    "тан",
    "нан",
    "ге",
    "ке",
    "ға",
    "қа",
    "не",
    "на",
    "ты",
    "ті",
    "ды",
    "ді",
    "ны",
    "ні",
    "де",
    "те",
    "да",
    "та",
];

/// **v4.41.0** — Kazakh number-to-words renderer. The inverse of
/// [`bare_kazakh_number`]: given an integer, produces its
/// canonical Kazakh number-word phrase. Used by the math-answer
/// pipeline to optionally emit «жүз елу» alongside `150` so the
/// user gets the answer in the same modality they asked
/// («Бесті отызға көбейтсем» → «Нәтижесі: 150 (жүз елу)»).
///
/// Supported range: 0 to 999_999_999 (up to «тоғыз жүз тоқсан
/// тоғыз миллион тоғыз жүз тоқсан тоғыз мың тоғыз жүз тоқсан
/// тоғыз»). Returns `None` for negative or larger numbers
/// — those continue to render as bare digits via the
/// `{math_value}` slot. Negative not implemented because Kazakh
/// math vocabulary («теріс» negation marker) is rare in
/// arithmetic-result contexts; user can read the digit directly.
pub fn render_kazakh_number_words(value: i64) -> Option<String> {
    if !(0..=999_999_999).contains(&value) {
        return None;
    }
    if value == 0 {
        return Some("нөл".to_string());
    }
    let mut parts: Vec<String> = Vec::new();
    let mut n = value;
    let millions = n / 1_000_000;
    n %= 1_000_000;
    if millions > 0 {
        if millions > 1 {
            parts.push(render_under_thousand(millions));
        } else {
            parts.push("бір".to_string());
        }
        parts.push("миллион".to_string());
    }
    let thousands = n / 1000;
    n %= 1000;
    if thousands > 0 {
        if thousands > 1 {
            parts.push(render_under_thousand(thousands));
        } else {
            parts.push("бір".to_string());
        }
        parts.push("мың".to_string());
    }
    if n > 0 {
        parts.push(render_under_thousand(n));
    }
    Some(parts.join(" "))
}

fn render_under_thousand(n: i64) -> String {
    let mut parts: Vec<String> = Vec::new();
    let hundreds = n / 100;
    if hundreds > 0 {
        if hundreds > 1 {
            parts.push(digit_word(hundreds).to_string());
        }
        parts.push("жүз".to_string());
    }
    let tens_value = (n % 100) / 10 * 10;
    if tens_value > 0 {
        parts.push(tens_word(tens_value).to_string());
    }
    let units = n % 10;
    if units > 0 {
        parts.push(digit_word(units).to_string());
    }
    parts.join(" ")
}

fn digit_word(d: i64) -> &'static str {
    match d {
        1 => "бір",
        2 => "екі",
        3 => "үш",
        4 => "төрт",
        5 => "бес",
        6 => "алты",
        7 => "жеті",
        8 => "сегіз",
        9 => "тоғыз",
        _ => "",
    }
}

fn tens_word(t: i64) -> &'static str {
    match t {
        10 => "он",
        20 => "жиырма",
        30 => "отыз",
        40 => "қырық",
        50 => "елу",
        60 => "алпыс",
        70 => "жетпіс",
        80 => "сексен",
        90 => "тоқсан",
        _ => "",
    }
}

fn bare_kazakh_number(stem: &str) -> Option<i64> {
    match stem {
        "нөл" => Some(0),
        "бір" => Some(1),
        "екі" => Some(2),
        "үш" => Some(3),
        "төрт" => Some(4),
        "бес" => Some(5),
        "алты" => Some(6),
        "жеті" => Some(7),
        "сегіз" => Some(8),
        "тоғыз" => Some(9),
        "он" => Some(10),
        "жиырма" => Some(20),
        "отыз" => Some(30),
        "қырық" => Some(40),
        "елу" => Some(50),
        "алпыс" => Some(60),
        "жетпіс" => Some(70),
        "сексен" => Some(80),
        "тоқсан" => Some(90),
        "жүз" => Some(100),
        "мың" => Some(1000),
        "миллион" => Some(1_000_000),
        "миллиард" => Some(1_000_000_000),
        _ => None,
    }
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
    // **v4.17.5** — disambiguation: «жақсырақ болу + ...» is a
    // willingness/improvement question, not a comparison. Live
    // REPL 2026-05-01 turn 20: «...жақсырақ және ақылды болуды
    // үйренуге дайынсыз ба?» — pre-v4.17.5 SelfComparison fired
    // because of `жақсырақ`. Defer to AskWillingness when growth-
    // verbs are co-present. The Intent dispatcher checks
    // `AskWillingness` BEFORE this comparison detector so this
    // guard is the belt-and-braces fallback for cases where the
    // growth-verb pattern doesn't match exactly but the
    // comparison-as-improvement reading is clearly wrong.
    if lower.contains("жақсырақ болу") || lower.contains("ақылды болу") {
        return false;
    }
    let has_comparison = lower.contains("артық")
        || lower.contains("жақсырақ")
        || lower.contains("озасың")
        || lower.contains("озасыз")
        || lower.contains("қалай үстем")
        || lower.contains("несімен бөлек")
        || lower.contains("неге сенемін")
        || lower.contains("несімен ерекше")
        || lower.contains("неге таңдау керек")
        // **v4.17.5** — distinguishing-question phrasings surfaced
        // by the 2026-05-01 live REPL transcript: «Сізді
        // қолданыстағы жасанды интеллект модельдерінен
        // ерекшелендіретін нәрсе.» pre-v4.17.5 fell through to
        // greedy retrieval and surfaced a poetry quote.
        || lower.contains("ерекшелендір")
        || lower.contains("ерекшелейт")
        || lower.contains("айырмашылығың")
        || lower.contains("айырмашылығыңыз")
        || lower.contains("айрықша қылатын")
        || lower.contains("айырық қылатын")
        || lower.contains("айырмашылықтарың")
        || lower.contains("айырмашылықтарыңыз");
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

    /// **v4.41.0** — word-form math evaluator tests.
    use super::try_evaluate_kazakh_word_math;

    #[test]
    fn word_math_basic_ops() {
        // 5 × 30 = 150
        assert_eq!(
            try_evaluate_kazakh_word_math("Бесті отызға көбейтсем"),
            Some(150)
        );
        // 100 / 10 = 10
        assert_eq!(try_evaluate_kazakh_word_math("Жүзді онға бөл"), Some(10));
        // 6 / 2 = 3
        assert_eq!(
            try_evaluate_kazakh_word_math("Алтыны екіге бөліңіз"),
            Some(3)
        );
        // 100 - 50 = 50
        assert_eq!(try_evaluate_kazakh_word_math("Жүзден елуді ал"), Some(50));
        // 5 + 3 = 8
        assert_eq!(try_evaluate_kazakh_word_math("Бес пен үш қос"), Some(8));
    }

    #[test]
    fn word_math_compound_numbers() {
        // 25 × 5 = 125 (compound «жиырма бес»)
        assert_eq!(
            try_evaluate_kazakh_word_math("Жиырма бесті бес көбейтсек"),
            Some(125)
        );
        // 100 + 50 = 150 (compound «жүз елу»)
        assert_eq!(
            try_evaluate_kazakh_word_math("Жүз елуді бір мыңға қос"),
            Some(1150)
        );
        // 1999 + 230 = 2229 (compound «бір мың тоғыз жүз тоқсан тоғыз»)
        assert_eq!(
            try_evaluate_kazakh_word_math("Бір мың тоғыз жүз тоқсан тоғызға қос екі жүз отыз"),
            Some(2229)
        );
    }

    #[test]
    fn word_math_inflected_imperatives() {
        // Imperative-form variants
        assert_eq!(
            try_evaluate_kazakh_word_math("Бесті отызға көбейтіңіз"),
            Some(150)
        );
        assert_eq!(
            try_evaluate_kazakh_word_math("Жүзді бесті бөлсем"),
            Some(20)
        );
    }

    #[test]
    fn word_math_rejects_non_math() {
        // No math verb
        assert_eq!(try_evaluate_kazakh_word_math("Бес отыз қанша"), None);
        // Single operand only
        assert_eq!(try_evaluate_kazakh_word_math("Бесті көбейтсек"), None);
        // Take-imperative without numerals doesn't trigger
        assert_eq!(try_evaluate_kazakh_word_math("Кітапты ал"), None);
    }

    #[test]
    fn word_math_division_non_integer_returns_none() {
        // 7 / 2 has remainder → None (math_refusal fallback)
        assert_eq!(try_evaluate_kazakh_word_math("Жетіні екіге бөл"), None);
        // 5 / 0 → None
        assert_eq!(try_evaluate_kazakh_word_math("Бесті нөлге бөл"), None);
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
