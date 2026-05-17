# Qazaq AI Ұстаз — product spec v1.0

**Working name:** «Qazaq AI Ұстаз» (Қазақ AI Teacher).
**Status:** draft v0, 2026-05-17.
**Target architecture release:** adam v6.0 + the forthcoming
`adam-curriculum` crate.
**Branch of record:** `experimental/agglutinative-neural` until the
product line forks off.
**Linked artefacts:** [`docs/MANIFESTO.md`](../MANIFESTO.md),
[`docs/architecture_neural_v6.md`](../architecture_neural_v6.md),
[`docs/roadmap_v6_v7.md`](../roadmap_v6_v7.md).

## Tagline

A measurable-mastery AI teacher in Kazakh, free for the learner,
offline-capable, audit-trail-complete. **Not** a chatbot tutor —
a structured pedagogical system with verifier-bounded outcomes.

## What this product is and is not

**Is.** A deterministic Kazakh-language teaching system that
covers four subject pillars at v1.0 and grows to a full
primary-through-tertiary curriculum over v2-v5.

**Is not.** A wrapper around an LLM. A chat assistant. A general-
knowledge oracle. A creative-writing tool.

The contract with the learner: **«concept X освоен (mastered)»**
means **«passed N independent test variants of concept X with
≥ 80 % score»**, not «we think you learned». Mastery per concept
is a measurable property recorded in the learner's progress
graph; the learner can show this graph to a parent, a teacher,
an employer, or an admissions committee as evidence of what they
know.

## Why this is the right first product

Three independent sufficient reasons:

1. **Architectural fit.** Every adam guarantee (deterministic,
   zero-hallucination, offline-capable, audit-trail-complete)
   maps directly to an education-product requirement. There is no
   wasted capability.

2. **Market gap.** Kazakhstan spends ~500 000 KZT per learner
   through programmes like TechOrda and adjacent AstanaHub
   tracks; published completion rates are 10-20 %. No
   architectural guarantee of mastery exists in any current
   product — they measure attendance, not learning. A product
   that measures **mastery per concept** competes on a
   dimension no incumbent offers.

3. **Path to revenue.** The same государственный воучер that
   currently flows to TechOrda can flow through our product,
   measured by mastery, at 5-10× lower per-learner cost. We do
   not need to displace incumbents — we need to deliver on the
   same voucher with verifiable better outcomes.

## v1.0 scope — four pillars

The MVP. Everything below ships in v1.0; later versions extend.

### Pillar 1 — Kazakh morphology (grades 5-11)

- **Curriculum graph**: ~250 concepts covering noun inflection
  (7 cases × 2 numbers × 7 possessives × 11 derivations), verb
  inflection (4 tenses × 3 persons × {Sg, Pl} × voice ×
  negation), participles, converbs, derivational suffixes,
  vowel-harmony, consonant-assimilation.
- **Substrate**: already 100 % covered by
  [`adam-kernel-fst`](../../crates/adam-kernel-fst/) — every
  concept is FST-derivable, every answer FST-verifiable.
- **Pedagogy unique to this pillar**: inflect-this-root drills
  with feature-bundle prompts, parse-this-surface drills, gap-
  filling on real Kazakh sentences (Wikipedia / textbook /
  Tatoeba corpora).

### Pillar 2 — School informatics (grades 5-11)

- **Curriculum graph**: ~400 concepts. Grades 5-7: algorithmic
  thinking, flowcharts, basic block-coding. Grades 8-9: text
  programming (Python introduction). Grades 10-11: data
  structures, algorithms, computational complexity at school
  level.
- **Substrate**: MOIN RK 11-grade informatics textbook
  (`data/external/textbooks_kz/grade_11/`) as v1 source;
  grades 5-9 need ingestion.
- **Pedagogy unique**: code-tracing drills, predict-the-output
  drills, fix-this-bug drills. The verifier checks executable
  outputs deterministically.

### Pillar 3 — Mathematics (grades 5-11)

- **Curriculum graph**: ~800 concepts. Grades 5-6: arithmetic,
  fractions, decimals. Grade 7-8: algebra basics, linear
  equations, geometry foundations. Grade 9-10: quadratics,
  trigonometry, sequences. Grade 11 ЕМН/ОГН: calculus
  foundations, probability, ЕНТ-style problems.
- **Substrate**: MOIN RK 7-grade algebra textbook
  (`data/external/textbooks_kz/grade_07/grade_07_algebra.pdf`)
  as v1 anchor; the rest need ingestion.
