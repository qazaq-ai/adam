#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

target_vocab="${1:-4096}"

cargo run --release --quiet -p adam-tokenizer --bin train_bpe -- \
  data/curated/adam_pretokenized_corpus_pack.json \
  "$target_vocab" \
  data/tokenizer/bpe_vocab.json \
  data/tokenizer/bpe_merges.json

jq empty data/tokenizer/bpe_vocab.json
jq empty data/tokenizer/bpe_merges.json
echo "BPE artifacts regenerated (target_vocab=$target_vocab)"
