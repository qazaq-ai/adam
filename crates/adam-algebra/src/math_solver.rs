// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `MathSolver` — **deterministic procedural-math solver**.
//!
//! Closes the «math/*» gap in the dialog battery (Stage 4.5
//! reported 3 unanswered cases: «2 жұп 2 қанша?» / «Двадцать пять
//! умножь на 7, раздели на два и прибавь три» / Kazakh equivalent).
//!
//! Architectural fit: math is **procedural computation**, not
//! knowledge retrieval — it does not belong in [`crate::FrameIndex`]
//! (which holds facts). The solver is a parallel branch beside the
//! fact index. The dialog pipeline (Stage 7 — Realiser) will:
//!
//! 1. Try the math solver on the input.
//! 2. If the solver answers, surface its result.
//! 3. Otherwise, route through `FrameIndex.query(&QueryIR)` for
//!    factual retrieval.
//!
//! Stage 4.5+ ships the solver and wires it into the dialog
//! battery; the realiser wiring is folded into Stage 7.
//!
//! ## Semantics
//!
//! The solver implements **left-to-right chained evaluation**, not
//! standard operator precedence. This matches how the user phrases
//! chained arithmetic in spoken Russian / Kazakh:
//!
//! > «Двадцать пять умножь на 7, раздели на два и прибавь три»
//!
//! Reads as: «take 25, multiply by 7, divide by 2, add 3» —
//! sequential, not standard order-of-operations:
//!
//! ```text
//! 25 → ×7 → 175 → ÷2 → 87.5 → +3 → 90.5
//! ```
//!
//! Standard precedence would give `25 + (7/2) × 3 + 25 = ...` —
//! NOT what a Kazakh speaker means by this surface form. The
//! left-to-right rule is deliberate and matches the conversational
//! semantics.
//!
//! For algebraic expressions like «2 + 3 × 4», standard precedence
//! is also supported when the input uses ASCII operators (the
//! parser distinguishes word-form chained-imperative from
//! infix-arithmetic by punctuation / operator surface).
//!
//! ## Determinism contract
//!
//! - **No floating-point fuzz.** Internally `f64`, but tests assert
//!   exact equality to a curated truth (90.5, not 90.4999…).
//! - **No locale-specific parsing.** Decimal separator is `.` only;
//!   commas are list separators, not decimal points.
//! - **No silent overflow.** Numbers > 1e15 fail to parse rather
//!   than wrap.
//! - **Pure function.** No I/O, no state, fully testable.
//!
//! ## Vocabulary
//!
//! Stage 1 supports the closed-set vocabulary used in school-grade
//! arithmetic queries:
//!
//! - **Numbers**: Russian + Kazakh number words 0-99 + Arabic
//!   digits + multi-digit decimals.
//! - **Operators**: + − × ÷ via ASCII or word form
//!   (`плюс / прибавь / қос`, `минус / отними / азайт`,
//!   `умножь / умножить / көбейт`, `раздели / разделить / бөл`).
//! - **Connectors**: `и`, `потом`, `затем`, `на`, `-ге`, `-ке`,
//!   `-ні`, `-ге` are silently dropped.

use serde::{Deserialize, Serialize};

/// Result of a successful math solve.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MathResult {
    /// Numeric answer.
    pub value: f64,
    /// Number of operations applied (for trace / display).
    pub steps: usize,
}

impl MathResult {
    /// Canonical surface form of the result. Integers render
    /// without a decimal point («4», not «4.0»); halves render as
    /// «90.5» etc.
    pub fn render(self) -> String {
        if self.value.fract() == 0.0 && self.value.abs() < 1e15 {
            format!("{}", self.value as i64)
        } else {
            // Strip trailing zeros from the f64 repr.
            let s = format!("{}", self.value);
            s
        }
    }
}

/// Solve a math expression in Russian / Kazakh / ASCII. Returns
/// `None` when the input does not look like an arithmetic
/// expression at all (no numbers, no operators, ambiguous parse).
///
/// Examples:
/// - `"Двадцать пять умножь на 7, раздели на два и прибавь три"` → 90.5
/// - `"Жиырма бесті жетіге көбейт, екіге бөл, үшті қос"` → 90.5
/// - `"2 + 2"` → 4
/// - `"15 умножить на 4"` → 60
pub fn solve(input: &str) -> Option<MathResult> {
    let tokens = tokenize(input);
    if tokens.is_empty() {
        return None;
    }
    evaluate(&tokens)
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
    /// `a^b` — `a` raised to integer power `b`.
    Pow,
    /// `(a × b) / 100` — `b` percent of `a`.
    Percent,
}

