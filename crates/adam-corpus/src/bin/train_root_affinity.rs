//! `train_root_affinity` — offline training pass that builds the
//! sparse root-pair co-occurrence prior shipped as
//! `data/retrieval/root_affinity.json`.
//!
//! **v4.29.0 — Track A first discourse-level statistical layer.**
//! Where `train_suffix_priors` operates over suffix chains
//! (within-word morphology), `train_root_affinity` operates over
//! roots (cross-word, cross-sentence semantic). For each curated
//! sample, we:
//!
//! 1. FST-analyse each token; extract the unique set of noun /
//!    verb roots that appear in the sample.
//! 2. Tally per-root counts (`single_counts[root] += 1`).
//! 3. Tally per-pair counts (`pair_counts[(a, b)] += 1` with `a <
//!    b` lex order to dedupe).
//! 4. After all samples, hand counts to
//!    [`adam_kernel_fst::root_affinity::RootAffinity::from_counts`]
//!    which computes PMI per pair, filters below `MIN_PAIR_COUNT`,
//!    and writes the sparse map.
//!
//! **Why uniform attribution per root.** A token with N FST parses
//! contributes its full +1 to *each* of its candidate roots'
//! single_count. Pre-fix this would over-count ambiguous tokens; in
//! practice the deduped per-sample set caps any one root at +1 per
//! sample regardless of how many times the token appears, so the
//! count is "sample contains root r" boolean, exactly what PMI
//! expects.
//!
//! **Performance.** The unique-token cache from
//! `train_suffix_priors` is reused: FST analyse runs once per
//! distinct surface form across the whole corpus (~340k forms after
//! v4.28.5), then samples are scanned to assemble per-sample root
//! sets. ~1-2 minutes end-to-end on M2 with the 8.85M-token corpus.
//!
//! **Output.** `data/retrieval/root_affinity.json` — schema v1
//! sparse PMI matrix. Pairs filtered by `MIN_PAIR_COUNT = 5` and
//! positive PMI.
//!
//! **CLI.**
//! ```text
//! cargo run --release -p adam-corpus --bin train_root_affinity
//! ```

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use adam_kernel_fst::lexicon::LexiconV1;
use adam_kernel_fst::parser::{Analysis, analyse};
use adam_kernel_fst::root_affinity::{RootAffinity, SCHEMA_VERSION};
use serde::Deserialize;

const CURATED_DIR: &str = "data/curated";
const OUTPUT_PATH: &str = "data/retrieval/root_affinity.json";

/// Minimum pair count threshold. Pairs observed fewer than this
/// many times across the corpus are filtered out as too noisy
/// (PMI on small counts is unreliable). 5 is empirical: at v4.28.5
/// 8.85M tokens / 1.08M samples / 9602 roots, this keeps the
/// strongest co-occurrence signal while suppressing one-off noise.
const MIN_PAIR_COUNT: u32 = 5;

#[derive(Debug, Deserialize)]
struct PackFile {
    #[serde(default)]
    samples: Vec<PackSample>,
}

#[derive(Debug, Deserialize)]
struct PackSample {
    #[serde(default)]
    text: Option<String>,
}

