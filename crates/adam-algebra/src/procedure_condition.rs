// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `procedure_condition` — **v6.8.13 (2026-06-23) — typed
//! numeric-condition parser + evaluator for SOP steps**.
//!
//! Codex's voice REPL audit on 2026-06-23 surfaced a concrete
//! industrial pilot gap: «Егер биіктік 1,5 метр болса СИЗ
//! керек пе?» answered «yes», but the kk_construction_height_005
//! fixture explicitly conditions PPE on «биіктік 1,8 м-ден
//! жоғары» (height above 1.8 m).  The cascade had no way to
//! evaluate the comparison — `ProcedureStep::condition` was a
//! free-text string the cascade couldn't reason over.
//!
//! This module turns that string into a typed [`ConditionExpr`]
//! and lets a typed [`ConditionInput`] from the user query be
//! evaluated against it.  Result is `Option<bool>`:
//!
//! - `Some(true)` — condition holds for the user's input;
//! - `Some(false)` — condition does NOT hold;
//! - `None` — variables don't match / condition unparseable /
//!   input unparseable.  Honest «I cannot answer» rather than
//!   guessing.
//!
//! ## v1 scope (deliberate — see commit message)
//!
//! - Single-clause conditions only (no AND / OR).
//! - Four comparators: `>` / `<` / `≥` / `≤`.
//! - Kazakh decimal separator («1,8» = 1.8).
//! - Same-unit assumption.  Different unit on either side returns
//!   `None` — no implicit conversion.
//! - Same-variable requirement.  User asking about «масса» when
//!   the step conditions on «биіктік» returns `None` (refuse
//!   honest mismatch).
//!
//! Each «not in v1» direction (multi-clause, unit conversion,
//! equality, range, Russian phrasings) is gated on the absence
//! of any existing fixture that needs it — YAGNI for the
//! current 15-procedure set.

use serde::{Deserialize, Serialize};

/// Typed predicate parsed from a `ProcedureStep::condition`
/// string.  Currently only numeric comparisons are modelled;
/// categorical state conditions («ауысым басталды» / «жұмыс
/// аяқталды») return `None` from [`parse_condition`] and the
/// caller falls back to legacy free-text behaviour.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConditionExpr {
    /// `<variable> <threshold> <unit>-ден жоғары` — strictly
    /// greater than.
    GreaterThan {
        var: String,
        threshold: f32,
        unit: String,
    },
    /// `<variable> <threshold> <unit>-нан төмен` — strictly less
    /// than.
    LessThan {
        var: String,
        threshold: f32,
        unit: String,
    },
    /// `<variable> <threshold> <unit>-нан кем емес` — greater
    /// than or equal.  Rare in current fixtures; included for
    /// parser symmetry.
    GreaterOrEqual {
        var: String,
        threshold: f32,
        unit: String,
    },
    /// `<variable> <threshold> <unit>-нан артық емес` — less
    /// than or equal.
    LessOrEqual {
        var: String,
        threshold: f32,
        unit: String,
    },
}

/// Typed numeric input parsed from a user query like «биіктік
/// 1,5 метр болса …».
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConditionInput {
    pub var: String,
    pub value: f32,
    pub unit: String,
}

/// Parse a `ProcedureStep::condition` string into a typed
/// [`ConditionExpr`].  Returns `None` for categorical
/// state conditions (no numeric comparison detectable) — the
/// caller treats those as opaque.
pub fn parse_condition(text: &str) -> Option<ConditionExpr> {
    let lower = text.trim().to_lowercase();
    let comparator = detect_comparator(&lower)?;
    // Find the comparator marker so we can split «<var> <num> <unit>» from the suffix.
    let marker_pos = comparator.marker_position(&lower)?;
    let lhs = lower[..marker_pos].trim_end_matches(|c: char| c == '-' || c.is_whitespace());
    let (var, threshold, unit) = parse_variable_value_unit(lhs)?;
    Some(comparator.into_expr(var, threshold, unit))
}

