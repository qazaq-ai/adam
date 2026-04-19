//! `adam-chat` — interactive REPL demo of the predictable Kazakh dialog
//! pipeline (v0.9.7 MVP — 25 intents + session state + FST slot expansion
//! + trilingual input + Lexicon-backed occupation extraction).
//!
//! Usage:
//!   adam_chat                — interactive REPL on stdin
//!   adam_chat --once "сәлем" — single-shot, print response + trace
//!   adam_chat --trace        — REPL with full Layer 1..5 trace per turn
//!
//! The REPL holds a single [`Conversation`] for the whole session, so
//! the user's name (once said) persists across turns: subsequent
//! greetings personalise automatically.

use std::{
    io::{self, BufRead, Write},
    process::ExitCode,
};

use adam_dialog::{
    Conversation, TemplateRepository, interpret_text_with_lexicon, plan_response_with_session,
    realise,
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
            let mut conv = Conversation::new();
            run_turn(&mut conv, input, &lex, &repo, trace, turn_seed(0));
            return ExitCode::SUCCESS;
        } else {
            eprintln!("--once requires an argument");
            return ExitCode::FAILURE;
        }
    }

    eprintln!(
        "adam-chat v0.9.7 — пікірлесейік! Type a sentence in Kazakh / Russian / English; ^D to quit."
    );
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut conv = Conversation::new();
    let mut turn = 0u64;
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        turn += 1;
        let seed = turn_seed(turn);
        run_turn(&mut conv, line, &lex, &repo, trace, seed);
        stdout.lock().flush().ok();
    }
    ExitCode::SUCCESS
}

fn run_turn(
    conv: &mut Conversation,
    input: &str,
    lex: &LexiconV1,
    repo: &TemplateRepository,
    trace: bool,
    seed: u64,
) {
    if trace {
        // Trace mode has to duplicate Conversation::turn so we can
        // surface intermediate state. Functionally identical.
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
        let intent = interpret_text_with_lexicon(input, &parses, Some(lex));
        // Fold entities BEFORE planning so "менің атым X" immediately
        // allows the very same turn's response to reference {name}.
        absorb_into(conv, &intent);
        let plan = plan_response_with_session(&intent, seed, repo, &conv.session);
        let out = realise(&plan);
        println!("┌─ input:    {input}");
        println!("├─ parses:   {parses:#?}");
        println!("├─ intent:   {intent:?}");
        println!("├─ session:  {:?}", conv.session);
        for t in &plan.trace {
            println!("├─ {t}");
        }
        println!("└─ output:   {out}");
    } else {
        let out = conv.turn(input, lex, repo, seed);
        println!("{out}");
    }
}

/// Mirror of `Conversation::absorb_entities` for the --trace path
/// (external binary can't call pub(crate)). Keep in lockstep with the
/// in-crate version when adding new entity types.
fn absorb_into(conv: &mut Conversation, intent: &adam_dialog::Intent) {
    use adam_dialog::Intent;
    match intent {
        Intent::StatementOfName { name } => {
            conv.session.insert("name".into(), name.clone());
        }
        Intent::StatementOfAge { years: Some(years) } => {
            conv.session.insert("age".into(), years.to_string());
        }
        Intent::StatementOfLocation { city: Some(city) } => {
            conv.session.insert("city".into(), city.clone());
        }
        Intent::StatementOfOccupation {
            occupation: Some(occupation),
        } => {
            conv.session.insert("occupation".into(), occupation.clone());
        }
        _ => {}
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
