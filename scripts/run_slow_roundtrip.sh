#!/usr/bin/env bash
# Run the slow FST synthesis-analysis roundtrip suite.
#
# These four tests exercise full-Lexicon synthesis of NounFeatures
# variants (plural, dative, p3sg) and one verb form (past 1sg), then
# parse the synthesised surface back. They run for ~30s each on M2
# (~150s total) and are #[ignore]'d from `cargo test --workspace` so
# the default suite stays fast. v4.1.6 adds this wrapper so they
# can be invoked from a nightly job (or manually) without remembering
# the exact `--ignored` flag plumbing.
#
# Pass criterion: each test asserts ≥90% (or ≥95% for the smoke test)
# roundtrip rate; concrete fail rates are eprint'd before the assert.
# Codex v4.1.5 audit recommendation #2: keep this surface visible.
#
# Usage:
#   bash ./scripts/run_slow_roundtrip.sh
#
# Optional:
#   bash ./scripts/run_slow_roundtrip.sh --release   # roughly 4× faster
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/.." && pwd)
cd "$repo_root"

mode="${1:-}"
case "$mode" in
  --release)
    cargo test --release --test roundtrip -p adam-kernel-fst -- --ignored --nocapture
    ;;
  "" )
    cargo test --test roundtrip -p adam-kernel-fst -- --ignored --nocapture
    ;;
  *)
    echo "unknown mode '$mode'; supported: --release" >&2
    exit 2
    ;;
esac
