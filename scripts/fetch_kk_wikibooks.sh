#!/usr/bin/env bash
# fetch_kk_wikibooks.sh — download every Kazakh Wikibooks article via
# the MediaWiki action API.
#
# kk.wikibooks.org has ≈ 434 mainspace pages at v6.0.0-rc3. Each one is
# Kazakh-language educational / curriculum content (Abai poems +
# prose, Java tutorial in Kazakh, Kazakhstan Constitution, language
# textbooks, …). All content is CC-BY-SA per Wikibooks licensing —
# safe to redistribute provided attribution is preserved (see the
# generated manifest below).
#
# Output:
#   data/external/wikibooks_kk/         — raw page JSON files
#   data/external/wikibooks_kk/_manifest.json   — index of fetched pages
#   data/external/wikibooks_kk/README.md        — licence + attribution
#
# Subsequent processing: `cargo run -p adam-corpus --bin
# process_kk_wikibooks` parses these → samples → corpus pack →
# extract_facts → world_core. That bin is part of the v6.0.0-rc3
# pipeline, not this fetcher.
#
# Idempotent — already-downloaded pages are skipped via file presence.

set -euo pipefail

DEST="data/external/wikibooks_kk"
mkdir -p "$DEST"

API="https://kk.wikibooks.org/w/api.php"
UA="adam-bot/6.0.0-rc3 (https://github.com/qazaq-ai/adam; baimurza.daulet@gmail.com)"

echo "fetch_kk_wikibooks: listing mainspace pages..."
TITLES=$(curl -sL -A "$UA" \
    "$API?action=query&list=allpages&aplimit=500&apnamespace=0&format=json" \
    | jq -r '.query.allpages[].title')

count_listed=$(echo "$TITLES" | wc -l | tr -d ' ')
echo "  → $count_listed pages in main namespace"

count_fetched=0
count_skipped=0
while IFS= read -r title; do
    # Filename: hash the title to avoid filename-encoding issues
    # (some Kazakh-letter combinations break filesystem encoders).
    hash=$(printf '%s' "$title" | shasum -a 256 | awk '{print $1}' | cut -c1-12)
    out="$DEST/page_${hash}.json"
    if [[ -f "$out" ]]; then
        count_skipped=$((count_skipped + 1))
        continue
    fi
    # Fetch via prop=extracts in plaintext mode (no wiki markup).
    # curl's --data-urlencode does the percent-encoding for us — no
    # need to shell out for URL escaping.
    body=$(curl -sL -A "$UA" --max-time 10 -G "$API" \
        --data-urlencode "action=query" \
        --data-urlencode "prop=extracts" \
        --data-urlencode "explaintext=1" \
        --data-urlencode "titles=$title" \
        --data-urlencode "format=json" \
        || true)
    if [[ -z "$body" || "${#body}" -lt 50 ]]; then
        echo "  WARN empty response for «$title»"
        continue
    fi
    # Build the per-page output via jq (single page in response,
    # extracted as the only `.query.pages.*` object).
    fetched_at=$(date -u +%FT%TZ)
    if ! echo "$body" | jq --arg src "kk.wikibooks.org" \
                            --arg lic "CC-BY-SA-3.0" \
                            --arg ts  "$fetched_at" \
        '.query.pages | to_entries | map({page_id: .key, title: .value.title, extract: (.value.extract // ""), source: $src, licence: $lic, fetched_at: $ts}) | .[0]' \
        > "$out" 2>/dev/null; then
        echo "  WARN parse fail for «$title»"
        rm -f "$out"
        continue
    fi
    count_fetched=$((count_fetched + 1))
    # Friendly progress every 25 pages
    if (( count_fetched % 25 == 0 )); then
        echo "  fetched $count_fetched / $count_listed ..."
    fi
    # Wikipedia's API ToS: stay polite — 1 req/sec is the ceiling.
    sleep 0.5
done <<< "$TITLES"

echo "fetch_kk_wikibooks: done — $count_fetched new, $count_skipped skipped"

# Build manifest via jq — collects per-page metadata into a single
# `_manifest.json` index without needing a foreign runtime.
fetched_at=$(date -u +%FT%TZ)
jq -s \
    --arg src "kk.wikibooks.org" \
    --arg lic "CC-BY-SA-3.0" \
    --arg attr "Kazakh Wikibooks contributors — list of authors at https://kk.wikibooks.org/" \
    --arg ts "$fetched_at" \
    '{
        schema_version: 1,
        source: $src,
        licence: $lic,
        attribution: $attr,
        fetched_at: $ts,
        page_count: length,
        pages: map({
            title: .title,
            extract_chars: (.extract // "" | length),
        }),
    }' \
    "$DEST"/page_*.json > "$DEST/_manifest.json"
echo "manifest: $(jq '.page_count' "$DEST/_manifest.json") pages indexed"

cat > "$DEST/README.md" <<EOF
# kk.wikibooks.org corpus

**Source:** Kazakh Wikibooks (https://kk.wikibooks.org/).
**Licence:** Creative Commons Attribution-ShareAlike 3.0 (CC-BY-SA-3.0).
**Attribution:** Kazakh Wikibooks contributors.
**Fetched:** $(date -u +%FT%TZ)
**Page count:** see \`_manifest.json\`.

Each \`page_*.json\` carries:
- \`title\` — page title
- \`extract\` — plain-text content (no wiki markup)
- \`source\`, \`licence\`, \`fetched_at\` — provenance

## Downstream processing

\`cargo run --release -p adam-corpus --bin process_kk_wikibooks\`
parses these into \`data/curated/wikibooks_kk_pack.json\` for the
v6.0.0-rc3 corpus expansion.

## Coverage

This corpus complements (not duplicates) \`wikipedia_kz_pack.json\`.
Wikibooks is curriculum-focused — textbook chapters, Abai literature,
Constitution texts, programming tutorials — vs Wikipedia's
encyclopaedic style.

## Re-fetching

This fetcher is idempotent. Re-running only downloads pages whose
hash file is absent. To force a complete refetch:

  rm -rf $DEST/page_*.json
  bash scripts/fetch_kk_wikibooks.sh
EOF

echo "fetch_kk_wikibooks: wrote $DEST/{README.md,_manifest.json}"
echo "fetch_kk_wikibooks: total pages on disk: $(ls $DEST/page_*.json | wc -l | tr -d ' ')"
