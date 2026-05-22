# factual_eval_100 — hallucination root-cause classification

**Date:** 2026-05-19
**Eval set:** `data/eval/factual_eval_100.json` (104 prompts, 17 categories)
**Day's ratchet:** 34 (rc4 morning) → 18 → 13 → **12** (rc4 evening)

This document classifies the **remaining 13 hallucinations** in
`factual_eval_100` after the rc4-evening hardening, so rc5 can pick
the right architectural fix instead of patching matchers.

## Day-of-rc4 progress

| Snapshot | correct | refusal | halluc | pass | grounded | commit |
|---|---:|---:|---:|---:|---:|---|
| rc4 morning release | 59 | 11 | 34 | 56.7 % | 67.3 % | 13fb9de |
| facts.json regen (rc4 release) | 65 | 10 | 29 | 62.5 % | 72.1 % | — |
| Matcher + REFUSAL_MARKERS | 70 | 12 | 22 | 67.3 % | 78.8 % | (intermediate) |
| Clock-intent misfire guards | 71 | 13 | 20 | 68.3 % | 80.8 % | (intermediate) |
| Proverb-fallback suppression on specific-factual | 71 | 15 | **18** | 68.3 % | 82.7 % | 7e91aec |
| **Definitional pattern «Y қандай Y»** | **72** | 19 | **13** | **69.2 %** | **87.5 %** | (this commit) |

Five fixes shipped, all guarded by unit tests in
`planner::tests::factual_guard_*` and the regression-gated
`crates/adam-dialog/tests/factual_eval_100.rs` ceiling.

## The remaining 13

### Category A — **adjacent-fact hallucination** (9 cases)

Adam correctly identifies the entity from the noun_hint but
surfaces an off-topic fact about it. The grounded_fact retrieval
ranks "any fact about X" higher than "fact matching the question's
predicate aspect".

| id | Q | adam's A | what's needed |
|---|---|---|---|
| `astro_002` | «Күн қандай аспан денесі?» | «Аспан денесі астрономия саласына жатады.» | predicate-aware fact selection (intent = IsA(Күн, ?)) |
| `geo_011` | «Каспий теңізі қай елде?» | «Каспий — теңіз.» | predicate-aware (intent = part_of(Каспий, ?country)) |
| `abai_003` | «… қара сөздер неше шығармадан тұрады?» | «Абай 1904 жылы қайтыс болған.» | numeric-aware fact rerank |
| `abai_004` | «Абай қай жылы туылған?» | «Абай — қазақ жазба әдебиетінің негізін салушы.» | year-aware fact rerank |
| `abai_005` | «Абай қай жылы қайтыс болған?» | same generic | year-aware fact rerank |
| `abai_006` | «Абайдың шын аты қандай?» | same generic | proper-noun-aware fact rerank |
| `const_001` | «Конституциясы қашан қабылданған?» | «Қазақстан — Орталық Азиядағы…» | wrong noun_hint (Қазақстан vs Конституция) |
| `ind_004` | «ССГПО қандай кәсіпорын?» | «Кәсіпорын — өндіріс…» | wrong noun_hint (генерический Y вместо X) |
| `java_002` | «Java тілі қашан шықты?» | «`Java` — объектіге бағытталған тіл.» | year-aware fact rerank |

**Naive numeric-guard attempt:** during this session I tried adding
`query_demands_numeric_answer` → `grounded_useful = false when no
digit` to drop these. **Net negative**: dropped +5 hallucinations
in this category but lost 5 *legitimate* answers in
`const_005 / const_006 / const_010` (legit numeric grounded_facts
that the heuristic correctly retrieved). Removed.

**Right fix for rc5:** predicate-aware fact selection upstream.
The retrieval layer already knows the fact's predicate (`is_a`,
`has_quantity`, `part_of`, …). For a query shape `Q-aspect` (year,
country, formula), prefer facts whose predicate matches the aspect
(`has_quantity` for year, `lives_in` for country, etc.). This is
the predicate-typed reasoner work scoped in the v3.9.5 → v4.x
typed-ontology roadmap.

### Category B — **noun_hint mis-extraction** (2 cases)

`chem_001` («Су химиялық формуласы қандай?» → generic water
definition) and `phys_005` («Жарық қандай құбылыс?» → quote about
NATO) — the noun_hint extractor picked the higher-frequency
common noun (су, құбылыс) over the qualified subject (`Су химиялық
формуласы`, `Жарық құбылысы`). Adam then retrieved a fact about
the wrong noun.

**Right fix for rc5:** when the input has a `noun + adj-modifier`
or `noun-poss + property` shape, prefer the compound. Already
half-done — `MULTIWORD_ENTITIES` solves this for entries we
explicitly enumerate. The general case wants compositional
topic extraction over arbitrary `химиялық формула / физикалық
құбылыс` shapes.

### Category C — **legitimate "I don't know" but adam answers anyway** (2 cases)

`neg_001` («Юпитердегі ауа қандай?» → describes Jupiter, not its
atmosphere) and the `astro_003` proverb miss («Ай не нәрсе?»). For
the negative_unknown category we want explicit refusal — the
adjacent-fact retrieval is the wrong mechanism here.

**Right fix for rc5:** add `is_meta_property_query` predicate that
detects «X-DAT/LOC-stem-property» shape (тегі / температурасы /
ауасы / атмосферасы) and routes to clarify-no-data when no
predicate-typed match exists.

## Per-category baseline at 13

| Category | correct | refusal | halluc | total |
|---|---:|---:|---:|---:|
| astronomy | 7 | 0 | 1 | 8 |
| geography_kz | 13 | 1 | 1 | 15 |
| abai_works | 6 | 0 | 4 | 10 |
| kz_constitution | 8 | 2 | 0 | 10 |
| kz_industry | 4 | 0 | 1 | 5 |
| time | 4 | 1 | 0 | 5 |
| programming_java | 4 | 0 | 1 | 5 |
| programming_rust | 3 | 2 | 0 | 5 |
| biology_basic | 3 | 2 | 0 | 5 |
| mathematics_basic | 4 | 1 | 0 | 5 |
| chemistry_school | 4 | 0 | 1 | 5 |
| physics_school | 2 | 2 | 1 | 5 |
| history_kazakhstan | 5 | 3 | 0 | 8 |
| kz_literature | 4 | 0 | 0 | 4 |
| animals | 1 | 2 | 0 | 3 |
| food | 1 | 2 | 0 | 3 |
| negative_unknown | 0 | 1 | 2 | 3 |

## rc5 priority order

1. **Predicate-typed fact rerank** — closes Category A entirely
   (9/13 = 69 % of remaining hallucinations). Largest single ROI.
2. **Compound-topic extraction** — closes Category B (2/13). Smaller
   absolute win but unlocks a lot of compound-subject prompts that
   rc4 doesn't currently measure.
3. **Meta-property refusal route** — closes Category C (2/13).
   Cheapest of the three; one new intent_subkey + template family.

Hitting all three lands the GA #4 ceiling at 0 — the rc5 gate.
