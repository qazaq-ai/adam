//! `adam-chat` — interactive REPL for the adam v4.0 Kazakh dialog pipeline.
//!
//! **Kazakh-only surface** (v1.1.0 revert). Input and output are both Kazakh.
//!
//! Capabilities at v4.0:
//!
//!   - **26 intents** — 25 conversational + Insult for polite non-engagement.
//!   - **Multi-turn session state** (`Conversation`): `name`, `age`, `city`,
//!     `occupation` persist across turns, feeding downstream templates.
//!   - **`{slot|features}` FST templates** with case / number / derivation /
//!     possessive feature tokens — no morphologically invalid form ever
//!     leaves the system.
//!   - **Retrieval fallback** (v1.6.0+) — when no intent matches, we rank
//!     the committed morpheme index by **overlap + pack_purity + length +
//!     loanword-density** and cite the top-1 sample verbatim with
//!     provenance.
//!   - **Session-aware framing** (v1.8.0) — when session has entities, the
//!     template wrapping the citation personalises automatically via
//!     `template_is_fillable`.
//!   - **Opt-in city composition** (v1.9.0, `--compose`) — rewrites city
//!     mentions inside retrieved quotes to the user's session city via
//!     FST feature-preserving synthesis. Biographical-year guarded.
//!   - **Adaptation marker** (v1.9.5) — when a swap happened, the response
//!     frame contains the «бейімд-» stem so the user can always distinguish
//!     a verbatim corpus quote from an adapted one.
//!   - **Rule-derived reasoning chains** (v2.7+) — when committed
//!     `facts.json` + `derived_facts.json` are present and the user probes
//!     a noun that matches a derivation, adam cites the chain (not a
//!     corpus quote) with the «байланыс-» trust marker. At v3.9.5 the
//!     reasoner has 5 active rules (R1 / R2 / R3 / R5 / R6 / R7); the
//!     latter two (LivesIn-via-PartOf, GoesTo-via-PartOf) turn
//!     city-level locations into country-level conclusions through
//!     curated `city PartOf country` chains.
//!   - **World Core curated knowledge** (v3.9.0+) — `data/world_core/*.jsonl`
//!     entries reviewed by `shaman` are merged into `facts.json` with
//!     `ConfidenceKind::HumanApproved`. At v3.9.5: **200 entries / 270
//!     facts** across 6 domains (astronomy, time, geography_kz,
//!     biology_basic, body_parts, society).
//!   - **Dialog closed-class sync** (v3.9.5) — `NOT_A_TOPIC` mirrors
//!     `adam_reasoning::patterns::is_closed_class`, so interrogatives
//!     like «Неліктен?» are correctly treated as function-word input
//!     rather than noun+ablative topics.
//!
//! Architecture reference: [`docs/architecture_v3.md`](../../../docs/architecture_v3.md).
//!
//! Usage:
//!   adam_chat                    — REPL with retrieval on
//!   adam_chat --once "сәлем"     — single-shot, print response (+ trace)
//!   adam_chat --trace            — REPL with full per-turn trace
//!   adam_chat --no-retrieval     — skip retrieval (v1.1.0 behaviour)
//!   adam_chat --compose          — opt into InSampleCitySwap composition
//!   adam_chat --safe             — **investor-safe reasoning mode**
//!                                  (v4.0.3): cite only derivations whose
//!                                  entire `source_chain` is rooted in
//!                                  `data/world_core/*.jsonl` (every
//!                                  supporting fact human-reviewed).
//!                                  Alias: `--curated-only`. Mirrors
//!                                  `adam_demo`'s default since v4.0.2.

use std::{
    io::{self, BufRead, Write},
    process::ExitCode,
};

use adam_dialog::{ComposeMode, Conversation, TemplateRepository};
use adam_kernel_fst::lexicon::LexiconV1;
use adam_reasoning::{Fact as ReasFact, reasoner::DerivedFact};
use adam_retrieval::MorphemeIndex;

