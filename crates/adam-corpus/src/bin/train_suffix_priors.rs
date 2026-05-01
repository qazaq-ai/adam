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
    // times. We dedupe: count token frequencies first, then run
    // analyse() once per unique form, multiplying chain
    // contributions by the form's frequency. Reduces 4.6M token
    // analyses to ~250K unique-form analyses (~20× faster on M2).
    let mut token_freq: HashMap<String, u64> = HashMap::new();
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
                // Not every JSON in `data/curated/` is a sample pack
                // (manifests / id-pack files have a different
                // shape). Skip silently when the schema doesn't
                // match instead of failing the whole training run.
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
            for token in tokenize(text) {
                *token_freq.entry(token).or_insert(0) += 1;
                total_tokens += 1;
            }
        }
    }
    eprintln!(
        "train_suffix_priors: {} samples, {} tokens, {} unique forms — analysing each unique form once",
        total_samples,
        total_tokens,
        token_freq.len()
    );

    let mut counts_f64: HashMap<String, f64> = HashMap::new();
    let mut analysed_forms: u64 = 0;
    let unique_total = token_freq.len() as u64;
    for (token, freq) in &token_freq {
        analysed_forms += 1;
        if analysed_forms % 25_000 == 0 {
            eprintln!(
                "train_suffix_priors: analysed {} / {} unique forms",
                analysed_forms, unique_total
            );
        }
        let parses = analyse(token, &lex);
        if parses.is_empty() {
            continue;
        }
        let weight = (*freq as f64) / parses.len() as f64;
        for parse in parses {
            let key = chain_key_for_analysis(&parse);
            *counts_f64.entry(key).or_insert(0.0) += weight;
        }
    }

    eprintln!(
        "train_suffix_priors: counted {} tokens across {} samples; \
         {} distinct chains observed",
        total_tokens,
        total_samples,
        counts_f64.len()
    );

    // Round float counts to integers for the SuffixPriors API
    // (which expects u64). Uniform attribution generates
    // fractional counts; integer rounding is fine since the prior
    // only cares about relative magnitudes after smoothing.
    let counts: HashMap<String, u64> = counts_f64
        .into_iter()
        .map(|(k, v)| (k, v.round().max(1.0) as u64))
        .collect();
    let mut priors = SuffixPriors::from_counts(counts);
    // Override `trained_on_tokens` with the true integer count
    // (from_counts sums the rounded values).
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
fn write_priors(priors: &SuffixPriors) -> std::io::Result<()> {
    let mut sorted: Vec<(&String, &f32)> = priors.chain_log_prob.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(b.0));

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"version\": {},\n", priors.version));
    json.push_str(&format!(
        "  \"trained_on_tokens\": {},\n",
        priors.trained_on_tokens
    ));
    json.push_str("  \"chain_log_prob\": {\n");
    for (i, (k, v)) in sorted.iter().enumerate() {
        let comma = if i + 1 == sorted.len() { "" } else { "," };
        let escaped = k.replace('\\', "\\\\").replace('"', "\\\"");
        json.push_str(&format!("    \"{escaped}\": {v}{comma}\n"));
    }
    json.push_str("  }\n");
    json.push_str("}\n");

    fs::write(OUTPUT_PATH, json)
}
