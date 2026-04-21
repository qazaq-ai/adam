//! Pure bench logic — isolated so unit tests can exercise the tier
//! driver without spawning a binary.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use adam_kernel_fst::lexicon::LexiconV1;
use adam_reasoning::{
    Fact, FactSource, extract_facts, graph::LexicalGraph, harness::IterationBudget, reasoner,
};
use rayon::prelude::*;
use serde::Deserialize;

use crate::{
    CANONICAL_COMMITTED_PACKS, CorpusPaths, MachineSignal, SHARD_PACK_PREFIXES, ScalingPoint,
    ScalingReport, SourcesSnapshot, StageMs,
};

/// One loaded sample, together with the pack it came from. We keep
/// the pack label so FactSource provenance stays correct even when the
/// sample was pulled from a shard file with a synthetic id.
#[derive(Debug, Clone)]
pub struct LoadedSample {
    pub pack_label: String,
    pub sample_id: String,
    pub text: String,
}

#[derive(Debug, Deserialize)]
struct PackFile {
    samples: Vec<PackSample>,
}

#[derive(Debug, Deserialize)]
struct PackSample {
    id: String,
    text: String,
}

/// Load the corpus in canonical order — committed packs first, then
/// (if enabled) shard files in lexical filename order. Returns a flat
/// Vec the bench driver slices per tier.
///
/// Missing committed packs are silently skipped (CI checks out a
/// reduced tree sometimes). Missing `shards_dir` is also silent.
/// Malformed pack files are reported on stderr and skipped.
pub fn load_corpus(paths: &CorpusPaths, use_shards: bool) -> (Vec<LoadedSample>, SourcesSnapshot) {
    let mut all = Vec::new();
    let mut committed_loaded = Vec::new();
    let mut shards_loaded = Vec::new();

    for &pack_name in CANONICAL_COMMITTED_PACKS {
        let path = paths.committed_dir.join(pack_name);
        match read_pack(&path) {
            Some(samples) => {
                committed_loaded.push(pack_name.to_string());
                for s in samples {
                    all.push(LoadedSample {
                        pack_label: pack_name.to_string(),
                        sample_id: s.id,
                        text: s.text,
                    });
                }
            }
            None => {
                eprintln!("scaling_bench: skipping missing {}", path.display());
            }
        }
    }

    if use_shards && paths.shards_dir.exists() {
        let mut shard_files: Vec<PathBuf> = match fs::read_dir(&paths.shards_dir) {
            Ok(rd) => rd
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.is_file())
                .filter(|p| {
                    let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                    SHARD_PACK_PREFIXES.iter().any(|pfx| name.starts_with(pfx))
                })
                .collect(),
            Err(e) => {
                eprintln!(
                    "scaling_bench: cannot list shards {}: {e}",
                    paths.shards_dir.display()
                );
                Vec::new()
            }
        };
        shard_files.sort();
        for path in shard_files {
            match read_pack(&path) {
                Some(samples) => {
                    let label = path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("<shard>")
                        .to_string();
                    shards_loaded.push(label.clone());
                    for s in samples {
                        all.push(LoadedSample {
                            pack_label: label.clone(),
                            sample_id: s.id,
                            text: s.text,
                        });
                    }
                }
                None => {
                    eprintln!("scaling_bench: skipping unparseable {}", path.display());
                }
            }
        }
    }

    let total_samples_available = all.len();
    let total_words_available: u64 = all
        .iter()
        .map(|s| s.text.split_whitespace().count() as u64)
        .sum();

    let snap = SourcesSnapshot {
        committed_packs_loaded: committed_loaded,
        shard_packs_loaded: shards_loaded,
        total_samples_available,
        total_words_available,
    };
    (all, snap)
}

fn read_pack(path: &Path) -> Option<Vec<PackSample>> {
    let raw = fs::read_to_string(path).ok()?;
    let parsed: PackFile = serde_json::from_str(&raw).ok()?;
    Some(parsed.samples)
}

