//! `train_suffix_priors` — offline training pass that builds the
//! frequency-based prior over suffix-chain signatures shipped as
//! `data/retrieval/suffix_chain_priors.json`.
//!
//! **v4.15.0 — first compositional ML layer.** This is the
//! offline counterpart to [`adam_kernel_fst::suffix_priors`]: it
//! walks the committed corpus packs, runs the FST analyser on each
//! token, attributes uniform `1/N` weight to each candidate parse,
//! tallies counts per chain signature, and writes the resulting
//! prior as JSON for the runtime to load.
//!
//! **Input.** `data/curated/*_pack.json` files, each carrying a
//! `samples: [{id, pack_name, source_id, domain, text}]` array. The
//! `text` field holds the Kazakh sentence (or fragment) tokenised
//! by whitespace + alphabetic-character filter.
//!
//! **Output.** `data/retrieval/suffix_chain_priors.json` —
//! [`SuffixPriors`] with per-chain log-probabilities, ready for
//! load at conversation startup.
//!
//! **Algorithm (uniform attribution).** For each token, the FST
//! returns `N` candidate analyses. Without ground-truth
//! disambiguation we attribute `1/N` weight to each parse's chain
//! signature — every reading gets equal credit. After all tokens,
//! [`SuffixPriors::from_counts`] computes log-probabilities with
//! add-one smoothing.
//!
//! **Why uniform attribution.** Picking only the first parse
//! (v3.2.0 deterministic order) would make the prior a circular
//! reflection of the existing tie-breaker. Uniform attribution is
//! unbiased given no labels — the prior captures which suffix
//! chains *appear at all* in real Kazakh text, not which ones the
//! existing parser would pick.
//!
//! **CLI.**
//! ```text
//! cargo run --release -p adam-corpus --bin train_suffix_priors
//! ```
//! Walks every pack under `data/curated/`, writes
//! `data/retrieval/suffix_chain_priors.json`. Idempotent: rerunning
//! produces a byte-identical artifact (HashMap ordering is sorted
//! before serialisation for determinism).
//!
//! **Cost.** ~10 seconds end-to-end on the current 4.6 M committed-
//! corpus token base on M2 8 GB. Output is ~50 KB JSON.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use adam_kernel_fst::lexicon::LexiconV1;
use adam_kernel_fst::parser::{Analysis, analyse};
use adam_kernel_fst::suffix_priors::{
    SCHEMA_VERSION, SuffixPriors, noun_chain_key, verb_chain_key,
};
use serde::Deserialize;

const CURATED_DIR: &str = "data/curated";
const OUTPUT_PATH: &str = "data/retrieval/suffix_chain_priors.json";

#[derive(Debug, Deserialize)]
struct PackFile {
    #[serde(default)]
    samples: Vec<PackSample>,
}

#[derive(Debug, Deserialize)]
struct PackSample {
    /// Optional because some packs (e.g. id-only packs that just
    /// reference samples elsewhere) skip the text payload. Skipped
    /// at iteration time when `None`.
    #[serde(default)]
    text: Option<String>,
}

