#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

tmp_actual="$(mktemp)"
trap 'rm -f "$tmp_actual"' EXIT

./scripts/run_tiny_training_miss_audit.sh > "$tmp_actual"

cargo run -p adam-train --bin tiny_training_miss_audit_delta -- \
  data/training/tiny_clean_training_miss_audit_report.json \
  "$tmp_actual"
