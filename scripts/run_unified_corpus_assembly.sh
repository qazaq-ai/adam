#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

cargo run --quiet -p adam-train --bin assemble_unified_corpus -- \
  data/curated/adam_training_corpus_manifest.json \
  > data/curated/adam_training_corpus_pack.json

jq empty data/curated/adam_training_corpus_pack.json
echo "adam_training_corpus_pack.json assembled"
