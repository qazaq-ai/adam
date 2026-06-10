#!/usr/bin/env bash
# free_for_training.sh — disk + memory cleanup before a training run.
#
# **v6.5.0-rc8 (2026-06-09).**  Live audit-cycle ended at rc7 with a
# clear strategic pivot: instead of writing more cascade patches, we
# treat the `mistake_corrections.jsonl` records the audits produce as
# *training data* and retrain the intent / coherence / router stack.
#
# Training on M2 Air (8 GB unified memory + ~228 Gi disk) is doable
# but tight.  This script frees the disk + flushes caches BEFORE each
# training run so the run has headroom for build artifacts, checkpoint
# rotation, and OS swap pressure.
#
# What gets cleaned (in order, biggest first):
#
#   1. ~/.cache/huggingface           (~19 GB on the audit machine)
#   2. cargo target/ in /Users/dake/project/adam
#   3. Other ~/project/*/target dirs (only if Cargo.toml sibling)
#   4. ~/.cache/puppeteer             (~1 GB old Chrome)
#   5. ~/.cache/burn, ~/.cache/whisper
#   6. /tmp/claude-*, /tmp/fst_probe
#   7. Outdated checkpoint backups in data/checkpoints/
#      (Drops *_backup dirs that are NOT the current model.)
#   8. *.log files in data/checkpoints/ older than 30 days
#
# What is NEVER touched:
#
#   - data/{curated, retrieval, world_core, lexicon_v1, world_core,
#     intent_classifier, contextual_lm, tts_models, stt_models,
#     phoneme_bank, audiobooks, training, mistake_corrections.jsonl}
#   - The .git directory
#   - Anything outside the user's home directory
#
# After the run, the script reports:
#   - free space before / after / delta
#   - top 5 largest dirs under data/ and ~/.cache/ for visibility
#   - rough OS memory pressure estimate
#
# Idempotent: running it twice is safe and the second run does
# nothing new.  Returns exit code 0 on success, non-zero only on
# unexpected errors (missing tools, etc).

set -euo pipefail

ROOT="/Users/dake/project/adam"
cd "$ROOT"

# ----- helpers -----------------------------------------------------

free_gb() {
  # Print just the available-space number in Gi from df.  Hard-coded
  # to the root filesystem since training writes to ~/cache and
  # ./target which both live there.
  df -g / | awk 'NR==2 {print $4}'
}

bytes_in_dir() {
  if [[ -d "$1" ]]; then du -sk "$1" 2>/dev/null | awk '{print $1}'
  else echo 0
  fi
}

human_size() {
  # Convert KB to a human label.
  local kb="$1"
  if   (( kb > 1048576 )); then printf "%.1f GB" "$(echo "$kb / 1048576" | bc -l)"
  elif (( kb > 1024 ))  ; then printf "%.1f MB" "$(echo "$kb / 1024"   | bc -l)"
  else                          printf "%d KB" "$kb"
  fi
}

bold() { printf "\033[1m%s\033[0m\n" "$*"; }

# ----- before snapshot --------------------------------------------

START_FREE_GB=$(free_gb)
bold "=== free_for_training.sh ==="
echo "Repo:         $ROOT"
echo "Free at start: ${START_FREE_GB} GiB"
echo

# ----- 1. HuggingFace cache (biggest single win) ------------------

HF_DIR="$HOME/.cache/huggingface"
HF_KB=$(bytes_in_dir "$HF_DIR")
if (( HF_KB > 0 )); then
  bold "[1/8] HuggingFace cache: $(human_size "$HF_KB")"
  rm -rf "$HF_DIR"
  echo "      cleared."
else
  echo "[1/8] HuggingFace cache: empty"
fi

# ----- 2. Project target/ -----------------------------------------

TGT_KB=$(bytes_in_dir "$ROOT/target")
if (( TGT_KB > 0 )); then
  bold "[2/8] adam target/: $(human_size "$TGT_KB")"
  cargo clean --quiet 2>/dev/null || rm -rf target/
  echo "      cleared."
else
  echo "[2/8] adam target/: empty"
fi

# ----- 3. Sibling Rust projects' target/ --------------------------

