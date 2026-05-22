# Intelligence Roadmap — Path to Human-Like Dialog

Status: **active**, updated 2026-05-16.

This document is the strategic plan for moving `adam` from "deterministic, traceable, but narrow" toward "deterministic, traceable, and conversationally adequate". It complements:
- [`docs/roadmap.md`](roadmap.md) — release-by-release history.
- [`docs/language_core_hybrid_roadmap.md`](language_core_hybrid_roadmap.md) — Language Core / Hybrid Surface Layer.
- [`docs/foundation_scope.md`](foundation_scope.md) — current capability inventory.
- [`docs/MANIFESTO.md`](MANIFESTO.md) — the four-inversion architectural position (algebra not statistics, CPU not cloud, verifier not RLHF, agglutinative-first).
- [`docs/architecture_neural_v6.md`](architecture_neural_v6.md) — the v6.0.0 production architecture spec for the algebra-anchored neural composition layer.

It does **not** propose abandoning the deterministic stack. Every gap below is closeable inside the existing Rust-only / graph-first architecture; the **v6.0 neural composition layer is additive** — it improves surface realisation under the same verifier gate, never replacing the deterministic path. See [`architecture_neural_v6.md`](architecture_neural_v6.md) §2 (pipeline) and §4 (failure modes).

## What the v4.3.1 dialog test revealed

A real test session (user input shared 2026-04-26) surfaced four classes of deficit. They are independent, addressable, and stack into a clear next-quarter program:

1. **Brittle entity extraction** — the v4.3.2 patch fixed one substring-match false positive (`интеллект` → phantom city `Жасанды`). The class of bug is bigger: any short stem in a substring check is a hazard. We need extraction filters that score candidates instead of accepting the first hit.
2. **No self/other distinction** — `сен кімсің?` (who are you, addressed to adam) and `менің атым кім?` (what is my name) collapse to the same `AskName` intent. Adam answers with the user's stored name in both cases. Adam should know it has its own identity that is distinct from what it remembers about the user.
3. **No recovery from a poisoned belief** — once a contradiction is logged (real or phantom), the planner routes every subsequent turn to `CheckContradiction`. There is no "neither, drop both", no "actually I was wrong, let's start over", no automatic confidence decay.
4. **Knowledge breadth too narrow for free-form questions** — `Қазақстан туралы не білесіз` / `Ресей дегеніміз не` / `Абай жайында не дейсің` get generic refusals. The world_core has facts about Kazakhstan and Russia, but the dialog's `SearchGraph` path doesn't surface them on bare topic questions.

## Five tracks toward intelligence

Each track is independent. Patches against any track are mergeable without touching the others.

### Track A — Stronger entity extraction

Goal: never log a fact the user did not assert.

- **Drop short-stem substring matchers**. v4.3.2 closed one (`ел`); audit `semantics.rs` for any other `token.contains(short_stem)` patterns and migrate to prefix / suffix / known-suffix-list matching.
- **Confidence-aware acceptance**. Today every Statement* intent that fires writes a `ConfidenceBand::Confirmed` belief fact. We can introduce a softer band — `Tentative` — for extractions that fired through string heuristics rather than full FST + lexicon agreement. Tentative facts surface in trace but do not block topic queries until the user re-confirms.
- **Multi-feature gating**. Today the bare-Locative-as-city path requires *either* a known place via `canonical_geo_entity` *or* `тұрамын/тұрамыз` co-occurrence. Tighten to **always** require one of: known place, explicit live-verb co-occurrence, or canonical alias from `language_core`. No more "any noun in locative case is a city".
- **Compound expression awareness**. `жасанды интеллект`, `машиналық оқыту`, `ұялы телефон` should be recognized as compound concepts before per-word parsing; otherwise their components keep producing parser surprises.

### Track B — Self / other distinction

Goal: adam knows there are two entities in the conversation: itself and the user.

