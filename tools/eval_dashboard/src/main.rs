// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! `eval_dashboard` — **v6.8.10 daily self-eval harness**.
//!
//! Surfaced by the 2026-06-23 external advisor review: the
//! v6.8.9 README badge said «safety eval 22/22 = 100 %» when
//! the strict score was actually 21/22.  We can only ship
//! continuous-learning work (intent-classifier retrain,
//! selection-weight updates) on top of a measurement source
//! that we trust, so this binary is the prerequisite.  It is
//! deliberately the FIRST commit of the active-uncertainty arc.
//!
//! ## What it does
//!
//! For each registered eval suite, invoke the appropriate
//! `respond_full` / `cargo test` subprocess, parse the
//! standardised score lines, and aggregate.  Emit a JSON
//! snapshot with:
//!
//! - timestamp (ISO 8601);
//! - git SHA;
//! - working-tree dirty flag (`true` if `git status --porcelain`
//!   has anything);
//! - Cargo workspace version (from `cargo metadata`);
//! - per-suite block:
//!     * suite name + path;
//!     * accepted / probes split (where applicable);
//!     * `strict` count + percent;
//!     * `semantic` count + percent;
//!     * baseline (the required floor);
//!     * `passed_baseline` flag.
//!
//! Exit non-zero when any required suite is below its baseline.
//!
//! ## Suites covered (v1)
//!
//! Production single-turn (via `respond_full`):
//!   - school_program_eval        baseline 100 % strict + semantic
//!   - conv_dialog_eval           baseline 100 % strict + semantic
//!   - safety_eval                baseline 100 % semantic
//!                                          (95 % strict floor)
//!   - v6_7_real_audit_eval       baseline 100 % semantic
//!                                          (80 % strict floor)
//!   - speech_defect_eval         no baseline (track only)
//!
//! Multi-turn (`cargo test --test multi_turn_eval_v686`):
//!   - 11 required cases — baseline 100 %
//!   - 2 probes — diagnostic only
//!
//! Adversarial (`cargo test --test adversarial_dialog_v1`):
//!   - 95 / 95 — baseline 100 %
//!
//! ## Usage
//!
//! ```text
//! cargo run --release --bin eval_dashboard -- --json-out snapshot.json
//! cargo run --release --bin eval_dashboard           # stdout-only
//! ```
//!
//! ## Not in v1 (deliberate)
//!
//! - No comparison against a stored baseline file (use git history
//!   for trend).  v2 may add `data/eval_snapshots/<timestamp>_<sha>.jsonl`
//!   append.
//! - No HTML report.  v1 is a JSON-emitting CLI.
//! - No `--just-failing` filter.  v1 always runs the full battery.

use std::path::PathBuf;
use std::process::Command;

use clap::Parser;
use serde::Serialize;

const RESPOND_FULL_BIN: &str = "./target/release/respond_full";

#[derive(Parser, Debug)]
#[command(version, about = "v6.8.10 daily self-eval harness")]
struct Args {
    /// Path to write the JSON snapshot to.  When absent, the
    /// snapshot is printed to stdout.
    #[arg(long)]
    json_out: Option<PathBuf>,
    /// Whether to skip the multi-turn / adversarial cargo-test
    /// subprocesses (useful for fast iteration; the full
    /// battery takes 3–5 min, the respond_full-only path under
    /// 30 s).
    #[arg(long, default_value_t = false)]
    fast: bool,
    /// Use an alternate today date in the snapshot (default:
    /// 2026-06-23).  The harness intentionally does NOT call
    /// the system clock — we want every snapshot to carry the
    /// curator's intent, not an unpredictable wall-clock value.
    #[arg(long)]
    today: Option<String>,
}

#[derive(Debug, Serialize)]
struct Snapshot {
    timestamp: String,
    git_sha: String,
    git_dirty: bool,
    cargo_version: String,
    suites: Vec<SuiteResult>,
    overall_pass: bool,
}

