#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

jq empty data/curated/corpus_manifest.json
jq empty data/curated/tokenizer_dry_run_pack.json
jq empty data/raw/source_registry.json
jq empty data/raw/source_scoring_rules.json
jq empty data/eval/benchmark_manifest.json
jq empty data/eval/kazakh_foundation_eval_dataset.json
jq empty data/eval/tokenizer_experiment_manifest.json
cargo fmt --all --check
cargo test -p adam-corpus --tests -- --nocapture
cargo test -p adam-tokenizer --tests -- --nocapture
cargo test -p adam-eval --tests -- --nocapture

echo "foundation validation passed"
