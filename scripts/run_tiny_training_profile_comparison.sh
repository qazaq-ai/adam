#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

cargo run -p adam-train --bin tiny_profile_comparison -- \
  data/training/tiny_clean_training_profile_suite_report.json
