# v6.2.0 — Neurosymbolic Agglutinative Algebra (design)

**Status.** Design draft. To be implemented on
`experimental/v6_2_agglutinative_algebra`, forked from `main` at
v6.1.50 (commit `44ff444c`) on 2026-05-24. **No code until this
doc is signed off** — same discipline as v6.1.0
(`v6_1_answer_ir_design.md`).

**Origin.** 11 voice + text REPL audits across 2026-05-23 and
2026-05-24 produced 22 fixes in 11 releases (v6.1.0 → v6.1.50).
On 2026-05-24 the user signalled:

> «Мы постоянно что-то исправляем, а баги не кончаются. Серьёзного
> интеллектуального продвижения модели не видно. … Необходима
> научно-исследовательская работа: как взять лучшее из LLM, но
> избавиться от вероятностности, дороговизны и галлюцинаций.»

A separate 2026-05-24 dialog with the codex external reviewer
converged on the same conclusion (saved verbatim in
[`project_v6_2_neurosymbolic_arc`](../docs/.gitkeep) memory
directive). v6.2.0 is the **architectural pivot** that conclusion
demands.

## Thesis under test

> A **neurosymbolic agglutinative architecture** — where typed
> agglutinative-suffix operators form an explicit algebra,
> small supervised neural components produce typed closed-set
> outputs at variation-tolerance pressure points, and a
> deterministic verifier remains the SOLE source of factual
> truth — can deliver LLM-comparable understanding of natural
> Kazakh while staying CPU-runnable on a smartphone (and
> eventually a watch), with **architectural impossibility of
> hallucination**.

If the thesis holds, adam becomes the first applied demonstrator
of an AI architecture that is simultaneously:
- **Cheap**: CPU-only, no GPU, single binary.
- **Safe**: every factual claim traces to a `source_fact_id`,
  verifier impossible to bypass.
- **Mobile**: ARM build, ≤ 200 MB RSS, ≤ 100 MB artifacts.
- **Natural**: dialog quality on par with LLM-class systems on
  the curated domain.

If the thesis fails, v6.1.50 remains the production opt-in and
the v6.2.0 work is documented as an honest negative result
(same discipline as the E3 retrieval-ranker fail noted in
`docs/third_path_results.md`).

## What's already in place (v6.1.50 inventory)

This audit is the foundation v6.2.0 builds on.

| Layer | Status at v6.1.50 | What v6.2 changes |
|---|---|---|
| **L0 — FST morphology** | shipped (`adam-kernel-fst`) | unchanged |
| **L1 — Typed Predicate enum** | 22 typed predicates ([`Predicate`](../crates/adam-reasoning/src/lib.rs)) | unchanged |
| **L2 — Suffix tags** | exist as flat fields on `Analysis` (case / number / possession / tense) | **typed operators** with composition algebra (new) |
| **L3 — Semantic frame** | ad-hoc structs (`SentenceFrame`) | **single `Frame` contract** (refactored) |
| **L4 — Question shape / focus** | `QuestionShape` (5), `PredicateFocus` (12) | absorbed into `QueryIR` (new central type) |
| **L5 — Retrieval** | curated `facts.json` + IsA-alias bridge | **+ learned ranker** + **sense disambiguation** |
| **L6 — Verifier** | `Verifier` for 5 yes/no shapes | **universal contract** for ALL factual replies |
| **L7 — Realiser** | template-driven, variant strategies | **+ natural realiser** (closed-set composition, no fact invention) |
| **Eval** | `factual_eval_100`, `v6_1_predicate_eval_30`, `v6_0_6_audit_regression` (32 cases) | **+ HumanDialogEval v1** (100-150 prompts, 7-axis) |
| **Mobile** | M2 8 GB tested | **+ ARM PoC** (Snapdragon 8 Gen 1 / Apple A17) |

## Five root causes (audit evidence)

The v6.1.x patch series surfaced these five categories repeatedly.
Each is a single symptom of a deeper architectural gap that no
amount of patching closes.

### Root #1 — No grammatical-context-aware STT correction

