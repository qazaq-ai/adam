// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `validate_procedures` — **v6.8.27 — authoring gate for
//! `data/procedures/*.jsonl`.**
//!
//! Codex 2026-06-25 priority #4: ProcedureIR data scale needs
//! an importer/validator BEFORE the curated set grows from
//! today's 15 to the 50–100 target.  This binary is that gate:
//! run it locally before pushing new procedures; CI runs it
//! via `scripts/validate_foundation.sh` so a malformed JSONL
//! line never reaches `procedure_loader::shared_procedures()`.
//!
//! ## Checks performed
//!
//! 1. **Schema valid** — `serde_json::from_str::<ProcedureIR>`
//!    succeeds (catches typos in field names, wrong types,
//!    missing required fields).
//! 2. **Structural invariants** — runs `check_invariants()` per
//!    record (non-empty id / title / steps; monotonic step
//!    sequence; non-empty `source.version_date`).
//! 3. **Unique id across all files** — collisions reported
//!    with both source paths.
//! 4. **Freshness lint** — `source.version_date` more than
//!    `STALE_YEARS` years older than today triggers a warning
//!    (NOT a failure; some regulations are decade-old and
//!    still in force).
//! 5. **Trilingual coverage report** — per file: count of
//!    records with `title_ru` filled / with `title_en` filled
//!    / with at least one entry in any of `aliases_kk` /
//!    `aliases_ru` / `aliases_en`.  Not a gate — coverage is
//!    bounded curatorial work.
//!
//! ## Exit codes
//!
//! - 0 — schema + invariants + uniqueness all passed.
//!   Freshness + coverage reported as INFO only.
//! - 1 — at least one hard failure (schema / invariant /
//!   collision).  CI-blocking.
//!
//! ## Usage
//!
//! ```sh
//! cargo run -p adam-algebra --bin validate_procedures
//! ```

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use adam_algebra::ProcedureIR;

const PROCEDURES_ROOT: &str = "data/procedures";

/// Records authored against a regulation older than this many
/// years trigger a freshness WARNING.  Not a failure — some
/// Kazakh labour-code articles haven't been revised in a
/// decade and are still authoritative.  Tune by emperia.
const STALE_YEARS: i64 = 7;

/// Today's date for the freshness lint, stamped at build time
/// via the `BUILD_DATE` env var (set in `build.rs`) or falling
/// back to a hard-coded constant when not set.  Validators
/// can't read system time deterministically inside CI, so this
/// keeps results reproducible.  Update on each major release.
const FALLBACK_TODAY: &str = "2026-06-26";

fn main() -> ExitCode {
    let root = PathBuf::from(PROCEDURES_ROOT);
    eprintln!("validate_procedures: scanning {}", root.display());

    let files = match collect_jsonl_files(&root) {
        Ok(v) if v.is_empty() => {
            eprintln!("  (no .jsonl files under {})", root.display());
            return ExitCode::SUCCESS;
        }
        Ok(v) => v,
        Err(e) => {
            eprintln!("ERROR walking {}: {e}", root.display());
            return ExitCode::FAILURE;
        }
    };

    let mut failed = false;
    let mut by_id: HashMap<String, PathBuf> = HashMap::new();
    let mut totals = Totals::default();

    for file in &files {
        let content = match fs::read_to_string(file) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("ERROR reading {}: {e}", file.display());
                failed = true;
                continue;
            }
        };
        let mut file_count = 0usize;
        let mut file_ru = 0usize;
        let mut file_en = 0usize;
        let mut file_aliased = 0usize;
        for (line_no, raw) in content.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            let proc = match ProcedureIR::from_jsonl_line(line) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("ERROR {}:{}: {e}", file.display(), line_no + 1);
                    failed = true;
                    continue;
                }
            };
            if let Some(prior) = by_id.get(&proc.id) {
                eprintln!(
                    "ERROR duplicate procedure id `{}` at {}:{} (first seen in {})",
                    proc.id,
                    file.display(),
                    line_no + 1,
                    prior.display()
                );
                failed = true;
            } else {
                by_id.insert(proc.id.clone(), file.clone());
            }
            // Freshness — soft warning.
            if let Some(years) = years_since(&proc.source.version_date) {
                if years >= STALE_YEARS {
                    eprintln!(
                        "WARN  {} regulation `{}` is {} year(s) old (version_date {})",
                        proc.id, proc.source.regulation_id, years, proc.source.version_date
                    );
                }
            }
            // Coverage tally.
            file_count += 1;
            if proc.title_ru.is_some() {
                file_ru += 1;
            }
            if proc.title_en.is_some() {
                file_en += 1;
            }
            if !proc.aliases_kk.is_empty()
                || !proc.aliases_ru.is_empty()
                || !proc.aliases_en.is_empty()
            {
                file_aliased += 1;
            }
        }
        if file_count > 0 {
            eprintln!(
                "  {} : {} records | ru {}/{}, en {}/{}, aliased {}/{}",
                file.display(),
                file_count,
                file_ru,
                file_count,
                file_en,
                file_count,
                file_aliased,
                file_count
            );
        }
        totals.records += file_count;
        totals.ru_titles += file_ru;
        totals.en_titles += file_en;
        totals.aliased += file_aliased;
    }

    eprintln!(
        "TOTAL: {} records | title_ru {}/{} ({}%), title_en {}/{} ({}%), aliased {}/{} ({}%)",
        totals.records,
        totals.ru_titles,
        totals.records,
        pct(totals.ru_titles, totals.records),
        totals.en_titles,
        totals.records,
        pct(totals.en_titles, totals.records),
        totals.aliased,
        totals.records,
        pct(totals.aliased, totals.records),
    );

    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

#[derive(Default)]
struct Totals {
    records: usize,
    ru_titles: usize,
    en_titles: usize,
    aliased: usize,
}

fn pct(num: usize, denom: usize) -> usize {
    (num * 100).checked_div(denom).unwrap_or(0)
}

fn collect_jsonl_files(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !root.exists() {
        return Ok(out);
    }
    for entry in fs::read_dir(root)? {
        let p = entry?.path();
        if p.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            out.push(p);
        }
    }
    out.sort();
    Ok(out)
}

/// Best-effort year difference between `today` and `iso_date`.
/// Returns `None` when the date string isn't `YYYY-MM-DD` shape.
/// Plain string arithmetic — no external time crate.
fn years_since(iso_date: &str) -> Option<i64> {
    let today = std::env::var("ADAM_BUILD_DATE").unwrap_or_else(|_| FALLBACK_TODAY.to_string());
    let today_year: i64 = today.get(0..4)?.parse().ok()?;
    let date_year: i64 = iso_date.get(0..4)?.parse().ok()?;
    Some(today_year - date_year)
}
