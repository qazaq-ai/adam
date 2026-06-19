#!/usr/bin/env bash
# pretrain_cleanup.sh — sudo-free memory cleanup before launching a
# GPU-bound trainer on M2 8GB. Invoked as a PreToolUse hook (see
# .claude/settings.json) and reads the Bash tool_input JSON from stdin.
#
# Behaviour:
#   - Match training-binary patterns; non-match → exit 0 immediately
#   - Clear app caches that bloat between sessions (VSCode, Cursor,
#     Anthropic). Restart Dock to release its shared graphics buffers
#   - Report free_before → free_after in MB
#   - NEVER block: always exit 0 even on warnings (low free RAM, etc.)

set -u

input="$(cat 2>/dev/null || true)"
cmd="$(printf '%s' "$input" | jq -r '.tool_input.command // empty' 2>/dev/null)"

# Match only training binary invocations.
case "$cmd" in
  *train_contextual_lm_gpu*|*train_bpe*|*train_intent_classifier*|*target/release/train_*)
    ;;
  *)
    exit 0
    ;;
esac

free_mb() {
  vm_stat | awk '/Pages free/{printf "%d", $3*16384/1048576}'
}

before="$(free_mb)"

# 1) Drop caches we can drop without sudo.
rm -rf "$HOME/Library/Caches/com.microsoft.VSCode"/* 2>/dev/null || true
rm -rf "$HOME/Library/Caches/com.microsoft.VSCode.ShipIt"/* 2>/dev/null || true
rm -rf "$HOME/Library/Caches/com.todesktop"* 2>/dev/null || true
rm -rf "$HOME/Library/Caches/com.anthropic"* 2>/dev/null || true

# 2) Restart Dock — releases shared GPU buffers without affecting
#    user-visible windows (the Dock just respawns).
killall -KILL Dock 2>/dev/null || true

# Give launchd ~0.5s to respawn Dock so vm_stat catches the new state.
sleep 1

after="$(free_mb)"

# Stderr so the line shows in the hook spinner without polluting stdout
# (stdout JSON, if any, is parsed by the harness).
printf '[pretrain_cleanup] free_before=%s MB → free_after=%s MB\n' \
  "$before" "$after" >&2

if [ "${after:-0}" -lt 500 ]; then
  printf '[pretrain_cleanup] WARNING: free RAM %s MB < 500 MB target — proceeding anyway\n' \
    "$after" >&2
fi

exit 0
