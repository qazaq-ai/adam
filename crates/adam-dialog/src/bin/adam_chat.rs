//! `adam-chat` — interactive REPL demo of the predictable Kazakh dialog
//! pipeline (v0.7.5 MVP — 10 intents from TOML templates).
//!
//! Usage:
//!   adam_chat                — interactive REPL on stdin
//!   adam_chat --once "сәлем" — single-shot, print response + trace
//!   adam_chat --trace        — REPL with full Layer 1..5 trace per turn
//!
//! The REPL is intentionally minimal — no history, no config. It exists
//! so that the v0.7.5 artifact is runnable and demonstrable without
//! writing Rust.

use std::{
    io::{self, BufRead, Write},
    process::ExitCode,
};

use adam_dialog::{
    TemplateRepository, interpret_text, plan_response_with_repo, realise, respond_with_repo,
};
use adam_kernel_fst::lexicon::LexiconV1;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let trace = args.iter().any(|a| a == "--trace");

    let lex = match LexiconV1::load_default() {
        Ok(l) => l,
        Err(e) => {
            eprintln!("cannot load lexicon: {e}");
            return ExitCode::FAILURE;
        }
    };

    let repo = match TemplateRepository::load_default() {
        Ok(r) => {
            eprintln!(
                "adam-chat: loaded {} template families from data/dialog/templates/v1.toml",
                r.len()
            );
            r
        }
        Err(e) => {
            eprintln!("warning: using hardcoded fallback templates ({e})");
            TemplateRepository::hardcoded_fallback()
        }
    };

    if let Some(pos) = args.iter().position(|a| a == "--once") {
        if let Some(input) = args.get(pos + 1) {
            run_once(input, &lex, &repo, trace, turn_seed(0));
            return ExitCode::SUCCESS;
        } else {
            eprintln!("--once requires an argument");
            return ExitCode::FAILURE;
        }
    }

    eprintln!("adam-chat v0.7.5 — пікірлесейік! Type a Kazakh sentence; ^D to quit.");
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut turn = 0u64;
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        turn += 1;
        let seed = turn_seed(turn);
        run_once(line, &lex, &repo, trace, seed);
        stdout.lock().flush().ok();
    }
    ExitCode::SUCCESS
}

fn run_once(input: &str, lex: &LexiconV1, repo: &TemplateRepository, trace: bool, seed: u64) {
    if trace {
        // Rebuild the pipeline in pieces so we can show the intermediate
        // states — this is the "predictable by construction" property
        // made visible.
        let parses: Vec<_> = input
            .split_whitespace()
            .flat_map(|t| {
                let cleaned: String = t
                    .chars()
                    .filter(|c| c.is_alphabetic() || *c == '-')
                    .collect::<String>()
                    .to_lowercase();
                adam_kernel_fst::parser::analyse(&cleaned, lex)
                    .into_iter()
                    .next()
            })
            .collect();
        let intent = interpret_text(input, &parses);
        let plan = plan_response_with_repo(&intent, seed, repo);
        let out = realise(&plan);
        println!("┌─ input:   {input}");
        println!("├─ parses:  {parses:#?}");
        println!("├─ intent:  {intent:?}");
        for t in &plan.trace {
            println!("├─ {t}");
        }
        println!("└─ output:  {out}");
    } else {
        let out = respond_with_repo(input, lex, repo, seed);
        println!("{out}");
    }
}

/// Seed derivation from a turn number. Keeps the chat reproducible if
/// someone wants to replay a specific session.
fn turn_seed(turn: u64) -> u64 {
    // xorshift-style mix so consecutive turns pick diverse templates.
    let mut s = turn.wrapping_mul(0x9E3779B97F4A7C15);
    s ^= s >> 33;
    s = s.wrapping_mul(0xFF51AFD7ED558CCD);
    s ^= s >> 33;
    s
}
