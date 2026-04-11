#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

cargo run -p adam-train --bin tiny_profile_suite -- \
  data/training/baseline_training_manifest.json \
  data/curated/tiny_clean_training_profile_suite_manifest.json
