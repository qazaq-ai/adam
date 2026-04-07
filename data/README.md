# Data Layout

- `raw/` stores source-side inputs before cleanup
- `curated/` stores Kazakh-only cleaned datasets
- `eval/` stores held-out evaluation manifests and benchmark inputs

No large datasets should be committed until curation policy is defined.

The current machine-readable contracts are:

- `raw/source_registry.json`
- `curated/corpus_manifest.json`
- `eval/benchmark_manifest.json`
- `eval/kazakh_foundation_eval_dataset.json`
- `eval/tokenizer_experiment_manifest.json`
