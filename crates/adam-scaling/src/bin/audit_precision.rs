//! `audit_precision` — sample `N` facts and `M` derivations from the
//! committed artifacts and render a Markdown review file for manual
//! native-speaker precision audit.
//!
//! Precision audit answers a question the raw fact count cannot:
//! **are the extracted facts actually correct?** A matcher that fires
//! on 100 surfaces and produces 100 facts is only useful if most of
//! those 100 are semantically valid. Without this audit, scaling
//! numbers can grow by adding false positives, not true knowledge.
//!
//! ## Output
//!
//! Writes `docs/precision_audit.md` with:
//!
//!   - deterministic random sample of `--facts-sample N` facts from
//!     `data/retrieval/facts.json`;
//!   - deterministic random sample of `--derivations-sample M`
//!     derivations from `data/retrieval/derived_facts.json`;
//!   - each entry includes the full source sentence, the extracted
//!     triple, the pattern id, and a checkbox for the reviewer;
//!   - at the bottom, a tally template the reviewer fills in to
//!     compute precision.
//!
//! ## Determinism
//!
//! Seeded by `--seed <u64>` (default 42). Same seed + same input
//! artifacts → byte-identical output. Reviewers across the team can
//! independently re-run and get the same sample.
//!
//! ## Usage
//!
//! ```
//! cargo run --release -p adam-scaling --bin audit_precision
//!   # uses 50 facts + all derivations (the committed set is tiny)
//! cargo run --release -p adam-scaling --bin audit_precision -- \
//!   --facts-sample 100 --derivations-sample 50 --seed 7
//! ```

use std::{collections::BTreeMap, env, fs, path::Path, process::ExitCode};

use adam_reasoning::{Fact, reasoner::DerivedFact};
use serde::Deserialize;

const FACTS_PATH: &str = "data/retrieval/facts.json";
const DERIVED_PATH: &str = "data/retrieval/derived_facts.json";
const OUT_PATH: &str = "docs/precision_audit.md";

const DEFAULT_FACTS_SAMPLE: usize = 50;
const DEFAULT_DERIV_SAMPLE: usize = 50;
const DEFAULT_SEED: u64 = 42;

#[derive(Debug, Deserialize)]
struct FactsFile {
    facts: Vec<Fact>,
    #[serde(default)]
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DerivedFile {
    derived: Vec<DerivedFact>,
    #[serde(default)]
    status: Option<String>,
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let facts_sample = parse_usize(&args, "--facts-sample").unwrap_or(DEFAULT_FACTS_SAMPLE);
    let deriv_sample = parse_usize(&args, "--derivations-sample").unwrap_or(DEFAULT_DERIV_SAMPLE);
    let seed = parse_u64(&args, "--seed").unwrap_or(DEFAULT_SEED);

    let facts_file: FactsFile = match load_json(FACTS_PATH) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("cannot read {FACTS_PATH}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let derived_file: DerivedFile = match load_json(DERIVED_PATH) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("cannot read {DERIVED_PATH}: {e}");
            return ExitCode::FAILURE;
        }
    };

    let facts_picked = deterministic_sample(&facts_file.facts, facts_sample, seed);
    let derived_picked = deterministic_sample(&derived_file.derived, deriv_sample, seed ^ 0x1);

    let md = render_markdown(
        facts_sample,
        deriv_sample,
        seed,
        facts_file.facts.len(),
        derived_file.derived.len(),
        facts_file.status.as_deref(),
        derived_file.status.as_deref(),
        &facts_picked,
        &derived_picked,
    );

    if let Some(parent) = Path::new(OUT_PATH).parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                eprintln!("cannot create {}: {e}", parent.display());
                return ExitCode::FAILURE;
            }
        }
    }
    if let Err(e) = fs::write(OUT_PATH, md) {
        eprintln!("write {OUT_PATH}: {e}");
        return ExitCode::FAILURE;
    }
    eprintln!(
        "wrote {OUT_PATH} — {} fact picks + {} derivation picks (seed={seed})",
        facts_picked.len(),
        derived_picked.len(),
    );
    ExitCode::SUCCESS
}

/// Deterministic sample: xorshift64 on a `(seed, index)` key, select
/// the `requested.min(items.len())` smallest-hash items. Same seed +
/// same input length → same sample indices.
fn deterministic_sample<T: Clone>(items: &[T], requested: usize, seed: u64) -> Vec<(usize, T)> {
    let n = items.len();
    if n == 0 {
        return Vec::new();
    }
    let take = requested.min(n);
    let mut with_hash: Vec<(u64, usize)> = (0..n).map(|i| (xorshift64_mix(seed, i), i)).collect();
    with_hash.sort_by_key(|&(h, _)| h);
    let mut picked: Vec<(usize, T)> = with_hash
        .into_iter()
        .take(take)
        .map(|(_, i)| (i, items[i].clone()))
        .collect();
    picked.sort_by_key(|&(i, _)| i); // stable presentation in index order
    picked
}

fn xorshift64_mix(seed: u64, i: usize) -> u64 {
    let mut x = seed.wrapping_add((i as u64).wrapping_mul(0x9E3779B97F4A7C15));
    x ^= x >> 33;
    x = x.wrapping_mul(0xFF51AFD7ED558CCD);
    x ^= x >> 33;
    x = x.wrapping_mul(0xC4CEB9FE1A85EC53);
    x ^= x >> 33;
    x
}