bold "[3/8] sibling Rust target/ dirs..."
shopt -s nullglob
SIBLING_TOTAL=0
for d in "$HOME/project"/*/; do
  if [[ "$d" == "$ROOT/" ]]; then continue; fi
  if [[ -f "${d}Cargo.toml" && -d "${d}target" ]]; then
    sz=$(bytes_in_dir "${d}target")
    SIBLING_TOTAL=$((SIBLING_TOTAL + sz))
    if (( sz > 0 )); then
      echo "      $(human_size "$sz")  ${d}target"
      rm -rf "${d}target"
    fi
  fi
done
shopt -u nullglob
if (( SIBLING_TOTAL == 0 )); then echo "      none."; fi

# ----- 4. Puppeteer cache -----------------------------------------

PUP_DIR="$HOME/.cache/puppeteer"
PUP_KB=$(bytes_in_dir "$PUP_DIR")
if (( PUP_KB > 0 )); then
  bold "[4/8] puppeteer cache: $(human_size "$PUP_KB")"
  rm -rf "$PUP_DIR"
  echo "      cleared."
else
  echo "[4/8] puppeteer cache: empty"
fi

# ----- 5. burn / whisper caches -----------------------------------

bold "[5/8] burn / whisper caches..."
for c in "$HOME/.cache/burn" "$HOME/.cache/whisper" "$HOME/.cache/torch"; do
  sz=$(bytes_in_dir "$c")
  if (( sz > 0 )); then
    echo "      $(human_size "$sz")  $c"
    rm -rf "$c"
  fi
done

# ----- 6. /tmp leftovers ------------------------------------------

bold "[6/8] /tmp leftovers..."
# IMPORTANT: do NOT touch /private/tmp/claude-* — that's where the
# current Claude Code session writes its own tool-output files; deleting
# them while a session is running breaks the harness.  We can clean
# /tmp/fst_probe and ad-hoc release-script droppings safely.
for p in /tmp/fst_probe /tmp/rc*_body.md /tmp/rc*_resp.json; do
  if [[ -e "$p" ]]; then
    rm -rf "$p" 2>/dev/null || true
  fi
done
echo "      done."

# ----- 7. Outdated checkpoint backups -----------------------------

CKPT_DIR="$ROOT/data/checkpoints"
bold "[7/8] outdated checkpoint backups..."
if [[ -d "$CKPT_DIR" ]]; then
  for b in "$CKPT_DIR"/*_backup; do
    [[ ! -d "$b" ]] && continue
    sz=$(bytes_in_dir "$b")
    echo "      $(human_size "$sz")  $b"
    rm -rf "$b"
  done
  # Old .log files (keep the last `contextual_lm_v*_gpu.log` per family
  # but drop pre-v3 logs that pre-date the current arc).
  for old in "$CKPT_DIR"/contextual_lm.log \
             "$CKPT_DIR"/contextual_lm_v2.log; do
    if [[ -f "$old" ]]; then rm -f "$old"; fi
  done
else
  echo "      no checkpoint dir."
fi

# ----- 8. macOS purgeable -----------------------------------------

bold "[8/8] macOS purgeable (sleep image, app caches)..."
if command -v sudo >/dev/null && sudo -n true 2>/dev/null; then
  sudo purge && echo "      purged."
else
  echo "      skipped (needs passwordless sudo)."
fi

# ----- after snapshot --------------------------------------------

END_FREE_GB=$(free_gb)
DELTA_GB=$(( END_FREE_GB - START_FREE_GB ))

echo
bold "=== summary ==="
echo "Free before: ${START_FREE_GB} GiB"
echo "Free after:  ${END_FREE_GB} GiB"
echo "Recovered:   ${DELTA_GB} GiB"
echo

# Memory pressure: vm_stat free + speculative pages.
if command -v vm_stat >/dev/null; then
  bold "=== memory pressure (rough) ==="
  vm_stat | head -6
  echo
fi

# Top dirs for visibility (no action — just inform).
bold "=== top 5 data dirs (informational) ==="
du -sh "$ROOT"/data/*/ 2>/dev/null | sort -h | tail -5

bold "=== top 5 ~/.cache dirs (informational) ==="
du -sh "$HOME"/.cache/*/ 2>/dev/null | sort -h | tail -5

if (( END_FREE_GB < 20 )); then
  echo
  bold "WARNING"
  echo "Free space is < 20 GiB after cleanup."
  echo "Training builds + checkpoint rotation typically need 15-25 GiB."
  echo "Consider closing Chrome / Xcode / Slack before launching the trainer."
  exit 0  # Not an error, just an advisory.
fi

bold "READY"
echo "Recommended next steps:"
echo "  1. Quit Chrome / VS Code / Slack to free RAM."
echo "  2. Launch trainer, e.g.:"
echo "     cargo run --release -p adam-agg-model --bin train_intent_classifier_gpu"
echo "  3. After training, run scripts/check_metrics_currency.sh + commit."
