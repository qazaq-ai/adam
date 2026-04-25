# Repository Layout

## Crates (10 total — workspace at v4.1.0)

- `adam-kernel`
  shared identity, versioning, and foundation contracts (L0)
- `adam-kernel-fst`
  **FST morphology** (L0): phonology (11 archiphonemes + 22+ twol rules; v2.3 glide-vowel fix), morphotactics (36 suffix templates incl. v1.4.0 predicate-person copula; v3.8.0 verb-root stem fix), synthesiser + inverse parser, dual-storage Lexicon loader (v3.2.0 determinism fix), `adam_fst` CLI
- `adam-tokenizer`
  pre-tokenizer + BPE trainer + encoder / decoder (L1)
- `adam-corpus`
  source acceptance, streaming processors (Wikipedia KZ, CC-100, Tatoeba, Common Voice, classics, **10 Kazakh textbooks** via `process_kazakh_textbooks` + tesseract-kaz OCR pipeline, v3.3.0 – v3.5.0), `corpus_audit`, `morpheme_coverage` (v1.5.5), `mine_lexicon_gaps` (v3.4.0), synthetic-sentence generation (L1)
- `adam-eval`
  evaluation suite, benchmark manifests, delta reports (L1)
- `adam-dialog`
  **dialog pipeline** (L1): 26-intent recognisers, `Conversation` session state + DST (`active_intent`, `intent_history`), session-aware template planner, slot-expanding realiser, `adam_chat` REPL + `adam_demo` scripted walkthrough + `adam_inspect` interactive knowledge query (v3.7.0+). `NOT_A_TOPIC` closed-class filter synced with reasoning crate (v3.9.5). **`BeliefState` belief layer** with `(Active, Superseded, Contested)` lifecycle + `BeliefConflict` log + `PendingQuestion` lifecycle (v4.0.27); single-active-fact invariant (v4.0.28); `ActionPlanner` + `Verifier` + `UncertaintyPolicy` (v4.0.31 → v4.0.34); `Tool::dispatch` audit-mode path (v4.0.37 → v4.0.39); **`BeliefState::resolve_contradiction` + `Conversation::try_resolve_pending_contradiction`** for user-driven belief revision (v4.1.0). Cognitive eval harness in `tests/cognitive_eval.rs` (22 / 22 canonical, 0 aspirational).
- `adam-retrieval` (v1.6.0+)
  **retrieval engine** (L1): `MorphemeIndex` inverted index, composite deterministic `rank` (v1.7.0), `compose::compose_with_city` opt-in in-sample swap with year guard + «бейімд-» marker (v1.9.0 – v1.9.5), `build_morpheme_index` binary
- `adam-reasoning` (v2.1 → v4.1.0)
  **ILMRR — reasoning engine** (L1): 11 FST-feature-checked pattern matchers (all 11 declared predicates), central `is_fragment_root` hygiene gate (v3.9.0), Lexical Graph projection (v2.3+), forward-chaining reasoner with **10 active rules** — R1 IsA-transitivity, R2 Has-inheritance, R3 Has-via-PartOf, R5 shared-IsA → RelatedTo, R6 LivesIn-via-PartOf, R7 GoesTo-via-PartOf, R8 After-transitivity, R9 PartOf-transitivity, R10 InDomain-inheritance, R11 InDomain-shared-target. R4 IsA-symmetry curator-warning only. **World Core** human-authored knowledge packs with `ConfidenceKind::HumanApproved` exclusive tier (v3.9.0+), iteration harness (v3.1.0+), binaries: `extract_facts`, `run_reasoner`, `build_lexical_graph`, `validate_world_core`
- `adam-scaling` (v3.2.0+)
  **scaling-law bench** (L1): deterministic 5-tier bench across committed + shard pool, `scaling_bench` + `audit_precision` binaries, normalized metrics (facts/10k-words, derivations/fact, predicate-coverage %, duplicate-rate %)
- `adam-train`
  legacy transformer baseline (v0.1 – v0.4); kept for CI regression, not on the current codepath (L2)

## Data

- `data/raw/`
  source registry, scoring rules (policy inputs only, no raw text)
- `data/external/`
  raw fetched text from authentic Kazakh sources (gitignored if > 50 MB)
