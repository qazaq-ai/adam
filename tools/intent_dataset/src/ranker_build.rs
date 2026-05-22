// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/qadam
//! # `ranker_dataset_build`
//!
//! Builds the labelled (query, candidates, picked) corpus for
//! the **E3 retrieval re-ranker** experiment. Walks every
//! `data/eval/*.json` query, sources candidate facts from
//! `data/retrieval/facts.json` (the canonical `ReasFact` pool),
//! filters down to candidates that share at least one query
//! token with the fact's subject or object root, scores each
//! candidate via the existing hand-set
//! `selection::default_v0` ranker, and emits one labelled row
//! per query where the picked candidate is index 0 (positive)
//! and the rest are negatives.
//!
//! Output: `data/retrieval_ranker/v1/dataset.jsonl`.
//!
//! Schema:
//! ```jsonl
//! {"id":"q_00001","query":"Абай туралы айтшы","candidates":[{"features":[..10..],"picked":1},{"features":[..10..],"picked":0},...]}
//! ```
//!
//! See `docs/e3_retrieval_ranker_design.md` § "Training data".

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use adam_dialog::selection::{self, CandidateFeatures, SelectionWeights};
use adam_reasoning::{ConfidenceKind, Fact as ReasFact, Predicate, SlotRef};
use adam_retrieval_ranker::CandidateFeatures as RankerFeatures;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct EvalCase {
    #[serde(default)]
    query: Option<String>,
    #[serde(default)]
    prompt: Option<String>,
    #[serde(default)]
    input: Option<String>,
}

impl EvalCase {
    fn surface(&self) -> Option<&str> {
        self.query
            .as_deref()
            .or(self.prompt.as_deref())
            .or(self.input.as_deref())
    }
}

#[derive(Debug, Deserialize)]
struct EvalEnvelope {
    #[serde(default)]
    cases: Option<Vec<EvalCase>>,
    #[serde(default)]
    prompts: Option<Vec<EvalCase>>,
}

/// Lenient subset of `Fact` we need for ranking.
#[derive(Debug, Deserialize, Clone)]
struct FactRow {
    subject: SlotRef,
    predicate: Predicate,
    object: SlotRef,
    confidence: ConfidenceKind,
    raw_text: String,
}

#[derive(Debug, Deserialize)]
struct FactsFile {
    facts: Vec<FactRow>,
}

#[derive(Debug, Serialize, Clone)]
struct CandidateRow {
    /// 10 features in `RankerFeatures::as_vec()` declaration order.
    features: [f32; RankerFeatures::N],
    /// 1 iff this candidate is the cascade's pick for this query.
    picked: u8,
    /// Diagnostic: stored so per-row failure modes are inspectable.
    /// Lowercase fact subject root.
    subject: String,
    /// Lowercase fact object root.
    object: String,
}

#[derive(Debug, Serialize)]
struct LabelledQuery {
    id: String,
    query: String,
    candidates: Vec<CandidateRow>,
    /// Index into `candidates` where `picked == 1`. Convenience
    /// for trainers that prefer the index over scanning.
    picked_idx: usize,
}

const DATASET_OUT: &str = "data/retrieval_ranker/v1/dataset.jsonl";
const EVAL_DIR: &str = "data/eval";
const FACTS_PATH: &str = "data/retrieval/facts.json";

const MAX_CANDIDATES_PER_QUERY: usize = 12;

/// Whitespace-tokenise, lowercase, strip surrounding
/// punctuation. Tokens of < 3 chars are kept so the cascade
/// agrees with our filter (cascade uses different lower bounds
/// in different paths; 3 is a safe floor for substring match).
fn tokenise(input: &str) -> Vec<String> {
    input
        .split_whitespace()
        .map(|tok| {
            tok.chars()
                .filter(|c| c.is_alphabetic() || c.is_ascii_digit() || *c == '-')
                .flat_map(|c| c.to_lowercase())
                .collect::<String>()
        })
        .filter(|t| !t.is_empty())
        .collect()
}

/// Score candidate via the existing hand-set v0 weights. This
/// is the **cascade-oracle** we're training to match (and
/// hopefully exceed) on richer features.
fn cascade_score(fact: &ReasFact, query_tokens: &[&str], last_topic: Option<&str>) -> f32 {
    let f = selection::extract_features(fact, query_tokens, last_topic);
    selection::score(&f, &SelectionWeights::default_v0())
}

/// Convert FactRow to ReasFact for the cascade ranker.
fn to_reas(row: &FactRow) -> ReasFact {
    ReasFact {
        subject: row.subject.clone(),
        predicate: row.predicate,
        object: row.object.clone(),
        pattern: String::new(),
        source: adam_reasoning::FactSource {
            pack: String::new(),
            sample_id: String::new(),
        },
        confidence: row.confidence,
        raw_text: row.raw_text.clone(),
    }
}

