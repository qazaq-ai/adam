#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

target_n="${1:-18000}"
seed="${2:-42}"

cargo run --quiet -p adam-corpus --bin synth_sentences -- "$target_n" "$seed" \
  > data/curated/synthetic_sentences_pack.json

jq empty data/curated/synthetic_sentences_pack.json
echo "synthetic_sentences_pack.json regenerated with target_n=$target_n seed=$seed"
