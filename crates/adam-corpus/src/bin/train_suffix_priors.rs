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
//! **v4.20.0 — root-level priors via UNAMBIGUOUS-only attribution.**
//! Closes the chain-collision blind spot surfaced by v4.19.5: when
//! two parses produce the same suffix chain (e.g. `он + Loc` and
//! `ол + Loc`), chain-level priors score them identically. Adding
//! root-level marginals breaks the tie. We *can't* use uniform
//! attribution for roots — splitting the count between он and ол
//! on the ambiguous token «онда» would zero the tiebreaker exactly
//! where we need it. Instead: a token contributes its full count
//! to a root only when `analyse()` returns parses with a single
//! distinct root. Bare nominative forms («ол», «олар», «онбес»)
//! and other unambiguous tokens build up the marginals; ambiguous
//! tokens are skipped from root counting entirely.
//!
//! **v4.20.5 — closed-class structural-pronoun boost.** v4.20.0
//! shipped the unambiguous-only attribution and ran into the
//! pronoun-bias problem: structural pronouns (ол, бұл, сол, мен,
//! сен, олар, біз, сіз) appear MOSTLY in inherently-ambiguous
//! inflected forms (онда, оны, оған, бұнда, маған, …) which get
//! filtered out by the unambiguous-only attribution. Result:
//! corpus marginal P(он=digit) > P(ол=pronoun), inverting reality.
//! v4.20.5 reads `data/lexicon/closed_class_root_boosts.json` —
//! a hand-curated multiplicative count boost per pronoun root —
//! and applies it AFTER the unambiguous-only counting completes
//! but BEFORE Laplace smoothing. The boost folds into the same
//! `root_log_prob` field; no schema change. Empirically validated
//! against the parse-disambiguation eval (the «онда» case must
//! flip from он to ол under `chain_tiebreak_root`).
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
/// **v4.20.5** — closed-class structural-pronoun boost file.
/// Path is optional: when missing, the training run falls back to
/// pure unambiguous-only attribution (v4.20.0 behaviour).
const CLOSED_CLASS_BOOST_PATH: &str = "data/lexicon/closed_class_root_boosts.json";

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

