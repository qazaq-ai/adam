# E2 — Discriminative Slot Extractor (Named Entity Recogniser)

**Status.** Design draft. Branch `experimental/e2-slot-extractor`,
forked from `experimental/agglutinative-neural` at commit
`9b9e6a7` on 2026-05-21.

**Predecessor.** E1 closed at 95.95 % test accuracy / 35 µs p99
on intent classification with the third-path discipline
(closed-set output, deterministic kernel preserved, opt-in
gating). E2 is the second experiment in the same arc, targeting
the next biggest bundle of hand-written cascade code: **slot
extractors** for `name`, `age`, `city`, `occupation`, `family`,
and friends.

**Hypothesis under test.** A small (~ 100 K – 1 M parameter)
sequence-labelling model trained on FST-synthesised + cascade-
labelled Kazakh utterances can replace the hand-written
`detect_statement_of_{name, age, location, occupation, family}`
functions in `crates/adam-dialog/src/semantics.rs` at
**equal-or-better span-precision**, **comparable or better
latency**, and **zero hallucination by construction** (the
output is a per-token tag from a closed BIO label set).

If the hypothesis holds, E2 closes the second-largest piece of
linear-growth-with-Codex-audit cascade debt and shrinks
`semantics.rs` by ~ 600 lines of pattern-specific extractor code.

## Scope

**In scope.**

- A single neural artefact that, given a Kazakh utterance
  tokenised on whitespace, returns one BIO tag per token from
  a closed inventory:
  - `O` (outside any slot)
  - `B-PER` / `I-PER` (person name span)
  - `B-LOC` / `I-LOC` (location / city span)
  - `B-AGE` / `I-AGE` (age value span, including the
    accompanying noun like «жас» if adjacent)
  - `B-OCC` / `I-OCC` (occupation noun span)
  - `B-FAM` / `I-FAM` (family-relation token span)
- A training pipeline producing the artefact from
  - existing `detect_statement_of_*` functions used as
    labelling oracle (where they fire), AND
  - FST-synthesised paraphrases that inflect the surface form
    through person / number / case while preserving the span
    structure.
- An evaluation harness comparing the model against the current
  hand-written extractors on a held-out test set, scored by
  span-level F1 (precision / recall on exact span matches).
- A production-gated integration point in
  `Conversation::turn_with_trace` so the new extractor is
  reachable behind `ADAM_NEURAL_SLOTS=1` while the default path
  stays deterministic.

**Out of scope (for now).**

- Free-form entity types (events, organisations beyond the
  occupational nouns we already track, abstract concepts).
  Closed inventory above is the v0 surface.
- Cross-sentence span resolution (anaphor resolution stays
  with the existing `DISCOURSE_ANAPHORS` + retrieval logic in
  `discourse.rs`).
- Anything involving free-form text generation. The
  deterministic template renderer keeps producing all surface
  answers.

## Why E2 follows E1

The same architectural argument E1 confirmed applies:

1. **Surface where structural debt is highest.** After the
   `detect_*` cascade, the slot extractors are the next biggest
   linear-growth surface in `semantics.rs`. Each Codex audit
   adds one or two pattern variants per slot type — the file is
   currently ~ 3 700 lines and ~ 600 of those are
   `detect_statement_of_*`.

2. **Closed output space.** Per-token tag from 9 labels. The
   model literally cannot emit anything else.

3. **Cheap evaluation.** The cascade IS the oracle on inputs it
   handles. We label thousands of examples automatically by
   running the existing extractors and recording the spans they
   identified.

4. **Trivial fallback.** If the model's span is empty or
   low-confidence, fall back to the cascade. Worst case = current
   behaviour.

## Architectural ladder

Same discipline as E1: start with the simplest model that
passes the success criteria; promote one rung only if it
doesn't.

### Rung A — Linear-chain CRF over hash features

- Input: hashed (token, prev-tag) features + character-trigram
  features per token + a small set of hand-rolled binary
  signals (token-starts-capital, is-numeric, is-known-city, …).
- Output: BIO tag per token, decoded via Viterbi over a
  9-state transition matrix.
- Parameters: ~ 100 K (feature × tag table + transition matrix).
- Implementation: pure Rust, zero new deps. Forward / backward
  pass is sparse-vector arithmetic. Training: averaged
  perceptron (cheaper than CRF gradient but converges fast on
  this scale).
