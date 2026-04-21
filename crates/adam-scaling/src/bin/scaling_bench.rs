//! `scaling_bench` — produce `data/scaling/scaling_report.json` +
//! `docs/scaling_report.md` from the canonical corpus walk.
//!
//! ## Default tiers
//!
//! With committed-only corpus the bench runs `[100, 500, 1000, 0]` —
//! four sample-count targets with the last being "all available". With
//! `--use-shards`, shards augment the pool and bigger tiers become
//! meaningful: `[100, 1000, 10000, 50000, 200000]`.
//!
//! Override with `--tiers 100,500,2000,0` (comma-separated; `0` =
//! uncapped).
//!
//! ## Budget
//!
//! The v3.1.0 harness is honoured: pass `--time-budget <SEC>` or
//! `--time-budget-mins <MIN>` to cap the whole bench. On deadline,
//! whichever tiers ran to completion are preserved in the report;
//! unfinished tiers are omitted. The report's `status` mirrors the
//! v3.1.0 convention.
//!
//! Progress monitor: `--progress-interval <SEC>` (default 30). SIGINT
//! / SIGTERM trigger graceful commit the same way as `extract_facts`.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
    sync::Arc,
    time::{Duration, Instant},
};

use adam_kernel_fst::lexicon::LexiconV1;
use adam_reasoning::harness::{IterationBudget, ProgressCounter, ProgressMonitor, StopReason};
use adam_scaling::{
    CorpusPaths, MachineSignal, ScalingReport,
    bench::{load_corpus, render_markdown, run_tier},
};
use serde::Serialize;

const JSON_OUT: &str = "data/scaling/scaling_report.json";
const MD_OUT: &str = "docs/scaling_report.md";
const DEFAULT_PROGRESS_INTERVAL_SECS: u64 = 30;

/// Default tiers for committed-only corpus. Hand-picked to produce a
/// useful curve in ≲ 3 min on M2 8-core with Rayon: every tier an
/// order of magnitude bigger than the last, stopping well short of
/// the full ~400k samples (which is opt-in via `--tiers 0`). Each
/// tier is designed to be meaningful — the first tier must have
/// enough fact-dense material to emit a non-zero data point, the last
/// tier must fit in the default 3h budget.
const DEFAULT_TIERS_COMMITTED: &[(usize, &str)] = &[
    (100, "T1_100"),
    (1_000, "T2_1k"),
    (10_000, "T3_10k"),
    (50_000, "T4_50k"),
];

/// With `--use-shards` the corpus pool grows to the full 77.9 M-word
/// local corpus. The biggest tier here is sized so a parallel
/// extraction finishes within a 3h budget (at ~0.3 ms/word × 20 M
/// words ≈ 100 min; allowing headroom for graph + reasoner).
const DEFAULT_TIERS_WITH_SHARDS: &[(usize, &str)] = &[
    (1_000, "T1_1k"),
    (10_000, "T2_10k"),
    (50_000, "T3_50k"),
    (200_000, "T4_200k"),
    (1_000_000, "T5_1M"),
];

/// Full report extended with v3.1.0-style status fields. Written atop
/// the library's `ScalingReport` so the JSON has a consistent schema
/// even when the run aborted mid-way.
#[derive(Debug, Serialize)]
struct FullReport {
    status: String,
    elapsed_s: u64,
    tiers_completed: usize,
    tiers_planned: usize,
    #[serde(flatten)]
    report: ScalingReport,
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let use_shards = args.iter().any(|a| a == "--use-shards");
    let tiers = parse_tiers(&args).unwrap_or_else(|| {
        if use_shards {
            DEFAULT_TIERS_WITH_SHARDS
                .iter()
                .map(|(n, label)| ((*label).to_string(), *n))
                .collect()
        } else {
            DEFAULT_TIERS_COMMITTED
                .iter()
                .map(|(n, label)| ((*label).to_string(), *n))
                .collect()
        }
    });
    let progress_interval = Duration::from_secs(
        parse_usize_flag(&args, "--progress-interval")
            .map(|n| n as u64)
            .unwrap_or(DEFAULT_PROGRESS_INTERVAL_SECS),
    );

    let budget = IterationBudget::from_args(&args);
    budget.install_signal_handler();

