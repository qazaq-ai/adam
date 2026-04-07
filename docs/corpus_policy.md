# Corpus Policy

## Core Rule

`adam` is a Kazakh-first text-model repository.

For the current foundation phase:

- curated corpora must be Kazakh-only
- curated corpora must be Cyrillic-only
- mixed-language corpora are not accepted as foundation corpora
- raw corpora may contain noise before curation, but they must be marked as raw

## Accepted Corpus Stages

- `raw`
  source-side material before cleanup
- `curated`
  cleaned Kazakh-only training-ready material
- `eval`
  held-out benchmark material

## Required Metadata

Every manifest must declare:

- name
- version
- language
- script
- stage
- source_policy
- domains

## Why This Is Strict

The repository should not drift into multilingual contamination at the
foundation stage. The first objective is a clean Kazakh text base, not broad
coverage theater.

