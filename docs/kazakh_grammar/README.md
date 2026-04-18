# Kazakh Grammar — Study Notes for adam v1.0.0

This directory is a linguistics workbook, not product documentation. It exists so that the deterministic FST + minimal LM architecture planned for adam v1.0.0 is built on **verified** Kazakh grammar, not on informal heuristics picked up during v0.3–v0.5.

## Reading order

1. [`00_architecture_v1.md`](00_architecture_v1.md) — what we are building and why
2. [`01_phonology.md`](01_phonology.md) — phoneme inventory, vowel harmony, consonant assimilation
3. [`02_morphology.md`](02_morphology.md) — suffix inventory, ordering constraints, allomorphy
4. [`03_syntax.md`](03_syntax.md) — word order, case government, clause structure
5. [`04_lexicon_sources.md`](04_lexicon_sources.md) — catalogue of academic + open-source lexicon resources
6. [`05_fst_rust_design.md`](05_fst_rust_design.md) — Rust types for two-level morphology (written after studies)

## Source attribution policy

Every claim in these notes cites either:

- **A published grammar** (Ысқақов, Мәмaнов, Кеңесбаев, Тәжмұратов, etc.)
- **A machine-readable corpus or ruleset** (Apertium-kaz, HFST-kazakh, KazNERD)
- **Direct corpus evidence** (forms actually present in our cleaned corpus)

Unsourced generalisations are marked `UNVERIFIED` and treated as hypotheses until confirmed.

## Notation

- **Roots** are written in their citation form, without ▁ prefix: `мектеп`, `бала`.
- **Morpheme boundaries** use `+`: `мектеп+тер+ім+із+де` (school+PL+POSS.1PL+LOC).
- **Allomorph sets** use curly braces: `{ға, ге, қа, ке, на, не}` (dative).
- **FST transitions** use `→`: `N[back,voiceless] + DAT → N-қа`.
- **Abstract feature bundles** use square brackets: `[noun, plural, dat_case]`.

## Languages

Source reading in Kazakh and Russian where applicable. Working notes in **English only** so that:

1. Claude/codex tools can index them for later work.
2. The files remain searchable by future contributors regardless of L1.
3. Ambiguity across Kazakh dialect spellings doesn't pollute the normative record.

## What this workbook is NOT

- Not a Kazakh language course for humans. We reference standard pedagogical works; we don't replicate them.
- Not a complete grammar. We document only what the v1.0.0 FST must handle.
- Not a translator between Kazakh and other languages.
