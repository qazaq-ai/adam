#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

tmp_acceptance_report="$(mktemp)"
trap 'rm -f "$tmp_acceptance_report"' EXIT

jq empty data/curated/corpus_manifest.json
jq empty data/curated/source_acceptance_report.json
jq empty data/curated/tokenizer_dry_run_pack.json
jq empty data/raw/source_registry.json
jq empty data/raw/source_scoring_rules.json
jq empty data/eval/benchmark_manifest.json
jq empty data/eval/kazakh_foundation_eval_dataset.json
jq empty data/eval/tokenizer_segmentation_eval_dataset.json
jq empty data/eval/tokenizer_experiment_manifest.json
jq empty data/tokenizer/segmentation_roots.json
jq empty data/tokenizer/segmentation_rules.json
jq empty data/training/baseline_training_manifest.json
jq empty data/training/baseline_training_assembly_report.json
jq empty data/training/baseline_training_consistency_report.json
cargo fmt --all --check
cargo test -p adam-corpus --tests -- --nocapture
cargo test -p adam-tokenizer --tests -- --nocapture
cargo test -p adam-eval --tests -- --nocapture
cargo test -p adam-train --tests -- --nocapture
./scripts/generate_source_acceptance_report.sh "$tmp_acceptance_report"
cmp -s "$tmp_acceptance_report" data/curated/source_acceptance_report.json
./scripts/run_tokenizer_dry_run.sh
./scripts/run_tokenizer_segmentation_eval.sh
./scripts/run_tokenizer_experiment.sh
./scripts/run_training_baseline_plan.sh
./scripts/run_training_baseline_assembly.sh
./scripts/run_training_baseline_consistency.sh

echo "foundation validation passed"
