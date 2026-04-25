# Foundation Scope

## Goal

Deliver a **predictable, auditable Kazakh reasoning engine** built entirely in Rust and runnable on a MacBook Air M2 8 GB. Every layer's decision must be traceable. No probabilistic free generation in the recognised-intent path. **Not** an LLM clone — intentionally narrower, intentionally cheaper, intentionally provenance-first.

## In scope (v1.0.0 → v4.1.0 delivered)

### Morphology + Lexicon
- Pure Kazakh **Lexicon** (~25.5 k roots: 13 606 pure Kazakh + 11 919 Apertium imports; v2.2 purged intervocalic-voicing-duplicate pollutions; v3.2.0 dual-storage for deterministic iteration).
- Deterministic **FST morphology** (phonology + morphotactics + inverse parser; v2.3 glide-vowel classification fix for у/и/ю; v3.8.0 verb-root stem vs infinitive fix).
- **Kazakh-only recogniser surface** (v0.9.6 trilingual experiment reverted in v1.1.0; Latin transliteration removed).

### Dialog layer
- **26-intent dialog pipeline** with multi-turn session state + follow-up resolution (v1.4.0).
- **FST-backed slot expansion** (`{slot|features}` — case × number × derivation × possessive × predicate-person).
- Template repository as external TOML data (34+ families; v4.0.34 added Tentative / Conflicted families for epistemic banding).
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

### Tool layer (v4.0.37 → v4.0.39)
- `Tool::dispatch(call, ctx) → ToolResult` — audit-mode tool path. Tools: `SearchBelief`, `SearchGraph`, `SearchRetrieval`, `RunLocalReasoner`. Distinct `ToolResult::empty` (ran fine, no findings) vs `ToolResult::unsupported` (couldn't run, missing context) semantics. `turn_with_trace` populates `tool_calls: Vec<ToolResult>` on `TurnTrace` for audit. Future minor will promote tools from audit to executive path.

### Cognitive evaluation harness (v4.0.34 → v4.1.0)
- `cognitive_eval` test in `crates/adam-dialog/tests/cognitive_eval.rs` runs `data/eval/cognitive_dialog_dataset.json` scenarios against the live `Conversation::turn` pipeline. Every scenario asserts `expect: { … }` (epistemic status, action, belief facts, contradiction count, etc.). `expected_failing: true` flags an aspirational scenario; harness reports which aspirationals are ready for promotion.
- **Baseline (v4.1.0): 22/22 canonical, 0/0 aspirational**. Both Codex strategic-review aspirationals closed (parse-failure distinction in v4.0.40, contradiction resolution via user choice in v4.1.0).

### Retrieval engine (v1.6.0+)
- Morpheme inverted index over the committed corpus, composite deterministic ranking (overlap + pack-purity + length + loanword-density), verbatim sample citation with `(pack, sample_id)` provenance.

### Reasoning engine (v2.1 → v4.1.0)
- **11 typed predicates**: IsA, LivesIn, Has, GoesTo, PartOf, RelatedTo, Causes, After, HasQuantity, DoesTo, InDomain.
- **11 FST-feature-checked pattern matchers** extracting facts from raw corpus with full `(pack, sample_id)` provenance and `ConfidenceKind::Grammar`.
- **Precision hardening** (v3.8.5 → v4.0): location allow-list, time-noun block, demonstrative closed-class, possessive-tainted object refusal, central `is_fragment_root` post-filter (`-`-prefixed). **v4.0 contradiction immune system**: `is_astronomical_object` helper blocks R6/R7 derivations against celestial-scale targets; object-side 3-char minimum in locative/dative matchers; 20+ new closed-class entries.
- **World Core** (v3.9.0+): human-authored Kazakh knowledge packs in `data/world_core/<domain>.jsonl` — **826 entries / 922 curated facts across 29 domains** (animals, astronomy, biology_basic, body_parts, clothing, colors, constellations_kz, cooking_methods, directions, emotions, food, geography_kz, house_parts, kinship_extended, kz_literature, language_features, materials, measurements, music_kz, numbers, plants, professions, proverbs, society, sports, time, tools_household, transport, weather_phenomena), all `approved` by `shaman` and emitted with `ConfidenceKind::HumanApproved`.
- **Forward-chaining reasoner**: **10 active rules** — R1 IsA-transitivity (574), R2 Has-inheritance (1 110), R3 Has-via-PartOf (55), R5 shared-IsA → RelatedTo (13 566), R6 LivesIn-via-PartOf (81), R7 GoesTo-via-PartOf (505), R8 After-transitivity (999), R9 PartOf-transitivity (175), R10 InDomain-inheritance (124), R11 InDomain-shared-target (151). R4 IsA-symmetry is curator-warning only. **17 340 derived facts on the committed runtime.** Every derivation carries `rule_id` + non-empty `source_chain` + `ConfidenceKind::RuleInferred`.
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
- **Full regression test suite — 577 workspace tests passing as of v4.1.0, 0 failing, 0 warnings**.
- `scripts/validate_foundation.sh` — foundation CI (lex / FST / corpus / world_core / reasoner end-to-end).
- `scripts/verify_release_version.sh` — manifest-consistency gate (every committed JSON pinned to current crate version).
- **Cognitive eval baseline 22 / 22 canonical, 0 / 0 aspirational** (v4.1.0). Tracks observable dialog behaviour at `(input → epistemic_status, action, belief_state)` level — distinct from unit tests, which gate state mechanics.

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

- ✅ World Core to 500–1 000 entries across 10+ domains (delivered: 826 / 922 / 29 domains).
- ✅ Belief layer with `BeliefState` lifecycle + contradiction logging (v4.0.27).
- ✅ Single-active-fact invariant (v4.0.28).
- ✅ Action planning + verifier + epistemic-status banding (v4.0.31 → v4.0.34).
- ✅ Cognitive eval harness with canonical / aspirational tracking (v4.0.34 → v4.0.36).
- ✅ Tool layer audit-mode dispatch (v4.0.37 → v4.0.39).
- ✅ Parse-failure path distinct from RefuseOutOfScope (v4.0.40).
- ✅ Belief revision via user choice — kernel signature feature (v4.1.0); cognitive baseline 22 / 22 canonical, 0 aspirational.

## v4.x next

- Tools as execution: replace `inject_*` with `Tool::dispatch` as the primary path, not just audit. Removes the audit / runtime split and lets the planner branch on tool results directly.
- Cognitive eval expansion to 50+ scenarios including tool-driven cases.
- More belief-layer behaviours: revision-via-user-choice for non-`Statement*` intents (free-form clarifications), multi-conflict resolution in a single turn, conflict provenance display in `adam_chat --trace`.
- Continued Lexicon + corpus expansion in the v1.x / v3.x cadence (one new domain or rule per patch, per `project_v4_direction`).
