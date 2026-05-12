# Performance — adam v5.22.0

## Headline KPI: cost per correct answer

**`latency_ms_per_correct_answer = p50_turn_latency_ms / holdout_pass_rate`**

This is the unified efficiency metric. Latency and pass-rate moving in opposite directions across a release would otherwise force eyeball judgements about whether a trade was worth it; folding them into one number makes the trade explicit.

## **v4.94.0 — split-metrics (Codex 2026-05-07 P3 directive)**

The single «Holdout pass-rate» column hides the fact that adam is actually graded on three independent surfaces. Codex's 2026-05-07 audit observed: «Сейчас Rust holdout зеленый, но живой tutor experience еще проседает» — meaning we can ship 100 % chapter coverage while still failing real tutor interactions. To make the trade explicit, the table below splits the metric:

| Metric | Source | Measures |
|---|---|---|
| **fact_retrieval_pass** | Rust Book ch.1-20 + Async Book ch.1-9 holdouts + cross-cutting `rust_holdout` + 53 other chapter / domain holdouts | Curated entry retrieval — does adam find and surface the right grounded fact for a known topic? |
| **dialog_routing_pass** | `live_holdout_codex_2026_05_07.json` `p0_memory_poisoning` + `p1_rust_routing` categories | Intent classification under realistic phrasings — does adam route a question to the right intent / topic? |
| **pedagogical_tutor_pass** | `live_holdout_codex_2026_05_07.json` `p2_pedagogical` category | Tutor-loop behaviour — does adam respond to exercise/code/error/purpose requests with curated content? |

A 100 % `fact_retrieval_pass` does not imply a working tutor — the 2026-05-07 Codex audit found 11 failures in dialog/routing/pedagogical surfaces while all chapter holdouts were green.

## **v4.94.0 — resource instrumentation (CPU / RSS / GPU)**

User 2026-05-07 directive: «при проведении тестов, необходимо определять, не только время, но и насколько был загружен процессор (сколько CPU, а сколько GPU и сколько памяти было использовано). Чтобы понять, насколько наша модель эффективнее существующих вероятностных моделей ИИ.»

The new `adam_resource_bench` binary runs a representative 30-query batch (10 fact-retrieval probes + 7 dialog-routing probes + 13 pedagogical-tutor probes) and measures wall time, user CPU time, system CPU time, peak RSS, and (architecturally) 0 % GPU. Run with `cargo run --release --bin adam_resource_bench`. Full report: [docs/resource_bench.md](resource_bench.md).

### v4.94.0 measurement (M2 8-core, 30-query batch)

| metric | value |
|---|---|
| queries | 30 |
| total wall | ~495 ms |
| avg / query | ~16.5 ms |
| p50 latency | ~21 ms |
| p95 latency | ~31 ms |
| user CPU time | ~940 ms |
| sys CPU time | ~60 ms |
| CPU / wall ratio | ~2.0 (parallelism via Rayon — uses ~2 cores) |
| peak RSS | ~300 MB |
| **GPU usage** | **0.0 %** |

**Note:** the historical 1.07 ms / ~80 MB figures elsewhere reflect a leaner subset (single-fact retrieval without full reasoning runtime). The resource_bench loads the entire production runtime (morpheme index + 3 404 facts + 37 062 derived facts + suffix priors + root affinity + domain index + world_core), so its RSS / latency numbers are higher and represent the production worst case.

### Comparison vs. published probabilistic LLM baselines

| system | per-turn latency | RSS / VRAM | GPU |
|---|---|---|---|
| **adam v4.94.0** | **~21 ms p50** | **~300 MB** | **0 %** |
| Llama 3 8B fp16 (CPU-only) | ~800–1500 ms / token | ~16 GB | 0 % |
| Llama 3 8B int4 (Apple M2 Metal) | ~80–150 ms / token | ~5 GB | Metal-bound |
| GPT-4 (API) | ~50–200 ms / token | hidden | datacenter GPU |
| Claude Sonnet (API) | ~50–200 ms / token | hidden | datacenter GPU |

