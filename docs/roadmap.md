# Roadmap

Version-by-version history of `adam`, grouped into two architectural eras. The final entry (v1.0.0) is the MVP cut.

## Lifecycle view

```
  v0.1.0 ────────────────── v0.4.0           v0.4.5 ──────────────── v1.0.0
  ├─ transformer era ───────────┤            ├─ FST + dialog era ────────┤
  │                              │            │                            │
  corpus curation                │            deterministic FST morphology │
  BPE tokenizer                  │            14 k-entry pure Kazakh Lexicon
  24.2 M transformer             │            25-intent dialog pipeline    │
  (archived as reference)        │            trilingual input, KZ output  │
                                 v            FST-guaranteed morphology    v
                              pivot                                       MVP
```

The transformer era (v0.1–v0.4) established the corpus, tokenizer, and training infrastructure. The FST + dialog era (v0.4.5+) replaced the stochastic model with a deterministic pipeline that holds predictability as a hard constraint. The v0.4.0 checkpoint is preserved in `data/training/` as a regression reference but is not on the v1.0.0 codepath.

## Phase 10 — FST + dialog layer (v0.4.5 → v1.0.0)

The MVP path. Each release is strictly additive — no rollbacks, no feature gating.

| Release | Scope | Result |
|---|---|---|
| v0.4.5 | `adam-kernel-fst` crate — phonology + morphotactics + parser + lexicon + CLI | 55 unit tests, 100% roundtrip on 36,238 full-lexicon cycles |
| v0.5.0 | Participles, converbs, vowel-final-stem aorist coalescence | 68 unit tests; most non-finite Kazakh verb forms covered |
| v0.5.5 | Pure Kazakh lexicon — drop 1,500 loanwords, add 500 Abai-attested classical roots | 14,106 entries, Abai coverage 88.8% → 97.8% |
| v0.6.0 | Derivational morphology — 11 word-formation suffixes | 78 unit tests; root→derived→inflected chains work end-to-end |
| v0.7.0 | First dialog layer — `adam-dialog` crate, 5-intent MVP, `adam_chat` CLI | 175 workspace tests; 5-layer predictable pipeline |
| v0.7.5 | Dialog widening to 10 intents; templates moved to `data/dialog/templates/v1.toml` | 183 workspace tests; data-driven template repo |
| v0.8.0 | 25 intents (+15: age, location, occupation, family, weather, time, compliment, request, well-wishes, statement-of-name); PersonName extraction | 201 workspace tests; first entity extraction lands |
| v0.8.5 | Multi-turn session state via `Conversation`; greeting `{name}` variants | 204 workspace tests; "Say your name once, get greeted forever" |
| v0.9.0 | Full entity absorption — Kazakh numeral parser (1–99), ablative/locative city, 1sg-copula occupation; `{age}/{city}/{occupation}` slots | 215 workspace tests; every statement contributes a remembered entity |
| v0.9.5 | FST-backed slot expansion — `{slot\|features}` parsed and rendered via `synthesise_noun`; cross-slot templates | 229 workspace tests; agreement becomes FST-guaranteed |
| v0.9.6 | Multilingual recogniser — Kazakh / Russian / English triggers for all 25 intents; Latin-root safety guard | 245 workspace tests |
| v0.9.7 | Lexicon-backed occupation recognition via generic 1sg-copula stripping + POS lookup | 251 workspace tests |
| v0.9.8 | Full slot syntax (Derivation + Possessive) + Latin→Cyrillic transliteration + triple-slot templates | 265 workspace tests; demo-ready UX |
| v0.9.9 | FST Instrumental harmony fix (`Алматыман → Алматымен`, `мұғалімбен → мұғаліммен`); 6 regression tests; template polish | 271 workspace tests; last stretch before MVP |
| **v1.0.0** | **MVP cut** — no new features; full documentation refresh; transformer-era narrative compressed into history section | **271 workspace tests**; investor-demoable |

## Pre-Phase-10 — transformer era (v0.1.0 → v0.4.0)

Historical context for readers who want the full lineage. None of the code or data on this path is load-bearing for v1.0.0 except where explicitly referenced (the v0.4.0 checkpoint, the corpus packs consumed by `validate_foundation.sh`).

- **Phase 6 (v0.1.1 – v0.1.6)** — authentic Kazakh source ingestion: Tatoeba (4,058), Kazakh Wikipedia (15,000), Common Voice KK (6,108). Unified corpus reaches 39 k samples, lossless tokenization at 0 unknowns / 100% roundtrip.
- **Phase 7 (v0.1.3 – v0.2.0)** — transformer baselines on real text. 4.28 M-parameter envelope saturates by v0.2.0 (PPL ~1100 flat); capacity, not data, is the bottleneck.
- **Phase 8 (v0.3.0)** — scale to 20 M params. First non-flat PPL delta since real-text onset (PPL 871). 20 M is the largest config fitting M2 8 GB unified memory.
- **Phase 9 (v0.4.0)** — 24.2 M transformer with literary sources (Abai Wikisource 2,253 samples) + CC-100 web-crawl (50 k filtered) + synthetic-generator min-length raised to 3 words + BPE vocab 4k → 8k. Val PPL 1691.89 on 12,101 held-out samples. Confirmed capacity limit: 24 M × 4 M tokens is ≈ 25× below Chinchilla-optimal.

The post-v0.4.0 pivot was a deliberate choice: further transformer scaling required an order-of-magnitude more data that didn't exist for Kazakh at the quality bar required. The deterministic FST + dialog path delivered MVP-grade predictable Kazakh in pure Rust on the same hardware.

## Post-v1.0.0 candidates

Not promised, not scheduled. Any of these would ship as v1.1.0+.

- Native-speaker review of the template set (phrasing, register, naturalness).
- Lexicon expansion: proper-noun sub-lexicon, modern-vocabulary tier (loanword-allowed, explicitly separated).
- Polished Latin→Cyrillic transliteration (silent-h handling for English names, alternate jh-cluster conventions).
- Verb slot expansion (`{root|verb_features}` with verb synthesiser dispatch).
- Additional intents beyond the 25-intent surface.

## Out of scope (permanent)

- Multilingual output — the response is always Kazakh, by design.
- Speech / multimodal.
- Cloud platform work.
- 50 M+ parameter transformer experiments on current hardware (M2 8 GB).
- Probabilistic / LLM-style free generation. The project's value proposition is predictability; any path that breaks that is explicitly out of scope.