- **Two new intents**: `AskAboutSystem` (who are you / what is your name / what are you, addressed to adam) and `AskAboutUser` (what is my name / where do I live, addressed about the user). Triggers branch on Kazakh second-person pronouns + person markers.
- **System self-card**: a small immutable record adam carries about itself (`name = адам`, `kind = тілдік модель`, `purpose = қазақ тілінде сөйлесу`). Not stored in `BeliefState` (which is the user's data); a separate `SystemIdentity` struct on the `Conversation`.
- **Planner branch**: `AskAboutSystem` → render from `SystemIdentity`, never from belief. `AskAboutUser` → existing belief-direct-answer path.
- **Cognitive eval scenarios**: pin "сен кімсің?" → adam-self answer (not user's name); pin "менің атым кім?" → user's stored name.

### Track C — Belief revision under pressure

Goal: a contradiction must be a recoverable state, not a permanent block.

- **`Action::DismissContradiction`**: a new action that drops both contested values back to `Superseded` without choosing one. Triggered by user saying "екеуі де жоқ" / "ешқайсысы дұрыс емес" / "білмеймін" / "өткізіп жібер" in response to a `CheckContradiction` prompt. Mirror of the v4.1.0 `resolve_contradiction` but in the "neither" direction.
- **Confidence decay on repeat refusal**. If `CheckContradiction` fires on the same `(subject, predicate)` for N consecutive turns and the user keeps changing topic without resolving, demote both contested facts to `Superseded` automatically and clear the contradiction. The trace records the demotion so it is auditable.
- **Per-turn extraction veto**. After v4.3.2 the dialog still records every Statement* fact it can pull. A user who sees a wrong record in `adam_chat --trace` should be able to say "жоқ, мен X емеспін" / "оны жоғалт" and have the corresponding fact go to `Superseded` immediately.
- **Contradiction-priority cap**. The planner's "contradiction dominates" rule (action.rs step 1) becomes "contradiction dominates *for at most K turns*"; after K, fall through to other branches even if `belief.contradictions` is non-empty. Surfaces the conflict but does not block other capability indefinitely.

### Track D — Knowledge breadth via world_core + retrieval routing

Goal: free-form topic questions get a real answer when adam has the data.

- **Topic-question router**. New planner branch on `Intent::Unknown { noun_hint: Some(topic) }`: dispatch `SearchGraph { subject: topic, predicate: None }` first; if it returns active facts, render a "X is a Y" / "X has Z" sentence per typed evidence. Falls through to the v1.6.0 retrieval path only when graph is empty.
- **World Core expansion**. The bottleneck for "free-form Kazakhstan questions" is curated facts on Kazakhstan, Russia, Abai, etc. — not the engine. Add per-domain entries that answer common dialog questions: `Қазақстан Іс_қ_Орталық_Азиядағы_мемлекет`, `Ресей көрші_мемлекет`, `Абай ақын_жазушы`, etc. (per `data/world_core/geography_kz.jsonl` schema, every entry approved by the user.)
- **Retrieval-driven topic answers**. The morpheme index already carries `wiki_kz` and `kazakh_classics` packs; `SearchRetrieval` can return a paragraph-length quote on `Абай`. Current code routes this only when reasoning chain is absent. Extend the planner to surface a short retrieved sentence whenever (a) graph has nothing, (b) reasoner has nothing, (c) retrieval has a high-confidence hit.
- **Multi-hop graph queries**. Today `SearchGraph` is single-step. A future patch can add `(subject) -[predicate]-> (?) -[predicate2]-> (?)` chains so questions like "where does the president of Kazakhstan live" decompose deterministically.

### Track E — Lexicon and corpus growth

Goal: every common modern Kazakh word parses correctly.

- **Lexicon backlog**. `интеллект` had no FST analysis at all in the v4.3.1 test. Modern domain vocabulary (technology, science, economy) is the biggest visible coverage gap. Per `project_morpheme_coverage_baseline` (v1.5.5: 79.48 %), the next coverage audit (Track E.1) will name the top-K gaps.
- **Disambiguation hints**. The FST returns multiple parses per word; `parse_input` keeps only the first. A small ranker — based on POS frequency, sentence context, neighbour-token agreement — would let the dialog pick the right one when surface ambiguity exists.
- **Compound entries**. As Track A noted, `жасанды интеллект` deserves a single multi-word lexicon entry, not two bare adjectives. Multi-token entries already exist in geography (`Каспий теңізі`); generalize the same mechanism for technology / science domains.
- **Curated corpus expansion** (per `project_corpus_purity_directive`): each new pack must pass loanword filter before ingestion. Per `project_v4_direction`: prefer curated / synthetic over raw web crawl. The `world_core` is the canonical surface; the corpus is the secondary support.

## Sequencing

The five tracks parallelise, but a sane single-developer cadence is:

| Phase | Patch level | Tracks |
|---|---|---|
| **Phase 1** (next 2-3 patches) | v4.3.x | Track A (extraction filters) + Track B (self/other) — both are bug-density wins, both are bounded scope. |
| **Phase 2** | v4.4.x minor | Track C (belief recovery) — needs new Action variant, planner branch, templates. Adds a minor's worth of capability. |
| **Phase 3** | v4.5.x → v4.6.x | Track D (knowledge breadth) — World Core expansion is data work the user can do; SearchGraph routing is a new planner branch. |
| **Phase 4** | v4.7.x onwards | Track E (lexicon + corpus growth) — slow, steady, multi-release. |

## Non-goals

This roadmap explicitly does **not** plan:

- A trained neural language model in the runtime. Per `project_retrieval_not_neural_v2` and `project_v4_direction`, the answer path stays deterministic. A constrained generative verbalizer is a Workstream-D future item in the language-core roadmap, fenced by ontology + faithfulness gates.
- Multilingual surfaces. Per `project_kazakh_only_directive`, Kazakh-only stays.
- Foreign-language runtime components. Per the v4.3.0 Rust-only contract test, this is enforced.
- Investor-demo framing. Per `project_v4_direction`, the framing is "intelligence, not investor readiness".

## Why this is reachable inside the deterministic architecture

A common counter-argument: "to be conversational, you need a probabilistic model". The five tracks above fall into two buckets:

- **Engineering hygiene** (A, C, parts of D) — better filters, more recovery options, more planner branches. Pure deterministic improvements; no probabilistic component required.
- **Knowledge breadth** (D, E) — more facts, more lexicon entries, more compound expressions. Per the v4.3.0 ontology gates, every new fact is type-validated; per v3.9.0+ World Core conventions, every new entry is human-reviewed. The expansion is bounded by curation throughput, not by architectural limits.

The system that emerges at the end of this roadmap is still the v4.3.0 stack — same ToolEvidence, same audits, same Rust-only / graph-first contracts — but with enough coverage and enough recovery to hold a believable Kazakh dialog without fabrication. That is the intelligence we are reaching for.
