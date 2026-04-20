# adam v2.0 — architecture reference

This document is the **single canonical description** of the v2.0 system. It freezes the architectural commitment that has crystallised across v1.5.0–v1.9.5.

> **v2.0 is not a trained neural model.** It is a deterministic retrieval + composition engine over a 77.9 M-word Kazakh corpus. See [`project_retrieval_not_neural_v2`](https://github.com/qazaq-ai/adam/blob/main/docs/roadmap.md#post-v10-direction).

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
                  │  L2.5  follow-up     (v1.4.0)                   │
                  │       resolve_follow_up — "ал сіз?" retagging   │
                  └─────────────────────────────────────────────────┘
                                         │
                                         ▼
                  ┌─────────────────────────────────────────────────┐
                  │  L2.75  retrieval inject (Intent::Unknown only) │
                  │       v1.6.5: rank→top-1 sample text             │
                  │       v1.9.0: optional compose_with_city         │
                  │       v1.9.5: example_adapted flag               │
                  └─────────────────────────────────────────────────┘
                                         │
                                         ▼
                  ┌─────────────────────────────────────────────────┐
                  │  L3  planner         (adam-dialog)              │
                  │       intent → template family → fillable pool  │
                  │       → seed-mod pick                           │
                  └─────────────────────────────────────────────────┘
                                         │
                                         ▼
                  ┌─────────────────────────────────────────────────┐
                  │  L4  realiser        (adam-dialog)              │
                  │       substitute {slot} → synthesise_noun for  │
                  │       {slot|features}                           │
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

Every layer is a pure function except `Conversation::turn`, which mutates `session`, `active_intent`, and `intent_history`. Every mutation is deterministic in (previous state, input).

---

## Three response paths, one pipeline

| path | triggered when | guarantees |
|---|---|---|
| **Recognised intent** | semantics matched one of 26 intents | template realisation, 0% fabrication |
| **Retrieval with verbatim quote** | `Unknown + noun_hint`, index has sample, `ComposeMode::Verbatim` (default) | quote is byte-identical to corpus, 0% fabrication |
| **Retrieval with adapted quote** | as above + `ComposeMode::InSampleCitySwap` + swap actually happened | quote is adapted; response frame explicitly marked with «бейімд-» |

The recogniser backbone (26 intents) **never invents text**. The retrieval path **never invents text** under `Verbatim`. Under `InSampleCitySwap` the text is grammatically re-inflected from a real source word; the **adaptation is disclosed in the frame by construction**.

---

## Determinism contract

`adam` guarantees: same `(input, session, seed)` → **byte-identical output** across runs, machines, and time.

- FST synth is a pure function (phonology rules are deterministic algorithms).
- FST parse enumerates deterministically; returns results in insertion order.
- `MorphemeIndex::rank` ties on `(pack, sample_id)` lex order.
- `compose_with_city` is a pure function (first match per token, no RNG).
- `inject_retrieval_example` does **not** consult `rng_seed`. Evidence selection is reproducible.
- Template pick uses `rng_seed % fillable_pool.len()`.

The **only** source of non-determinism in the API is the `rng_seed` argument to `Conversation::turn`. Pass the same seed twice, get the same response twice.

---

## Retrieval engine (v1.6.0–v1.9.5)

### Index build

`build_morpheme_index` walks committed packs, parses every unique word once via FST, indexes the sample under every root returned. The unique-word cache drops a full-corpus build from ~75 minutes to ~10 minutes.

- **Keys**: root surface strings (e.g. `балаларды` indexes under `бала`).
- **Values**: `SampleRef { pack, sample_id }` — provenance back to exactly one sentence.
- **Sample texts** (v1.6.5+): `SampleRef::text_key() → text` — so dialog can cite the actual sentence.

### Ranking

```
score = 0.40 · overlap_ratio           (share of input morphemes the sample matches)
      + 0.30 · pack_purity              (Abai 1.00, Wikipedia 0.85, CC-100 0.75)
      + 0.15 · length_goodness          (Gaussian μ=8 words, σ=6)
      − 0.15 · loanword_density         (Russian-only letters + loanword suffixes)
```

All four components are pure deterministic functions. Weights are editorial constants (not learned). Ties break on `(pack, sample_id)`.

### Composition (option B, opt-in)

`compose_with_city(sample, user_city, lexicon) → Composition` rewrites city mentions inside the sample to the user's city, preserving full FST features (locative stays locative, etc.).

**Safety guards:**

1. **Closed list** — only the 20 cities in `PLACE_NAMES` are swappable.
2. **User's city must also be in the list** — otherwise the FST can't re-synthesise.
3. **Biographical-year guard** — any 4-digit number in [1500, 2100] refuses the whole swap. Keeps biographies untouched.
4. **No name or number swaps** — out of scope.

### Composition marker (v1.9.5)

When a swap happens, `example_adapted: bool` flips true and the planner routes to the `unknown.with_adapted_evidence` template family. Every template there contains the Kazakh stem **«бейімд-»** ("adapt-"), so the user always sees the adaptation disclosed.

**Test-enforced invariants:**

- `adapted_evidence_templates_announce_the_adaptation` — swap → marker fires.
- `verbatim_mode_never_claims_adaptation` — no swap → marker NEVER fires.

The negative invariant is trust-critical: **v2.0 never claims to have adapted a quote it didn't actually adapt.**

---

## What is v2.0 NOT

- **Not a trained neural model.** No parameters, no embeddings, no PyTorch.
- **Not multilingual.** Kazakh-only surface (v1.1.0 revert of v0.9.6's RU/EN triggers).
- **Not generative.** Every token out of the system is either from a template, a corpus sample, or an FST synthesis of a morpheme bundle.
- **Not a generalist.** adam answers what it was built to answer: 26 conversational intents + retrieval-based responses for known topics. Questions outside that envelope get `unknown.with_noun` or bare `unknown` — honest.
- **Not self-modifying.** v2.0 cannot change its own code. That is a separate architectural direction, not on the retrieval roadmap.

---

## Trade-offs taken explicitly

| choice | what we lose | what we gain |
|---|---|---|
| Retrieval over a curated corpus, not a trained LM | generalisation outside recognised topics | 0% hallucination on corpus path, full provenance, runs on M2 |
| Kazakh-only surface | coverage of RU/EN speakers | a corpus and dialect that are native, not translated |
| Deterministic rank | novelty across reruns | reproducible demos, auditable trace |
| Frame-only composition by default | responses feel less personalised | quote is byte-identical to source — a hard promise |
| Opt-in in-sample swap | one extra flag on `Conversation` | the base case stays safe; users opt into fabrication risk explicitly |
| Closed 20-city swap list | coverage of rare cities | 100% swap candidates are known-valid FST roots |
| Biographical-year guard | some valid swaps refused | biographies never rewritten — a hard promise |

---

## Where the code lives

| Concern | Crate | File |
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
| Committed index artifact | data | `data/retrieval/morpheme_index.json` |
| Corpus packs | data | `data/curated/*_pack.json` |

---

## How to run the demo

```bash
# Scripted 15-turn walkthrough, deterministic, safe to record for investors.
cargo run --release -p adam-dialog --bin adam_demo

# Interactive REPL with retrieval on (v2.0 default).
cargo run --release -p adam-dialog --bin adam_chat

# Same, with in-sample city swap enabled (marker will fire on adapted quotes).
cargo run --release -p adam-dialog --bin adam_chat -- --compose

# Full Layer 1..5 trace per turn.
cargo run --release -p adam-dialog --bin adam_chat -- --trace

# v1.1.0 fallback — no retrieval, no composition (regression reference).
cargo run --release -p adam-dialog --bin adam_chat -- --no-retrieval
```

---

## Post-v2.0 directions (committed but not shipped)

- **Option C** — pre-compute `(pattern, slot_types)` pairs at index-build time. Moves swap candidate analysis offline; enables swap types beyond city.
- **Kazakh technical corpus** — translate key chapters of the Rust Book into Kazakh as a new source pack. Doubles as educational material and corpus-vocabulary expansion.
- **Diversity** — allow consecutive turns for the same query to cite different samples; current top-1 is deterministic by design.