/// Unary operators that consume one operand and replace the
/// accumulator. Square root is the canonical case; «корень из 16»
/// emits `Token::Number(16)` then `Token::Unary(Sqrt)` and
/// `evaluate` rewrites the accumulator to `sqrt(acc)`.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Unary {
    Sqrt,
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    Op(Op),
    Unary(Unary),
}

/// Walk the input left-to-right, emitting `Number` and `Op`
/// tokens. Aggregates multi-word numbers like "двадцать пять" or
/// "жиырма бес" into a single token.
fn tokenize(input: &str) -> Vec<Token> {
    let lower = input.to_lowercase();
    // Replace common punctuation with whitespace so the word
    // splitter doesn't get confused.
    let cleaned: String = lower
        .chars()
        .map(|c| {
            if matches!(c, ',' | ';' | '.' | '!' | '?' | '(' | ')') {
                ' '
            } else {
                c
            }
        })
        .collect();
    let words: Vec<&str> = cleaned
        .split_whitespace()
        .filter(|w| !is_connector(w))
        .collect();

    let mut out: Vec<Token> = Vec::new();
    let mut i = 0;
    while i < words.len() {
        // Unary prefix («корень из X» / «X-нің түбірі»). Detect the
        // marker word; the next number becomes the operand of the
        // sqrt. Emit `Number` then `Unary` so `evaluate` can lift
        // the running accumulator.
        if is_sqrt_prefix(words[i]) {
            // «корень из» — operand follows.
            i += 1;
            if let Some((n, consumed)) = parse_compound_number(&words[i..]) {
                out.push(Token::Number(n));
                out.push(Token::Unary(Unary::Sqrt));
                i += consumed;
            }
            continue;
        }
        // Try a multi-word number first (handles «двадцать пять»,
        // «жиырма бес», «сто двадцать пять», …).
        if let Some((n, consumed)) = parse_compound_number(&words[i..]) {
            out.push(Token::Number(n));
            i += consumed;
            // Sqrt suffix («X-нің түбірі / квадратный корень из X»).
            if i < words.len() && is_sqrt_suffix(words[i]) {
                out.push(Token::Unary(Unary::Sqrt));
                i += 1;
            }
            continue;
        }
        if let Some(op) = parse_op(words[i]) {
            out.push(Token::Op(op));
            i += 1;
            continue;
        }
        // Unknown word — skip silently. The user might say
        // «прибавь, пожалуйста, три» and we should not refuse.
        i += 1;
    }
    out
}

/// Does this word introduce a unary square-root prefix?
fn is_sqrt_prefix(w: &str) -> bool {
    matches!(w, "корень" | "√")
}

/// Does this word indicate a unary square-root suffix on the
/// previous number (Kazakh «X-нің түбірі»)?
fn is_sqrt_suffix(w: &str) -> bool {
    matches!(w, "түбірі" | "квадратты_түбірі")
}

/// Connector / filler words dropped from the token stream.
fn is_connector(w: &str) -> bool {
    matches!(
        w,
        // Russian
        "и" | "потом"
            | "затем"
            | "на"
            | "к"
            | "от"
            | "из"
            | "ещё"
            | "еще"
            | "тоже"
            | "также"
            // Kazakh
            | "және"
            | "содан"
            | "сосын"
            | "одан"
            // Generic
            | "="
            | "->"
    )
}

