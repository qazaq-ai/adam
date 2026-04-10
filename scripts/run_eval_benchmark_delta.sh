#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

cargo run -p adam-eval --bin delta -- \
  data/eval/benchmark_manifest.json \
  data/eval/benchmark_report.json
