# Tokenizer Dry Run

## Purpose

Before real tokenizer training, the repository should support a deterministic
dry run on a curated sample pack.

## Inputs

- experiment manifest:
  `data/eval/tokenizer_experiment_manifest.json`
- sample pack:
  `data/curated/tokenizer_dry_run_pack.json`

## Output

The dry run produces a small report with:

- experiment name
- sample count
- normalized non-empty count
- total character count
- average character count
- participating domains