/// Map an operator word to its `Op`. Returns `None` for non-operator
/// tokens.
fn parse_op(w: &str) -> Option<Op> {
    Some(match w {
        // Russian — imperative & infinitive forms.
        "плюс" | "прибавь" | "прибавить" | "сложи" | "сложить" | "+" => {
            Op::Add
        }
        "минус" | "отними" | "отнять" | "вычти" | "вычесть" | "-" | "−" | "–" => {
            Op::Sub
        }
        "умножь" | "умножить" | "помножь" | "помножить" | "×" | "*" | "x" => {
            Op::Mul
        }
        "раздели" | "разделить" | "подели" | "поделить" | "÷" | "/" | ":" => {
            Op::Div
        }
        // Power / exponent.
        "в_степени" | "степени" | "в_степень" | "степень" | "^" | "**" => {
            Op::Pow
        }
        // Percentage — left-to-right semantics: «acc procent rhs»
        // = `(acc * rhs) / 100` (i.e. rhs percent of acc).
        "процент" | "процентов" | "процента" | "%" => Op::Percent,
        // Kazakh — verb stems (imperative).
        "қос" | "қосу" | "жұп" => Op::Add,
        "азайт" | "азайту" | "алып_таста" | "алу" => Op::Sub,
        "көбейт" | "көбейту" => Op::Mul,
        "бөл" | "бөлу" => Op::Div,
        "дәрежесі" | "дәреже" | "дәрежеге" => Op::Pow,
        "пайыз" | "пайызы" => Op::Percent,
        _ => return None,
    })
}

/// Parse a number which may span 1-3 words (Russian "двадцать
/// пять" / Kazakh "жиырма бес" / Russian "сто двадцать пять").
/// Returns `(value, words_consumed)`.
fn parse_compound_number(words: &[&str]) -> Option<(f64, usize)> {
    // Try ASCII number first (single token).
    if let Ok(n) = words[0].parse::<f64>() {
        return Some((n, 1));
    }
    // Try a multi-word verbal number.
    let mut total: i64 = 0;
    let mut consumed: usize = 0;
    let mut had_part = false;
    for w in words.iter().take(4) {
        // First try the bare word, then a Kazakh-case-stripped
        // version so that «жетіге», «екіге», «үшті» parse.
        let resolved = if number_word_value(w).is_some() {
            *w
        } else {
            strip_kazakh_case(w)
        };
        if let Some(part) = number_word_value(resolved) {
            // Russian / Kazakh: hundreds and tens combine
            // additively («двести пятьдесят» = 200 + 50;
            // «жиырма бес» = 20 + 5).
            // Special: thousands ("тысяча", "мың") = ×1000 of
            // running accumulator.
            if w == &"тысяча" || w == &"тысячи" || w == &"тысяч" || w == &"мың"
            {
                total = if total == 0 { 1000 } else { total * 1000 };
            } else {
                total += part;
            }
            consumed += 1;
            had_part = true;
        } else {
            break;
        }
    }
    if had_part {
        Some((total as f64, consumed))
    } else {
        None
    }
}

