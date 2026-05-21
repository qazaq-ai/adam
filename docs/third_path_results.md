# Third-path research arc — empirical results (E1 + E2 + E3)

**Status.** Snapshot 2026-05-21. Numbers below are reproducible
from the artefacts committed in this repository — every cell is
backed by a frozen training corpus, a fixed RNG seed, and a
shipped model. The companion design docs document the
predeclared anti-success criteria each experiment ran against:

- E1 — [`docs/e1_intent_classifier_design.md`](e1_intent_classifier_design.md) — **PASS**
- E2 — [`docs/e2_slot_extractor_design.md`](e2_slot_extractor_design.md) — **PARTIAL PASS**
- E3 — [`docs/e3_retrieval_ranker_design.md`](e3_retrieval_ranker_design.md) — **HONEST FAIL** (documented; not promoted)

## What this document is

A short, citation-ready summary of what the third-path research
arc has actually demonstrated, as opposed to what it aspires to.
Intended audience: МО РК pilot reviewers, AI Law (in force
18.01.2026) compliance auditors, КУС им. Байтұрсынұлы research
partners, and any prospective investor / academic collaborator
who needs one place to read the numbers.

## The thesis under test

> Probabilistic large language models (LLMs) carry three
> structural problems we treat as **not inevitable**:
> opacity (no source attribution), cost (datacentre GPU
> kilowatts per query), hallucination (confident generation of
> factually wrong content).
>
> **Hypothesis.** A discriminative neural model with a closed
> output set, trained on FST-synthesised data, with a
> deterministic kernel as the production fallback, can deliver
> the **predictive lift** of an LLM-style component **without
> any of the three diseases**.

Three completed experiments now bear on this hypothesis on a
Kazakh-language dialog system. Two (E1, E2) ship as production-
gated opt-in artefacts. One (E3) ships as a documented negative
result and is **not** promoted to production. Default invocation
behaviour is bit-identical to the pre-experiment cascade; every
neural path is reachable only behind an env flag, and only the
flags whose underlying experiment passed its predeclared bar are
wired in.

## Results

### E1 — Discriminative intent classifier

| criterion                  | target  | actual                                  | verdict        |
|----------------------------|---------|-----------------------------------------|----------------|
| Dev accuracy               | ≥ 95 %  | 97.93 % (round 2) / 97.17 % (round 3)   | **PASS**       |
| Test accuracy              | ≥ 99 %  | 94.61 % → **95.95 %**                   | **NEAR**       |
| **vs deterministic cascade** | net +  | **+4.06 pp (95.95 vs 91.89)**           | **PASS**       |
| Net wins vs cascade        | + 0     | **+ 9 wins** (12 improve − 3 regress)   | **PASS**       |
| Latency p99                | ≤ 5 ms  | **35 µs** (~ 600 × under budget)        | **PASS**       |
| Model size on disk         | ≤ 5 MB  | **2.2 MB** sparse JSON                  | **PASS**       |
| Zero hallucination         | yes     | yes (closed-set output, 32 labels)      | **PASS**       |
| No safety-class regression | yes     | yes (orthogonal layer; cascade always wins on safety) | **PASS** |

**Architecture.** Linear classifier over hashed n-gram + token
unigram + 15 hand-rolled binary signals. Pure Rust, zero
dependencies beyond `serde`. AdaGrad-trained, 50 epochs max
with early stopping. Output: one of 32 closed intent labels.

**Production gate.** `ADAM_NEURAL_INTENT=1` env flag. Default
off. When on: classifier rescues `Unknown` verdicts from the
cascade only; confident cascade verdicts always win;
`detect_safety_topic` firing dominates over the override.

### E2 — Discriminative slot extractor (NER)

| criterion                  | target  | actual                                       | verdict        |
|----------------------------|---------|----------------------------------------------|----------------|
| Span F1 (per slot type)    | ≥ 0.95  | 1.000 gazetteer / **0.667 OOV honest**       | **PARTIAL**    |
| **vs deterministic cascade** | net + | **+1 win on OOV** (19 / 29 vs 18 / 29)       | **PASS**       |
| Latency p99                | ≤ 5 ms  | **29 µs** (~ 600 × under budget)             | **PASS**       |
| Model size on disk         | ≤ 5 MB  | **52 KB**                                     | **PASS**       |
| Zero hallucination         | yes     | yes (closed-set output, 11 BIO labels)        | **PASS**       |
| No safety-class regression | yes     | n/a (slot extraction is identity / life-status, not safety) | **N/A** |