/// Build the 10-feature vector for the E3 ranker. First 5 are
/// the existing `CandidateFeatures` mirrored from `selection`;
/// last 5 are E3's additions (TF-IDF cosine, predicate match,
/// IsA distance, raw-len normalised, candidate position).
///
/// **Round-2 baseline**: the 5 additions are stubbed to
/// reasonable proxies (tfidf_cosine ≈ subject_overlap when
/// no TF-IDF table is loaded; predicate_match = 1.0 if any
/// query token is the predicate's slug; isa_distance = 1.0
/// always since the IsA graph traversal isn't wired into this
/// builder yet; raw_len_norm = raw_text.chars() / 200 capped;
/// cand_pos = idx / list_len). The trainer can still learn
/// non-trivial weights over these proxies — and a Round 3
/// commit replaces the proxies with real implementations.
fn build_features(
    fact: &ReasFact,
    cascade_feats: &CandidateFeatures,
    query_tokens: &[&str],
    cand_idx: usize,
    cand_count: usize,
) -> [f32; RankerFeatures::N] {
    let raw_len = fact.raw_text.chars().count() as f32;
    let raw_len_norm = (raw_len / 200.0).min(1.0);
    let cand_pos = if cand_count > 0 {
        cand_idx as f32 / cand_count as f32
    } else {
        0.0
    };
    // Predicate match proxy: 1.0 if any query token equals the
    // predicate's debug slug (lowercased). E.g. query containing
    // «isa» / «has» — rare in user queries, so this fires
    // sparingly; sufficient as a v1 signal.
    let pred_slug = format!("{:?}", fact.predicate).to_lowercase();
    let predicate_match = if query_tokens.iter().any(|t| pred_slug.contains(t)) {
        1.0
    } else {
        0.0
    };
    // TF-IDF cosine proxy: token-overlap fraction between query
    // and fact raw_text, no IDF weighting. Replace with real
    // TF-IDF in Round 3.
    let raw_tokens: Vec<String> = tokenise(&fact.raw_text);
    let raw_set: std::collections::HashSet<&str> = raw_tokens.iter().map(String::as_str).collect();
    let overlap = query_tokens.iter().filter(|t| raw_set.contains(*t)).count();
    let tfidf_cosine = if query_tokens.is_empty() {
        0.0
    } else {
        (overlap as f32 / query_tokens.len() as f32).min(1.0)
    };
    // IsA distance proxy: 0.0 if subject literally equals first
    // query token, 1.0 otherwise. Replace with graph BFS in
    // Round 3.
    let isa_distance = if let Some(t0) = query_tokens.first() {
        if fact.subject.root.to_lowercase() == *t0 {
            0.0
        } else {
            1.0
        }
    } else {
        1.0
    };
    [
        cascade_feats.confidence,
        cascade_feats.raw_text_richness,
        cascade_feats.subject_overlap,
        cascade_feats.object_overlap,
        cascade_feats.recency_match,
        tfidf_cosine,
        predicate_match,
        isa_distance,
        raw_len_norm,
        cand_pos,
    ]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the canonical fact pool.
    let facts_raw = fs::read_to_string(FACTS_PATH)?;
    let facts_file: FactsFile = serde_json::from_str(&facts_raw)?;
    let facts: Vec<FactRow> = facts_file.facts;
    eprintln!("loaded {} facts", facts.len());

    // Pre-tokenise facts for faster matching.
    let fact_tokens: Vec<Vec<String>> = facts
        .iter()
        .map(|f| {
            let mut t = Vec::with_capacity(4);
            t.extend(tokenise(&f.subject.root));
            t.extend(tokenise(&f.object.root));
            t
        })
        .collect();

    let eval_files: Vec<PathBuf> = fs::read_dir(EVAL_DIR)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "json").unwrap_or(false))
        .collect();

    let mut emitted: Vec<LabelledQuery> = Vec::new();
    let mut total_seen = 0usize;
    let mut skipped_no_candidate = 0usize;
    let mut per_size: HashMap<usize, usize> = HashMap::new();

    for path in &eval_files {
        let bytes = match fs::read(path) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let env: EvalEnvelope = match serde_json::from_slice(&bytes) {
            Ok(e) => e,
            Err(_) => continue,
        };
        let cases: Vec<EvalCase> = env
            .cases
            .into_iter()
            .flatten()
            .chain(env.prompts.into_iter().flatten())
            .collect();
        for case in cases {
            let Some(surface) = case.surface() else {
                continue;
            };
            let trimmed = surface.trim();
            if trimmed.is_empty() {
                continue;
            }
            total_seen += 1;
            let qt = tokenise(trimmed);
            let qt_refs: Vec<&str> = qt.iter().map(String::as_str).collect();
            if qt_refs.is_empty() {
                continue;
            }
            // Candidate filter: fact's subject OR object root
            // shares at least one token (length ≥ 3) with the
            // query.
            let mut cand_indices: Vec<usize> = Vec::new();
            for (i, t) in fact_tokens.iter().enumerate() {
                if t.iter().any(|f_t| {
                    f_t.chars().count() >= 3 && qt_refs.iter().any(|q| q.contains(f_t.as_str()))
                }) {
                    cand_indices.push(i);
                }
            }
            if cand_indices.is_empty() {
                skipped_no_candidate += 1;
                continue;
            }
            // Cap the candidate list size. Score every fact via
            // the hand-set ranker and keep the top
            // MAX_CANDIDATES_PER_QUERY. Within the kept set,
            // the argmax-by-cascade-score becomes the positive
            // label.
            let mut scored: Vec<(usize, f32)> = cand_indices
                .iter()
                .map(|&i| {
                    let reas = to_reas(&facts[i]);
                    (i, cascade_score(&reas, &qt_refs, None))
                })
                .collect();
            scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            scored.truncate(MAX_CANDIDATES_PER_QUERY);
            if scored.len() < 2 {
                // Need at least 2 candidates for a meaningful
                // pointwise pair.
                skipped_no_candidate += 1;
                continue;
            }
            *per_size.entry(scored.len()).or_default() += 1;
            // **Data-leakage prevention.** Shuffle candidate
            // order before emitting so `cand_pos` does not
            // directly encode "is this the cascade pick". The
            // cascade-picked fact (index 0 in the sorted list)
            // is recorded as `picked_idx` AFTER shuffling.
            let cand_count = scored.len();
            // Deterministic per-query shuffle (seed = id-hash) so
            // dataset is reproducible.
            let seed: u64 = trimmed.bytes().fold(0xc0de_d00d_u64, |a, b| {
                a.wrapping_mul(31).wrapping_add(b as u64)
            });
            let mut perm: Vec<usize> = (0..cand_count).collect();
            let mut s = seed;
            for i in (1..perm.len()).rev() {
                s = s
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                let j = (s >> 32) as usize % (i + 1);
                perm.swap(i, j);
            }
            // `original_idx[k]` = where the k-th shuffled
            // candidate came from in the sorted list. The
            // sorted-list index 0 was the cascade pick.
            let cascade_pick_shuffled = perm.iter().position(|&p| p == 0).unwrap_or(0);
            let candidates: Vec<CandidateRow> = perm
                .iter()
                .enumerate()
                .map(|(local_idx, &orig)| {
                    let (fact_idx, _score) = scored[orig];
                    let fact = &facts[fact_idx];
                    let reas = to_reas(fact);
                    let cascade_feats = selection::extract_features(&reas, &qt_refs, None);
                    let features =
                        build_features(&reas, &cascade_feats, &qt_refs, local_idx, cand_count);
                    CandidateRow {
                        features,
                        picked: u8::from(local_idx == cascade_pick_shuffled),
                        subject: fact.subject.root.to_lowercase(),
                        object: fact.object.root.to_lowercase(),
                    }
                })
                .collect();
            let id = format!("q_{:05}", emitted.len() + 1);
            emitted.push(LabelledQuery {
                id,
                query: trimmed.to_string(),
                candidates,
                picked_idx: cascade_pick_shuffled,
            });
        }
    }

    let out_path = Path::new(DATASET_OUT);
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut buf = String::new();
    for ex in &emitted {
        buf.push_str(&serde_json::to_string(ex)?);
        buf.push('\n');
    }
    fs::write(out_path, buf)?;

    eprintln!("=== E3 ranker-dataset build summary ===");
    eprintln!("eval files scanned:   {}", eval_files.len());
    eprintln!("queries seen:          {total_seen}");
    eprintln!(
        "skipped (<2 cands):    {skipped_no_candidate} ({:.1}%)",
        100.0 * skipped_no_candidate as f64 / total_seen.max(1) as f64
    );
    eprintln!("labelled queries:      {}", emitted.len());
    let total_cands: usize = emitted.iter().map(|q| q.candidates.len()).sum();
    eprintln!("total candidate rows:  {total_cands}");
    eprintln!("output:                {DATASET_OUT}");
    eprintln!();
    eprintln!("--- candidates-per-query histogram (top 5) ---");
    let mut sizes: Vec<(usize, usize)> = per_size.into_iter().collect();
    sizes.sort_by(|a, b| b.1.cmp(&a.1));
    for (size, count) in sizes.iter().take(5) {
        eprintln!("  {size:>2} cands → {count} queries");
    }
    Ok(())
}