    let paths = CorpusPaths::default();
    eprintln!(
        "scaling_bench: corpus root = {}, shards = {} ({}use-shards)",
        paths.committed_dir.display(),
        paths.shards_dir.display(),
        if use_shards { "" } else { "no " }
    );
    eprintln!(
        "scaling_bench: tiers = {}",
        tiers
            .iter()
            .map(|(l, n)| format!("{l}={n}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    if let Some(rem) = budget.remaining_secs() {
        eprintln!("scaling_bench: time budget = {rem}s, progress interval = {progress_interval:?}");
    } else {
        eprintln!(
            "scaling_bench: no time budget, progress interval = {progress_interval:?} (Ctrl-C → graceful commit)"
        );
    }

    let lexicon = match LexiconV1::load_default() {
        Ok(l) => l,
        Err(e) => {
            eprintln!("cannot load lexicon: {e:?}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!(
        "scaling_bench: lexicon loaded ({} curated + {} apertium)",
        lexicon.curated_count, lexicon.apertium_count
    );

    let (corpus, sources) = load_corpus(&paths, use_shards);
    eprintln!(
        "scaling_bench: corpus loaded — {} samples, {} words",
        sources.total_samples_available, sources.total_words_available,
    );

    // Filter tiers down to what's actually feasible given the corpus —
    // uncapped always runs; other targets clamp to corpus size. Labels
    // aren't rewritten, only the target value. A tier with
    // target_samples > corpus.len() still produces a point with
    // samples_scanned = corpus.len().
    let tiers_planned = tiers.len();
    let counters = ProgressCounter::new();
    let monitor = ProgressMonitor::spawn(
        budget.clone(),
        Arc::clone(&counters),
        progress_interval,
        "scaling_bench",
    );

    let started = Instant::now();
    let mut points = Vec::new();
    for (label, target) in &tiers {
        if budget.should_stop() {
            eprintln!(
                "scaling_bench: stopping before tier `{label}` — {} elapsed, deadline/interrupt hit",
                budget.elapsed_secs()
            );
            break;
        }
        eprintln!(
            "scaling_bench: tier `{label}` target={} — samples available {}",
            target,
            corpus.len()
        );
        let point = run_tier(label.clone(), *target, &corpus, &lexicon);
        eprintln!(
            "  → scanned={} words={} facts={} derivations={} nodes={} edges={} extract={}ms graph={}ms reason={}ms",
            point.samples_scanned,
            point.words_scanned,
            point.facts_extracted,
            point.derivations,
            point.graph_nodes,
            point.graph_edges,
            point.elapsed_ms.extract,
            point.elapsed_ms.graph,
            point.elapsed_ms.reason,
        );
        counters.add_items(point.facts_extracted);
        counters.add_extra(point.words_scanned);
        points.push(point);
    }
    let total_elapsed_s = started.elapsed().as_secs();

    let reason = budget.stop_reason();
    monitor.join();

    let report = ScalingReport {
        version: env!("CARGO_PKG_VERSION").to_string(),
        total_elapsed_s,
        machine: MachineSignal {
            rayon_threads: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1),
        },
        sources,
        tiers: points,
    };

    let tiers_completed = report.tiers.len();
    let full = FullReport {
        status: reason.as_str().to_string(),
        elapsed_s: budget.elapsed_secs(),
        tiers_completed,
        tiers_planned,
        report,
    };

    if reason != StopReason::Completed {
        eprintln!(
            "NOTE: bench ended as `{}` — {}/{} tiers completed",
            full.status, tiers_completed, tiers_planned
        );
    }

    if let Err(e) = write_json(Path::new(JSON_OUT), &full) {
        eprintln!("write {JSON_OUT}: {e}");
        return ExitCode::FAILURE;
    }
    if let Err(e) = write_markdown(Path::new(MD_OUT), &full) {
        eprintln!("write {MD_OUT}: {e}");
        return ExitCode::FAILURE;
    }
    eprintln!("wrote {JSON_OUT}");
    eprintln!("wrote {MD_OUT}");
    ExitCode::SUCCESS
}

fn parse_tiers(args: &[String]) -> Option<Vec<(String, usize)>> {
    let idx = args.iter().position(|a| a == "--tiers")?;
    let spec = args.get(idx + 1)?;
    let mut out = Vec::new();
    for (i, part) in spec.split(',').enumerate() {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        let n: usize = match trimmed.parse() {
            Ok(v) => v,
            Err(_) => {
                eprintln!("scaling_bench: bad tier value `{trimmed}`, skipping");
                continue;
            }
        };
        out.push((format!("T{}_{}", i + 1, n), n));
    }
    if out.is_empty() { None } else { Some(out) }
}

fn parse_usize_flag(args: &[String], name: &str) -> Option<usize> {
    let idx = args.iter().position(|a| a == name)?;
    args.get(idx + 1).and_then(|s| s.parse().ok())
}

fn write_json(path: &Path, full: &FullReport) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    let json = serde_json::to_string_pretty(full).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

fn write_markdown(path: &Path, full: &FullReport) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    let mut md = render_markdown(&full.report);
    md.push_str(&format!(
        "\n## Run metadata\n\n- status: `{}`\n- elapsed: {} s\n- tiers completed: {} / {}\n",
        full.status, full.elapsed_s, full.tiers_completed, full.tiers_planned,
    ));
    fs::write(path, md).map_err(|e| e.to_string())?;
    Ok(())
}

/// Silence unused-import warnings when `PathBuf` is only referenced
/// through re-exports — kept for consistency with sister binaries.
#[allow(dead_code)]
fn _touch(_: &PathBuf) {}