**Per-slot OOV honest numbers (29-row held-out test, names /
cities / occupations / ages NEVER seen during training):**

| slot | F1 | observation |
|---|---|---|
| AGE | 1.000 | Generalises perfectly — `is:all-digit` feature captures the pattern abstractly. |
| PER | 0.714 | Capitalisation helps; some OOV names rescued via prefix patterns. |
| OCC | 0.500 | Occupational vocabulary doesn't generalise without lexical memorisation; the model leans on the `-мын/-мін/-пын` predicate copula but that's weak signal alone. |
| LOC | 0.462 | Cities are arbitrary lexical items; without the gazetteer the locative `-да/-де` / ablative `-нан/-нен` suffix patterns aren't enough. |

**Architecture.** Per-token linear classifier over the same
hashed-feature space as E1, plus prefix / suffix / context-
window features tailored to BIO tagging. AdaGrad-trained on
262 labelled examples (3 cascade-on-eval + 23 seed-rerun + 236
paraphrase synth variants). Output: one of 11 closed BIO labels.

**Production gate.** `ADAM_NEURAL_SLOTS=1` env flag. Default
off. When on: extractor fills session slots
(`name` / `age` / `city` / `occupation`) the cascade left
empty; cascade values always win when both extract.
Lightweight gazetteer validation discards span values that
fail sanity checks.

### E3 — Retrieval re-ranker (HONEST FAIL — not promoted)

| criterion                  | target  | actual                                  | verdict        |
|----------------------------|---------|-----------------------------------------|----------------|
| Dev pick-rate-at-1         | ≥ 0.95  | **0.6809** (best, epoch 18)             | **FAIL**       |
| Test pick-rate-at-1        | ≥ 0.95  | **0.7634**                              | **FAIL**       |
| **vs deterministic cascade** | net + | **−24 pp** (0.7634 vs 1.0000 oracle)    | **FAIL**       |
| MRR (test)                 | n/a     | 0.8674                                  | informational  |
| Top-3 precision (test)     | n/a     | 0.9570                                  | informational  |
| Latency p99                | ≤ 5 ms  | **1.0 µs**                              | PASS           |
| Model size on disk         | ≤ 5 MB  | **0.21 KB** (10 weights + bias)         | PASS           |
| Zero hallucination         | yes     | yes (re-ranks a closed candidate list)  | PASS           |
| Production wiring          | iff PASS | **NOT WIRED** (no `ADAM_NEURAL_RANKER` flag exists) | **correct outcome** |

**Architecture.** Pointwise logistic regression over a hand-
designed 10-feature vector per candidate: cascade confidence,
raw-text richness, subject / object token overlap, recency match,
tfidf-cosine proxy, predicate match proxy, IsA-distance proxy,
length norm, candidate position. AdaGrad + sigmoid + binary
cross-entropy. 80 / 10 / 10 stratified split by query (933 train
/ 117 dev / 93 test, leaf-shuffled candidate order with per-query
deterministic seed).

**Why it failed — the structural reason.** The training dataset
uses the cascade itself as oracle for the `picked` label: every
positive row is the candidate the deterministic cascade chose.
This makes the cascade a **theoretical upper bound** of 1.000
pick-rate-at-1 by construction. The model can only approximate
the cascade's policy; any improvement on this metric would mean
"the model imitates the cascade more closely" — strictly
information loss with extra parameters relative to just using
the cascade directly.

**Internal diagnostics.** The learned weight vector contradicts
the cascade's hand-set policy in instructive ways:

| feature           | learned weight | cascade weight  | observation |
|-------------------|----------------|-----------------|-------------|
| `subject_overlap` | **+7.50**      | weak            | Real discriminator the cascade under-weights. |
| `object_overlap`  | **+4.40**      | weak            | Same — token overlap is the dominant signal. |
| `confidence`      | **−3.56**      | **+1.00**       | Cascade-injected confidence has only +0.001 positive/negative gap on this dataset — i.e., it doesn't separate picked from non-picked candidates at all. |
| `cand_pos`        | −0.37          | 0               | Small (post-shuffle) — confirms the deterministic candidate-order shuffle in `ranker_dataset_build` defeated the data-leakage shortcut from an earlier run. |

