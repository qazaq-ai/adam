//! `extract_facts` — walk committed corpus packs, run every pattern
//! matcher on each sample, and emit the structured facts into
//! `data/retrieval/facts.json`.
//!
//! ## v3.1.0 iteration harness
//!
//! This binary honours the v3.1.0 iteration contract:
//!
//!   - `--time-budget <SEC>` / `--time-budget-mins <MIN>` — hard cap.
//!     On deadline hit, the binary commits a partial artifact with
//!     `status: "timed_out"` and exits 0.
//!   - `--progress-interval <SEC>` (default 30) — a background monitor
//!     thread prints `[hh:mm:ss] extract_facts samples=N items=M
//!     extra=W elapsed=S` every interval.
//!   - SIGINT / SIGTERM → same as budget, but `status: "interrupted"`.
//!   - Pack walk is sequential; per-pack sample matchers run through
//!     `rayon::par_iter` so FST parsing saturates the M2's 8 cores.
//!     Output order is preserved (rayon `map→collect` keeps input
//!     order) so artifacts stay byte-identical between runs.
//!
//! ## Output artifact schema (v3.1.0 fields marked)
//!
//! ```json
//! {
//!   "version": "<crate version>",
//!   "status": "completed" | "timed_out" | "interrupted",   // v3.1.0
//!   "elapsed_s": 12345,                                     // v3.1.0
//!   "packs_completed": 6,                                   // v3.1.0
//!   "packs_total": 8,                                       // v3.1.0
//!   "built_from": ["wikipedia_kz_pack.json", ...],
//!   "counts": {
//!     "samples_scanned": 3191,
//!     "samples_with_facts": 1234,
//!     "facts_total": 2345,
//!     "by_predicate": { "is_a": 1800, ... },
//!     "by_pack": { "wikipedia_kz_pack.json": 1500, ... }
//!   },
//!   "facts": [ Fact, ... ]
//! }
//! ```
//!
//! Downstream binaries (`build_lexical_graph`, `run_reasoner`) treat
//! any `status` value as first-class — a partial `facts.json` is still
//! a valid `facts.json`, just smaller.

use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
    sync::Arc,
    time::Duration,
};

use adam_kernel_fst::lexicon::LexiconV1;
use adam_reasoning::{
    Fact, FactSource, extract_facts,
    harness::{IterationBudget, ProgressCounter, ProgressMonitor, StopReason},
};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

const CURATED_DIR: &str = "data/curated";
const COMMITTED_OUTPUT: &str = "data/retrieval/facts.json";
const FULL_OUTPUT: &str = "data/retrieval/facts_full.json";
const COMMITTED_DEFAULT_LIMIT: usize = 500;
const DEFAULT_PROGRESS_INTERVAL_SECS: u64 = 30;

/// Chunk size for rayon — larger chunks amortise the monitor-thread
/// atomic reads; smaller chunks give finer budget-check granularity.
/// 128 samples × ~0.3ms/word × ~15 words/sample ≈ 0.6s/chunk — a good
/// middle ground.
const CHUNK_SIZE: usize = 128;

/// Same canonical pack list as `corpus_audit` / `build_morpheme_index`.
/// Kept in sync manually — a future consolidation lives in v2.x.
/// v3.3.0 added `kazakh_textbooks_pack.json` — Kazakh secondary-school
/// textbooks (grades 7–11) OCR'd via tesseract-kaz from MES-published
/// PDFs. Silently skipped if the pack is absent (shipped opt-in so
/// CI checkouts without the textbook corpus still pass).
const SOURCE_PACKS: &[&str] = &[
    "tatoeba_kazakh_pack.json",
    "wikipedia_kz_pack.json",
    "common_voice_kk_pack.json",
    "cc100_kk_pack.json",
    "abai_wikisource_pack.json",
    "kazakh_proverbs_pack.json",
    "synthetic_sentences_pack.json",
    "kazakh_classics_pack.json",
    "kazakh_textbooks_pack.json",
];

#[derive(Debug, Deserialize)]
struct PackFile {
    samples: Vec<Sample>,
}

#[derive(Debug, Deserialize)]
struct Sample {
    id: String,
    text: String,
}

#[derive(Debug, Serialize)]
struct Counts {
    samples_scanned: usize,
    samples_with_facts: usize,
    facts_total: usize,
    by_predicate: BTreeMap<String, usize>,
    by_pack: BTreeMap<String, usize>,
}

#[derive(Debug, Serialize)]
struct Artifact {
    version: String,
    /// v3.1.0 — `"completed"` | `"timed_out"` | `"interrupted"`.
    status: String,
    /// v3.1.0 — wall-clock seconds from process start to commit.
    elapsed_s: u64,
    /// v3.1.0 — packs fully scanned (not counting a partial pack).
    packs_completed: usize,
    /// v3.1.0 — total packs the run attempted.
    packs_total: usize,
    built_from: Vec<String>,
    counts: Counts,
    facts: Vec<Fact>,
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let full_mode = args.iter().any(|a| a == "--full");
    let limit: Option<usize> = if full_mode {
        None
    } else {
        Some(parse_usize_flag(&args, "--limit").unwrap_or(COMMITTED_DEFAULT_LIMIT))
    };
    let output_path = if full_mode {
        FULL_OUTPUT
    } else {
        COMMITTED_OUTPUT
    };
    let progress_interval = Duration::from_secs(
        parse_usize_flag(&args, "--progress-interval")
            .map(|n| n as u64)
            .unwrap_or(DEFAULT_PROGRESS_INTERVAL_SECS),
    );

    let budget = IterationBudget::from_args(&args);
    budget.install_signal_handler();

