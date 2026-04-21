//! adam-scaling — deterministic scaling-law bench.
//!
//! Answers the question a neural-era investor would ask as
//! "where's your perplexity-vs-FLOPS curve?" in this project's terms:
//! **given N input words, how many facts do we extract, how many rule
//! derivations fire, how dense is the lexical graph, and how long does
//! it take on this hardware?**
//!
//! Unlike transformer scaling laws, every number here is measured on a
//! fully deterministic pipeline — same corpus slice + same Lexicon +
//! same matchers → byte-identical artifacts + identical metrics across
//! runs (wall-clock drifts; everything else is fixed).
//!
//! ## What a tier is
//!
//! A "tier" is one data point on the curve: `target_samples` → observed
//! counts. The bench walks through the canonical pack order (same list
//! [`extract_facts`] uses), accumulating samples until it hits the
//! target. It optionally opts into the `data/curated/shards/` directory
//! for tiers that need more than the committed ~3 000 samples.
//!
//! ## Determinism contract
//!
//! 1. Packs are iterated in a fixed canonical order
//!    ([`CANONICAL_COMMITTED_PACKS`] then
//!    [`SHARD_PACK_PREFIXES`]'s shard files in lexical order).
//! 2. Samples inside each pack are iterated in `samples[]` array order.
//! 3. Extraction calls `adam_reasoning::extract_facts` which is pure.
//! 4. Graph projection and reasoner are pure.
//!
//! Consequently `scaling_report.json` is byte-identical across runs on
//! the same `(data/curated, data/curated/shards, Lexicon)` state,
//! except for the `elapsed_ms` fields which are wall-clock.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub mod bench;

/// Bench-specific pack order. **NOT** the same as extract_facts — the
/// bench walks fact-dense packs (Abai, proverbs, classics, Wikipedia,
/// synthetic) first so every tier, even `T1_100`, contains enough
/// matchable content to produce a non-zero data point. Purely
/// conversational packs (Tatoeba, Common Voice, CC-100) tail the list;
/// they contribute 0-few facts per 100 samples in the current matcher
/// set but are still useful for the bigger tiers where volume
/// dominates.
///
/// Order is load-bearing for determinism — the scaling bench walks
/// this list top-to-bottom to fill each tier deterministically.
pub const CANONICAL_COMMITTED_PACKS: &[&str] = &[
    "abai_wikisource_pack.json",
    "kazakh_proverbs_pack.json",
    "kazakh_classics_pack.json",
    "wikipedia_kz_pack.json",
    "synthetic_sentences_pack.json",
    "tatoeba_kazakh_pack.json",
    "common_voice_kk_pack.json",
    "cc100_kk_pack.json",
];

/// Which shard prefixes we opt into when `--use-shards` is set. Shards
/// are locally rebuilt via `build_morpheme_index --full` (or cc100 /
/// wikipedia stream jobs) and live in `data/curated/shards/`. They are
/// gitignored — the bench silently skips if the directory is empty.
pub const SHARD_PACK_PREFIXES: &[&str] = &["wikipedia_kz_shard_", "cc100_kk_shard_"];

/// Paths used by the default bench configuration. Kept in a struct so
/// tests can inject a mock corpus root.
#[derive(Debug, Clone)]
pub struct CorpusPaths {
    pub committed_dir: PathBuf,
    pub shards_dir: PathBuf,
}

impl Default for CorpusPaths {
    fn default() -> Self {
        Self {
            committed_dir: PathBuf::from("data/curated"),
            shards_dir: PathBuf::from("data/curated/shards"),
        }
    }
}

/// One (corpus_size → metrics) data point on the scaling curve.
///
/// `target_samples = 0` means "all available samples" (no cap).
/// `samples_scanned` may be less than `target_samples` if the available
/// corpus is smaller — the report caller must trust the scanned count,
/// not the target, when plotting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScalingPoint {
    /// Human label for the tier (e.g. `"T1_500"`, `"T5_full_committed"`).
    pub label: String,
    /// Sample-count ceiling for this tier. `0` = uncapped.
    pub target_samples: usize,
    /// How many samples the bench actually consumed from the canonical
    /// pack walk. `≤ target_samples` unless `target_samples == 0`.
    pub samples_scanned: usize,
    /// Total whitespace-token count across every consumed sample. The
    /// X-axis on the scaling curve.
    pub words_scanned: u64,
    /// Extracted facts (output of every pattern matcher summed).
    pub facts_extracted: usize,
    /// Breakdown by predicate string (from `Predicate::as_str`).
    pub facts_by_predicate: std::collections::BTreeMap<String, usize>,
    /// Rule-derived facts (output of the forward-chaining reasoner at
    /// fixpoint).
    pub derivations: usize,
    /// Breakdown by rule id (e.g. `R5_shared_is_a_target`).
    pub derivations_by_rule: std::collections::BTreeMap<String, usize>,
    /// Lexical graph node count after projection.
    pub graph_nodes: usize,
    /// Lexical graph edge count after projection.
    pub graph_edges: usize,
    /// Per-stage wall-clock in milliseconds. Only wall-clock metric —
    /// everything else is deterministic.
    pub elapsed_ms: StageMs,
}

/// Wall-clock per pipeline stage.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageMs {
    pub extract: u64,
    pub graph: u64,
    pub reason: u64,
}

impl StageMs {
    pub fn total(&self) -> u64 {
        self.extract + self.graph + self.reason
    }
}

/// The report artifact written to `data/scaling/scaling_report.json`.
/// Alongside this the bench writes `docs/scaling_report.md` — a
/// human-readable projection of the same data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScalingReport {
    pub version: String,
    /// Wall-clock for the whole bench run (all tiers + IO), seconds.
    pub total_elapsed_s: u64,
    /// Machine signal — `rayon_threads` tells the reader whether the
    /// `extract` timings should be compared against other single- or
    /// multi-thread runs. Not a full fingerprint.
    pub machine: MachineSignal,
    /// Source layout the bench walked.
    pub sources: SourcesSnapshot,
    pub tiers: Vec<ScalingPoint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineSignal {
    pub rayon_threads: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourcesSnapshot {
    /// Committed packs actually loaded (missing packs are skipped +
    /// not listed here).
    pub committed_packs_loaded: Vec<String>,
    /// Shard files loaded. Empty if `--use-shards` was off or the
    /// directory was absent.
    pub shard_packs_loaded: Vec<String>,
    /// Total samples available to the bench after the load.
    pub total_samples_available: usize,
    /// Total whitespace-token count across every loaded sample.
    pub total_words_available: u64,
}
