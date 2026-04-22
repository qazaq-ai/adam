//! `adam_demo` — scripted end-to-end walkthrough for investors.
//!
//! v3.0 shipped the full four-part demo: intents + retrieval +
//! composition + rule-derived reasoning. v3.0.1 is a polish pass —
//! stale banners fixed, probes tuned, claims reworded for honesty.
//!
//! Parts 1 + 2: 12 canonical conversational turns (verbatim retrieval
//! vs opt-in InSampleCitySwap). Part 3: synthetic sample showing the
//! v1.9.5 «бейімд-» adaptation marker. **Part 4:** rule-derived
//! reasoning chain showing the v2.6–v2.7 arc — adam concludes via R5
//! (shared-type relation) and cites the derivation in Kazakh prose
//! with the «байланыс-» trust marker.
//!
//! Each turn prints:
//!   • user line,
//!   • adam response,
//!   • session snapshot after the turn.
//!
//! The demo is **fully deterministic** for reproducible presentations —
//! seeds are derived from turn number (same xorshift as `adam_chat`),
//! the ranker has no rng, `compose_with_city` is a pure function, and
//! the reasoner is deterministic. Re-running the binary always prints
//! the same lines.

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
        input: "білім туралы айтшы",
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
    println!("║ adam v3.7.5 — 4-part scripted demo (intents + retrieval +   ║");
    println!("║              composition + reasoning, deterministic)        ║");
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

    // --- Part 4 (v2.9): the reasoner tells the user something no single
    //     corpus sentence states. This is the v3.0-ladder pay-off: adam
    //     doesn't just cite, it concludes. The chain has full provenance,
    //     and the «байланыс-» marker makes the inference distinguishable
    //     from a verbatim quote at the textual level alone. ---
    println!("\n────────────────────────────────────────────────────────────────");
    println!("PART 4 — rule-derived reasoning chain (v2.6 R5 + v2.7 dialog)");
    println!("         loading committed facts.json + derived_facts.json");
    println!("         reasoner produces RelatedTo derivations; dialog");
    println!("         cites them with the «байланыс-» trust marker.");
    println!("────────────────────────────────────────────────────────────────\n");
    run_reasoning_chain_demo(&lex, &repo);

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

