#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

cargo run -p adam-train --bin foundation_overview -- \
  data/curated/source_acceptance_summary_report.json \
  data/curated/source_acceptance_delta_report.json \
  data/eval/tokenizer_experiment_report.json \
  data/eval/tokenizer_experiment_delta_report.json \
  data/eval/benchmark_report.json \
  data/eval/benchmark_delta_report.json \
  data/training/baseline_training_consistency_report.json \
  data/training/baseline_training_delta_report.json \
  data/training/tiny_clean_training_report.json \
  data/training/tiny_clean_training_miss_audit_report.json \
  data/training/tiny_clean_training_miss_audit_delta_report.json \
  data/training/tiny_clean_training_profile_baseline_report.json \
  data/training/tiny_clean_training_profile_baseline_delta_report.json \
  data/training/tiny_clean_training_profile_strategy_report.json \
  data/training/tiny_clean_training_profile_strategy_delta_report.json \
  data/training/tiny_clean_training_profile_experiment_matrix_report.json \
  data/training/tiny_clean_training_profile_experiment_matrix_delta_report.json \
  data/training/tiny_clean_training_profile_experiment_matrix_policy_report.json \
  data/training/tiny_clean_training_profile_experiment_matrix_policy_delta_report.json \
  data/training/tiny_clean_training_profile_promotion_report.json \
  data/training/tiny_clean_training_profile_promotion_delta_report.json