    let lexicon = match LexiconV1::load_default() {
        Ok(l) => l,
        Err(e) => {
            eprintln!("cannot load lexicon: {e:?}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!(
        "extract_facts: lexicon loaded ({} curated + {} apertium entries)",
        lexicon.curated_count, lexicon.apertium_count
    );
    eprintln!(
        "extract_facts: mode = {}, output = {output_path}",
        if full_mode {
            "FULL (every sample, gitignored)".to_string()
        } else {
            format!("committed (first {} per pack)", limit.unwrap())
        }
    );
    if let Some(rem) = budget.remaining_secs() {
        eprintln!("extract_facts: time budget = {rem}s, progress interval = {progress_interval:?}");
    } else {
        eprintln!(
            "extract_facts: no time budget, progress interval = {progress_interval:?} (Ctrl-C → graceful commit)"
        );
    }

    let counters = ProgressCounter::new();
    let monitor = ProgressMonitor::spawn(
        budget.clone(),
        Arc::clone(&counters),
        progress_interval,
        "extract_facts",
    );

    let packs_total = SOURCE_PACKS.len();
    let mut artifact = Artifact {
        version: env!("CARGO_PKG_VERSION").to_string(),
        status: StopReason::Completed.as_str().to_string(),
        elapsed_s: 0,
        packs_completed: 0,
        packs_total,
        built_from: Vec::new(),
        counts: Counts {
            samples_scanned: 0,
            samples_with_facts: 0,
            facts_total: 0,
            by_predicate: BTreeMap::new(),
            by_pack: BTreeMap::new(),
        },
        facts: Vec::new(),
    };

    for pack_name in SOURCE_PACKS {
        if budget.should_stop() {
            break;
        }
        let path = Path::new(CURATED_DIR).join(pack_name);
        if !path.exists() {
            eprintln!("skipping missing: {}", path.display());
            continue;
        }
        artifact.built_from.push((*pack_name).to_string());
        eprintln!("scanning {} ...", path.display());
        let pack = match load_pack(&path) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("cannot load {}: {e}", path.display());
                // Partial commit is preferable to hard-fail on a
                // single malformed pack — flag and move on.
                continue;
            }
        };

        let effective_limit = limit.unwrap_or(pack.samples.len()).min(pack.samples.len());
        let samples_slice = &pack.samples[..effective_limit];

        // Parallel chunked scan. Each chunk becomes a flat Vec of
        // facts, preserving input order on collect. Budget is checked
        // between chunks so SIGINT / timeout gets caught within ~1s
        // on an 8-core M2.
        let chunks: Vec<&[Sample]> = samples_slice.chunks(CHUNK_SIZE).collect();
        let lex_ref = &lexicon;
        let pack_name_owned = (*pack_name).to_string();
        let budget_ref = &budget;
        let counters_ref = &counters;

        let mut pack_stopped_early = false;
        for chunk in chunks {
            if budget_ref.should_stop() {
                pack_stopped_early = true;
                break;
            }
            let produced: Vec<Fact> = chunk
                .par_iter()
                .flat_map_iter(|sample| {
                    let source = FactSource {
                        pack: pack_name_owned.clone(),
                        sample_id: sample.id.clone(),
                    };
                    let facts = extract_facts(&sample.text, &[], lex_ref, &source);
                    counters_ref.inc_sample();
                    counters_ref.add_extra(
                        sample
                            .text
                            .split_whitespace()
                            .count()
                            .try_into()
                            .unwrap_or(0),
                    );
                    facts.into_iter()
                })
                .collect();

            if !produced.is_empty() {
                counters_ref.add_items(produced.len());
                let mut seen_samples_with_facts = std::collections::BTreeSet::<String>::new();
                for f in &produced {
                    let pred = f.predicate.as_str().to_string();
                    *artifact.counts.by_predicate.entry(pred).or_insert(0) += 1;
                    *artifact
                        .counts
                        .by_pack
                        .entry(pack_name_owned.clone())
                        .or_insert(0) += 1;
                    seen_samples_with_facts.insert(f.source.sample_id.clone());
                }
                artifact.counts.samples_with_facts += seen_samples_with_facts.len();
                artifact.counts.facts_total += produced.len();
                artifact.facts.extend(produced);
            }
            artifact.counts.samples_scanned += chunk.len();
        }

        if !pack_stopped_early {
            artifact.packs_completed += 1;
        } else {
            // Pack didn't finish — mark the run and stop the outer
            // loop on the next iteration's budget check.
            break;
        }
    }

    let reason = budget.stop_reason();
    artifact.status = reason.as_str().to_string();
    artifact.elapsed_s = budget.elapsed_secs();

    monitor.join();

    eprintln!(
        "DONE: status={} scanned={} samples_with_facts={} facts={} packs={}/{} elapsed={}s",
        artifact.status,
        artifact.counts.samples_scanned,
        artifact.counts.samples_with_facts,
        artifact.counts.facts_total,
        artifact.packs_completed,
        artifact.packs_total,
        artifact.elapsed_s,
    );
    for (pred, n) in &artifact.counts.by_predicate {
        eprintln!("  predicate {pred}: {n}");
    }

    if let Some(parent) = Path::new(output_path).parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                eprintln!("cannot create {}: {e}", parent.display());
                return ExitCode::FAILURE;
            }
        }
    }
    let json = match serde_json::to_string_pretty(&artifact) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("serialise: {e}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = fs::write(output_path, json) {
        eprintln!("write {output_path}: {e}");
        return ExitCode::FAILURE;
    }
    eprintln!("wrote {output_path}");
    ExitCode::SUCCESS
}

fn parse_usize_flag(args: &[String], name: &str) -> Option<usize> {
    let idx = args.iter().position(|a| a == name)?;
    args.get(idx + 1).and_then(|s| s.parse().ok())
}

fn load_pack(path: &PathBuf) -> Result<PackFile, String> {
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}
