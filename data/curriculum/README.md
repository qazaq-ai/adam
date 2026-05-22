# `data/curriculum/` — «Qazaq AI Ұстаз» content

Per-pillar curriculum content for the v6.0+ educational product
described in [`docs/product/qazaq_ai_ustaz_v1.md`](../../docs/product/qazaq_ai_ustaz_v1.md).
Loaded by the `adam-curriculum` crate; structure mirrored exactly
across pillars so authoring tooling can be shared.

## Layout

```
data/curriculum/
├── kazakh_morphology/
│   ├── concepts.jsonl       # 1 Concept per line
│   └── test_items.jsonl     # 1 TestItem per line, ≥5 per concept (v1.0 floor)
├── school_informatics/
│   └── …
├── mathematics/
│   └── …
└── rust_programming/
    └── …
```

## File format

Both files are **JSON-lines** (`*.jsonl`) — one JSON object per
line, UTF-8, blank lines and `#`-prefixed comments allowed. The
Rust types are:

- `concepts.jsonl` → `adam_curriculum::Concept`
- `test_items.jsonl` → `adam_curriculum::TestItem`

See the rustdoc on those types for the exact field set. The
loaders (`load_concepts_jsonl`, `load_test_items_jsonl`) accept
both files and return validated graphs / item banks.

## Authoring rules (v1.0)

The product spec
([`docs/product/qazaq_ai_ustaz_v1.md`](../../docs/product/qazaq_ai_ustaz_v1.md))
§"The 'measurable mastery per concept' rubric") fixes these
floors. Every concept must satisfy:

1. **Definition.** Non-empty `name_kk` and `explanation_kk`,
   authored by a subject expert, reviewed by a native-speaker
   linguist. Both fields must be valid Kazakh; loanwords flagged
   for follow-up.
2. **Examples set.** ≥ 5 worked examples in the target grade
   level. Authored, not LLM-generated.
3. **Test bank.** ≥ 20 independent items per concept at GA
   (`v1.0` ships with ≥ 5 per concept as the floor — see
   `adam_curriculum::audit_coverage`).
4. **Common-mistake catalogue.** ≥ 3 entries per test item, each
   tied to a specific misconception with a remediation pointer
   (Kazakh-language explanation).
5. **Mastery threshold.** Default 0.80. Subject experts may set
   per-concept; standardisation per-pillar is one of the open
   product-spec questions.

A concept is **mastered** when the L10-edu outcome verifier
records a session of ≥ 5 items at ≥ threshold score AND all
prerequisites are already mastered.

## v1.0 status

| Pillar | Concepts shipped | Items shipped | Floor met? |
|---|---:|---:|---|
| kazakh_morphology | 10 | 50 (5/concept) | ≥ 5/concept ✓ (GA target 20) |
| school_informatics | 0 | 0 | TODO |
| mathematics | 0 | 0 | TODO |
| rust_programming | 0 | 0 | TODO |

Curriculum authoring is the v6.0 critical-path bottleneck — see
the product-spec §"Risks named honestly" entry "Curriculum-
authoring is the bottleneck".

## Validating

```bash
# Load + audit pillar:
cargo run --release -p adam-curriculum --example audit -- \
  data/curriculum/kazakh_morphology
```

(The `audit` example is forthcoming; for now, the integration
test `crates/adam-curriculum/tests/load_seed.rs` validates the
seed content on every CI run.)
