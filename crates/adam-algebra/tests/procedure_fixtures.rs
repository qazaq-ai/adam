// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! Integration test that loads `data/procedures/labor_safety_kz.jsonl`
//! and asserts every record parses cleanly, satisfies the structural
//! invariants, and carries a regulation source within a reasonable
//! freshness window.
//!
//! The freshness window is *not* a tight gate — historical ГОСТ from
//! the Soviet era are still in force in Kazakhstan and intentionally
//! ship with very old `version_date` values.  The test asserts each
//! source EITHER falls within 5 years OR is explicitly tagged as a
//! ГОСТ (which by definition survives across decades).

use adam_algebra::{ProcedureIR, ProcedureSource};

const FIXTURE_PATH: &str = "../../data/procedures/labor_safety_kz.jsonl";

fn load_all() -> Vec<ProcedureIR> {
    let text =
        std::fs::read_to_string(FIXTURE_PATH).expect("labor_safety_kz.jsonl must be readable");
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| ProcedureIR::from_jsonl_line(l).expect("every line must parse + pass invariants"))
        .collect()
}

#[test]
fn fixtures_parse_and_pass_invariants() {
    let all = load_all();
    assert!(
        !all.is_empty(),
        "fixture file must carry at least one record"
    );
}

#[test]
fn fixtures_carry_currency_or_are_gost() {
    // Today defaults to the curator's last-known date; can be
    // overridden via `ADAM_TODAY=YYYY-MM-DD` for time-travel runs.
    let today = std::env::var("ADAM_TODAY").unwrap_or_else(|_| "2026-06-22".into());
    // 5-year freshness window — Kazakh Labor Code amendments
    // typically land more often than that; ГОСТ are exempt because
    // they're explicitly long-lived state standards.
    const MAX_AGE_DAYS: i64 = 1825;

    let all = load_all();
    for p in &all {
        let is_gost = is_gost_source(&p.source);
        let fresh = p
            .is_within_freshness_window(&today, MAX_AGE_DAYS)
            .expect("dates parse");
        assert!(
            fresh || is_gost,
            "procedure {} cites {} ({}) with version_date {} — outside the \
             5-year freshness window and not a ГОСТ; refresh the fixture or \
             confirm the regulation is still in force",
            p.id,
            p.source.regulation_kk,
            p.source.regulation_id,
            p.source.version_date,
        );
    }
}

#[test]
fn fixtures_have_unique_ids() {
    let all = load_all();
    let mut seen = std::collections::HashSet::new();
    for p in &all {
        assert!(
            seen.insert(p.id.clone()),
            "duplicate id in fixture file: {}",
            p.id,
        );
    }
}

fn is_gost_source(source: &ProcedureSource) -> bool {
    let id = source.regulation_id.to_uppercase();
    // Long-lived industry standards that survive across decades:
    // - ГОСТ / GOST — state standards (USSR + post-Soviet
    //   succession, still in force in Kazakhstan).
    // - СТ РК — Kazakh state standards.
    // - ПУЭ — Правила устройства электроустановок (Electrical
    //   Installation Code), the canonical electrical-safety
    //   rulebook used across CIS countries.  The 7th edition
    //   (2002-2003) is the current reference; not superseded.
    // - ПОТ — Правила охраны труда (long-lived safety codes).
    id.starts_with("ГОСТ")
        || id.starts_with("GOST")
        || id.starts_with("СТ РК")
        || id.starts_with("ПУЭ")
        || id.starts_with("ПОТ")
}