#[allow(clippy::too_many_arguments)]
fn render_markdown(
    facts_sample: usize,
    deriv_sample: usize,
    seed: u64,
    facts_total: usize,
    deriv_total: usize,
    facts_status: Option<&str>,
    deriv_status: Option<&str>,
    facts: &[(usize, Fact)],
    derivations: &[(usize, DerivedFact)],
) -> String {
    let mut out = String::new();
    out.push_str("# Precision audit — native-speaker review\n\n");
    out.push_str(&format!(
        "**Target:** {}-fact sample + {}-derivation sample from the committed artifacts, seed `{seed}`.\n\n",
        facts_sample, deriv_sample
    ));
    out.push_str(&format!(
        "- `facts.json`: {facts_total} facts total (upstream status: `{}`) — sampled {} here.\n",
        facts_status.unwrap_or("<pre-v3.1>"),
        facts.len(),
    ));
    out.push_str(&format!(
        "- `derived_facts.json`: {deriv_total} derivations total (upstream status: `{}`) — sampled {} here.\n\n",
        deriv_status.unwrap_or("<pre-v3.1>"),
        derivations.len(),
    ));
    out.push_str("## How to review\n\n");
    out.push_str("For each fact, mark the checkbox if the triple `(subject, predicate, object)` is **correct**: the sentence genuinely asserts that the subject has the claimed relation to the object, and both root resolutions are correct. When unsure, leave unchecked and add a one-line note in the Comments row. Update the **Tally** section at the bottom with your counts. Precision is defined as `correct / reviewed`.\n\n");

    out.push_str("---\n\n");
    out.push_str("## Fact sample\n\n");
    for (idx, f) in facts {
        out.push_str(&format!(
            "### Fact #{idx}\n\n- Triple: `({} — {} — {})`\n- Predicate: `{}`\n- Pattern: `{}`\n- Source: `{} / {}`\n- Sentence:\n\n    > {}\n\n- [ ] Correct\n- Comment:\n\n",
            f.subject.root,
            f.predicate.as_str(),
            f.object.root,
            f.predicate.as_str(),
            f.pattern,
            f.source.pack,
            f.source.sample_id,
            f.raw_text.replace('\n', " "),
        ));
    }

    out.push_str("---\n\n");
    out.push_str("## Derivation sample\n\n");
    for (idx, d) in derivations {
        let sources: Vec<String> = d
            .source_chain
            .iter()
            .map(|s| format!("{}/{}", s.pack, s.sample_id))
            .collect();
        out.push_str(&format!(
            "### Derivation #{idx}\n\n- Triple: `({} — {} — {})`\n- Rule: `{}`\n- Confidence: `{}`\n- Source chain: {}\n\n- [ ] Derivation is semantically valid\n- [ ] Underlying facts are both correct\n- Comment:\n\n",
            d.subject.root,
            d.predicate.as_str(),
            d.object.root,
            d.rule_id,
            d.confidence.as_str(),
            if sources.is_empty() {
                "_empty (unsound!)_".to_string()
            } else {
                sources.join(", ")
            },
        ));
    }

    out.push_str("---\n\n");
    out.push_str("## Tally\n\n");
    out.push_str("Fill in after review. `N` = number of items you ended up reviewing; `C` = number you marked correct.\n\n");
    out.push_str(&format!(
        "- Facts: C = __ / N = {} (precision = ___%)\n",
        facts.len()
    ));
    out.push_str(&format!(
        "- Derivations (semantic): C = __ / N = {} (precision = ___%)\n",
        derivations.len()
    ));
    out.push_str(&format!(
        "- Derivations (both underlying facts): C = __ / N = {} (precision = ___%)\n\n",
        derivations.len()
    ));

    out.push_str("## By-pattern + by-rule summary\n\n");
    let mut by_pattern: BTreeMap<String, usize> = BTreeMap::new();
    for (_, f) in facts {
        *by_pattern.entry(f.pattern.clone()).or_insert(0) += 1;
    }
    out.push_str("Sampled facts by pattern:\n\n");
    for (pat, n) in &by_pattern {
        out.push_str(&format!("- `{pat}`: {n}\n"));
    }
    let mut by_rule: BTreeMap<String, usize> = BTreeMap::new();
    for (_, d) in derivations {
        *by_rule.entry(d.rule_id.clone()).or_insert(0) += 1;
    }
    out.push_str("\nSampled derivations by rule:\n\n");
    for (rule, n) in &by_rule {
        out.push_str(&format!("- `{rule}`: {n}\n"));
    }

    out
}

fn load_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, String> {
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}

fn parse_usize(args: &[String], name: &str) -> Option<usize> {
    let idx = args.iter().position(|a| a == name)?;
    args.get(idx + 1).and_then(|s| s.parse().ok())
}

fn parse_u64(args: &[String], name: &str) -> Option<u64> {
    let idx = args.iter().position(|a| a == name)?;
    args.get(idx + 1).and_then(|s| s.parse().ok())
}
