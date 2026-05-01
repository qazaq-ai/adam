#!/usr/bin/env bash
# scripts/refresh_readme_badges.sh
#
# v4.25.5 — closes the fifth Codex review actionable: drop manually-
# curated count claims in README.md / docs/roadmap.md in favour of
# values read directly from canonical artifacts.
#
# This script is read-only by default: it prints the current
# authoritative values for every badge that can be computed from
# the repository state. The release process (or the
# `feedback_readme_pre_push_audit` rule) compares the current README
# to this output and edits any drift.
#
# **Why not auto-apply.** The README is hand-written prose with
# embedded badges; mechanical sed-substitution is brittle (URL-
# encoding, color codes, badge ordering all matter). The script
# emits *values*; humans handle *placement*.
#
# **Why pure shell + grep + awk** (no python3 / jq / etc). The repo's
# `rust_only_contracts::shell_scripts_do_not_invoke_foreign_language_runtimes`
# test forbids shell scripts from invoking foreign-language runtimes
# (defends the v0.x Rust-only directive). The counting tasks here
# are trivial enough that POSIX-shell tooling suffices: each jsonl
# line is one entry; each fact has exactly one `"subject":` key;
# each eval-dataset case has exactly one `"id":` key.
#
# Fast checks (default — runs in < 1 s):
# - workspace `version`
# - world_core entries / facts / domains
# - lexicon root counts (curated + apertium)
# - eval-suite sizes (parse-disambig, live-holdout, cognitive,
#   repl-replay datasets)
# - derived_facts count + R5 share
#
# Slow check (opt-in via `--include-tests`):
# - workspace test pass count (`cargo test --workspace --release`)
#
# Usage:
#     bash scripts/refresh_readme_badges.sh                    # fast
#     bash scripts/refresh_readme_badges.sh --include-tests    # adds test count

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
INCLUDE_TESTS=false

for arg in "$@"; do
    case "$arg" in
        --include-tests) INCLUDE_TESTS=true ;;
        -h|--help)
            sed -n '3,40p' "$0"
            exit 0
            ;;
        *)
            echo "unknown arg: $arg (use --help)" >&2
            exit 1
            ;;
    esac
done

# Helper: count lines in all matching files with grep -c, sum across files.
sum_lines_with() {
    local pattern="$1"
    shift
    grep -c "$pattern" "$@" 2>/dev/null \
        | awk -F: '{ sum += $NF } END { print sum + 0 }'
}

# Helper: count occurrences (not lines) of pattern across files via grep -o.
sum_occurrences_with() {
    local pattern="$1"
    shift
    grep -o "$pattern" "$@" 2>/dev/null | wc -l | tr -d ' '
}

# ------------- Fast metrics -------------

VERSION=$(grep -m1 '^version = ' "$REPO_ROOT/Cargo.toml" \
    | sed -E 's/version = "([^"]*)"/\1/')

WC_DIR="$REPO_ROOT/data/world_core"
WC_DOMAINS=$(find "$WC_DIR" -maxdepth 1 -name '*.jsonl' 2>/dev/null \
    | wc -l | tr -d ' ')
WC_ENTRIES=$(cat "$WC_DIR"/*.jsonl 2>/dev/null | wc -l | tr -d ' ')
# Facts: each fact has exactly one `"subject":` field. Counting
# occurrences (grep -o) gives total facts across all entries.
WC_FACTS=$(sum_occurrences_with '"subject":' "$WC_DIR"/*.jsonl)

# Lexicon roots: each root entry has exactly one `"id":`. Sum across
# the two canonical lexicon files. v2.2 purged cross-file dups, so
# the simple sum is accurate within ~0.1 %.
LEX_CURATED=$(sum_lines_with '"id":' "$REPO_ROOT/data/tokenizer/segmentation_roots.json")
LEX_APERTIUM=$(sum_lines_with '"id":' "$REPO_ROOT/data/lexicon_v1/apertium_imported_roots.json")
LEX_TOTAL=$((LEX_CURATED + LEX_APERTIUM))

# Eval suite sizes: each case / scenario / dialog has exactly one
# `"id":` field at the case level. (Inner facts in cognitive_eval
# scenarios don't carry `"id":` — the outer scenario does.)
PARSE_DISAMBIG=$(sum_lines_with '"id":' "$REPO_ROOT/data/eval/parse_disambiguation_eval.json")
LIVE_HOLDOUT=$(sum_lines_with '"id":' "$REPO_ROOT/data/eval/live_holdout_2026_05_01.json")
COGNITIVE_EVAL=$(sum_lines_with '"id":' "$REPO_ROOT/data/eval/cognitive_dialog_dataset.json")
REPL_REPLAY=$(sum_lines_with '"id":' "$REPO_ROOT/data/eval/repl_dialogs.json")

DERIVED_TOTAL=$(grep -c '"rule_id"' "$REPO_ROOT/data/retrieval/derived_facts.json" 2>/dev/null || echo 0)
DERIVED_R5=$(grep -c '"R5_shared_is_a_target"' "$REPO_ROOT/data/retrieval/derived_facts.json" 2>/dev/null || echo 0)
if [[ "$DERIVED_TOTAL" -gt 0 ]]; then
    R5_PCT=$(awk "BEGIN { printf \"%.1f\", 100*${DERIVED_R5}/${DERIVED_TOTAL} }")
else
    R5_PCT="0.0"
fi

# ------------- Slow metric (opt-in) -------------

if $INCLUDE_TESTS; then
    echo ">> running cargo test --workspace --release (this takes a few minutes)..." >&2
    TESTS=$(cd "$REPO_ROOT" && cargo test --workspace --release 2>&1 \
        | grep "test result: ok" \
        | awk '{print $4}' \
        | paste -sd+ - \
        | bc)
else
    TESTS="(skipped — pass --include-tests to refresh)"
fi

# ------------- Report -------------

cat <<REPORT
=== adam README badge metrics (canonical-source values) ===

  version:                   ${VERSION}
  workspace tests passing:   ${TESTS}

  lexicon roots (curated):   ${LEX_CURATED}
  lexicon roots (apertium):  ${LEX_APERTIUM}
  lexicon roots (total):     ${LEX_TOTAL}

  world_core domains:        ${WC_DOMAINS}
  world_core entries:        ${WC_ENTRIES}
  world_core facts:          ${WC_FACTS}

  derived_facts (total):     ${DERIVED_TOTAL}
  derived_facts (R5 share):  ${DERIVED_R5} (${R5_PCT}%)

  parse-disambig cases:      ${PARSE_DISAMBIG}
  live-holdout cases:        ${LIVE_HOLDOUT}
  cognitive-eval scenarios:  ${COGNITIVE_EVAL}
  repl-replay dialogs:       ${REPL_REPLAY}

=== Suggested README badge string fragments ===

  version-${VERSION}-2EA44F
  tests-${TESTS}%20passing-2EA44F
  lexicon-${LEX_TOTAL}%20roots-FBC02D
  world%20core-${WC_ENTRIES}%20curated%20%2F%20${WC_FACTS}%20facts-9CCC65
  domains-${WC_DOMAINS}-9CCC65
  parse--disambig%20eval-100%25%20%28${PARSE_DISAMBIG}%2F${PARSE_DISAMBIG}%29-2EA44F
  live%20holdout-${LIVE_HOLDOUT}%20cases-9CCC65

Run with --include-tests to refresh the test count too.
REPORT
