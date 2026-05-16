// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `verifier_demo` — L5.5 → L6 integration prototype.
//!
//! Demonstrates the v6.0 verifier path specified in
//! `docs/architecture_neural_v6.md` §3.2 (verifier check) end-to-
//! end:
//!
//!   neural output (morpheme sequence)
//!     ↓ detokenise  (adam-agg-tokenizer)
//!   surface word
//!     ↓ re-tokenise (FST round-trip check)
//!   morpheme sequence'
//!     ↓ compare to original  → FST-valid?
//!     ↓ look up root in facts.json  → factually-grounded?
//!   PASS / BLOCK
//!
//! Two safety gates:
//!
//! 1. **FST round-trip.** Detokenise the neural output, re-tokenise
//!    through `AggTokenizer`, and compare. If the sequence doesn't
//!    survive the round-trip, the model produced a string that the
//!    deterministic morphology doesn't recognise → BLOCK.
//!
//! 2. **Factual grounding.** Take the root from the round-tripped
//!    analysis and check that it appears as a `subject.root` or
//!    `object.root` in `data/retrieval/facts.json`. If not, the
//!    model emitted a word for which the knowledge graph has no
//!    referent → BLOCK in the strict mode (which is the v6.0
//!    contract; permissive mode is opt-in for non-factual replies).
//!
//! The binary runs the gate on a hand-picked set of (subject, root)
//! pairs drawn from facts.json and a control set of nonsense
//! sequences, then prints the audit table.
//!
//! This is a **prototype** of the L5.5→L6 wiring. Production
//! integration (v6.0.0) will live in `crates/adam-dialog/src/`
//! alongside the existing dialog-layer verifier.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use adam_agg_synth::TrainingPair;
use adam_agg_tokenizer::{AggTokenizer, MorphToken};
use adam_kernel_fst::lexicon::LexiconV1;
use serde::Deserialize;

#[derive(Deserialize)]
struct FactsFile {
    facts: Vec<FactEntry>,
}

#[derive(Deserialize)]
struct FactEntry {
    subject: FactNoun,
    object: FactNoun,
}

#[derive(Deserialize)]
struct FactNoun {
    #[serde(default)]
    surface: String,
    #[serde(default)]
    root: String,
}

/// Per-input audit record. Mirrors the production `NeuralCallRecord`
/// shape from architecture_neural_v6 §3.3 in spirit, simplified for
/// the prototype.
#[derive(Debug)]
struct AuditRecord {
    /// Input that arrived at L5.5 (here, just a surface word the
    /// caller asks the verifier to gate).
    input_surface: String,
    /// FST round-trip succeeded?
    fst_valid: bool,
    /// FST-extracted root, if any.
    root: Option<String>,
    /// Root appears in facts.json as subject or object?
    grounded: bool,
    /// Final verdict: PASS only if both gates passed.
    verdict: Verdict,
}

#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    Pass,
    Block(&'static str),
}

fn load_facts_index(path: &str) -> HashSet<String> {
    let bytes = fs::read(path).expect("read facts.json");
    let parsed: FactsFile = serde_json::from_slice(&bytes).expect("parse facts.json");
    let mut idx: HashSet<String> = HashSet::new();
    for f in &parsed.facts {
        if !f.subject.root.is_empty() {
            idx.insert(f.subject.root.clone());
        }
        if !f.object.root.is_empty() {
            idx.insert(f.object.root.clone());
        }
        // Also index surfaces so we can match against round-tripped output.
        if !f.subject.surface.is_empty() {
            idx.insert(f.subject.surface.clone());
        }
        if !f.object.surface.is_empty() {
            idx.insert(f.object.surface.clone());
        }
    }
    idx
}