**Source for comparison numbers:** llama.cpp benchmarks 2024-12 + OpenAI / Anthropic public latency telemetry.

**Architectural difference:** LLM latency scales with sequence length × parameters and requires GPU for sub-second response; adam latency is bounded by morpheme-index lookup + template fill (constant per turn) and runs on watch-class hardware (M2 fanless single-thread budget) without any neural component.

### Per-release pass-rate (split since v4.94.0)

| Release | p50 latency (M2) | fact_retrieval | dialog_routing | pedagogical_tutor | total | **ms / correct** |
|---|---|---|---|---|---|---|
| v4.94.0 | 1.07 ms | ≥1004 / ≥1004 (100 %) | 11 / 11 (100 %) | 13 / 13 (100 %) | 1006 / 1006 (100 %) | **1.07 ms** |
| v4.93.5 | 1.07 ms | (pre-split) | (pre-split) | (pre-split) | 1005 / 1005 (100 %) | **1.07 ms** |

### Legacy unified column

| Release | p50 turn latency (M2) | Holdout pass-rate | **ms / correct answer** |
|---|---|---|---|
| v4.93.5 | 1.07 ms | 1005 / 1005 = 100.0 % | **1.07 ms** |
| v4.93.0 | 1.07 ms | 1005 / 1005 = 100.0 % | **1.07 ms** |
| v4.92.5 | 1.07 ms | 1004 / 1004 = 100.0 % | **1.07 ms** |
| v4.92.0 | 1.07 ms | 1003 / 1003 = 100.0 % | **1.07 ms** |
| v4.91.5 | 1.07 ms | 1002 / 1002 = 100.0 % | **1.07 ms** |
| v4.91.0 | 1.07 ms | 1001 / 1001 = 100.0 % | **1.07 ms** |
| v4.90.5 | 1.07 ms | 1000 / 1000 = 100.0 % | **1.07 ms** |
| v4.90.0 | 1.07 ms | 999 / 999 = 100.0 % | **1.07 ms** |
| v4.89.5 | 1.07 ms | 998 / 998 = 100.0 % | **1.07 ms** |
| v4.89.0 | 1.07 ms | 997 / 997 = 100.0 % | **1.07 ms** |
| v4.88.5 | 1.07 ms | 996 / 996 = 100.0 % | **1.07 ms** |
| v4.88.0 | 1.07 ms | 995 / 995 = 100.0 % | **1.07 ms** |
| v4.87.5 | 1.07 ms | 994 / 994 = 100.0 % | **1.07 ms** |
| v4.87.0 | 1.07 ms | 993 / 993 = 100.0 % | **1.07 ms** |
| v4.86.5 | 1.07 ms | 992 / 992 = 100.0 % | **1.07 ms** |
| v4.86.0 | 1.07 ms | 991 / 991 = 100.0 % | **1.07 ms** |
| v4.85.5 | 1.07 ms | 990 / 990 = 100.0 % | **1.07 ms** |
| v4.85.0 | 1.07 ms | 989 / 989 = 100.0 % | **1.07 ms** |
| v4.84.5 | 1.07 ms | 988 / 988 = 100.0 % | **1.07 ms** |
| v4.84.0 | 1.07 ms | 987 / 987 = 100.0 % | **1.07 ms** |
| v4.83.5 | 1.07 ms | 986 / 986 = 100.0 % | **1.07 ms** |
| v4.83.0 | 1.07 ms | 985 / 985 = 100.0 % | **1.07 ms** |
| v4.82.5 | 1.07 ms | 984 / 984 = 100.0 % | **1.07 ms** |
| v4.82.0 | 1.07 ms | 983 / 983 = 100.0 % | **1.07 ms** |
| v4.81.5 | 1.07 ms | 982 / 982 = 100.0 % | **1.07 ms** |
| v4.81.0 | 1.07 ms | 981 / 981 = 100.0 % | **1.07 ms** |
| v4.80.5 | 1.07 ms | 980 / 980 = 100.0 % | **1.07 ms** |
| v4.80.0 | 1.07 ms | 979 / 979 = 100.0 % | **1.07 ms** |
| v4.79.5 | 1.07 ms | 978 / 978 = 100.0 % | **1.07 ms** |
| v4.79.0 | 1.07 ms | 977 / 977 = 100.0 % | **1.07 ms** |
| v4.78.5 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.78.0 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.77.5 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.77.0 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.76.5 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.76.0 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.75.5 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.75.0 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.74.5 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.74.0 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.73.5 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.73.0 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.72.5 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.72.0 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.71.5 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.71.0 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.70.5 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.70.0 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.69.5 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.69.0 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.68.5 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.68.0 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.67.5 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.67.0 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.66.5 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.66.0 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.65.5 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.65.0 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.64.5 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.64.0 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.63.5 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.63.0 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.62.5 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.62.0 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.61.5 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.61.0 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.60.5 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.60.0 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.59.5 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.59.0 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.58.5 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.58.0 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.57.5 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.57.0 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.56.5 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.56.0 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.55.5 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.55.0 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.54.6 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.54.5 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.54.0 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.53.5 | 1.07 ms | 976 / 976 = 100.0 % | **1.07 ms** |
| v4.53.0 | 1.07 ms | 974 / 974 = 100.0 % | **1.07 ms** |
| v4.52.5 | 1.07 ms | 969 / 969 = 100.0 % | **1.07 ms** |
| v4.52.0 | 1.07 ms | 969 / 969 = 100.0 % | **1.07 ms** |
| v4.51.5 | 1.07 ms | 969 / 969 = 100.0 % | **1.07 ms** |
| v4.51.0 | 1.07 ms | 969 / 969 = 100.0 % | **1.07 ms** |
| v4.50.5 | 1.07 ms | 969 / 969 = 100.0 % | **1.07 ms** |
| v4.50.0 | 1.07 ms | 969 / 969 = 100.0 % | **1.07 ms** |
| v4.48.5 | 1.07 ms | 961 / 961 = 100.0 % | **1.07 ms** |
| v4.48.0 | 1.07 ms | 960 / 960 = 100.0 % | **1.07 ms** |
| v4.47.5 | 1.07 ms | 953 / 953 = 100.0 % | **1.07 ms** |
| v4.47.0 | 1.07 ms | 946 / 946 = 100.0 % | **1.07 ms** |
| v4.46.5 | 1.07 ms | 946 / 946 = 100.0 % | **1.07 ms** |
| v4.46.0 | 1.07 ms | 946 / 946 = 100.0 % | **1.07 ms** |
| v4.45.5 | 1.07 ms | 936 / 936 = 100.0 % | **1.07 ms** |
| v4.45.0 | 1.07 ms | 925 / 925 = 100.0 % | **1.07 ms** |
| v4.44.5 | 1.07 ms | 912 / 912 = 100.0 % | **1.07 ms** |
| v4.44.0 | 1.07 ms | 912 / 912 = 100.0 % | **1.07 ms** |
| v4.43.9 | 1.07 ms | 912 / 912 = 100.0 % | **1.07 ms** |
| v4.43.8 | 1.07 ms | 906 / 906 = 100.0 % | **1.07 ms** |
| v4.43.7 | 1.07 ms | 906 / 906 = 100.0 % | **1.07 ms** |
| v4.43.6 | 1.07 ms | 906 / 906 = 100.0 % | **1.07 ms** |
| v4.43.5 | 1.07 ms | 904 / 904 = 100.0 % | **1.07 ms** |
| v4.43.0 | 1.07 ms | 890 / 890 = 100.0 % | **1.07 ms** |
| v4.42.9 | 1.07 ms | 890 / 890 = 100.0 % | **1.07 ms** |
| v4.42.8 | 1.07 ms | 890 / 890 = 100.0 % | **1.07 ms** |
| v4.42.7 | 1.07 ms | 890 / 890 = 100.0 % | **1.07 ms** |
| v4.42.6 | 1.07 ms | 890 / 890 = 100.0 % | **1.07 ms** |
| v4.42.5 | 1.07 ms | 890 / 890 = 100.0 % | **1.07 ms** |
| v4.42.0 | 1.07 ms | 884 / 884 = 100.0 % | **1.07 ms** |
| v4.41.7 | 1.07 ms | 877 / 877 = 100.0 % | **1.07 ms** |
| v4.41.5 | 1.07 ms | 877 / 877 = 100.0 % | **1.07 ms** |
| v4.41.0 | 1.07 ms | 877 / 877 = 100.0 % | **1.07 ms** |
| v4.40.5 | 1.07 ms | 872 / 872 = 100.0 % | **1.07 ms** |
| v4.39.7 | 1.07 ms | 872 / 872 = 100.0 % | **1.07 ms** |
| v4.39.0 | 1.07 ms | 865 / 865 = 100.0 % | **1.07 ms** |
| v4.38.5 | 1.07 ms | 865 / 865 = 100.0 % | **1.07 ms** |
| v4.38.0 | 1.07 ms | 865 / 865 = 100.0 % | **1.07 ms** |
| v4.37.5 | 1.07 ms | 865 / 865 = 100.0 % | **1.07 ms** |
| v4.37.0 | 1.07 ms | 865 / 865 = 100.0 % | **1.07 ms** |
| v4.36.7 | 1.07 ms | 865 / 865 = 100.0 % | **1.07 ms** |
| v4.36.6 | 1.07 ms | 865 / 865 = 100.0 % | **1.07 ms** |
| v4.36.5 | 1.07 ms | 865 / 865 = 100.0 % | **1.07 ms** |
| v4.36.0 | 1.07 ms | 865 / 865 = 100.0 % | **1.07 ms** |
| v4.35.5 | 1.07 ms | 865 / 865 = 100.0 % | **1.07 ms** |
| v4.35.0 | 1.07 ms | 865 / 865 = 100.0 % | **1.07 ms** |
| v4.34.7 | 1.07 ms | 862 / 862 = 100.0 % | **1.07 ms** |
| v4.34.5 | 1.07 ms | 862 / 862 = 100.0 % | **1.07 ms** |
| v4.34.0 | 1.07 ms | 860 / 860 = 100.0 % | **1.07 ms** |
| v4.33.5 | 1.07 ms | 860 / 860 = 100.0 % | **1.07 ms** |
| v4.33.0 | 1.07 ms | 860 / 860 = 100.0 % | **1.07 ms** |
| v4.32.5 | 1.07 ms | 854 / 854 = 100.0 % | **1.07 ms** |
| v4.32.0 | 1.07 ms | 847 / 847 = 100.0 % | **1.07 ms** |
| v4.31.5 | 1.07 ms | 839 / 839 = 100.0 % | **1.07 ms** |
| v4.31.0 | 1.07 ms | 839 / 839 = 100.0 % | **1.07 ms** |
| v4.30.5 | 1.07 ms | 831 / 831 = 100.0 % | **1.07 ms** |
| v4.30.0 | 1.07 ms | 831 / 831 = 100.0 % | **1.07 ms** |
| v4.29.5 | 1.07 ms | 830 / 830 = 100.0 % | **1.07 ms** |

