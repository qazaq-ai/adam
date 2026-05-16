# Training Baseline

> **Legacy context (transformer era, v0.1 – v0.4).** The v1.0.0 dialog pipeline does not train or evaluate a transformer. The assembly contracts and regression reports documented here are preserved because `scripts/validate_foundation.sh` still exercises them as CI regression surface (see `data/training/README.md`). For the v1.0.0 architecture see [`docs/kazakh_grammar/07_dialog_architecture.md`](kazakh_grammar/07_dialog_architecture.md).

> **v6.0 forward-looking note (2026-05-16).** The v0.4 transformer described here is unrelated to the v6.0 algebra-anchored neural composition layer. The v6.0 training pipeline (FST-validated synthetic + Root-decomposed real corpus, ~109k pairs, ~1M parameters, M2 CPU, 39 minutes end-to-end) is documented in [`research/results_real_mix_2026_05_16.md`](research/results_real_mix_2026_05_16.md). The training source binary is `crates/adam-agg-model/src/bin/poc_kazakh_train.rs`.

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

The deterministic delta report is built through:

- `scripts/run_training_baseline_delta.sh`

The first tiny clean training prototype is built through:

- `scripts/run_tiny_clean_training_assembly.sh`
- `scripts/run_tiny_clean_training.sh`
- `scripts/run_tiny_training_miss_audit.sh`
- `scripts/run_tiny_training_miss_audit_delta.sh`

The next deterministic mini clean prototype is built through:

- `scripts/run_mini_clean_training_assembly.sh`
- `scripts/run_mini_clean_training.sh`
- `scripts/run_mini_training_miss_audit.sh`

The next clean corpus tier is assembled through:

- `scripts/run_clean_training_corpus_assembly.sh`
- `scripts/run_clean_training_corpus_report.sh`

Controlled tiny profile comparisons are built through:

- `scripts/run_tiny_training_profile_suite.sh`
- `scripts/run_tiny_training_profile_comparison.sh`
- `scripts/run_tiny_training_profile_baseline.sh`
- `scripts/run_tiny_training_profile_baseline_delta.sh`
- `scripts/run_tiny_training_profile_strategy.sh`
- `scripts/run_tiny_training_profile_strategy_delta.sh`
- `scripts/run_tiny_training_profile_promotion.sh`
- `scripts/run_tiny_training_profile_promotion_delta.sh`
- `scripts/run_tiny_training_profile_experiment_matrix.sh`
- `scripts/run_tiny_training_profile_experiment_matrix_delta.sh`
- `scripts/run_tiny_training_profile_experiment_matrix_policy.sh`
- `scripts/run_tiny_training_profile_experiment_matrix_policy_delta.sh`

Its clean corpus is now sourced through:

- `data/curated/tiny_clean_training_selection_manifest.json`
- `data/curated/tiny_clean_training_profile_suite_manifest.json`
- `data/curated/tiny_clean_training_profile_baseline_manifest.json`
- `data/curated/tiny_clean_training_profile_strategy_manifest.json`
- `data/curated/tiny_clean_training_profile_promotion_manifest.json`
- `data/curated/tiny_clean_training_profile_experiment_matrix_manifest.json`
- `data/curated/tiny_clean_training_profile_experiment_matrix_policy_manifest.json`
- `data/curated/tiny_clean_training_manifest.json`
- `data/curated/tiny_clean_general_pack.json`
- `data/curated/tiny_clean_reference_pack.json`
- `data/curated/tiny_clean_education_pack.json`
- `data/curated/tiny_clean_training_pack.json` as the assembled regression artifact

The larger clean corpus tier is sourced through:

- `data/curated/clean_training_corpus_manifest.json`
- `data/curated/clean_general_extension_pack.json`
- `data/curated/clean_reference_extension_pack.json`
- `data/curated/clean_education_extension_pack.json`
- `data/curated/clean_training_corpus_pack.json`
- `data/curated/mini_clean_training_manifest.json`
- `data/curated/mini_clean_training_pack.json`
- `data/training/clean_training_corpus_report.json`
- `data/training/mini_clean_training_report.json`
- `data/training/mini_clean_training_miss_audit_report.json`
- `data/training/tiny_clean_training_profile_suite_report.json`
- `data/training/tiny_clean_training_profile_comparison_report.json`
- `data/training/tiny_clean_training_profile_baseline_report.json`
- `data/training/tiny_clean_training_profile_baseline_delta_report.json`
- `data/training/tiny_clean_training_profile_strategy_report.json`
- `data/training/tiny_clean_training_profile_strategy_delta_report.json`
- `data/training/tiny_clean_training_profile_promotion_report.json`
- `data/training/tiny_clean_training_profile_promotion_delta_report.json`
- `data/training/tiny_clean_training_profile_experiment_matrix_report.json`
- `data/training/tiny_clean_training_profile_experiment_matrix_delta_report.json`
- `data/training/tiny_clean_training_profile_experiment_matrix_policy_report.json`
- `data/training/tiny_clean_training_profile_experiment_matrix_policy_delta_report.json`
- `data/training/tiny_clean_training_miss_audit_report.json`
- `data/training/tiny_clean_training_miss_audit_delta_report.json`

