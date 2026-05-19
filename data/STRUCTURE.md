# `data/` — Structure & Regeneration Guide

**Purpose.** This document explains every subdirectory of `data/`,
distinguishes **source artefacts** (commit to git) from **derived
build cache** (gitignored, regenerable on demand), and tells you how
to recover anything that's been cleaned up.

**Why this document exists.** v6.0.0-rc2 audit found 5.7 GB on disk
with no clear separation between source and cache; cleanup script
returned 5.1 GB. After this commit, `data/` should stay **under 1 GB**
in a fresh checkout; any growth beyond that is regenerated build
artefacts, not source bloat.

## Directory map

| Path | Role | Size | Tracked? |
|---|---|---:|---|
| `data/world_core/` | Curated knowledge graph (`*.jsonl`, ≈ 3 676 facts, 62 domains) | ≈ 6 MB | **yes** — source of truth |
| `data/retrieval/` | Built indices: `facts.json`, `morpheme_index.json`, `derived_facts.json`, `lexical_graph.json` | ≈ 72 MB | yes — products committed for CI determinism |
| `data/dialog/` | Dialog templates (`templates/v1.toml`) + slot inventory | ≈ 150 KB | **yes** |
| `data/curriculum/` | adam-curriculum content (`{pillar}/concepts.jsonl + test_items.jsonl`) | ≈ 50 KB | **yes** |
| `data/curated/` | Source corpus packs (`tatoeba`, `wikipedia_kz`, `cc100_kk`, `kazakh_textbooks`, `wikibooks_kk`, …) | ≈ 250 MB | **yes** |
| `data/curated/shards/` | **Build cache.** Local `cc100_kk_shard_*.json` for retrieval-index assembly | up to 1.4 GB | **no** (gitignored) |
| `data/curated/real_corpus_pairs.json` | **Build product.** Generated from `kazakhBooks.csv` for L5.5 training | ≈ 32 MB | **no** (gitignored) |
| `data/curated/filtered_*` | Pre-filtered corpus packs used by retrieval-index builder | ≈ 17 MB | **yes** |
| `data/external/` | Raw downloaded sources (textbook PDFs, HF datasets, wikibooks JSON) | ≈ 90 MB tracked + variable gitignored | mixed |
| `data/external/textbooks_kz/` | 11-grade textbook PDFs from MES KZ + Атамұра / Мектеп publishers | ≈ 87 MB | **no** (PDFs gitignored per source-licence) |
| `data/external/wikibooks_kk/` | kk.wikibooks.org page dump (CC-BY-SA-3.0) — 434 pages | ≈ 4 MB | **no** (regenerable via fetcher) |
| `data/external/huggingface_kz/` | HF datasets — `aman_instructions/` (52 k Q&A, MIT), occasional `kazakhBooks.csv` | varies | **no** (large; gitignored) |
| `data/lexicon_v1/` | Apertium-imported Kazakh roots | ≈ 5 MB | **yes** |
| `data/lexicon_v2/` | rc2 Lexicon triage CSVs (auto-approve / auto-exclude / needs-review) | ≈ 180 KB | **yes** |
| `data/tokenizer/` | Segmentation roots + lexicon-v1 export | ≈ 1.3 MB | **yes** |
| `data/eval/` | Cognitive eval / REPL replay datasets | ≈ 540 KB | **yes** |
| `data/training/` | Baseline LM training artefacts | ≈ 14 MB | **partially** (large `.safetensors` gitignored) |
| `data/scaling/` | Scaling-bench report | < 10 KB | **yes** |
| `data/foundation/` | CI foundation overview JSON | < 10 KB | **yes** |
| `data/raw/` | Pre-processing scratchpads | < 500 KB | **yes** |
| `data/checkpoints/` | L5.5 neural checkpoints (rc1+) | varies | **no** (gitignored; regenerable via `poc_kazakh_train`) |

## How to regenerate the gitignored build cache

After a `git clone` or after running the rc2 cleanup script, the
following artefacts are absent. Run the listed commands only if you
need them — most users don't.

### `data/curated/shards/` (~1.4 GB) — CC100 sharded packs

Required only for **full-scale** retrieval-index rebuilds. The
committed `data/retrieval/morpheme_index.json` was built from the
top-N committed shard slice; daily users never need to regenerate.

```bash
cargo run --release -p adam-corpus --bin process_cc100_kk -- --full
# → writes data/curated/shards/cc100_kk_shard_{02..NN}_pack.json
```

### `data/external/huggingface_kz/kazakhBooks.csv` (~3.7 GB) — book corpus

Required only when **rebuilding** `data/curated/real_corpus_pairs.json`
(input to the v6.0 L5.5 neural-composer training). The committed
`real_corpus_pairs.json` (~32 MB) is the build product; daily users
don't need the CSV.

```bash
bash scripts/fetch_huggingface_kazakh.sh
# → downloads ~3.7 GB to data/external/huggingface_kz/kazakhBooks.csv
# subsequent: cargo run --release -p adam-agg-synth --bin build_real_corpus_pairs
```

### `data/external/wikibooks_kk/` (~4 MB raw, 6.7 MB pack) — kk.wikibooks.org

Required for the v6.0.0-rc3 `wikibooks_kk_pack.json` rebuild. The
committed pack and `world_core/abai_works.jsonl` carry the
distilled facts; raw page dump is regenerable.

```bash
bash scripts/fetch_kk_wikibooks.sh
# → writes data/external/wikibooks_kk/page_*.json (~434 files)
# subsequent: cargo run --release -p adam-corpus --bin process_kk_wikibooks
```

### `data/external/textbooks_kz/*.pdf` — MES KZ textbooks

Manifest at `scripts/textbooks_kz_manifest.json` lists URLs for
official open-access editions. Most URLs are TODO stubs awaiting
publisher confirmation; the 7 already-downloaded PDFs cover grades
7, 8, and 11.

```bash
bash scripts/fetch_textbooks_kz.sh
# → respects manifest; only downloads entries with non-empty url
```

### `data/checkpoints/poc_kazakh/v6_*/` — L5.5 neural checkpoints

Regeneratable via:

```bash
# Smoke checkpoint (~10 min on M2 CPU):
POC_EPOCHS=1 POC_BATCH=8 POC_CHECKPOINT_DIR=data/checkpoints/smoke \
  cargo run --release -p adam-agg-model --bin poc_kazakh_train

# Full checkpoint (~95 min on M2 CPU):
POC_REAL_PACK=data/curated/real_corpus_pairs.json POC_ALPHA=0.5 \
  cargo run --release -p adam-agg-model --bin poc_kazakh_train
```

## What NOT to commit to git

- Anything in `data/curated/shards/`
- Anything matching `data/external/huggingface_kz/*.{csv,parquet,tsv,json}` (HF datasets are licence-checked but bulky)
- `data/external/textbooks_kz/*.pdf` (publisher licence)
- `data/external/wikibooks_kk/page_*.json` and `_manifest.json` (regenerable)
- `data/external/wikibooks_kk/` (subdirectory itself)
- `data/checkpoints/` (neural model artefacts)
- `data/curated/real_corpus_pairs.json` (build product)
- `data/training/*.safetensors*` (LM training artefacts)

All of these are already covered by `.gitignore` rules; the goal of
this section is to make the policy human-readable.

## When to update this document

Update this file whenever:

- A new corpus / dataset is added under `data/external/` or `data/curated/`.
- A new build product moves from "gitignored cache" to "committed".
- A subdirectory is renamed or restructured.

CI does NOT enforce this document — it's reference material. Drift
between this file and reality is a slow-burn cost; please keep it
current as you add corpora.
