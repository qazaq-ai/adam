# `data/external/huggingface_kz/` — Hugging Face Kazakh datasets

Open-license Kazakh-language corpora downloaded from the Hugging Face
Hub. Each file is large (3–4 GB) and gitignored. The manifest at
`scripts/huggingface_kazakh_manifest.json` is the single source of
truth — it records license, expected size, and the resolvable URL for
every file in this directory.

## Why HF instead of scraping Kazakh-school portals

`opiq.kz`, `okulyk.kz`, and similar school-textbook viewers hide the
PDF behind a JavaScript viewer; there are no stable, scrapable URLs.
Even when a PDF is reachable, the publisher's terms of service do
not unambiguously permit redistribution. Hugging Face was built for
exactly this case — datasets are uploaded with an explicit license
(Apache-2.0, CC-BY-4.0, …) and stable `resolve/main/<file>` URLs.

## Currently tracked datasets

| Dataset | Split | License | Size | Status |
|---|---|---|---|---|
| `kz-transformers/multidomain-kazakh-dataset` | `kazakhBooks` | Apache-2.0 | ~3.97 GB | scheduled |
| `kz-transformers/multidomain-kazakh-dataset` | `kazakhNews` | Apache-2.0 | — | TODO (no url filled) |
| `kz-transformers/multidomain-kazakh-dataset` | `leipzig` | Apache-2.0 | — | TODO |
| `issai/KazCulture` | `train` | CC-BY-4.0 | — | TODO |
| `kz-transformers/kazakh-unified-national-testing-mc` | `train` | Apache-2.0 | — | TODO |
| `IS2AI/KazParC` | `all` | CC-BY-4.0 | — | TODO |

## Usage

```bash
# Edit the manifest if you need new datasets; the URL fields are
# pre-filled for high-priority entries.
$EDITOR scripts/huggingface_kazakh_manifest.json

# Fetch. Idempotent: re-runs skip already-downloaded entries.
./scripts/fetch_huggingface_kazakh.sh
```

The script writes `sha256` and `size_bytes` back into the manifest on
success.

## Downstream pipeline

```
data/external/huggingface_kz/kazakhBooks.csv      (3.97 GB)
  ↓ (csv→sentence dedup + loanword filter — TODO)
data/curated/huggingface_kazakh_books_pack.json
  ↓ (AggTokenizer morpheme tokenisation — TODO)
adam-agg-model real-data training path
```

The csv→pack and tokenisation steps are the missing pieces. See
`crates/adam-corpus/src/bin/` for the existing patterns
(`process_wikipedia_kz`, `process_rust_book_kk`, etc.) that should be
mirrored.
