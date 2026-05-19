# kk.wikibooks.org corpus

**Source:** Kazakh Wikibooks (https://kk.wikibooks.org/).
**Licence:** Creative Commons Attribution-ShareAlike 3.0 (CC-BY-SA-3.0).
**Attribution:** Kazakh Wikibooks contributors.
**Fetched:** 2026-05-19T08:21:22Z
**Page count:** see `_manifest.json`.

Each `page_*.json` carries:
- `title` — page title
- `extract` — plain-text content (no wiki markup)
- `source`, `licence`, `fetched_at` — provenance

## Downstream processing

`cargo run --release -p adam-corpus --bin process_kk_wikibooks`
parses these into `data/curated/wikibooks_kk_pack.json` for the
v6.0.0-rc3 corpus expansion.

## Coverage

This corpus complements (not duplicates) `wikipedia_kz_pack.json`.
Wikibooks is curriculum-focused — textbook chapters, Abai literature,
Constitution texts, programming tutorials — vs Wikipedia's
encyclopaedic style.

## Re-fetching

This fetcher is idempotent. Re-running only downloads pages whose
hash file is absent. To force a complete refetch:

  rm -rf data/external/wikibooks_kk/page_*.json
  bash scripts/fetch_kk_wikibooks.sh
