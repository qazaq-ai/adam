# adam

Core repository for a Kazakh-first language model.

## Purpose

`adam` is a new foundation repository for a Kazakh-only text model stack.
It is separate from `qazaq-ir`.

- `qazaq-ir` remains the deterministic linguistic engine
- `adam` is the future model, data, tokenizer, and evaluation stack

The immediate goal is not scale theater. The immediate goal is to build a clean
Kazakh-first foundation:

- corpus policy
- tokenizer policy
- evaluation harness
- small-model training path
- deterministic linguistic validation hooks

The first practical milestone is not a giant model. It is a strict foundation
that prevents early contamination and weak evaluation habits.

## Repository Layout

- `crates/adam-core`
  shared model-facing types and repository constants
- `crates/adam-tokenizer`
  tokenizer configuration and segmentation interfaces
- `crates/adam-corpus`
  corpus manifests, metadata, and curation contracts
- `crates/adam-eval`
  evaluation task definitions and benchmark summary types
- `crates/adam-train`
  baseline training manifests and planning runners
- `data/`
  raw, curated, and evaluation dataset roots
- `docs/`
  scope, architecture, and roadmap documents

## Current Scope

This repository is for a Kazakh-first text model only.

Out of scope for the current foundation phase:

- multilingual expansion
- speech
- multimodal
- cloud platform work
- chat product features

## First Principle

The repo should grow from clean data and hard evaluation, not from broad claims.

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
