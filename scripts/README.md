# Scripts

This directory will hold repeatable local utilities for:

- corpus inspection
- tokenizer training
- evaluation runs
- release verification

Current starter utility:

- `validate_foundation.sh`
  validates manifests, formatting, and foundation contract tests
- `run_tokenizer_dry_run.sh`
  runs the tokenizer dry-run report from machine-readable manifests
- `generate_source_acceptance_report.sh`
  regenerates the curated source acceptance report from registry and scoring rules
- `run_tokenizer_segmentation_eval.sh`
  runs the tokenizer segmentation reference evaluation report
- `run_tokenizer_experiment.sh`
  runs the tokenizer experiment report with real segmentation scoring
- `run_training_baseline_plan.sh`
  builds the baseline training plan from current manifests