- Why first: trains in seconds, ships < 1 MB, full
  transparency (you can print why a tag was assigned to a
  token).

### Rung B — Tiny BiLSTM with character embeddings

- Input: token embeddings (16-dim) + char-CNN summary (16-dim)
  per token; concatenate, pass through a 1-layer BiLSTM with
  hidden size 32.
- Output: per-token logits → BIO tag via Viterbi over the
  same transition matrix.
- Parameters: ~ 500 K total.
- Why second: handles morphological variation the CRF can't
  generalise across (e.g. the same root inflected through five
  cases is still a city). Embeddings still readable — top-k
  cosine neighbours of every root.

### Rung C — Char-aware Transformer with span head

- Input: same as Rung B + a 1-head self-attention layer.
- Output: per-token tag.
- Parameters: ~ 1 M total.
- Why third: only if Rung B fails on long-range compositional
  patterns (multi-word names with case markers spread across
  tokens). Latency budget still permits this.

The discipline: **don't promote** unless the previous rung
fails a binary criterion. Each rung adds parameters but also
adds opacity.

## Training data

### Sources

1. **Cascade oracle.** Run the existing
   `detect_statement_of_*` functions against every input in
   `data/eval/*.json` and the in-repo unit-test phrase
   literals. For each input the function fires on, store
   `(input_tokens, span_start, span_end, slot_type)` as a
   labelled training example. Confidence policy: keep only
   spans the cascade's own confidence band scores `High`.

2. **FST-synthesised paraphrases.** For each cascade-labelled
   span:
   - Inflect the head noun through the six standard case
     markers + 1sg/2sg-informal/2sg-polite predicate copulas.
   - Replace the span's lexical content with other
     valid-for-slot tokens (other Kazakh names for `PER`,
     other Kazakh cities for `LOC`, other valid integer ages
     for `AGE`, other lexicon-tagged-noun occupations for
     `OCC`).
   - Permute word order within the SOV / OSV permissible
     family.

3. **Negative examples.** Sample sentences from the same eval
   corpora that contain **no slot-bearing pattern** but
   surface tokens that look similar (e.g. genitive
   pronouns + nouns: «менің кітабым бар» — has «менің» and a
   noun but no slot fires). Label all tokens `O`. Forces the
   model to learn the boundary patterns, not just memorise
   the slot-token vocabulary.

### Schema

`data/slot_extractor/v1/dataset.jsonl`:

```jsonl
{"id": "ds_00001", "tokens": ["менің", "атым", "Дәулет"], "tags": ["O", "O", "B-PER"], "source": "cascade"}
{"id": "ds_00002", "tokens": ["мен", "Алматыда", "тұрамын"], "tags": ["O", "B-LOC", "O"], "source": "cascade"}
{"id": "ds_00003", "tokens": ["маған", "25", "жас"], "tags": ["O", "B-AGE", "I-AGE"], "source": "cascade"}
{"id": "ds_00004", "tokens": ["мен", "бағдарламашымын"], "tags": ["O", "B-OCC"], "source": "cascade"}
{"id": "ds_00005", "tokens": ["менің", "екі", "балам", "бар"], "tags": ["O", "O", "B-FAM", "O"], "source": "cascade"}
```

Split: 80 % train / 10 % dev / 10 % test, stratified by slot
type so every type appears in test.

### Target size

- Minimum: 1 500 labelled sentences across the 5 slot types
  (median 300 per type).
- Aspirational: 10 000 with at least 500 per type.

## Evaluation

### Success criteria (binary, must hit all)

1. **Span-level F1 ≥ 0.95** on the frozen test set, **per
   slot type**. Below this, retry with the next rung or
   abort.
2. **Latency p99 ≤ 5 ms** per inference on M2 CPU (single
   thread). Above this and the extractor is slower than the
   cascade — there is no point.
3. **Model size ≤ 5 MB** on disk.
4. **Zero hallucination.** Output domain is exactly the 9
   closed BIO labels.

### Comparison metric

For every test-set input, run:
- The deterministic cascade (existing extractor functions).
- The neural extractor.

Report per slot type a 2×2 contingency on exact-span match:
- Both extract the same span → **double win**.
- Cascade right + neural wrong → **neural regression** (count).
- Cascade wrong + neural right → **neural improvement** (count).
- Both wrong → **shared blind spot**.

