# v6.1.0 — AnswerIR + Predicate-Aware Retrieval (design)

**Status.** Design draft. To be implemented on
`experimental/v6_1_answer_ir`, forked from `main` at commit
`6822e181` on 2026-05-22.

**Origin.** Codex external audit on v6.0.0 (HEAD `cb385bfc`,
2026-05-22) closed all four actionable correctness findings + the
ten doc-drift items, but flagged that the project's remaining
architectural gap is **not latency, not hallucination, not
safety** — it is **answer relevance and completeness**. Concrete
audit evidence:

- «Қазақстан туралы айтыңыз» returns one ОДКБ fact, not a
  multi-fact summary.
- «Жасанды интеллект туралы заң қандай санаттарға жіктейді?»
  surfaces an effective-date fact instead of the classification
  fact.
- «Ахмет Байтұрсынұлы қашан туылған?» pre-v6.0.15 surfaced the
  bio IsA fact instead of the birth-date `related_to` fact (the
  predicate-keyword hack landed the date for this case but the
  pattern is brittle).
- «Төте жазу деген не?» returns a creation-history sentence
  (kru_005a was added in v6.0.15 to give it a clean definitional
  raw_text; the same fix is needed for many curated subjects).

The audit's architectural recommendation, paraphrased:

> «Не пытаться превращать adam в маленький LLM. Правильная
> траектория: deterministic claim compiler + tiny neural
> components только для закрытых решений. Neural может выбирать
> intent, slot, rank, style/template variant, но не должен
> свободно порождать факты.»

v6.1.0 operationalises this. The central abstraction is
`AnswerIR` — a typed intermediate representation that sits
**between** the deterministic cascade / opt-in neural transducers
and the surface realiser. Every claim in the output is bound to
a typed `Claim { subject, predicate, object, source_fact_id }`
that the verifier already validates. The generator (template
realiser today; neural composer tomorrow) consumes `AnswerIR`
and CANNOT invent facts because the interface doesn't expose a
free-text channel.

This document is the **predeclared design + success criteria**
for the experiment, in the discipline of the third-path arc
(E1 / E2 / E3) it follows.

### Pre-existing substrate (revised 2026-05-23)

Initial-draft framing called `AnswerIR` and `QuestionShape` new.
Audit of the v6.0.0 source tree shows both already exist:

- [`crates/adam-dialog/src/answer_ir.rs`](../crates/adam-dialog/src/answer_ir.rs)
  (675 lines, shipped v5.9.0 / G3.0 of the proof-carrying arc) —
  defines `AnswerIR`, `AnswerNode`, `AnswerShape` covering
  `YesNoConfirm` / `YesNoDeny` / `YesNoUnknown` / `SafetyRefusal` /
  `IsAProofChain`. Proof-driven composition, not retrieval-driven.
- [`crates/adam-dialog/src/question_shape.rs`](../crates/adam-dialog/src/question_shape.rs)
  (526 lines, shipped v4.12.0) — `QuestionShape` enum covering
  `Definition` / `Causal` / `YesNoCheck` / `Listing` / `Comparison`,
  orthogonal to `Intent`.
- `adam_reasoning::Predicate` — typed predicate enum already
  carrying `IsA`, `RelatedTo`, `PartOf`, `GoesTo`, `HasQuantity`,
  `Causes`, `After`, `LivesIn`, `Has`, `Does`.

v6.1.0 is therefore **extension**, not green-field:

1. Extend `AnswerShape` with `Definition`, `DatedFact`,
   `Classification`, `List`, `BroadTopic` (retrieval-driven shapes
   the v5.9.0 substrate does not yet cover).
2. Extend `AnswerNode` with the corresponding leaves
   (`DefinitionFact`, `DatedFact`, `Classification`, `MultiClaim`).
3. Add a NEW `PredicateFocus` enum (refines `QuestionShape`
   downstream — the question-shape detector says «this is a
   Definition», the predicate-focus detector says «...and the
   predicate target is `BornIn`»). Lives in a sibling module
   `crates/adam-dialog/src/predicate_focus.rs`.
