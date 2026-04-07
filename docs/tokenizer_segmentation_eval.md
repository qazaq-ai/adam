# Tokenizer Segmentation Eval

## Purpose

Tokenizer quality must be checked against known Kazakh morphology collisions,
not only against aggregate character counts.

## Dataset Contract

The segmentation reference dataset lives in:

- `data/eval/tokenizer_segmentation_eval_dataset.json`

It stores token-level expected segmentations for cases that are easy to get
wrong if the system guesses from the suffix tail instead of respecting
Kazakh morphology.

## Starter Examples

- `мекемеден -> мекеме + ден`
- `келді -> кел + ді`
- `қаралды -> қара + л + ды`

## Runner

The segmentation contract can be checked with:

- `scripts/run_tokenizer_segmentation_eval.sh`
