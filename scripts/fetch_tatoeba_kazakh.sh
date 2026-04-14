#!/usr/bin/env bash
# Download Tatoeba sentence corpus and extract Kazakh sentences.
#
# Output: data/external/tatoeba_kazakh_sentences.tsv
#   Columns: sentence_id<TAB>lang_code<TAB>text  (filtered to lang_code == "kaz")
#
# License: sentences are CC-BY 2.0 FR (https://creativecommons.org/licenses/by/2.0/fr/)
# Attribution: Tatoeba.org contributors, https://tatoeba.org
#
# Note: `data/external/` is gitignored. The full archive is ~200 MB compressed
# / ~1 GB uncompressed — we do not commit it.
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
out_dir="$repo_root/data/external"
mkdir -p "$out_dir"
cd "$out_dir"

archive="sentences.tar.bz2"
extracted="sentences.csv"
kazakh_out="tatoeba_kazakh_sentences.tsv"
url="https://downloads.tatoeba.org/exports/sentences.tar.bz2"

if [[ ! -f "$archive" ]]; then
  echo "downloading Tatoeba sentence archive (~200 MB) from $url"
  curl -L -o "$archive" "$url"
else
  echo "reusing existing $archive"
fi

if [[ ! -f "$extracted" ]]; then
  echo "extracting archive..."
  tar -xjf "$archive"
fi

echo "filtering Kazakh sentences..."
awk -F'\t' '$2 == "kaz"' "$extracted" > "$kazakh_out"

count=$(wc -l < "$kazakh_out")
echo "wrote $kazakh_out ($count Kazakh sentences)"