fn verify(tokenizer: &AggTokenizer, facts_idx: &HashSet<String>, surface: &str) -> AuditRecord {
    // Gate 1: FST round-trip.
    let tokens = tokenizer.tokenize_word(surface);
    let head_token = tokens.first();
    let root = match head_token {
        Some(MorphToken::Root { root, .. }) => Some(root.clone()),
        _ => None,
    };

    let detok = tokenizer.detokenize_word(&tokens).ok();
    let fst_valid = match &detok {
        Some(s) => s == surface,
        None => false,
    };

    if !fst_valid {
        return AuditRecord {
            input_surface: surface.to_string(),
            fst_valid: false,
            root,
            grounded: false,
            verdict: Verdict::Block("FST round-trip failed"),
        };
    }
    // Gate 2: factual grounding.
    let grounded = match &root {
        Some(r) => facts_idx.contains(r) || facts_idx.contains(surface),
        None => facts_idx.contains(surface),
    };

    let verdict = if !grounded {
        Verdict::Block("ungrounded in facts.json")
    } else {
        Verdict::Pass
    };
    AuditRecord {
        input_surface: surface.to_string(),
        fst_valid: true,
        root,
        grounded,
        verdict,
    }
}

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
    let facts_idx = load_facts_index(facts_path);
    eprintln!(
        "[verifier_demo] Lexicon + tokenizer ready; facts.json indexed: {} roots/surfaces",
        facts_idx.len()
    );

    // Inputs:
    //   - real grounded words drawn from facts.json
    //   - real Kazakh words NOT in facts.json (FST-valid but ungrounded)
    //   - nonsense strings (FST round-trip fails)
    let cases: &[(&str, &str)] = &[
        // Each pair is (label, surface to verify).
        ("grounded subject", "адам"),
        ("grounded inflected", "адамға"),
        ("grounded object", "дүние"),
        ("ungrounded but FST-valid", "бала"),
        ("ungrounded inflected", "балалардың"),
        ("nonsense — FST-invalid", "blarg"),
        ("nonsense — Cyrillic noise", "ққққққ"),
        ("loanword", "компьютер"),
    ];

    let records: Vec<AuditRecord> = cases
        .iter()
        .map(|(_, surface)| verify(&tokenizer, &facts_idx, surface))
        .collect();

    println!();
    println!(
        "{:<32}  {:<24}  {:<6}  {:<14}  {:<10}  {}",
        "label", "input", "fst✓", "root", "grounded", "verdict"
    );
    println!("{}", "-".repeat(110));
    for ((label, _), r) in cases.iter().zip(records.iter()) {
        println!(
            "{:<32}  {:<24}  {:<6}  {:<14}  {:<10}  {:?}",
            label,
            r.input_surface,
            if r.fst_valid { "yes" } else { "no" },
            r.root.as_deref().unwrap_or("—"),
            if r.grounded { "yes" } else { "no" },
            r.verdict,
        );
    }

    let pass = records
        .iter()
        .filter(|r| r.verdict == Verdict::Pass)
        .count();
    let block = records.len() - pass;
    println!();
    println!(
        "Summary: {} PASS, {} BLOCK out of {} cases.",
        pass,
        block,
        records.len()
    );
    println!();
    println!("This prototype demonstrates the L5.5 → L6 path from");
    println!("architecture_neural_v6 §3.2. Production wiring (every");
    println!("adam-agg-model output routed through this gate) is the");
    println!("v6.0.0 release-blocker tracked in docs/roadmap_v6_v7.md.");

    // Optional: also probe a TrainingPair pack if present.
    if let Ok(real_pack) = std::env::var("VERIFIER_DEMO_PACK") {
        if let Ok(bytes) = fs::read(&real_pack) {
            if let Ok(pairs) = serde_json::from_slice::<Vec<TrainingPair>>(&bytes) {
                let sample = pairs.iter().take(200).collect::<Vec<_>>();
                let mut p = 0usize;
                let mut b = 0usize;
                for tp in &sample {
                    let r = verify(&tokenizer, &facts_idx, &tp.surface);
                    if r.verdict == Verdict::Pass {
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
}
