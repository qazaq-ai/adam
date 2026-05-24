// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `time_units` — quantitative time-unit answers and false-statement
//! correction.
//!
//! **v6.1.50 — 2026-05-24 voice REPL audit fix.** Pre-v6.1.50 the
//! query «Жылда неше ай?» («how many months in a year?») routed to
//! topic_extraction on «жыл», fetched the «тоқсан» (quarter) fact
//! (semantically adjacent), and replied «Бір жылда төрт тоқсан
//! болады.» — wrong concept entirely. The user asked for a Count
//! answer-shape; the cascade had no such shape.
//!
//! This module ships a small closed-set table of canonical time-unit
//! counts + two detectors:
//!
//!   - `detect_time_unit_count_question` — pattern «X-та/-те/-да/-де
//!     неше Y?» where X and Y are recognised time units. Returns
//!     the canonical count and the canonical Kazakh wording.
//!   - `detect_time_unit_false_statement` — pattern «X-та N Y бар»
//!     where N differs from the canonical count. Returns the
//!     correction string so adam can disagree («Жоқ, дұрысы — N₀
//!     Y болады»).
//!
//! v6.2.0 will generalise this via the Count / Disagreement
//! AnswerIR shapes documented in
//! [[project_v6_2_architectural_pivot]]. v6.1.50 is the closed-set
//! patch that closes the user's specific audit findings.

/// Closed-set canonical time-unit counts. Source: standard Kazakh
/// timekeeping vocabulary; values are the unambiguous integer counts
/// every speaker agrees on (approximations like «month ≈ 30 days»
/// flagged in the `approx` field).
pub struct TimeUnitFact {
    /// Outer unit (the «контейнер»). Stored in nominative-singular
    /// form; the detector accepts -да / -де / -та / -те locative
    /// suffixes via stem-prefix matching.
    pub outer: &'static str,
    /// Inner unit (the «контент»). Same form rules as `outer`.
    pub inner: &'static str,
    /// Canonical count: how many `inner` units fit in one `outer`.
    pub count: u32,
    /// `true` when the count is a standard approximation (e.g. «one
    /// month is approximately 30 days»). The reply prepends
    /// «шамамен» (= "approximately") when set.
    pub approx: bool,
}

pub const TIME_UNIT_TABLE: &[TimeUnitFact] = &[
    TimeUnitFact {
        outer: "жыл",
        inner: "ай",
        count: 12,
        approx: false,
    },
    TimeUnitFact {
        outer: "жыл",
        inner: "тоқсан",
        count: 4,
        approx: false,
    },
    TimeUnitFact {
        outer: "жыл",
        inner: "апта",
        count: 52,
        approx: true,
    },
    TimeUnitFact {
        outer: "жыл",
        inner: "күн",
        count: 365,
        approx: true,
    },
    TimeUnitFact {
        outer: "тоқсан",
        inner: "ай",
        count: 3,
        approx: false,
    },
    TimeUnitFact {
        outer: "ай",
        inner: "күн",
        count: 30,
        approx: true,
    },
    TimeUnitFact {
        outer: "ай",
        inner: "апта",
        count: 4,
        approx: true,
    },
    TimeUnitFact {
        outer: "апта",
        inner: "күн",
        count: 7,
        approx: false,
    },
    TimeUnitFact {
        outer: "тәулік",
        inner: "сағат",
        count: 24,
        approx: false,
    },
    TimeUnitFact {
        outer: "күн",
        inner: "сағат",
        count: 24,
        approx: false,
    },
    TimeUnitFact {
        outer: "сағат",
        inner: "минут",
        count: 60,
        approx: false,
    },
    TimeUnitFact {
        outer: "минут",
        inner: "секунд",
        count: 60,
        approx: false,
    },
];

/// Does `token` look like the locative case of `unit`?
///
/// Accepts: unit + {да/де/та/те} (Kazakh locative variants by
/// phonetic class). E.g. unit="жыл" matches "жылда"; unit="ай"
/// matches "айда"; unit="сағат" matches "сағатта"; unit="тәулік"
/// matches "тәулікте".
fn is_locative_of(token: &str, unit: &str) -> bool {
    if !token.starts_with(unit) {
        return false;
    }
    let suffix = &token[unit.len()..];
    matches!(suffix, "да" | "де" | "та" | "те")
}