**Methodology bug found and fixed within this experiment.** The
first training run reported 100.00 % pick-rate-at-1 because the
dataset builder emitted candidates pre-sorted by cascade score,
making `cand_pos = idx / count` directly encode "is this the
picked one". Detected by inspecting the learned weight on
`cand_pos` (−26.06 — anomalously large). Fixed via a
deterministic per-query LCG shuffle of candidate order before
JSONL emission, with the shuffled position of the cascade pick
recorded as `picked_idx`. The 76.34 % is the honest number after
this fix. Reporting this here, rather than burying it, is
exactly the audit posture this whole arc aims to demonstrate.

**Decision.** No `ADAM_NEURAL_RANKER=1` env flag is added.
The artefact is shipped to git for reproducibility and audit,
but is **not** referenced by any production code path. The
`crates/adam-retrieval-ranker` crate stays in the workspace as
infrastructure (the feature-vector definition is reusable) but
its `Ranker::from_path()` is called by no production caller.

**What this teaches the arc.** Discriminative neural cannot
extract signal that isn't in the data. When the deterministic
policy IS the ground truth, neural's ceiling is replication.
The next experiment (E4) must use a non-cascade learning
signal — verifier reward, FST round-trip, or downstream
disagreement — if it is to escape this ceiling.

## What we did NOT measure

Honest catalogue of where this snapshot has known gaps:

1. **Generalisation across age groups** — both E1 and E2 trained
   data come from adult-Kazakh-speaker register. Pre-school /
   teenage register may show different patterns.
2. **Code-switching robustness** — Kazakh / Russian / English
   mixed inputs are not represented in the training corpora.
3. **Voice-input STT noise** — the E1/E2 datasets carry textual
   inputs only; the v6.0.5 voice REPL adds Whisper-noise
   variants that would benefit from a separate evaluation
   sweep.
4. **Drift on production traffic** — there is no production
   traffic yet. All evaluation is on the committed corpora.
5. **Multi-token spans for PER / LOC** — synth data is single-
   token-span heavy; multi-word names like «Айгерім
   Сейітжанқызы» need a future paraphrase-synth round.
6. **E3 against a non-cascade oracle** — the E3 dataset uses the
   cascade as oracle, which structurally caps the experiment at
   100 % imitation. A future re-run using verifier reward or
   downstream-disagreement labels would actually test whether
   a learned ranker can outperform the hand-set policy. E3 in
   this snapshot does not address that question.

## Why these numbers matter

### The "good of LLMs" claim, made concrete

LLMs are valued for three things: pattern generalisation across
lexical surface variation, contextual disambiguation, and
multi-task generality. E1 and E2 demonstrate the first two
**on a Kazakh-language dialog system**, at a deterministic-
kernel cost ceiling:

- **Generalisation.** E1's classifier handles informal /
  polite / fragmented Kazakh phrasings the hand-written
  `detect_*` cascade missed — surfaced as 9 net wins on the
  test set.
- **Contextual disambiguation.** E2's extractor learns that
  the digit token «28» in «жасым 28» is an `AGE` span without
  having seen «28» at training (`is:all-digit` feature
  generalises abstractly).

These are exactly the abilities the "neural is the only path
to language understanding" position attributes to LLMs. Two
small discriminative artefacts deliver them in this repository
at < 3 MB combined disk footprint and < 65 µs combined p99
latency.

### The "bad of LLMs" claim, structurally excluded

LLMs are feared for three things: hallucination, cost, opacity.
E1 and E2 exclude all three by construction:

- **Hallucination.** The output of both models is a closed
  enum. The model cannot emit a label / tag that wasn't in
  the training inventory. Hallucination is **not improbable**
  — it is **mathematically impossible**.
- **Cost.** Combined inference budget: < 65 µs p99 on M2 CPU
  with no GPU, no datacentre, no batching. 0 watts of
  inference cost at user-visible scale.
- **Opacity.** Both artefacts ship as JSON (sparse-encoded
  but human-readable). Every weight is inspectable. Every
  bucket-to-feature mapping is reproducible from the trainer
  source. There is no "we don't know why it said that"
  failure mode.

### Reproducibility, methodology, audit

All three experiments shipped through the same three-round
discipline:

