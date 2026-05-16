# `data/external/textbooks_kz/` — Kazakh school textbooks (grades 1–11)

Per-grade Kazakh-language school textbooks following the MOIN RK
curriculum. Fetched and managed by `scripts/fetch_textbooks_kz.sh`
which reads `scripts/textbooks_kz_manifest.json`.

## Why

`data/external/` previously held an ad-hoc set of seven PDFs, five of
them grade 11. A research model trained on only an 11th-grade slice of
physics cannot recover earlier concepts (force, mass, velocity at the
7-8 grade level) and will hallucinate them. The manifest enumerates the
full curriculum so the gap is visible per row, and adding new URLs is a
one-line edit.

## Layout

```
data/external/textbooks_kz/
├── README.md
├── .gitignore                   # *.pdf — never commit textbook PDFs to git
├── grade_01/
│   ├── grade_01_ana_tili.pdf    (when fetched)
│   ├── grade_01_matematika.pdf
│   └── …
├── grade_02/
│   …
└── grade_11/
    └── …
```

The hierarchy is enforced by the manifest's `filename` field. The script
creates `grade_NN/` automatically.

## Usage

```bash
# 1. Edit scripts/textbooks_kz_manifest.json — fill in `url` for any
#    entry you have a legal, publicly-redistributable source for.
$EDITOR scripts/textbooks_kz_manifest.json

# 2. Fetch. Idempotent: re-runs skip already-downloaded entries.
./scripts/fetch_textbooks_kz.sh
```

The script writes the computed `sha256` and `size_bytes` back into the
manifest on success, so subsequent runs verify integrity rather than
re-downloading.

## Migration of existing data/external/*.pdf

Six of the seven existing PDFs in `data/external/` are listed in the
manifest with a `local_seed` pointer. Running the fetcher once will
copy them into the per-grade layout without any network access. After
that they live under `data/external/textbooks_kz/grade_NN/` and the old
locations can be removed (do it manually after confirming the seed
copies are in place).

## Legal

Only fill `url` with sources you are LEGALLY allowed to fetch and
process per their terms of service. Recommended sources:

- **opiq.kz** — official MOIN RK e-textbook platform
- **okulyk.kz** — official e-textbook archive
- **bilim-all.kz** — open educational portal
- Publisher pages: **Атамұра**, **Мектеп**, **Арман-ПВ**, **Білім**

PDFs are gitignored (see `.gitignore` next to this README). They never
leave the local checkout. The manifest itself (with SHA + size, no
URLs that violate ToS) is committed so contributors can verify what
their tree should contain.

## Downstream

Once textbooks are in place, the next pipeline stage is:

```
data/external/textbooks_kz/**/*.pdf
  ↓ (PDF → plain text extraction — TODO)
  ↓ (loanword filter — existing v1.x infra)
  ↓ (AggTokenizer morpheme tokenisation)
data/curated/textbooks_kz_pack.json
  ↓ adam-agg-model real-data training path
```

The PDF→text extraction step is the missing piece — `pdftotext` works
for born-digital books, but scanned books need OCR (recommend
`tesseract` with `kaz.traineddata`).
