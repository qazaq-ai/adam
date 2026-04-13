#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

cargo run -p adam-train --bin mini_train -- \
  data/training/baseline_training_manifest.json \
  data/curated/mini_clean_training_manifest.json \
  data/curated/clean_training_corpus_manifest.json \
  data/curated/clean_training_corpus_pack.json
