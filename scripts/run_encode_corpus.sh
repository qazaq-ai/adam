#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

cargo run --release --quiet -p adam-tokenizer --bin encode_corpus -- \
  data/curated/adam_training_corpus_pack.json \
  > data/curated/adam_training_ids_pack.json

jq empty data/curated/adam_training_ids_pack.json
echo "adam_training_ids_pack.json generated"