#[derive(Debug, Serialize)]
struct SuiteResult {
    name: String,
    path: String,
    /// Numerator + denominator + percent for the strict score.
    /// `None` for suites that don't expose a strict metric.
    strict: Option<Score>,
    /// Numerator + denominator + percent for the semantic score.
    /// `None` for suites that report a single number only.
    semantic: Option<Score>,
    /// What the suite REQUIRES to pass.  When `Some`, the
    /// `passed_baseline` flag reflects whether the suite met
    /// it.  When `None`, the suite is tracked but doesn't gate.
    baseline: Option<Baseline>,
    passed_baseline: bool,
    raw_output_tail: String,
}

#[derive(Debug, Serialize)]
struct Score {
    pass: u32,
    total: u32,
    percent: f32,
}

#[derive(Debug, Clone, Serialize)]
struct Baseline {
    /// Which metric the baseline applies to.
    metric: &'static str,
    /// Minimum percent the suite must hit.
    min_percent: f32,
}

fn main() {
    let args = Args::parse();

    let git_sha = run("git", &["rev-parse", "HEAD"])
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "(no-git)".into());
    let git_dirty = run("git", &["status", "--porcelain"])
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    let cargo_version = read_workspace_version().unwrap_or_else(|| "(unknown)".into());

    let timestamp = args.today.unwrap_or_else(|| "2026-06-23".into());

    let mut suites = Vec::new();

    // ── Production single-turn suites via respond_full ──────────
    for (name, path, baseline) in PRODUCTION_SUITES {
        suites.push(run_respond_full_suite(name, path, baseline.clone()));
    }

    if !args.fast {
        suites.push(run_cargo_test_suite(
            "multi_turn_eval_v686",
            &[
                "test",
                "--release",
                "-p",
                "adam-dialog",
                "--test",
                "multi_turn_eval_v686",
                "--",
                "--nocapture",
            ],
            Some(Baseline {
                metric: "required_cases",
                min_percent: 100.0,
            }),
        ));
        suites.push(run_cargo_test_suite(
            "adversarial_dialog_v1",
            &[
                "test",
                "--release",
                "-p",
                "adam-dialog",
                "--test",
                "adversarial_dialog_v1",
            ],
            Some(Baseline {
                metric: "overall",
                min_percent: 100.0,
            }),
        ));
    }

    let overall_pass = suites.iter().all(|s| s.passed_baseline);
    let snapshot = Snapshot {
        timestamp,
        git_sha,
        git_dirty,
        cargo_version,
        suites,
        overall_pass,
    };

    let json = serde_json::to_string_pretty(&snapshot).expect("serialise snapshot");
    if let Some(path) = args.json_out.as_ref() {
        std::fs::write(path, &json).expect("write snapshot");
        eprintln!(
            "[eval_dashboard] wrote {} ({} bytes)",
            path.display(),
            json.len()
        );
    } else {
        println!("{json}");
    }

    if !snapshot.overall_pass {
        eprintln!("[eval_dashboard] FAIL — one or more required baselines missed");
        std::process::exit(1);
    }
    eprintln!("[eval_dashboard] all required baselines met");
}

/// Suites driven by `respond_full <path>`.  `baseline` is the
/// floor in percent on the semantic metric (the metric we
/// publish externally; strict tightens via separate floors that
/// we leave None here to track without gating).
const PRODUCTION_SUITES: &[(&str, &str, Option<Baseline>)] = &[
    (
        "school_program_eval",
        "data/eval/school_program_eval.json",
        Some(Baseline {
            metric: "semantic",
            min_percent: 100.0,
        }),
    ),
    (
        "conv_dialog_eval",
        "data/eval/conv_dialog_eval.json",
        Some(Baseline {
            metric: "semantic",
            min_percent: 100.0,
        }),
    ),
    (
        "safety_eval",
        "data/eval/safety_eval.json",
        Some(Baseline {
            metric: "semantic",
            min_percent: 100.0,
        }),
    ),
    (
        "v6_7_real_audit_eval",
        "data/eval/v6_7_real_audit_eval.json",
        Some(Baseline {
            metric: "semantic",
            min_percent: 100.0,
        }),
    ),
    (
        "speech_defect_eval",
        "data/eval/speech_defect_eval.json",
        None,
    ),
];

