// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! v6.0 acceptance-criteria bench: L5.5 neural inference latency.
//!
//! Measures forward-pass and greedy-generation latency on a TinyAgt
//! at the v6.0-default size (vocab=5241 to match real-corpus training,
//! d=64, layers=2 — ~1 M parameters). Three scenarios:
//!
//! - `forward_single` — one [1, seq_len] forward pass. Models the
//!   per-token cost in autoregressive decoding when KV caching is
//!   absent (Phase 0 baseline; KV cache is a Phase 1 optimisation).
//! - `generate_constrained_6tokens` — greedy FST-constrained
//!   generation of 6 new tokens from a 2-token prefix [BOS, R]. This
//!   is the canonical L5.5 surface-realisation cost.
//! - `generate_beam_w4_6tokens` — beam-search variant with width 4.
//!
//! Targets from `docs/architecture_neural_v6.md` §5 (release-blocking
//! on M2 8 GB reference hardware):
//! - p50 turn latency (neural path): ≤ 150 ms
//! - p99 turn latency (neural path): ≤ 400 ms
//!
//! These numbers must be re-measured after every model-shape change
//! and recorded in `docs/performance.md`.
//!
//! Run: `cargo bench -p adam-agg-model --bench neural_inference`.

use adam_agg_model::generate::{generate_constrained, generate_constrained_beam};
use adam_agg_model::{TinyAgt, TinyAgtConfig};
use burn::backend::NdArray;
use burn::backend::ndarray::NdArrayDevice;
use burn::prelude::*;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

type B = NdArray<f32>;

const BENCH_VOCAB: usize = 5241;
const BENCH_D_MODEL: usize = 64;
const BENCH_N_LAYERS: usize = 2;
const BENCH_MAX_SEQ_LEN: usize = 32;

fn build_model() -> (TinyAgt<B>, NdArrayDevice, Vec<String>) {
    let device = NdArrayDevice::default();
    let cfg = TinyAgtConfig::new(
        BENCH_VOCAB,
        BENCH_MAX_SEQ_LEN,
        BENCH_D_MODEL,
        4,
        BENCH_N_LAYERS,
        128,
    );
    let model: TinyAgt<B> = cfg.init(&device);

    // Synthesise a minimal compact-label vocabulary so the
    // FST-constrained decoder has labels to dispatch on. The
    // structure mirrors what `poc_kazakh_train` builds at runtime:
    //   0 = <unk>, 1 = BOS, 2 = EOS, 3 = <spc>,
    //   4..N = R: / S: tokens.
    let mut labels: Vec<String> = vec!["<unk>".into(), "BOS".into(), "EOS".into(), "<spc>".into()];
    // Fake a few roots and one suffix per slot so the validator finds
    // valid continuations at every state.
    labels.push("R:бала".into()); // id 4
    labels.push("S:Derivation(Diminutive)".into()); // 5
    labels.push("S:Number(Plural)".into()); // 6
    labels.push("S:Possessive(P1Sg)".into()); // 7
    labels.push("S:Case(Dative)".into()); // 8
    labels.push("S:Predicate(P1Sg)".into()); // 9
    // Pad out to BENCH_VOCAB with placeholder Unk labels so the
    // decoder's loop doesn't index out of bounds.
    while labels.len() < BENCH_VOCAB {
        labels.push("<pad>".into());
    }

    (model, device, labels)
}

fn bench_forward_single(c: &mut Criterion) {
    let (model, device, _labels) = build_model();
    // Single-token-input forward pass (worst case for autoregressive
    // decoding without KV cache).
    let tokens: Tensor<B, 2, Int> = Tensor::from_data(
        burn::tensor::TensorData::new(vec![1i64, 4i64], [1, 2]),
        &device,
    );

    c.bench_function("forward_2_tokens", |b| {
        b.iter(|| {
            let logits = model.forward(black_box(tokens.clone()));
            // Force materialisation.
            let _ = logits.dims();
        })
    });
}

fn bench_generate_constrained(c: &mut Criterion) {
    let (model, device, labels) = build_model();
    let prefix = vec![1i64, 4i64]; // [BOS, R:бала]

    c.bench_function("generate_constrained_greedy_6tokens", |b| {
        b.iter(|| {
            let out = generate_constrained(&model, &labels, black_box(&prefix), 6, &device);
            black_box(out);
        })
    });
}

fn bench_generate_beam(c: &mut Criterion) {
    let (model, device, labels) = build_model();
    let prefix = vec![1i64, 4i64];

    c.bench_function("generate_constrained_beam_w4_6tokens", |b| {
        b.iter(|| {
            let out = generate_constrained_beam(&model, &labels, black_box(&prefix), 6, 4, &device);
            black_box(out);
        })
    });
}

criterion_group!(
    benches,
    bench_forward_single,
    bench_generate_constrained,
    bench_generate_beam
);
criterion_main!(benches);