/// Parse a user query (or any sentence) for a numeric input
/// shape «<variable> <value> <unit>(-suffix) болса/болғанда/болса
/// не болады».  Returns the first such match, or `None`.
pub fn parse_user_input(text: &str) -> Option<ConditionInput> {
    let mut lower = text.trim().to_lowercase();
    // Strip Kazakh conditional prefix words so they don't end up
    // inside the variable token («егер биіктік 1,5 ...» must
    // parse as var=«биіктік», not var=«егер биіктік»).  Order
    // matters: longer phrases first.
    const CONDITIONAL_PREFIXES: &[&str] = &["егер де ", "егер ", "если ", "когда "];
    for p in CONDITIONAL_PREFIXES {
        if let Some(rest) = lower.strip_prefix(p) {
            lower = rest.to_string();
            break;
        }
    }
    // Strip a trailing «болса»/«болғанда» clause marker so the
    // unit token isn't confused with the trailing connective.
    let cleaned = lower
        .replace(" болса", " ")
        .replace(" болғанда", " ")
        .replace(" болса не", " ");
    let (var, value, unit) = parse_variable_value_unit(&cleaned)?;
    Some(ConditionInput { var, value, unit })
}

/// Evaluate a parsed [`ConditionExpr`] against a parsed
/// [`ConditionInput`].  Returns `None` when the variables or
/// units don't match — explicit refusal rather than a guess.
pub fn evaluate(expr: &ConditionExpr, input: &ConditionInput) -> Option<bool> {
    let (expr_var, threshold, expr_unit) = match expr {
        ConditionExpr::GreaterThan {
            var,
            threshold,
            unit,
        }
        | ConditionExpr::LessThan {
            var,
            threshold,
            unit,
        }
        | ConditionExpr::GreaterOrEqual {
            var,
            threshold,
            unit,
        }
        | ConditionExpr::LessOrEqual {
            var,
            threshold,
            unit,
        } => (var, *threshold, unit),
    };
    if !variables_match(expr_var, &input.var) {
        return None;
    }
    if !units_match(expr_unit, &input.unit) {
        return None;
    }
    Some(match expr {
        ConditionExpr::GreaterThan { .. } => input.value > threshold,
        ConditionExpr::LessThan { .. } => input.value < threshold,
        ConditionExpr::GreaterOrEqual { .. } => input.value >= threshold,
        ConditionExpr::LessOrEqual { .. } => input.value <= threshold,
    })
}

#[derive(Debug, Clone, Copy)]
enum Comparator {
    Gt,
    Lt,
    Ge,
    Le,
}

impl Comparator {
    /// Find where the comparator marker starts in the lower-cased
    /// condition.  Returns the byte position of the first char
    /// of the marker.
    fn marker_position(self, lower: &str) -> Option<usize> {
        for needle in self.markers() {
            if let Some(pos) = lower.find(needle) {
                return Some(pos);
            }
        }
        None
    }

    fn markers(self) -> &'static [&'static str] {
        match self {
            Self::Gt => &["жоғары", "артық", "көп", "асады", "асып"],
            Self::Lt => &["төмен", "кем", "аз", "төменгі"],
            Self::Ge => &["кем емес", "төмен емес"],
            Self::Le => &["артық емес", "көп емес", "жоғары емес"],
        }
    }

    fn into_expr(self, var: String, threshold: f32, unit: String) -> ConditionExpr {
        match self {
            Self::Gt => ConditionExpr::GreaterThan {
                var,
                threshold,
                unit,
            },
            Self::Lt => ConditionExpr::LessThan {
                var,
                threshold,
                unit,
            },
            Self::Ge => ConditionExpr::GreaterOrEqual {
                var,
                threshold,
                unit,
            },
            Self::Le => ConditionExpr::LessOrEqual {
                var,
                threshold,
                unit,
            },
        }
    }
}

/// Order matters: check «X емес» (≥ / ≤) BEFORE bare «X» (> / <),
/// otherwise «кем емес» (≥) would be misclassified as just «кем»
/// (<).
fn detect_comparator(lower: &str) -> Option<Comparator> {
    if lower.contains("кем емес") || lower.contains("төмен емес") {
        Some(Comparator::Ge)
    } else if lower.contains("артық емес")
        || lower.contains("көп емес")
        || lower.contains("жоғары емес")
    {
        Some(Comparator::Le)
    } else if Comparator::Gt.markers().iter().any(|m| lower.contains(m)) {
        Some(Comparator::Gt)
    } else if Comparator::Lt.markers().iter().any(|m| lower.contains(m)) {
        Some(Comparator::Lt)
    } else {
        None
    }
}

