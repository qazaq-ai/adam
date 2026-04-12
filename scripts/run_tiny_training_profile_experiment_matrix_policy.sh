#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

cargo run -p adam-train --bin tiny_profile_experiment_matrix_policy -- \
  data/curated/tiny_clean_training_profile_experiment_matrix_policy_manifest.json \
  data/training/tiny_clean_training_profile_experiment_matrix_report.json