fn main() -> ExitCode {
    let curated = Path::new(CURATED_DIR);
    if !curated.is_dir() {
        eprintln!(
            "train_suffix_priors: expected curated dir at {}; aborting",
            curated.display()
        );
        return ExitCode::FAILURE;
    }

    let lex = match LexiconV1::load_default() {
        Ok(l) => l,
        Err(err) => {
            eprintln!("train_suffix_priors: lexicon load failed: {err:?}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!(
        "train_suffix_priors: lexicon loaded ({} entries)",
        lex.total()
    );

    let pack_paths = match collect_pack_paths(curated) {
        Ok(p) => p,
        Err(err) => {
            eprintln!("train_suffix_priors: failed to enumerate packs: {err}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!(
        "train_suffix_priors: found {} curated packs",
        pack_paths.len()
    );

    // **Performance.** FST analyse is O(lexicon_size) per token —
    // expensive when the same surface form appears thousands of
    // times. We dedupe: collect unique tokens first, run analyse()
    // once per form, cache the resulting chain keys, then walk the
    // corpus a second time to tally both unigram and bigram counts.
    //
    // **v4.16.0** — second pass now also computes bigram counts
    // `(prev_chain, current_chain) → count` for context-aware
    // priors. We retain the original sample order (sample
    // boundaries reset prev_chain to None) so adjacency is
    // meaningful — a bigram only counts within the same sentence.
    let mut all_samples: Vec<Vec<String>> = Vec::new();
    let mut total_tokens: u64 = 0;
    let mut total_samples: u64 = 0;

    for path in &pack_paths {
        let bytes = match fs::read(path) {
            Ok(b) => b,
            Err(err) => {
                eprintln!(
                    "train_suffix_priors: skipping {} (read error: {err})",
                    path.display()
                );
                continue;
            }
        };
        let pack: PackFile = match serde_json::from_slice(&bytes) {
            Ok(p) => p,
            Err(err) => {
                eprintln!(
                    "train_suffix_priors: skipping {} (schema mismatch: {err})",
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
            if !toks.is_empty() {
                all_samples.push(toks);
            }
        }
    }

    // Build the unique-token → chain-keys cache once.
    let mut unique_tokens: HashMap<String, ()> = HashMap::new();
    for sample in &all_samples {
        for tok in sample {
            unique_tokens.entry(tok.clone()).or_default();
        }
    }
    eprintln!(
        "train_suffix_priors: {} samples, {} tokens, {} unique forms — analysing each unique form once",
        total_samples,
        total_tokens,
        unique_tokens.len()
    );

    // chain_keys_for[token] = list of all chain keys returned by
    // analyse(token). Empty Vec means analyse returned no parses
    // (closed-class word, OOV, etc).
    let mut chain_keys_for: HashMap<String, Vec<String>> =
        HashMap::with_capacity(unique_tokens.len());
    let unique_total = unique_tokens.len() as u64;
    let mut analysed_forms: u64 = 0;
    for token in unique_tokens.keys() {
        analysed_forms += 1;
        if analysed_forms % 25_000 == 0 {
            eprintln!(
                "train_suffix_priors: analysed {} / {} unique forms",
                analysed_forms, unique_total
            );
        }
        let parses = analyse(token, &lex);
        let keys: Vec<String> = parses.iter().map(chain_key_for_analysis).collect();
        chain_keys_for.insert(token.clone(), keys);
    }

    // Second pass: walk samples in order, tally unigrams and
    // bigrams using the cache.
    let mut counts_f64: HashMap<String, f64> = HashMap::new();
    let mut bigram_counts_f64: HashMap<(String, String), f64> = HashMap::new();
    for sample in &all_samples {
        let mut prev_keys: Option<&Vec<String>> = None;
        for token in sample {
            let Some(curr_keys) = chain_keys_for.get(token) else {
                prev_keys = None;
                continue;
            };
            if curr_keys.is_empty() {
                prev_keys = None;
                continue;
            }
            // Unigram contribution: 1/N per parse.
            let curr_weight = 1.0_f64 / curr_keys.len() as f64;
            for key in curr_keys {
                *counts_f64.entry(key.clone()).or_insert(0.0) += curr_weight;
            }
            // Bigram contribution: 1/(N_prev * N_curr) per pair.
            if let Some(prev) = prev_keys {
                if !prev.is_empty() {
                    let pair_weight = 1.0_f64 / (prev.len() as f64 * curr_keys.len() as f64);
                    for p in prev {
                        for c in curr_keys {
                            *bigram_counts_f64
                                .entry((p.clone(), c.clone()))
                                .or_insert(0.0) += pair_weight;
                        }
                    }
                }
            }
            prev_keys = Some(curr_keys);
        }
    }

    eprintln!(
        "train_suffix_priors: counted {} tokens across {} samples; \
         {} distinct chains, {} distinct chain bigrams observed",
        total_tokens,
        total_samples,
        counts_f64.len(),
        bigram_counts_f64.len(),
    );

    // Round float counts to integers for the SuffixPriors API
    // (which expects u64). Uniform attribution generates
    // fractional counts; integer rounding is fine since the prior
    // only cares about relative magnitudes after smoothing.
    let counts: HashMap<String, u64> = counts_f64
        .into_iter()
        .map(|(k, v)| (k, v.round().max(1.0) as u64))
        .collect();
    // **Bigram pruning.** Drop pairs with raw count < 2.0 — they're
    // single-occurrence noise that bloats the artifact ~6× without
    // meaningful signal. Verified empirically: 305K observed
    // bigrams → 50K-ish after pruning, but we keep all the
    // statistically informative pairs (the ones the parse-time
    // re-ranker actually consults).
    let bigram_counts: HashMap<(String, String), u64> = bigram_counts_f64
        .into_iter()
        .filter(|(_, v)| *v >= 2.0)
        .map(|(k, v)| (k, v.round() as u64))
        .collect();
    let mut priors = SuffixPriors::from_counts_with_bigrams(counts, bigram_counts);
    // Override `trained_on_tokens` with the true integer count
    // (from_counts_with_bigrams sums the rounded unigram values).
    priors.trained_on_tokens = total_tokens;

    if let Err(err) = write_priors(&priors) {
        eprintln!("train_suffix_priors: failed to write artifact: {err}");
        return ExitCode::FAILURE;
    }
    eprintln!(
        "train_suffix_priors: wrote {} (schema v{}, {} chains)",
        OUTPUT_PATH,
        SCHEMA_VERSION,
        priors.len()
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

fn chain_key_for_analysis(parse: &Analysis) -> String {
    match parse {
        Analysis::Noun { features, .. } => noun_chain_key(features),
        Analysis::Verb { features, .. } => verb_chain_key(features),
    }
}

/// Write the priors as pretty-printed JSON. Sort keys for
/// reproducible byte output (`HashMap` iteration is otherwise
/// unstable across runs).
///
/// **v4.16.0** — also serialises `transition_log_prob` as a
/// nested map. Outer-key sorted alphabetically; inner rows
/// sorted alphabetically too — byte-stable across runs.
fn write_priors(priors: &SuffixPriors) -> std::io::Result<()> {
    let mut chain_sorted: Vec<(&String, &f32)> = priors.chain_log_prob.iter().collect();
    chain_sorted.sort_by(|a, b| a.0.cmp(b.0));

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"version\": {},\n", priors.version));
    json.push_str(&format!(
        "  \"trained_on_tokens\": {},\n",
        priors.trained_on_tokens
    ));
    json.push_str("  \"chain_log_prob\": {\n");
    for (i, (k, v)) in chain_sorted.iter().enumerate() {
        let comma = if i + 1 == chain_sorted.len() { "" } else { "," };
        let escaped = k.replace('\\', "\\\\").replace('"', "\\\"");
        json.push_str(&format!("    \"{escaped}\": {v}{comma}\n"));
    }
    json.push_str("  },\n");

    // transition_log_prob (v4.16.0).
    serialize_nested_map(
        &mut json,
        "transition_log_prob",
        &priors.transition_log_prob,
    );
    json.push_str(",\n");
    // **v4.17.0** — pos_transition_log_prob serialised the same
    // way for byte-stable output across runs.
    serialize_nested_map(
        &mut json,
        "pos_transition_log_prob",
        &priors.pos_transition_log_prob,
    );
    json.push_str("\n}\n");

    fs::write(OUTPUT_PATH, json)
}

/// **v4.17.0** — serialise a nested `HashMap<String,
/// HashMap<String, f32>>` block with sorted keys at both levels
/// for byte-stable output. Writes the field as `"name": { ... }`
/// (no trailing newline / comma).
fn serialize_nested_map(
    json: &mut String,
    field_name: &str,
    map: &HashMap<String, HashMap<String, f32>>,
) {
    let mut outer_sorted: Vec<(&String, &HashMap<String, f32>)> = map.iter().collect();
    outer_sorted.sort_by(|a, b| a.0.cmp(b.0));
    json.push_str(&format!("  \"{field_name}\": {{\n"));
    for (oi, (prev_key, row)) in outer_sorted.iter().enumerate() {
        let outer_comma = if oi + 1 == outer_sorted.len() {
            ""
        } else {
            ","
        };
        let prev_escaped = prev_key.replace('\\', "\\\\").replace('"', "\\\"");
        json.push_str(&format!("    \"{prev_escaped}\": {{\n"));
        let mut inner_sorted: Vec<(&String, &f32)> = row.iter().collect();
        inner_sorted.sort_by(|a, b| a.0.cmp(b.0));
        for (ii, (curr_key, score)) in inner_sorted.iter().enumerate() {
            let inner_comma = if ii + 1 == inner_sorted.len() {
                ""
            } else {
                ","
            };
            let curr_escaped = curr_key.replace('\\', "\\\\").replace('"', "\\\"");
            json.push_str(&format!("      \"{curr_escaped}\": {score}{inner_comma}\n"));
        }
        json.push_str(&format!("    }}{outer_comma}\n"));
    }
    json.push_str("  }");
}
