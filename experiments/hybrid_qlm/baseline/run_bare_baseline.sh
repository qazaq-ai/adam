#!/usr/bin/env bash
# Hybrid QLM — Day 1-3 bare-baseline harness.
#
# Measures the chosen base model on a small slice of the
# existing eval suite WITHOUT the adam-kernel verifier.
# This is the honest «what does the LM alone do on our Kazakh
# probes» measurement that anchors every later experiment.
#
# Setup prerequisites (run once, manually):
#   1. brew install llama.cpp   (provides llama-completion + Metal backend)
#   2. Download Gemma-3 1B IT q4_k_m gguf to:
#        data/lm_models/gemma-3-1b-it-q4_k_m.gguf
#   (Or Gemma-3 4B — set LM_MODEL env var.  See day_1.md /
#    day_3_results.md for the choice rationale.)
#
# Apple Silicon notes:
#   * `-ngl 99` offloads ALL layers to Metal GPU.  Without it
#     llama-cli is CPU-only and ~10× slower on M-class hardware.
#   * `llama-completion` is the SINGLE-SHOT binary; `llama-cli`
#     went interactive-only in late 2026 (~build 9820).
#
# Output:
#   experiments/hybrid_qlm/baseline/day_3_results.md

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$REPO_ROOT"

LM_MODEL="${LM_MODEL:-data/lm_models/gemma-3-1b-it-q4_k_m.gguf}"
LLAMA_BIN="${LLAMA_BIN:-llama-completion}"
OUT_FILE="${OUT_FILE:-experiments/hybrid_qlm/baseline/day_3_results.md}"
N_PREDICT="${N_PREDICT:-96}"
TEMP="${TEMP:-0.2}"
CTX="${CTX:-512}"
NGL="${NGL:-99}"

if ! command -v "$LLAMA_BIN" >/dev/null 2>&1; then
    echo "ERROR: $LLAMA_BIN not on PATH.  Run: brew install llama.cpp" >&2
    exit 1
fi
if [[ ! -f "$LM_MODEL" ]]; then
    echo "ERROR: model not found at $LM_MODEL" >&2
    echo "       See experiments/hybrid_qlm/baseline/day_1.md for setup." >&2
    exit 1
fi

echo "[bare-baseline] model       : $LM_MODEL"
echo "[bare-baseline] binary      : $LLAMA_BIN"
echo "[bare-baseline] GPU layers  : $NGL (Metal)"
echo "[bare-baseline] ctx / n_pred: $CTX / $N_PREDICT"

# 10-query probe across school_program / chemistry / biology /
# geography / SOP / known-defective speech-defect inputs.
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
    echo "**Binary:** \`$LLAMA_BIN\` (Metal, -ngl $NGL)"
    echo "**System prompt:** \`$SYSTEM_PROMPT\`"
    echo "**Sampling:** temperature=$TEMP, n_predict=$N_PREDICT, ctx=$CTX"
    echo ""
    echo "## Probes"
    echo ""
} > "$OUT_FILE"

for q in "${QUERIES[@]}"; do
    echo "[bare-baseline] probe: $q"
    PROMPT="$SYSTEM_PROMPT

Сұрақ: $q
Жауап:"
    # llama-completion writes generation to stdout, perf to stderr.
    # The Gemma chat template inserts «user» / «model» role lines
    # before the actual answer; strip them + the trailing
    # «> EOF by user» marker so the captured text is just the
    # model's response.
    RAW=$("$LLAMA_BIN" \
        -m "$LM_MODEL" \
        -ngl "$NGL" \
        --no-warmup \
        -c "$CTX" \
        -n "$N_PREDICT" \
        --temp "$TEMP" \
        -p "$PROMPT" 2>/dev/null)
    CLEAN=$(printf '%s\n' "$RAW" \
        | awk '/^model$/{flag=1; next} /^> EOF/{flag=0} flag' \
        | sed '/^$/d')

    {
        echo "### \`$q\`"
        echo ""
        echo "$CLEAN"
        echo ""
    } >> "$OUT_FILE"
done

echo "[bare-baseline] done.  Results: $OUT_FILE"
