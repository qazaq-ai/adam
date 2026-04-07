#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

cargo run -p adam-tokenizer --bin dry_run -- \
  data/eval/tokenizer_experiment_manifest.json \
  data/curated/tokenizer_dry_run_pack.json
