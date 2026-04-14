#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

cargo run --release --quiet -p adam-train --bin generation_showcase -- \
  "${1:-data/training/adam_baseline_checkpoint.safetensors}" \
  "${2:-data/training/generation_showcase_report.json}"

jq empty data/training/generation_showcase_report.json
echo "generation_showcase_report.json generated"
