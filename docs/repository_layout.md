# Repository Layout

## Crates

- `adam-core`
  shared model identity and foundation contracts
- `adam-tokenizer`
  tokenizer profiles and segmentation interfaces
- `adam-corpus`
  corpus manifests and curation contracts
- `adam-eval`
  evaluation suite definitions
- `adam-train`
  baseline training manifests and planning runners

## Data

- `data/raw`
  source-side inputs before curation
- `data/curated`
  cleaned Kazakh-only corpora
- `data/eval`
  held-out benchmark assets
- `data/training`
  reproducible baseline training plans