Voice REPL audit 2026-05-24: «Менің атым Дәулет. Мен
Бағдарламасы.» Whisper emits the «-сы» suffix where the
grammatical context («Мен» subject) demands «-шы-мын» 1sg-copula.
The cascade has no facility to say: «I expect a 1sg-copula form
after «Мен» — what's the nearest phonetic neighbour that fits?»

**Why patches don't compose**: voice aliases enumerate specific
mishearings («Бағдарламасы → Бағдарламашы», «кубейт → көбейт») —
they're a band-aid. The real fix needs N-best lattice rescoring
+ phonetic-bounded fuzzy match + FST round-trip + **grammatical
expectation** at the noun-hint level.

### Root #2 — No answer-shape detection

Voice REPL audit 2026-05-24: «Жылда неше ай?» (= "how many
months in a year?") routes to topic extraction over «жыл»,
retrieves the «тоқсан» fact (semantically adjacent, wrong
concept), replies «Бір жылда төрт тоқсан болады.» The cascade
does NOT recognise «неше X?» as a **quantitative** question.

**Why patches don't compose**: v6.1.50 ships a closed-set time-
unit Count + Disagreement table — works for time units, doesn't
cover Function / Procedure / ListExamples / SenseDisambiguation
/ Comparison / Definition. Every new answer-shape needs its own
detector. The real fix needs a typed `AnswerShape` discriminator
with a closed-set ontology and a learned classifier from input
to shape.

### Root #3 — No sense disambiguation

Voice REPL audit 2026-05-24:
- «Мүгін қандай күн» — cascade picked «мүк» (= moss) from topic
  extraction and answered with botany. (Whisper-noise for
  «Бүгін»; but even with correct transcription «Ағаш қандай
  материал?» retrieves the data-structure sense of «ағаш».)
- «Ай туралы не білесіз?» — cascade mixes moon and month.
- «Заң» — law document vs. legal rule vs. effective date.

**Why patches don't compose**: per-word aliases can't capture
context-dependent sense. The real fix needs `SenseKey` per fact
+ a domain-aware filter that consults
`dialog_context.subject_under_discussion` and
`predicate_focus` at retrieval time.

### Root #4 — No structured indexing

Every detector currently scans the same data with substring
patterns:

```rust
if joined.contains("моделсің") || joined.contains("моделсіз") ...
```

41 such detectors run per turn. Each new audit grows the
`contains` set, never the index. `MULTIWORD_ENTITIES` is a
linear scan. `VOICE_ALIASES` is a linear scan. `extracted_facts.iter().find()`
is a linear scan.

**Why patches don't compose**: linear scans are fast NOW (4 116
facts / ~100 multiword entities) but won't scale to v6.2.0's
ambition (5 000+ curated entries → 50 000+). The real fix needs
indexes by POS, predicate, sentence-type, domain, sense — built
offline, mmap-loaded at startup.

### Root #5 — No human-quality eval gate

`factual_eval_100` and `v6_1_predicate_eval_30` pin specific
contracts (hallucination ceiling, predicate routing). They DON'T
catch:
- Tautology («Ұлттық тағам — тағам»).
- Wrong-sense retrieval («Ағаш материал?» → data-structure).
- Robotic phrasing («Қысқаша айтсам, X — қала» on short answers).
- Unsupported claims when an exact fact exists.
- Repetitive honorific.

**Why patches don't compose**: without a **human-rated**
relevance / naturalness / continuity eval gate, every release
ships with relevance regressions invisible to the existing
pins. The real fix needs `HumanDialogEval v1` as a CI gate.

## The unified abstraction — Agglutinative Algebra

The mathematical kernel that makes v6.2.0 possible.

### Suffix operators as types

A Kazakh word is a composition of typed operators:

```text
бала + лар + ы + на
root + plural + possessive_3sg + dative
≈ Dative(Possessive3(Plural(бала)))
```

Each suffix is a **typed operator** over meaning:

| Operator | Surface | Type signature |
|---|---|---|
| `Plural` | -лар / -лер / -дар / -дер / -тар / -тер | `Noun → PluralNoun` |
| `Possessive(person, number)` | -ым / -ың / -ы / -іңіз / -міз / … | `Noun → PossessedNoun` |
| `Case(c)` | -ны / -на / -нан / -нда / -мен / -ның | `Noun → CaseMarkedNoun` |
| `Verb-Person(p, n)` | -мын / -сың / … | `VerbStem → ConjugatedVerb` |
| `Verb-Tense(t)` | -ды / -ган / … | `VerbStem → TensedVerb` |
| `Negation` | -ма / -ме / -па / -пе / емес | `Predicate → NegatedPredicate` |
| `Question` | -ма / -ме / -ба / -бе / -па / -пе | `Statement → Question` |

