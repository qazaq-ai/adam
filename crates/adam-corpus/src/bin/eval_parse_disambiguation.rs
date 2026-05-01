//! `eval_parse_disambiguation` — empirical eval of v4.15+ FST parse-
//! selection strategies.
//!
//! **v4.19.0** — long-deferred from v4.17.5. Quantifies how often
//! each prior strategy picks the gold-correct parse on a hand-
//! curated test set of ambiguous Kazakh tokens.
//!
//! **v4.19.5** — adds the `with_context` strategy that parses the
//! full sentence (not just the isolated token) using greedy
//! bigram-aware selection mirroring v4.16.0 runtime logic. Tests
//! whether sentence context closes the «онда» (gold = `ол + Loc`
//! anaphoric, isolated-token priors pick `он + Loc` "in ten")
//! residual failure surfaced by v4.19.0.
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
//! 4. **Smoothed** — v4.16.5 Jelinek-Mercer interpolation
//!    (`α=0.3`).
//! 5. **POS-conditioned** — v4.17.0 4-tier ladder (full bigram →
//!    POS-bigram → unigram → floor). Identical to bigram on
//!    isolated tokens; gold-test is to confirm it doesn't
//!    regress.
//! 6. **With-context** — v4.19.5. Parse the full sentence
//!    greedily left-to-right with bigram-aware re-rank (mirrors
//!    `parse_input_with_priors` in `adam-dialog`), then read off
//!    the root that won for the target token. This is what the
//!    runtime *would* see if sentence-level context were plumbed
//!    into the FST selector at the `analyse()` boundary.
//! 7. **Chain+root** — v4.20.0. Additive log-prob combination:
//!    `log P(chain | prev) + log P(root)`. Tests whether root-
//!    level priors break the chain-collision tie surfaced by
//!    v4.19.5 (`он+Loc` and `ол+Loc` produce the same chain).
//!    Empirically REGRESSES on this set — a high-frequency root
//!    can override a correctly-scored chain.
//! 8. **Chain-then-root tiebreak** — v4.20.0. Sort by chain DESC;
//!    among parses tied on chain (within ε), pick the one with
//!    the highest root score. Strict tiebreaker — the principled
//!    formulation. Doesn't regress, doesn't lift on this set:
//!    онда's chain DOES collide on prior, but the root prior
//!    under unambiguous-only attribution puts он (digit, common
//!    in dates) above ол (pronoun, mostly appears in inherently-
//!    ambiguous forms that get filtered out by the attribution
//!    policy).
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

    // Run all 6 strategies per case; tally hits.
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
        let with_ctx = pick_with_sentence_context(&case.sentence, &case.token, &lex, &priors, 0.3)
            .unwrap_or_else(|| baseline.clone());
        let chain_plus_root = best_under(&parses, |a| score_chain_plus_root(a, &priors, 0.3));
        let chain_tiebreak_root = pick_chain_with_root_tiebreak(&parses, &priors, 0.3);

        let strategies = [
            ("baseline", baseline),
            ("unigram", unigram),
            ("bigram", bigram),
            ("smoothed", smoothed),
            ("pos_conditioned", pos_cond),
            ("with_context", with_ctx),
            ("chain_plus_root", chain_plus_root),
            ("chain_tiebreak_root", chain_tiebreak_root),
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
    println!("=== Parse-disambiguation eval (v4.21.0) ===");
    println!("Cases with non-empty FST parses: {total_with_parses}");
    println!();
    println!("Per-strategy accuracy:");
    for name in [
        "baseline",
        "unigram",
        "bigram",
        "smoothed",
        "pos_conditioned",
        "with_context",
        "chain_plus_root",
        "chain_tiebreak_root",
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
        let ctx = row.get("with_context").map(String::as_str).unwrap_or("?");
        let cpr = row
            .get("chain_plus_root")
            .map(String::as_str)
            .unwrap_or("?");
        let ctr = row
            .get("chain_tiebreak_root")
            .map(String::as_str)
            .unwrap_or("?");
        let unique: std::collections::HashSet<&str> =
            [baseline, unigram, bigram, smoothed, pos, ctx, cpr, ctr]
                .into_iter()
                .collect();
        let marker = if unique.len() == 1 { "  =" } else { "  ⚠" };
        println!(
            "  {marker} {id:<40} base={baseline} uni={unigram} bi={bigram} sm={smoothed} pos={pos} ctx={ctx} cpr={cpr} ctr={ctr}"
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

/// **v4.19.5** — Walk the sentence left-to-right, greedily picking
/// each token's parse under bigram-aware Jelinek-Mercer smoothed
/// scoring (mirrors `parse_input_inner` in `adam-dialog`). Track
/// `prev_chain` across tokens. When we hit `target_token`, return
/// the root that won.
///
/// Tokenization mirrors the dialog runtime: split on whitespace,
/// keep alphabetic + `-`, lowercase. Returns `None` if the target
/// isn't found in the sentence (shouldn't happen for our eval set).
fn pick_with_sentence_context(
    sentence: &str,
    target_token: &str,
    lex: &LexiconV1,
    priors: &SuffixPriors,
    alpha: f32,
) -> Option<String> {
    let target_clean: String = target_token
        .chars()
        .filter(|c| c.is_alphabetic() || *c == '-')
        .collect::<String>()
        .to_lowercase();
    let mut prev_chain: Option<String> = None;
    for token in sentence.split_whitespace() {
        let cleaned: String = token
            .chars()
            .filter(|c| c.is_alphabetic() || *c == '-')
            .collect::<String>()
            .to_lowercase();
        if cleaned.is_empty() {
            continue;
        }
        let mut analyses = analyse(&cleaned, lex);
        if analyses.is_empty() {
            // Sentence boundary — reset context (matches runtime).
            prev_chain = None;
            continue;
        }
        let prev = prev_chain.as_deref();
        analyses.sort_by(|a, b| {
            let sa = score_smoothed_with_prev(a, priors, prev, alpha);
            let sb = score_smoothed_with_prev(b, priors, prev, alpha);
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });
        let chosen = analyses.first()?;
        let chosen_root = root_of(chosen);
        let chosen_key = match chosen {
            Analysis::Noun { features, .. } => noun_chain_key(features),
            Analysis::Verb { features, .. } => verb_chain_key(features),
        };
        if cleaned == target_clean {
            return Some(chosen_root);
        }
        prev_chain = Some(chosen_key);
    }
    None
}

fn score_smoothed_with_prev(
    parse: &Analysis,
    priors: &SuffixPriors,
    prev: Option<&str>,
    alpha: f32,
) -> f32 {
    match parse {
        Analysis::Noun { features, .. } => priors.score_noun_smoothed(features, prev, alpha),
        Analysis::Verb { features, .. } => priors.score_verb_smoothed(features, prev, alpha),
    }
}

/// **v4.20.0** — combined chain + root score. The two log-prob
/// terms add to give a joint score on the probability scale:
/// `score = log P(chain) + log P(root)`. When two parses share
/// the same chain (the chain-collision case surfaced by v4.19.5),
/// the chain term is identical and the root term decides — which
/// is exactly the tiebreaker the new prior axis is meant to
/// provide.
fn score_chain_plus_root(parse: &Analysis, priors: &SuffixPriors, alpha: f32) -> f32 {
    let chain_score = match parse {
        Analysis::Noun { features, .. } => priors.score_noun_smoothed(features, None, alpha),
        Analysis::Verb { features, .. } => priors.score_verb_smoothed(features, None, alpha),
    };
    let root = match parse {
        Analysis::Noun { root, .. } | Analysis::Verb { root, .. } => &root.root,
    };
    chain_score + priors.score_root(root)
}

/// **v4.20.0** — root-as-strict-tiebreaker. Sort parses by chain
/// score DESC; among parses tied on chain score (within `EPSILON`),
/// pick the one with the highest root score. Avoids the
/// `chain_plus_root` regression where a high-frequency root can
/// override a chain-score difference. This is the principled
/// formulation: when chains differ, the chain wins (so v4.16.0's
/// bigram-aware re-rank stays in charge); when chains collide
/// (v4.19.5's blind spot), the root term decides.
fn pick_chain_with_root_tiebreak(parses: &[Analysis], priors: &SuffixPriors, alpha: f32) -> String {
    if parses.is_empty() {
        return String::new();
    }
    // **v4.20.5** — env-var `ADAM_DEBUG_TIEBREAK=1` dumps per-parse
    // scores. Useful for understanding why a specific ambiguity
    // isn't resolving as expected.
    let debug = std::env::var("ADAM_DEBUG_TIEBREAK").as_deref() == Ok("1");
    const EPSILON: f32 = 1e-4;
    let mut indexed: Vec<(usize, f32, f32, String, String)> = parses
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let chain_key = match p {
                Analysis::Noun { features, .. } => noun_chain_key(features),
                Analysis::Verb { features, .. } => verb_chain_key(features),
            };
            let chain_score = match p {
                Analysis::Noun { features, .. } => {
                    priors.score_noun_smoothed(features, None, alpha)
                }
                Analysis::Verb { features, .. } => {
                    priors.score_verb_smoothed(features, None, alpha)
                }
            };
            let root = match p {
                Analysis::Noun { root, .. } | Analysis::Verb { root, .. } => root.root.clone(),
            };
            let root_score = priors.score_root(&root);
            (i, chain_score, root_score, root, chain_key)
        })
        .collect();
    if debug {
        eprintln!("--- chain_tiebreak_root debug ---");
        for entry in &indexed {
            eprintln!(
                "  parse[{}] root={} chain={} chain_score={:.4} root_score={:.4}",
                entry.0, entry.3, entry.4, entry.1, entry.2
            );
        }
    }
    // Stable sort by chain DESC; ties broken by root DESC.
    indexed.sort_by(|a, b| {
        let chain_diff = (a.1 - b.1).abs();
        if chain_diff < EPSILON {
            b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal)
        } else {
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        }
    });
    if debug {
        eprintln!("  picked: parse[{}] root={}", indexed[0].0, indexed[0].3);
    }
    root_of(&parses[indexed[0].0])
}
