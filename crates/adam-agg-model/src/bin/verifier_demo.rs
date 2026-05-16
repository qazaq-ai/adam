// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `verifier_demo` — CLI inspector for the L6 verifier.
//!
//! Consumes the production `adam_agg_model::verifier::Verifier`
//! module (introduced 2026-05-16; see `crates/adam-agg-model/src/
//! verifier.rs`) and exercises it against a hand-picked control
//! set of surfaces. Used to sanity-check that:
//!
//!   - the verifier loads `data/retrieval/facts.json` correctly,
//!   - the FST round-trip gate fires where expected,
//!   - the factual-grounding gate fires where expected,
//!   - audit records are well-formed.
//!
//! For unit-level coverage of these properties see
//! `crates/adam-agg-model/src/verifier.rs` (4 tests) and
//! `crates/adam-agg-model/tests/verifier_integration.rs` (5 tests).
//!
//! Optional probe: set `VERIFIER_DEMO_PACK=<path>` to additionally
//! run the verifier against the first 200 surfaces of a
//! `build_real_corpus_pairs`-style pack. The exit code stays 0
//! regardless of pass/block ratios — this is a diagnostic, not a
//! gate.

use std::path::Path;

use adam_agg_model::verifier::{BlockReason, Verdict, Verifier};
use adam_agg_synth::TrainingPair;
use adam_agg_tokenizer::AggTokenizer;
use adam_kernel_fst::lexicon::LexiconV1;

fn main() {
    let curated = "data/tokenizer/segmentation_roots.json";
    let apertium = "data/lexicon_v1/apertium_imported_roots.json";
    if !Path::new(curated).exists() {
        eprintln!("Lexicon files missing; run from repo root.");
        std::process::exit(1);
    }
    let lex = LexiconV1::load(curated, apertium).expect("lexicon load");
    let tokenizer = AggTokenizer::build(lex);

    let facts_path = "data/retrieval/facts.json";
    let facts_idx = Verifier::load_facts_index(facts_path).expect("facts.json load");
    eprintln!(
        "[verifier_demo] Lexicon + tokenizer ready; facts.json indexed: {} roots/surfaces",
        facts_idx.len()
    );

    // Strict mode for the demo — every ungrounded result is a BLOCK.
    // (The production runtime defaults to permissive per architecture
    // _neural_v6 §3.2; we use strict here so the demo table shows the
    // grounding gate firing clearly.)
    let verifier = Verifier::new(tokenizer, facts_idx, true);

    let cases: &[(&str, &str)] = &[
        ("grounded subject", "адам"),
        ("grounded inflected", "адамға"),
        ("grounded object", "дүние"),
        ("ungrounded but FST-valid", "бала"),
        ("ungrounded inflected", "балалардың"),
        ("nonsense — FST-invalid", "blarg"),
        ("nonsense — Cyrillic noise", "ққққққ"),
        ("loanword", "компьютер"),
    ];

    println!();
    println!(
        "{:<32}  {:<24}  {:<6}  {:<14}  {:<10}  {}",
        "label", "input", "fst✓", "root", "grounded", "verdict"
    );
    println!("{}", "-".repeat(110));
    let mut pass = 0usize;
    let mut block = 0usize;
    for (label, surface) in cases {
        let record = verifier.check(surface);
        let (fst_ok, root, grounded, verdict_str) = match &record.verdict {
            Verdict::Pass { root, grounded, .. } => {
                pass += 1;
                ("yes", root.clone(), *grounded, "Pass".to_string())
            }
            Verdict::Block(BlockReason::FstRoundTripFailed) => {
                block += 1;
                ("no", None, false, "Block(FstRoundTripFailed)".to_string())
            }
            Verdict::Block(BlockReason::Ungrounded) => {
                block += 1;
                ("yes", None, false, "Block(Ungrounded)".to_string())
            }
        };
        println!(
            "{:<32}  {:<24}  {:<6}  {:<14}  {:<10}  {}",
            label,
            surface,
            fst_ok,
            root.as_deref().unwrap_or("—"),
            if grounded { "yes" } else { "no" },
            verdict_str,
        );
    }
    println!();
    println!(
        "Summary: {} PASS, {} BLOCK out of {} cases.",
        pass,
        block,
        cases.len()
    );

    // Optional probe — run the verifier across the first 200
    // surfaces of a real-corpus pack. Useful to spot-check the
    // gate's behaviour against an actual production sample without
    // re-tokenising every pack.
    if let Ok(real_pack) = std::env::var("VERIFIER_DEMO_PACK") {
        if let Ok(bytes) = std::fs::read(&real_pack) {
            if let Ok(pairs) = serde_json::from_slice::<Vec<TrainingPair>>(&bytes) {
                let sample = pairs.iter().take(200).collect::<Vec<_>>();
                let mut p = 0usize;
                let mut b = 0usize;
                for tp in &sample {
                    let r = verifier.check(&tp.surface);
                    if matches!(r.verdict, Verdict::Pass { .. }) {
                        p += 1;
                    } else {
                        b += 1;
                    }
                }
                println!();
                println!(
                    "Real-pack probe ({}): {} PASS, {} BLOCK out of {} sampled pairs.",
                    real_pack,
                    p,
                    b,
                    sample.len()
                );
            }
        }
    }

    println!();
    println!("This binary is the diagnostic surface of the production");
    println!("`adam_agg_model::verifier::Verifier` module. For the");
    println!("v6.0 L5.5 → L6 wiring see `docs/architecture_neural_v6.md`.");
}
