#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

cargo run -p adam-train --bin assemble_tiny_clean_training -- \
  data/curated/tiny_clean_training_selection_manifest.json \
  data/curated/clean_training_corpus_manifest.json \
  data/curated/clean_training_corpus_pack.json
