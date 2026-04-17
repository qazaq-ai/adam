#!/usr/bin/env bash
# Download Abai Qunanbayuly's public-domain works from kk.wikisource.org.
#
# Abai Qunanbayuly (1845–1904) is the canonical classical Kazakh author.
# His works entered public domain in 1974 (70 years post mortem) and are
# hosted on Kazakh Wikisource under CC-BY-SA 4.0 / public domain.
#
# Output: data/external/abai_wikisource_plain.txt
#   One work per record, records separated by ASCII RS (0x1e), same format
#   as data/external/wikipedia_kz_plain.txt so we can reuse the processor
#   pattern.
#
# Attribution: Wikisource contributors, https://kk.wikisource.org, CC-BY-SA 4.0
# Source author: Abai Qunanbayuly (public domain).
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
out_dir="$repo_root/data/external"
mkdir -p "$out_dir"
cd "$out_dir"

index_html="abai_wikisource_index.html"
out_file="abai_wikisource_plain.txt"

# User-Agent per Wikimedia policy:
# https://meta.wikimedia.org/wiki/User-Agent_policy
UA="adam-corpus/0.4.0 (foundation LM research; contact: baimurza.daulet@gmail.com)"

echo "downloading Abai index page..."
curl -sLA "$UA" "https://kk.wikisource.org/wiki/Абай_Құнанбайұлы" -o "$index_html"

# Extract unique work-page paths from the author index.
# Exclude meta namespaces (Special:, Category:, Template:, Author:, File:, Help:).
work_paths=$(grep -oE 'href="/wiki/[^":]*"' "$index_html" \
  | sort -u \
  | grep -vE "/wiki/(Main_Page|Special|Category|Template|Author|File|Help)" \
  | sed -E 's/^href="//; s/"$//')

work_count=$(printf "%s\n" "$work_paths" | wc -l | tr -d ' ')
echo "found $work_count work pages"

: > "$out_file"
i=0
for path in $work_paths; do
  i=$((i + 1))
  url="https://kk.wikisource.org${path}"
  tmp_html=$(mktemp)
  if curl -sLA "$UA" "$url" -o "$tmp_html"; then
    # Extract <p> content only from the mw-parser-output body.
    # Strip all HTML tags. Normalize whitespace. Append RS separator.
    perl -0777 -ne '
      while (/<p[^>]*>(.*?)<\/p>/gs) {
        $p = $1;
        $p =~ s/<[^>]+>//g;
        $p =~ s/&nbsp;/ /g;
        $p =~ s/&amp;/&/g;
        $p =~ s/&lt;/</g;
        $p =~ s/&gt;/>/g;
        $p =~ s/&quot;/"/g;
        $p =~ s/\s+/ /g;
        $p =~ s/^\s+|\s+$//g;
        print "$p\n" if length($p) > 0;
      }
    ' "$tmp_html" >> "$out_file"
    printf "\x1e" >> "$out_file"
  fi
  rm -f "$tmp_html"
  if [[ $((i % 20)) -eq 0 ]]; then
    echo "  fetched $i/$work_count"
  fi
  # Rate-limit to be polite to Wikimedia.
  sleep 0.15
done

bytes=$(wc -c < "$out_file" | tr -d ' ')
echo "done: $out_file ($bytes bytes, $work_count works)"
