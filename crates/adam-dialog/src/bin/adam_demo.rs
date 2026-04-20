//! `adam_demo` — scripted v2.0 end-to-end walkthrough.
//!
//! 15 canonical turns that demonstrate, in order:
//!
//!   1. Kazakh-only greeting + response.
//!   2. Name extraction + session storage.
//!   3. City extraction via FST locative+P1Sg (v1.8.5 fix).
//!   4. Age extraction via Kazakh numeral parser.
//!   5. Occupation extraction via FST predicate-person (v1.4.0).
//!   6. Personalised greeting (session slots feed templates).
//!   7. Cross-slot template firing (requires all four entities).
//!   8. AskHowAreYou + follow-up "ал сіз?" (v1.4.0 DST).
//!   9. Retrieval on a recognised topic (Abai, «бала»).
//!  10. Session-aware retrieval frame (name + city wrap the citation).
//!  11. FST-rendered `{city|locative}` in a session-aware template.
//!  12. InSampleCitySwap composition + v1.9.5 marker.
//!  13. Biographical-year guard (1845 → swap refused).
//!  14. Insult intent — polite non-engagement, no escalation.
//!  15. Farewell.
//!
//! Each turn prints:
//!   • user line,
//!   • adam response,
//!   • session snapshot after the turn.
//!
//! The demo is **fully deterministic** for reproducible presentations —
//! seeds are derived from turn number (same xorshift as `adam_chat`),
//! the ranker has no rng, and `compose_with_city` is a pure function.
//! Re-running the binary always prints the same lines.

use std::process::ExitCode;

use adam_dialog::{ComposeMode, Conversation, TemplateRepository};
use adam_kernel_fst::lexicon::LexiconV1;
use adam_retrieval::MorphemeIndex;

const RETRIEVAL_INDEX_PATH: &str = "data/retrieval/morpheme_index.json";

/// One scripted turn.
struct Step {
    label: &'static str,
    input: &'static str,
}

const SCRIPT: &[Step] = &[
    Step {
        label: "01. Greeting (casual)",
        input: "сәлем",
    },
    Step {
        label: "02. Statement of name — entity extraction",
        input: "менің атым Дәулет",
    },
    Step {
        label: "03. Statement of location (FST loc+P1Sg, v1.8.5 fix)",
        input: "мен Алматыдамын",
    },
    Step {
        label: "04. Statement of age — Kazakh numeral parser",
        input: "менің жасым отыз",
    },
    Step {
        label: "05. Statement of occupation (FST predicate-person, v1.4.0)",
        input: "мен мұғаліммін",
    },
    Step {
        label: "06. Personalised greeting (session feeds templates)",
        input: "сәлем",
    },
    Step {
        label: "07. AskHowAreYou + follow-up 'ал сіз?' (v1.4.0 DST)",
        input: "қалайсыз",
    },
    Step {
        label: "07b. Follow-up — reflective question",
        input: "ал сіз?",
    },
    Step {
        label: "08. Retrieval on a known topic (бала → Abai)",
        input: "бала туралы бірдеңе айт",
    },
    Step {
        label: "09. Session-aware frame over retrieval (name + city + quote)",
        input: "мектеп керек пе",
    },
    Step {
        label: "10. Biographical-year guard (1845 → no swap)",
        input: "Абай жайында не дейсің",
    },
    Step {
        label: "11. Insult — polite non-engagement",
        input: "сен ақымақсың",
    },
    Step {
        label: "12. Farewell",
        input: "сау бол",
    },
];

