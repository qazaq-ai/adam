#!/usr/bin/env bash
# **v6.0.0-rc4 — post-sweep checkpoint comparison.**
#
# Read metrics directly from each checkpoint's `training.json` +
# `config.json` (committed by poc_kazakh_train at [6c/7]). This is
# the source of truth — survives /tmp wipes / reboots that would
# discard the original training log.
#
# Usage: bash scripts/compare_rc4_sweep.sh
#
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

sweep_dir="data/checkpoints/rc4_sweep"

echo "## rc4 sweep — checkpoint comparison (from training.json)"
echo
printf "| job | d_model × layers × heads | params | data | train CE | held-out CE | gap |\n"
printf "|---|---|---:|---|---:|---:|---:|\n"

best_job=""
best_ce="999.0"

for dir in "$sweep_dir"/*/; do
    [ -d "$dir" ] || continue
    job=$(basename "$dir")
    tr="$dir/training.json"
    cf="$dir/config.json"

    if [ ! -f "$tr" ] || [ ! -f "$cf" ]; then
        printf "| %s | — | — | — | — | — | — |\n" "$job"
        continue
    fi

    d=$(jq -r '.d_model' "$cf")
    l=$(jq -r '.n_layers' "$cf")
    h=$(jq -r '.n_heads' "$cf")
    arch="${d}×${l}×${h}"

    vocab=$(jq -r '.vocab_size' "$cf")
    # Rough params estimate: 2 × vocab × d (in/out embed) + n_layers × (~12 × d²)
    params=$(awk -v v="$vocab" -v d="$d" -v l="$l" 'BEGIN {printf "%.1fM", (2*v*d + l*12*d*d)/1000000}')

    train=$(jq -r '.final_train_ce' "$tr" | xargs printf "%.3f")
    held=$(jq -r '.heldout_ce' "$tr" | xargs printf "%.3f")
    gap=$(awk -v t="$train" -v h="$held" 'BEGIN {printf "%.3f", h-t}')

    # Data hint from job name
    case "$job" in
        *_fst)   data="fst-only";;
        *_mixed) data="fst+real";;
        *)       data="?";;
    esac

    printf "| %s | %s | %s | %s | %s | %s | %s |\n" \
        "$job" "$arch" "$params" "$data" "$train" "$held" "$gap"

    is_better=$(awk -v c="$held" -v b="$best_ce" 'BEGIN {print (c+0 < b+0) ? 1 : 0}')
    [ "$is_better" = "1" ] && { best_ce="$held"; best_job="$job"; }
done

echo
if [ -n "$best_job" ]; then
    echo "**Best checkpoint: \`$best_job\`** (held-out CE = $best_ce)."
    echo
    echo "Load it into adam_chat with:"
    echo
    echo '```'
    echo "cargo run --bin adam_chat --features neural --release -- \\"
    echo "    --neural-model data/checkpoints/rc4_sweep/$best_job"
    echo '```'
else
    echo "_(No checkpoints found in \`$sweep_dir\` — sweep not run yet?)_"
fi
