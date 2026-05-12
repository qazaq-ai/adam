#!/usr/bin/env bash
# **v5.19.0 — Voice arc V3 companion.** Download a Kazakh-supporting
# Whisper GGML model. Defaults to `large-v3-q5_0` (quantized; 1.0 GB,
# ~3× more accurate on Kazakh phonemes than `medium`). Pass `large-v3`
# as the first arg for the full-precision variant (3.1 GB).
#
# Usage:
#   bash scripts/download_whisper_model.sh                # → large-v3-q5_0
#   bash scripts/download_whisper_model.sh large-v3       # full precision
#   bash scripts/download_whisper_model.sh medium         # fallback
#
# Destination:
#   $HOME/whisper-models/ggml-<variant>.bin
#
# The script is idempotent — it skips download if the file already
# exists with non-zero size (no checksum verification; whisper.cpp
# itself ships with one via `whisper-cli`'s built-in model loader).

set -euo pipefail

variant="${1:-large-v3-q5_0}"
target_dir="${HOME}/whisper-models"
target_path="${target_dir}/ggml-${variant}.bin"
url="https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-${variant}.bin"

mkdir -p "${target_dir}"

if [[ -s "${target_path}" ]]; then
    echo "model already present: ${target_path}"
    exit 0
fi

echo "downloading ggml-${variant} from ${url}"
echo "destination: ${target_path}"

# Disk-space sanity check — refuse to download if < 2 GB free.
free_gb=$(df -k "${target_dir}" | awk 'NR==2 {printf "%.1f", $4/1024/1024}')
echo "free space: ${free_gb} GiB"

if (( $(echo "${free_gb} < 2.0" | bc -l) )); then
    echo "ERROR: less than 2 GiB free in ${target_dir}." >&2
    echo "  Free up space and retry, or pick a smaller variant:" >&2
    echo "    - large-v3-q5_0  (1.0 GiB, quantized, recommended)" >&2
    echo "    - medium-q5_0    (0.5 GiB, smaller fallback)" >&2
    echo "    - base-q5_0      (0.06 GiB, fastest, lower accuracy)" >&2
    exit 1
fi

curl -L --fail --progress-bar -o "${target_path}.tmp" "${url}"
mv "${target_path}.tmp" "${target_path}"

actual_size=$(du -h "${target_path}" | awk '{print $1}')
echo "downloaded ${actual_size} → ${target_path}"

cat <<EOF

Next: run adam-chat with this model:

  ADAM_WHISPER_BIN=/opt/homebrew/bin/whisper-cli \\
    ./target/release/adam_chat --voice-input \\
      --whisper-model ${target_path}

(Adjust ADAM_WHISPER_BIN path if your whisper-cli is elsewhere.)
EOF