/// Detect a «X-та неше Y?» quantitative question over time units.
///
/// Returns `Some((outer, inner, count, approx))` when the pattern
/// matches; `None` otherwise.
///
/// **Discipline.** Surface-level word-by-word match — no FST. The
/// cascade calls this BEFORE topic extraction so a successful match
/// short-circuits the routing to a direct Count answer.
pub fn detect_time_unit_count_question(
    input: &str,
) -> Option<(&'static str, &'static str, u32, bool)> {
    let lower = input.to_lowercase();
    let mut tokens: Vec<&str> = lower
        .split(|c: char| !c.is_alphabetic())
        .filter(|t| !t.is_empty())
        .collect();
    // Require «неше» / «нешеу» somewhere in the input.
    if !tokens.iter().any(|t| *t == "неше" || *t == "нешеу") {
        return None;
    }
    // Look for an outer-unit-locative token + an inner-unit token
    // (inner token is the noun the «неше» modifies).
    for fact in TIME_UNIT_TABLE {
        let outer_match = tokens.iter().any(|t| is_locative_of(t, fact.outer));
        let inner_match = tokens.iter().any(|t| {
            *t == fact.inner
                || t.starts_with(fact.inner)
                    && (*t).chars().count() <= fact.inner.chars().count() + 3
        });
        if outer_match && inner_match {
            // Pop the «inner» token from candidates so the next
            // iteration doesn't double-count if multiple inner forms
            // share a prefix.
            tokens.retain(|t| !t.starts_with(fact.inner));
            return Some((fact.outer, fact.inner, fact.count, fact.approx));
        }
    }
    None
}

/// Render a Count answer for a successful detector match.
///
/// Format: «Бір {outer-да/-те} {count} {inner} болады.» Adds
/// «шамамен» (approximately) prefix when `approx`.
///
/// Examples:
///   - (жыл, ай, 12, false) → «Бір жылда 12 ай болады.»
///   - (ай, күн, 30, true)  → «Бір айда шамамен 30 күн болады.»
pub fn render_time_unit_count_answer(outer: &str, inner: &str, count: u32, approx: bool) -> String {
    let outer_loc = match outer {
        "жыл" => "жылда",
        "тоқсан" => "тоқсанда",
        "ай" => "айда",
        "апта" => "аптада",
        "тәулік" => "тәулікте",
        "күн" => "күнде",
        "сағат" => "сағатта",
        "минут" => "минутта",
        _ => outer,
    };
    if approx {
        format!("Бір {outer_loc} шамамен {count} {inner} болады.")
    } else {
        format!("Бір {outer_loc} {count} {inner} болады.")
    }
}

/// Detect a «X-та N Y бар/болады» false-or-true statement over
/// time units. Returns `Some((outer, inner, asserted_count,
/// canonical_count, approx))` when the pattern matches a known
/// outer-inner pair; the caller can compare counts to decide
/// whether to confirm or correct.
///
/// Pattern accepted: input contains a number (digits OR Kazakh
/// cardinals up to 100) followed by the inner-unit noun (within
/// 2 tokens), plus the outer-unit in locative form, plus «бар»
/// or «болады» as the predicate.
pub fn detect_time_unit_statement(
    input: &str,
) -> Option<(&'static str, &'static str, u32, u32, bool)> {
    let lower = input.to_lowercase();
    let tokens: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect();
    let has_predicate = tokens
        .iter()
        .any(|t| matches!(*t, "бар" | "болады" | "тұр"));
    if !has_predicate {
        return None;
    }
    for fact in TIME_UNIT_TABLE {
        let outer_match = tokens.iter().any(|t| is_locative_of(t, fact.outer));
        if !outer_match {
            continue;
        }
        // Find a cardinal that is IMMEDIATELY followed by the inner
        // unit (or its first few inflection tokens). «Бір сағатта 20
        // минут» — the «бір» modifies «сағат» (the outer), not the
        // assertion; the assertion is «20 минут». Walk the token
        // stream pairwise and require <cardinal> <inner-prefix>
        // adjacency.
        for (i, t) in tokens.iter().enumerate() {
            let value = t.parse::<u32>().ok().or_else(|| kazakh_cardinal(t));
            let Some(asserted) = value else { continue };
            let next = tokens.get(i + 1).copied().unwrap_or("");
            let next_matches_inner = next == fact.inner
                || (next.starts_with(fact.inner)
                    && next.chars().count() <= fact.inner.chars().count() + 3);
            if next_matches_inner {
                return Some((fact.outer, fact.inner, asserted, fact.count, fact.approx));
            }
        }
    }
    None
}