Valid composition is **FST-enforced**: illegal chains
(`Plural ∘ Tense` on the same root) are rejected at parse time.

### Algebra rules

```text
1. Identity:        Root(root) = root
2. Composition:     Op_b(Op_a(x)) is valid iff Op_b accepts Op_a's output type
3. Commutativity:   GENERALLY false — order matters in Kazakh
4. Idempotence:     specific ops are idempotent (e.g. Plural ∘ Plural = Plural)
5. Inverses:        Negation ∘ Negation ≈ Affirmative (modulo discourse)
```

These are not philosophy — they're **executable rust traits**
that any implementation must satisfy. v6.2.0 Stage 1 implements
the type system; the FST already enforces the validity gate.

### Why this matters for AI

LLMs learn grammar implicitly from billions of tokens — the
grammar is hidden in attention weights. Adam's agglutinative
algebra makes the grammar **explicit in the type system**:
illegal compositions can't even be constructed. A small
supervised neural component (say, a 1 MB classifier) only has
to learn the **assignment** from surface form to typed
composition — orders of magnitude smaller learning problem than
«predict the next token».

## The 7-layer architecture

```text
User text
  ↓ MorphLayer       (FST: surface → morph lattice [Vec<Analysis>])
  ↓ OperatorLayer    (typed suffix operators: Case / Person / Tense / …)
  ↓ FrameLayer       (Frame { agent, predicate, object, modality, polarity })
  ↓ QueryIRLayer     (QueryIR { subject, predicate, sense, shape, domain, … })
  ↓ EvidenceLayer    (retrieve + RANK facts; deterministic FIRST, learned LATER)
  ↓ ProofLayer       (Verifier: every claim has source_fact_id; reject if not)
  ↓ Realizer         (AnswerIR → natural Kazakh; closed-set composition)
```

**Critical contract**: each layer emits a **typed, inspectable,
verifiable intermediate object**. No layer is a black box. Any
factual claim that exits the Realizer MUST trace back through
ProofLayer to a curated `source_fact_id`. Neural is allowed
**between** layers (e.g. ranking candidates inside EvidenceLayer)
but cannot bypass layer boundaries.

### Layer-by-layer responsibilities

#### MorphLayer (L0)
- **Input**: raw user text + lexicon.
- **Output**: `Vec<MorphLattice>` — for each token, the FST-valid
  analyses ranked by suffix-chain prior.
- **Status**: shipped in `adam-kernel-fst`. No v6.2 change.

#### OperatorLayer (L1)
- **Input**: morph lattice.
- **Output**: `Vec<SuffixOp>` for each token — typed operators
  with their semantic roles.
- **Status**: NEW in v6.2. Implements the Agglutinative Algebra
  type system (Stage 1).

#### FrameLayer (L2)
- **Input**: operator stream.
- **Output**: `Frame { agent, predicate, object, modality,
  polarity, evidentiality, tense, aspect }` — a *typed semantic
  frame*, NOT a black box.
- **Status**: partial (`SentenceFrame` exists). Refactor to one
  canonical type in v6.2 (Stage 2).

#### QueryIRLayer (L3)
- **Input**: Frame + dialog context.
- **Output**:
  ```rust
  pub struct QueryIR {
      pub subject: SlotRef,                 // canonical-noun-resolved
      pub predicate: Option<PredicateFocus>,
      pub sense: Option<SenseKey>,          // disambiguation key
      pub shape: AnswerShape,               // Count / TimeWhen / Definition / …
      pub domain: Option<Domain>,           // KRU / time / science / …
      pub discourse: DiscourseHint,         // anaphora, continuation
      pub safety: SafetyMode,
      pub confidence: f32,                  // parser confidence (0..1)
  }
  ```
- **Status**: NEW in v6.2 (Stage 3). The CENTRAL contract. Every
  factual-reply path MUST construct a QueryIR before retrieval.