fn main() -> ExitCode {
    let curated = Path::new(CURATED_DIR);
    if !curated.is_dir() {
        eprintln!(
            "train_root_affinity: expected curated dir at {}; aborting",
            curated.display()
        );
        return ExitCode::FAILURE;
    }

    let lex = match LexiconV1::load_default() {
        Ok(l) => l,
        Err(err) => {
            eprintln!("train_root_affinity: lexicon load failed: {err:?}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!(
        "train_root_affinity: lexicon loaded ({} entries)",
        lex.total()
    );

    let pack_paths = match collect_pack_paths(curated) {
        Ok(p) => p,
        Err(err) => {
            eprintln!("train_root_affinity: failed to enumerate packs: {err}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!(
        "train_root_affinity: found {} curated packs (incl. shards)",
        pack_paths.len()
    );

    // Pass 1: collect all samples, tally unique tokens.
    let mut all_samples: Vec<Vec<String>> = Vec::new();
    let mut total_tokens: u64 = 0;
    let mut total_samples: u64 = 0;
    let mut unique_tokens: HashSet<String> = HashSet::new();

    for path in &pack_paths {
        let bytes = match fs::read(path) {
            Ok(b) => b,
            Err(err) => {
                eprintln!(
                    "train_root_affinity: skipping {} (read error: {err})",
                    path.display()
                );
                continue;
            }
        };
        let pack: PackFile = match serde_json::from_slice(&bytes) {
            Ok(p) => p,
            Err(err) => {
                eprintln!(
                    "train_root_affinity: skipping {} (schema mismatch: {err})",
                    path.display()
                );
                continue;
            }
        };
        for sample in &pack.samples {
            total_samples += 1;
            let Some(text) = sample.text.as_deref() else {
                continue;
            };
            let toks: Vec<String> = tokenize(text).collect();
            total_tokens += toks.len() as u64;
            for t in &toks {
                unique_tokens.insert(t.clone());
            }
            if !toks.is_empty() {
                all_samples.push(toks);
            }
        }
    }
    eprintln!(
        "train_root_affinity: {} samples, {} tokens, {} unique forms",
        total_samples,
        total_tokens,
        unique_tokens.len()
    );

    // Pass 2: FST-analyse each unique form once, cache the deduped
    // root list. A token whose FST returns no parses contributes
    // nothing.
    let mut roots_for: HashMap<String, Vec<String>> = HashMap::with_capacity(unique_tokens.len());
    let unique_total = unique_tokens.len() as u64;
    let mut analysed = 0u64;
    for token in &unique_tokens {
        analysed += 1;
        if analysed % 25_000 == 0 {
            eprintln!(
                "train_root_affinity: analysed {} / {} unique forms",
                analysed, unique_total
            );
        }
        let parses = analyse(token, &lex);
        if parses.is_empty() {
            roots_for.insert(token.clone(), Vec::new());
            continue;
        }
        let mut seen = HashSet::new();
        let mut roots: Vec<String> = Vec::new();
        for parse in &parses {
            let r = match parse {
                Analysis::Noun { root, .. } | Analysis::Verb { root, .. } => root.root.clone(),
            };
            if seen.insert(r.clone()) {
                roots.push(r);
            }
        }
        roots_for.insert(token.clone(), roots);
    }

    // Pass 3: walk samples, accumulate single + pair counts.
    let mut single_counts: HashMap<String, u64> = HashMap::new();
    let mut pair_counts: HashMap<(String, String), u64> = HashMap::new();

    for sample in &all_samples {
        // Build the deduped set of roots for this sample by union
        // of each token's candidate roots. Note: ambiguous tokens
        // contribute *all* their candidate roots to the sample's
        // root set — this is consistent with PMI's "did pair
        // co-occur" view and over-counts only on truly ambiguous
        // tokens (which is mild at PMI granularity).
        let mut sample_roots: HashSet<String> = HashSet::new();
        for tok in sample {
            if let Some(roots) = roots_for.get(tok) {
                for r in roots {
                    sample_roots.insert(r.clone());
                }
            }
        }
        // Single counts.
        for r in &sample_roots {
            *single_counts.entry(r.clone()).or_insert(0) += 1;
        }
        // Pair counts (lex-sorted-smaller first).
        let sorted: Vec<&String> = {
            let mut v: Vec<&String> = sample_roots.iter().collect();
            v.sort();
            v
        };
        for (i, a) in sorted.iter().enumerate() {
            for b in sorted.iter().skip(i + 1) {
                *pair_counts.entry(((*a).clone(), (*b).clone())).or_insert(0) += 1;
            }
        }
    }
    eprintln!(
        "train_root_affinity: collected {} distinct roots; {} distinct pairs (pre-filter)",
        single_counts.len(),
        pair_counts.len()
    );

    let aff = RootAffinity::from_counts(total_samples, single_counts, pair_counts, MIN_PAIR_COUNT);

    eprintln!(
        "train_root_affinity: post-filter ({} pair-count, positive PMI): {} pairs across {} outer roots",
        MIN_PAIR_COUNT,
        aff.pair_count(),
        aff.pair_pmi.len()
    );

    if let Err(err) = write_affinity(&aff) {
        eprintln!("train_root_affinity: failed to write artifact: {err}");
        return ExitCode::FAILURE;
    }
    eprintln!(
        "train_root_affinity: wrote {} (schema v{}, {} pairs, {} roots)",
        OUTPUT_PATH,
        SCHEMA_VERSION,
        aff.pair_count(),
        aff.root_count(),
    );
    ExitCode::SUCCESS
}

fn collect_pack_paths(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path
            .extension()
            .and_then(|s| s.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("json"))
            .unwrap_or(false)
        {
            out.push(path);
        }
    }
    // **v4.28.5 parity** — also scan filtered shards.
    let shards_dir = dir.join("shards");
    if shards_dir.is_dir() {
        for entry in fs::read_dir(&shards_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("filtered_") && n.ends_with(".json"))
                .unwrap_or(false)
            {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

fn tokenize(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split_whitespace().filter_map(|tok| {
        let cleaned: String = tok
            .chars()
            .filter(|c| c.is_alphabetic() || *c == '-')
            .collect::<String>()
            .to_lowercase();
        if cleaned.is_empty() {
            None
        } else {
            Some(cleaned)
        }
    })
}

/// Write the affinity as pretty-printed JSON. Sort keys at both
/// levels for byte-stable reproducibility (HashMap iteration is
/// otherwise unstable across runs).
fn write_affinity(aff: &RootAffinity) -> std::io::Result<()> {
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"version\": {},\n", aff.version));
    json.push_str(&format!(
        "  \"trained_on_samples\": {},\n",
        aff.trained_on_samples
    ));
    json.push_str(&format!("  \"min_pair_count\": {},\n", aff.min_pair_count));

    // root_log_prob, sorted.
    let mut singles: Vec<(&String, &f32)> = aff.root_log_prob.iter().collect();
    singles.sort_by(|a, b| a.0.cmp(b.0));
    json.push_str("  \"root_log_prob\": {\n");
    for (i, (k, v)) in singles.iter().enumerate() {
        let comma = if i + 1 == singles.len() { "" } else { "," };
        let escaped = k.replace('\\', "\\\\").replace('"', "\\\"");
        json.push_str(&format!("    \"{escaped}\": {v}{comma}\n"));
    }
    json.push_str("  },\n");

    // pair_pmi, nested sorted.
    let mut outer_sorted: Vec<(&String, &HashMap<String, f32>)> = aff.pair_pmi.iter().collect();
    outer_sorted.sort_by(|a, b| a.0.cmp(b.0));
    json.push_str("  \"pair_pmi\": {\n");
    for (oi, (outer, row)) in outer_sorted.iter().enumerate() {
        let outer_comma = if oi + 1 == outer_sorted.len() {
            ""
        } else {
            ","
        };
        let outer_esc = outer.replace('\\', "\\\\").replace('"', "\\\"");
        json.push_str(&format!("    \"{outer_esc}\": {{\n"));
        let mut inner_sorted: Vec<(&String, &f32)> = row.iter().collect();
        inner_sorted.sort_by(|a, b| a.0.cmp(b.0));
        for (ii, (inner, score)) in inner_sorted.iter().enumerate() {
            let inner_comma = if ii + 1 == inner_sorted.len() {
                ""
            } else {
                ","
            };
            let inner_esc = inner.replace('\\', "\\\\").replace('"', "\\\"");
            json.push_str(&format!("      \"{inner_esc}\": {score}{inner_comma}\n"));
        }
        json.push_str(&format!("    }}{outer_comma}\n"));
    }
    json.push_str("  }\n");
    json.push_str("}\n");

    fs::write(OUTPUT_PATH, json)
}
