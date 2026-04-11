#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

cargo run -p adam-train --bin tiny_profile_baseline -- \
  data/curated/tiny_clean_training_profile_baseline_manifest.json \
  data/training/tiny_clean_training_profile_comparison_report.json