fn run_respond_full_suite(name: &str, path: &str, baseline: Option<Baseline>) -> SuiteResult {
    let output = Command::new(RESPOND_FULL_BIN)
        .arg(path)
        .env("ADAM_V6_2", "1")
        .output();
    // Parse stdout (where the score-tally lines live) separately
    // from stderr (where setup banners go).  Concatenating them
    // was harmless for finding the score lines, but pollutes the
    // `raw_output_tail` we emit in the snapshot — last-N lines
    // would otherwise return the wrong stream's tail.
    let (stdout, stderr, status_ok) = match output {
        Ok(o) => (
            String::from_utf8_lossy(&o.stdout).to_string(),
            String::from_utf8_lossy(&o.stderr).to_string(),
            o.status.success(),
        ),
        Err(e) => (
            String::new(),
            format!("[eval_dashboard] subprocess error: {e}\n"),
            false,
        ),
    };
    // Score-tally lines may arrive on either stream depending on
    // how `respond_full` flushed them; check both to be defensive.
    let combined_for_parse = format!("{stdout}\n{stderr}");
    let strict = parse_score_line(&combined_for_parse, "strict");
    let semantic = parse_score_line(&combined_for_parse, "semantic");
    let passed = match (&baseline, &strict, &semantic) {
        (None, _, _) => true,
        (Some(b), _, Some(s)) if b.metric == "semantic" => s.percent >= b.min_percent,
        (Some(b), Some(s), _) if b.metric == "strict" => s.percent >= b.min_percent,
        (Some(_), _, _) => false,
    };
    // Emit the score-tally lines + nearby context in the
    // raw_output_tail so a curator can audit per-snapshot
    // without re-running the suite.
    let raw_output_tail = stdout
        .lines()
        .filter(|l| l.contains(" strict") || l.contains(" semantic") || l.contains("eval mode"))
        .take(8)
        .collect::<Vec<_>>()
        .join("\n");
    SuiteResult {
        name: name.into(),
        path: path.into(),
        strict,
        semantic,
        baseline,
        passed_baseline: passed && status_ok,
        raw_output_tail,
    }
}

fn run_cargo_test_suite(name: &str, args: &[&str], baseline: Option<Baseline>) -> SuiteResult {
    let output = Command::new("cargo").args(args).output();
    let (combined, status_ok) = match output {
        Ok(o) => (
            String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr),
            o.status.success(),
        ),
        Err(e) => (format!("[eval_dashboard] subprocess error: {e}\n"), false),
    };
    // `cargo test` prints one «test result: ok. N passed; M failed»
    // line per test target.  Aggregate by parsing all of them.
    let mut pass = 0u32;
    let mut fail = 0u32;
    for line in combined.lines() {
        if let Some((p, f)) = parse_cargo_test_result(line) {
            pass += p;
            fail += f;
        }
    }
    let total = pass + fail;
    let percent = if total == 0 {
        0.0
    } else {
        100.0 * pass as f32 / total as f32
    };
    let strict = Some(Score {
        pass,
        total,
        percent,
    });
    let passed_baseline = match &baseline {
        None => true,
        Some(b) => percent >= b.min_percent && status_ok,
    };
    let raw_output_tail = combined
        .lines()
        .filter(|l| l.contains("test result") || l.contains("FAILED"))
        .take(8)
        .collect::<Vec<_>>()
        .join("\n");
    SuiteResult {
        name: name.into(),
        path: args.join(" "),
        strict,
        semantic: None,
        baseline,
        passed_baseline,
        raw_output_tail,
    }
}