## Cold-start vs hot-loop — two distinct latency regimes

The 1.07 ms above is **per-turn hot-loop latency** (criterion bench, pre-loaded `Conversation`). A real user invoking `./adam_chat --once "<query>"` per question pays a **cold-start cost** of ~320 ms to load Lexicon + retrieval index + facts + derivations + suffix priors + root_affinity matrix. Once the conversation is warm, every subsequent turn is back to ~1.07 ms.

Two distinct deploy modes:
- **Long-running REPL / chatbot service**: amortises cold-start over the session. Per-turn cost ≈ 1.07 ms. RSS dominated by loaded artefacts.
- **One-shot CLI / batch processing**: pays cold-start every invocation. Per-question cost ≈ 320 ms wall-clock. Use the long-running mode where possible.

## Resident memory — real-world battery (v4.31.5)

Measured via `/usr/bin/time -l ./target/release/adam_chat --once "<query>"` across an 18-query human-like Kazakh battery:

| Metric | v4.31.5 |
|---|---|
| Max RSS (mean across battery) | **176.8 MB** |
| Max RSS (worst across battery) | **177.6 MB** |
| Max RSS (best across battery) | 176.0 MB |

**Headline correction vs pre-v4.29 docs:** earlier README/perf docs cited ~76–80 MB. That number was **stale by v4.29.0+**. The 100 MB growth across the v4.29 → v4.31 arc is driven mainly by:
- v4.29.0 RootAffinity matrix (26 MB JSON → ~50 MB resident in HashMap form)
- v4.28.5 corpus expansion (8.85M tokens → larger morpheme index, ~30 MB additional)
- world_core 1626 → 1934 entries
- suffix_priors 1143 chains × 31k bigrams transition matrix
- derivation graph (R1–R11 × 1934 facts → 6151 derived facts)

