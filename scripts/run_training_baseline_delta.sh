#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

cargo run -p adam-train --bin delta -- \
  data/training/baseline_training_manifest.json \
  data/training/baseline_training_assembly_report.json \
  data/training/baseline_training_consistency_report.json