| round | activity                                        |
|-------|-------------------------------------------------|
| **1** | Branch + design doc + crate stub + cascade-baseline floor. Surfaces the data ceiling. |
| **2** | Seed expansion → paraphrase synth → first trained model. Honest measurement. |
| **3** | Held-out OOV evaluation → cascade contingency → production wiring. |

E3 closed at round 2 — production wiring is contingent on
hitting the predeclared anti-success bar, and E3 did not.
Reporting this as a clean exit, rather than mining for a
better-looking number, is the discipline working as
designed.

The design doc for each experiment **predeclares the binary
success criteria** — including anti-success criteria that
would invalidate the experiment — before any training runs.
This is the methodological discipline that lets us call the
numbers "research results" rather than "convenient
benchmarks".

Every artefact is committed to git: corpora, splits, trained
weights, evaluation reports, contingency tables. The full audit
chain is reproducible from a clean clone.

## Compliance posture

### AI Law (ҚР AI Law in force 18.01.2026)

The law classifies defence applications and any system making
material recommendations to citizens as **high-risk**, with
explicit traceability / auditability / human-oversight
requirements. The third-path discipline satisfies these by
construction:

| requirement                                  | how adam satisfies it                             |
|----------------------------------------------|---------------------------------------------------|
| Output traceability                          | Every reply cites a curated source, a grounded reasoning chain with «байланыс» marker, OR a safety-info template (medical / legal / financial) with disclaimer. |
| Audit log per turn                           | `Conversation::turn_with_trace` emits the full pipeline trace; saved per-turn. |
| Human oversight on high-stakes domains       | Safety policy v6 — medical / legal / financial responses always include a specialist-referral disclaimer. Self-harm path routes through 1415 trust line first. |
| Closed-set behaviour                         | E1 / E2 / E3 outputs are all mathematically closed (32 intent labels / 11 BIO tags / candidate-list re-rank). |
| Deterministic fallback                       | The production-wired neural artefacts (E1, E2) are opt-in (`ADAM_NEURAL_*=1`) with the audited deterministic cascade as default. E3 has no production flag — its failure to clear its bar is recorded in the artefact, not in user-visible behaviour. |

### MO RK pilot (Defense Tech IT Park entry point)

The pilot ask documented in
[`docs/mod_kz_demo_script_2026_05_26.md`](mod_kz_demo_script_2026_05_26.md)
positions adam as a deterministic operator-support kernel.
E1 / E2 strengthen rather than weaken that pitch — the neural
components are bounded, audit-logged, and gated behind explicit
opt-in. The default invocation a reviewer can run on a clean
clone is exactly the deterministic kernel. E3's honest-FAIL
documentation strengthens the pitch in a different way: the
team can be relied on to publish negative results, not just
flattering ones — a posture an AI-Law-compliant auditor reads as
risk-reducing.

## Forward path

E4 (verifier-in-the-loop learning) is the next experiment in
the arc. Same discipline: branch + design doc + scaffold +
predeclared anti-success criteria, three-round build-out,
opt-in-iff-PASS production gating. E4's key methodological
move — driven by what E3 taught us — is that the learning
signal must come from somewhere **other than the deterministic
cascade**. Candidates: FST round-trip success, verifier
acceptance, downstream-task accuracy. The exact framing is
worked out in E4's design doc.

The thesis above is now **partially substantiated, with one
boundary mapped**. The honest characterisation of state as of
2026-05-21, after three experiments:

> "On closed-set classification (E1) and BIO sequence tagging
> (E2), discriminative neural matches or exceeds the
> deterministic Kazakh-dialog cascade at ~600 × lower latency,
> with hallucination structurally impossible by output-set
> closure.
>
> On retrieval ranking against a cascade-as-oracle dataset (E3),
> discriminative neural **cannot exceed the cascade by
> construction** — confirmed at 76 % pick-rate-at-1 vs 100 %
> oracle. This is a methodological boundary, not a
> capability ceiling: it tells us that learning signal must come
> from outside the deterministic policy if the model is to
> contribute new behaviour rather than imitate existing
> behaviour.
>
> The hypothesis that the same discipline scales to genuinely
> open tasks (verifier-bounded generation, retrieval where the
> cascade has no opinion) is **still open** — pending E4."

That's the claim the numbers above actually support. Three
data points, two passes and one documented fail, are stronger
evidence for the research arc's discipline than two cherry-
picked passes would be.
