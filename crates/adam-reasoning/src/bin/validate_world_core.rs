//! `validate_world_core` — sanity-check the `data/world_core/*.jsonl`
//! knowledge packs before they flow into `facts.json`.
//!
//! This binary is the **authoring gate** for v3.9.0's curated knowledge
//! stack. Every pull request adding entries to `data/world_core/*.jsonl`
//! should run this first — CI will also run it via
//! `scripts/validate_foundation.sh` in a follow-up.
//!
//! Checks performed (per entry):
//!
//! 1. **Schema valid** — serde deserialisation succeeds.
//! 2. **Structural validity** — `id` / `kk` / `facts` / `domain` all
//!    non-empty; every fact has non-empty subject + object; no self-
//!    tautologies; no dash-prefixed fragment roots (the v3.9.0 Part A
//!    hygiene gate applies uniformly, curated data included).
//! 3. **Unique id across all domains** — ids namespace globally.
//! 4. **Kazakh-only audit** — every `kk` sentence contains only
//!    cyrillic letters, dash, ASCII digits, and common punctuation
//!    (same rule as the corpus-purity directive for curated packs).
//!
//! Exit codes:
//!   0 — all entries passed; prints summary per domain.
//!   1 — at least one entry failed; each failure is printed, summary
//!       at the end. Used as a CI gate.
//!
//! Usage:
//!   cargo run -p adam-reasoning --bin validate_world_core

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

use adam_reasoning::world_core::{ReviewStatus, WorldCoreEntry, load_world_core_dir};

const WORLD_CORE_ROOT: &str = "data/world_core";

