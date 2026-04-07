#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

cargo run -p adam-tokenizer --bin segmentation_eval -- \
  data/eval/tokenizer_segmentation_eval_dataset.json \
  data/tokenizer/segmentation_roots.json \
  data/tokenizer/segmentation_rules.json
