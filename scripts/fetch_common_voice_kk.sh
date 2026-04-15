#!/usr/bin/env bash
# Download Kazakh sentence prompts from the Mozilla Common Voice repository.
#
# Output: data/external/common_voice_kk_sentences.txt
#   Format: one sentence per line (UTF-8)
#
# License: CC0-1.0 (public domain dedication)
#   https://github.com/common-voice/common-voice/blob/main/LICENSE
# Attribution: Mozilla Common Voice contributors, Kazakh sentence-collector.
#
# Note: `data/external/` is gitignored; the upstream file (~450 KB) is small
# enough to refetch on demand.
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
out_dir="$repo_root/data/external"
mkdir -p "$out_dir"

url="https://raw.githubusercontent.com/common-voice/common-voice/main/server/data/kk/sentence-collector.txt"
out="$out_dir/common_voice_kk_sentences.txt"

echo "downloading $url"
curl -sL "$url" -o "$out"

lines=$(wc -l < "$out")
echo "wrote $out ($lines lines)"