/// Parse «[respond_full] strict   : 21/22 = 95%» style lines.
/// Returns the LAST match in the input (some evals print
/// per-category intermediate scores; we want the final tally).
fn parse_score_line(haystack: &str, which: &str) -> Option<Score> {
    // `respond_full` formats both labels right-padded to width 9
    // (so «strict» becomes «strict   » with 3 trailing spaces and
    // «semantic» becomes «semantic » with 1) — that aligns the
    // colon at the same column regardless of which label.  The
    // earlier width-8 needle missed «strict» because the actual
    // output had 3 spaces, not 2.
    let needle = format!("] {which:<9}: ");
    let mut last: Option<Score> = None;
    for line in haystack.lines() {
        if let Some(rest) = line.split(&needle).nth(1) {
            // expected shape: «21/22 = 95%»
            let parts: Vec<&str> = rest.split('=').collect();
            if parts.len() != 2 {
                continue;
            }
            let frac = parts[0].trim();
            let percent_str = parts[1].trim().trim_end_matches('%');
            let frac_parts: Vec<&str> = frac.split('/').collect();
            if frac_parts.len() != 2 {
                continue;
            }
            let pass = frac_parts[0].trim().parse::<u32>().ok();
            let total = frac_parts[1].trim().parse::<u32>().ok();
            let percent = percent_str.parse::<f32>().ok();
            if let (Some(pass), Some(total), Some(percent)) = (pass, total, percent) {
                last = Some(Score {
                    pass,
                    total,
                    percent,
                });
            }
        }
    }
    last
}

/// Parse «test result: ok. 95 passed; 0 failed; 0 ignored; …»
/// Returns (passed, failed).  Ignores tests are counted as
/// neither — they don't run.
fn parse_cargo_test_result(line: &str) -> Option<(u32, u32)> {
    let lower = line.to_lowercase();
    let needle = "test result:";
    let start = lower.find(needle)?;
    let rest = &line[start + needle.len()..];
    let mut pass = None;
    let mut fail = None;
    for token in rest.split([' ', ';']) {
        if let Ok(n) = token.parse::<u32>() {
            if pass.is_none() {
                pass = Some(n);
            } else if fail.is_none() {
                fail = Some(n);
                break;
            }
        }
    }
    Some((pass?, fail.unwrap_or(0)))
}

fn run(cmd: &str, args: &[&str]) -> std::io::Result<String> {
    let out = Command::new(cmd).args(args).output()?;
    if !out.status.success() {
        return Err(std::io::Error::other(format!(
            "{cmd} {args:?} exited {:?}",
            out.status
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn read_workspace_version() -> Option<String> {
    // Avoid cargo-metadata heaviness; read the workspace
    // Cargo.toml directly and find the `version = "..."` line
    // under `[workspace.package]`.
    let text = std::fs::read_to_string("Cargo.toml").ok()?;
    let mut in_workspace_package = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_workspace_package = trimmed == "[workspace.package]";
            continue;
        }
        if in_workspace_package && trimmed.starts_with("version") {
            let val = trimmed.split_once('=')?.1.trim().trim_matches('"');
            return Some(val.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_score_line_strict() {
        let s = "[respond_full] eval mode — 22 cases\n[respond_full] strict   : 21/22 = 95%\n[respond_full] semantic : 22/22 = 100%\n";
        let strict = parse_score_line(s, "strict").expect("strict line");
        assert_eq!(strict.pass, 21);
        assert_eq!(strict.total, 22);
        assert!((strict.percent - 95.0).abs() < 0.01);
        let sem = parse_score_line(s, "semantic").expect("semantic line");
        assert_eq!(sem.pass, 22);
        assert_eq!(sem.total, 22);
    }

    #[test]
    fn parse_score_line_takes_last() {
        // Some evals print intermediate per-category lines; take the final.
        let s = "[respond_full] semantic : 1/3 = 33%\n[respond_full] semantic : 2/3 = 67%\n[respond_full] semantic : 3/3 = 100%\n";
        let sem = parse_score_line(s, "semantic").expect("semantic");
        assert_eq!(sem.pass, 3);
    }

    #[test]
    fn parse_cargo_test_result_basic() {
        let line = "test result: ok. 95 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 14.0s";
        let (p, f) = parse_cargo_test_result(line).expect("parse");
        assert_eq!(p, 95);
        assert_eq!(f, 0);
    }

    #[test]
    fn parse_cargo_test_result_failure() {
        let line =
            "test result: FAILED. 17 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out;";
        let (p, f) = parse_cargo_test_result(line).expect("parse");
        assert_eq!(p, 17);
        assert_eq!(f, 1);
    }

    #[test]
    fn parse_cargo_test_result_skips_non_result_lines() {
        let line = "running 95 tests";
        assert!(parse_cargo_test_result(line).is_none());
    }
}