- **Pedagogy unique**: step-by-step problem solving where the
  verifier checks each step, not just the final answer. Common-
  mistake catalogue per concept so the system can name the
  specific misconception, not just say "wrong".

### Pillar 4 — Rust programming

- **Curriculum graph**: ~150 concepts following the Kazakh
  translation of *The Rust Programming Language* in
  `data/raw/rust_book_kk/` (20 chapters, already shipped as
  v4.7.x). Covers: language basics, ownership, borrowing,
  traits, lifetimes, error handling, collections, generics,
  closures, iterators, smart pointers, concurrency, macros.
- **Substrate**: already-translated 10 277 lines of Kazakh
  technical text + the actual `rustc` for compile-time
  verification.
- **Pedagogy unique**: write-this-function drills where the
  learner's code is compiled and tested deterministically by
  the same `cargo` the rest of the project uses. Compile errors
  become teachable moments tied to specific concepts.

## The "measurable mastery per concept" rubric

For every concept in every pillar:

1. **Definition.** A short Kazakh-language explanation, plus
   prerequisite-concept links. Authored by a subject expert,
   reviewed by a native-speaker linguist.
2. **Examples set.** ≥ 5 worked examples in the learner's
   target grade level. Authored, not LLM-generated.
3. **Test bank.** ≥ 20 independent test items per concept.
   Each item must have a deterministic correct answer the
   verifier can check.
4. **Common-mistake catalogue.** ≥ 3 typical wrong answers per
   item, each tied to a specific misconception with a remediation
   pointer.
5. **Mastery threshold.** Learner passes ≥ 80 % score on a
   random N-item subset (N typically 5-10) drawn from the
   concept's test bank. The same subset is never used twice for
   the same learner.

A concept is **mastered** when the threshold is reached. The
mastery is recorded with: (concept_id, learner_id, timestamp,
items_attempted, items_correct, time_taken). This record is the
public-facing "verifiable proficiency" artifact.

## Pricing model

**For the learner: free.** Offline-installable. No subscription,
no ads, no telemetry beyond explicit progress sharing.

**For the institution (school / гос. программа / частная
компания):** capex per device. A ~50 000 KZT tablet/Chromebook
serves 30 learners over 6 months. Per-learner cost: ~1 700 KZT.
Compared to TechOrda's ~500 000 KZT per voucher, this is **300×
cheaper**.

**Revenue path:**

- **Stage 1 (year 1):** government-grant-funded pilot. МОН РК
  + AstanaHub Foundation + international development partners
  (UNICEF, World Bank "Education for Knowledge Economy" line)
  cover the initial curriculum-authoring team.
- **Stage 2 (year 2):** TechOrda-voucher integration. Voucher
  redeemable through our product; outcome verification gates
  payment.
- **Stage 3 (year 3+):** SaaS to private schools, корпоративных
  программ обучения (бизнес-обучение программированию для
  сотрудников), и регионального экспорта (Kyrgyz, Uzbek,
  Tatar versions in v6.1+).

## v1.0 launch criteria

The product ships under the «Qazaq AI Ұстаз» brand when **all**:

- [ ] adam v6.0.0 GA shipped (acceptance criteria in
  [`architecture_neural_v6.md`](../architecture_neural_v6.md) §9
  all green).
- [ ] Four pillars each have at least 50 % of their published
  concept count covered: morphology ≥ 125 / 250, informatics
  ≥ 200 / 400, math ≥ 400 / 800, Rust ≥ 75 / 150.
- [ ] Each covered concept has ≥ 5 examples, ≥ 20 test items,
  ≥ 3 common-mistake entries.
- [ ] Learner-facing UI (Tauri-packaged Rust app) installable
  on Linux / macOS / Windows / Android tablet.
- [ ] One pilot school deployment for 4 weeks; observed
  per-concept mastery rate published.
- [ ] Privacy audit: no learner data leaves the device unless
  explicitly exported by the learner.

## v2.0 / v3.0 / v5.0 roadmap

- **v2.0** — add grades 1-4. Reading, basic numeracy, world-
  studies, Kazakh language. ~1 800 new concepts. Target: 2027.
- **v3.0** — add high-school subjects beyond math/informatics:
  physics, chemistry, biology, history, geography, КазЛит,
  English. ~3 500 new concepts. Target: 2028.
- **v4.0** — early-stage university programmes. Computer
  science, applied mathematics, economics basics. ~2 000 new
  concepts. Target: 2029.
