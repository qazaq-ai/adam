#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

tmp_acceptance_report="$(mktemp)"
trap 'rm -f "$tmp_acceptance_report"' EXIT

jq empty data/curated/corpus_manifest.json
jq empty data/curated/source_acceptance_report.json
jq empty data/curated/source_acceptance_summary_report.json
jq empty data/curated/source_acceptance_delta_report.json
jq empty data/curated/clean_training_corpus_manifest.json
jq empty data/curated/clean_training_corpus_pack.json
jq empty data/curated/clean_general_extension_pack.json
jq empty data/curated/clean_reference_extension_pack.json
jq empty data/curated/clean_education_extension_pack.json
jq empty data/curated/tokenizer_dry_run_pack.json
jq empty data/curated/tiny_clean_training_manifest.json
jq empty data/curated/tiny_clean_training_selection_manifest.json
jq empty data/curated/tiny_clean_training_profile_suite_manifest.json
jq empty data/curated/tiny_clean_training_profile_baseline_manifest.json
jq empty data/curated/tiny_clean_training_profile_strategy_manifest.json
jq empty data/curated/tiny_clean_general_pack.json
jq empty data/curated/tiny_clean_reference_pack.json
jq empty data/curated/tiny_clean_education_pack.json
jq empty data/curated/tiny_clean_training_pack.json
jq empty data/raw/source_registry.json
jq empty data/raw/source_scoring_rules.json
jq empty data/eval/benchmark_manifest.json
jq empty data/eval/benchmark_report.json
jq empty data/eval/benchmark_delta_report.json
jq empty data/eval/kazakh_foundation_eval_dataset.json
jq empty data/eval/tokenizer_segmentation_eval_dataset.json
jq empty data/eval/tokenizer_experiment_manifest.json
jq empty data/eval/tokenizer_experiment_report.json
jq empty data/eval/tokenizer_experiment_delta_report.json
jq empty data/tokenizer/segmentation_roots.json
jq empty data/tokenizer/segmentation_rules.json
jq empty data/training/baseline_training_manifest.json
jq empty data/training/baseline_training_assembly_report.json
jq empty data/training/baseline_training_consistency_report.json
jq empty data/training/baseline_training_delta_report.json
jq empty data/training/clean_training_corpus_report.json
jq empty data/training/tiny_clean_training_profile_suite_report.json
jq empty data/training/tiny_clean_training_profile_comparison_report.json
jq empty data/training/tiny_clean_training_profile_baseline_report.json
jq empty data/training/tiny_clean_training_profile_baseline_delta_report.json
jq empty data/training/tiny_clean_training_profile_strategy_report.json
jq empty data/training/tiny_clean_training_profile_strategy_delta_report.json
jq empty data/training/tiny_clean_training_report.json
jq empty data/foundation/foundation_overview_report.json
jq empty data/foundation/foundation_overview_delta_report.json
cargo fmt --all --check
cargo test -p adam-corpus --tests -- --nocapture
cargo test -p adam-tokenizer --tests -- --nocapture
cargo test -p adam-eval --tests -- --nocapture
cargo test -p adam-train --tests -- --nocapture
./scripts/generate_source_acceptance_report.sh "$tmp_acceptance_report"
cmp -s "$tmp_acceptance_report" data/curated/source_acceptance_report.json
./scripts/run_source_acceptance_summary.sh
./scripts/run_source_acceptance_delta.sh
./scripts/run_tokenizer_dry_run.sh
./scripts/run_eval_benchmark_report.sh
./scripts/run_eval_benchmark_delta.sh
./scripts/run_tokenizer_segmentation_eval.sh
./scripts/run_tokenizer_experiment.sh
./scripts/run_tokenizer_experiment_delta.sh
./scripts/run_training_baseline_plan.sh
./scripts/run_training_baseline_assembly.sh
./scripts/run_training_baseline_consistency.sh
./scripts/run_training_baseline_delta.sh
./scripts/run_clean_training_corpus_assembly.sh
./scripts/run_clean_training_corpus_report.sh
./scripts/run_tiny_clean_training.sh
./scripts/run_tiny_training_profile_suite.sh
./scripts/run_tiny_training_profile_comparison.sh
./scripts/run_tiny_training_profile_baseline.sh
./scripts/run_tiny_training_profile_baseline_delta.sh
./scripts/run_tiny_training_profile_strategy.sh
./scripts/run_tiny_training_profile_strategy_delta.sh
./scripts/run_foundation_overview.sh
./scripts/run_foundation_overview_delta.sh

echo "foundation validation passed"