/// Run one tier — take the first `target_samples` entries (or all if
/// `target_samples == 0`), extract, project, reason, and report.
pub fn run_tier(
    label: impl Into<String>,
    target_samples: usize,
    corpus: &[LoadedSample],
    lexicon: &LexiconV1,
) -> ScalingPoint {
    let slice_end = if target_samples == 0 {
        corpus.len()
    } else {
        target_samples.min(corpus.len())
    };
    let slice = &corpus[..slice_end];

    let words_scanned: u64 = slice
        .iter()
        .map(|s| s.text.split_whitespace().count() as u64)
        .sum();

    // Stage 1 — extract. Parallel per-sample via rayon; chunked to
    // keep memory pressure low and budget-check granularity high in
    // the enclosing binary. Input-order-preserving collect keeps the
    // fact array deterministic (same order every run).
    let t0 = Instant::now();
    let facts: Vec<Fact> = slice
        .par_iter()
        .flat_map_iter(|s| {
            let source = FactSource {
                pack: s.pack_label.clone(),
                sample_id: s.sample_id.clone(),
            };
            extract_facts(&s.text, &[], lexicon, &source).into_iter()
        })
        .collect();
    let extract_ms = t0.elapsed().as_millis() as u64;

    let mut facts_by_predicate: BTreeMap<String, usize> = BTreeMap::new();
    for f in &facts {
        *facts_by_predicate
            .entry(f.predicate.as_str().to_string())
            .or_insert(0) += 1;
    }

    // Stage 2 — graph projection.
    let t1 = Instant::now();
    let graph = LexicalGraph::from_facts(&facts);
    let graph_ms = t1.elapsed().as_millis() as u64;

    // Stage 3 — reasoner. Use a fresh unbounded budget for each tier;
    // the outer bench driver is the one with a wall-clock cap.
    let t2 = Instant::now();
    let budget = IterationBudget::unbounded_for_tests();
    let (derived, _iterations) = reasoner::run_with_budget(&facts, &budget);
    let reason_ms = t2.elapsed().as_millis() as u64;

    let mut derivations_by_rule: BTreeMap<String, usize> = BTreeMap::new();
    for d in &derived {
        *derivations_by_rule.entry(d.rule_id.clone()).or_insert(0) += 1;
    }

    ScalingPoint {
        label: label.into(),
        target_samples,
        samples_scanned: slice.len(),
        words_scanned,
        facts_extracted: facts.len(),
        facts_by_predicate,
        derivations: derived.len(),
        derivations_by_rule,
        graph_nodes: graph.nodes.len(),
        graph_edges: graph.edges.len(),
        elapsed_ms: StageMs {
            extract: extract_ms,
            graph: graph_ms,
            reason: reason_ms,
        },
    }
}

/// Run every tier in order, build a full [`ScalingReport`]. Tiers are
/// monotone non-decreasing by the caller's convention — the driver
/// accepts any list and preserves order in the output.
pub fn run_bench(
    tiers: &[(String, usize)],
    corpus: &[LoadedSample],
    lexicon: &LexiconV1,
    sources: SourcesSnapshot,
) -> ScalingReport {
    let started = Instant::now();
    let points: Vec<ScalingPoint> = tiers
        .iter()
        .map(|(label, n)| run_tier(label.clone(), *n, corpus, lexicon))
        .collect();
    let total_elapsed_s = started.elapsed().as_secs();

    ScalingReport {
        version: env!("CARGO_PKG_VERSION").to_string(),
        total_elapsed_s,
        machine: MachineSignal {
            rayon_threads: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1),
        },
        sources,
        tiers: points,
    }
}