Promotion criterion: **net positive after subtracting
regressions** AND **no regression on safety-relevant slot
extractions** (none currently exist — slots are purely
identity / life-status, not safety-class).

## Production wiring

### Gate

`ADAM_NEURAL_SLOTS={1|0}` environment variable, default off.
When on:
1. The extractor runs first on the parsed tokens.
2. For each detected span, validate against the lexicon:
   - `PER` spans → check the token matches the personal-name
     gazetteer or has the right capitalisation pattern.
   - `LOC` spans → check the token lemma is in
     `weather::kazakh_city_coords` or the geo entity catalog.
   - `AGE` spans → check the span contains an integer 0..120.
   - `OCC` spans → check the token has `part_of_speech =
     "noun"` and is in the occupations gazetteer
     (`data/lexicon_v1/apertium_imported_roots.json`).
3. Invalid spans → discard (do not commit to session).
4. Spans that pass validation → write to session slots
   (`session.insert(slot_name, span_text)`).
5. If the cascade also produced a span for the same slot and
   it differs from the neural span → trust the cascade (safety
   conservatism on session state).

### Logging

Every disagreement between cascade and extractor is logged at
`data/slot_extractor/disagreements.jsonl`. The next iteration's
labelled-improvement queue.

### Rollback

`ADAM_NEURAL_SLOTS=0` (default) entirely bypasses the new code
path. No risk of regression in the default invocation.

## Implementation plan (first sprint)

1. **Branch + design doc** ✓ (this document)
2. **Crate stub** — `crates/adam-slot-extractor/` with the
   public `extract()` signature, empty implementation, types
   for the loaded model artefact, error types.
3. **Dataset generator** — extends `tools/intent_dataset/` to
   emit BIO-tagged sentences whenever any
   `detect_statement_of_*` cascade function fires. Writes to
   `data/slot_extractor/v1/dataset.jsonl`.
4. **Baseline harness** — measures the cascade's own
   per-slot-type span-F1 on the test split. We need this
   baseline before claiming anything about the extractor.
5. **Rung A trainer** — averaged-perceptron over BIO tags.
6. **Rung A evaluator** — runs the trained model against the
   frozen test set, emits the 2×2 contingency + per-class
   F1.
7. **Rung A integration** — `Conversation::turn` gated branch.
8. **Stop-or-go decision** — meet success criteria on Rung A?
   Ship behind `ADAM_NEURAL_SLOTS=1`; otherwise promote to
   Rung B.

## Open questions deferred to first sprint

- **What's the right tokenisation unit?** Whitespace-split is
  the simplest and matches the cascade. But Kazakh
  agglutination means many "tokens" carry multiple morphemes;
  some slot patterns straddle the morpheme boundary
  («Алматыдамын» = `Алматы-Loc-1sg.Pred`, all one token).
  Open question: does the model need morpheme-aware
  tokenisation to handle 1sg.Pred-suffixed locations?
  Probably yes for Rung B; Rung A whitespace is the v0 floor.
- **How to handle ambiguous spans?** «Атым Айдар» — is the
  span just `Айдар` or the whole `Атым Айдар`? The cascade
  picks `Айдар`. The training data should reflect that
  consistently.
- **Multi-word names** («Айгерім Сейітжанқызы»). Cascade
  handles these unevenly; the model should learn to merge
  adjacent `B-PER` + `I-PER` consistently.

## Anti-success: what would invalidate the experiment

1. **Rung A F1 below 0.90 on dev**. Hash-feature CRF is
   insufficient for Kazakh agglutination on this scale.
2. **Rung B F1 below 0.95 on test**. The slot space is
   structurally hard for discriminative ML. (Unlikely;
   well-established results for similar tasks in low-resource
   Turkic languages.)
3. **Net regression on session-write slots after promotion**
   compared to the cascade. The cascade's session state has
   been audit-trail-stable for 50+ commits; any regression
   that loses session continuity (e.g. classifier mistakenly
   commits `name = "Алматы"` because LOC ⊂ PER token-form-wise)
   is a hard stop.
4. **Latency > 5 ms even on Rung A**. Then the implementation
   has a bug; the architecture is well within budget.

Same discipline as E1: predeclared, binary, written down
before the experiment runs.
