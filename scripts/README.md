# Scripts

This directory will hold repeatable local utilities for:

- corpus inspection
- tokenizer training
- evaluation runs
- release verification

Current starter utility:

- `validate_foundation.sh`
  validates manifests, formatting, and foundation contract tests
- `verify_release_version.sh`
  checks that workspace, manifests, and lockfile versions match a target release
- `bump_foundation_version.sh`
  updates the foundation version, verifies it, and runs full validation
- `cut_release.sh`
  performs a clean release cut: bump, validate, commit, push, tag, and trigger GitHub Release
- `run_slow_roundtrip.sh` (v4.1.6+)
  runs the four `#[ignore]`d FST synthesis-analysis roundtrip tests in
  `crates/adam-kernel-fst/tests/roundtrip.rs` (noun plural / dative /
  p3sg + verb past 1sg). ~150s on M2; ~40s with `--release`. Held out
  of the default `cargo test --workspace` because of duration but kept
  green and ready for periodic / nightly invocation. Codex v4.1.5
  audit recommendation #2.
- `run_tokenizer_dry_run.sh`
  runs the tokenizer dry-run report from machine-readable manifests
- `run_eval_benchmark_report.sh`
  builds the deterministic eval benchmark report from the benchmark manifest
- `run_eval_benchmark_delta.sh`
  builds the deterministic eval benchmark delta report against the expected benchmark artifact
- `generate_source_acceptance_report.sh`
  regenerates the curated source acceptance report from registry and scoring rules
- `run_source_acceptance_summary.sh`
  builds the deterministic source acceptance summary report from current artifacts
- `run_source_acceptance_delta.sh`
  builds the deterministic source acceptance delta report against the expected summary artifact
- `run_tokenizer_segmentation_eval.sh`
  runs the tokenizer segmentation evaluation against deterministic roots and rules
- `run_tokenizer_experiment.sh`
  runs the tokenizer experiment report with deterministic segmentation scoring
- `run_tokenizer_experiment_delta.sh`
  builds the tokenizer experiment drift summary against the expected experiment artifact
- `run_training_baseline_plan.sh`
  builds the baseline training plan from current manifests
- `run_training_baseline_assembly.sh`
  builds the deterministic train/validation assembly report from current manifests
- `run_training_baseline_consistency.sh`
  builds the deterministic training consistency report from current manifests
- `run_training_baseline_delta.sh`
  builds the deterministic training delta report against the expected assembly and consistency artifacts
- `run_tiny_training_profile_suite.sh`
  builds the deterministic tiny clean profile suite report from the clean corpus
- `run_tiny_training_profile_comparison.sh`
  compares the deterministic tiny profile suite and records profile gaps
- `run_tiny_training_profile_baseline.sh`
  checks the expected baseline tiny profile policy against the comparison report
- `run_tiny_training_profile_baseline_delta.sh`
  builds the deterministic tiny profile baseline drift report
- `run_tiny_training_profile_strategy.sh`
  derives promotable tiny profile candidates from the current comparison and baseline reports
- `run_tiny_training_profile_strategy_delta.sh`
  builds the deterministic tiny profile strategy drift report
- `run_tiny_training_profile_promotion.sh`
  promotes the active tiny training profile from the matrix policy report
- `run_tiny_training_profile_promotion_delta.sh`
  builds the deterministic tiny profile promotion drift report
- `run_tiny_training_profile_experiment_matrix.sh`
  builds the downstream tiny training experiment matrix for promotable profiles
- `run_tiny_training_profile_experiment_matrix_delta.sh`
  builds the deterministic drift report for the tiny training experiment matrix
- `run_tiny_training_profile_experiment_matrix_policy.sh`
  applies threshold and ranking rules to the experiment matrix to select the active profile
- `run_tiny_training_profile_experiment_matrix_policy_delta.sh`
  builds the deterministic drift report for the experiment matrix policy layer
- `run_tiny_clean_training_assembly.sh`
  assembles the deterministic tiny clean training pack from the selection manifest and clean corpus pack
- `run_tiny_clean_training.sh`
  trains a tiny deterministic prototype on the accepted clean training pack
- `run_tiny_training_miss_audit.sh`
  builds a machine-readable audit of validation misses for the active tiny training profile
- `run_tiny_training_miss_audit_delta.sh`
  builds the deterministic drift report for the tiny training miss audit layer
- `run_mini_clean_training_assembly.sh`
  assembles the deterministic mini clean training pack from the clean corpus manifest
- `run_mini_clean_training.sh`
  trains the larger deterministic mini clean prototype on the accepted clean corpus subset
- `run_mini_training_miss_audit.sh`
  builds a machine-readable audit of validation misses for the mini clean training prototype
- `run_foundation_overview.sh`
  builds the unified cross-layer foundation overview from corpus, tokenizer, eval, and training artifacts
- `run_foundation_overview_delta.sh`
  builds the unified foundation drift summary against the expected overview artifact