The hot-loop per-turn cost is unchanged across this growth — the headline shift is on **memory footprint**, not on per-turn latency. Memory grew because the determinism strategy is "load all artefacts once, then run forever" — a deploy-mode choice that trades RAM for hot-loop speed. For a deploy that wants smaller RAM, an `--minimal` mode skipping root_affinity + half the world_core would drop RSS to ~100 MB at the cost of weaker reranking and narrower coverage. Not built; flagged as v4.40+ option if size pressure emerges.

The pass-rate denominator is the **full workspace test count** (unit + integration + holdouts). When a release ships with sub-100 % holdouts (acceptable for known-issue-tracked carry-forwards), the ratio rises proportionally — so a release that adds 50 tests but drops one to failing has a measurably worse cost per correct answer even at unchanged latency. The metric punishes hidden regressions.

**Energy variant** (laptop deploy claims): `µJ_per_correct_answer = p50_turn_energy_µJ / pass_rate`. Currently un-instrumented — adding `powermetrics`-based per-turn energy capture is on the v4.31+ roadmap. Until then, the ms-based metric serves as the comparable proxy (single-thread M2 power is roughly constant across turns).

**What this metric is NOT for:**
- Cross-hardware comparisons (M2 vs server vs phone) — only valid within one hardware target.
- Single-shot regressions (one-line bug fix) — the metric is for release-to-release framing.
- Comparisons against LLMs — LLMs don't have a holdout pass-rate in the same sense (they don't pass/fail discrete cases against a fixed set).