/// Part 4: load the committed fact + derivation artefacts, attach to
/// a fresh `Conversation`, and show that adam cites a derived chain —
/// not a corpus quote — when the user's topic matches a derivation.
/// The «байланыс-» marker in the response tells the user (or a
/// reviewer) that this sentence was **reasoned**, not retrieved.
fn run_reasoning_chain_demo(lex: &LexiconV1, repo: &TemplateRepository) {
    let Some((extracted, derived)) = load_reasoning_artefacts() else {
        println!("(reasoning artefacts not found — skipping Part 4.)");
        println!("Run `cargo run -p adam-reasoning --bin extract_facts`");
        println!("    then `cargo run -p adam-reasoning --bin run_reasoner`");
        println!("to regenerate data/retrieval/facts.json + derived_facts.json.");
        return;
    };

    println!("Loaded reasoning artefacts:");
    println!("  extracted facts:      {}", extracted.len());
    println!("  rule-derived facts:   {}", derived.len());
    println!();

    if derived.is_empty() {
        println!("(no derivations in the committed artefact — Part 4 is a no-op for");
        println!(" this corpus snapshot. Add pattern coverage or extend the corpus");
        println!(" to unlock chains.)");
        return;
    }

    // v3.7.5: pick ONE representative derivation per `rule_id` so the
    // demo surfaces the full roster of active reasoning operations, not
    // four seeds of the same chain. Per rule we pick the first
    // derivation whose SUBJECT root is a genuine content noun — short
    // demonstrative / pronoun subjects (like "ана" = "that one") route
    // through a different intent path in the dialog planner and would
    // miss the reasoning hook. Filter: subject.root ≥ 4 chars + not in
    // the demo-local closed-class list below.
    const DEMO_CLOSED_CLASS: &[&str] = &[
        "ана",
        "ол",
        "сол",
        "осы",
        "бұл",
        "мына",
        "кейбір",
        "бәрі",
        "барлық",
    ];
    let subject_is_content_noun =
        |root: &str| -> bool { root.chars().count() >= 4 && !DEMO_CLOSED_CLASS.contains(&root) };
    let mut per_rule: std::collections::BTreeMap<String, &adam_reasoning::reasoner::DerivedFact> =
        std::collections::BTreeMap::new();
    for d in &derived {
        // Skip derivations whose subject is a demonstrative/closed-class
        // root — dialog planner's Unknown intent extraction won't light
        // up the reasoning chain for these. We still keep them in the
        // runtime pool (they're valid facts); we just don't probe with
        // them in the demo.
        if !subject_is_content_noun(&d.subject.root) {
            continue;
        }
        per_rule.entry(d.rule_id.clone()).or_insert(d);
    }
    println!(
        "Picking one representative derivation per rule id ({} total rules fired):",
        per_rule.len(),
    );
    for (rule, d) in &per_rule {
        println!(
            "  [{}]  {} --{}--> {}",
            rule,
            d.subject.root,
            d.predicate.as_str(),
            d.object.root,
        );
        println!("    source_chain:");
        for s in &d.source_chain {
            println!("      {} / {}", s.pack, s.sample_id);
        }
    }
    println!();

    // Fresh session so reasoning — not retrieval — is the path exercised.
    // The v2.7 priority rule in the planner puts reasoning above retrieval
    // when both are available; we don't attach a morpheme index here so
    // the contrast is visually obvious.
    let mut conv = Conversation::new().with_reasoning_chains(extracted.clone(), derived.clone());

    println!("For each rule, probing adam with «<root> туралы бірдеңе айт»:");
    println!(
        "(each probe triggers the specific rule-derived chain; «байланыс-» marker = REASONED, not RETRIEVED)"
    );
    println!();
    for (rule, d) in &per_rule {
        let probe_noun = &d.subject.root;
        let input = format!("{probe_noun} туралы бірдеңе айт");
        println!("  ── {rule} ──");
        println!("  probe: «{input}»");
        // Two seeds per rule — enough to show the same chain surfaces,
        // without repeating the previous bloat of 4 seeds × 1 rule.
        for seed_n in [1u64, 8] {
            let seed = turn_seed(seed_n);
            let out = conv.turn(&input, lex, repo, seed);
            let marker_present = out.contains("байланыс");
            println!(
                "  seed {:>2} [{}]: {out}",
                seed_n,
                if marker_present { "chain" } else { "plain" }
            );
        }
        println!();
    }
    println!(
        "NOTE: every response above containing «байланыс-» is REASONED, not RETRIEVED. The v2.7"
    );
    println!(
        "trust invariant (tested bi-directionally) guarantees the marker never appears without an"
    );
    println!(
        "actual derivation backing it — and NEVER appears on a retrieval-only answer. With {} derivations",
        derived.len()
    );
    println!(
        "in the committed runtime and {} rules active, this demo surfaces the full roster of",
        per_rule.len()
    );
    println!("cognitive operations the deterministic reasoner performs at this scale.");
}

/// Helper: load both committed artefacts together. Missing either →
/// return `None`; the caller prints a helpful message.
fn load_reasoning_artefacts() -> Option<(
    Vec<adam_reasoning::Fact>,
    Vec<adam_reasoning::reasoner::DerivedFact>,
)> {
    #[derive(serde::Deserialize)]
    struct FactsFile {
        facts: Vec<adam_reasoning::Fact>,
    }
    #[derive(serde::Deserialize)]
    struct DerivedFile {
        derived: Vec<adam_reasoning::reasoner::DerivedFact>,
    }
    let facts_raw = std::fs::read_to_string("data/retrieval/facts.json").ok()?;
    let derived_raw = std::fs::read_to_string("data/retrieval/derived_facts.json").ok()?;
    let facts: FactsFile = serde_json::from_str(&facts_raw).ok()?;
    let derived: DerivedFile = serde_json::from_str(&derived_raw).ok()?;
    Some((facts.facts, derived.derived))
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
