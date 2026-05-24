# Research arc

This document is the detailed research roadmap for **Qazaq IR / ARK**
(Agglutinative Reasoning Kernel) — the deterministic AI kernel built
on agglutinative-language morphology, and (since 2026-05-22) its
algebra-anchored neural extension wired into `main` as the v6.0.0
release.

The high-level mission lives in [`MISSION.md`](MISSION.md). The
"why we are perpendicular to LLMs" position lives in
[`docs/MANIFESTO.md`](docs/MANIFESTO.md). PoC empirical results are in
[`docs/research/results_real_mix_2026_05_16.md`](docs/research/results_real_mix_2026_05_16.md).
The production architecture spec for the v6.0.0 release is in
[`docs/architecture_neural_v6.md`](docs/architecture_neural_v6.md).
The three-experiment third-path arc (E1 / E2 / E3) is summarised in
[`docs/third_path_results.md`](docs/third_path_results.md).

This document zooms into open research questions, methodology, the
completed research arcs, and the upcoming milestones at the layer
below those documents.

## Open research questions

### Q1. Can a deterministic kernel match LLMs on conversational coherence?

**Status.** Partially answered. adam (v6.0.0) demonstrates multi-turn
dialog with belief-state tracking, contradiction recovery, anaphora
resolution, predicate-keyword routing for KRU-domain answer
relevance, and curriculum-aware self-recall — all without
neural-style generation. The remaining gap is **scope**: adam covers
41 intent variants and a curated `world_core` of 3 461 entries / 4 114
facts across 66 domains, far narrower than what an LLM is trained on.

**Open sub-questions:**
- How does the curated-graph approach scale as the fact base grows
  10× / 100× / 1000×?
- Does coherence degrade gracefully when topics fall outside curated
  domains? (Current honest-fallback templates suggest yes, but at what
  point does refusal-rate become user-hostile?)
- Can the morpheme-decomposition core handle code-switching (Kazakh +
  Latin tech tokens, Kazakh + Russian)?

### Q2. Where is the boundary between «kernel» and «trainable component»?

**Status (updated 2026-05-22).** Working hypothesis confirmed at PoC
scale and extended: **tiny ML lives inside the algebraic envelope;
large ML doesn't.** The v6.0.0 release ships this hypothesis as a
working production artefact:

- **E1 (intent classifier)** — discriminative model rescues
  `Intent::Unknown` verdicts behind `ADAM_NEURAL_INTENT=1`. 95.95 %
  test accuracy, +9 net wins vs the cascade on a free-dialog test
  set, 35 µs p99 latency, 2.2 MB on disk. See
  [`docs/e1_intent_classifier_design.md`](docs/e1_intent_classifier_design.md).
- **E2 (slot extractor)** — BIO sequence-tagger fills personal slots
  behind `ADAM_NEURAL_SLOTS=1`. 11 closed labels, +1 win on the OOV
  holdout. PARTIAL PASS (deferred fixes catalogued in
  [`docs/e2_slot_extractor_design.md`](docs/e2_slot_extractor_design.md)).
- **E3 (retrieval ranker)** — pointwise logistic regression.
  Documented HONEST FAIL (not promoted to production); see
  [`docs/e3_retrieval_ranker_design.md`](docs/e3_retrieval_ranker_design.md).

All shipping neural artefacts run on CPU, are bounded outputs (no
generative free-form text), and have inspectable JSON weight files.
Default behaviour without the opt-in flags is bit-identical to the
deterministic cascade. The v6.0 architecture spec
([`architecture_neural_v6.md`](docs/architecture_neural_v6.md))
formalises this as L5.5 in the pipeline.

**Open sub-questions:**
- What is the largest trainable component compatible with the
  kernel-determinism guarantee? E.g. could a small (~10 MB) neural
  classifier sit *inside* the kernel as a confidence-band oracle
  without breaking auditability?
- Can we **formally verify** the determinism property end-to-end?
  Candidate framework: TLA+ specifications of the dialog FSM with
  invariants stating «for any (input, seed, facts), output is uniquely
  determined».
- What happens if we replace the template-ranking perceptron with a
  rule-based scoring function? (Would simplify the kernel but possibly
  hurt naturalness.)

### Q3. How does the architecture generalise across agglutinative languages?