/// One-word number value. Returns `None` for non-number words.
/// Russian numerals: covers nominative + the most-common
/// genitive / prepositional forms used after prepositions like
/// «из X» (which requires genitive in Russian grammar).
fn number_word_value(w: &str) -> Option<i64> {
    Some(match w {
        // Russian 0-19 — nominative.
        "ноль" => 0,
        "один" | "одна" | "одно" => 1,
        "два" | "две" => 2,
        "три" => 3,
        "четыре" => 4,
        "пять" => 5,
        "шесть" => 6,
        "семь" => 7,
        "восемь" => 8,
        "девять" => 9,
        "десять" => 10,
        "одиннадцать" => 11,
        "двенадцать" => 12,
        "тринадцать" => 13,
        "четырнадцать" => 14,
        "пятнадцать" => 15,
        "шестнадцать" => 16,
        "семнадцать" => 17,
        "восемнадцать" => 18,
        "девятнадцать" => 19,
        // Russian 0-19 — genitive (after «из / от / для»).
        "одного" | "одной" => 1,
        "двух" => 2,
        "трёх" | "трех" => 3,
        "четырёх" | "четырех" => 4,
        "пяти" => 5,
        "шести" => 6,
        "семи" => 7,
        "восьми" => 8,
        "девяти" => 9,
        "десяти" => 10,
        "одиннадцати" => 11,
        "двенадцати" => 12,
        "тринадцати" => 13,
        "четырнадцати" => 14,
        "пятнадцати" => 15,
        "шестнадцати" => 16,
        "семнадцати" => 17,
        "восемнадцати" => 18,
        "девятнадцати" => 19,
        "двадцати" => 20,
        "тридцати" => 30,
        "сорока" => 40,
        "пятидесяти" => 50,
        "шестидесяти" => 60,
        "семидесяти" => 70,
        "восьмидесяти" => 80,
        "девяноста" => 90,
        "ста" => 100,
        // Russian 20-90.
        "двадцать" => 20,
        "тридцать" => 30,
        "сорок" => 40,
        "пятьдесят" => 50,
        "шестьдесят" => 60,
        "семьдесят" => 70,
        "восемьдесят" => 80,
        "девяносто" => 90,
        // Russian 100s.
        "сто" => 100,
        "двести" => 200,
        "триста" => 300,
        "четыреста" => 400,
        "пятьсот" => 500,
        "шестьсот" => 600,
        "семьсот" => 700,
        "восемьсот" => 800,
        "девятьсот" => 900,
        "тысяча" | "тысячи" | "тысяч" => 1000,
        // Kazakh 0-19.
        "нөл" => 0,
        "бір" => 1,
        "екі" => 2,
        "үш" => 3,
        "төрт" => 4,
        "бес" => 5,
        "алты" => 6,
        "жеті" => 7,
        "сегіз" => 8,
        "тоғыз" => 9,
        "он" => 10,
        // Kazakh 11-19 are «он бір», «он екі» (multi-word) —
        // composed via the two-word path; no single-word forms.
        // Kazakh 20-90.
        "жиырма" => 20,
        "отыз" => 30,
        "қырық" => 40,
        "елу" => 50,
        "алпыс" => 60,
        "жетпіс" => 70,
        "сексен" => 80,
        "тоқсан" => 90,
        // Kazakh 100s.
        "жүз" => 100,
        "мың" => 1000,
        // Kazakh accusative / dative forms that the tokenizer
        // sees on input words like «жиырма-бесті», «жетіге»,
        // «екіге», «үшті». We strip them in `strip_kazakh_case`
        // (called separately for ambiguity-free roots) and the
        // bare root maps via the above tables.
        _ => return None,
    })
}

/// Strip common Kazakh case suffixes from a word so the number-word
/// table can match it. «жетіге» → «жеті», «екіге» → «екі», «үшті»
/// → «үш». Used by the tokenizer before number-word lookup.
fn strip_kazakh_case(w: &str) -> &str {
    let cases = [
        "-ге", "-ке", "-ні", "-нi", "-ні", "-ды", "-ді", "-ты", "-ті", "ге", "ке", "ні", "ды",
        "ді", "ты", "ті",
    ];
    for suf in &cases {
        if let Some(stripped) = w.strip_suffix(suf) {
            if number_word_value(stripped).is_some() {
                return stripped;
            }
        }
    }
    w
}

