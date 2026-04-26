#!/usr/bin/env bash
# Download Kazakh Wikipedia XML dump and extract plain article text.
#
# Output: data/external/wikipedia_kz_plain.txt
#   One article per block, articles separated by "\x1e" (ASCII record separator).
#   Within an article, text is space-normalized MediaWiki source minus markup.
#
# License: CC-BY-SA 4.0 (Wikipedia articles)
# Attribution: "Wikipedia contributors, Kazakh Wikipedia
#               (https://kk.wikipedia.org), CC-BY-SA 4.0"
#
# Note: data/external/ is gitignored. The 155 MB compressed dump is not committed.
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
out_dir="$repo_root/data/external"
mkdir -p "$out_dir"
cd "$out_dir"

archive="kkwiki-latest-pages-articles.xml.bz2"
url="https://dumps.wikimedia.org/kkwiki/latest/$archive"
out_file="wikipedia_kz_plain.txt"

if [[ ! -f "$archive" ]]; then
  echo "downloading $url (~155 MB)"
  curl -L -o "$archive" "$url"
else
  echo "reusing existing $archive"
fi

echo "streaming extraction (bzcat + Rust extractor; articles separated by RS 0x1e)..."

bzcat "$archive" | cargo run --quiet -p adam-corpus --bin extract_wikipedia_plain > "$out_file"

size=$(wc -c < "$out_file")
articles=$(tr -cd $'\x1e' < "$out_file" | wc -c)
echo "wrote $out_file: $articles articles, $size bytes"
