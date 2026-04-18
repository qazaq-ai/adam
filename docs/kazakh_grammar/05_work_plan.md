# Daily Work Plan — v1.0.0 Deterministic Architecture

Status: **active**. Aligned to Astana time (GMT+5). Target: 10h working days, rest at night.

## Token budget discipline

User subscription: 5× Pro tier, estimated ~150–200k tokens/day usable, peaking around 250k on long-context sessions.

Work-type cost estimates:

| activity | tokens/unit | pace |
|---|---|---|
| Reading grammar + note-taking | ~5k/section | 10–20 sections/day |
| Writing lexicon entry (1 root) | ~500 | 200–400/day |
| Writing FST rule | ~2k | 50–100/day |
| Rust coding + debugging | ~10k/feature | 10–15 features/day |
| Long design documents | ~20k each | 2–3/day |
| Running training experiments | minimal (I/O only) | unlimited |

**Budget rule**: aim for ≤150k tokens/day under normal load so heavy-debug days don't overflow. Keep one "long design" slot per week.

## Weekly milestones

### Week 1 (starting 2026-04-18) — Phonology + Framework

**Daily work (Sat–Fri, skip Sun for rest):**
- Mon–Tue: Complete `01_phonology.md` with full allomorph matrix (today's stub extended).
- Wed: Fork Apertium-kaz locally, catalogue its `twolc` rule file mapping to our model.
- Thu: Design `adam-kernel-fst` module layout (Rust types for 2-level morphology).
- Fri: Write `05_fst_rust_design.md` and skeleton Rust types (no implementation yet).

**Deliverables**:
- 6 markdown files in `docs/kazakh_grammar/` complete
- Rust skeleton types for `PhonologicalRule`, `SuffixSlot`, `RealisationTable`
- No runtime behaviour yet; tests are smoke-tests only

### Week 2 (starting 2026-04-25) — Morphology Implementation

- Mon–Wed: Build `adam-kernel-fst::phonology` module with all 8-matrix tables.
- Thu–Fri: Build `adam-kernel-fst::morphotactics` module (noun + verb state machines).

**Deliverables**:
- Tests: given `(бала, noun, plural_dat)`, synthesis outputs `балаларға`.
- Tests: given `балаларға`, parser returns `(бала, noun, plural_dat)`.
- Coverage on 464 existing segmentation_eval items: target 99%+.

### Week 3 (starting 2026-05-02) — Lexicon Expansion

- Mon–Tue: Import Apertium-kaz lexicon; map 30k entries to our schema.
- Wed–Thu: Manual verification pass on top 2000 most-frequent.
- Fri: Merge with existing 4454 curated entries; dedupe and conflict-resolve.

**Deliverables**:
- `data/tokenizer/segmentation_roots.json` at 15k–30k entries.
- FST coverage on full unified corpus: target 90%+ (up from 71% with 4454 roots).

### Week 4 (starting 2026-05-09) — Evaluation & Adjustment

- Refine rules based on corpus failures.
- Write `06_v1_evaluation.md` documenting FST-based metrics (replacing BPE perplexity).
- Commit-tag `v1.0.0-alpha` (not a release, just a milestone).

### Weeks 5–8 — Root-LM

- Design small transformer or GRU over root+feature sequences.
- Training data prepared via FST parser.
- Training runs remain ≤2h (fast-iter principle).

### Weeks 9–12 — Integration + Release

- End-to-end inference binary (`generate_v1`).
- Full `validate_foundation_v1` pipeline.
- v1.0.0 release when all success criteria in `00_architecture_v1.md` §9 met.

## Daily rhythm (sample)

Astana GMT+5. Working window ~08:00–18:00 with 1h lunch.

| block | 08:00–11:00 | 11:00–13:00 | 14:00–17:00 | 17:00–18:00 |
|---|---|---|---|---|
| Mon | deep design / architecture | lexicon entries | Rust coding | review + commit |
| Tue | study (read grammars) | lexicon entries | Rust coding | review |
| Wed | lexicon entries | Rust coding | test/debug | review |
| Thu | Rust coding | test/debug | lexicon | commit + push |
| Fri | weekly review + writeup | FST rules | FST rules | week-end checkpoint |

Saturday: lighter load — reading & notes only.
Sunday: rest (no work).

## Progress tracking

Each working day ends with:
1. Git commit with clear title and what-was-learned notes.
2. Update `05_work_plan.md` in-place with actual hours logged (this file).
3. One-paragraph summary added to a running log file `docs/kazakh_grammar/DAILY_LOG.md`.

## Stop-conditions (when to pause and reassess)

- If FST coverage doesn't exceed 85% by end of Week 3 → reassess lexicon strategy.
- If root-LM can't converge to reasonable perplexity-on-roots in 2h training → reassess factorisation.
- If at any point we discover a structural reason the FST+LM hybrid can't work for Kazakh → document and pivot (publish what we learned).

## What "success" looks like week-by-week

- W1: Can read own notes months later and understand the system.
- W2: FST round-trips 99% of existing test corpus.
- W3: FST round-trips 90% of production corpus.
- W4: Evaluation pipeline proves it.
- W5-8: Root-LM converges.
- W9-12: Integrated system generates grammatical Kazakh.

No v1.0.0 tag until an outside speaker of Kazakh reviews 50 generations and rates ≥40 as "natural-sounding".