- **v5.0** — comprehensive university coverage including
  jurisprudence, medicine basics, engineering foundations.
  Cross-language extension (Kyrgyz / Uzbek / Tatar). Target:
  2030+.

## Risks named honestly

1. **Curriculum-authoring is the bottleneck.** Architecture work
   ends ~v6.0; everything after is content. Each pillar needs
   1-2 full-time subject experts. Total team: 5-8 people over
   2-3 years for v1.0 → v3.0. Without this team the product
   ceases to grow.

2. **Government procurement cycles are slow.** Voucher-
   integration with TechOrda is 12-18 months minimum. Mitigation:
   parallel commercial track (paid private schools / corporate
   training) for cashflow.

3. **«Гарантия mastery» is a legal trap if mis-worded.** We use
   **"measurable mastery per concept"** language exclusively,
   never «100 % выпускаются образованными». The mastery record
   is per-concept, not per-course; learners who drop out have
   partial-but-real mastery records, not a binary fail.

4. **Incumbent political backing.** TechOrda + AstanaHub have
   established government relationships. Mitigation: position
   as voucher-redemption product *within* their ecosystem, not
   replacement *of* it. Their KPIs improve when our product
   delivers mastery; this is win-win.

5. **LLM-tutor competitors (Khan Academy + GPT) move fast.**
   Mitigation: our defensible USPs — offline, 10× cheaper,
   verified mastery, Kazakh-native morphology — are not things
   they can match on their architecture. Their advantage:
   breadth and creative tasks; ours: depth and verifiability.

6. **Privacy regulation tightens.** Kazakhstan personal-data
   law evolves; learner data needs careful handling. Mitigation:
   privacy-by-construction (no data leaves the device by
   default), which we already have as an architectural choice
   from MANIFESTO §3.2.

## What we explicitly do not claim

- We do **not** guarantee any specific completion rate. We
  guarantee a **measurement framework** (mastery per concept)
  and a per-learner **record**. The completion rate is the
  product of the framework and the learner's effort, not the
  framework alone.
- We do **not** claim adam will replace teachers. It will
  replace **bad / overpriced courseware**, and free up teacher
  time for the things teachers actually do best (motivation,
  social skills, creative-task feedback).
- We do **not** claim adam «знает всё». Coverage is bounded by
  the curriculum graph — outside it, adam refuses or asks
  clarification. That refusal is a feature; an honest "I don't
  know this yet" is more useful than a confident wrong answer.

## Open questions to resolve before launch announcement

1. **Brand name lock.** «Qazaq AI Ұстаз» is the working name;
   a trademark search + state registration is needed.
2. **Mastery threshold standardisation.** 80 % is the default;
   subject experts may argue for 75 % on conceptual subjects
   vs 90 % on rote subjects. Standardise per-pillar.
3. **Common-mistake authoring scale.** ≥ 3 misconceptions per
   item × ~1 600 v1.0 concepts × 20 items = 96 000 misconception
   entries. Realistic timeline?
4. **Tablet OS choice.** Android tablets cheapest for schools;
   Apple iPads have Educational programmes; Linux laptops have
   most flexibility. Pilot answers this.
5. **Offline-sync model.** Mastery records need to be portable
   (a learner switching schools) but private. Probably an
   exported signed JSON, importable by a new install. Spec
   pending.

## Status of authoring infrastructure

Done:
- adam-kernel-fst (pillar 1 substrate).
- adam-agg-tokenizer + adam-agg-synth + adam-agg-model (deep
  morphology + neural composition).
- rust_book_kk corpus (pillar 4 substrate, ~10 277 lines).
- MOIN RK grade-11 informatics + grade-7 algebra textbooks
  (pillars 2 and 3 anchor sources).

Missing:
- `adam-curriculum` crate (concept graph + diagnostic engine +
  lesson planner + outcome verifier — L7-L10-edu).
- Curriculum-authoring CLI for subject experts.
- Learner-facing app (Tauri shell over the adam workspace).
- Per-pillar content for everything except Rust + grade-11
  informatics + grade-7 algebra.

## Immediate next actions (post-training-run)

1. Read training results; decide whether v6.0 architecture is
   green to build the product on.
2. If green: scaffold `adam-curriculum` crate with the four-
   layer L7-L10-edu interfaces.
3. Author the first ~25 concepts in Pillar 1 (morphology) as
   the reference curriculum-format example.
4. Outreach materials for МОН РК + AstanaHub + TechOrda
   informational meetings.
