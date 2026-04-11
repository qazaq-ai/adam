#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

tmp_actual="$(mktemp)"
trap 'rm -f "$tmp_actual"' EXIT

./scripts/run_foundation_overview.sh > "$tmp_actual"

cargo run -p adam-train --bin foundation_delta -- \
  data/foundation/foundation_overview_report.json \
  "$tmp_actual"
