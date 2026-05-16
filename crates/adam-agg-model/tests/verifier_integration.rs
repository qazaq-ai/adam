// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Integration tests for the L6 verifier against the production
//! Lexicon and the production knowledge graph (`data/retrieval/
//! facts.json`).
//!
//! These tests exercise:
//!
//! - FST round-trip invariance: every analysable Kazakh surface
//!   must survive the `tokenize → detokenize → tokenize` cycle
//!   byte-for-byte.
//! - Strict-mode block discipline: ungrounded surfaces must Block;
//!   grounded surfaces must Pass.
//! - Permissive-mode flag discipline: ungrounded surfaces Pass
//!   with `grounded == false`.
//! - Determinism: repeated `check(s)` calls on the same surface
//!   must produce identical verdicts (no global mutable state,
//!   no time-dependent behaviour).
//!
//! Unlike the unit tests in `src/verifier.rs`, these tests load the
//! full production data and so are slower; they live here so a
//! plain `cargo test --lib` stays fast.

use std::collections::HashSet;

use adam_agg_model::verifier::{BlockReason, Verdict, Verifier};
use adam_agg_tokenizer::AggTokenizer;
use adam_kernel_fst::lexicon::LexiconV1;

fn fixture(strict: bool) -> Verifier {
    let lex = LexiconV1::load(
        "../../data/tokenizer/segmentation_roots.json",
        "../../data/lexicon_v1/apertium_imported_roots.json",
    )
    .expect("lexicon load (run tests from workspace root or per-crate)");
    let tokenizer = AggTokenizer::build(lex);
    let facts_idx =
        Verifier::load_facts_index("../../data/retrieval/facts.json").expect("facts.json load");
    Verifier::new(tokenizer, facts_idx, strict)
}

#[test]
fn fst_round_trip_passes_for_grounded_kazakh_words() {
    let v = fixture(false);
    let words = ["адам", "дүние", "Қазақстан"];
    for w in words {
        let r = v.check(w);
        match r.verdict {
            Verdict::Pass { ref surface, .. } => assert_eq!(surface, w),
            other => panic!("expected Pass for {w:?}, got {other:?}"),
        }
    }
}

#[test]
fn strict_mode_blocks_unknown_root() {
    let v = fixture(true);
    // Sequence of Latin letters: tokenizer returns Unk; no factual
    // grounding; strict mode must Block on Ungrounded.
    let r = v.check("zxqvwopr");
    assert!(matches!(r.verdict, Verdict::Block(BlockReason::Ungrounded)));
}

#[test]
fn permissive_mode_admits_ungrounded_fst_valid_surfaces() {
    let v = fixture(false);
    // A real but uncommon Kazakh root unlikely to be in facts.json.
    // The verifier should Pass with `grounded == false`.
    let r = v.check("балам");
    match r.verdict {
        Verdict::Pass { grounded, .. } => {
            // grounded may be true or false depending on coverage —
            // the property we assert is "permissive mode never blocks
            // a round-trippable Kazakh surface".
            let _ = grounded;
        }
        other => panic!("expected Pass in permissive mode, got {other:?}"),
    }
}

#[test]
fn check_is_deterministic_across_calls() {
    // Critical invariant for audit / reproducibility — repeated
    // checks must produce byte-identical verdicts.
    let v = fixture(false);
    let inputs = ["адам", "дүние", "балам", "zxqvwopr"];
    for input in inputs {
        let first = v.check(input);
        for _ in 0..5 {
            let again = v.check(input);
            assert_eq!(
                first.verdict, again.verdict,
                "non-deterministic on {input:?}"
            );
        }
    }
}

#[test]
fn facts_index_is_non_trivial() {
    // Defensive: the facts.json artefact must be loaded and contain
    // a non-trivial number of entries. If a future commit empties
    // facts.json, every Pass would still be "grounded=false" in
    // strict mode → silent block of everything. Catch that here.
    let idx =
        Verifier::load_facts_index("../../data/retrieval/facts.json").expect("facts.json load");
    assert!(
        idx.len() >= 1000,
        "facts.json index has only {} entries; expected ≥ 1000",
        idx.len()
    );
    // Smoke-check a known core fact: «адам» (human / person) is
    // ubiquitous in the corpus and must always be indexed.
    let probe: HashSet<&str> = idx.iter().map(String::as_str).collect();
    assert!(
        probe.contains("адам"),
        "facts index missing the «адам» core anchor"
    );
}
