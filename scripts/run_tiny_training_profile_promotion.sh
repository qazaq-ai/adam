#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

cargo run -p adam-train --bin tiny_profile_promotion -- \
  data/curated/tiny_clean_training_profile_promotion_manifest.json \
  data/training/tiny_clean_training_profile_experiment_matrix_policy_report.json
