# E3 — Discriminative Retrieval Re-ranker

**Status.** Design draft. Branch `experimental/e3-retrieval-ranker`,
forked from `experimental/agglutinative-neural` at commit
`e86d2fd` on 2026-05-21.

**Predecessors.**
- E1 (intent classifier) — closed at 95.95 % test, +9 net wins
  vs cascade, 35 µs p99, opt-in production wiring.
- E2 (slot extractor) — closed at 0.667 OOV F1, +1 net win vs
  cascade, 29 µs p99, opt-in production wiring.

E3 is the third experiment in the same arc, targeting the
**fact-selection step** in the retrieval pipeline. Currently
implemented in [`crates/adam-dialog/src/selection.rs`] as a
linear scorer with **hand-set weights** (`SelectionWeights::default_v0`).
E3 replaces the hand-set weights with **learned** weights and
adds a richer feature set.

## Hypothesis under test

> A pointwise learn-to-rank model trained on
> `(query, fact, picked_by_cascade)` triples can replace the
> hand-set `SelectionWeights` in `adam-dialog::selection` at
> **equal-or-better rank-of-target-fact accuracy**, ≤ 5 ms
> ranking latency on a typical candidate list of 5–20 facts,
> ≤ 5 MB on disk, and **zero hallucination by construction**
> (the model emits scores; the caller's argmax over the
> closed candidate set picks the winning fact — no novel
> output is generated).

If the hypothesis holds, E3 closes the third big lever in the
pipeline. After E1 (intent) + E2 (slots) + E3 (retrieval
ranker), three of the four most-touched cascade surfaces are
running learned models with the same opt-in / cascade-fallback
/ closed-output discipline.

## Why this is the right next experiment

1. **Highest-leverage surface left.** `selection::score` is on
   the hot path of every factual query. Improving its ranking
   directly affects `factual_eval_100`, the hallucination gate
   for GA-level shipping.

2. **Cascade is already the oracle.** The current
   `select_top` returns a fact when there's a candidate match;
   that fact IS the labelled-positive for training. No
   labelling effort beyond running the cascade on a corpus.

3. **Closed output by construction.** Same as E1 and E2 — the
   model emits scores; the argmax over a CLOSED candidate list
   picks a fact. No risk of emitting a fact that wasn't in the
   candidate set.

4. **Trivial fallback.** If the learned weights regress on a
   class (rare facts, low-confidence chains), fall back to the
   hand-set `default_v0`. Worst case = current behaviour.

## Scope

**In scope.**

- A single learned weight vector for the
  `CandidateFeatures` space currently in
  [`crates/adam-dialog/src/selection.rs`], plus 5–10 additional
  features that the current scorer doesn't use (TF-IDF
  similarity between query and fact raw_text; predicate-
  category match; subject IsA-distance from query head; ...).
- A training pipeline producing the learned weights from
  - `(query, candidate_facts[], cascade_picked_fact_idx)`
    triples mined from `data/eval/*.json`,
- An evaluation harness comparing the learned ranker against
  the hand-set `default_v0` on a held-out test set.
- A production-gated integration point so the new ranker is
  reachable behind `ADAM_NEURAL_RANKER=1` while the default
  path stays deterministic.

**Out of scope (for now).**

- Generating novel candidates (the candidate set is fixed by
  the retrieval pre-filter).
- Cross-language ranking — Kazakh corpus only.
- Listwise training objectives (LambdaMART etc.) — pointwise
  logistic regression first, promote only if necessary.

## Architectural ladder

Same discipline as E1 / E2.

### Rung A — Pointwise logistic regression

- Input: the existing `CandidateFeatures` (5 features) plus 5
  additional engineered features (10 total).
- Output: scalar score; calling `select_top` argmaxes over
  the candidate set.
- Parameters: 10–20 weights + bias. Pure linear model.
- Training: binary cross-entropy on
  `(features, picked=1 OR 0)` pairs. AdaGrad.
- Why first: drop-in replacement for the existing
  `SelectionWeights` struct; 100 % readable artefact
  (literally 11 numbers).

### Rung B — Pairwise margin learning

- Input: same features.
- Output: scalar score; the loss prefers
  `score(positive) > score(negative) + margin`.
- Parameters: same 10–20 + bias.
- Why second: standard upgrade when pointwise misses subtle
  preferences over close candidates.

### Rung C — Tiny MLP

- Input: same features + per-fact embedding from FST roots.
- Output: scalar score.
- Parameters: ~ 50 K.
- Why third: only if the linear forms hit a generalisation
  floor. Embedding makes weights less inspectable; we accept
  that only if measurably needed.

## Training data

### Source

- Walk every `data/eval/*.json` and `tests/end_to_end.rs`
  query string.
- For each query, **invoke the production retrieval pipeline**
  once to get the candidate set + the cascade's pick.
  Specifically: run `Tool::SearchRetrieval` followed by the
  current `select_top`, capture both the full candidate list
  and the index of the cascade-picked fact.
- Emit one row per `(query, candidate_fact_idx, features,
  picked={0,1})` triple. Most rows are negative (1 positive
  per query × N candidates).

### Schema

`data/retrieval_ranker/v1/dataset.jsonl`:

