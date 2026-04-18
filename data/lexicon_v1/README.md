# adam v1.0.0 Lexicon

This directory holds the v1.0.0 lexicon materials, separate from the
v0.3-v0.5 `data/tokenizer/segmentation_roots.json` (which is frozen at
211 curated entries for compatibility with existing segmentation tests).

The v1.0.0 FST (to be built) consumes the union of:
- `data/tokenizer/segmentation_roots.json` (211 curated, authoritative)
- `data/lexicon_v1/apertium_imported_roots.json` (Apertium-kaz, this dir)
- future `data/lexicon_v1/corpus_derived_roots.json` (week 3)
- future `data/lexicon_v1/proper_nouns.json` (week 4)

## Files

- `apertium_imported_roots.json` — 11,919 entries imported from
  Apertium-kaz `tests/vocabulary/input.csv` (2025 snapshot). Filtered to
  avoid surface-prefix conflicts with the 211 curated roots. See
  `apertium_import_report.json` for full statistics.
- `apertium_import_report.json` — audit trail for the import.

## Licensing note

Apertium-kaz is GPL-3.0. The lemma-POS tuples we extract are factual
linguistic data (per standard dictionary-copyright jurisprudence) and
not themselves copyrightable, but we document provenance here. Our
schema transformation, filtering, and integration code is our own.

## POS distribution (current import)

| POS | count |
|---|---|
| noun | 5,740 |
| adjective | 3,378 |
| verb | 2,412 |
| adverb | 335 |
| pronoun | 21 |
| conjunction | 18 |
| postposition | 14 |
| numeral | 1 |
| **total** | **11,919** |

## What is NOT in this file

- The 211 curated v0.4.0 roots (kept in `data/tokenizer/segmentation_roots.json`)
- 9,693 Apertium entries rejected as loanwords (no Kazakh-specific
  letters AND length ≥ 5 — mostly Russian technical vocabulary)
- 3,817 Apertium entries rejected for surface-prefix conflict with the
  211 curated roots (would break v0.4.0 segmentation tests if merged)
- 1,902 Apertium entries already present in the 211 curated roots
- 316 Apertium entries with POS classes we don't model yet
  (interjections, ideophones, copulas, abbreviations)
