# Eval Baseline

> **Legacy context (transformer era, v0.1 – v0.4).** Describes benchmark assembly for the pre-v0.4.5 transformer baseline. The current adam architecture (v1.0.0 → v3.0) validates correctness via `cargo test --workspace` (**357 passing** as of v3.0) + `scripts/validate_foundation.sh`, not via a benchmark perplexity score. For the current architecture see [`docs/architecture_v3.md`](architecture_v3.md). Earlier snapshots: [`docs/architecture_v2.md`](architecture_v2.md) (v2.0–v2.3 retrieval era), [`docs/kazakh_grammar/07_dialog_architecture.md`](kazakh_grammar/07_dialog_architecture.md) (v1.0.0 MVP).

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
