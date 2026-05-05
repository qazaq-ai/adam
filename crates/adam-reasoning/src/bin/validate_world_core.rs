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
        if let Some(reason) = non_kazakh_reason(&entry.kk) {
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

/// Returns `Some(reason)` if the Kazakh sentence contains characters
/// outside the allowed set. Allowed: Cyrillic (Kazakh alphabet), ASCII
/// digits, dash, common punctuation, whitespace, em-dash, quotes.
///
/// **v4.7.0** — corpus-purity carve-out for technical text:
/// backtick-quoted spans (`fn`, `let`, `Vec<T>`, `Cargo.toml` etc.)
/// are treated as code identifiers and bypass the Cyrillic-only
/// check. The carve-out applies ONLY inside paired backticks; bare
/// Latin prose outside backticks is still flagged. This lets the
/// `programming_rust.jsonl` domain (and any future technical
/// domain) embed Rust keywords / types / commands verbatim while
/// keeping the Kazakh-only directive intact for free prose.
fn non_kazakh_reason(kk: &str) -> Option<String> {
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
            || is_cyrillic(ch);
        if !ok {
            return Some(format!(
                "contains non-Kazakh / non-punctuation character: `{ch}` (U+{:04X})",
                ch as u32
            ));
        }
    }
    None
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
