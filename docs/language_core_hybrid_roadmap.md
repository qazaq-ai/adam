# Language Core + Hybrid Roadmap

Status: **active**. This document tracks the incremental migration toward a stronger Kazakh language core and a deterministic-first hybrid architecture for `adam`.

This roadmap is complementary to:
- [docs/roadmap.md](/Users/dake/project/adam/docs/roadmap.md) for release history
- [docs/kazakh_grammar/05_work_plan.md](/Users/dake/project/adam/docs/kazakh_grammar/05_work_plan.md) for the older grammar/FST work plan

## Purpose

This file exists to keep one focused execution log for the current direction:
- strengthen Kazakh language knowledge before stylistic generation
- reuse existing architecture instead of replacing it
- avoid duplicating rules that already live in `adam-kernel-fst`, `world_core`, or dialog planner/policy layers
- add probabilistic generation only at a tightly constrained surface layer, if needed

## Non-Duplication Rules

- Do not duplicate morphology already implemented in `adam-kernel-fst`.
- Do not create a second geography memory when `data/world_core/geography_kz.jsonl` already provides canonical entities.
- Do not spread normalization rules back across `semantics.rs`; keep them in `language_core`.
- Do not let any future generative layer invent facts, entities, or memory writes.
- Prefer canonical entity ids and aliases over storing many surface forms as separate knowledge entries.

## Architectural Direction

Target stack:

1. `adam-kernel-fst`
   Morphology, parsing, synthesis, agreement, feature-safe generation.

2. `language_core`
   Orthography, mixed-script cleanup, proper-noun normalization, entity-shape rules, alias normalization.

3. `world_core` + entity memory
   Canonical entities, geography hierarchy, ontology facts, curated knowledge.

4. `adam-dialog`
   Intent detection, policy routing, memory updates, retrieval arbitration, reasoning arbitration.

5. `verbalizer`
   Deterministic verbalization first; constrained generative verbalization only as an optional outer layer.

## Workstreams

### A. Language Core

- [x] Extract proper-noun normalization into a dedicated `language_core` module.
- [x] Normalize mixed Latin/Cyrillic proper nouns before memory writes.
- [x] Recover lowercase named places from user text.
- [ ] Add alias and spelling-variant normalization for people and places.
- [ ] Add deterministic orthography validation for session-bound entities.
- [ ] Add polite/register-aware surface constraints that can be reused by verbalizers.

### B. Geography and Canonical Entities

- [x] Reuse `data/world_core/geography_kz.jsonl` as the canonical geography source.
- [x] Detect geo entity kinds for dialog shaping.
- [x] Route `теңіз/өзен/көл/тау` through geo-feature-aware dialog families.
- [x] Add geography alias normalization for historical, Russian-form, and descriptor variants.
- [x] Add canonical entity ids for remembered places instead of plain strings.
- [ ] Extend alias normalization toward colloquial and typo variants with deterministic guards.
- [ ] Expand beyond geography into person-name and organization normalization.

### C. Dialog Quality

- [x] Prefer grounded facts over abstract reasoning when both are available.
- [x] Separate grounded facts from retrieval examples in unknown-topic answers.
- [x] Add deterministic verbalizers for grounded facts.
- [x] Add deterministic verbalizers for reasoning outputs.
- [x] Add answer-mode selection for general/explanation/example queries.
- [ ] Improve self-referential profile questions with canonical entity backing.
- [ ] Tighten insult, refusal, and clarification surfaces without losing determinism.

### D. Hybrid Surface Layer

- [ ] Define a structured answer contract for any future generative verbalizer.
- [ ] Implement a verifier that rejects new facts/entities introduced by generation.
- [ ] Add deterministic fallback whenever constrained generation fails verification.
- [ ] Keep generative output optional and disabled by default until verification is stable.

## Done So Far

### 2026-04-25

- Extracted orthography/proper-noun normalization from `semantics.rs` into `crates/adam-dialog/src/language_core.rs`.
- Reused `data/world_core/geography_kz.jsonl` as the canonical source for geography normalization instead of introducing a duplicate gazetteer.
- Added mixed-script normalization for inputs such as `Aлматы` and case recovery for inputs such as `дӘУЛEТ`.
- Recovered lowercase place names in user inputs such as `қашар ауылында`.
- Fixed location extraction for origin patterns like `Каспий жақтанмын`.
- Added geo-feature-aware response routing for `теңіз`, `өзен`, `көл`, and `тау`.
- Added a geography alias layer for historical and Russian-form variants such as `Алма-Ата`, `Усть-Каменогорск`, and descriptor phrases such as `Каспий теңізі`.
- Kept `adam-kernel-fst` untouched as the morphology authority.

## Next Up

### Immediate

- Build an alias/variant layer on top of canonical geography entities.
- Normalize colloquial and typo variants into one canonical place entry.
- Start carrying entity kind and canonical identity together through memory updates.

### Near-Term

- Introduce canonical entity records for remembered people and places.
- Add entity linker rules that resolve normalized text into stable ids.
- Reduce plain string storage in profile/session memory.

### Later

- Define a constrained JSON contract for a future generative verbalizer.
- Add a verifier that enforces fact preservation and entity preservation.
- Enable a small probabilistic surface layer only after deterministic checks are in place.

## Daily Log

### 2026-04-25

Done:
- Created the dedicated roadmap for `language_core` and hybrid migration.
- Captured the current non-duplication rules and workstreams in one place.
- Recorded the already completed normalization and geo-feature milestones.
- Extended geography normalization with a canonical alias layer over `world_core`.

In progress:
- Transition from raw place strings toward canonical entity-aware memory.

Next:
- Canonical entity-aware memory for places, instead of plain strings.

Risks / duplication avoided:
- Avoided creating a second morphology system outside `adam-kernel-fst`.
- Avoided creating a second hardcoded geography database outside `world_core`.
- Avoided re-embedding normalization helpers back into `semantics.rs`.
- Avoided building a second geography lexicon by treating aliases as a thin layer over canonical `world_core` entries.

### 2026-04-26

Done:
- Added a canonical geography entity resolver that carries `id + canonical + kind` out of `world_core`.
- Switched remembered place entities in dialog memory from string keys to stable `geo_kz_*` ids when a canonical geography match exists.
- Started carrying canonical geography metadata through session updates via `city_id` and `geo_kind`, while keeping `city` as a render-safe string slot.
- Added regression coverage for canonical geo ids in `language_core`, belief entity memory, and end-to-end location absorption.

In progress:
- Reducing string-only session logic so more dialog decisions can consult canonical entity metadata directly.

Next:
- Extend the same canonical-entity pattern from geography into remembered person and organization names.
- Add deterministic colloquial / typo alias guards on top of the canonical geography layer.

Risks / duplication avoided:
- Avoided creating a second place-id scheme outside `data/world_core/geography_kz.jsonl`.
- Avoided breaking existing template rendering by keeping `session[\"city\"]` as the canonical surface form during the migration.
- Avoided duplicating geography kind heuristics across planner and memory paths by threading `geo_kind` from the same canonical resolver.

## Daily Update Template

Copy this block for each workday:

```md
### YYYY-MM-DD

Done:
- ...

In progress:
- ...

Next:
- ...

Risks / duplication avoided:
- ...
```
