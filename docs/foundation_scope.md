# Foundation Scope

## Goal

Deliver a **predictable, auditable Kazakh dialog system** built entirely in Rust and runnable on a MacBook Air M2 8 GB. The output must be grammatically correct by construction and every layer's decision must be traceable. No probabilistic free generation.

## In Scope (v1.0.0 delivered)

- Pure pre-modern Kazakh Lexicon (14 k curated roots, loanwords purged)
- Deterministic FST morphology (phonology + morphotactics + inverse parser)
- 25-intent dialog pipeline with multi-turn session state
- Trilingual input recognition (Kazakh / Russian / English) with Kazakh-only output
- FST-backed slot expansion for grammatical agreement (case, number, derivation, possessive)
- Latin → Cyrillic transliteration for foreign names
- Template repository as external TOML data (editable without recompiling)
- Full regression test suite and foundation CI

## Out of Scope (permanent)

- **Multilingual output** — response is always Kazakh by design
- **Audio / speech / multimodal**
- **Probabilistic free generation** — the project's core proposition is predictability
- **Arbitrary-topic question answering** — the 25-intent surface is the product
- **Cloud orchestration** — runs entirely on a single developer's laptop
- **Product UI** — `adam_chat` is the reference REPL; any UI is downstream work

## Out of Scope (near-term, may move)

- Native-speaker review of the template set
- Lexicon expansion (proper nouns, modern-vocabulary tier)
- Verb slot expansion in templates
- Intents beyond the 25-intent surface
- Polished Latin→Cyrillic transliteration (silent-h handling etc.)

None of these block v1.0.0. Any would ship as v1.1.0+.
