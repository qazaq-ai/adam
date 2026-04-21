//! `extract_facts` — walk committed corpus packs, run every v2.1
//! pattern matcher on each sample, and emit the structured facts into
//! `data/retrieval/facts.json`.
//!
//! Output artifact schema:
//!
//! ```json
//! {
//!   "version": "<crate version>",
//!   "built_from": ["wikipedia_kz_pack.json", "abai_wikisource_pack.json", ...],
//!   "counts": {
//!     "samples_scanned": 3191,
//!     "samples_with_facts": 1234,
//!     "facts_total": 2345,
//!     "by_predicate": { "is_a": 1800, "lives_in": 545 },
//!     "by_pack": { "wikipedia_kz_pack.json": 1500, ... }
//!   },
//!   "facts": [ Fact, Fact, ... ]
//! }
//! ```
//!
//! Determinism: samples are scanned in pack order, then `samples[]`
//! order inside each pack. Each pattern matcher appends facts in its
//! own deterministic order. So the output JSON is byte-identical
//! across runs on the same `data/curated/*_pack.json` set.

use std::{
    collections::BTreeMap,
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::ExitCode,
    time::Instant,
};

use adam_kernel_fst::{lexicon::LexiconV1, parser::analyse};
use adam_reasoning::{Fact, FactSource, extract_facts};
use serde::{Deserialize, Serialize};

const CURATED_DIR: &str = "data/curated";
const COMMITTED_OUTPUT: &str = "data/retrieval/facts.json";
const FULL_OUTPUT: &str = "data/retrieval/facts_full.json";
const COMMITTED_DEFAULT_LIMIT: usize = 500;

/// Same canonical pack list as `corpus_audit` / `build_morpheme_index`.
/// Kept in sync manually — a future consolidation lives in v2.x.
const SOURCE_PACKS: &[&str] = &[
    "tatoeba_kazakh_pack.json",
    "wikipedia_kz_pack.json",
    "common_voice_kk_pack.json",
    "cc100_kk_pack.json",
    "abai_wikisource_pack.json",
    "kazakh_proverbs_pack.json",
    "synthetic_sentences_pack.json",
    "kazakh_classics_pack.json",
];

const PROGRESS_EVERY: usize = 1_000;

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
    built_from: Vec<String>,
    counts: Counts,
    facts: Vec<Fact>,
}

fn main() -> ExitCode {
    // v2.1 mode selection — mirrors build_morpheme_index / process_cc100_kk:
    //   default      — first 500 samples per pack, writes committed artifact
    //   --full       — every sample, writes gitignored artifact
    //   --limit N    — override the per-pack cap (ignored in --full mode)
    let args: Vec<String> = env::args().collect();
    let full_mode = args.iter().any(|a| a == "--full");
    let limit: Option<usize> = if full_mode {
        None
    } else {
        Some(parse_flag(&args, "--limit").unwrap_or(COMMITTED_DEFAULT_LIMIT))
    };
    let output_path = if full_mode {
        FULL_OUTPUT
    } else {
        COMMITTED_OUTPUT
    };

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

    let mut artifact = Artifact {
        version: env!("CARGO_PKG_VERSION").to_string(),
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

    let started = Instant::now();

    for pack_name in SOURCE_PACKS {
        let path = Path::new(CURATED_DIR).join(pack_name);
        if !path.exists() {
            eprintln!("skipping missing: {}", path.display());
            continue;
        }
        artifact.built_from.push(pack_name.to_string());
        eprintln!("scanning {} ...", path.display());
        let pack = match load_pack(&path) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("cannot load {}: {e}", path.display());
                return ExitCode::FAILURE;
            }
        };

        // Per-sample cache — a word that appears twice in the same
        // sentence parses once. Scoped per sample so the memory doesn't
        // grow across the whole corpus.
        for (i, sample) in pack.samples.iter().enumerate() {
            if let Some(n) = limit {
                if i >= n {
                    break;
                }
            }
            artifact.counts.samples_scanned += 1;

            // Build a sentence-level parse list for v2.1 — currently
            // only used as a sanity hook; patterns run their own
            // per-token `analyse` to stay self-contained.
            let parses = Vec::new();

            let source = FactSource {
                pack: pack_name.to_string(),
                sample_id: sample.id.clone(),
            };
            let facts = extract_facts(&sample.text, &parses, &lexicon, &source);
            if !facts.is_empty() {
                artifact.counts.samples_with_facts += 1;
                for f in &facts {
                    let pred = f.predicate.as_str().to_string();
                    *artifact.counts.by_predicate.entry(pred).or_insert(0) += 1;
                    *artifact
                        .counts
                        .by_pack
                        .entry(pack_name.to_string())
                        .or_insert(0) += 1;
                }
                artifact.counts.facts_total += facts.len();
                artifact.facts.extend(facts);
            }

            if (i + 1) % PROGRESS_EVERY == 0 {
                eprintln!(
                    "  progress: pack_samples={} facts_so_far={} elapsed={:.1}s",
                    i + 1,
                    artifact.counts.facts_total,
                    started.elapsed().as_secs_f64(),
                );
                let _ = std::io::stderr().flush();
            }
        }
    }

    // Unused right now, kept so the binary can grow a second parse
    // path without re-shaping main().
    let _ = analyse; // silence unused-import lint

    eprintln!(
        "DONE: scanned {} samples, {} with facts, {} facts total, elapsed {:.1}s",
        artifact.counts.samples_scanned,
        artifact.counts.samples_with_facts,
        artifact.counts.facts_total,
        started.elapsed().as_secs_f64()
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

fn parse_flag(args: &[String], name: &str) -> Option<usize> {
    let idx = args.iter().position(|a| a == name)?;
    args.get(idx + 1).and_then(|s| s.parse().ok())
}

fn load_pack(path: &PathBuf) -> Result<PackFile, String> {
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}
