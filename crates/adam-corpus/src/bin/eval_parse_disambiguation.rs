//! `eval_parse_disambiguation` — empirical eval of v4.15+ FST parse-
//! selection strategies.
//!
//! **v4.19.0** — long-deferred from v4.17.5. Quantifies how often
//! each prior strategy picks the gold-correct parse on a hand-
//! curated test set of ambiguous Kazakh tokens.
//!
//! **Strategies measured.**
//!
//! 1. **Baseline** — v3.2.0 deterministic lexicographic order
//!    (no priors). The first parse from `analyse()`.
//! 2. **Unigram** — v4.15.5 priors. Parses sorted by `P(chain)`.
//! 3. **Bigram** — v4.16.0 priors. Parses sorted by
//!    `P(chain | prev_chain)` (here prev = `None` since each test
//!    case is a single isolated token, so this collapses to
//!    unigram-with-row-floor — included anyway as a sanity check
//!    that the bigram path doesn't regress vs unigram on
//!    no-context queries).
//! 3. **Smoothed** — v4.16.5 Jelinek-Mercer interpolation
//!    (`α=0.3`).
//! 4. **POS-conditioned** — v4.17.0 4-tier ladder (full bigram →
//!    POS-bigram → unigram → floor). Identical to bigram on
//!    isolated tokens; gold-test is to confirm it doesn't
//!    regress.
//!
//! **Test set.** `data/eval/parse_disambiguation_eval.json` —
//! 20 hand-labeled cases drawn from past live-REPL bugs and
//! cross-domain ambiguous nouns. Each case has a sentence (for
//! human context), the ambiguous token, and the gold root.
//!
//! **Output.** Per-strategy accuracy + per-case detail.
//!
//! **What this does NOT measure.** End-to-end dialog quality —
//! that's what REPL replay locks. This binary isolates parse-
//! selection ONLY: did the right ROOT win for an ambiguous
//! surface? Downstream `noun_hint` filtering / template
//! selection are separate layers.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use adam_kernel_fst::lexicon::LexiconV1;
use adam_kernel_fst::parser::{Analysis, analyse};
use adam_kernel_fst::suffix_priors::{SuffixPriors, noun_chain_key, verb_chain_key};
use serde::Deserialize;

const EVAL_PATH: &str = "data/eval/parse_disambiguation_eval.json";
const PRIORS_PATH: &str = "data/retrieval/suffix_chain_priors.json";

#[derive(Debug, Deserialize)]
struct EvalFile {
    cases: Vec<EvalCase>,
}

#[derive(Debug, Deserialize, Clone)]
struct EvalCase {
    id: String,
    #[allow(dead_code)]
    sentence: String,
    token: String,
    gold_root: String,
    #[allow(dead_code)]
    #[serde(default)]
    note: String,
}

