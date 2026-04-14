#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

if [[ $# -lt 1 ]]; then
  echo "usage: $0 <prompt> [max_new_tokens=32] [temperature=0.0] [top_k=0] [seed=42] [checkpoint=...]" >&2
  exit 1
fi

cargo run --release --quiet -p adam-train --bin generate -- "$@"
