# Foundation Scope

## Goal

Deliver a **predictable, auditable Kazakh dialog system** built entirely in Rust and runnable on a MacBook Air M2 8 GB. Every layer's decision must be traceable. No probabilistic free generation in the recognised-intent path.

## In Scope (v1.0.0 → v1.3.x delivered)

- Pure pre-modern Kazakh Lexicon (14 k curated roots, loanwords purged)
- Deterministic FST morphology (phonology + morphotactics + inverse parser)
- 26-intent dialog pipeline (25 conversational + Insult) with multi-turn session state
- **Kazakh-only recogniser surface** (the v0.9.6 trilingual experiment was reverted in v1.1.0; Latin transliteration removed)
- FST-backed slot expansion for grammatical agreement (case, number, derivation, possessive)
- Template repository as external TOML data (editable without recompiling)
- Smart Unknown handler — FST-parsed noun hint feeds context-aware fallback
- Kazakh corpus (v1.x expansion toward 100 M+ words): Wikipedia full dump (committed first shard + local shards 02–10), Wikisource classics, Tatoeba, Common Voice, CC-100, Abai Wikisource, proverbs
- Full regression test suite and foundation CI

## Scope of "FST-guaranteed" claim (accurate wording)

The FST synthesiser guarantees **grammatical correctness of slot-expanded template fragments** (e.g. `{city|locative}` → FST-synthesised `Алматыда`). Literal template text (e.g. `"сәлем"`, `"қайырлы таң"`) is pre-verified Kazakh committed in `data/dialog/templates/v1.toml`, not synthesised at runtime. Put together: no morphologically invalid word can leave the system through a slot path, and literal template strings are audited offline.

This is a weaker claim than "whole output is FST-guaranteed" — which would require every literal template token to pass back through the FST at runtime, which the current realiser does not do. Stated accurately for v1.3.x+.

## Out of Scope (permanent)

- **Multilingual input and output** — Kazakh only, both directions, by design
- **Audio / speech / multimodal**
- **Probabilistic free generation in the rule-based backbone** — predictability is the product
- **Cloud orchestration** — runs entirely on a single developer's laptop
- **Product UI** — `adam_chat` is the reference REPL; any UI is downstream work

## v2.0 direction — retrieval, not neural generation

The v2.0 "minimally thinking Kazakh model" is **not** a trained transformer LM. The approach is morpheme-indexed retrieval over the 100 M+ word corpus plus rule-based compositional synthesis. Reasons: predictability, zero hallucinations, cheap (M2 8 GB), safe (cannot say what isn't in the corpus), and exploits Kazakh's rich agglutinative morphology that the FST already unpacks. See `project_retrieval_not_neural_v2` memory and the roadmap for details.

## Out of Scope (near-term, v1.4.0+)

- Native-speaker review of the template set
- Lexicon expansion: proper nouns (place names), modern-vocabulary tier
- FST predicate-person markers (1sg / 2sg predicate copulas) — needed for full FST-NER refactor
- Verb slot expansion in templates (`{root|verb_features}`)
- Intents beyond the current 26-intent surface
- DST (`Conversation` → `ContextStack` with active_intent + history)
