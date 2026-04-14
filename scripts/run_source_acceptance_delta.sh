#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

cargo run -p adam-corpus --bin corpus_delta -- \
  data/curated/source_acceptance_report.json \
  data/raw/source_registry.json \
  data/raw/source_scoring_rules.json \
  data/curated/source_acceptance_summary_report.json
