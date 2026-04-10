# Eval Baseline

## Purpose

The eval layer should produce a deterministic benchmark report from the
versioned benchmark manifest, not just validate that the suite shape is legal.

## Runner

The benchmark report is built through:

- `scripts/run_eval_benchmark_report.sh`

## Output

The production eval benchmark regression artifact is stored at:

- `data/eval/benchmark_report.json`

The report captures:

- benchmark suite name and target language
- layer count and task count
- category-aware breakdown by eval task family
- critical guard breakdown for deterministic benchmark coverage zones