#### EvidenceLayer (L4)
- **Input**: QueryIR + indexed knowledge graph.
- **Output**: `Vec<Claim>` ranked by relevance.
- **Status**: partial. v6.2 adds: structured indexes
  (`HashMap<(subject, predicate), Vec<&Fact>>`), sense filter,
  learned ranker (Stage 4-5).

#### ProofLayer (L5)
- **Input**: ranked claims.
- **Output**: `ClaimSet` where every claim has a verified
  `source_fact_id`.
- **Status**: partial (`Verifier` for 5 shapes). v6.2 makes it
  universal: every factual reply MUST pass through this layer.

#### Realizer (L6)
- **Input**: `AnswerIR { claims, shape, discourse, style }`.
- **Output**: natural Kazakh text.
- **Status**: partial (templates + variant strategies). v6.2
  adds the natural-discourse composer that avoids tautology,
  smooth multi-claim joining, no fact invention (Stage 6).

## Where neural is allowed (and where it's not)

| Pressure point | Neural allowed? | Output type |
|---|---|---|
| **Intent classification** | Yes (E1 already exists) | closed set of 41 labels |
| **Slot extraction** | Yes (E2 already exists) | closed BIO tag set |
| **STT N-best rescoring** | Yes (v6.2 new) | re-rank K parses, no new text |
| **Sense disambiguation** | Yes (v6.2 new) | closed `SenseKey` |
| **Evidence ranking** | Yes (v6.2 new) | re-order existing facts, no fact synthesis |
| **Answer-shape detection** | Yes (v6.2 new) | closed `AnswerShape` enum |
| **Realiser style picking** | Yes (rng OK today) | closed template variant set |
| **Fact assertion** | **NO — verifier owns this** | only from curated `source_fact_id` |
| **Free text generation** | **NO — Realizer is closed-set** | template composition only |

The line is clear: **neural for SELECTION, deterministic for
TRUTH**. This is the architectural guarantee that makes
hallucination impossible by construction.

## Predeclared success criteria

v6.2.0 ships **iff all of these hold** measured on the v6.1.50-
frozen baselines + the new `HumanDialogEval v1`:

| Criterion | Target | How measured |
|---|---|---|
| HumanDialogEval relevance | ≥ 90 % | Frozen 100-prompt battery, 5-pt human relevance scale |
| HumanDialogEval naturalness | ≥ 85 % | Same battery, 5-pt human naturalness scale |
| Hallucination | **0** | Every factual reply traces to `source_fact_id` |
| Wrong-sense | **0** | Audit-flagged set: Ай / Күн / Ағаш / заң / бағдарлама / еңбек |
| Cascade pass-through | 32 / 32 | `v6_0_6_audit_regression` |
| `factual_eval_100` ceiling | ≤ 3 | Existing test |
| `v6_1_predicate_eval_30` | ≥ 29 / 30 | Existing test (currently 29 / 30) |
| Latency p99 | ≤ 35 ms | Default cascade on M2 8 GB (v6.1.45 baseline) |
| RSS | ≤ 320 MB | Default cascade on M2 8 GB (v6.1.45 baseline) |
| **ARM PoC** | RSS ≤ 200 MB, p99 ≤ 50 ms | Snapdragon 8 Gen 1 OR Apple A17 build |
| **Artifact size** | ≤ 100 MB total | Binary + lexicon + facts.json + indices + models |

The **mobile PoC** is the single most important new criterion. If
v6.2.0 can't run on a phone within budget, the whole thesis is
suspect. Watch deployment is the long-term goal, not v6.2.0 ship
gate.

## Predeclared anti-success (rollback triggers)

Roll back to v6.1.50 if ANY fire:

1. **Verifier bypass.** A factual reply emerges without a
   `source_fact_id` trace at ANY exit point. Hard fail.
2. **Hallucination regression.** A learned component is found
   to be the SOURCE of an unsupported claim. Hard fail.
3. **Cascade regression.** `ADAM_ANSWER_IR=0` no longer
   bit-identical to v6.0.0 default. Hard fail.
4. **HumanDialogEval regression.** Drops below 80 % relevance
   OR 75 % naturalness.
5. **Latency / RSS regression.** Default cascade p99 > 50 ms
   OR RSS > 400 MB.