fn main() -> ExitCode {
    let lex = match LexiconV1::load_default() {
        Ok(l) => l,
        Err(e) => {
            eprintln!("cannot load lexicon: {e}");
            return ExitCode::FAILURE;
        }
    };
    let repo = match TemplateRepository::load_default() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("cannot load templates: {e}");
            return ExitCode::FAILURE;
        }
    };
    let index = load_retrieval_index();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║ adam v2.0 — scripted demo (15 canonical turns, deterministic)║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    // --- Part 1: retrieval on, compose off (v1.8.5 verbatim behaviour) ---
    println!("────────────────────────────────────────────────────────────────");
    println!("PART 1 — retrieval ON, compose = Verbatim (default v2.0)");
    println!("────────────────────────────────────────────────────────────────\n");
    run_script(&lex, &repo, index.clone(), ComposeMode::Verbatim);

    // --- Part 2: same script under InSampleCitySwap. On this committed
    //     corpus + this session, swap rarely fires (guards refuse year-
    //     bearing biographies; non-biographical quotes rarely mention
    //     one of the 20 cities in PLACE_NAMES). That's the SAFE case —
    //     composition only kicks in when ALL guards pass. Part 3 below
    //     uses a synthetic sample to demonstrate the swap + marker path
    //     explicitly. ---
    println!("\n────────────────────────────────────────────────────────────────");
    println!("PART 2 — same script, compose = InSampleCitySwap (v1.9.0 opt-in)");
    println!("         On real corpus, guards refuse most swaps — this is the");
    println!("         safe case (v1.9.5 marker only fires when a swap actually");
    println!("         happened).");
    println!("────────────────────────────────────────────────────────────────\n");
    run_script(&lex, &repo, index, ComposeMode::InSampleCitySwap);

    // --- Part 3: synthetic sample to SHOW the swap + marker path end-to-end. ---
    println!("\n────────────────────────────────────────────────────────────────");
    println!("PART 3 — synthetic sample demonstrating swap + v1.9.5 marker");
    println!("         user is in Шымкент; retrieved sample mentions Алматыда;");
    println!("         the marker «бейімделген» tells the user the quote was");
    println!("         adapted — not the original source.");
    println!("────────────────────────────────────────────────────────────────\n");
    run_synthetic_swap_demo(&lex, &repo);

    ExitCode::SUCCESS
}

/// Part 3: inject a hand-crafted sample so the swap path is guaranteed
/// to fire (the committed corpus happens not to surface city-bearing
/// quotes for our canonical queries). Synthetic for demonstration only;
/// NOT part of the committed retrieval index.
fn run_synthetic_swap_demo(lex: &LexiconV1, repo: &TemplateRepository) {
    use adam_retrieval::SampleRef;
    let mut idx = MorphemeIndex::new();
    let sref = SampleRef {
        pack: "abai_wikisource_pack.json".into(),
        sample_id: "demo_synth_001".into(),
    };
    idx.insert("бала", sref.clone());
    idx.remember_text(&sref, "Бала Алматыда жақсы өмір сүреді");
    idx.refresh_stats();

    let mut conv = Conversation::new()
        .with_morpheme_index(idx)
        .with_compose_mode(ComposeMode::InSampleCitySwap);
    conv.session.insert("name".into(), "Дәулет".into());
    conv.session.insert("city".into(), "Шымкент".into());

    println!("Synthetic sample (pack=abai_wikisource, id=demo_synth_001):");
    println!("  «Бала Алматыда жақсы өмір сүреді»\n");
    println!("Session: {{ name=Дәулет, city=Шымкент }}\n");

    let input = "бала туралы бірдеңе айт";
    for seed_n in [1u64, 4, 8, 12, 16] {
        let seed = turn_seed(seed_n);
        let out = conv.turn(input, lex, repo, seed);
        println!("turn {seed_n:>2}: {out}");
        println!();
    }
}

fn run_script(
    lex: &LexiconV1,
    repo: &TemplateRepository,
    index: Option<MorphemeIndex>,
    compose: ComposeMode,
) {
    let mut conv = Conversation::new().with_compose_mode(compose);
    if let Some(idx) = index {
        conv = conv.with_morpheme_index(idx);
    }
    for (i, step) in SCRIPT.iter().enumerate() {
        let seed = turn_seed(i as u64 + 1);
        let out = conv.turn(step.input, lex, repo, seed);
        println!("{}", step.label);
        println!("  user : {}", step.input);
        println!("  adam : {out}");
        if !conv.session.is_empty() {
            let mut keys: Vec<&String> = conv.session.keys().collect();
            keys.sort();
            let rendered: Vec<String> = keys
                .iter()
                .map(|k| format!("{k}={}", conv.session[*k]))
                .collect();
            println!("  sess : {{ {} }}", rendered.join(", "));
        }
        println!();
    }
}

fn load_retrieval_index() -> Option<MorphemeIndex> {
    let raw = std::fs::read_to_string(RETRIEVAL_INDEX_PATH).ok()?;
    let mut idx: MorphemeIndex = serde_json::from_str(&raw).ok()?;
    idx.refresh_stats();
    Some(idx)
}

fn turn_seed(turn: u64) -> u64 {
    let mut s = turn.wrapping_mul(0x9E3779B97F4A7C15);
    s ^= s >> 33;
    s = s.wrapping_mul(0xFF51AFD7ED558CCD);
    s ^= s >> 33;
    s
}