4. Extend `adam_reasoning::Predicate` with the 11 new typed
   variants (`BornIn`, `EffectiveFrom`, `Classifies`, …) so the
   retrieval planner has typed targets instead of overloaded
   `RelatedTo`.
5. Add `build_answer_ir(ProofObject | RetrievalResult,
   PredicateFocus, DialogContext) -> AnswerIR` as the new entry
   path next to the existing `compose(proof, shape, rng_seed)`.

The proof-carrying composition path (`YesNo*` + `SafetyRefusal` +
`IsAProofChain`) is unchanged. v6.1.0 is the retrieval-driven
sibling: same `AnswerIR` substrate, different source of claims
(curated facts.json + reasoner derivations instead of proof
objects built from beliefs).

## The thesis under test

> A typed, claim-level intermediate representation (`AnswerIR`)
> sitting between intent classification and surface realisation
> enables (a) multi-claim answers without a generative LLM, (b)
> predicate-aware retrieval so questions with the same subject
> but different predicates («X кім?», «X қашан туылған?», «X
> қандай санатта?») route to the right fact, and (c) typed
> safety / discourse / style modulation — all without expanding
> the closed-set guarantees that v6.0.0 ships.

If the thesis holds, v6.1.0 closes the relevance / completeness
gap the Codex audit identified, AND becomes the substrate for a
future neural composer (L5.5+) that can paraphrase / re-order /
re-style **already-verified** claims without ever generating
new factual content.

If the thesis fails, the architectural extension is rolled back
and the pre-v6.1.0 pre-action-plan probe + KRU whitelist remain
the production path. The kru_baitursynov.jsonl rewrite back to
typed predicates (see Stage 2 below) is also rolled back.

## Scope

**In scope.**

1. `AnswerIR` Rust type and its dependent types (`Claim`,
   `PredicateFocus`, `StyleTarget`, `SafetyMode`, `DiscourseHint`).
2. Extension of `adam_reasoning::Predicate` with the typed
   variants the current `related_to` over-uses:
   - `BornIn`, `DiedIn`, `FoundedIn`, `RenamedIn`, `EffectiveFrom`
   - `Classifies`, `RiskLevel`, `LocatedIn`, `NamedAfter`,
     `MemberOf`, `Authored`
3. Question-shape → `PredicateFocus` planner in
   `crates/adam-dialog/src/answer_ir/`:
   - «X қашан туылған?» → `PredicateFocus::BornIn`
   - «X қашан күшіне енді?» → `PredicateFocus::EffectiveFrom`
   - «X қандай санат?» / «жіктейді» → `PredicateFocus::Classifies`
   - «X қайда орналасқан?» → `PredicateFocus::LocatedIn`
   - «X-ні кім жасады?» → `PredicateFocus::Authored`
   - bare «X кім?» / «X деген не?» → `PredicateFocus::IsA`
4. Predicate-aware retrieval planner that, given
   `(noun_hint, predicate_focus)`, returns the highest-confidence
   matching fact or honest empty.
5. Multi-claim answer composer for broad-topic queries («X
   туралы айтыңыз») — returns up to N=3 claims with a
   non-repetition contract across consecutive turns
   («ал тағы айт» picks unseen claims).
6. `extract_facts` extension to parse + emit the new predicate
   variants from `data/world_core/*.jsonl`.
7. `world_core/kru_baitursynov.jsonl` re-promoted to use typed
   predicates (the v6.0.13 canonical-only rewrite was a forced
   workaround for the missing variants).
8. Regression test pack `tests/v6_1_answer_ir_regression.rs`:
   ≥ 30 multi-predicate query cases pinned with the typed answer
   they should surface.

**Out of scope.**

- Neural answer composition (L5.5+ TinyAgt or larger). The
  generator stays template-based in v6.1.0.
- Cross-language port. Kazakh only.
- Streaming output. Single-shot reply per turn.
- New domain world_core additions. Re-uses the v6.0.0 surface.

## The Rust types (sketch)

**Reconciliation with v5.9.0 substrate.** The existing
`AnswerIR { shape, root: AnswerNode, source_proof: ProofObject }`
keeps its tree-walking realiser contract. v6.1.0 extends it as
follows (the diff is purely additive on `AnswerShape` /
`AnswerNode` and replaces `source_proof` with a sum so the
retrieval path can carry curated fact ids instead of synthesising
a proof object):