/// **v4.20.5** — closed-class structural-pronoun boost file. Maps
/// root → multiplicative count factor applied AFTER unambiguous-
/// only counting completes. Compensates for the systematic
/// under-counting of pronouns whose inflected forms are inherently
/// ambiguous and thus filtered from the v4.20.0 attribution scheme.
#[derive(Debug, Deserialize, Default)]
struct ClosedClassBoosts {
    #[serde(default)]
    boosts: std::collections::HashMap<String, f64>,
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
    // **v4.20.0** — also cache the unique-root list per token. We
    // use this in the second pass to skip ambiguous tokens from
    // root counting (per the unambiguous-only attribution policy
    // documented at the module level). When `unique_roots.len() ==
    // 1`, the single root receives the full token count;
    // otherwise the token is skipped from root tallies entirely.
    let mut unique_roots_for: HashMap<String, Vec<String>> =
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
        // Dedupe roots while preserving v3.2.0 deterministic order.
        let mut seen = std::collections::HashSet::new();
        let mut roots: Vec<String> = Vec::new();
        for parse in &parses {
            let r = match parse {
                Analysis::Noun { root, .. } | Analysis::Verb { root, .. } => root.root.clone(),
            };
            if seen.insert(r.clone()) {
                roots.push(r);
            }
        }
        unique_roots_for.insert(token.clone(), roots);
    }

    // Second pass: walk samples in order, tally unigrams,
    // bigrams, and root unigrams using the cache.
    let mut counts_f64: HashMap<String, f64> = HashMap::new();
    let mut bigram_counts_f64: HashMap<(String, String), f64> = HashMap::new();
    // **v4.20.0** — root counts under unambiguous-only attribution.
    let mut root_counts: HashMap<String, u64> = HashMap::new();
    let mut root_unambiguous_tokens: u64 = 0;
    let mut root_skipped_ambiguous: u64 = 0;
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
            // **v4.20.0** — root contribution: full count to the
            // single root iff the token is unambiguous; otherwise
            // skip. This keeps the root prior a true tiebreaker
            // for chain-collision pairs (the marginal P(root)
            // builds up from elsewhere in the corpus, where the
            // surface form pins the root unambiguously).
            if let Some(roots) = unique_roots_for.get(token) {
                if roots.len() == 1 {
                    *root_counts.entry(roots[0].clone()).or_insert(0) += 1;
                    root_unambiguous_tokens += 1;
                } else {
                    root_skipped_ambiguous += 1;
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
    eprintln!(
        "train_suffix_priors: root counts (unambiguous-only attribution): \
         {} distinct roots from {} unambiguous-token instances; \
         {} ambiguous-token instances skipped",
        root_counts.len(),
        root_unambiguous_tokens,
        root_skipped_ambiguous,
    );

    // **v4.20.5** — apply closed-class structural-pronoun boost.
    // Compensates for the systematic under-counting of pronouns
    // whose inflected forms are inherently ambiguous and got
    // filtered by the v4.20.0 unambiguous-only attribution.
    let boosts: ClosedClassBoosts = match fs::read(CLOSED_CLASS_BOOST_PATH) {
        Ok(b) => match serde_json::from_slice::<ClosedClassBoosts>(&b) {
            Ok(parsed) => parsed,
            Err(err) => {
                eprintln!(
                    "train_suffix_priors: closed-class boost file at {} \
                     present but malformed ({}); skipping boost",
                    CLOSED_CLASS_BOOST_PATH, err
                );
                ClosedClassBoosts::default()
            }
        },
        Err(_) => {
            eprintln!(
                "train_suffix_priors: no closed-class boost file at {} — \
                 running with pure unambiguous-only attribution",
                CLOSED_CLASS_BOOST_PATH
            );
            ClosedClassBoosts::default()
        }
    };
    if !boosts.boosts.is_empty() {
        let mut applied = 0usize;
        let mut skipped_unseen = Vec::new();
        for (root, factor) in &boosts.boosts {
            if let Some(count) = root_counts.get_mut(root) {
                let boosted = (*count as f64 * factor).round().max(1.0) as u64;
                *count = boosted;
                applied += 1;
            } else {
                skipped_unseen.push(root.clone());
            }
        }
        eprintln!(
            "train_suffix_priors: applied closed-class boost to {} roots \
             (skipped {} roots not seen unambiguously: {:?})",
            applied,
            skipped_unseen.len(),
            skipped_unseen,
        );
    }

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
    let mut priors =
        SuffixPriors::from_counts_with_bigrams_and_roots(counts, bigram_counts, root_counts);
    // Override `trained_on_tokens` with the true integer count
    // (from_counts_with_bigrams_and_roots sums the rounded unigram
    // values).
    priors.trained_on_tokens = total_tokens;

    if let Err(err) = write_priors(&priors) {
        eprintln!("train_suffix_priors: failed to write artifact: {err}");
        return ExitCode::FAILURE;
    }
    eprintln!(
        "train_suffix_priors: wrote {} (schema v{}, {} chains, {} roots)",
        OUTPUT_PATH,
        SCHEMA_VERSION,
        priors.len(),
        priors.root_log_prob.len(),
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
    json.push_str(",\n");
    // **v4.20.0** — root_log_prob (root-level marginals).
    let mut root_sorted: Vec<(&String, &f32)> = priors.root_log_prob.iter().collect();
    root_sorted.sort_by(|a, b| a.0.cmp(b.0));
    json.push_str("  \"root_log_prob\": {\n");
    for (i, (k, v)) in root_sorted.iter().enumerate() {
        let comma = if i + 1 == root_sorted.len() { "" } else { "," };
        let escaped = k.replace('\\', "\\\\").replace('"', "\\\"");
        json.push_str(&format!("    \"{escaped}\": {v}{comma}\n"));
    }
    json.push_str("  }\n");
    json.push_str("}\n");

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
