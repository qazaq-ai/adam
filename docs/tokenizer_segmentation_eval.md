# Tokenizer Segmentation Eval

## Purpose

Tokenizer quality must be checked against known Kazakh morphology collisions,
not only against aggregate character counts.

## Dataset Contract

The segmentation reference dataset lives in:

- `data/eval/tokenizer_segmentation_eval_dataset.json`

The deterministic segmentation config lives in:

- `data/tokenizer/segmentation_roots.json`
- `data/tokenizer/segmentation_rules.json`

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

The full tokenizer experiment runner also scores segmentation exact matches and
reports failure cases and category-level morphology breakdowns:

- `scripts/run_tokenizer_experiment.sh`
- `scripts/run_tokenizer_experiment_delta.sh`

## Reporting

Segmentation reports must not stop at a single aggregate score.

- every run should expose exact-match rates by morphology category
- imperative, negation, tense, and voice chains must be inspectable separately
- critical deterministic zones must expose guard buckets for `imperative`, `negation`, `voice`, and their intersections
- the production experiment snapshot should be stored as `data/eval/tokenizer_experiment_report.json`
- drift against that snapshot should be inspectable through `data/eval/tokenizer_experiment_delta_report.json`
