// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! v6.0 acceptance: L6 verifier latency.
//!
//! Measures `Verifier::check()` latency on three surfaces that
//! exercise different code paths:
//!
//! - `grounded` — a known factual root («адам») that exercises the
//!   full FST round-trip + facts-index hit.
//! - `ungrounded_fst_valid` — a real Kazakh root not in facts
//!   («бала»); FST passes, grounding fails. Worst-case strict-mode
//!   block cost.
//! - `fst_invalid` — a non-Kazakh string («blarg»); short-circuits
//!   after the FST round-trip detects mismatch.
//!
//! The architecture spec
//! ([`docs/architecture_neural_v6.md`](../../../docs/architecture_neural_v6.md)
//! §5) lists `≤ 150 ms` for the entire neural-enabled turn. The
//! verifier sits inside that budget alongside the model forward
//! pass; per-check cost must therefore be in the microsecond
//! range, not the millisecond range.
//!
//! Run: `cargo bench -p adam-agg-model --bench verifier_latency`.

use adam_agg_model::verifier::{Verdict, Verifier};
use adam_agg_tokenizer::AggTokenizer;
use adam_kernel_fst::lexicon::LexiconV1;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn build_verifier(strict: bool) -> Verifier {
    let lex = LexiconV1::load(
        "../../data/tokenizer/segmentation_roots.json",
        "../../data/lexicon_v1/apertium_imported_roots.json",
    )
    .expect("lexicon load");
    let tokenizer = AggTokenizer::build(lex);
    let facts_idx =
        Verifier::load_facts_index("../../data/retrieval/facts.json").expect("facts.json load");
    Verifier::new(tokenizer, facts_idx, strict)
}

fn bench_grounded(c: &mut Criterion) {
    let v = build_verifier(false);
    c.bench_function("verifier_check_grounded_short_root", |b| {
        b.iter(|| {
            let r = v.check(black_box("адам"));
            // Force consumption so the compiler doesn't elide.
            assert!(matches!(r.verdict, Verdict::Pass { .. }));
        });
    });
}

fn bench_ungrounded_fst_valid(c: &mut Criterion) {
    let v = build_verifier(true);
    c.bench_function("verifier_check_ungrounded_fst_valid", |b| {
        b.iter(|| {
            let r = v.check(black_box("балалардың"));
            // Strict mode → ungrounded block or grounded pass depending
            // on facts coverage. Either way, the work is the same.
            let _ = r.verdict;
        });
    });
}

fn bench_fst_invalid(c: &mut Criterion) {
    let v = build_verifier(false);
    c.bench_function("verifier_check_fst_invalid_short_circuit", |b| {
        b.iter(|| {
            let r = v.check(black_box("blarg"));
            let _ = r.verdict;
        });
    });
}

criterion_group!(
    benches,
    bench_grounded,
    bench_ungrounded_fst_valid,
    bench_fst_invalid
);
criterion_main!(benches);
