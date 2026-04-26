#!/usr/bin/env bash
# Fetch classical Kazakh authors' public-domain works from kk.wikisource.org.
#
# All authors below died more than 70 years ago and their works are in the
# public domain globally. Kazakh Wikisource hosts them under CC-BY-SA 4.0.
#
# Abai is in a separate script (fetch_abai_wikisource.sh) — the 146-work
# author-page corpus is large enough that it makes sense to keep it as a
# stand-alone pack. This script captures the REMAINING classical core:
#
#   Ыбырай Алтынсарин  (1841–1889) — 8 works on Wikisource
#   Мағжан Жұмабаев   (1893–1938) — 7 works on Wikisource
#
# Shakarim, Zhambyl, Saken Seyfullin, Mirzhakyp Dulatov are also public
# domain but their pages don't exist on kk.wikisource yet. When they do
# (or when we add them manually), extend the AUTHORS list below.
#
# Output: data/external/kazakh_classics_plain.txt
#   One work per record, records separated by ASCII RS (0x1e), same
#   format as abai_wikisource_plain.txt.

set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
out_dir="$repo_root/data/external"
mkdir -p "$out_dir"
cd "$out_dir"

out_file="kazakh_classics_plain.txt"
UA="adam-corpus/1.2.0 (foundation LM research; contact: baimurza.daulet@gmail.com)"

# Authors whose index pages currently exist on kk.wikisource.
AUTHORS=(
    "Ыбырай_Алтынсарин"
    "Мағжан_Жұмабаев"
)

: > "$out_file"
total_works=0

for author in "${AUTHORS[@]}"; do
    echo "=== fetching $author ==="
    index_html=$(mktemp)
    curl -sLA "$UA" "https://kk.wikisource.org/wiki/$author" -o "$index_html"

    # Extract unique work-page paths linked from the author page.
    # Exclude meta namespaces and the author's own page.
    work_paths=$(grep -oE 'href="/wiki/[^":]*"' "$index_html" \
      | sort -u \
      | grep -vE "/wiki/(Main_Page|Special|Category|Template|Author|File|Help|Санат|Арнайы|$author)" \
      | sed -E 's/^href="//; s/"$//' || true)
    rm -f "$index_html"

    if [[ -z "$work_paths" ]]; then
        echo "  no work pages found for $author"
        continue
    fi

    work_count=$(printf "%s\n" "$work_paths" | wc -l | tr -d ' ')
    echo "  found $work_count work pages"
    total_works=$((total_works + work_count))

    for path in $work_paths; do
        url="https://kk.wikisource.org${path}"
        tmp_html=$(mktemp)
        if curl -sLA "$UA" "$url" -o "$tmp_html"; then
            cargo run --quiet -p adam-corpus --bin extract_html_paragraphs -- "$tmp_html" >> "$out_file"
            printf "\x1e" >> "$out_file"
        fi
        rm -f "$tmp_html"
        sleep 0.15
    done
done

bytes=$(wc -c < "$out_file" | tr -d ' ')
echo "done: $out_file ($bytes bytes, $total_works works across ${#AUTHORS[@]} authors)"
