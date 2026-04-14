#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

cargo run --release --quiet -p adam-tokenizer --bin encode_corpus -- \
  data/curated/adam_training_corpus_pack.json \
  data/tokenizer/bpe_vocab.json \
  data/tokenizer/bpe_merges.json \
  data/curated/adam_validation_ids_pack.json \
  "${1:-0.05}" \
  > data/curated/adam_training_ids_pack.json

jq empty data/curated/adam_training_ids_pack.json
jq empty data/curated/adam_validation_ids_pack.json
echo "adam_training_ids_pack.json + adam_validation_ids_pack.json generated"
