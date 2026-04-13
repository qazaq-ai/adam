# adam

Core repository for a Kazakh-first language model foundation.

## Purpose

`adam` is a foundation repository for a Kazakh-only text model stack built entirely in Rust.
The immediate goal is not scale theater. The goal is a clean, deterministic Kazakh-first
foundation that prevents early contamination and weak evaluation habits.

- strict corpus policy before any training data is accepted
- deterministic morphological segmentation with a finite-state machine
- hard evaluation harness that must pass before any version is released
- small-model training path with validated train/validation splits
- every layer produces a golden artifact that is regression-tested

## Architecture

The stack is organized into three layers:

| Layer | Crate | Responsibility |
|-------|-------|----------------|
| L0 | `adam-kernel` | Kazakh identity types, FSM morphological engine, `KernelError` |
| L1 | `adam-tokenizer` | Tokenizer pipeline, segmentation experiments, experiment reports |
| L1 | `adam-corpus` | Corpus manifests, source acceptance, curation contracts |
| L1 | `adam-eval` | Evaluation task definitions and benchmark summary types |
| L2 | `adam-train` | Training profiles, sequence packs, baseline planning runners |

## Repository Layout

- `crates/adam-kernel`
  L0 foundation: model identity constants, `ModelIdentity`, `FoundationPrinciples`,
  all FSM morphological types (`SegmentationLexicon`, `SegmentationRuleSet`, etc.),
  `deterministic_segment_token`, `KernelError`, and the `coverage_report` binary
- `crates/adam-tokenizer`
  L1 tokenizer: tokenizer configuration, segmentation eval, experiment reports;
  depends on `adam-kernel` and re-exports its FSM types
- `crates/adam-corpus`
  corpus manifests, source metadata, and curation contracts
- `crates/adam-eval`
  evaluation task definitions and benchmark summary types
- `crates/adam-train`
  baseline training manifests, profile selection, and training runners
- `data/`
  raw, curated, evaluation, tokenizer, and training dataset roots
- `docs/`
  scope, architecture, and policy documents
- `scripts/`
  shell runners for every pipeline step and foundation validation

## Key Binaries

| Binary | Crate | Purpose |
|--------|-------|---------|
| `coverage_report` | adam-kernel | Measure FSM segmentation coverage on real Kazakh text |
| `run_experiment` | adam-tokenizer | Run a full tokenizer segmentation experiment |
| `segmentation_eval` | adam-tokenizer | Evaluate segmentation against the eval dataset |
| `delta` | adam-tokenizer | Detect drift in experiment golden artifacts |
| `report` | adam-eval | Build the eval benchmark report |
| `report` | adam-corpus | Build the corpus acceptance report |

## Current Scope

This repository targets a Kazakh-first text model only.

Out of scope for the current foundation phase:

- multilingual expansion
- speech or multimodal
- cloud platform work
- chat product features

## First Principle

The repo grows from clean data and hard evaluation, not from broad claims.

## Foundation Validation

The full foundation validation is a single command:

```bash
bash ./scripts/validate_foundation.sh
```

It verifies every layer in order: corpus → tokenizer → eval → train → tiny training →
miss audit → profile policy → strategy → experiment matrix → promotion.
All layers must be green before a release is cut.

## Release Flow

Versioning is deterministic and script-enforced.

- validate the foundation: `bash ./scripts/validate_foundation.sh`
- verify the version across all manifests and Cargo.lock: `bash ./scripts/verify_release_version.sh x.y.z`
- cut and publish a release: `bash ./scripts/cut_release.sh x.y.z`
- pushing tag `vX.Y.Z` triggers CI: format check → version verification → full foundation validation

## Foundation Policies

- [corpus policy](docs/corpus_policy.md)
- [curation workflow](docs/curation_workflow.md)
- [source classification](docs/source_classification.md)
- [source scoring](docs/source_scoring.md)
- [tokenizer policy](docs/tokenizer_policy.md)
- [tokenizer experiment plan](docs/tokenizer_experiment_plan.md)
- [tokenizer dry run](docs/tokenizer_dry_run.md)
- [tokenizer segmentation eval](docs/tokenizer_segmentation_eval.md)
- [evaluation policy](docs/evaluation_policy.md)
- [training baseline](docs/training_baseline.md)