## Output

The current runners do not train a model yet.

The tiny clean prototype is the first exception: it trains a minimal
deterministic next-token baseline on a curated clean pack assembled from
domain-aware clean artifacts that contain only accepted training sources.

The new mini clean prototype takes the next step: it trains on a larger
reference-heavy clean subset from the same corpus, using the same deterministic
ordering and stratified validation discipline. This gives a more honest
downstream signal than the tiny tier and exposes unresolved ambiguity and
unseen-context misses as explicit clean-corpus targets.

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
- `data/training/baseline_training_delta_report.json`
- `data/training/tiny_clean_training_report.json`
- `data/training/tiny_clean_training_profile_suite_report.json`
- `data/training/tiny_clean_training_profile_comparison_report.json`
- `data/training/tiny_clean_training_profile_baseline_report.json`
- `data/training/tiny_clean_training_profile_baseline_delta_report.json`
- `data/training/tiny_clean_training_profile_strategy_report.json`
- `data/training/tiny_clean_training_profile_strategy_delta_report.json`
- `data/training/tiny_clean_training_profile_promotion_report.json`
- `data/training/tiny_clean_training_profile_promotion_delta_report.json`
- `data/training/tiny_clean_training_profile_experiment_matrix_report.json`
- `data/training/tiny_clean_training_profile_experiment_matrix_delta_report.json`
- `data/training/tiny_clean_training_profile_experiment_matrix_policy_report.json`
- `data/training/tiny_clean_training_profile_experiment_matrix_policy_delta_report.json`

The foundation-wide cross-layer summary is also stored as:

- `data/foundation/foundation_overview_report.json`
- `data/foundation/foundation_overview_delta_report.json`

The foundation overview now also requires:

- `data/training/tiny_clean_training_miss_audit_report.json`
- `data/training/tiny_clean_training_miss_audit_delta_report.json`
- `data/training/tiny_clean_training_profile_baseline_delta_report.json`
- `data/training/tiny_clean_training_profile_strategy_delta_report.json`
- `data/training/tiny_clean_training_profile_promotion_delta_report.json`
- `data/training/tiny_clean_training_profile_experiment_matrix_delta_report.json`
- `data/training/tiny_clean_training_profile_experiment_matrix_policy_delta_report.json`

The active tiny training pack is no longer selected from the static
`tiny_clean_training_selection_manifest.json` path. It is now promoted from the
profile suite through:

- `data/curated/tiny_clean_training_profile_promotion_manifest.json`
- `data/training/tiny_clean_training_profile_promotion_report.json`

This means the tiny training report reflects the currently promoted profile from
the matrix-based profile policy layer.

The next controlled experiment layer now fixes downstream tiny training behavior
for the promotable profile set through:

- `data/curated/tiny_clean_training_profile_experiment_matrix_manifest.json`
- `data/training/tiny_clean_training_profile_experiment_matrix_report.json`
- `data/curated/tiny_clean_training_profile_experiment_matrix_policy_manifest.json`
- `data/training/tiny_clean_training_profile_experiment_matrix_policy_report.json`

The experiment matrix policy report now records per-candidate eligibility
decisions and explicit rejection reasons, so weak profiles are preserved as
machine-readable contract data instead of disappearing behind the selected
profile summary.

The tiny clean training split now uses deterministic round-robin domain
ordering together with a stratified validation split. This keeps the tiny
training holdout reproducible while preventing the old tail-only validation
bias from collapsing the evaluation into a single dominant domain.

The tiny training layer now also publishes a miss-audit artifact. This keeps
the `validation_exact_match_rate_bps` metric tied to concrete next-token misses
instead of hiding them behind a single aggregate score. The miss audit is
promoted into the foundation overview, so cross-layer readiness now tracks not
only tiny training quality, but also whether the exact miss set still matches
the expected deterministic baseline.
