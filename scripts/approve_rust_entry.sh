#!/usr/bin/env bash
# scripts/approve_rust_entry.sh
#
# v4.28.0 — flip `reviewer: "claude"` → `reviewer: "shaman"` for one
# or more entries in `data/world_core/programming_rust.jsonl` after
# native-speaker review (per docs/rust_glossary_review_v4.28.md).
#
# **Why pure shell + sed.** The repo's `rust_only_contracts` test
# forbids shell scripts from invoking foreign-language runtimes. The
# replacement is a single-line per-entry change that sed handles
# safely — much smaller surface area than a full JSON parse.
#
# Usage:
#     bash scripts/approve_rust_entry.sh rust_111 rust_112 rust_113
#     bash scripts/approve_rust_entry.sh --all   # flip all rust_111…rust_179

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
JSONL="$REPO_ROOT/data/world_core/programming_rust.jsonl"

if [[ $# -eq 0 ]]; then
    echo "usage: $0 <rust_111> [rust_112] ... | --all" >&2
    exit 1
fi

flip_one() {
    local entry_id="$1"
    if ! grep -q "\"id\": \"${entry_id}\"" "$JSONL"; then
        echo "warning: entry ${entry_id} not found in $JSONL" >&2
        return 1
    fi
    if ! grep -E "\"id\": \"${entry_id}\".*\"reviewer\": \"claude\"" "$JSONL" >/dev/null; then
        echo "info: entry ${entry_id} already reviewed (not by claude); skipping"
        return 0
    fi
    # Flip on the matching line only.
    # Using awk: read each line, if it contains both the id and reviewer="claude", replace; else passthrough.
    awk -v id="\"id\": \"${entry_id}\"" \
        '{ if (index($0, id) > 0 && index($0, "\"reviewer\": \"claude\"") > 0) {
              gsub("\"reviewer\": \"claude\"", "\"reviewer\": \"shaman\"")
           }
           print
         }' "$JSONL" > "$JSONL.tmp"
    mv "$JSONL.tmp" "$JSONL"
    echo "flipped ${entry_id}: reviewer claude → shaman"
}

if [[ "$1" == "--all" ]]; then
    flipped=0
    for n in $(seq 111 179); do
        if flip_one "rust_${n}"; then
            flipped=$((flipped + 1))
        fi
    done
    echo ""
    echo "flipped $flipped of 69 entries"
else
    for entry_id in "$@"; do
        flip_one "$entry_id"
    done
fi

echo ""
echo "next steps:"
echo "  cargo run --release --bin validate_world_core    # verify schema"
echo "  cargo run --release --bin extract_facts          # regenerate facts.json"
echo "  cargo run --release --bin run_reasoner           # regenerate derived_facts.json"