- `data/curated/`
  per-source Kazakh-only packs + training / validation manifests (see `data/curated/README.md`)
- `data/lexicon_v1/`
  the authoritative Lexicon: 13 606 pure Kazakh + 11 919 Apertium imports (~25.5 k roots); v2.2 purged intervocalic-voicing-duplicate Apertium pollutions; see `data/lexicon_v1/README.md`
- `data/tokenizer/`
  BPE vocab + merges + segmentation rules + curated root list
- `data/dialog/`
  dialog-layer template repository (`templates/v1.toml`, 34+ families incl. v1.8.0 session-aware evidence, v1.9.5 adapted-evidence, v2.7 `unknown.with_derived_chain`, v4.0.34 Tentative / Conflicted families for epistemic banding; see `data/dialog/README.md`)
- `data/world_core/` (v3.9.0+)
  human-authored Kazakh knowledge packs — **29 domains, 826 entries / 922 curated facts** at v4.1.0: animals, astronomy, biology_basic, body_parts, clothing, colors, constellations_kz, cooking_methods, directions, emotions, food, geography_kz, house_parts, kinship_extended, kz_literature, language_features, materials, measurements, music_kz, numbers, plants, professions, proverbs, society, sports, time, tools_household, transport, weather_phenomena. All `approved` by `shaman`. Schema + authoring guide in `data/world_core/README.md`
- `data/retrieval/`
  morpheme inverted index (v1.6.0+), committed `facts.json` (v2.1+) — **mixed source at v3.9.0+** (text-extracted `Grammar` 14 526 facts + curated `HumanApproved` 922 facts = 15 448 total), `lexical_graph.json` (v2.3+), `derived_facts.json` (v2.4+; **17 340 derivations from 10 active rules at v4.1.0**)
- `data/eval/cognitive_dialog_dataset.json`
  **22 canonical scenarios, 0 aspirational** (v4.1.0). Drives the `cognitive_eval` harness test in `crates/adam-dialog/tests/cognitive_eval.rs`. Each scenario is a turn-list + an `expect: { … }` block (epistemic_status, action, belief facts, contradiction count, etc.)
- `data/scaling/` (v3.2.0+)
  `scaling_report.json` across 5 tiers from `adam-scaling::scaling_bench`
- `data/eval/`
  benchmark + tokenizer-experiment manifests, held-out datasets, delta reports
- `data/training/`
  legacy transformer artifacts (v0.4.0 checkpoint + assembly / consistency / delta reports); see `data/training/README.md`
- `data/foundation/`
  top-level foundation-overview report aggregating every layer

See [data/README.md](../data/README.md) for load-bearing / generatable flagging per subdirectory.

## Scripts

- `scripts/validate_foundation.sh` — full CI-level validation; runs every layer's regression tests.
- `scripts/bump_foundation_version.sh <x.y.z>` — workspace version bump with drift verification.
- `scripts/cut_release.sh` — create / tag / push a release from a clean working tree.
- `scripts/verify_release_version.sh` — enforce strict `x.y.z` tag format.
- `scripts/fetch_*.sh` — fetch external sources (Tatoeba, Wikipedia KZ, Common Voice KK, CC-100, Abai Wikisource).
- `scripts/run_*.sh` — per-stage regenerators (synth sentences, BPE training, encode, evaluation, foundation-overview assembly).

## Documentation

- `docs/roadmap.md` — version-by-version history, lifecycle view of the two eras.
- `docs/kazakh_grammar/` — linguistic reference (phonology / morphology / syntax / lexicon sources / work plan / Apertium twol catalogue) + the dialog architecture spec.
- `docs/corpus_policy.md`, `docs/corpus_sources.md`, `docs/curation_workflow.md` — corpus curation.
- `docs/tokenizer_policy.md`, `docs/tokenizer_experiment_plan.md` — tokenizer.
- `docs/evaluation_policy.md`, `docs/eval_baseline.md` — evaluation.
- `docs/training_baseline.md` — legacy transformer baseline context.
- `docs/foundation_scope.md` — overall project scope.
- `docs/source_classification.md`, `docs/source_scoring.md` — source-quality framework.