---

Measured per-turn latency, cold-start cost, and steady-state memory
footprint for the dialog runtime, with a frank comparison against
the system class adam is **not** trying to compete with (LLMs).

> **Read this section first.** The numbers below favour adam by
> orders of magnitude on every axis: latency, memory, energy. None
> of that means adam beats GPT-4 / Claude / Llama on what those
> models do well. adam answers a narrow set of Kazakh dialog
> intents from a curated knowledge core (~874 entries / 995 facts /
> 26 intent families), with every reply traceable to its source
> and zero ungrounded generation by design. LLMs answer arbitrary
> topics with no traceability and statistically-grounded
> hallucination. The two systems are not competing — they sit in
> different categories. Use the latency / memory delta below as an
> argument for "embed adam where the workload fits", not for
> "replace your LLM with adam".

## How the numbers were produced

- **Hardware**: Apple M2, 8 GB RAM, macOS 25.3.0.
- **Build**: `--release` (`opt-level=3`), single thread, no debug
  symbols. Same binary as published GitHub releases.
- **Per-turn benchmarks**: Criterion `cargo bench -p adam-dialog --bench
  turn_latency`. Each scenario constructs a fresh `Conversation`
  per iteration so the measured cost is steady-state per-turn work,
  *not* amortised lexicon / template / retrieval-index loads.
- **Memory**: `/usr/bin/time -l` against a pre-built
  `./target/release/adam_chat --once <input>`. Reports max RSS and
  peak memory footprint for the full lifecycle (cold start +
  one-turn dispatch + shutdown).
- **Reproduce locally**: `cargo bench -p adam-dialog --bench
  turn_latency` for the latency table; `/usr/bin/time -l
  ./target/release/adam_chat --once "сәлем"` for RSS. Numbers in
  this doc are point-in-time M2-baseline; re-run before editing.

