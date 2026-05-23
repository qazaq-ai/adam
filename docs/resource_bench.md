# adam Resource Benchmark — v6.0.0

> **2026-05-22 refresh** at the v6.0.0 GA merge. The bench machine
> is the same M2 8 GB as the v4.93.5 reading; the deterministic
> pipeline is on the default cascade path (opt-in neural off).
> Default-path numbers therefore reflect the v6.0.0 cascade with
> the 7 rounds of audit-driven fixes landed, NOT a regression of
> the pre-merge v5.32.0 numbers.
>
> For the **opt-in neural-enabled path** (L5.5 TinyAgt composer,
> E1 intent rescue, E2 slot rescue) RSS / latency contracts live
> in [`architecture_neural_v6.md`](architecture_neural_v6.md) §5;
> measured numbers in
> [`bench/neural_inference_2026_05_16.md`](bench/neural_inference_2026_05_16.md).
> v6.0 must not regress the deterministic-path RSS recorded here.

Loaded runtime; running 30 queries.

## Latency
| metric | value |
|---|---|
| queries run | 30 |
| total wall-time | 293.47 ms |
| avg / query | 9.782 ms |
| p50 latency | 13.355 ms |
| p95 latency | 19.616 ms |
| p99 latency | 19.670 ms |

## Resource
| metric | value |
|---|---|
| user CPU time | ≈ 420 ms (cold-start single-shot) |
| system CPU time | ≈ 40 ms |
| total CPU / wall ratio | ≈ 2.0 (multi-thread on tool-dispatch) |
| peak RSS | 315.6 MB |
| GPU usage | **0.0 %** (deterministic-path default; opt-in neural also CPU-only — see `architecture_neural_v6.md`) |

## Comparison vs. published probabilistic LLM baselines

> **Methodology caveat (2026-05-22 codex audit).** adam numbers in
> the table below are measured on this benchmark run. The LLM
> numbers are from llama.cpp / OpenAI / Anthropic public sources
> as cited; an independent, methodology-aligned, head-to-head
> benchmark against current LLM endpoints has NOT yet been
> performed. Treat the row deltas as approximate-magnitude
> illustration, not as a fair comparison until the formal
> benchmark lands. See
> [`bench/our_numbers_vs_published_llm.md`](bench/our_numbers_vs_published_llm.md)
> for the full caveats list.

| system | per-turn latency | RSS / VRAM | GPU |
|---|---|---|---|
| **adam 6.0.0 (this release)** | **13.36 ms p50** | **316 MB** | **0 %** |
| Llama 3 8B fp16 (CPU-only) | ~800–1500 ms / token | ~16 GB | 0 % |
| Llama 3 8B int4 (Apple M2) | ~80–150 ms / token | ~5 GB | Metal-bound |
| GPT-4 (API) | ~50–200 ms / token | hidden | datacenter GPU |
| Claude Sonnet (API) | ~50–200 ms / token | hidden | datacenter GPU |

**Source for comparison numbers:** llama.cpp benchmarks 2024-12 + OpenAI / Anthropic public latency telemetry. adam numbers measured on this benchmark run.

**Architectural difference:** LLM latency scales with sequence length × parameters; adam latency is constant per turn and bounded by the morpheme index lookup + template fill (no autoregressive sampling).
