#!/usr/bin/env bash
# Download CC-100 Kazakh monolingual corpus (Common Crawl, language-filtered).
#
# Output: data/external/cc100_kk.txt.xz  (~931 MB compressed)
# License: CC-100 is released by Facebook/Meta AI under the original Common Crawl
#          terms of use. Attribution:
#          "Conneau et al. 2020, Unsupervised Cross-lingual Representation Learning
#           at Scale (XLM-R), https://data.statmt.org/cc-100/"
#
# Note: data/external/ is gitignored. The 931 MB compressed file is not committed,
#       and we never decompress the full 5-8 GB body — processing streams through xz.
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
out_dir="$repo_root/data/external"
mkdir -p "$out_dir"
cd "$out_dir"

archive="cc100_kk.txt.xz"
url="https://data.statmt.org/cc-100/kk.txt.xz"

if [[ ! -f "$archive" ]]; then
  echo "downloading $url (~931 MB)"
  curl -L -o "$archive" "$url"
else
  echo "reusing existing $archive ($(ls -la "$archive" | awk '{print $5}') bytes)"
fi

echo "done: $out_dir/$archive"
