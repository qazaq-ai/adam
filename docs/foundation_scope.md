# Foundation Scope

## Goal

Deliver a **predictable, auditable Kazakh reasoning engine** built entirely in Rust and runnable on a MacBook Air M2 8 GB. Every layer's decision must be traceable. No probabilistic free generation in the recognised-intent path. **Not** an LLM clone — intentionally narrower, intentionally cheaper, intentionally provenance-first.

## In scope (v1.0.0 → v3.9.5 delivered)

### Morphology + Lexicon
- Pure Kazakh **Lexicon** (~14 k roots: curated + Apertium; v2.2 purged 87 intervocalic-voicing-duplicate pollutions; v3.2.0 dual-storage for deterministic iteration).
- Deterministic **FST morphology** (phonology + morphotactics + inverse parser; v2.3 glide-vowel classification fix for у/и/ю; v3.8.0 verb-root stem vs infinitive fix).
- **Kazakh-only recogniser surface** (v0.9.6 trilingual experiment reverted in v1.1.0; Latin transliteration removed).

### Dialog layer
- **26-intent dialog pipeline** with multi-turn session state + follow-up resolution (v1.4.0).
- **FST-backed slot expansion** (`{slot|features}` — case × number × derivation × possessive × predicate-person).
- Template repository as external TOML data (**34 families as of v3.9.5**).
- **Session-aware composition** (v1.8.0+): frame around retrieved quote personalises when session has name/city/age/occupation.
- **Opt-in city swap** (v1.9.0+): `ComposeMode::InSampleCitySwap` rewrites city mentions via FST feature-preserving synthesis, year-guarded; adapted responses carry the «бейімд-» marker (v1.9.5).
- **Dialog `NOT_A_TOPIC` synced with reasoning closed-class** (v3.9.5) — one source of truth for "what is a content noun" across layers; closes the «Неліктен → Нелікте тұрасыз ба» misparse.

### Retrieval engine (v1.6.0+)
- Morpheme inverted index over the committed corpus, composite deterministic ranking (overlap + pack-purity + length + loanword-density), verbatim sample citation with `(pack, sample_id)` provenance.

### Reasoning engine (v2.1 → v3.9.5)
- **11 typed predicates**: IsA, LivesIn, Has, GoesTo, PartOf, RelatedTo, Causes, After, HasQuantity, DoesTo, InDomain.
- **11 FST-feature-checked pattern matchers** extracting facts from raw corpus with full `(pack, sample_id)` provenance and `ConfidenceKind::Grammar`.
- **Precision hardening** (v3.8.5 + v3.9.0): location allow-list, time-noun block, demonstrative closed-class, possessive-tainted object refusal, central `is_fragment_root` post-filter (refuses `-`-prefixed fragment roots).
- **World Core** (v3.9.0+): human-authored Kazakh knowledge packs in `data/world_core/<domain>.jsonl` — **200 entries / 270 curated facts as of v3.9.5** across 6 domains (astronomy, time, geography_kz, biology_basic, body_parts, society), emitted with `ConfidenceKind::HumanApproved` — the exclusive tier marking human-reviewed claims.
- **Forward-chaining reasoner**: 5 active rules (R1 IsA-transitivity, R2 Has-inheritance, R3 Has-via-PartOf, R5 shared-IsA → RelatedTo, R6 LivesIn-via-PartOf, R7 GoesTo-via-PartOf). R4 IsA-symmetry is curator-warning only. Every derivation carries a `rule_id` + non-empty `source_chain` + `ConfidenceKind::RuleInferred`.
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
- **Full regression test suite (440 workspace tests as of v3.9.5, 0 failing, 0 warnings)** + `scripts/validate_foundation.sh` foundation CI + `scripts/verify_release_version.sh` manifest-consistency gate.

## Scope of the "FST-guaranteed" claim (accurate wording)

The FST synthesiser guarantees **grammatical correctness of slot-expanded template fragments** (e.g. `{city|locative}` → FST-synthesised `Алматыда`). Literal template text (e.g. `"сәлем"`, `"қайырлы таң"`) is pre-verified Kazakh committed in `data/dialog/templates/v1.toml`, not synthesised at runtime. Put together: no morphologically invalid word can leave the system through a slot path, and literal template strings are audited offline.

This is a weaker claim than "whole output is FST-guaranteed" — which would require every literal template token to pass back through the FST at runtime, which the current realiser does not do.

## Out of scope (permanent)

- **Multilingual input and output** — Kazakh only, both directions, by design.
- **Audio / speech / multimodal**.
- **Probabilistic free generation in the rule-based backbone** — predictability is the product.
- **Cloud orchestration** — runs entirely on a single developer's laptop.
- **Product UI** — `adam_chat` is the reference REPL; any UI is downstream work.

## Architectural stance (v3.9.0+ direction — committed)

adam is **not competing with ChatGPT on breadth.** It is becoming an **auditable Kazakh reasoning engine** — narrower than an LLM, cheaper by orders of magnitude, but provably unable to hallucinate. Every output is one of:

1. A template realisation (recognised intent, 0 % fabrication).
2. A verbatim corpus quote (byte-identical to the source pack).
3. An FST-synthesised slot value (grammatically correct by construction).
4. A rule-derived chain with `rule_id` + non-empty `source_chain` + «байланыс-» marker.
5. A curated World Core fact with a named human reviewer.

Nothing else can leave the system. No free-text generator, no learned probability, no neural component in the runtime. See the `project_retrieval_not_neural_v2` memory and [`docs/architecture_v3.md`](architecture_v3.md) for details.

## v4.0 target — investor-ready MVP

- Expand World Core to 500–1 000 entries across 10+ domains (numbers, colors, kz_literature, food, clothing).
- Full scripted investor demo (`adam_demo_v4` — single command, single narrative, ~3-minute screencast).
- Native-speaker review workflow (web UI for community contributions).
- `validate_world_core` integrated into `validate_foundation.sh` as a CI gate.
- R4 activation (diagnostic surface for IsA symmetry, curator warning).
