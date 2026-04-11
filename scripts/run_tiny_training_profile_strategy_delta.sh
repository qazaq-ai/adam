#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

tmp_actual_report="$(mktemp)"
trap 'rm -f "$tmp_actual_report"' EXIT

./scripts/run_tiny_training_profile_strategy.sh > "$tmp_actual_report"

cargo run -p adam-train --bin tiny_profile_strategy_delta -- \
  data/training/tiny_clean_training_profile_strategy_report.json \
  "$tmp_actual_report"
