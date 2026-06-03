#!/usr/bin/env bash
# Phase 13 step 1 (2026-05-31): generate the committed reference
# listening set in data/listening_tests/phase13_neural_piper/ from
# the per-word config in README.md. Idempotent — re-running
# overwrites the WAVs bit-for-bit identical given the same model +
# espeak-ng + ffmpeg versions, modulo neural noise injection from
# the Piper defaults (noise_scale=0.667, noise_w=0.8 — see model
# config).
#
# Build-time tools required:
#   - espeak-ng (brew install espeak-ng)
#   - python3 + venv at data/tts_models/.venv/ (see README.md)
#   - ffmpeg
#
# Run from repo root:
#   bash tools/synthesize_piper/synthesize.sh
set -euo pipefail
cd "$(dirname "$0")/../.."

MODEL="data/tts_models/kk_KZ-issai-high.onnx"
OUT="data/listening_tests/phase13_neural_piper"

if [[ ! -f "$MODEL" ]]; then
    echo "[synthesize_piper] missing model: $MODEL" >&2
    echo "[synthesize_piper] fetch with:" >&2
    echo "  curl -L -o $MODEL https://huggingface.co/rhasspy/piper-voices/resolve/main/kk/kk_KZ/issai/high/kk_KZ-issai-high.onnx" >&2
    exit 2
fi
if [[ ! -d "data/tts_models/.venv" ]]; then
    echo "[synthesize_piper] missing venv. Setup:" >&2
    echo "  python3 -m venv data/tts_models/.venv && \\" >&2
    echo "  source data/tts_models/.venv/bin/activate && \\" >&2
    echo "  pip install piper-tts" >&2
    exit 2
fi

# shellcheck disable=SC1091
source data/tts_models/.venv/bin/activate

mkdir -p "$OUT"

# Per-word config from README.md.
# Tuple format: lowercase_id|capitalized_input|length_scale|trim(yes/no)
WORDS=(
    "сәлем|Сәлем.|1.1|yes"
    "қазақстан|Қазақстан.|1.1|yes"
    "балалар|Балалар.|1.1|yes"
    "мен|Мен.|1.2|yes"
    "сен|Сен.|1.1|yes"
    "адам|Адам.|1.2|yes"
    "бала|Бала.|1.4|yes"
    "бұл|Бұл.|1.3|yes"
    "олар|Олар.|1.0|no"  # no-trim — the canonical-config form is the cleanest
)

for entry in "${WORDS[@]}"; do
    IFS="|" read -r word cap scale trim <<<"$entry"
    raw="/tmp/_piper_${word}_$$.wav"
    final="${OUT}/tts_neural_${word}.wav"

    echo "$cap" | piper \
        --model "$MODEL" \
        --length-scale "$scale" \
        --sentence-silence 0.0 \
        --output-file "$raw" >/dev/null 2>&1

    if [[ "$trim" == "yes" ]]; then
        ffmpeg -hide_banner -loglevel error -y \
            -i "$raw" \
            -af "areverse,silenceremove=start_periods=1:start_silence=0.08:start_threshold=-55dB:start_duration=0.2,areverse" \
            "$final"
    else
        cp "$raw" "$final"
    fi
    rm -f "$raw"

    bytes=$(wc -c <"$final")
    # 22050 Hz × 2 bytes per sample.
    dur_ms=$(python3 -c "print(int(($bytes-44)/44.1))")
    printf "  %-12s scale=%s trim=%s → %s ms (%s b)\n" "$word" "$scale" "$trim" "$dur_ms" "$bytes"
done

echo
echo "[synthesize_piper] wrote ${#WORDS[@]} WAVs to $OUT/"
