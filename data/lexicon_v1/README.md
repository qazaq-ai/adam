# adam Lexicon

This directory holds the adam Kazakh Lexicon materials consumed by the FST
parser / synthesiser (`adam-kernel-fst::lexicon::LexiconV1`).

The FST consumes the union of two files at runtime:

- `data/tokenizer/segmentation_roots.json` — curated roots (**4,432 as of v2.2** after the 87-entry intervocalic-voicing pollution purge; v1.4.5 added 20 modern professions; v2.1+ is driving new additions from extracted-fact gap signals)
- `data/lexicon_v1/apertium_imported_roots.json` — 11,919 Apertium-kaz imports (read-only; any cleanup happens in the curated file)

Total ~16.4 k roots. The `LexiconV1` loader merges them and resolves
collisions curated-wins.

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
