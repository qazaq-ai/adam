#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

cargo run -p adam-tokenizer --bin delta -- \
  data/eval/tokenizer_experiment_manifest.json \
  data/eval/tokenizer_experiment_report.json
