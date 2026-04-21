# Foundation Scope

## Goal

Deliver a **predictable, auditable Kazakh dialog system** built entirely in Rust and runnable on a MacBook Air M2 8 GB. Every layer's decision must be traceable. No probabilistic free generation in the recognised-intent path.

## In Scope (v1.0.0 → v2.3 delivered)

- Pure Kazakh Lexicon (~16.4 k roots: curated + Apertium; v2.2 purged 87 intervocalic-voicing-duplicate pollutions)
- Deterministic FST morphology (phonology + morphotactics + inverse parser; v2.3 glide-vowel classification fix for у/и/ю)
- 26-intent dialog pipeline with multi-turn session state + follow-up resolution (v1.4.0)
- **Kazakh-only recogniser surface** (v0.9.6 trilingual experiment reverted in v1.1.0; Latin transliteration removed)
- FST-backed slot expansion (`{slot|features}` — case × number × derivation × possessive × predicate-person)
- Template repository as external TOML data
- **Retrieval engine** (v1.6.0+): morpheme inverted index over committed corpus, composite deterministic ranking (overlap + pack-purity + length + loanword-density), verbatim sample citation with `(pack, sample_id)` provenance
- **Opt-in composition** (v1.9.0+): in-sample city swap via FST feature-preserving synthesis, year-guarded; adapted responses carry the «бейімд-» marker (v1.9.5)
- **Fact extraction + lexical graph** (v2.1 → v2.3): typed `Fact` tuples from copula / locative / possessive patterns, projected into a deterministic `LexicalGraph` (nodes + edges with merged provenance)
- Kazakh corpus at ~3.84 M committed / ~77.9 M local words; sources: Tatoeba, Wikipedia KZ, Common Voice KK, CC-100, Abai Wikisource, proverbs, synthetic, Kazakh classics
- Full regression test suite (335+ workspace tests) and foundation CI

## Scope of "FST-guaranteed" claim (accurate wording)

The FST synthesiser guarantees **grammatical correctness of slot-expanded template fragments** (e.g. `{city|locative}` → FST-synthesised `Алматыда`). Literal template text (e.g. `"сәлем"`, `"қайырлы таң"`) is pre-verified Kazakh committed in `data/dialog/templates/v1.toml`, not synthesised at runtime. Put together: no morphologically invalid word can leave the system through a slot path, and literal template strings are audited offline.

This is a weaker claim than "whole output is FST-guaranteed" — which would require every literal template token to pass back through the FST at runtime, which the current realiser does not do. Stated accurately for v1.3.x – v2.3.x.

## Out of Scope (permanent)

- **Multilingual input and output** — Kazakh only, both directions, by design
- **Audio / speech / multimodal**
- **Probabilistic free generation in the rule-based backbone** — predictability is the product
- **Cloud orchestration** — runs entirely on a single developer's laptop
- **Product UI** — `adam_chat` is the reference REPL; any UI is downstream work

## v2.0 direction — retrieval, not neural generation

The v2.0 "minimally thinking Kazakh model" is **not** a trained transformer LM. The approach is morpheme-indexed retrieval over the 100 M+ word corpus plus rule-based compositional synthesis. Reasons: predictability, zero hallucinations, cheap (M2 8 GB), safe (cannot say what isn't in the corpus), and exploits Kazakh's rich agglutinative morphology that the FST already unpacks. See `project_retrieval_not_neural_v2` memory and the roadmap for details.

## Post-v2.3 work (committed, not yet shipped)

- **Rule reasoner v0** (v2.4 target) — forward-chaining over the Lexical Graph to answer chained questions; every inferred fact carries a traceable rule-provenance
- **Lexical graph enrichment** — Lexicon-level edges (POS co-occurrence, shared domain) beyond just fact-derived ones
- More fact-extraction patterns (dative-motion → `GoesTo`, verb-derived action facts)
- Native-speaker review of templates + Lexicon proper-noun expansion
- Verb slot expansion in templates (`{root|verb_features}`)

### Delivered since v1.3.x

- v1.4.0 FST-NER refactor + DST (`Conversation::active_intent` / `intent_history`) + predicate-person copula
- v1.4.5 Modern-vocabulary Lexicon additions (20 professions + conversational nouns)
- v1.6.0–v1.9.5 full retrieval-and-composition stack
- v2.0 investor-demoable commitment release
- v2.1–v2.3 ILMRR bootstrap (fact extraction, Lexicon purge, lexical graph)