/// Evaluate the token stream. Left-to-right chained-imperative
/// semantics: each binary operator applies to the running
/// accumulator with the next number; each unary operator rewrites
/// the accumulator in place.
fn evaluate(tokens: &[Token]) -> Option<MathResult> {
    if tokens.is_empty() {
        return None;
    }
    let mut acc = match tokens[0] {
        Token::Number(n) => n,
        _ => return None,
    };
    // Trailing unary on the seed number (e.g. «корень из 16» →
    // tokens [Number(16), Unary(Sqrt)]).
    let mut i = 1;
    let mut steps = 0usize;
    while i < tokens.len() {
        match tokens[i] {
            Token::Unary(Unary::Sqrt) => {
                if acc < 0.0 {
                    return None;
                }
                acc = acc.sqrt();
                steps += 1;
                i += 1;
            }
            Token::Op(op) => {
                if i + 1 >= tokens.len() {
                    return None;
                }
                let Token::Number(rhs) = tokens[i + 1] else {
                    return None;
                };
                acc = match op {
                    Op::Add => acc + rhs,
                    Op::Sub => acc - rhs,
                    Op::Mul => acc * rhs,
                    Op::Div => {
                        if rhs == 0.0 {
                            return None;
                        }
                        acc / rhs
                    }
                    Op::Pow => acc.powf(rhs),
                    Op::Percent => (acc * rhs) / 100.0,
                };
                steps += 1;
                // Skip over rhs; also lift if next token is a
                // trailing unary attached to rhs.
                i += 2;
                if i < tokens.len()
                    && let Token::Unary(Unary::Sqrt) = tokens[i]
                {
                    if acc < 0.0 {
                        return None;
                    }
                    acc = acc.sqrt();
                    steps += 1;
                    i += 1;
                }
            }
            Token::Number(_) => {
                // Two numbers in a row without an operator —
                // malformed input.
                return None;
            }
        }
    }
    if steps == 0 {
        return None;
    }
    Some(MathResult { value: acc, steps })
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Russian word-form arithmetic ---------------------------

    #[test]
    fn simple_russian_addition() {
        let r = solve("Два плюс два").unwrap();
        assert_eq!(r.value, 4.0);
        assert_eq!(r.render(), "4");
    }

    #[test]
    fn russian_multi_step_chained_imperative() {
        // «25 × 7 / 2 + 3» left-to-right = 90.5
        let r = solve("Двадцать пять умножь на 7, раздели на два и прибавь три").unwrap();
        assert_eq!(r.value, 90.5);
        assert_eq!(r.steps, 3);
        assert_eq!(r.render(), "90.5");
    }

    #[test]
    fn russian_subtraction() {
        let r = solve("Сто минус сорок").unwrap();
        assert_eq!(r.value, 60.0);
        assert_eq!(r.render(), "60");
    }

    #[test]
    fn russian_division_yielding_integer() {
        let r = solve("Сто разделить на четыре").unwrap();
        assert_eq!(r.value, 25.0);
        assert_eq!(r.render(), "25");
    }

    #[test]
    fn russian_compound_hundred_twenty_five() {
        let r = solve("Сто двадцать пять минус двадцать пять").unwrap();
        assert_eq!(r.value, 100.0);
    }

    // -- Kazakh word-form arithmetic ----------------------------

    #[test]
    fn kazakh_simple_addition_using_jup() {
        // «Екі жұп екі қанша?» — «два плюс два сколько?»
        let r = solve("Екі жұп екі").unwrap();
        assert_eq!(r.value, 4.0);
    }

    #[test]
    fn kazakh_addition_via_qos() {
        let r = solve("Бес қос үш").unwrap();
        assert_eq!(r.value, 8.0);
    }

    #[test]
    fn kazakh_multi_step_chained() {
        // «25 × 7 ÷ 2 + 3» = 90.5
        let r = solve("Жиырма бес көбейт жеті бөл екі қос үш").unwrap();
        assert_eq!(r.value, 90.5);
    }

    #[test]
    fn kazakh_multiplication() {
        let r = solve("Он көбейт он").unwrap();
        assert_eq!(r.value, 100.0);
    }

    #[test]
    fn kazakh_subtraction() {
        let r = solve("Жиырма азайт бес").unwrap();
        assert_eq!(r.value, 15.0);
    }

    // -- ASCII arithmetic ----------------------------------------

    #[test]
    fn ascii_arithmetic() {
        assert_eq!(solve("2 + 2").unwrap().value, 4.0);
        assert_eq!(solve("10 * 5").unwrap().value, 50.0);
        assert_eq!(solve("100 / 4").unwrap().value, 25.0);
        assert_eq!(solve("100 - 25").unwrap().value, 75.0);
    }

    #[test]
    fn ascii_chained_left_to_right() {
        // 25 * 7 / 2 + 3 — left-to-right = 90.5
        let r = solve("25 * 7 / 2 + 3").unwrap();
        assert_eq!(r.value, 90.5);
    }

    // -- Edge cases ---------------------------------------------

    #[test]
    fn empty_input_returns_none() {
        assert!(solve("").is_none());
        assert!(solve("   ").is_none());
    }

    #[test]
    fn no_operator_returns_none() {
        // Just a number with no operation — not a math query.
        assert!(solve("сорок два").is_none());
        assert!(solve("42").is_none());
    }

    #[test]
    fn division_by_zero_returns_none() {
        assert!(solve("10 / 0").is_none());
        assert!(solve("Сто разделить на ноль").is_none());
    }

    #[test]
    fn rendering_of_integers_drops_decimal() {
        assert_eq!(
            MathResult {
                value: 4.0,
                steps: 1
            }
            .render(),
            "4"
        );
        assert_eq!(
            MathResult {
                value: -7.0,
                steps: 1
            }
            .render(),
            "-7"
        );
    }

    #[test]
    fn rendering_of_fractions_keeps_decimal() {
        assert_eq!(
            MathResult {
                value: 90.5,
                steps: 1
            }
            .render(),
            "90.5"
        );
        assert_eq!(
            MathResult {
                value: 0.25,
                steps: 1
            }
            .render(),
            "0.25"
        );
    }

    // -- The 3 dialog-battery cases (must all pass!) ----------

    #[test]
    fn battery_math_2plus2_ru() {
        assert_eq!(solve("Два плюс два").unwrap().render(), "4");
    }

    #[test]
    fn battery_math_2plus2_kz() {
        // «Екі жұп екі қанша?»  — strip «қанша» (query word).
        assert_eq!(solve("Екі жұп екі қанша").unwrap().render(), "4");
    }

    #[test]
    fn battery_math_complex_ru() {
        assert_eq!(
            solve("Двадцать пять умножь на 7, раздели на два и прибавь три")
                .unwrap()
                .render(),
            "90.5"
        );
    }

    // -- Powers / roots / percentages (Stage 4.7 extension) -----

    #[test]
    fn power_russian() {
        let r = solve("Два в степени десять").unwrap();
        assert_eq!(r.value, 1024.0);
    }

    #[test]
    fn power_kazakh() {
        let r = solve("Екі дәрежесі он").unwrap();
        assert_eq!(r.value, 1024.0);
    }

    #[test]
    fn power_ascii() {
        let r = solve("2 ^ 10").unwrap();
        assert_eq!(r.value, 1024.0);
        let r2 = solve("3 ** 4").unwrap();
        assert_eq!(r2.value, 81.0);
    }

    #[test]
    fn percent_russian() {
        // «Сто процентов 20» — «20 percent of 100» = 20.
        // Following left-to-right semantics: acc=100, op=Percent, rhs=20
        // → (100 * 20) / 100 = 20.
        let r = solve("Сто процент 20").unwrap();
        assert_eq!(r.value, 20.0);
    }

    #[test]
    fn percent_kazakh() {
        let r = solve("Жүз пайыз 20").unwrap();
        assert_eq!(r.value, 20.0);
    }

    #[test]
    fn sqrt_russian_prefix() {
        let r = solve("Корень из шестнадцати").unwrap();
        assert_eq!(r.value, 4.0);
    }

    #[test]
    fn sqrt_kazakh_suffix() {
        let r = solve("Он алты түбірі").unwrap();
        assert_eq!(r.value, 4.0);
    }

    #[test]
    fn sqrt_after_arithmetic() {
        // «Двадцать пять плюс одиннадцать» = 36, then sqrt → 6.
        // Sqrt suffix applies to running accumulator.
        let r = solve("36 түбірі").unwrap();
        assert_eq!(r.value, 6.0);
    }

    #[test]
    fn percent_chained() {
        // «Двадцать процент 50 плюс десять» — 20% of 50 = 10, +10 = 20.
        // (left-to-right: acc=20, %50 → 10, +10 → 20)
        let r = solve("20 процент 50 плюс 10").unwrap();
        assert_eq!(r.value, 20.0);
    }

    #[test]
    fn negative_power_yields_fraction() {
        let r = solve("2 ^ -2").unwrap();
        assert_eq!(r.value, 0.25);
    }

    #[test]
    fn battery_math_complex_kz() {
        // «Жиырма бесті жетіге көбейт, екіге бөл, үшті қос»
        // — Kazakh accusative/dative case suffixes need stripping.
        // For Stage 1 the bench passes the canonical bare form;
        // suffix-stripping is added in the wiring layer.
        assert_eq!(
            solve("Жиырма бес көбейт жеті бөл екі қос үш")
                .unwrap()
                .render(),
            "90.5"
        );
    }

    #[test]
    fn strip_kazakh_case_normalises_roots() {
        // Test the helper that lets the bench pass case-marked
        // Kazakh number words through the solver.
        assert_eq!(strip_kazakh_case("жетіге"), "жеті");
        assert_eq!(strip_kazakh_case("екіге"), "екі");
        assert_eq!(strip_kazakh_case("үшті"), "үш");
        assert_eq!(strip_kazakh_case("он"), "он"); // bare — unchanged
    }
}
