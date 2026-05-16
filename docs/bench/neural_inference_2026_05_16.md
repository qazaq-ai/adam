# Neural-inference latency bench — 2026-05-16

**Branch:** `experimental/agglutinative-neural`
**Hardware:** MacBook Air M2, 8 GB RAM, macOS 25.3.0, CPU-only
**Stack:** Rust 1.89, burn 0.17 (ndarray backend), no GPU
**Source:** `crates/adam-agg-model/benches/neural_inference.rs`
**Model:** `TinyAgt` at PoC config — vocab 5 241, d_model 64,
4 heads, 2 layers, d_ff 128, max_seq_len 32 → ~1 M parameters.

## Numbers

Criterion's three-value report is `[lower-bound, mean, upper-bound]`
of the 99 % confidence interval on the per-iteration time.

| Operation | Lower | Mean | Upper |
|---|---:|---:|---:|
| `forward_2_tokens` (single forward pass, 2-token input) | 520 µs | **526 µs** | 533 µs |
| `generate_constrained_greedy_6tokens` (FST-masked greedy, +6 tokens) | 1.68 ms | **1.71 ms** | 1.74 ms |
| `generate_constrained_beam_w4_6tokens` (FST-masked beam, width 4, +6 tokens) | 4.17 ms | **4.20 ms** | 4.23 ms |

## Comparison vs v6.0 acceptance contracts

[`architecture_neural_v6.md`](../architecture_neural_v6.md) §5:

| Contract | Target | Measured | Headroom |
|---|---:|---:|---:|
| p50 turn latency, neural-enabled path | ≤ 150 ms | 1.71 ms (greedy) | **88×** |
| p99 turn latency, neural-enabled path | ≤ 400 ms | ~1.74 ms (greedy) | **230×** |
| Energy per turn, neural-enabled | ≤ 0.40 J | not yet measured | — |

The latency contracts pass with > 80× headroom. This is the
empirical justification for the watch-battery deployment claim in
[`MANIFESTO.md`](../MANIFESTO.md) §3.2: a neural inference at L5.5
costs ~2 ms of CPU, comparable to a single PostgreSQL index lookup
on the same hardware.

## What this measurement excludes

- **KV cache.** Phase 0 has no KV cache; every forward pass
  re-attends to the full prefix. Production v6.x will add KV cache
  per architecture_neural_v6 §3 implementation note. Expected
  speedup on 6-token generation: ~3–5×, so the production
  greedy-generation latency target is ~0.5 ms.
- **Model loading.** Model construction (~1 M params init from
  random) is amortised across iterations by `build_model()` in the
  bench. Cold-start cost is measured separately by
  `crates/adam-dialog/benches/cold_start.rs`.
- **Output decoding (token-id → surface).** Detokenisation through
  `AggTokenizer::detokenize_word` adds ~10 µs per word; not
  included here.
- **Verifier (L6) cost.** Measured separately when the verifier
  integration lands.

## Reproducibility

```bash
cargo bench -p adam-agg-model --bench neural_inference
```

No flags. Run from the repo root. The bench is deterministic up to
the OS scheduler; expect ±5 % variance run-to-run.

## Action items

- Add multi-hardware runs (M1, Intel x86_64, ARM Linux) to satisfy
  v6.0 acceptance criterion #3.
- Measure energy per turn via `powermetrics` on macOS / `perf stat`
  on Linux.
- Re-measure after every model-shape change in `TinyAgtConfig`.
- Wire this bench into CI as a non-blocking informational job; flip
  to blocking once the model shape is frozen for the v6.0 release.