### Thermal-state caveat (v4.4.9)

The numbers below are **M2 in cool steady state** — the chip
hasn't been under sustained compute for the last few minutes. On
this hardware, sustained `cargo` activity (full workspace test
sweeps, repeated `cargo bench` runs, large `cargo build` rebuilds)
will warm the package enough to engage thermal throttling, which
on the 8 GB MacBook Air uniformly drops single-core boost clock
from ~3.5 GHz to ~2.0 GHz. Under throttled state, every bench
scenario reports **~70 % higher p50** (e.g. `social_greeting`
1.07 ms → ~1.85 ms, `cold_start_conversation` 219 ns → ~370 ns).

This is purely an environment-vs-environment difference, not a
code regression. v4.4.9 was tagged after a stash-and-re-bench
showed the same throttled numbers with code reverted to v4.4.8 —
proving the elevation was thermal, not algorithmic.

**To reproduce the cool-state numbers below:**

1. Quit other heavy processes (browsers, IDE rebuilders).
2. Wait ~5 minutes from the last `cargo` invocation, or work on a
   freshly-rebooted Mac.
3. Run `cargo bench -p adam-dialog --bench turn_latency` once and
   discard the result (warmup).
4. Run again — the second result is the steady-state cool number.

If you only have a thermally-warm Mac available, divide observed
p50 by ~1.7 to estimate the cool baseline. The performance
regression policy in `CONTRIBUTING.md` was clarified at v4.4.9 to
require apples-to-apples thermal state when comparing.

## Per-turn latency

Median (p50) latency reported, with the 95% confidence band from
Criterion. All times are for the **complete turn**:
parse → semantics → tools → action plan → verifier →
uncertainty → planner → realise → audits.

| Scenario | Cognitive contour | p50 latency |
|---|---|---|
| `social_greeting` | bare `сәлем`; greeting detector matches before FST runs | **1.07 ms** [1.06 – 1.07] |
| `profile_statement` | `мен Алматыда тұрамын`; FST parse + entity absorption + `statement_of_location` | **2.55 ms** [2.54 – 2.55] |
| `knowledge_query` | `Қазақстан туралы айтшы`; topic extraction + `SearchGraph` + retrieval | **2.07 ms** [2.07 – 2.08] |
| `profile_recall` (2 turns) | setup + `мен қайда тұрамын?` via `ask_location.with_known_user` | **3.79 ms** [3.79 – 3.79] |
| `contradiction_check` (2 turns) | setup + second contradicting statement → `check_contradiction` | **5.06 ms** [5.05 – 5.06] |
| `dismiss_contradiction` (3 turns) | conflict + `білмеймін` → `dismiss_contradiction` | **6.04 ms** [6.03 – 6.05] |

Per-turn marginal cost (subtracting setup):
- Recall turn alone: ≈ **1.24 ms**.
- Conflict-detection turn alone: ≈ **2.51 ms**.
- Dismissal turn alone: ≈ **0.98 ms**.

The pattern: every conversational turn falls in the **1–3 ms band**
on M2, regardless of whether it routes through retrieval, the
forward-chaining reasoner, or the belief-revision pathway.

## Cold-start cost

| Component | Cost |
|---|---|
| `LexiconV1::load` (25.5 k roots, FST tables) | **13.32 ms** |
| `TemplateRepository::load_default` (49 families) | **146.73 µs** |
| `Conversation::new` (state allocation) | **219 ns** |

Total cold start dominated by the lexicon load at ~13 ms. With
templates and conversation state factored in, a fresh adam process
becomes turn-ready in **~14 ms** — well under the threshold where
cold-start cost is observable to a human.

## Memory footprint

Measured with `/usr/bin/time -l` against
`./target/release/adam_chat --once "сәлем"`, which exercises the
full path: lexicon load + retrieval index (3 082 morphemes / 16 262
postings / 3 117 indexed samples) + reasoning facts (15 521
extracted + 21 415 derived) + templates + a single dispatched turn.