6. **Mobile PoC fails to build OR exceeds 250 MB RSS / 80 ms p99.**

Each stage has its own success + anti-success — see Stages
below.

## Implementation stages

Each stage ships independently on the experimental branch
behind `ADAM_V62=1`. Default behaviour unchanged until the
default-on gate fires at the end.

### Stage 1 — Agglutinative Algebra type system (~3 days)

- Add `crates/adam-algebra/` workspace member.
- Define `Root`, `SuffixOp`, `Composition` traits.
- Implement typed operators for all v6.1 morphology tags.
- Compose against `adam-kernel-fst` for validity.
- Unit tests for the algebra rules (identity, composition,
  inverse, idempotence).
- **Success**: 100 % of v6.1 test corpus surfaces typecheck.
- **Anti-success**: any FST-valid surface fails to typecheck.

### Stage 2 — Frame layer refactor (~2 days)

- Single `Frame { agent, predicate, object, modality,
  polarity, evidentiality, tense, aspect }` type.
- `Frame::from_morph_lattice(&MorphLattice) -> Option<Frame>`.
- Migrate `SentenceFrame` / `SentenceDecomposition` callers.
- **Success**: every v6.1 NLG rule path can be expressed via
  `Frame` without loss.
- **Anti-success**: any v6.1 rule cannot be expressed.

### Stage 3 — QueryIR (~3 days)

- Define `QueryIR` struct + builder.
- Rule-based `text → QueryIR` resolver (no neural yet).
- Migrate `Stage 3 typed-focus probe` from v6.1 to consume
  `QueryIR` instead of `(noun_hint, focus)` pair.
- **Success**: `v6_1_predicate_eval_30` passes on the new path.
- **Anti-success**: any v6.1.50 fact-resolution case regresses.

### Stage 4 — Indexed knowledge graph (~2 days)

- `data/indices/v1/` mmap-loaded indices:
  `facts_by_(subject, predicate)`, `facts_by_domain`,
  `facts_by_sense`, `multiword_entities_trie`,
  `voice_aliases_by_first_token`, `sentence_type_aho_corasick`.
- Offline `cargo run --bin build_indices` from facts.json.
- Retrieval rewritten to consult indices only.
- **Success**: retrieval lookup time drops to O(1) / O(log n);
  baseline benchmark improves.
- **Anti-success**: any retrieval result diverges from v6.1.50.

### Stage 5 — Sense disambiguation (~3 days)

- Add `SenseKey` field to `Fact`.
- Curate sense keys for the audit-flagged set (Ай, Күн, Ағаш,
  заң, бағдарлама, еңбек, тоқсан) — ~30 facts total.
- Domain-aware filter in EvidenceLayer.
- **Success**: 0 wrong-sense on the audit-flagged battery.
- **Anti-success**: any cross-sense regression.

### Stage 6 — Learned components (~5-7 days)

ONLY in this stage do neural components enter:

- `crates/adam-neural-parser/`: small classifier (~ 1 MB)
  trained on `text → QueryIR` paired data. Closed-set output.
- `crates/adam-neural-ranker/`: evidence ranker, learned from
  human-rated relevance pairs.
- Sense disambiguator: closed-set classifier `(surface, context)
  → SenseKey`.

All three are typed, all three are gated by ProofLayer.

- **Success**: HumanDialogEval relevance ≥ 90 %,
  naturalness ≥ 85 %.
- **Anti-success**: any learned component found to be the SOURCE
  of an unsupported claim.

### Stage 7 — Natural realiser (~3 days)

- AnswerIR-driven discourse composer (replaces ad-hoc concat).
- Multi-claim joining with discourse markers («әрі қарай», «және
  бір ерекше дерек»), no tautology, no repetitive honorific.
- **Success**: HumanDialogEval naturalness ≥ 85 %.
- **Anti-success**: any factual hallucination in realiser output.

### Stage 8 — HumanDialogEval v1 + decision gate (~2 days)

- 100-prompt frozen battery (50 single-turn + 50 multi-turn).
- 7-axis scoring: relevance / directness / completeness /
  naturalness / continuity / safety appropriateness /
  traceability.
