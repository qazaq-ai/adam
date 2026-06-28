# Phase classify-eval — dialog-act classifier comparison

**Pack:** `experiments/hybrid_qlm/classify_eval/classify_eval.json`
**Items:** 30 labeled Kazakh queries across 5 classes (Greeting / FactualQuery / Clarify / RefusalSignal / Other).

## Summary

- Keyword rules:  **27 / 30** (90%)
- LM-4B classify: **20 / 30** (66%)
- Random baseline: 20%

- LM lift over keyword: -24 pp
- Keyword lift over random: +70 pp
- LM lift over random:      +46 pp

### Per-class breakdown

| Class           | Total | Keyword     | LM-4B       |
| --------------- | ----- | ----------- | ----------- |
| Greeting        |     6 | 6/6 (100%) | 6/6 (100%) |
| FactualQuery    |     8 | 5/8 (62%) | 7/8 (87%) |
| Clarify         |     5 | 5/5 (100%) | 3/5 (60%) |
| RefusalSignal   |     5 | 5/5 (100%) | 4/5 (80%) |
| Other           |     6 | 6/6 (100%) | 0/6 (0%) |

## Disagreements

- expected `FactualQuery` | kw ✗ `Other` | lm ✓ `FactualQuery` | input: `Темірдің химиялық таңбасы.`
- expected `FactualQuery` | kw ✓ `FactualQuery` | lm ✗ `Clarify` | input: `Алгоритм дегеніміз не?`
- expected `FactualQuery` | kw ✗ `Other` | lm ✓ `FactualQuery` | input: `Балқаш көлі туралы.`
- expected `FactualQuery` | kw ✗ `Other` | lm ✓ `FactualQuery` | input: `Жатыс септігі сұрағы.`
- expected `Clarify` | kw ✓ `Clarify` | lm ✗ `Other` | input: `Қайталашы.`
- expected `Clarify` | kw ✓ `Clarify` | lm ✗ `Greeting` | input: `Қалай дедіңіз?`
- expected `RefusalSignal` | kw ✓ `RefusalSignal` | lm ✗ `Other` | input: `Дұрыс емес.`
- expected `Other` | kw ✓ `Other` | lm ✗ `FactualQuery` | input: `Менің атым Дәулет.`
- expected `Other` | kw ✓ `Other` | lm ✗ `FactualQuery` | input: `Алматыда тұрамын.`
- expected `Other` | kw ✓ `Other` | lm ✗ `Greeting` | input: `Сау бол!`
- expected `Other` | kw ✓ `Other` | lm ✗ `FactualQuery` | input: `Менің белім ауырады.`
- expected `Other` | kw ✓ `Other` | lm ✗ `FactualQuery` | input: `Бүгін жұмыс істедім.`
- expected `Other` | kw ✓ `Other` | lm ✗ `FactualQuery` | input: `Кешегі күн жақсы өтті.`