fn main() -> ExitCode {
    let root = PathBuf::from(WORLD_CORE_ROOT);
    eprintln!("validate_world_core: scanning {}", root.display());

    let report = match load_world_core_dir(&root) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("fatal: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Per-domain stats over the accepted entries.
    let mut per_domain: BTreeMap<String, DomainStats> = BTreeMap::new();
    for (entry, _) in &report.entries {
        let stats = per_domain.entry(entry.domain.clone()).or_default();
        stats.total += 1;
        match entry.review_status {
            ReviewStatus::Approved => stats.approved += 1,
            ReviewStatus::Pending => stats.pending += 1,
            ReviewStatus::Rejected => stats.rejected += 1,
        }
        stats.fact_count += entry.facts.len();
        // Non-Kazakh sentence audit.
        if let Some(reason) = non_kazakh_reason(&entry.kk, &entry.domain) {
            stats.non_kazakh.push((entry.id.clone(), reason));
        }
    }

    // Pretty report.
    println!("## Domain summary\n");
    println!("| domain | entries | approved | pending | rejected | facts |");
    println!("|---|---:|---:|---:|---:|---:|");
    let mut grand_total = 0usize;
    let mut grand_approved = 0usize;
    let mut grand_facts = 0usize;
    for (domain, stats) in &per_domain {
        println!(
            "| {domain} | {} | {} | {} | {} | {} |",
            stats.total, stats.approved, stats.pending, stats.rejected, stats.fact_count,
        );
        grand_total += stats.total;
        grand_approved += stats.approved;
        grand_facts += stats.fact_count;
    }
    println!(
        "| **TOTAL** | **{grand_total}** | **{grand_approved}** | — | — | **{grand_facts}** |"
    );

    // Non-Kazakh warnings.
    let mut any_non_kazakh = false;
    for stats in per_domain.values() {
        for (id, reason) in &stats.non_kazakh {
            if !any_non_kazakh {
                println!("\n## Kazakh-purity warnings\n");
                any_non_kazakh = true;
            }
            println!("- `{id}` — {reason}");
        }
    }

    // Hard rejections.
    if !report.rejected.is_empty() {
        println!("\n## Rejected entries ({})\n", report.rejected.len());
        for err in &report.rejected {
            println!("- {err}");
        }
        eprintln!(
            "validate_world_core: {} entry/entries rejected",
            report.rejected.len()
        );
        return ExitCode::FAILURE;
    }

    eprintln!(
        "validate_world_core: OK — {grand_total} entries / {grand_approved} approved / {grand_facts} facts"
    );
    ExitCode::SUCCESS
}

#[derive(Debug, Default)]
struct DomainStats {
    total: usize,
    approved: usize,
    pending: usize,
    rejected: usize,
    fact_count: usize,
    non_kazakh: Vec<(String, String)>,
}

/// Domain-name prefixes whose Kazakh text inherently carries non-Kazakh
/// *letters* — programming keywords, chemical formulas, physics / maths
/// symbols.  In these domains the Cyrillic-only rule is relaxed to allow
/// non-Cyrillic letters (`async`, `Future-ды`, `H₂O`), because flagging a
/// domain's own subject matter is noise, not a purity signal.  General
/// knowledge domains (history, literature, geography …) stay strict.
const TECHNICAL_DOMAIN_PREFIXES: &[&str] = &[
    "programming",
    "rust_curriculum",
    "language_features",
    "computer_science",
    "chemistry",
    "physics",
    "mathematics",
];

fn domain_allows_latin(domain: &str) -> bool {
    TECHNICAL_DOMAIN_PREFIXES
        .iter()
        .any(|p| domain.starts_with(p))
}

/// Non-Cyrillic characters that are *notation, not prose*: maths / science
/// operators, typographic marks, sub- and superscripts, and the Greek
/// letters used as maths symbols.  Allowing these clears formula /
/// notation warnings in every domain without ever letting Latin or Russian
/// *prose* pass — letters are handled separately, so no word can sneak
/// through on the back of a symbol.
const ALLOWED_SYMBOLS: &[char] = &[
    // operators & comparison
    '*', '/', '+', '=', '<', '>', '±', '×', '÷', '·', '−', '≤', '≥', '≠', '≈', '≡', //
    // arrows & set notation
    '→', '←', '↔', '⟺', '⊂', //
    // typographic & structural punctuation
    '–', '_', '#', '|', '&', '\\', '[', ']', '{', '}', '\'', '…', '~', '′', '″', //
    // units & marks
    '°', '№', '%', '‰', 'µ', '√', '∞', //
    // superscripts / subscripts
    '⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹', '⁺', '⁻', //
    '₀', '₁', '₂', '₃', '₄', '₅', '₆', '₇', '₈', '₉', //
    // Greek letters used as maths / science symbols
    'α', 'β', 'γ', 'δ', 'ε', 'θ', 'λ', 'π', 'ρ', 'σ', 'τ', 'φ', 'ω', 'Δ', 'Φ', 'Ω', 'Σ', 'Π',
];

/// Returns `Some(reason)` if the Kazakh sentence contains characters
/// outside the allowed set. Allowed: Cyrillic (Kazakh alphabet), ASCII
/// digits, common punctuation, whitespace, em-dash, quotes, plus the
/// science / maths notation in [`ALLOWED_SYMBOLS`].
///
/// **Technical-text carve-outs.**
/// 1. **Backtick spans** (`fn`, `let`, `Vec<T>`, `Cargo.toml`) are treated
///    as code identifiers and bypass the check (since v4.7.0).
/// 2. **Notation symbols** ([`ALLOWED_SYMBOLS`]) are allowed everywhere —
///    formulas and units are not loanwords.
/// 3. **Technical domains** ([`TECHNICAL_DOMAIN_PREFIXES`]) additionally
///    allow non-Cyrillic letters, so a Rust or chemistry entry can name
///    its own vocabulary (`async`, `H₂O`, `Future-ды`) without a warning.
///
/// Bare Latin prose in a general-knowledge domain is still flagged, so the
/// Kazakh-only directive holds where it matters.
fn non_kazakh_reason(kk: &str, domain: &str) -> Option<String> {
    let allow_latin = domain_allows_latin(domain);
    let mut in_code = false;
    for ch in kk.chars() {
        if ch == '`' {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        let ok = ch.is_whitespace()
            || matches!(
                ch,
                ',' | '.' | ';' | ':' | '-' | '—' | '«' | '»' | '"' | '(' | ')' | '?' | '!'
            )
            || ch.is_ascii_digit()
            || is_cyrillic(ch)
            || ALLOWED_SYMBOLS.contains(&ch)
            || (allow_latin && ch.is_alphabetic() && !is_cyrillic(ch));
        if !ok {
            return Some(format!(
                "contains non-Kazakh / non-punctuation character: `{ch}` (U+{:04X})",
                ch as u32
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn technical_domain_allows_latin_vocabulary() {
        assert!(non_kazakh_reason("async fn — Future-ды қайтарады", "programming_rust").is_none());
        assert!(non_kazakh_reason("H₂O — судың формуласы", "chemistry_school").is_none());
    }

    #[test]
    fn general_domain_still_flags_bare_latin() {
        // A stray Latin word in a history entry must still warn.
        assert!(non_kazakh_reason("бұл hello деген сөз", "world_history").is_some());
    }

    #[test]
    fn notation_symbols_allowed_everywhere() {
        // Units, operators, superscripts, № — all notation, no letters.
        assert!(
            non_kazakh_reason("ауданы 5 м² · 2 = 10, №1, ≥ 3, ±0.5 → 7", "world_history").is_none()
        );
    }

    #[test]
    fn degree_letter_still_flags_in_general_domain() {
        // «°C» carries a Latin C; outside a technical domain that is still
        // a warning (write °С in Cyrillic, or use a technical domain).
        assert!(non_kazakh_reason("температура 5 °C", "world_history").is_some());
    }

    #[test]
    fn backtick_code_span_still_bypasses() {
        assert!(non_kazakh_reason("`Vec<T>` дегеніміз тізбек", "world_history").is_none());
    }
}

fn is_cyrillic(ch: char) -> bool {
    matches!(ch, 'А'..='я') || matches!(ch, 'Ё' | 'ё')
        // Kazakh-specific extensions.
        || matches!(
            ch,
            'Ә' | 'ә' | 'Ғ' | 'ғ' | 'Қ' | 'қ' | 'Ң' | 'ң' | 'Ө' | 'ө' | 'Ұ' | 'ұ' | 'Ү' | 'ү' | 'Һ' | 'һ' | 'І' | 'і'
        )
}

#[allow(dead_code)]
fn summarise_entry(e: &WorldCoreEntry) -> String {
    format!(
        "{} [{}] — {} facts, review={:?}",
        e.id,
        e.domain,
        e.facts.len(),
        e.review_status
    )
}
