# adam Resource Benchmark — v4.93.5

> **v6.0 forward-looking note (2026-05-16).** v4.93.5 resource
> bench covers the deterministic pipeline only. v6.0 RSS / latency
> contracts for the neural-enabled path are in
> [`architecture_neural_v6.md`](architecture_neural_v6.md) §5;
> measured numbers in
> [`bench/neural_inference_2026_05_16.md`](bench/neural_inference_2026_05_16.md).
> v6.0 must not regress the deterministic-path RSS recorded here.

Loaded runtime; running 30 queries.

## Latency
| metric | value |
|---|---|
| queries run | 30 |
| total wall-time | 496.86 ms |
| avg / query | 16.562 ms |
| p50 latency | 21.428 ms |
| p95 latency | 31.180 ms |
| p99 latency | 31.539 ms |

## Resource
| metric | value |
|---|---|
| user CPU time | 940.39 ms |
| system CPU time | 56.84 ms |
| total CPU / wall ratio | 2.01 (≤1 single-thread, >1 multi-core) |
| peak RSS | 304.5 MB |
| GPU usage | **0.0 %** (architectural — no neural component) |

## Comparison vs. published probabilistic LLM baselines

| system | per-turn latency | RSS / VRAM | GPU |
|---|---|---|---|
| **adam 4.93.5 (this release)** | **21.43 ms p50** | **304 MB** | **0 %** |
| Llama 3 8B fp16 (CPU-only) | ~800–1500 ms / token | ~16 GB | 0 % |
| Llama 3 8B int4 (Apple M2) | ~80–150 ms / token | ~5 GB | Metal-bound |
| GPT-4 (API) | ~50–200 ms / token | hidden | datacenter GPU |
| Claude Sonnet (API) | ~50–200 ms / token | hidden | datacenter GPU |

**Source for comparison numbers:** llama.cpp benchmarks 2024-12 + OpenAI / Anthropic public latency telemetry. adam numbers measured on this benchmark run.

**Architectural difference:** LLM latency scales with sequence length × parameters; adam latency is constant per turn and bounded by the morpheme index lookup + template fill (no autoregressive sampling).