- CI runner (manual rater this release; v6.3 may automate).
- Default-on decision: flip `ADAM_ANSWER_IR=1` as default
  ONLY when all predeclared success criteria are met.

### Stage 9 — ARM PoC (~3 days)

- `cargo build --target aarch64-apple-ios` (or Android equiv).
- Quantise lexicon / facts.json / indices to fit within 100 MB.
- Profile on real device.
- **Success**: RSS ≤ 200 MB, p99 ≤ 50 ms on Apple A17 /
  Snapdragon 8 Gen 1.
- **Anti-success**: any of those budgets exceeded.

## Open research questions

These need to be answered before specific stages start. The
v6.2.0 owner should resolve them as part of the design-doc
sign-off process.

1. **Operator-layer learning vs. rule-based.** Stage 1
   implements the algebra deterministically. Should Stage 6
   later replace any operator with a learned classifier? My
   bet: no — operators are FST-enforced, learning adds risk
   without benefit. The user / codex should weigh in.
2. **Neural parser size.** Codex suggests «small encoder
   ≈ 1 MB». Concrete target: 0.5-2 MB Rust hash-feature linear
   classifier (no transformer), or 5-10 MB perceptron with
   morphological priors. To be decided in Stage 6.
3. **Sense-key inventory.** ~30 facts in v6.2.0 Stage 5 cover
   the audit-flagged set. Long-term: every world_core fact
   should have a sense-key. Open: how to bootstrap the full
   inventory without manual labelling.
4. **Mobile platform.** Apple A17 (iOS) or Snapdragon 8 Gen 1
   (Android)? Codex / user has stated preference. iOS is
   simpler for adam's Rust+say(1) stack; Android needs
   espeak-ng + JNI work.
5. **Watch deployment** is post-v6.2.0. Apple Watch S9 / Series
   10 has 1 GB RAM, 64 GB storage, M-class CPU. Budget for
   v6.3+: ≤ 30 MB RSS, ≤ 10 MB artifacts.

## Cross-references

- [`docs/v6_1_answer_ir_design.md`](v6_1_answer_ir_design.md) — v6.1
  design doc; same predeclared-discipline pattern.
- [`docs/architecture_neural_v6.md`](architecture_neural_v6.md) — v6.0
  base architecture; L5.5 neural composer (now generalised).
- [`docs/third_path_results.md`](third_path_results.md) — E1/E2/E3
  research-arc discipline (predeclared success + honest fail).
- [`RESEARCH.md`](../RESEARCH.md) §«v6.2 arc» — strategic framing.
- Codex 2026-05-24 architectural analysis — captured in
  `project_v6_2_neurosymbolic_arc` memory directive.

## Scope discipline

- **No code until this doc is signed off.**
- Each stage has a separate commit on
  `experimental/v6_2_agglutinative_algebra`.
- Each stage's anti-success can fire independently — partial
  rollback within the arc is allowed.
- Final merge to `main` only when ALL predeclared success
  criteria are met. Otherwise the branch closes as honest fail
  (precedent: E3 retrieval ranker).
- v6.1.50 production opt-in stays available even after v6.2.0
  merges — the env flag stays for one release as transition
  insurance.

## What this is NOT

- **Not** an LLM. No free text generation. No transformer
  layers. No GPU.
- **Not** a rebuild of v6.1.50. The deterministic kernel stays;
  neural components plug in at typed pressure points.
- **Not** a guarantee of LLM-breadth coverage. v6.2.0 ships
  smart understanding on the curated domain; world-knowledge
  breadth is a separate v6.3+ data-curation problem.
- **Not** a one-week sprint. Combined stage estimate is ~25-30
  days; the design-doc sign-off itself may take a week of
  thinking.

## The bet, stated plainly

If this design works, adam becomes the first applied
demonstration of an AI that is:

- **Demonstrably smarter** per byte than LLM on its target
  language (Kazakh).
- **Architecturally hallucination-free** (proof-carrying).
- **Single-binary, CPU-only**, sub-200 MB RAM.
- **Mobile-runnable** today, **watch-runnable** within v6.3.

If it doesn't work — we ship v6.1.50 as the stable opt-in and
document the negative result with the same discipline as E3.
Either way, the **process** is preserved: predeclared criteria,
honest measurement, no marketing.
