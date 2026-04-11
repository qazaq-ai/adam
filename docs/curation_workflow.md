# Curation Workflow

## Rule

Raw data is not training data.

## Stages

1. register a source in `data/raw/source_registry.json`
2. mark whether it is allowed for training yet
3. move only reviewed material into curated manifests
4. keep held-out evaluation material separate from training material
5. maintain at least a minimal multi-source training baseline so deterministic
   assembly can validate distribution rather than a single-source fallback

## Required Checks

- language remains Kazakh
- script remains Cyrillic
- provenance is recorded
- stage is explicit
- training permission is explicit
- source classification is explicit

## Regression Artifacts

The curated source acceptance layer now keeps deterministic regression artifacts:

- `data/curated/source_acceptance_report.json`
- `data/curated/source_acceptance_summary_report.json`
- `data/curated/source_acceptance_delta_report.json`

These artifacts make acceptance decisions observable by domain, quality tier,
and acceptance guard instead of relying only on full-report equality.
