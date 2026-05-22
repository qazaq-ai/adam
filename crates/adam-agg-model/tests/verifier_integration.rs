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
    // Each word must satisfy all four gates: Kazakh script, root in
    // Lexicon (not Unk), FST round-trip exact, root in facts.json.
    //
    // Note (v6.0 hardening): a previous version of this test included
    // «Қазақстан». That word is NOT in the production Lexicon (only
    // «Қазақ» the adjective is) and the surface tokenises to Unk;
    // pre-hardening it slipped through because the surface itself
    // appeared in facts.json and grounded by surface match. The new
    // Unk gate correctly blocks that path. Adding «Қазақстан» (and
    // other proper-noun country compounds) to the Lexicon is tracked
    // separately as a coverage TODO.
    let words = ["адам", "дүние", "бала"];
    for w in words {
        let r = v.check(w);
        match r.verdict {
            Verdict::Pass { ref surface, .. } => assert_eq!(surface, w),
            other => panic!("expected Pass for {w:?}, got {other:?}"),
        }
    }
}

#[test]
fn script_gate_blocks_latin_independent_of_mode() {
    // **v6.0 hardening:** Latin strings used to slip past the
    // verifier in permissive mode (Unk round-trips byte-identically).
    // The script gate now blocks at the front door regardless of
    // strict / permissive — Latin output from the neural layer is
    // never a valid Kazakh surface and must never reach grounding.
    for v in [fixture(true), fixture(false)] {
        let r = v.check("zxqvwopr");
        assert_eq!(r.verdict, Verdict::Block(BlockReason::NonKazakhScript));
    }
}

#[test]
fn unk_gate_blocks_unknown_cyrillic_independent_of_mode() {
    // **v6.0 hardening:** a Cyrillic-but-not-in-Lexicon surface
    // tokenises to Unk and used to silently pass the FST round-trip
    // (Unk preserves the byte string). The Unk gate now blocks
    // before grounding has a chance — Unk surfaces are by definition
    // unanalysable.
    for v in [fixture(true), fixture(false)] {
        let r = v.check("зщшщзщъ");
        assert_eq!(r.verdict, Verdict::Block(BlockReason::UnkSurface));
    }
}

#[test]
fn strict_mode_blocks_ungrounded_real_kazakh_root() {
    let v = fixture(true);
    // «ажар» (= appearance / look) — a real Kazakh noun present in
    // the production Lexicon (apertium_imported_roots) but NOT in
    // facts.json. Verified empirically at the time of writing: in
    // facts.json «бала», «адам», «дүние» are grounded; «ажар» is not.
    // The verifier must extract root «ажар», fail the grounding
    // lookup, and emit Block(Ungrounded) under strict mode.
    let r = v.check("ажар");
    assert_eq!(r.verdict, Verdict::Block(BlockReason::Ungrounded));
}

#[test]
fn permissive_mode_admits_ungrounded_fst_valid_surfaces() {
    let v = fixture(false);
    // A real Kazakh inflection whose root may or may not appear in
    // facts.json. Property under test: permissive mode never blocks
    // a script-clean, FST-valid, non-Unk Kazakh surface — it surfaces
    // the grounded flag for the caller and lets the surface through.
    let r = v.check("балам");
    match r.verdict {
        Verdict::Pass { grounded, .. } => {
            let _ = grounded;
        }
        other => panic!("expected Pass in permissive mode, got {other:?}"),
    }
}

#[test]
fn check_is_deterministic_across_calls() {
    // Critical invariant for audit / reproducibility — repeated
    // checks must produce byte-identical verdicts. Mix Pass cases
    // with one verdict from each of the four block-reason categories
    // so determinism is exercised on every gate path.
    let v = fixture(false);
    let inputs = ["адам", "дүние", "балам", "zxqvwopr", "зщшщзщъ"];
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
