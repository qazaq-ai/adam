#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

cargo run -p adam-train --bin report_clean_corpus -- \
  data/curated/clean_training_corpus_manifest.json \
  data/curated/clean_training_corpus_pack.json