/// Render a correction or confirmation reply for a detected
/// time-unit statement. `correct == true` → user's assertion
/// matches the canonical count; `false` → correct them.
pub fn render_time_unit_statement_reply(
    outer: &str,
    inner: &str,
    asserted: u32,
    canonical: u32,
    approx: bool,
) -> String {
    let outer_loc = match outer {
        "жыл" => "жылда",
        "тоқсан" => "тоқсанда",
        "ай" => "айда",
        "апта" => "аптада",
        "тәулік" => "тәулікте",
        "күн" => "күнде",
        "сағат" => "сағатта",
        "минут" => "минутта",
        _ => outer,
    };
    if asserted == canonical {
        if approx {
            format!("Иә, дұрыс — бір {outer_loc} шамамен {canonical} {inner} болады.")
        } else {
            format!("Иә, дұрыс — бір {outer_loc} {canonical} {inner} болады.")
        }
    } else if approx {
        format!(
            "Кешіріңіз, дәл емес — бір {outer_loc} шамамен {canonical} {inner} болады, {asserted} емес."
        )
    } else {
        format!(
            "Кешіріңіз, дәл емес — бір {outer_loc} {canonical} {inner} болады, {asserted} емес."
        )
    }
}

/// Closed list of Kazakh cardinal numerals 1-100 + common round
/// numbers. Returns the integer value when `token` matches.
fn kazakh_cardinal(token: &str) -> Option<u32> {
    match token {
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
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_question_year_months() {
        let got = detect_time_unit_count_question("Жылда неше ай?");
        assert_eq!(got, Some(("жыл", "ай", 12, false)));
    }

    #[test]
    fn count_question_hour_minutes() {
        let got = detect_time_unit_count_question("Бір сағатта неше минут?");
        assert_eq!(got, Some(("сағат", "минут", 60, false)));
    }

    #[test]
    fn count_question_week_days() {
        let got = detect_time_unit_count_question("Бір аптада неше күн?");
        assert_eq!(got, Some(("апта", "күн", 7, false)));
    }

    #[test]
    fn count_question_year_days_approx() {
        let got = detect_time_unit_count_question("Жылда неше күн?");
        assert_eq!(got, Some(("жыл", "күн", 365, true)));
    }

    #[test]
    fn count_question_no_neshe_marker_returns_none() {
        assert_eq!(detect_time_unit_count_question("Жылда ай қанша?"), None);
    }

    #[test]
    fn count_question_non_time_unit_returns_none() {
        assert_eq!(
            detect_time_unit_count_question("Қалада неше адам бар?"),
            None
        );
    }

    #[test]
    fn render_count_answer_year_months() {
        assert_eq!(
            render_time_unit_count_answer("жыл", "ай", 12, false),
            "Бір жылда 12 ай болады."
        );
    }

    #[test]
    fn render_count_answer_year_days_approx() {
        assert_eq!(
            render_time_unit_count_answer("жыл", "күн", 365, true),
            "Бір жылда шамамен 365 күн болады."
        );
    }

    #[test]
    fn statement_false_hour_minutes() {
        // «Бір сағатта 20 минут бар.» — false (60 is canonical).
        let got = detect_time_unit_statement("Бір сағатта 20 минут бар.");
        assert_eq!(got, Some(("сағат", "минут", 20, 60, false)));
        let reply = render_time_unit_statement_reply("сағат", "минут", 20, 60, false);
        assert!(reply.starts_with("Кешіріңіз"));
        assert!(reply.contains("60 минут"));
        assert!(reply.contains("20 емес"));
    }

    #[test]
    fn statement_true_year_months() {
        let got = detect_time_unit_statement("Бір жылда 12 ай бар.");
        assert_eq!(got, Some(("жыл", "ай", 12, 12, false)));
        let reply = render_time_unit_statement_reply("жыл", "ай", 12, 12, false);
        assert!(reply.starts_with("Иә, дұрыс"));
    }

    #[test]
    fn statement_kazakh_cardinal_jiyrma() {
        let got = detect_time_unit_statement("Бір сағатта жиырма минут бар.");
        assert_eq!(got, Some(("сағат", "минут", 20, 60, false)));
    }
}
