# Training Baseline

## Purpose

Before any real training run, the repository should be able to assemble a
deterministic baseline plan and a reproducible train/validation dry-run from
validated manifests.

## Manifest

The starter plan lives in:

- `data/training/baseline_training_manifest.json`

It binds together:

- curated corpus manifest
- source registry
- source scoring rules
- source acceptance report
- tokenizer experiment manifest
- evaluation suite manifest
- validation split policy

## Runner

The baseline plan is built through:

- `scripts/run_training_baseline_plan.sh`

The deterministic assembly dry-run is built through:

- `scripts/run_training_baseline_assembly.sh`

The deterministic consistency report is built through:

- `scripts/run_training_baseline_consistency.sh`

## Output

The current runners do not train a model yet.

The planning report captures:

- accepted source count
- rejected source count
- corpus name
- tokenizer experiment name
- evaluation task count
- max steps
- batch token budget
- context window
- validation split basis points

The assembly report captures:

- total token budget and full-sequence accounting
- train and validation sequence budgets
- train and validation token budgets
- deterministic per-source allocations weighted by accepted source scores
- category-aware breakdown for domain and source-type allocations
- critical guard buckets for split coverage and concentration zones
- leftover token remainder when the global budget is not divisible by the context window

The expected production assembly is also stored as a machine-readable
regression artifact:

- `data/training/baseline_training_assembly_report.json`
- `data/training/baseline_training_consistency_report.json`