```rust
// EXTEND in crates/adam-dialog/src/answer_ir.rs:

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnswerSource {
    /// v5.9.0 proof-carrying path.
    Proof(ProofObject),
    /// v6.1.0 retrieval-driven path.
    Retrieval {
        fact_ids: Vec<FactSource>,
        predicate_focus: PredicateFocus,
    },
}

pub struct AnswerIR {
    pub shape: AnswerShape,
    pub root: AnswerNode,
    pub source: AnswerSource,  // was: source_proof: ProofObject
}

pub enum AnswerShape {
    // existing v5.9.0:
    YesNoConfirm, YesNoDeny, YesNoUnknown,
    SafetyRefusal, IsAProofChain,
    // v6.1.0 retrieval-driven additions:
    Definition,      // «X — Y.»
    DatedFact,       // «X 1872 жылы туылған.»
    Classification,  // «X жоғары тәуекелді санатқа жатады.»
    List,            // «X-нің Y-лары: A, B, C.»
    BroadTopic,      // ≤ 3 claims for «X туралы айтыңыз»
}

pub enum AnswerNode {
    // existing v5.9.0:
    ConfirmVerdict { .. }, DenyVerdict { .. },
    Subject { .. }, Predicate { .. },
    Punctuation { .. }, ChainCitation { .. },
    Hedge { .. }, RefusalBody { .. },
    ProofPrologue { .. }, ProofChainSteps { .. },
    ProofConclusion { .. }, Sequence { .. },
    // v6.1.0 retrieval-leaf additions:
    DatedFactBody { subject: String, date: String, predicate_verb: String },
    ClassificationBody { subject: String, category: String },
    MultiClaimItem { claim: Claim, index: usize },
}

pub struct Claim {
    pub subject: SlotRef,
    pub predicate: Predicate,
    pub object: SlotRef,
    pub source_fact_id: FactSource,
    pub confidence: ConfidenceKind,
}

pub enum PredicateFocus {
    /// «X кім?» / «X деген не?» — definition probe.
    IsA,
    /// «X қашан туылған?» — date probe.
    BornIn,
    /// «X қашан күшіне енді?» — date probe (laws / agreements).
    EffectiveFrom,
    /// «X қашан құрылған?» — date probe (institutions / events).
    FoundedIn,
    /// «X қандай санат?» / «жіктейді» — classification probe.
    Classifies,
    /// «X қандай тәуекелді?» — risk-level probe.
    RiskLevel,
    /// «X қайда?» / «қай қалада?» — location probe.
    LocatedIn,
    /// «X-ні кім жасады?» — actor probe.
    Authored,
    /// «X-нің авторы кім?» / «X-нің атасы кім?» — relational
    /// genitive probe. The pre-plan probe currently skips
    /// genitive-possessive shapes entirely; this variant
    /// re-enables them by typing the relation.
    Relational,
}

pub enum StyleTarget {
    /// «X — Y.» (single IsA sentence).
    Definition,
    /// Date + context: «X 1872 жылы 5 қыркүйекте туылған.»
    DatedFact,
    /// Up to N items: «X-нің Y-лары: A, B, C.»
    List,
    /// «X жоғары тәуекелді санатқа жатады.»
    Classification,
    /// «X — қажет емес» / refusal.
    Refusal,
    /// «Сізді ұқтым.» (resolution / acknowledgement).
    Acknowledgement,
    /// Multi-claim ≤ 3 claims for broad-topic queries.
    BroadTopic,
}

pub enum SafetyMode {
    Allowed,
    Medical,
    Legal,
    Financial,
    SelfHarm,
    CurrentData,
}

pub struct DiscourseHint {
    /// Carry from previous turn («ал тағы айт» about same X).
    pub continues_topic: Option<String>,
    /// Already-emitted fact ids in this conversation; the
    /// answer planner skips them when fulfilling
    /// `style = BroadTopic`.
    pub already_seen_fact_ids: Vec<FactSource>,
}
```

## Pipeline

The default path becomes:

```
input
  ↓ (parse + intent classification — existing cascade + opt-in E1)
intent
  ↓ (apply_tool_results — retrieval + reasoner, existing)
intent (now with grounded_fact / reasoning_chain / example)
  ↓ (NEW: build AnswerIR)
AnswerIR { claims, predicate_focus, safety, discourse, style }
  ↓ (NEW: predicate-aware fact selection over claims)
AnswerIR (claims filtered + ranked by predicate_focus)
  ↓ (existing planner + realiser, consuming AnswerIR instead of raw intent)
output
```

The `build_answer_ir` step does:

1. Detect `PredicateFocus` from input + intent (rule-based
   pattern table; same shape as `detect_ask_*` cascade).
2. Pull candidate `Claim`s from `extracted_facts` whose
   `subject.root` matches the noun_hint AND (if
   `predicate_focus.is_some()`) whose `predicate` matches the
   focus, with `IsA` fallback for pure definitions.
3. For `style = BroadTopic` queries, pull ≤ 3 highest-
   confidence + lowest-already-seen-overlap claims.
4. Bind `safety` from `detect_safety_topic`.
5. Bind `discourse` from `DialogContext` + the existing
   anaphora-resolution layer.
6. Return.

The realiser branch dispatches on `style`:

- `Definition` → one IsA claim → existing `unknown.with_grounded_fact`
  template family.
- `DatedFact` → one date claim → new template
  «{subject} — {object}-да {predicate-verb}.» (e.g.
  «Ахмет Байтұрсынұлы — 1872 жылы 5 қыркүйекте туылған.»).
- `Classification` → one classify claim → new template
  «{subject} {object} санатқа жатады.».
- `List` / `BroadTopic` → ≤ 3 claims → new
  `multi_claim.broad_topic` family rendering each claim as a
  separate sentence with «байланыс» preserve-marker on
  derived ones.
- `Refusal` → existing safety templates.
- `Acknowledgement` → existing resolve / dismiss templates.

## Predeclared success criteria

The experiment ships **iff all of these hold** measured on the
v6.0.0-frozen `factual_eval_100` + a new `v6_1_predicate_eval_30`
held-out battery built from the audit findings:

| criterion | target | how measured |
|---|---|---|
| Predicate-shaped query answers the QUESTION | ≥ 25 / 30 | `v6_1_predicate_eval_30` cases hand-graded against the expected predicate target |
| `factual_eval_100` ceiling maintained | ≤ 3 hallucinations | existing test |
| 0 hallucinated claims in `v6_1_predicate_eval_30` | yes | every output claim must have a non-null `source_fact_id` that resolves to a curated fact in `facts.json` |
| Latency p99 increment | ≤ 5 ms over the v6.0.0 cascade | `resource_bench` re-run |
| Cascade pass-through invariant | 0 failures across the 32-case `v6_0_6_audit_regression` suite | existing test |

## Predeclared anti-success (rollback triggers)

The experiment is rolled back **iff any of these fire**:

1. `factual_eval_100` hallucinations exceed the v6.0.0-rc5
   ceiling of 3.
2. Latency p99 regresses by more than 10 ms.
3. Any output claim surfaces a fact that doesn't resolve to a
   curated `source_fact_id` (i.e. the realiser invented
   content, which would mean the type system was bypassed).
4. The 32-case audit-regression suite loses any case (regression
   on already-shipped contracts).
5. `cognitive_eval` canonical scenarios lose any case.

If any rollback trigger fires after 3 implementation iterations,
the branch is closed as `experimental/v6_1_answer_ir` (no merge
to `main`) and the v6.0.0 production path stays authoritative.

## Stages

### Stage 1 — `PredicateFocus` + detector (1-2 days)

- Add `crates/adam-dialog/src/predicate_focus.rs` (new
  sibling module, not folded into `answer_ir.rs` until the type
  reconciliation lands in Stage 3).
- Define `PredicateFocus` enum + `Option<PredicateFocus>
  detect(input: &str, shape: Option<QuestionShape>, parses:
  &[Analysis]) -> Option<PredicateFocus>` covering the ~12
  question patterns catalogued above.
- Unit tests in the same module — every pattern → expected
  focus mapping.