/// Parse a `<word> <number> <unit>` shape out of `lhs`.  Returns
/// `(variable, value, unit)` where:
///
/// - `variable` is the tokens BEFORE the first numeric token,
///   joined with single spaces and trimmed;
/// - `value` is the numeric token (Kazakh decimal separator
///   «1,8» is normalised to `1.8`);
/// - `unit` is the next non-empty token AFTER the numeric token,
///   stripped of common case-suffix markers (`-ден`, `-нан`,
///   `-нен`, `-ға`).
fn parse_variable_value_unit(lhs: &str) -> Option<(String, f32, String)> {
    let tokens: Vec<&str> = lhs.split_whitespace().collect();
    let mut number_idx = None;
    let mut parsed_value = 0.0f32;
    for (i, tok) in tokens.iter().enumerate() {
        if let Some(v) = parse_kazakh_number(tok) {
            number_idx = Some(i);
            parsed_value = v;
            break;
        }
    }
    let n = number_idx?;
    if n == 0 {
        return None; // no variable preceding the number
    }
    let var = tokens[..n].join(" ").trim().to_string();
    if var.is_empty() {
        return None;
    }
    let unit_tok_raw = tokens.get(n + 1)?;
    let unit = strip_case_suffix(unit_tok_raw);
    if unit.is_empty() {
        return None;
    }
    Some((var, parsed_value, unit))
}

/// Parse «1,8», «1.8», «100», «0» — Kazakh decimal comma
/// normalised to ASCII period.
fn parse_kazakh_number(token: &str) -> Option<f32> {
    // Strip trailing punctuation that often hugs a number in
    // free-form sentences («1,8.» / «1,8,»).
    let trimmed = token.trim_end_matches(['.', ',', ';', ':', '!', '?']);
    if trimmed.is_empty() {
        return None;
    }
    let normalised = trimmed.replace(',', ".");
    normalised.parse::<f32>().ok()
}

/// Strip common Kazakh case suffixes off a unit token so «м-ден»
/// / «м-нан» / «метр» all canonicalise to a comparable token.
fn strip_case_suffix(token: &str) -> String {
    let lowered = token.to_lowercase();
    let trimmed = lowered.trim_end_matches(['.', ',', ';', ':']);
    // Order matters — longer suffixes first.
    const SUFFIXES: &[&str] = &[
        "-ден", "-нан", "-нен", "-дан", "-тен", "-тан", "-ға", "-ге", "-қа", "-ке",
    ];
    for s in SUFFIXES {
        if let Some(stripped) = trimmed.strip_suffix(s) {
            return canonical_unit(stripped);
        }
    }
    canonical_unit(trimmed)
}

/// Canonicalise common Kazakh unit surfaces («метр» → «м»,
/// «градус» → «°c», «процент» → «%», etc.) so equivalence
/// comparison works.  Conservative — unknown surfaces pass
/// through unchanged.
fn canonical_unit(s: &str) -> String {
    match s.trim() {
        "м" | "метр" | "метра" | "метров" | "метры" => "м".into(),
        "см" | "сантиметр" | "сантиметра" | "сантиметров" => {
            "см".into()
        }
        "мм" | "миллиметр" | "миллиметра" | "миллиметров" => {
            "мм".into()
        }
        "кг" | "килограмм" | "килограмма" | "килограммов" => {
            "кг".into()
        }
        "тонна" | "тонн" | "т" => "т".into(),
        "г" | "грамм" | "грамма" | "граммов" => "г".into(),
        "атм" | "атмосфера" => "атм".into(),
        "бар" => "бар".into(),
        "па" | "паскаль" | "паскаля" => "па".into(),
        "градус" | "градусов" | "°c" => "°c".into(),
        "%" | "процент" | "процента" | "процентов" => "%".into(),
        other => other.to_string(),
    }
}

