#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

output_path="${1:-data/curated/source_acceptance_report.json}"

cargo run -p adam-corpus --bin generate_acceptance_report -- \
  data/raw/source_registry.json \
  data/raw/source_scoring_rules.json \
  "$output_path"