- No production wiring yet. `lib.rs` exposes the new module but
  nothing on the runtime hot path calls it; v6.0.0 cascade
  remains unchanged.

### Stage 2 — Predicate enum extension + world_core re-promotion (2 days)

- Extend `adam_reasoning::Predicate` enum with the 11 new
  variants.
- Update `extract_facts` to parse the new predicate names from
  world_core JSONL.
- Rewrite `data/world_core/kru_baitursynov.jsonl` BACK to use
  the typed predicates (the v6.0.13 canonical-only rewrite was
  a forced workaround for the missing variants).
- Regenerate `data/retrieval/facts.json` +
  `derived_facts.json`.
- Verify factual_eval_100 still on the ceiling.

### Stage 3 — `build_answer_ir` + predicate-aware retrieval (2 days)

- Implement the pipeline step described above on a behind-flag
  basis: `ADAM_ANSWER_IR=1` defaults to off; v6.0.0 cascade
  unchanged.
- Wire `AnswerIR` into the existing planner via a thin adapter
  (`AnswerIR → extra_slots + intent_for_render` shim) so the
  realiser doesn't need a v6.1-wide rewrite up front.
- Cascade pass-through invariant: when `ADAM_ANSWER_IR=0`
  (default), behaviour is bit-identical to v6.0.0 cascade.

### Stage 4 — BroadTopic multi-claim composer (1-2 days)

- Implement the `≤ 3 claims with non-repetition` planner for
  «X туралы айтыңыз» queries.
- Persist `DiscourseHint::already_seen_fact_ids` in
  `DialogContext` so «ал тағы айт» picks unseen claims.
- New `multi_claim.broad_topic.kk.toml` template family.

### Stage 5 — Eval + decision gate (1 day)

- Build the 30-case `v6_1_predicate_eval_30` held-out battery
  from the audit-flagged failure shapes.
- Hand-grade each output against expected predicate target.
- Run the full success-criteria checklist.
- Decide: ship behind `ADAM_ANSWER_IR=1` opt-in, ship as
  default, or rollback.

## What this is NOT

- **Not** a free-form generator. Every output claim is bound to
  a curated fact. The realiser's job is composition, not
  invention.
- **Not** a step toward LLM scale. The new types are small,
  inspectable, and the `Claim` source is always traceable.
- **Not** a replacement for the existing cascade. The v6.0.0
  cascade remains the default when `ADAM_ANSWER_IR=0`. v6.1.0
  ships either as an opt-in flag or as the default ONLY if all
  success criteria + zero regressions confirm.

## Cross-references

- [`docs/third_path_results.md`](third_path_results.md) — the
  E1 / E2 / E3 discipline this design inherits.
- [`docs/architecture_neural_v6.md`](architecture_neural_v6.md) —
  the v6.0.0 layered pipeline; `AnswerIR` slots between L5
  (template planner) and the existing intent layer.
- [`RESEARCH.md`](../RESEARCH.md) §"Completed research arc —
  Agglutinative-Neural" — the precedent for this experiment's
  discipline.
- [Codex audit 2026-05-22 transcript] — the external review
  whose architectural recommendation this design implements;
  see commit `6822e181` and the audit findings catalogued in
  it.

## Open questions before Stage 1 starts

1. Should `AnswerIR` be in `adam-dialog` or a new
   `adam-answer-ir` crate? Pros of new crate: clear boundary,
   easier neural-composer follow-on. Pros of in-crate: lower
   refactor cost.
2. The `Authored` predicate overlaps with the existing
   `DoesTo` we're already extracting. Should they unify?
3. For `BroadTopic` answers, how do we rank when there are
   > 3 candidate claims? Options: HumanApproved confidence
   first, then fact-id alphabetical (deterministic); or a
   tiny perceptron trained on a hand-labelled relevance set.
4. Discourse continuity — `already_seen_fact_ids` lives in
   `DialogContext` per turn; how long does it carry? Reset on
   topic switch? Reset after N turns? TBD; default proposal:
   reset on every topic switch detected by the existing
   `topic_extraction` layer.

These are the items the v6.1.0 owner should resolve before
opening the implementation PRs.