/// Variables match if their canonical forms are equal.  v1
/// equality is whitespace + case insensitive; richer synonym
/// support (e.g. «биіктік» ↔ «бой» ↔ «ұзындық») is a future
/// fixture-driven extension.
fn variables_match(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

fn units_match(a: &str, b: &str) -> bool {
    canonical_unit(&a.to_lowercase()) == canonical_unit(&b.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_height_threshold_canonical_case() {
        // Exactly the kk_construction_height_005 step 2 condition.
        let expr = parse_condition("биіктік 1,8 м-ден жоғары").expect("parse");
        assert_eq!(
            expr,
            ConditionExpr::GreaterThan {
                var: "биіктік".into(),
                threshold: 1.8,
                unit: "м".into(),
            },
        );
    }

    #[test]
    fn parse_pressure_below_threshold() {
        let expr = parse_condition("қысым 5 атм-нен төмен").expect("parse");
        assert!(matches!(
            expr,
            ConditionExpr::LessThan { ref var, threshold, ref unit }
            if var == "қысым" && (threshold - 5.0).abs() < 0.01 && unit == "атм"
        ));
    }

    #[test]
    fn parse_categorical_condition_returns_none() {
        // Existing fixtures have categorical state strings the
        // parser should explicitly NOT misread as numeric.
        assert!(parse_condition("ауысым басталды").is_none());
        assert!(parse_condition("тергеу аяқталды").is_none());
        assert!(parse_condition("акт ресімделген").is_none());
        assert!(parse_condition("кемшілік табылды").is_none());
    }

    #[test]
    fn parse_user_input_height_query() {
        let input = parse_user_input("биіктік 1,5 метр болса").expect("parse");
        assert_eq!(input.var, "биіктік");
        assert!((input.value - 1.5).abs() < 0.01);
        assert_eq!(input.unit, "м"); // «метр» canonicalised
    }

    /// **Regression coverage.**  «егер» (if) at the start must
    /// NOT end up inside the variable token — that would prevent
    /// the conditional input from matching a procedure step's
    /// condition.  v6.8.13 first-test failure surfaced this.
    #[test]
    fn parse_user_input_strips_kazakh_conditional_prefix() {
        let input = parse_user_input("Егер биіктік 1,5 метр болса").expect("parse");
        assert_eq!(input.var, "биіктік");
        assert!((input.value - 1.5).abs() < 0.01);
        let input = parse_user_input("Если высота 1,5 метров").expect("parse");
        assert_eq!(input.var, "высота");
    }

    #[test]
    fn evaluate_height_pilot_case_codex_audit() {
        // The exact Codex pilot scenario:
        //   condition: «биіктік 1,8 м-ден жоғары» (PPE needed
        //              when height > 1.8 m)
        //   user query: «биіктік 1,5 метр болса …»
        //   expected: false (1.5 m is NOT above 1.8 m)
        let expr = parse_condition("биіктік 1,8 м-ден жоғары").unwrap();
        let input = parse_user_input("биіктік 1,5 метр болса").unwrap();
        assert_eq!(evaluate(&expr, &input), Some(false));
    }

    #[test]
    fn evaluate_height_above_threshold() {
        let expr = parse_condition("биіктік 1,8 м-ден жоғары").unwrap();
        let input = parse_user_input("биіктік 2 метр болса").unwrap();
        assert_eq!(evaluate(&expr, &input), Some(true));
    }

    #[test]
    fn evaluate_variable_mismatch_returns_none() {
        // Honest «I cannot answer» when user asks about a
        // different variable than the condition is on.
        let expr = parse_condition("биіктік 1,8 м-ден жоғары").unwrap();
        let input = parse_user_input("масса 50 кг болса").unwrap();
        assert_eq!(evaluate(&expr, &input), None);
    }

    #[test]
    fn evaluate_unit_mismatch_returns_none() {
        // Same variable, different unit — v1 is conservative
        // and refuses rather than auto-converting.
        let expr = parse_condition("биіктік 1,8 м-ден жоғары").unwrap();
        // «биіктік 150 см» — same var, different unit (см vs м).
        let input = parse_user_input("биіктік 150 см болса").unwrap();
        assert_eq!(evaluate(&expr, &input), None);
    }

    #[test]
    fn canonical_unit_metric_synonyms() {
        assert_eq!(canonical_unit("метр"), "м");
        assert_eq!(canonical_unit("метров"), "м");
        assert_eq!(canonical_unit("килограмм"), "кг");
        assert_eq!(canonical_unit("процент"), "%");
        assert_eq!(canonical_unit("атмосфера"), "атм");
    }

    #[test]
    fn parse_kazakh_decimal_separator() {
        // Kazakh + Russian use «,» as decimal separator.
        assert_eq!(parse_kazakh_number("1,8"), Some(1.8));
        assert_eq!(parse_kazakh_number("1.8"), Some(1.8));
        assert_eq!(parse_kazakh_number("0"), Some(0.0));
        assert_eq!(parse_kazakh_number("100"), Some(100.0));
        // Trailing punctuation tolerated.
        assert_eq!(parse_kazakh_number("1,8."), Some(1.8));
    }
}