const RETRIEVAL_INDEX_PATH: &str = "data/retrieval/morpheme_index.json";
const FACTS_PATH: &str = "data/retrieval/facts.json";
const DERIVED_FACTS_PATH: &str = "data/retrieval/derived_facts.json";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let trace = args.iter().any(|a| a == "--trace");
    let no_retrieval = args.iter().any(|a| a == "--no-retrieval");
    let compose = args.iter().any(|a| a == "--compose");
    // v4.0.3 — investor-safe chat mode. When `--safe` (or the
    // longer alias `--curated-only`) is passed, `inject_reasoning_chain`
    // only cites derivations whose full `source_chain` comes from
    // human-reviewed World Core entries. Mirrors the `adam_demo` Part 4
    // investor-safe default added in v4.0.2.
    let safe = args.iter().any(|a| a == "--safe" || a == "--curated-only");

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

    // v2.0: load the committed morpheme index by default. Skip on
    // --no-retrieval (for v1.1.0-style behaviour) or when the file is
    // absent (e.g. running in a trimmed CI checkout).
    let index = if no_retrieval {
        None
    } else {
        match load_retrieval_index() {
            Some(idx) => {
                eprintln!(
                    "adam-chat: retrieval on — {} morphemes, {} postings, {} indexed samples",
                    idx.unique_morphemes, idx.total_postings, idx.samples_indexed
                );
                Some(idx)
            }
            None => {
                eprintln!(
                    "adam-chat: retrieval index not found at {RETRIEVAL_INDEX_PATH} — falling back to v1.1.0 noun-echo"
                );
                None
            }
        }
    };

    // v2.7: load rule-derived facts + their supporting extracted
    // facts if present. When both exist, Intent::Unknown can cite a
    // reasoning chain (marked with «байланыс-») alongside retrieval.
    // Absent artefacts silently disable the path — v2.6 behaviour
    // is preserved.
    let (extracted, derived) = load_reasoning_chains();
    if !derived.is_empty() {
        eprintln!(
            "adam-chat: reasoning on — {} derived facts available ({} supporting extracted facts)",
            derived.len(),
            extracted.len(),
        );
    }

    let mut conv = Conversation::new();
    if let Some(idx) = index {
        conv = conv.with_morpheme_index(idx);
    }
    if compose {
        conv = conv.with_compose_mode(ComposeMode::InSampleCitySwap);
        eprintln!(
            "adam-chat: compose mode = InSampleCitySwap (v1.9.0 opt-in; adapted quotes marked with «бейімд-»)"
        );
    }
    if safe {
        conv = conv.with_curated_only_reasoning(true);
        eprintln!(
            "adam-chat: --safe mode — reasoning chains filtered to fully-curated (world_core-only) source chains"
        );
    }
    if !derived.is_empty() || !extracted.is_empty() {
        conv = conv.with_reasoning_chains(extracted, derived);
    }

    if let Some(pos) = args.iter().position(|a| a == "--once") {
        if let Some(input) = args.get(pos + 1) {
            run_turn(&mut conv, input, &lex, &repo, trace, turn_seed(0));
            return ExitCode::SUCCESS;
        } else {
            eprintln!("--once requires an argument");
            return ExitCode::FAILURE;
        }
    }

    eprintln!("adam-chat v4.0 — пікірлесейік! Қазақ тілінде сөйлесейік; ^D to quit.");
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
        run_turn(&mut conv, line, &lex, &repo, trace, seed);
        stdout.lock().flush().ok();
    }
    ExitCode::SUCCESS
}

fn load_retrieval_index() -> Option<MorphemeIndex> {
    let raw = std::fs::read_to_string(RETRIEVAL_INDEX_PATH).ok()?;
    let mut idx: MorphemeIndex = serde_json::from_str(&raw).ok()?;
    idx.refresh_stats();
    Some(idx)
}

/// Load the v2.7 reasoning artefacts — `facts.json` + `derived_facts.json`.
/// Silently returns empty vectors for any missing / malformed file so
/// embedders running in trimmed checkouts get v2.6-identical behaviour.
fn load_reasoning_chains() -> (Vec<ReasFact>, Vec<DerivedFact>) {
    #[derive(serde::Deserialize)]
    struct FactsFile {
        facts: Vec<ReasFact>,
    }
    #[derive(serde::Deserialize)]
    struct DerivedFile {
        derived: Vec<DerivedFact>,
    }
    let extracted = std::fs::read_to_string(FACTS_PATH)
        .ok()
        .and_then(|raw| serde_json::from_str::<FactsFile>(&raw).ok())
        .map(|f| f.facts)
        .unwrap_or_default();
    let derived = std::fs::read_to_string(DERIVED_FACTS_PATH)
        .ok()
        .and_then(|raw| serde_json::from_str::<DerivedFile>(&raw).ok())
        .map(|d| d.derived)
        .unwrap_or_default();
    (extracted, derived)
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
        // v4.0.25 — trace through the REAL runtime path via
        // `turn_with_trace`. Pre-v4.0.25 this branch manually
        // re-implemented turn() but stopped before
        // `inject_retrieval_example` / `inject_reasoning_chain`, so
        // `--trace` output was materially false for v4.0.20+ features
        // (Codex v4.0.23 re-review #2). Now trace prints the
        // post-injection intent — the exact state the planner saw.
        let (out, trace) = conv.turn_with_trace(input, lex, repo, seed);
        println!("┌─ input:    {input}");
        println!("├─ parses:   {:#?}", trace.parses);
        println!("├─ intent:   {:?}", trace.intent_after_injection);
        println!("├─ session:  {:?}", trace.session_snapshot);
        // v4.0.27 — belief snapshot (Codex v4.0.26 roadmap Phase 1).
        let d = trace.belief_digest;
        println!(
            "├─ belief:   entities={} facts={} active={} contested={} pending={} conflicts={}",
            d.entities,
            d.facts_total,
            d.facts_active,
            d.facts_contested,
            d.pending_questions,
            d.contradictions
        );
        if !trace.belief_snapshot.contradictions.is_empty() {
            for c in &trace.belief_snapshot.contradictions {
                println!(
                    "├─ belief conflict: {} {}: fact[{}] vs fact[{}] @ turn {}",
                    c.subject, c.predicate, c.fact_a_index, c.fact_b_index, c.detected_at_turn
                );
            }
        }
        for t in &trace.plan_trace {
            println!("├─ {t}");
        }
        println!("└─ output:   {out}");
    } else {
        let out = conv.turn(input, lex, repo, seed);
        println!("{out}");
    }
}

fn turn_seed(turn: u64) -> u64 {
    let mut s = turn.wrapping_mul(0x9E3779B97F4A7C15);
    s ^= s >> 33;
    s = s.wrapping_mul(0xFF51AFD7ED558CCD);
    s ^= s >> 33;
    s
}
