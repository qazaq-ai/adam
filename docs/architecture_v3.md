# adam v3.0 — architecture reference

This is the **single canonical description** of the v3.0 system. It freezes the architectural commitment assembled over the v2.5 → v2.9 reasoning ladder on top of the v2.0 retrieval foundation.

> **v3.0 is not a trained neural model.** It is a deterministic **retrieval + composition + reasoning** engine over a curated Kazakh corpus. See [`project_retrieval_not_neural_v2`](roadmap.md#post-v10-direction). The v3.0 addition to v2.0's commitment: **adam now concludes, not just retrieves.** Every derivation is produced by an explicit rule and carries full source-chain provenance.
>
> The v2.0 architecture reference at [`architecture_v2.md`](architecture_v2.md) remains valid as a snapshot of the v2.0–v2.3 retrieval era; this file supersedes it for v3.0.

---

## The pipeline

```
                  ┌─────────────────────────────────────────────────┐
  user input ──▶  │  L1  parser          (adam-kernel-fst)          │
                  │       FST analyse → list<Analysis>              │
                  └─────────────────────────────────────────────────┘
                                         │
                                         ▼
                  ┌─────────────────────────────────────────────────┐
                  │  L2  semantics       (adam-dialog)              │
                  │       interpret_text_with_lexicon → Intent      │
                  │       26 recognisers + first_noun_root fallback │
                  └─────────────────────────────────────────────────┘
                                         │
                                         ▼
                  ┌─────────────────────────────────────────────────┐
                  │  L2.25  follow-up    (v1.4.0)                   │
                  │       resolve_follow_up — "ал сіз?" retagging   │
                  └─────────────────────────────────────────────────┘
                                         │
                                         ▼
                  ┌─────────────────────────────────────────────────┐
                  │  L2.5  retrieval inject (Intent::Unknown only)  │
                  │       v1.6.5: rank→top-1 sample text            │
                  │       v1.9.0: optional compose_with_city         │
                  │       v1.9.5: example_adapted flag               │
                  └─────────────────────────────────────────────────┘
                                         │
                                         ▼
                  ┌─────────────────────────────────────────────────┐
                  │  L2.75  reasoning inject (v2.7 new)             │
                  │       scan derived_facts for noun_hint match    │
                  │       render_derivation_as_kazakh → chain text  │
                  │       always contains «байланыс-» trust marker  │
                  └─────────────────────────────────────────────────┘
                                         │
                                         ▼
                  ┌─────────────────────────────────────────────────┐
                  │  L3  planner         (adam-dialog)              │
                  │       reasoning_chain?  →  with_derived_chain   │
                  │       example+adapted?  →  with_adapted_evidence│
                  │       example?          →  with_evidence        │
                  │       noun_hint?        →  with_noun            │
                  │       else              →  unknown              │
                  └─────────────────────────────────────────────────┘
                                         │
                                         ▼
                  ┌─────────────────────────────────────────────────┐
                  │  L4  realiser        (adam-dialog)              │
                  │       substitute {slot}; {slot|features} routes │
                  │       to synthesise_noun                        │
                  └─────────────────────────────────────────────────┘
                                         │
                                         ▼
                  ┌─────────────────────────────────────────────────┐
                  │  L5  FST synth       (adam-kernel-fst)          │
                  │       phonology + morphotactics → surface form  │
                  └─────────────────────────────────────────────────┘
                                         │
                                         ▼
                                    adam response
```

Every layer is a pure function except `Conversation::turn`, which mutates `session`, `active_intent`, and `intent_history`. Every mutation is deterministic in (previous state, input, seed).

The **offline reasoning sub-system** (parallel to the runtime pipeline above) produces the artefacts that L2.75 consumes:

```
  data/curated/*_pack.json
         │
         ▼
  ┌─────────────────────────┐
  │  extract_facts binary   │  (adam-reasoning)
  │    pattern matchers:    │
  │      copula       is_a  │
  │      locative     lives_in
  │      possessive   has   │
  │      dative       goes_to
  └─────────────────────────┘
         │
         ▼
  data/retrieval/facts.json     ← typed Fact triples
         │
         ▼
  ┌─────────────────────────┐
  │ build_lexical_graph bin │
  │   fact projection →     │
  │   nodes + typed edges   │
  └─────────────────────────┘
         │
         ▼
  data/retrieval/lexical_graph.json
         │
         ▼
  ┌─────────────────────────┐
  │  run_reasoner binary    │
  │    forward-chaining:    │
  │      R1  IsA transitivity
  │      R2  Has inheritance
  │      R5  shared IsA → RelatedTo
  └─────────────────────────┘
         │
         ▼
  data/retrieval/derived_facts.json   ← DerivedFact with rule_id + source_chain
```

---

## Four response paths, one pipeline

| path | triggered when | guarantees |
|---|---|---|
| **Recognised intent** | semantics matched one of 26 intents | template realisation, 0% fabrication |
| **Retrieval, verbatim quote** | `Unknown + noun_hint`, index has sample, `ComposeMode::Verbatim` (default) | quote is byte-identical to corpus, 0% fabrication |
| **Retrieval, adapted quote** | as above + `ComposeMode::InSampleCitySwap` + swap actually happened | quote is adapted; response frame explicitly marked with «бейімд-» |
| **Reasoning chain (v3.0)** | `Unknown + noun_hint`, `derived_facts` attached, match found | response cites derivation in Kazakh prose; always marked with **«байланыс-»**; `rule_id` + `source_chain` fully auditable |

The recogniser backbone (26 intents) **never invents text**. The retrieval path **never invents text** under `Verbatim`. Under `InSampleCitySwap` the text is grammatically re-inflected from a real source word; the **adaptation is disclosed in the frame by construction**. The reasoning path produces a **new claim** that no single corpus sentence states — but every such claim is pinned to a rule id and a source chain; the dialog layer marks the response with «байланыс-» so the user sees inference, not assertion.

---

## Trust markers — disclose everything

| marker | introduced | meaning |
|---|---|---|
| none | — | template realisation from recognised intent |
| quote in «…» | v1.6.5 | verbatim corpus citation |
| **«бейімд-»** | v1.9.5 | adapted quote — a city mention was swapped in |
| **«байланыс-»** | v2.7 | rule-derived reasoning chain — not a quote |

Every marker is test-enforced in both directions: *when it should fire, it fires; when it should not, it never does*. The negative invariants are the critical ones — adam never tells the user "I reasoned this" or "I adapted this" when it didn't.

---

## Determinism contract (v3.0, reaffirmed)

Same `(input, session, seed)` → byte-identical output across runs, machines, and time.

- FST synth is a pure function.
- FST parse enumerates deterministically.
- `MorphemeIndex::rank` ties on `(pack, sample_id)`.
- `compose_with_city` is a pure function; no RNG.
- `inject_retrieval_example` does not consult `rng_seed`.
- **`reasoner::run` is deterministic** — rules fire in declared order; fixpoint via bounded iteration (`MAX_ITER = 8`).
- **`render_derivation_as_kazakh` is a pure function** of the `DerivedFact`; every branch contains «байланыс-».
- `inject_reasoning_chain` picks the first matching derivation in stable order; does not consult `rng_seed`.

The **only** source of non-determinism in the API is the `rng_seed` argument to `Conversation::turn`. Same seed → same response.

---

## Reasoning engine (v2.1 → v2.9)

### Fact extraction

Four pattern matchers in `adam_reasoning::patterns`:

| pattern | emitted predicate | rule ref |
|---|---|---|
| `X — Y` (em-dash copula) | `IsA` | v2.1 |
| `X Y-да тұрады` | `LivesIn` | v2.1 |
| `X-тың Y-сы бар` | `Has` | v2.2 |
| `X Y-ке барады` | `GoesTo` | v2.5 |

Every matcher is type-checked on FST features (`Case`, `Predicate` enum, verb root), never on raw verb surface. Outputs `Fact` with categorical `ConfidenceKind::Grammar` + full `FactSource` provenance.

### Lexical graph

`LexicalGraph::from_facts(&[Fact]) -> LexicalGraph` — pure projection into nodes + typed edges. `BTreeMap`/sorted `Vec` so the on-disk JSON is byte-identical across runs.

API: `outgoing(root)`, `incoming(root)`, per-node `NodeStats`.

### Rule reasoner

`reasoner::run(&[Fact]) -> Vec<DerivedFact>` — forward-chaining with bounded iteration.

| rule | formula | status |
|---|---|---|
| **R1** | `A IsA B ∧ B IsA C ⟹ A IsA C` | active (v2.4) |
| **R2** | `A IsA B ∧ B Has X ⟹ A Has X` | active (v2.8) |
| R3 | `A LivesIn B ∧ B PartOf C ⟹ A LivesIn C` | documented, deferred |
| R4 | `A IsA B ∧ B IsA A` → diagnostic | documented, deferred |
| **R5** | `A IsA X ∧ B IsA X ⟹ RelatedTo(A, B)` | active (v2.6) |

Every `DerivedFact` carries:
- `rule_id: String` — stable identifier (never a probability score)
- `source_chain: Vec<FactSource>` — non-empty by invariant
- `confidence: ConfidenceKind::RuleInferred` — distinguishes derivations from extracted facts

### Dialog integration (v2.7)

`Conversation::with_reasoning_chains(extracted, derived)` attaches the artefacts. On `Intent::Unknown`, `inject_reasoning_chain` scans for a match and renders via `render_derivation_as_kazakh`. The planner routes to `unknown.with_derived_chain` — a template family whose every template contains «байланыс-».

---

## What v3.0 is NOT

- **Not a trained neural model.** No parameters, no embeddings, no gradient descent.
- **Not multilingual.** Kazakh-only surface (v1.1.0 revert stands).
- **Not generative.** Every token out is from a template, a corpus sample, an FST synthesis, or a rule-derived chain cited via the marker path.
- **Not a generalist.** 26 intents + retrieval + reasoning over the extracted-fact graph. Honest «түсінбедім» outside.
- **Not self-modifying.** Separate architectural direction, not on this roadmap.

---

## Trade-offs taken explicitly

| choice | what we lose | what we gain |
|---|---|---|
| Retrieval over a curated corpus, not a trained LM | generalisation outside recognised topics | no ungrounded generation on the corpus path (every quote is byte-identical to a committed sample), full provenance, runs on M2 |
| Kazakh-only surface | coverage of RU/EN speakers | a corpus and dialect that are native, not translated |
| Deterministic rank | novelty across reruns | reproducible demos, auditable trace |
| Frame-only composition by default | responses feel less personalised | quote is byte-identical to source — a hard promise |
| Opt-in in-sample swap | one extra flag on `Conversation` | the base case stays safe; users opt in explicitly |
| Closed 20-city swap list | coverage of rare cities | 100% swap candidates are known-valid FST roots |
| Biographical-year guard | some valid swaps refused | biographies never rewritten — a hard promise |
| **Rule-based reasoning, not neural** | **no emergent generalisation** | **every conclusion has a `rule_id` and a `source_chain`** |
| **Conservative monotonic inheritance in R2** | **real-world Has may not inherit through IsA** | **consumers can filter by `RuleInferred` confidence** |
| **«байланыс-» marker on every chain** | **slightly more verbose responses** | **inference distinguishable from quotation at the textual level** |

---

## Where the code lives

| concern | crate | file |
|---|---|---|
| Phonology | `adam-kernel-fst` | `src/phonology.rs` |
| Morphotactics | `adam-kernel-fst` | `src/morphotactics.rs` |
| FST analyser | `adam-kernel-fst` | `src/parser.rs` |
| Lexicon | `adam-kernel-fst` | `src/lexicon.rs` |
| Intent types | `adam-dialog` | `src/intent.rs` |
| Intent recognisers | `adam-dialog` | `src/semantics.rs` |
| Session + turn | `adam-dialog` | `src/conversation.rs` |
| Planner | `adam-dialog` | `src/planner.rs` |
| Realiser + slot syntax | `adam-dialog` | `src/realiser.rs`, `src/slot_syntax.rs` |
| Templates | data | `data/dialog/templates/v1.toml` |
| Morpheme index | `adam-retrieval` | `src/lib.rs` |
| Ranking | `adam-retrieval` | `src/lib.rs::rank` |
| Composition | `adam-retrieval` | `src/compose.rs` |
| **Fact types** | **`adam-reasoning`** | **`src/lib.rs`** |
| **Pattern matchers** | **`adam-reasoning`** | **`src/patterns.rs`** |
| **Lexical Graph** | **`adam-reasoning`** | **`src/graph.rs`** |
| **Rule reasoner** | **`adam-reasoning`** | **`src/reasoner.rs`** |
| **Kazakh renderer** | **`adam-dialog`** | **`src/conversation.rs::render_derivation_as_kazakh`** |
| Committed index artefact | data | `data/retrieval/morpheme_index.json` |
| **Committed facts artefact** | **data** | **`data/retrieval/facts.json`** |
| **Committed graph artefact** | **data** | **`data/retrieval/lexical_graph.json`** |
| **Committed derivations** | **data** | **`data/retrieval/derived_facts.json`** |
| Corpus packs | data | `data/curated/*_pack.json` |

---

## How to run the v3.0 demo

```bash
# 4-part scripted walkthrough — the canonical investor demo.
cargo run --release -p adam-dialog --bin adam_demo

# Interactive REPL with retrieval + reasoning on (v3.0 default).
cargo run --release -p adam-dialog --bin adam_chat

# Probe the reasoning path directly.
cargo run --release -p adam-dialog --bin adam_chat -- --once "кітап туралы бірдеңе айт"

# Full Layer 1..5 trace per turn.
cargo run --release -p adam-dialog --bin adam_chat -- --trace

# Rebuild reasoning artefacts from committed corpus.
cargo run --release -p adam-reasoning --bin extract_facts
cargo run --release -p adam-reasoning --bin build_lexical_graph
cargo run --release -p adam-reasoning --bin run_reasoner

# v1.1.0 fallback — retrieval and reasoning both off (regression reference).
cargo run --release -p adam-dialog --bin adam_chat -- --no-retrieval
```

---

## Post-v3.0 directions (committed but not shipped)

- **More pattern matchers** — densifying the fact graph so R1 transitivity fires on real corpus (currently awaiting middle-of-chain nodes).
- **Pattern coverage for `PartOf`** — activates R3.
- **R4 activation** — diagnostic surface for `IsA` symmetry (curator review).
- **Predicate types beyond the current six** — e.g. `Causes`, `Enables`, `Prevents` for causal reasoning.
- **Option C composition** — pre-compute `(pattern, slot_types)` at index-build time; enables swap types beyond city.
- **Kazakh technical corpus** — Rust Book translation as new source pack.
- **Diversity** — allow consecutive turns for the same query to cite different top-k samples / derivations; current top-1 is deterministic by design.

Each is additive. None requires rethinking the architecture shipped at v3.0.
