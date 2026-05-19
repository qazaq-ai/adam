# Foundation Scope

> **Forward-looking update 2026-05-16.** The "no probabilistic free
> generation" line below describes v5.x. v6.0.0 introduces an
> additive **algebra-anchored neural composition layer (L5.5)** —
> see [`architecture_neural_v6.md`](architecture_neural_v6.md) for
> the spec and [`MANIFESTO.md`](MANIFESTO.md) for the position. The
> v6.0 path remains predictable, auditable, and CPU-resident; the
> neural layer sits between the deterministic template (L5) and the
> verifier (L6), and a missing model file boots-and-runs the v5.x
> behaviour unchanged.

## Goal

Deliver a **predictable, auditable Kazakh reasoning engine** built entirely in Rust and runnable on a MacBook Air M2 8 GB. Every layer's decision must be traceable. No probabilistic free generation in the recognised-intent path. **Not** an LLM clone — intentionally narrower, intentionally cheaper, intentionally provenance-first.

In v6.0.0 (forthcoming, see Q3 2026 milestone) this goal extends:
deterministic core remains load-bearing, **plus** an algebra-anchored
neural composition layer that selects among morpho-valid surface
variants under the same verifier gate. Hallucination remains
architecturally impossible because the verifier still blocks any
output that doesn't ground in `world_core` / `facts.json`.

## In scope (v1.0.0 → v4.22.5 delivered)

### Morphology + Lexicon
- Pure Kazakh **Lexicon** (~25.5 k roots: 13 606 pure Kazakh + 11 919 Apertium imports; v2.2 purged intervocalic-voicing-duplicate pollutions; v3.2.0 dual-storage for deterministic iteration).
- Deterministic **FST morphology** (phonology + morphotactics + inverse parser; v2.3 glide-vowel classification fix for у/и/ю; v3.8.0 verb-root stem vs infinitive fix).
- **Kazakh-only recogniser surface** (v0.9.6 trilingual experiment reverted in v1.1.0; Latin transliteration removed).

### Dialog layer
- **26-intent dialog pipeline** with multi-turn session state + follow-up resolution (v1.4.0).
- **FST-backed slot expansion** (`{slot|features}` — case × number × derivation × possessive × predicate-person).
- Template repository as external TOML data (**67 families** at v4.22.5; v4.0.34 added Tentative / Conflicted families for epistemic banding; v4.3.4 added four `ask_about_system.*` aspect families; v4.4.0 added `dismiss_contradiction`; v4.4.5 added `check_contradiction`; v4.18.0+ added respectful Kazakh address `{name_respect}`; v4.18.5 added composite-question `intro_and_capabilities`).
- **Session-aware composition** (v1.8.0+): frame around retrieved quote personalises when session has name/city/age/occupation.
- **Opt-in city swap** (v1.9.0+): `ComposeMode::InSampleCitySwap` rewrites city mentions via FST feature-preserving synthesis, year-guarded; adapted responses carry the «бейімд-» marker (v1.9.5).
- **Dialog `NOT_A_TOPIC` synced with reasoning closed-class** (v3.9.5) — one source of truth for "what is a content noun" across layers.
- **Action planning + verifier** (v4.0.31 → v4.0.32): `ActionPlanner::plan(intent, belief, task) → ActionPlan` classifies the chosen action; `Verifier::verify` gates evidence-bearing plans; misaligned `(plan, intent)` pairs strip injected evidence to fall back safely.
- **Epistemic status banding** (v4.0.33 → v4.0.34): `UncertaintyPolicy::derive` maps `(plan, verification, intent, belief)` → `EpistemicStatus { Certain, Supported, Derived, Tentative, Conflicted, Unknown }`; templates branch on the band.
- **Parse-failure path** (v4.0.40): input that doesn't reduce to a topic/evidence/chain routes to `AskClarification` (Tentative) instead of `RefuseOutOfScope` (Unknown). Distinct rationale strings make the trace tell auditors whether the dialog is asking "tell me more about X" vs "could you rephrase?".