```jsonl
{"query_id":"q_00001","query":"Абай туралы айтшы","cand_idx":0,"features":{"confidence":1.0,"richness":0.6,"subject_overlap":1.0,"object_overlap":0.0,"recency_match":0.0,"tfidf_cosine":0.71,"predicate_match":1.0,"isa_distance":0.0,"raw_len":47,"cand_pos":0},"picked":1}
```

### Target size

- Minimum: 500 queries × ~ 10 candidates / query = 5 000 rows.
- Aspirational: 5 000 queries × ~ 15 candidates / query =
  75 000 rows.

## Evaluation

### Primary metric

**Pick-rate-at-1**: fraction of test queries where the model's
argmax matches the cascade-picked fact.

### Secondary metrics

- **MRR (Mean Reciprocal Rank)** of the cascade-picked fact
  in the model's ranked list. Insensitive to ties.
- **Precision at top-3**: fraction of queries where the
  cascade-picked fact is in the model's top-3.
- **Latency p99** per ranking pass.

### Success criteria (binary, all must hit)

1. **Pick-rate-at-1 ≥ 0.95** vs the hand-set baseline (which
   IS the oracle, so the baseline scores 1.000 by
   construction; the criterion is "doesn't lose more than 5 %
   of cascade's picks").
2. **Latency p99 ≤ 5 ms** per ranking pass.
3. **Model size ≤ 5 MB** (in practice ≤ 1 KB given 10 floats).
4. **Zero hallucination.** Output is a score; selection
   argmaxes over a closed candidate set; novel facts are not
   emitted.

## Production wiring

`ADAM_NEURAL_RANKER={1|0}` env flag, default off.

When on, `Tool::SearchRetrieval` uses the learned weights
instead of `SelectionWeights::default_v0` for its
`select_top` call. The candidate set is unchanged; only the
ranking criterion differs.

**Safety override**: when the cascade has high-confidence
specific evidence (e.g. `ConfidenceKind::HumanApproved` on the
top candidate), we keep the cascade pick. The learned ranker
is a tie-breaker for moderate-confidence cases.

## Implementation plan (first sprint)

1. **Branch + design doc** ✓ (this document)
2. **Crate stub** — `crates/adam-retrieval-ranker/` with the
   public `score()` signature, an artefact loader (~ 1 KB
   JSON), error types.
3. **Dataset generator** — `tools/intent_dataset/src/
   ranker_build.rs` walks the corpora, invokes the existing
   retrieval pipeline, emits the `(query, candidates, picked)`
   triples.
4. **Baseline harness** — measures `pick-rate-at-1` of the
   current hand-set weights on the test split. This is the
   floor — learned weights must beat it.
5. **Rung A trainer** — pointwise logistic regression with
   AdaGrad.
6. **Rung A evaluator** — pick-rate-at-1, MRR, top-3
   precision, latency.
7. **Rung A integration** — production hook gated on the env
   flag.

## Open questions deferred to first sprint

- **Feature engineering.** What's the right addition to the
  existing 5? TF-IDF cosine between query and raw_text is the
  obvious first; predicate-match is the second; IsA-distance
  is the third. The fourth onward depends on what the failure
  mode of the linear model looks like on the test split.
- **How to handle near-ties.** Multiple equally-scoring
  candidates → stable tie-break by index (matches the existing
  `select_top` contract).
- **Listwise vs pointwise.** Pointwise is the v0 starting
  point; if Rung A misses by a wide margin on rare-fact
  queries, promotion to pairwise margin is the first response.

## Anti-success: what would invalidate the experiment

1. **Rung A pick-rate-at-1 below 0.90 on dev.** Learning the
   ranking from 10 features alone is structurally insufficient.
2. **Rung B (pairwise) pick-rate-at-1 below 0.95 on test.**
   The retrieval ranking task is too noisy at the granularity
   adam needs.
3. **Net regression on `factual_eval_100`** when the
   production hook is enabled. Hard stop — that benchmark is
   the GA-level hallucination floor.
4. **Latency > 5 ms even on Rung A.** Then the
   implementation has a bug; 10-float dot products take
   nanoseconds.

In any of these cases we write up the negative result, archive
the artefacts, and **return to the hand-set `default_v0`
weights**. The hand-set baseline has been audit-trail-stable
for 30+ commits; preserving that is a feature, not a bug.

## Why E3 might fail differently from E1 / E2

E1 and E2 both replaced **hand-written branching logic** (60+
`detect_*` functions + 5 `detect_statement_of_*` extractors).
The learned model only had to imitate a small closed enum of
behaviours.

E3 replaces a **scoring function** whose hand-set weights are
**already calibrated against a much larger evaluation surface**
than the cascade detectors were. The room for improvement is
narrower:

- E1 found 9 cascade-Unknown cases where the classifier
  produced a confident, correct intent.
- E2 found 4 cascade-empty cases where the extractor produced
  a correct span.
- E3 might find 0–2 cases where the learned ranker picks a
  better fact than the hand-set weights, simply because the
  hand-set weights have been tuned over many commits.

Even a 1–2 net-win improvement at 600× lower latency would
still be a valid demonstration of the third-path discipline.
But this experiment is the one most likely to return a
**negative result** — which we accept up-front and document
honestly.