fn main() -> ExitCode {
    let eval_path = Path::new(EVAL_PATH);
    if !eval_path.exists() {
        eprintln!("eval file not found: {}", eval_path.display());
        return ExitCode::FAILURE;
    }
    let bytes = match fs::read(eval_path) {
        Ok(b) => b,
        Err(err) => {
            eprintln!("read eval file: {err}");
            return ExitCode::FAILURE;
        }
    };
    let eval: EvalFile = match serde_json::from_slice(&bytes) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("parse eval file: {err}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!(
        "eval_parse_disambiguation: loaded {} cases from {}",
        eval.cases.len(),
        EVAL_PATH
    );

    let lex = match LexiconV1::load_default() {
        Ok(l) => l,
        Err(err) => {
            eprintln!("lexicon load: {err:?}");
            return ExitCode::FAILURE;
        }
    };

    let priors = match SuffixPriors::load(PRIORS_PATH) {
        Ok(p) => p,
        Err(err) => {
            eprintln!("priors load: {err}; running with empty priors");
            SuffixPriors::empty()
        }
    };

    // Run all 5 strategies per case; tally hits.
    let mut hits: HashMap<&'static str, usize> = HashMap::new();
    let mut total_with_parses = 0usize;
    let mut per_case: Vec<(String, HashMap<&'static str, String>)> = Vec::new();
    for case in &eval.cases {
        let parses = analyse(&case.token, &lex);
        if parses.is_empty() {
            // Empty parse list — strategies all produce no prediction.
            // Skip from accuracy denominator.
            eprintln!(
                "  [skip] {}: token «{}» — analyse() returned no parses",
                case.id, case.token
            );
            continue;
        }
        total_with_parses += 1;

        let baseline = parses.first().map(root_of).unwrap_or_default();
        let unigram = best_under(&parses, |a| score_unigram(a, &priors));
        let bigram = best_under(&parses, |a| score_bigram_no_context(a, &priors));
        let smoothed = best_under(&parses, |a| score_smoothed_no_context(a, &priors, 0.3));
        let pos_cond = best_under(&parses, |a| score_pos_conditioned_no_context(a, &priors));

        let strategies = [
            ("baseline", baseline),
            ("unigram", unigram),
            ("bigram", bigram),
            ("smoothed", smoothed),
            ("pos_conditioned", pos_cond),
        ];
        let mut row: HashMap<&'static str, String> = HashMap::new();
        for (name, picked) in &strategies {
            let correct = picked == &case.gold_root;
            if correct {
                *hits.entry(name).or_insert(0) += 1;
            }
            row.insert(*name, picked.clone());
        }
        per_case.push((case.id.clone(), row));
    }

    // Report.
    println!("=== Parse-disambiguation eval (v4.19.0) ===");
    println!("Cases with non-empty FST parses: {total_with_parses}");
    println!();
    println!("Per-strategy accuracy:");
    for name in [
        "baseline",
        "unigram",
        "bigram",
        "smoothed",
        "pos_conditioned",
    ] {
        let h = hits.get(name).copied().unwrap_or(0);
        let pct = if total_with_parses == 0 {
            0.0
        } else {
            100.0 * h as f64 / total_with_parses as f64
        };
        println!("  {name:<18} {h}/{total_with_parses}  ({pct:.1}%)");
    }
    println!();
    println!("Per-case (= when all strategies agree, ⚠ when they diverge):");
    for (id, row) in &per_case {
        let baseline = row.get("baseline").map(String::as_str).unwrap_or("?");
        let unigram = row.get("unigram").map(String::as_str).unwrap_or("?");
        let bigram = row.get("bigram").map(String::as_str).unwrap_or("?");
        let smoothed = row.get("smoothed").map(String::as_str).unwrap_or("?");
        let pos = row
            .get("pos_conditioned")
            .map(String::as_str)
            .unwrap_or("?");
        let unique: std::collections::HashSet<&str> = [baseline, unigram, bigram, smoothed, pos]
            .into_iter()
            .collect();
        let marker = if unique.len() == 1 { "  =" } else { "  ⚠" };
        println!(
            "  {marker} {id:<40} base={baseline} uni={unigram} bi={bigram} sm={smoothed} pos={pos}"
        );
    }
    ExitCode::SUCCESS
}

fn root_of(parse: &Analysis) -> String {
    match parse {
        Analysis::Noun { root, .. } | Analysis::Verb { root, .. } => root.root.clone(),
    }
}

fn best_under(parses: &[Analysis], score: impl Fn(&Analysis) -> f32) -> String {
    if parses.is_empty() {
        return String::new();
    }
    let mut best_idx = 0usize;
    let mut best_score = score(&parses[0]);
    for (i, p) in parses.iter().enumerate().skip(1) {
        let s = score(p);
        if s > best_score {
            best_score = s;
            best_idx = i;
        }
    }
    root_of(&parses[best_idx])
}

fn score_unigram(parse: &Analysis, priors: &SuffixPriors) -> f32 {
    match parse {
        Analysis::Noun { features, .. } => priors.score_noun(features),
        Analysis::Verb { features, .. } => priors.score_verb(features),
    }
}

fn score_bigram_no_context(parse: &Analysis, priors: &SuffixPriors) -> f32 {
    match parse {
        Analysis::Noun { features, .. } => priors.score_noun_given_prev(features, None),
        Analysis::Verb { features, .. } => priors.score_verb_given_prev(features, None),
    }
}

fn score_smoothed_no_context(parse: &Analysis, priors: &SuffixPriors, alpha: f32) -> f32 {
    match parse {
        Analysis::Noun { features, .. } => priors.score_noun_smoothed(features, None, alpha),
        Analysis::Verb { features, .. } => priors.score_verb_smoothed(features, None, alpha),
    }
}

fn score_pos_conditioned_no_context(parse: &Analysis, priors: &SuffixPriors) -> f32 {
    // No prev_chain to condition on for isolated tokens — the
    // POS-conditioned tier collapses to bigram-with-fallback,
    // which itself collapses to unigram. Included for parity in
    // the comparison output.
    let chain = match parse {
        Analysis::Noun { features, .. } => noun_chain_key(features),
        Analysis::Verb { features, .. } => verb_chain_key(features),
    };
    priors.score_chain_given_prev(&chain, None)
}