### Belief layer (v4.0.27 → v4.1.0)
- `BeliefState`: structured fact log with `(subject, predicate, object)` schema, `ConfidenceBand`, `Provenance`, `FactStatus { Active, Superseded, Contested }`. Append-only — no fact is ever deleted; status flips preserve history.
- **Single-active-fact invariant** (v4.0.28): exactly one `Active` fact per `(subject, predicate)` at any time; new facts on the same slot supersede priors automatically.
- `BeliefConflict` log + `PendingQuestion::ContradictionToResolve` lifecycle: when a user re-states a slot with a different value, both facts go `Contested` and a pending question is raised.
- **Belief revision via user choice** (v4.1.0): `BeliefState::resolve_contradiction(subject, predicate, chosen_object)` flips the chosen value to `Active`, supersedes the rest, drops the matching `BeliefConflict` and `ContradictionToResolve` pending question. `Conversation::try_resolve_pending_contradiction` derives the chosen value from explicit `Statement*` intent (priority 1) or raw-input substring match (priority 2 — handles short replies like «астанада дұрыс»).

### Language Core (v4.3.0)
- `crates/adam-dialog/src/language_core.rs` — orthography + mixed-script (Latin/Cyrillic) cleanup + proper-noun normalization + canonical entity resolution. **Non-duplication rules**: morphology stays in `adam-kernel-fst`; geography stays in `data/world_core/geography_kz.jsonl`; normalization rules live here, not back in `semantics.rs`.
- **Canonical geography entities**: `canonical_geo_entity(surface) → GeoEntity { id, canonical, kind }` resolves any surface form (canonical name, Russian-form alias like `Алма-Ата`, descriptor phrase like `Каспий теңізі`, lowercase / mixed-case input) to a stable `geo_kz_NNN` id from `world_core/geography_kz.jsonl`. `canonical_geo_id`, `geo_entity_kind` are the leaner accessors.
- **`EntityMemory.canonical_id`** carries the id through `BeliefState`. Session adds `city_id` and `geo_kind` slots alongside the render-safe `city` string.
- **Geo-feature-aware response routing**: dialog templates branch on `geo_kind` so `теңіз` / `өзен` / `көл` / `тау` get appropriate framings (sea / river / lake / mountain).

### Typed Evidence (v4.3.0)
- `ToolResult.evidence: Vec<ToolEvidence>` carries machine-readable claims alongside textual `findings`.
- Variants: `BeliefFact { subject, predicate, object }`, `GraphFact { subject, predicate, object, confidence, rendered }`, `RetrievalSample { text }`, `DerivedFact { subject, predicate, object, rule_id, confidence, rendered, support_chain }`.
- The audit substrate for response-faithfulness checks: every dialog reply can be traced back to *which typed claim* justified it.

### Ontology gates (v4.3.0)
- `crates/adam-reasoning/src/ontology.rs` — type constraints for admissible symbolic facts.
- `validate_fact` / `validate_derived_fact_with_supports` reject `RulePredicateMismatch`, `PlaceObjectRequired` (spatial predicates need place-typed objects), `TimeLikeRequired` (temporal predicates need time-typed objects), `EmptySupportChain`, `SupportPatternMismatch`, `MissingSupportSource`.
- Graph and reasoner consumers reject structurally invalid facts before verbalisation.

### Response-quality audit (v4.3.0)
- `crates/adam-dialog/src/quality.rs` — deterministic response-quality gate. Catches machine-visible defects without judging open-ended "intelligence".
- `audit_response` rejects empty / placeholder-leaked / Latin-debug-artifact / double-space replies.
- `audit_trace_faithfulness` verifies surface-vs-trace consistency: the rendered reply must match the action and evidence the trace records.
- `audit_typed_faithfulness` verifies the surfaced answer is backed by the correct evidence class (graph fact vs retrieval sample vs rule-derived conclusion).
- `audit_graph_admissibility` runs ontology gates over a fact set and reports `GraphAdmissibilityIssue`s.

