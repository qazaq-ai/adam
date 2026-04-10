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
- `run_tokenizer_dry_run.sh`
  runs the tokenizer dry-run report from machine-readable manifests
- `generate_source_acceptance_report.sh`
  regenerates the curated source acceptance report from registry and scoring rules
- `run_tokenizer_segmentation_eval.sh`
  runs the tokenizer segmentation evaluation against deterministic roots and rules
- `run_tokenizer_experiment.sh`
  runs the tokenizer experiment report with deterministic segmentation scoring
- `run_training_baseline_plan.sh`
  builds the baseline training plan from current manifests
- `run_training_baseline_assembly.sh`
  builds the deterministic train/validation assembly report from current manifests
- `run_training_baseline_consistency.sh`
  builds the deterministic training consistency report from current manifests