/// Render the report as a Markdown document. The scaling-law curve is
/// an ASCII table (not a plot) — deterministic across runs and diffs
/// cleanly. Wall-clock ms is the only non-deterministic field.
pub fn render_markdown(report: &ScalingReport) -> String {
    let mut out = String::new();
    out.push_str("# adam scaling report\n\n");
    out.push_str(&format!(
        "Generated from `data/scaling/scaling_report.json` — version {}, total wall-clock {} s, Rayon threads {}.\n\n",
        report.version, report.total_elapsed_s, report.machine.rayon_threads,
    ));
    out.push_str(&format!(
        "Corpus loaded: {} committed pack(s) + {} shard pack(s) = **{} samples / {} words available**.\n\n",
        report.sources.committed_packs_loaded.len(),
        report.sources.shard_packs_loaded.len(),
        report.sources.total_samples_available,
        report.sources.total_words_available,
    ));
    out.push_str("## Scaling-law data points\n\n");
    out.push_str(
        "| tier | samples | words | facts | derivations | graph nodes | graph edges | extract ms | graph ms | reason ms |\n",
    );
    out.push_str("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    for p in &report.tiers {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            p.label,
            p.samples_scanned,
            p.words_scanned,
            p.facts_extracted,
            p.derivations,
            p.graph_nodes,
            p.graph_edges,
            p.elapsed_ms.extract,
            p.elapsed_ms.graph,
            p.elapsed_ms.reason,
        ));
    }
    out.push_str("\n## Predicates by tier\n\n");
    for p in &report.tiers {
        let parts: Vec<String> = p
            .facts_by_predicate
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        out.push_str(&format!(
            "- **{}** ({} facts): {}\n",
            p.label,
            p.facts_extracted,
            if parts.is_empty() {
                "—".to_string()
            } else {
                parts.join(", ")
            }
        ));
    }
    out.push_str("\n## Rule activations by tier\n\n");
    for p in &report.tiers {
        let parts: Vec<String> = p
            .derivations_by_rule
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        out.push_str(&format!(
            "- **{}** ({} derivations): {}\n",
            p.label,
            p.derivations,
            if parts.is_empty() {
                "—".to_string()
            } else {
                parts.join(", ")
            }
        ));
    }
    out.push_str("\n*Generated by `cargo run --release -p adam-scaling --bin scaling_bench`.*\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    // Mini synthetic pack with facts that we know should extract.
    fn write_pack(path: &PathBuf, samples: &[(&str, &str)]) {
        let wrapped: serde_json::Value = serde_json::json!({
            "version": "test",
            "samples": samples.iter().map(|(id, text)| {
                serde_json::json!({"id": id, "text": text})
            }).collect::<Vec<_>>()
        });
        std::fs::write(path, serde_json::to_string_pretty(&wrapped).unwrap()).unwrap();
    }

    fn load_lex() -> Option<LexiconV1> {
        // Use the real Lexicon — the bench is inherently about real
        // facts. If the Lexicon isn't available in the test env
        // (CI stripped checkout), skip gracefully.
        let curated = "../../data/tokenizer/segmentation_roots.json";
        let apertium = "../../data/lexicon_v1/apertium_imported_roots.json";
        if !std::path::Path::new(curated).exists() {
            return None;
        }
        LexiconV1::load(curated, apertium).ok()
    }

    #[test]
    fn run_tier_on_two_sample_synthetic_pack_produces_deterministic_counts() {
        let Some(lex) = load_lex() else { return };
        let dir = tempdir().unwrap();
        let curated = dir.path().join("curated");
        std::fs::create_dir_all(&curated).unwrap();
        write_pack(
            &curated.join("tatoeba_kazakh_pack.json"),
            &[("t_001", "Абай — ақын"), ("t_002", "екі сөз")],
        );
        // Only one canonical pack present — others are skipped.
        let paths = CorpusPaths {
            committed_dir: curated,
            shards_dir: dir.path().join("shards"),
        };
        let (corpus, sources) = load_corpus(&paths, false);
        assert_eq!(corpus.len(), 2);
        assert_eq!(sources.committed_packs_loaded.len(), 1);

        let t1 = run_tier("T_small", 0, &corpus, &lex);
        assert_eq!(t1.samples_scanned, 2);
        assert!(t1.words_scanned >= 2);
        // Re-run yields identical counts (elapsed_ms aside).
        let t2 = run_tier("T_small", 0, &corpus, &lex);
        assert_eq!(t1.samples_scanned, t2.samples_scanned);
        assert_eq!(t1.facts_extracted, t2.facts_extracted);
        assert_eq!(t1.graph_nodes, t2.graph_nodes);
        assert_eq!(t1.graph_edges, t2.graph_edges);
        assert_eq!(t1.derivations, t2.derivations);
    }

    #[test]
    fn tier_target_caps_samples_scanned() {
        let Some(lex) = load_lex() else { return };
        let dir = tempdir().unwrap();
        let curated = dir.path().join("curated");
        std::fs::create_dir_all(&curated).unwrap();
        write_pack(
            &curated.join("tatoeba_kazakh_pack.json"),
            &[("a", "Абай — ақын"), ("b", "x"), ("c", "y"), ("d", "z")],
        );
        let paths = CorpusPaths {
            committed_dir: curated,
            shards_dir: dir.path().join("shards"),
        };
        let (corpus, _) = load_corpus(&paths, false);
        let t = run_tier("T_2", 2, &corpus, &lex);
        assert_eq!(t.samples_scanned, 2);
        assert_eq!(t.target_samples, 2);
    }

    #[test]
    fn missing_shards_dir_is_silent() {
        let dir = tempdir().unwrap();
        let curated = dir.path().join("curated");
        std::fs::create_dir_all(&curated).unwrap();
        // Intentionally no shards dir.
        let paths = CorpusPaths {
            committed_dir: curated,
            shards_dir: dir.path().join("does-not-exist"),
        };
        let (corpus, snap) = load_corpus(&paths, true);
        assert!(corpus.is_empty());
        assert!(snap.shard_packs_loaded.is_empty());
    }

    #[test]
    fn render_markdown_includes_every_tier() {
        let report = ScalingReport {
            version: "test".into(),
            total_elapsed_s: 0,
            machine: MachineSignal { rayon_threads: 4 },
            sources: SourcesSnapshot {
                committed_packs_loaded: vec!["a.json".into()],
                shard_packs_loaded: vec![],
                total_samples_available: 10,
                total_words_available: 100,
            },
            tiers: vec![
                ScalingPoint {
                    label: "T1".into(),
                    target_samples: 5,
                    samples_scanned: 5,
                    words_scanned: 42,
                    facts_extracted: 2,
                    facts_by_predicate: [("is_a".to_string(), 2usize)].into_iter().collect(),
                    derivations: 0,
                    derivations_by_rule: Default::default(),
                    graph_nodes: 3,
                    graph_edges: 2,
                    elapsed_ms: StageMs {
                        extract: 10,
                        graph: 1,
                        reason: 0,
                    },
                },
                ScalingPoint {
                    label: "T2".into(),
                    target_samples: 10,
                    samples_scanned: 10,
                    words_scanned: 100,
                    facts_extracted: 5,
                    facts_by_predicate: [("is_a".to_string(), 3usize), ("has".to_string(), 2usize)]
                        .into_iter()
                        .collect(),
                    derivations: 1,
                    derivations_by_rule: [("R5_shared_is_a_target".to_string(), 1usize)]
                        .into_iter()
                        .collect(),
                    graph_nodes: 6,
                    graph_edges: 5,
                    elapsed_ms: StageMs {
                        extract: 20,
                        graph: 1,
                        reason: 0,
                    },
                },
            ],
        };
        let md = render_markdown(&report);
        assert!(md.contains("T1"));
        assert!(md.contains("T2"));
        assert!(md.contains("R5_shared_is_a_target=1"));
        // BTreeMap iteration → alphabetical: "has" before "is_a".
        assert!(md.contains("has=2, is_a=3"));
    }
}
