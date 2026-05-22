# Eval Baseline

> **Legacy context (transformer era, v0.1 – v0.4).** Describes benchmark assembly for the pre-v0.4.5 transformer baseline. The current adam architecture (v1.0.0 → v4.1.0) validates correctness via `cargo test --workspace` (**577 passing as of v4.1.0**, 0 failing, 0 warnings) + `scripts/validate_foundation.sh` + the **cognitive eval harness at 22 / 22 canonical, 0 aspirational** in `crates/adam-dialog/tests/cognitive_eval.rs` over `data/eval/cognitive_dialog_dataset.json`. Not via a benchmark perplexity score. For the current architecture see [`docs/architecture_v3.md`](architecture_v3.md) (v3.0 retrieval reference, still load-bearing) and [`docs/foundation_scope.md`](foundation_scope.md) (v4.x scope incl. belief / tool / cognitive-eval layers). Earlier snapshots: [`docs/architecture_v2.md`](architecture_v2.md) (v2.0–v2.3), [`docs/kazakh_grammar/07_dialog_architecture.md`](kazakh_grammar/07_dialog_architecture.md) (v1.0.0 MVP).

> **v6.0 forward-looking note (2026-05-16).** The v6.0 release adds an algebra-anchored neural composition layer ([`architecture_neural_v6.md`](architecture_neural_v6.md)). v6.0's evaluation surface extends the cognitive eval + repl-replay datasets with: (a) the held-out generalisation CE measurement from [`research/results_real_mix_2026_05_16.md`](research/results_real_mix_2026_05_16.md), (b) the L5.5 latency bench from [`bench/neural_inference_2026_05_16.md`](bench/neural_inference_2026_05_16.md), and (c) the planned LLM-baseline comparison (v6.0.0 acceptance criterion #4).

## Purpose

The eval layer should produce a deterministic benchmark report from the
versioned benchmark manifest, not just validate that the suite shape is legal.

## Runner

The benchmark report is built through:

- `scripts/run_eval_benchmark_report.sh`
- `scripts/run_eval_benchmark_delta.sh`

## Output

The production eval benchmark regression artifact is stored at:

- `data/eval/benchmark_report.json`
- `data/eval/benchmark_delta_report.json`

The report captures:

- benchmark suite name and target language
- layer count and task count
- category-aware breakdown by eval task family
- critical guard breakdown for deterministic benchmark coverage zones

The delta report captures:

- machine-readable drift summary against the expected benchmark artifact
- field-level drift for suite-wide counts
- category and guard drift for deterministic task families