**Status.** Theoretical claim made, not yet validated. The
30-language catalogue in [`MISSION.md`](MISSION.md#agglutinative-languages)
identifies candidates, but adam currently only exists for Kazakh.

**Open sub-questions:**
- What is the actual porting cost from Kazakh to Karakalpak (≈ closest
  language)? Estimate: ~1-2 weeks of Lexicon adaptation + per-suffix
  phonology rule deltas. To be measured.
- Where do the differences live? E.g. Hungarian has 18 cases vs
  Kazakh's 7 — does the FST architecture handle that purely through
  data, or does it need code changes?
- What about non-Eurasian agglutinative languages (Tamil, Quechua,
  Swahili)? Phonology assumptions baked into the Kazakh G2P module
  may not transfer cleanly.

### Q4. What is the cost-quality frontier for kernel-pure voice?

**Status.** v5.0.0 ships TTS via OS-bundled voices (`Aru` on macOS).
v5.1.0 adds optional Piper neural backend. v5.2.0 ships the Kazakh
G2P module as substrate for kernel-pure concatenative TTS.

**Open sub-questions:**
- How does naturalness-perception scale with phoneme-bank quality?
  (One native speaker recording each of 33 phonemes vs studio-quality
  multi-speaker corpus.)
- Is concatenative kernel-pure TTS preferable to neural Piper for
  educational deployment? Trade-off: determinism / offline / no model
  vs naturalness.

### Q5. What architectural extensions does the curriculum surface need?

**Status.** v4.98.0 → v4.99.5 shipped a 5-stage Rust curriculum tree
with adaptive difficulty + auto-advance + student-side query intents.
Closed the Codex round-2 audit of educational use case.

**Open sub-questions:**
- Beyond Rust: can the same curriculum tree generalise to other
  programming languages? Mathematics? Natural science?
- What's the right granularity for «adaptive difficulty»? Currently
  binary (Easy / Hard) thresholded on pass/fail counts; could become
  multi-dimensional (concept × variation × prior knowledge).
- How does the cargo-check verifier loop generalise to non-Rust
  languages with different toolchain expectations?

## Completed research arc — Agglutinative-Neural (2026-05-15 → 2026-05-22)

The `experimental/agglutinative-neural` branch ran for one week as a
parallel research arc to test whether neural training, weights,
tokens, and generation could be used **inside** the deterministic
envelope without inheriting the three structural LLM problems
(opacity, cloud cost, hallucination). It merged to `main` as the
v6.0.0 release.

### Thesis

The LLM industry uses neural networks **one way**: scale everything,
train on the internet, push parameters to hundreds of billions, accept
opacity and hallucination as the price. The hypothesis of this arc:
**the same tools can be used differently.** Small corpus → clean data
→ agglutinative structure → discrete weights → CPU inference →
deterministic verifier on top. The result is a model that is
**smarter by virtue of compactness and structure**, not by mass.

This is not a retreat from the deterministic kernel. It is its
extension with a mathematically-bounded neural layer that:

- Cannot generate facts absent from the curated corpus (verifier
  blocks).
- Cannot produce morphologically invalid Kazakh (FST blocks).
- Cannot drift into fantasy beyond N tokens without verification
  (token budget per reply).
- Can improve **how** something is said — word choice, ordering,
  composition — i.e. the surface layer currently driven by templates.

### What's neural, what stays deterministic

| Component | Implementation |
|---|---|
| Intent recognition | deterministic cascade + opt-in neural rescue (E1) |
| Topic extraction | deterministic (FST + multiword_entities) |
| Fact lookup | deterministic (retrieval over world_core + curated probe) |
| Reasoning chain | deterministic (R1-R10 forward chaining) |
| Slot fill | deterministic cascade + opt-in neural BIO extractor (E2) |
| Retrieval ranking | deterministic `default_v0` (E3 trained but NOT promoted) |
| Morphological synthesis | deterministic (FST) |
| Verifier | deterministic (`proof_object`, no claim ships without support) |
| Realiser | deterministic (template leak guard, quality checks) |

**Principle.** The neural layer may **choose how to say something**,
but **may not invent what to say**. Every factual claim passes
through `proof_object` and `verifier::is_supported`.

### Hard budgets (preserved)

| Resource | Limit | Status |
|---|---|---|
| Model parameters | ≤ 10 M (PoC) → ≤ 100 M (if PoC passed) | E1 = 2.2 MB sparse JSON, E2 = 52 KB, E3 = 0.21 KB — far below cap |
| RSS at inference | ≤ 1 GB | confirmed |
| Latency p50 on CPU | ≤ 200 ms | confirmed (E1 35 µs / E2 29 µs p99) |
| CPU inference must work | M2 8 GB — primary target | confirmed |
| GPU inference | acceptable as speedup, not as dependency | preserved |
| Cloud in any form | **forbidden** at every phase | preserved |

### Three sub-experiments

The arc decomposed into three discriminative-neural experiments, each
predeclaring its anti-success criteria before training. Citation-
ready summary in
[`docs/third_path_results.md`](docs/third_path_results.md).

| # | Experiment | Result | Production wiring |
|---|---|---|---|
| **E1** | Intent classifier (32 closed labels) | **PASS** — 95.95 % test acc, +9 net wins | `ADAM_NEURAL_INTENT=1` (opt-in) |
| **E2** | Slot extractor (11 BIO labels) | **PARTIAL PASS** — 0.667 OOV F1 | `ADAM_NEURAL_SLOTS=1` (opt-in) |
| **E3** | Retrieval ranker (pointwise LR) | **HONEST FAIL** — 0.7634 pick-rate-at-1 vs cascade 1.000 oracle | **NOT wired** (documented; the cascade-as-oracle setup capped the upper bound at imitation) |

### What did NOT happen on this branch

- ❌ No `main` was disturbed — the branch ran in parallel and the
  merge to `main` only flipped the default-off opt-in flags.
- ❌ No cloud training. Everything ran on M2 8 GB CPU + small Rust
  helpers.
- ❌ No pretrained third-party weights. Every artefact trained from
  scratch on FST-synthesised data + cascade-labelled corpus.
- ❌ No rebranding of deterministic as «retrograde». Deterministic
  cascade remains the production-default route.
- ❌ No claim of «we are now an LLM replacement». The arc explicitly
  re-thinks what the neural tools can do **within** the deterministic
  envelope; it does NOT claim to replace LLM-breadth coverage.

### What this arc deferred

The v6.0.0 merge documents the following items as known scope for the
next research arc (v6.1.0+):

- **Predicate-aware retrieval** — extend the `Predicate` enum with
  `BornIn / EffectiveFrom / Classifies / RiskLevel / FoundedIn`, and
  add a question-shape → predicate planner so multi-predicate-per-
  subject queries («X қашан туылған?», «X қандай санаттарға
  жіктейді?») route to the right fact rather than the first IsA match.
- **Morphology-aware slot extractor v2** — consume `Vec<Analysis>`
  (POS / case / particle / predicate-copula) instead of `Vec<String>`,
  so the gates the v6.0.6+ audit added as denylist patches become
  structural rather than band-aid.
- **P8 explicit-revert UX** — let the user supersede a profile slot
  silently when intent is to update rather than to disambiguate.
- **3-token patronymic NEG correction** + multi-token occupation
  segmentation (`дата инженер`) + multi-step math chaining.

## v6.1 arc — AnswerIR + voice REPL audit (2026-05-23 → 2026-05-24)

Status: **shipped** as 11 releases (v6.1.0 → v6.1.50). Behind
`ADAM_ANSWER_IR=1` opt-in flag; default cascade bit-identical to
v6.0.0.

**Predicate-aware retrieval (Stage 1-3, v6.1.0).** `PredicateFocus`
enum with 12 typed shapes (BornIn / DiedIn / FoundedIn / RenamedIn
/ EffectiveFrom / Classifies / RiskLevel / LocatedIn / Authored /
NamedAfter / MemberOf / Relational + IsA). 11 new typed `Predicate`
variants in `adam_reasoning`. KRU world_core rewritten to use the
typed predicates. Stage 3 typed-focus probe overrides upstream
IsA-preference fallback when a typed fact matches. Decision gate:
`v6_1_predicate_eval_30` 28 / 30 pass — above predeclared
threshold ≥ 25 / 30.

**BroadTopic multi-claim composer (Stage 4, v6.1.0).** «X туралы
айтыңыз» queries surface ≤ 3 facts ranked by predicate tier
(IsA → date facts → categorisation → membership → relational).
`DialogContext.broad_topic_subject` + `broad_topic_seen` persist
per-subject seen state; «ал тағы айт» surfaces unseen claims,
clears on subject switch.

**v6.1.5 → v6.1.50 — 11 user-driven REPL audit closures.** Text
(2026-05-23) + voice (2026-05-23 → 2026-05-24) audits surfaced
and closed: NamedAfter / contrastive-farewell / continuation-
exhausted (v6.1.5); IsA-alias bridge for КРУ + 1sg-self-recall
split-bail (v6.1.10); gender-pitch vocative wiring («Ағай / Апай»)
+ whisper greedy decode + thread auto-detect (v6.1.15); Қысқаша
айтсам length-gating + VAD 1300→1000ms + `--no-fallback` +
child-voice F0 band + multi-token canonical-name aliases (v6.1.20);
«атаңыз/тізімдеңіз» enumerative verbs + «моделісің бі?»
architectural-identity probe (v6.1.25); Muslim greeting protocol
+ affirmation-after-clarification (v6.1.30); honorific opt-in →
v6.1.35 → smart consecutive-turn dedup v6.1.40; whisper
`--audio-ctx 768` + Russian-style city spellings + облыс
mishearings (v6.1.45); time-unit Count / Disagreement answer-
shape + Кәзір/нише/Мүгін voice aliases (v6.1.50).

**v6.1.50 = freeze point.** The patch-by-patch strategy reached
its ceiling. Every new audit surfaces new mis-recognitions, new
wrong-sense retrievals, new semantic mismatches — they don't
compose into a smarter model.

## v6.2 arc — architectural redesign (planned, design doc next)

User signal 2026-05-24 (after 11 voice REPL audits): «с такой
архитектурой мы никогда не сможем все исправить». v6.2.0 is the
architectural pivot: ONE arc addressing five root causes, NOT
five separate features.

**The five root causes** (memory:
[[project_v6_2_architectural_pivot]]):

1. **Context-aware STT correction.** Whisper emits «Мен
   Бағдарламасы» (suffix mismatch with expected «-мын» 1sg
   copula). N-best lattice + phonetic-bounded fuzzy + FST
   round-trip + grammatical-expectation bias.
2. **Answer-shape detection.** «Жылда неше ай?» (= "how many
   months in a year?") routes to topic extraction over «жыл»,
   retrieves «тоқсан» (quarter) fact — semantically adjacent
   but wrong concept. Need explicit Count / TimeWhen / Function
   / Procedure / Disagreement / ListExamples / SenseDisambiguation
   AnswerIR shapes. (v6.1.50 ships a closed-set Count /
   Disagreement table as a patch; v6.2.0 generalises.)
3. **Sense disambiguation.** «Ай» = moon / month / howl;
   «Күн» = sun / day. `SenseKey` per fact + domain filter at
   retrieval.
4. **Structured indexing.** Every detector / lookup scans the
   same data. Hash / trie / Aho-Corasick indexes by POS,
   predicate, sentence-type, domain, sense.
5. **HumanDialogEval gate.** Until 100-150-prompt 7-axis eval
   is the CI gate, releases ship with relevance regressions
   invisible to `factual_eval_100` / `v6_1_predicate_eval_30`.

Design doc: `docs/v6_2_architectural_redesign.md` (to be written
next session). Predeclared structure: thesis → 5 root-cause
problems with audit evidence → unified `QueryContext`
abstraction → indexing infra → STT correction pipeline →
HumanDialogEval v1 → 5 implementation stages with predeclared
success + anti-success per stage → default-on promotion gate
(0 P0 + ≥ 90 % relevance + ≥ 85 % naturalness + 0 wrong-sense)
→ rollback path to v6.1.50.

### Memory-directive lineage

For continuity, the directives that gated this arc:

- [`project_retrieval_not_neural_v2`](memory/project_retrieval_not_neural_v2.md) —
  the arc IS the production realisation of this directive.
- [`project_deterministic_directive_confirmed`](memory/project_deterministic_directive_confirmed.md) —
  reaffirmed for `main`; relaxed for `experimental` arcs as
  «reject LLM scale + opacity + cloud, accept neural tools used
  differently».
- [`project_v4_direction`](memory/project_v4_direction.md) —
  «no LLM-breadth race» preserved: smart-narrow not broad-mediocre.
- [`project_engineering_framing`](memory/project_engineering_framing.md) —
  agglutinative algebra of meaning stays the central concept; this
  arc was its mathematical implementation.

## Methodology

The research is empirical-engineering, not theoretical. We follow
this loop:

1. **Hypothesis** — formulate a claim about what the kernel can or
   cannot do (e.g. «multi-turn anaphora can resolve via DialogContext
   without neural attention»).
2. **Implementation** — build the smallest version of the feature
   that tests the hypothesis.
3. **Live testing** — exercise it in a real REPL session with novel
   Kazakh phrasings (not templated holdouts; per
   `feedback_real_human_testing_with_memory`).
4. **Audit** — submit transcripts to Codex (third-party AI auditor)
   for adversarial review. Fold findings into next iteration.
5. **Regression test** — every audited bug becomes a permanent test
   case so it cannot reoccur silently.
6. **Release** — bundle 1-7 innovations per release, version per
   significance (`feedback_versioning_post_1_0`).

Investor-readiness self-assessment after v6.0.0: pre-seed / angel /
strategic-pilot READY; institutional VC seed pending GA blocker
closure (Lexicon V2 native-speaker review, arXiv submission, alpha
partner deployment).

## Milestones

### Q2 2026 — Demonstrator stability ✅ shipping

- ✅ Multi-turn dialog with belief-state tracking
- ✅ Contradiction recovery + explicit-pick resolution
- ✅ Anaphora resolution with overcarry guard
- ✅ Voice output via OS-bundled TTS
- ✅ Voice input via whisper.cpp + pitch-based gender hint (v6.0.5)
- ✅ Kazakh-only refusal for non-Kazakh inputs
- ✅ Curriculum tree with adaptive difficulty (Rust track)
- ✅ Codex round-3 audit closed (architectural pass)
- ✅ Public repository (2026-05-08), BUSL-1.1 license
- ✅ **Safety policy v6** (2026-05-21) — informational + emergency-
  triage + disclaimer for medical/legal/financial; crisis-line first
  for self-harm
- ✅ **Agglutinative-Neural research arc shipped as v6.0.0**
  (2026-05-22) — E1 PASS, E2 PARTIAL PASS, E3 HONEST FAIL, all
  opt-in behind env flags

### Q3 2026 — Kernel-pure voice + first port

- 🟡 Phoneme bank: hand-record native-speaker WAVs for the 33 Kazakh
  phonemes
- 🟡 `PhonemeBankTtsBackend`: load + splice via existing G2P module
- 🟡 Validate the architecture's portability: prototype Karakalpak
  Lexicon adaptation (closest-language test)
- 🟡 First school pilot in Almaty / Astana (MVP deployment in 2-3
  classrooms)

### Q4 2026 — Multi-language extension

- 🟡 Choose second language for full port: candidates Kyrgyz (Kipchak,
  closest after Karakalpak) or Turkish (largest existing NLP base for
  comparison)
- 🟡 Document the porting cost honestly; identify which architectural
  pieces are language-agnostic vs language-specific
- 🟡 Publish a comparative paper (or technical blog post)

### 2027 — Formal verification

- 🟡 Specify ARK's deterministic guarantees in TLA+ (or similar)
- 🟡 Machine-check key invariants:
  - «For any (input, seed, facts), output is uniquely determined»
  - «No claim is emitted without a backing fact in `world_core` or a
    grounded reasoning chain»
  - «Belief state has at most one Active fact per (subject, predicate)»
- 🟡 Publish verified-kernel artefact

### 2027+ — Vertical applications

Per the parent Qazna Technologies vision (FinTech / DefenseTech /
HealthTech / EdTech), explore non-educational applications of the
deterministic kernel:

- **FinTech** — auditable compliance assistants, regulatory document
  analysis (Kazakh-language)
- **DefenceTech** — offline-capable, deterministic operator-support
  AI for restricted environments
- **HealthTech** — symptom-triage with traceable reasoning chains
  (no hallucination tolerance)
- **EdTech** — adam continues as the educational vertical;
  potentially expand to other subjects beyond Rust

## How this is funded

The research is currently self-funded (founder time + minimal
infrastructure costs — 0 % GPU).

We are pursuing **two parallel funding tracks**:

1. **Angel pre-seed / seed-stage private capital** — to accelerate
   Q3 2026 milestones (phoneme bank recording, first port, school
   pilots). Target: $200K–300K for 12 months.

2. **State research grants and academic joint-research partnerships**
   — every state in the 30-language agglutinative catalogue has a
   direct strategic interest in deterministic AI for its own national
   language. Priority partners: Japan (JST/JSPS), South Korea (NRF),
   Finland (Academy of Finland), Turkey (TÜBİTAK), Uzbekistan,
   Hungary (NKFIH), Estonia (ETAg), Mongolia, Kyrgyzstan, Tatarstan.
   See [`COLLABORATION.md`](COLLABORATION.md#international--agglutinative-language-alignment)
   for engagement terms.

These tracks are complementary, not competing: state grants fund
research milestones (ports to new languages, formal verification,
phoneme-bank recording with native speakers), while private capital
funds applied product / pilot deployment. We pursue both.

See [`COLLABORATION.md`](COLLABORATION.md) for the full collaboration
framework, including investor-engagement terms.

## Publications and external references

- [adam GitHub repository](https://github.com/qazaq-ai/adam) — source
  of truth (BUSL-1.1)
- [`docs/preprint/arxiv_v0_draft.md`](docs/preprint/arxiv_v0_draft.md) —
  arXiv submission v0 draft (algebra-anchored neural composition)
- (Planned) Habr / Medium technical blog post — Q3 2026
- (Planned) Comparative paper on Karakalpak port — Q4 2026
- (Planned) TLA+ verified-kernel artefact — 2027
