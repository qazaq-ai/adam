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
//! 1. Packs are iterated in a **bench-specific** canonical order
//!    (see [`CANONICAL_COMMITTED_PACKS`] — fact-dense packs first,
//!    conversational packs last). This is **not** the same order
//!    `extract_facts` uses; see the [`CANONICAL_COMMITTED_PACKS`]
//!    docstring for why the two orders diverge.
//! 2. Shard files (`--use-shards`) come after committed packs, in
//!    lexical filename order ([`SHARD_PACK_PREFIXES`]).
//! 3. Samples inside each pack are iterated in `samples[]` array order.
//! 4. Extraction calls `adam_reasoning::extract_facts` which is pure.
//! 5. Graph projection and reasoner are pure.
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
    "kazakh_textbooks_pack.json",
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
    /// v3.3.0 — normalized quality signals. Unlike raw counts these
    /// compare meaningfully across tiers and across releases. Computed
    /// from the raw fields above; always present. See
    /// [`NormalizedMetrics`] for the formulas.
    #[serde(default)]
    pub normalized: NormalizedMetrics,
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

/// Normalized quality signals for a [`ScalingPoint`]. Raw counts grow
/// with corpus size; these ratios tell you *what kind* of growth it is.
///
/// ## Interpretation
///
/// - **`facts_per_10k_words`** — extraction density. A higher number
///   means the matchers are finding more grammar-indexed structure per
///   unit of text. Expected to stabilise or slowly drift up as more
///   matchers ship; a sharp drop is a regression signal.
/// - **`derivations_per_fact`** — reasoning leverage. Each fact in the
///   graph produces on average this many rule-derived conclusions. As
///   the graph densifies, this ratio grows super-linearly — the
///   reasoning scaling law.
/// - **`predicate_coverage_pct`** — breadth across predicate types.
///   `(predicates with ≥ 1 fact) / (total predicate variants available)
///   × 100`. At v3.3.0 total = 6 (IsA, LivesIn, Has, GoesTo, PartOf,
///   RelatedTo). 100 % means every extractor is firing at this scale.
/// - **`duplicate_fact_rate_pct`** — hygiene signal. Two facts are
///   considered duplicates when their `(subject.root, predicate,
///   object.root)` triple coincides. Low is good (different samples
///   produce distinct claims); high signals template-like corpus or
///   over-confident matchers.
///
/// All fields are floats rendered to 2-4 decimals in the Markdown
/// report. A tier with zero facts has all-zero ratios (not NaN) —
/// this keeps JSON clean and plots linearisable.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct NormalizedMetrics {
    pub facts_per_10k_words: f64,
    pub derivations_per_fact: f64,
    pub predicate_coverage_pct: f64,
    pub duplicate_fact_rate_pct: f64,
}

impl Eq for NormalizedMetrics {}

/// Total number of [`adam_reasoning::Predicate`] variants the v3.5.0
/// codebase defines. Used as the denominator for
/// [`NormalizedMetrics::predicate_coverage_pct`]. Update this when the
/// `Predicate` enum grows.
///
/// v3.5.0 grew this from 6 to 11 (+Causes, +After, +HasQuantity,
/// +DoesTo, +InDomain).
pub const TOTAL_PREDICATE_VARIANTS: usize = 11;

impl NormalizedMetrics {
    /// Derive the normalized signals from a [`ScalingPoint`]'s raw
    /// counts + the duplicate count the caller computed (it needs the
    /// underlying fact list, not just the counts, so we can't derive
    /// it here).
    pub fn compute(
        facts_extracted: usize,
        words_scanned: u64,
        derivations: usize,
        distinct_predicates_fired: usize,
        duplicate_fact_count: usize,
    ) -> Self {
        let facts = facts_extracted as f64;
        let words = words_scanned as f64;
        Self {
            facts_per_10k_words: if words > 0.0 {
                (facts / words) * 10_000.0
            } else {
                0.0
            },
            derivations_per_fact: if facts > 0.0 {
                derivations as f64 / facts
            } else {
                0.0
            },
            predicate_coverage_pct: (distinct_predicates_fired as f64
                / TOTAL_PREDICATE_VARIANTS as f64)
                * 100.0,
            duplicate_fact_rate_pct: if facts > 0.0 {
                (duplicate_fact_count as f64 / facts) * 100.0
            } else {
                0.0
            },
        }
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
