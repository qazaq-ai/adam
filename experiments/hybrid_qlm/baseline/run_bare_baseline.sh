#!/usr/bin/env bash
# Hybrid QLM — Day 1-3 bare-baseline harness.
#
# Measures the chosen base model (Gemma-3 4B q4_k_m, see
# experiments/hybrid_qlm/baseline/day_1.md) on a small slice of
# the existing eval suite WITHOUT the adam-kernel verifier.
# This is the honest «what does the LM alone do on our Kazakh
# probes» measurement that anchors every later experiment.
#
# Output: experiments/hybrid_qlm/baseline/day_3_results.md
#
# Setup prerequisites (run once, manually):
#   1. brew install llama.cpp
#   2. Download Gemma-3 4B q4_k_m gguf to
#      data/lm_models/gemma-3-4b-it-q4_k_m.gguf
#   (See day_1.md for the why and the source.)
#
# This script does NOT download models — keeps git clean and
# leaves dependency provenance with the user.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$REPO_ROOT"

LM_MODEL="${LM_MODEL:-data/lm_models/gemma-3-4b-it-q4_k_m.gguf}"
LLAMA_CLI="${LLAMA_CLI:-llama-cli}"
OUT_FILE="${OUT_FILE:-experiments/hybrid_qlm/baseline/day_3_results.md}"
N_PREDICT="${N_PREDICT:-128}"
TEMP="${TEMP:-0.2}"

if ! command -v "$LLAMA_CLI" >/dev/null 2>&1; then
    echo "ERROR: $LLAMA_CLI not on PATH.  Run: brew install llama.cpp" >&2
    exit 1
fi

if [[ ! -f "$LM_MODEL" ]]; then
    echo "ERROR: model not found at $LM_MODEL" >&2
    echo "       See experiments/hybrid_qlm/baseline/day_1.md for setup." >&2
    exit 1
fi

# RAM hygiene reminder — Gemma 4B q4_k_m + adam runtime + this
# script's Python + macOS itself want around 5 GB combined on
# an 8 GB M2.  Close IDE / browser / Slack before running.
echo "[bare-baseline] using model: $LM_MODEL"
echo "[bare-baseline] llama-cli  : $($LLAMA_CLI --version 2>&1 | head -1)"
echo "[bare-baseline] free RAM (before run):"
vm_stat | awk '/Pages free/{print "  " $0}'

# 10-query probe — a small slice of school_program, geography,
# capital queries, and a known-defective speech-defect input.
# Each query is run through the bare LM with NO adam-kernel
# in the loop, so we see honest LM output.  The Kazakh system
# prompt asks for short factual answers in Kazakh.
QUERIES=(
    "Қазақстанның астанасы қай қала?"
    "Темірдің химиялық таңбасы қандай?"
    "Күмістің химиялық таңбасы қандай?"
    "Судың формуласы қандай?"
    "Жүрек не үшін керек?"
    "Ахмет Байтұрсынұлы кім?"
    "Қазақстанның ұлттық валютасы."
    "Алматы туралы қысқаша айтшы."
    "СИЗ беру тәртібі қандай?"
    "Хазах деген не?"
)

SYSTEM_PROMPT="Сен қазақша жауап бересің. Қысқа, нақты, шынайы жауап. Біле алмасаң — «білмеймін» деп жаз. Ойдан тауып айтпа."

mkdir -p "$(dirname "$OUT_FILE")"
{
    echo "# Day 3 — bare-baseline results"
    echo ""
    echo "**Model:** \`$LM_MODEL\`"
    echo "**llama-cli:** \`$($LLAMA_CLI --version 2>&1 | head -1)\`"
    echo "**System prompt:** \`$SYSTEM_PROMPT\`"
    echo "**Sampling:** temperature=$TEMP, n_predict=$N_PREDICT"
    echo ""
    echo "## Probes"
    echo ""
} > "$OUT_FILE"

for q in "${QUERIES[@]}"; do
    echo "[bare-baseline] probe: $q"
    # Single-turn chat-style invocation.  -no-cnv keeps llama-cli
    # in non-conversational mode so we get a single completion
    # rather than an interactive session.
    RESP=$("$LLAMA_CLI" \
        -m "$LM_MODEL" \
        -p "$SYSTEM_PROMPT

Сұрақ: $q
Жауап:" \
        -n "$N_PREDICT" \
        --temp "$TEMP" \
        -no-cnv \
        2>/dev/null | tail -n +2 | head -n 20 | tr '\n' ' ')

    {
        echo "### \`$q\`"
        echo ""
        echo "$RESP"
        echo ""
    } >> "$OUT_FILE"
done

echo "[bare-baseline] done.  Results: $OUT_FILE"
