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

Eleven pattern matchers in `adam_reasoning::patterns` (v2.x baseline + v3.5.x scale-up):

| pattern | emitted predicate | since |
|---|---|---|
| `X — Y` (em-dash copula) | `IsA` | v2.1 |
| `X Y-да тұрады / мекендейді / орналасады` | `LivesIn` | v2.1 (v3.8.0 verb-root fix) |
| `X-тың Y-сы бар` | `Has` | v2.2 |
| `X Y-ке барады / келеді` | `GoesTo` | v2.5 (v3.8.0 verb-root fix) |
| `X-тің себебі / салдары Y` | `Causes` | v3.5.0 |
| `X-тен кейін / соң Y` | `After` | v3.5.0 |
| `X-тің N Y-сы бар` (quantified) | `HasQuantity` | v3.5.0 |
| `X Y-ні V-лайды` (transitive) | `DoesTo` | v3.5.0 (v3.5.5 stopwords, v3.6.0 passive refuse) |
| `X пен Y / X және Y` | `RelatedTo` | v3.5.0 |
| `X-ның саласы / ғылымы Y` | `InDomain` | v3.5.0 |
| `X-тің бөлігі / бөлшегі Y` | `PartOf` | v3.5.5 |

Every matcher is type-checked on FST features (`Case`, `Predicate` enum, verb root), never on raw verb surface. Outputs `Fact` with categorical `ConfidenceKind::Grammar` + full `FactSource` provenance. At v3.8.5, **9 of 11 predicates fire** on the committed 200 k-sample runtime (`Causes` and `InDomain` remain at 0 — literal head-word patterns rare in current corpus; v3.9+ target loosens them). **v3.8.5 precision hardening**: added `is_location_root` (refuses Қазақстан / Ресей / Алматы etc. as LivesIn subjects — countries can't reside), `is_time_noun` (refuses жыл / күн / ай as subjects of LivesIn / GoesTo / DoesTo — time adverbials are not agents), expanded `is_closed_class` with demonstrative qualifiers (мұндай / сондай / ондай / кейбір / өз / …), rejected LivesIn / GoesTo objects whose FST analysis retains a P3 possessive (fragment parses), 3-char minimum subject-root length. Result: facts dropped from 14 430 to **13 627** (−803, −5.6 %) with LivesIn the biggest precision-win (572 → 315, −44.9 %). **v3.9.0** added a central `is_fragment_root` post-filter (refuses any root starting with `-`) closing the 87 dash-prefixed fragment facts Codex flagged on v3.8.5 (`-дүниежүзілік`, `-ға`, `-жыл`, `-ғасыр`, …).

### World Core — curated knowledge packs (v3.9.0)

A second, orthogonal source of facts: human-authored JSONL in `data/world_core/<domain>.jsonl`. Each entry is one short Kazakh sentence + 1–3 typed facts + `reviewer` + `review_status`. Only `approved` entries reach the runtime. Emitted facts carry `ConfidenceKind::HumanApproved` + `source.pack = "world_core/<domain>.jsonl"` — the confidence-kind tier is **exclusive** to world_core; text extraction never produces `HumanApproved` facts.

| component | location |
|---|---|
| Schema + loader + validator | `adam_reasoning::world_core` |
| Validator binary | `cargo run -p adam-reasoning --bin validate_world_core` |
| Pipeline merge | `extract_facts` calls `load_world_core_facts(...)` after scanning text packs |
| `adam_inspect` split | per-root output has two sections: **Curated** (HumanApproved) first, **Extracted** (Grammar) after |
| Authoring guide | `data/world_core/README.md` |

Seed data shipped with v3.9.0: **80 entries / 126 curated facts** across `astronomy` (30), `time` (20), `geography_kz` (30).

**Expanded in v3.9.5 to 200 entries / 270 facts** with three new domains: `biology_basic` (40 / 41), `body_parts` (40 / 55), `society` (40 / 48). Long-term target: 5 000+ entries across 20+ domains by v5.x.

### Lexical graph

`LexicalGraph::from_facts(&[Fact]) -> LexicalGraph` — pure projection into nodes + typed edges. `BTreeMap`/sorted `Vec` so the on-disk JSON is byte-identical across runs.

API: `outgoing(root)`, `incoming(root)`, per-node `NodeStats`.

### Rule reasoner

`reasoner::run(&[Fact]) -> Vec<DerivedFact>` — forward-chaining with bounded iteration.

| rule | formula | status |
|---|---|---|
| **R1** | `A IsA B ∧ B IsA C ⟹ A IsA C` | active (v2.4) |
| **R2** | `A IsA B ∧ B Has X ⟹ A Has X` | active (v2.8) |
| **R3** | `A Has X ∧ X PartOf Y ⟹ A Has Y` | active (v3.5.5) |
| R4 | `A IsA B ∧ B IsA A` → diagnostic | curator-warning only |
| **R5** | `A IsA X ∧ B IsA X ⟹ RelatedTo(A, B)` | active (v2.6) |
| **R6** | `A LivesIn B ∧ B PartOf C ⟹ A LivesIn C` | active (v3.9.5) |
| **R7** | `A GoesTo B ∧ B PartOf C ⟹ A GoesTo C` | active (v3.9.5) |

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

## Post-v3.0 directions

### Shipped since v3.0 (v3.1 → v3.8, all additive)

- **v3.1.0** — iteration harness: `--time-budget`, `--progress-interval`, SIGINT → graceful commit, Rayon `par_iter` on extract hot loop (~3.5× speedup on M2 8-core).
- **v3.2.0** — `adam-scaling` crate + `scaling_bench` binary: empirical `(corpus_size → facts → derivations → graph density → wall-clock)` curves with normalized metrics. **Parser determinism fix** (dual-storage Lexicon) — closes a 2-year latent `HashMap.values()` non-determinism in `parser::analyse`.
- **v3.3.0** — Codex-review polish pass + `audit_precision` bin + **gold-corpus pilot** (3 secondary-school textbooks OCR'd via `tesseract-kaz`).
- **v3.4.0** — `mine_lexicon_gaps` binary: mines the committed corpus for high-frequency tokens no Lexicon root prefixes; auto-tags vowel harmony + final-sound class; writes `docs/lexicon_gap_candidates.md` for native-speaker review.
- **v3.5.0** — 7 more textbooks OCR'd (total 10 books, 434 k raw words, 28 110 samples) + **+6 pattern matchers** (copula_causes, temporal_after, quantity_count, agent_verb, nominal_conjunction, domain_membership) bringing predicate count 6 → 11.
- **v3.5.5** — `structural_part_of` matcher + **R3 rule activation** (`Has + PartOf → Has` mereological inheritance).
- **v3.6.0** — first `--use-shards` full-scale run: 5 tiers on 54.27 M-word pool (9 packs + 27 shards); T5_1M hits 3 h budget with 67 806 facts extracted partial. **R3 first real fire at T4_200k** (2 derivations).
- **v3.6.5** — committed runtime scaled to T4_200k equivalent (**13 345 facts, 207 derivations**); `adam_chat` / `adam_demo` now surface the 200× larger pool directly to the user.
- **v3.7.0** — `adam_inspect` binary: interactive "what does adam know about `<root>`?" query over the committed pool, with full provenance per claim.
- **v3.7.5** — `adam_demo` Part 4 refreshed to iterate one derivation per rule id (R1 / R2 / R3 / R5), showing all four cognitive operations in one demo run.
- **v3.8.0** — **critical verb-root bug fix**: `locative_lives_in` / `dative_goes_to` compared the infinitive forms (`"тұру"` / `"бару"`) against FST-stored stems (`"тұр"` / `"бар"`); neither predicate had ever fired at any scale since v2.1 / v2.5. Fix unblocks **LivesIn (572 facts) + GoesTo (1 864 facts)** at T4_200k. Predicate coverage jumps **7/11 → 9/11**.
- **v3.8.5** — **precision hardening** in response to Codex external review: matcher filters (location / time-noun / demonstrative blocklists, possessive-object refusal, 3-char minimum stem), renderer FST synthesis (case suffixes no longer dash-concatenated), demo preview / actual-render alignment (subject-first two-pass in `inject_reasoning_chain`), contradicting README rule-count row removed. Facts drop to **13 627** (−803, −5.6 %) with LivesIn the biggest precision-win (572 → 315, −44.9 %); derivations 207 → 205; coverage holds at 9/11. **423 workspace tests** (+7). First release with a morphology-regression test.
- **v3.9.0** — **World Core v1 + fragment-root hygiene gate**. Codex's second-pass review crystallised the architectural direction: **not** an LLM-clone, but an *auditable Kazakh reasoning engine*. Ships (a) central `is_fragment_root` post-filter that drops the 87 `-`-prefixed fragment facts Codex measured; (b) World Core infrastructure — `data/world_core/*.jsonl` human-authored knowledge packs, `adam_reasoning::world_core` loader / validator / emitter, `validate_world_core` binary, pipeline merge into `extract_facts`, `ConfidenceKind::HumanApproved` as exclusive tier; (c) seed data (80 entries / 126 curated facts across `astronomy`, `time`, `geography_kz`); (d) `adam_inspect` split into **Curated** + **Extracted** sections. First release where adam has structured foundational knowledge beyond what the Kazakh corpus makes explicit.
- **v3.9.5** — **World Core expansion + R6/R7 rules + dialog closed-class sync**. World Core grows 80 → 200 entries / 126 → 270 facts with three new domains (`biology_basic` 40 / 41, `body_parts` 40 / 55, `society` 40 / 48). R6 (`LivesIn + PartOf → LivesIn`) and R7 (`GoesTo + PartOf → GoesTo`) activate — both were waiting on v3.8.0's verb-root fix + v3.9.0's curated city-PartOf-country chains. **R7 fires 640× and R6 fires 103×** on the v3.9.5 T4 runtime; R3 first-fires on curated chains (15×); R5 nearly doubles (489 → 933). Total derivations **704 → 2 058 (×2.9)**. Dialog `NOT_A_TOPIC` synced with `adam-reasoning::patterns::is_closed_class` — closes the «Неліктен → Нелікте тұрасыз ба» REPL bug. **440 tests** (+7); 6 active rules (R1 / R2 / R3 / R5 / R6 / R7). Facts: 13 501 extracted + 270 curated = 13 771. Graph 3 151 / 12 317. Predicate coverage 11/11.

### Committed but not yet shipped (v4.0+ targets)

- **FST genitive-after-vowel phonology fix** — the `{D}{I}ң` genitive template produces `қаладың` instead of `қаланың` on vowel-final stems (discovered during v3.8.5 renderer work; sidestepped by using dative/ablative in the reasoning-chain renderer). Dedicated phonology fix is a v4.x target since it affects 48+ existing FST roundtrip tests.
- **Loosen `copula_causes` + `domain_membership`** — literal head-word patterns (`себебі`, `саласы`, `ғылымы`) are rare even at T4. Accept broader causal / domain constructions to push coverage **9/11 → 11/11**.
- **R4 activation** — diagnostic surface for `IsA` symmetry (curator review; remains documented-only because its output is a curator warning, not a fact).
- **Native-speaker precision audit** — 50-fact / 50-derivation sample in `docs/precision_audit.md` is primed; unblocks first Lexicon PR from v3.4.0 candidates.
- **`occurrence_count` first-class field** — Codex #4 follow-up: capture the T5 duplicate signal (25.7 % at 1 M samples) as a per-fact weight rather than de-duplicating on extraction.
- **`--persist-tier` on `scaling_bench`** — write per-tier facts / derived artefacts so `adam_chat --facts-tier T5_1M` can expose the biggest pool interactively.
- **Option C composition** — pre-compute `(pattern, slot_types)` at index-build time; enables swap types beyond city.
- **Kazakh technical corpus** — Rust Book translation as new source pack.
- **Diversity** — allow consecutive turns for the same query to cite different top-k samples / derivations; current top-1 is deterministic by design.

Each is additive. None requires rethinking the architecture shipped at v3.0.
