#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

cargo run -p adam-train --bin tiny_training_miss_audit -- \
  data/training/baseline_training_manifest.json \
  data/curated/tiny_clean_training_profile_suite_manifest.json \
  data/training/tiny_clean_training_profile_promotion_report.json