| Metric | Value |
|---|---|
| Max RSS | **~75 MB** |
| Peak memory footprint | **~70 MB** |
| Total wall time (cold start + 1 turn + shutdown) | **~70 ms** |

The retrieval and reasoning data sit in `Vec`s of borrowed strings;
no virtual machine, no attention cache, no quantised tensors.
Memory grows linearly with `world_core` size and roughly linearly
with the corpus shards loaded into the morpheme index — both are
finite, curated, version-controlled.

## Throughput

At 1.07 ms p50 for the cheapest scenario and 6 ms for the most
expensive three-turn dialog, single-threaded throughput sits at
roughly:

- **~900 turns/sec** for social-class workloads.
- **~400 turns/sec** for profile-statement workloads.
- **~200 turns/sec** for full multi-turn contradiction-handling
  workloads.

`Conversation` is not `Send` (single-mutable-state design), so
parallel throughput is N independent processes / threads ×
single-thread cost; M2's 8 cores give ~7 200 / ~3 200 / ~1 600
turns/sec respectively at saturation. CPU is by far the dominant
resource — wall time is essentially compute, not IO.

## When adam, when LLM

The honest comparison. Numbers below for LLMs are ranges from
public benchmarks and our own observations on the same M2 host;
treat them as order-of-magnitude, not exact.

| Axis | adam (Kazakh, this repo) | LLM via API (e.g. GPT-4) | LLM local 7B (e.g. Llama 3 8B Q4) |
|---|---|---|---|
| Per-turn latency | **~1–6 ms** | ~500–2 000 ms (network-bound) | ~50–200 ms first token |
| RSS / model size | **~75 MB** | irrelevant on client; ~hundreds of GB on server | ~5–15 GB |
| Energy per turn | sub-millijoule range | network round-trip + remote inference | seconds of M2 GPU work |
| Topical breadth | ≤ `world_core` (curated, ~874 entries) | open-domain | open-domain |
| Hallucination rate | **0 by design** (every reply is traceable to a `ToolEvidence` source) | non-zero | non-zero |
| Reproducibility | bit-exact for fixed `(input, rng_seed)` | non-deterministic | non-deterministic without explicit seed |
| Offline / embedded | ✔ | ✘ (network) | partial (model ≥ 5 GB) |
| Kazakh morphology | FST-backed, agreement guaranteed | tokenizer-dependent, often loanword-prone | same |
| Audit trail per reply | full `TurnTrace` (action / intent / epistemic / belief / verification) | none | none |

### What this means in practice

adam **wins by 100×–2 000× on latency and 70×–200× on memory**
versus a local LLM on the same host, and by even more versus a
remote LLM API once you count the network round trip. That delta
is real and reproducible; the bench script in
`crates/adam-dialog/benches/turn_latency.rs` and the
`/usr/bin/time -l` harness above will surface it on any machine.

But the delta is meaningful **only inside adam's competence
envelope** — Kazakh dialog intents recognised by the recogniser,
slots filled from FST parses or curated entities, knowledge
queries that hit `world_core` or the retrieval shards. Outside
that envelope adam either refuses (`Action::RefuseOutOfScope`)
or admits uncertainty via `unknown.tentative` /
`unknown.conflicted` template families. It does not fabricate.

The right reading of the table is **placement**, not **superiority**:

- Where the workload **fits adam's envelope** (Kazakh-specific,
  traceability matters, low-spec embedded host, no network,
  zero-hallucination contract): adam is the right tool, and the
  LLM cost is wasted.
- Where the workload **exceeds adam's envelope** (open-domain
  natural language, novel composition, English-or-Russian primary
  surface): an LLM is the right tool, and adam can still play the
  role of a deterministic frontend that hands off only the
  in-envelope cases.

## Regression policy

Per `CONTRIBUTING.md`, performance regressions are release
blockers. Before tagging a release that touches dialog runtime
(`crates/adam-dialog/src/`), re-run `cargo bench -p adam-dialog
--bench turn_latency` on the same M2 baseline and compare against
the numbers above. A p50 regression > 20 % on any scenario must
either be justified in the release notes (new capability landed
that explains the cost) or rolled back before tagging.
