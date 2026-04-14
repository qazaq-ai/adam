#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

cargo run --release --quiet -p adam-train --bin eval_perplexity -- \
  "${1:-data/training/adam_baseline_checkpoint.safetensors}" \
  "${2:-data/curated/adam_validation_ids_pack.json}" \
  "${3:-data/training/validation_perplexity_report.json}"
