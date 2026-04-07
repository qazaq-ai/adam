# Training Baseline

## Purpose

Before any real training run, the repository should be able to assemble a
deterministic baseline plan from validated manifests.

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

## Runner

The baseline plan is built through:

- `scripts/run_training_baseline_plan.sh`

## Output

The current runner does not train a model yet.

It produces a reproducible planning report with:

- accepted source count
- rejected source count
- tokenizer experiment name
- evaluation task count
- max steps
- batch token budget
- context window
