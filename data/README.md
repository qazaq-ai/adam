# Data Layout

- `raw/` stores source-side inputs before cleanup
- `curated/` stores Kazakh-only cleaned datasets
- `eval/` stores held-out evaluation manifests and benchmark inputs

No large datasets should be committed until curation policy is defined.

The current machine-readable contracts are:

- `raw/source_registry.json`
- `raw/source_scoring_rules.json`
- `curated/corpus_manifest.json`
- `curated/tokenizer_dry_run_pack.json`
- `eval/benchmark_manifest.json`
- `eval/kazakh_foundation_eval_dataset.json`
- `eval/tokenizer_experiment_manifest.json`