### Tool layer (v4.0.37 → v4.3.0)
- `Tool::dispatch(call, ctx) → ToolResult` — audit-mode tool path. Tools: `SearchBelief`, `SearchGraph`, `SearchRetrieval`, `RunLocalReasoner`. Distinct `ToolResult::empty` (ran fine, no findings) vs `ToolResult::unsupported` (couldn't run, missing context) semantics. `turn_with_trace` populates `tool_calls: Vec<ToolResult>` on `TurnTrace` for audit. Future minor will promote tools from audit to executive path.

### Cognitive evaluation harness (v4.0.34 → v4.4.5)
- `cognitive_eval` test in `crates/adam-dialog/tests/cognitive_eval.rs` runs `data/eval/cognitive_dialog_dataset.json` scenarios against the live `Conversation::turn` pipeline. Every scenario asserts `expect: { … }` (epistemic status, action, belief facts, contradiction count, etc.). `expected_failing: true` flags an aspirational scenario; harness reports which aspirationals are ready for promotion.
- **Baseline (v4.4.5): 54/54 canonical, 0/0 aspirational**. Growth log: v4.2.6 → 38/38 (multi-slot lifecycle + compound flows). v4.3.1 → 41/41 (person canonical entities). v4.3.2 → 42/42 (phantom-city false-positive fix). v4.3.3 → 44/44 (self/other distinction). v4.3.4 → 48/48 (SystemIdentity entity). v4.3.5 → 50/50 (topic-marker extraction + famous Kazakhs). v4.4.0 → 52/52 (belief-poisoning recovery). v4.4.5 → 54/54 (CheckContradiction renderer + AskAge self-recall). Per `CONTRIBUTING.md`, every dialog defect ships with at least one new scenario.

### REPL replay harness (v4.4.6+)
- `repl_replay` test in `crates/adam-dialog/tests/repl_replay.rs` runs `data/eval/repl_dialogs.json` dialogs through `Conversation::turn` with deterministic seeds, asserting on per-turn `output_contains_lower_any` / `output_not_contains_lower` substring expectations on the user-facing reply text. Complementary to `cognitive_eval` (which asserts on trace signals); this asserts on what the user actually sees.
- **Baseline (v4.4.6): 27 canonical + 3 aspirational dialogs across 11 categories**. The 3 aspirational document a real `statement_of_*` slot-echo gap (some variants don't interpolate the slot value); promotion-ready when every variant interpolates.

### Retrieval engine (v1.6.0+)
- Morpheme inverted index over the committed corpus, composite deterministic ranking (overlap + pack-purity + length + loanword-density), verbatim sample citation with `(pack, sample_id)` provenance.

### Reasoning engine (v2.1 → v4.3.0)
- **11 typed predicates**: IsA, LivesIn, Has, GoesTo, PartOf, RelatedTo, Causes, After, HasQuantity, DoesTo, InDomain.
- **11 FST-feature-checked pattern matchers** extracting facts from raw corpus with full `(pack, sample_id)` provenance and `ConfidenceKind::Grammar`.
- **Precision hardening** (v3.8.5 → v4.0): location allow-list, time-noun block, demonstrative closed-class, possessive-tainted object refusal, central `is_fragment_root` post-filter (`-`-prefixed). **v4.0 contradiction immune system**: `is_astronomical_object` helper blocks R6/R7 derivations against celestial-scale targets; object-side 3-char minimum in locative/dative matchers; 20+ new closed-class entries.
- **World Core** (v3.9.0+): human-authored Kazakh knowledge packs in `data/world_core/<domain>.jsonl` — **874 entries / 995 curated facts across 30 domains** (animals, astronomy, biology_basic, body_parts, clothing, colors, constellations_kz, cooking_methods, directions, emotions, food, geography_kz, house_parts, kinship_extended, kz_literature, language_features, materials, measurements, music_kz, notable_kazakhstanis, numbers, plants, professions, proverbs, society, sports, time, tools_household, transport, weather_phenomena), all `approved` by `shaman` and emitted with `ConfidenceKind::HumanApproved`. v4.3.5 added kz_literature surname-keyings + the new `notable_kazakhstanis` domain.
- **Forward-chaining reasoner**: **10 active rules** — R1 IsA-transitivity (633), R2 Has-inheritance (1 142), R3 Has-via-PartOf (55), R5 shared-IsA → RelatedTo (17 550), R6 LivesIn-via-PartOf (81), R7 GoesTo-via-PartOf (505), R8 After-transitivity (999), R9 PartOf-transitivity (175), R10 InDomain-inheritance (124), R11 InDomain-shared-target (151). R4 IsA-symmetry is curator-warning only. **21 415 derived facts on the committed runtime** over **15 521 extracted + curated facts**. Every derivation carries `rule_id` + non-empty `source_chain` + `ConfidenceKind::RuleInferred`.
- **Dialog integration** (v2.7+): rule-derived chains surface in `Intent::Unknown` with the mandatory «байланыс-» trust marker (test-enforced bi-directionally).

### Tooling + demos
- **`adam_chat`** — interactive REPL; autoloads retrieval index + reasoning artefacts.
- **`adam_demo`** — 4-part scripted walkthrough (intents + retrieval + composition + reasoning) for investor presentations.
- **`adam_inspect`** — "what does adam know about `<root>`?" query with Curated + Extracted split output (v3.9.0+).
- **`extract_facts`**, **`run_reasoner`**, **`build_lexical_graph`** — pipeline binaries with iteration-harness support (`--time-budget`, SIGINT → graceful commit, Rayon par_iter).
- **`validate_world_core`** — authoring gate for World Core entries (v3.9.0+).
- **`mine_lexicon_gaps`** — v3.4.0 Lexicon expansion pipeline; produces `docs/lexicon_gap_candidates.md` for native-speaker review.
- **`scaling_bench`** (v3.2.0) — deterministic scaling-law bench across 5 tiers, emits `data/scaling/scaling_report.json` + `docs/scaling_report.md`.

### Corpus
- Kazakh corpus at **4.57 M committed / 77.9 M local words** across 9 committed source packs: Tatoeba, Wikipedia KZ, Common Voice KK, CC-100, Abai Wikisource, proverbs, synthetic, Kazakh classics, Kazakh textbooks (10 books OCR'd via tesseract-kaz).
- **79.48 % morpheme coverage** of the committed pool (v1.5.5 audit baseline).

### Quality gates
- **Full regression test suite — 1 556 workspace tests passing as of v6.0.0-rc1, 0 failing, 2 ignored**. (Cumulative growth from 822 at v4.22.5 → 969 at v4.52.5 → 1 528 (mid-arc) → **1 556 at v6.0.0-rc1** with full L5.5/L6 wire-up, weather + clock + industry modules.)
- `scripts/validate_foundation.sh` — foundation CI (lex / FST / corpus / world_core / reasoner end-to-end).
- `scripts/verify_release_version.sh` — manifest-consistency gate (every committed JSON pinned to current crate version).
- `scripts/run_slow_roundtrip.sh` (v4.1.6+) — runs the four `#[ignore]`d FST synthesis-analysis roundtrip tests on demand.
- **Cognitive eval baseline 54 / 54 canonical, 0 / 0 aspirational** (v4.4.5). Tracks observable dialog behaviour at `(input → epistemic_status, action, belief_state, output)` level — distinct from unit tests, which gate state mechanics.
- **REPL replay baseline 27 / 27 canonical + 3 aspirational** (v4.4.6). Tracks user-facing surface text per turn — complementary to cognitive_eval which tracks trace signals.
- **Performance regression policy (v4.4.7+)**: `cargo bench -p adam-dialog --bench turn_latency` baseline in `docs/performance.md`; p50 regression > 20 % is a release blocker per `CONTRIBUTING.md`.
- **Stack contracts (v4.3.0)**: `crates/adam-eval/tests/rust_only_contracts.rs` rejects any non-Rust source file in the repository; `crates/adam-eval/tests/graph_first_contracts.rs` rejects external graph-stack markers (Cypher / SPARQL / Gremlin / Python graph libs) and verifies the canonical Rust graph entrypoints exist.

### Scope of "100 %" / "zero" claims

The wording adam uses for its quality numbers is deliberately scoped to avoid the marketing trap of overstating narrow benchmarks as general capability. Read these the right way:

- **"100 % tokenizer"** in the foundation report means **464 / 464 on the hand-authored segmentation eval** in `data/eval/tokenizer_segmentation_eval_dataset.json`. It is **not** a general "Kazakh tokenizer accuracy" benchmark — those would require a held-out segmented corpus, which we do not yet have.
- **"100 % training validation"** in the tiny-training prototype means **15 / 15 next-token validation checks** on the clean-pipeline prototype (`data/training/baseline_training_manifest.json`). It is **not** an ML-model accuracy claim.
- **`benchmark_manifest.json`** is a **coverage / contract benchmark manifest** (4 task families + guards + layers), not a single AI-benchmark score.
- **"Zero hallucination"** means **zero ungrounded generation inside the deterministic recognised / grounded runtime path** — the recogniser refuses or admits uncertainty (`unknown.tentative` / `unknown.conflicted`) outside the envelope. It is **not** a general open-domain hallucination benchmark.
- **Scaling report**: the T5 tier targeted 1 M samples but `status: "timed_out"` after scanning **940 288**. Useful as a scaling artefact (per-tier `facts_per_10k_words`, `derivations_per_fact`, `predicate_coverage_pct`); **not** a "1 M benchmark completed without caveat".

## Scope of the "FST-guaranteed" claim (accurate wording)

The FST synthesiser guarantees **grammatical correctness of slot-expanded template fragments** (e.g. `{city|locative}` → FST-synthesised `Алматыда`). Literal template text (e.g. `"сәлем"`, `"қайырлы таң"`) is pre-verified Kazakh committed in `data/dialog/templates/v1.toml`, not synthesised at runtime. Put together: no morphologically invalid word can leave the system through a slot path, and literal template strings are audited offline.

This is a weaker claim than "whole output is FST-guaranteed" — which would require every literal template token to pass back through the FST at runtime, which the current realiser does not do.

## Out of scope (permanent)

- **Multilingual input and output** — Kazakh only, both directions, by design.
- **Audio / speech / multimodal**.
- **Probabilistic free generation in the rule-based backbone** — predictability is the product.
- **Cloud orchestration** — runs entirely on a single developer's laptop.
- **Product UI** — `adam_chat` is the reference REPL; any UI is downstream work.

## Architectural stance (v4.x direction — committed)

adam is **not competing with ChatGPT on breadth.** It is becoming an **auditable Kazakh cognitive kernel** — narrower than an LLM, cheaper by orders of magnitude, but provably unable to hallucinate and capable of revising its beliefs on demand. Every output is one of:

1. A template realisation (recognised intent, 0 % fabrication).
2. A verbatim corpus quote (byte-identical to the source pack).
3. An FST-synthesised slot value (grammatically correct by construction).
4. A rule-derived chain with `rule_id` + non-empty `source_chain` + «байланыс-» marker.
5. A curated World Core fact with a named human reviewer.
6. A clarifying question raised by `ActionPlanner` when the planner refuses to answer on top of unresolved evidence (Tentative epistemic band) or an unresolved belief contradiction (Conflicted band).

Nothing else can leave the system. No free-text generator, no learned probability, no neural component in the runtime. See `project_retrieval_not_neural_v2` and `project_v4_direction` memories, plus [`docs/architecture_v3.md`](architecture_v3.md) for the v3.0 retrieval-era reference (still valid as the foundation underneath the v4.x belief / tool / cognitive-eval layers).

## v4.x targets — committed

- ✅ World Core to 500–1 000 entries across 10+ domains (delivered: **874 / 995 / 30 domains** at v4.4.7; expanded to **1626 / 1792 / 38 domains** by v4.17.5 — added physics_school / chemistry_school / biology_school / history_kazakhstan + adam_self).
- ✅ Belief layer with `BeliefState` lifecycle + contradiction logging (v4.0.27).
- ✅ Single-active-fact invariant (v4.0.28).
- ✅ Action planning + verifier + epistemic-status banding (v4.0.31 → v4.0.34).
- ✅ Cognitive eval harness with canonical / aspirational tracking (v4.0.34 → v4.0.36).
- ✅ Tool layer audit-mode dispatch (v4.0.37 → v4.0.39).
- ✅ Parse-failure path distinct from RefuseOutOfScope (v4.0.40).
- ✅ Belief revision via user choice — kernel signature feature (v4.1.0); cognitive baseline 22 / 22 canonical, 0 aspirational.
- ✅ Tools as execution — `inject_*` retired; `turn_with_trace` is a tool-loop interpreter (v4.2.0).
- ✅ Cognitive eval at 38/38 canonical (v4.2.6), AnswerDirect rendering closed + digit-token bug closed (v4.2.5).
- ✅ Language Core layer with canonical geography entity resolution (v4.3.0); typed `ToolEvidence` (v4.3.0); ontology gates rejecting structurally invalid facts (v4.3.0); response-quality audit (v4.3.0); Rust-only + Graph-first contract tests (v4.3.0).
- ✅ Person canonical entities (v4.3.1); phantom-city false-positive fix (v4.3.2); self/other distinction + SystemIdentity (v4.3.3–4); topic-marker extraction + famous Kazakhs (v4.3.5).
- ✅ Cognitive eval expansion to **54/54 canonical** (v4.4.5) — original 50+ target met and exceeded.
- ✅ Belief-poisoning recovery: `Action::DismissContradiction` + contradiction-priority cap (v4.4.0).
- ✅ Real-dialog adequacy fixes: `check_contradiction` template family + AskAge / AskOccupation 1sg-self-recall (v4.4.5–6).
- ✅ REPL replay battery for surface-text regressions (v4.4.6); `CONTRIBUTING.md` policy that every dialog defect ships with at least one new scenario or replay dialog.
- ✅ Performance baseline + bench harness + `> 20 % p50 regression = release blocker` policy (v4.4.7).

## v4.x next

- Hybrid surface layer (per `docs/language_core_hybrid_roadmap.md`): structured answer contract for any future generative verbalizer, with a verifier that rejects new facts/entities introduced by generation. Disabled by default until verification is stable.
- Organization canonical-entity layer (geography + person done; orgs next).
- Deterministic colloquial / typo alias guards on top of canonical geography.
- More belief-layer behaviours: revision-via-user-choice for non-`Statement*` intents, multi-conflict resolution in a single turn, conflict provenance display in `adam_chat --trace`.
- Outstanding REPL-replay-surfaced defects: `менің атым кім?` parsing "Кім" as a name → phantom contradiction (queued for v4.4.8 or v4.4.9).
- Promote 3 aspirational REPL-replay dialogs to canonical by tightening `statement_of_*` template families so every variant interpolates the slot.
- Continued Lexicon + corpus expansion in the v1.x / v3.x cadence (one new domain or rule per patch, per `project_v4_direction`).
