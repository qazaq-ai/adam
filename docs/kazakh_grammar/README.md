# Kazakh Grammar — Reference Workbook

This directory is a linguistics workbook, not product documentation. It is the normative source behind `adam-kernel-fst`'s phonology and morphotactics implementation. The v1.0.0 FST + dialog system ships built on these notes.

## Reading order

1. [`00_architecture_v1.md`](00_architecture_v1.md) — architectural commitments (deterministic FST + dialog layer)
2. [`01_phonology.md`](01_phonology.md) — phoneme inventory, vowel harmony, consonant assimilation
3. [`02_morphology.md`](02_morphology.md) — suffix inventory, ordering constraints, allomorphy
4. [`03_syntax.md`](03_syntax.md) — word order, case government, clause structure
5. [`04_lexicon_sources.md`](04_lexicon_sources.md) — academic + open-source lexicon resources catalogue
6. [`05_work_plan.md`](05_work_plan.md) — Phase-10 work plan (delivered v0.4.5 → v1.0.0)
7. [`06_apertium_twol_catalogue.md`](06_apertium_twol_catalogue.md) — the 54 Apertium-kaz twol phonology rules, 22+ implemented
8. [`07_dialog_architecture.md`](07_dialog_architecture.md) — **shipped v1.0.0 dialog layer architecture**

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
