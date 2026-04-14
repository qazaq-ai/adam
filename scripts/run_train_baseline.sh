#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

mkdir -p data/training
cargo run --release --quiet -p adam-train --bin train_baseline -- \
  "${1:-3000}" \
  "${2:-16}" \
  "${3:-64}" \
  "${4:-3e-4}" \
  "${5:-100}" \
  "${6:-100}" \
  "${7:-data/training/adam_baseline_checkpoint.safetensors}" \
  "${8:-42}"
