#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

cargo run --release --quiet -p adam-train --bin data_loader_smoke_test -- "${1:-8}" "${2:-64}" "${3:-42}"
