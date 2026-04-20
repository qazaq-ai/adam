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
| v1.0.0 | **MVP cut** — no new features; full documentation refresh; transformer-era narrative compressed into history section | 271 workspace tests; investor-demoable |
| v1.1.0 | **Kazakh-only revert + modern Lexicon + smart Unknown** — reverted v0.9.6 multilingual triggers, removed transliteration module; added Insult intent; Unknown handler extracts noun hint via FST; +12 modern Kazakh Lexicon roots | 253 workspace tests; course-correction toward thinking Kazakh model |
| v1.1.5 | **Corpus audit baseline** — new `corpus_audit` binary measures the v1.x starting position per-source: word counts, vocab, Kazakh purity, dedup. Reports **2.24 M words baseline** (45× expansion needed to reach the 100 M target). Adds expansion plan through v1.5.0 | 256 workspace tests; measurable progress toward thinking model |
| v1.2.0 | **Classical literature expansion** — new `kazakh_classics_pack.json` (111 samples, 926 words, **100.00 % Kazakh purity**) from Ыбырай Алтынсарин + Мағжан Жұмабаев (kk.wikisource, public domain). OCR deferred | 256 workspace tests; literary core added |
| v1.3.0 | **Wikipedia re-extract** — rewrote `process_wikipedia_kz` (byte-by-byte → 64 KB chunked streaming, 100× faster; added 10 % loanword-density filter). Committed pack: 150 k samples / 1.61 M words / 99.99 % purity. Local uncapped: 1.4 M samples / 15 M words. Corpus total 2.85 M words, expansion gap 45× → 35× | 256 workspace tests; +27 % corpus |
| v1.3.5 | **Sharding + docs drift + v2.0 direction committed** — `--full` mode writes 10 shards (≤50 MB each) to gitignored `data/curated/shards/` for local retrieval-engine fuel; `corpus_audit --local` scans them. Docs fixed: badge 253→256, `foundation_scope` no longer claims trilingual/transliteration, "FST-guaranteed" wording tightened. **v2.0 committed as retrieval-over-corpus, not a trained LM** | 256 workspace tests; local corpus 16 M words, gap to 100 M = 6.2× |
| v1.4.0 | **FST-NER + DST + predicate-copula FST** — added `Predicate` feature to `NounFeatures` (7 person variants + 6 suffix templates); +30 Kazakh place-name entries in Lexicon; `detect_statement_of_location` / `detect_statement_of_occupation` refactored to use `Analysis` as primary path; `Conversation` gained `active_intent` + `intent_history` + `resolve_follow_up` | 262 workspace tests; closes Codex/Antigravity critiques |
| v1.4.5 | **Lexicon polish +20 nouns** — modern Kazakh professions (нұсқаушы, кеңесші, жетекші, жүргізуші, саудагер, …) and common conversational vocabulary (әке, аға, апа, бүгін, ертең, кеше, мекеме, кеңсе, ұйым). All native Kazakh, no loanwords | 262 workspace tests; each new profession round-trips through FST-NER |
| v1.5.0 | **CC-100 re-extract** — rewrote `process_cc100_kk` (stdin streaming, old 50 k cap removed, 10 % loanword-density filter, sharding at 140 k samples/shard). Committed pack: 140 k samples / 4.01 M words committed. 33 local shards (02–34) add 4.6 M samples / 74 M more local words. **Corpus: 4.01 M committed / 77.9 M local; gap to 100 M = 1.3×**. Purity −1.6 pp (web-crawl tax, accepted per corpus-purity directive) | 262 workspace tests; +27× local corpus |
| v1.5.5 | **Morpheme-coverage audit** — new `morpheme_coverage` binary. Prefix-matches every committed word against the 14,247-root Lexicon, reports per-pack coverage + top 20 uncovered words by frequency. Baseline: **79.48 % across 3.84 M committed words** (synthetic 99.82 %, real-corpus packs 76–81 %). Concrete Lexicon gaps identified: `деп`, `осы`, `оның`, `деген`, `пен`, `орта`. v1.5.5 was originally planned as "government sources"; that directive moves to v1.6.x since the sources need dedicated scraping infrastructure | 267 workspace tests; coverage delta now a measurable PR metric |
| v1.6.0 | **Retrieval engine bootstrap** — new `adam-retrieval` crate with `MorphemeIndex` (BTreeMap for deterministic JSON) + `SampleRef` + `search` + `search_conjunction`. New `build_morpheme_index` binary parses committed packs through the FST (unique-word cache → ~10 min full-corpus build). Two modes: default `--limit 500/pack` writes the committed 1.6 MB snapshot; `--full` writes the gitignored 700 MB full index. FST throughput measured at 1.155 ms/word. No `Intent::Unknown` integration yet (v1.6.5+) | 274 workspace tests; retrieval crate shipped |
| **v1.6.5** | **Retrieval wired into Intent::Unknown** — `MorphemeIndex` gains `sample_texts` so dialog can cite the actual sentence, not just the sample id. `Conversation::with_morpheme_index` attaches the index; when Unknown fires with a recognised noun, the response quotes a verbatim corpus example. New `unknown.with_evidence` template family wraps quotes in «…». Deterministic: picks the first (sorted) posting — reproducible across runs. No regression: without an index, v1.1.0 noun-echo behaviour is preserved | 279 workspace tests; dialog now cites Abai, Wikipedia, CC-100 for its fallback turns |

## Pre-Phase-10 — transformer era (v0.1.0 → v0.4.0)

Historical context for readers who want the full lineage. None of the code or data on this path is load-bearing for v1.0.0 except where explicitly referenced (the v0.4.0 checkpoint, the corpus packs consumed by `validate_foundation.sh`).

- **Phase 6 (v0.1.1 – v0.1.6)** — authentic Kazakh source ingestion: Tatoeba (4,058), Kazakh Wikipedia (15,000), Common Voice KK (6,108). Unified corpus reaches 39 k samples, lossless tokenization at 0 unknowns / 100% roundtrip.
- **Phase 7 (v0.1.3 – v0.2.0)** — transformer baselines on real text. 4.28 M-parameter envelope saturates by v0.2.0 (PPL ~1100 flat); capacity, not data, is the bottleneck.
- **Phase 8 (v0.3.0)** — scale to 20 M params. First non-flat PPL delta since real-text onset (PPL 871). 20 M is the largest config fitting M2 8 GB unified memory.
- **Phase 9 (v0.4.0)** — 24.2 M transformer with literary sources (Abai Wikisource 2,253 samples) + CC-100 web-crawl (50 k filtered) + synthetic-generator min-length raised to 3 words + BPE vocab 4k → 8k. Val PPL 1691.89 on 12,101 held-out samples. Confirmed capacity limit: 24 M × 4 M tokens is ≈ 25× below Chinchilla-optimal.

The post-v0.4.0 pivot was a deliberate choice: further transformer scaling required an order-of-magnitude more data that didn't exist for Kazakh at the quality bar required. The deterministic FST + dialog path delivered MVP-grade predictable Kazakh in pure Rust on the same hardware.

## Post-v1.0 direction

Post-v1.0.0 testing exposed that the MVP was a programmed toy — it answered only the 25 scenarios we enumerated, with no generalisation. The user approved a honest course correction and a long-term commitment to a truly thinking Kazakh model.

### Course-correction rationale

- **v0.9.6 multilingual was a mistake.** The Russian / English triggers diluted the Kazakh-first thesis. A Russian speaker writing "Я разработчик" got "түсінбедім" because "разработчик" wasn't in the Lexicon — the recogniser widened the surface without adding real coverage.
- **Fundamental trade-off acknowledged.** Rule-based systems can't generalise; neural systems can but hallucinate. You can't have both. The v1.0.0 predictable pipeline is kept, and generalisation will come as a neural **fallback** for `Intent::Unknown` only — the 26-intent deterministic backbone stays as the 0-hallucination guarantee.

### Committed sequence

- **v1.1.0 (done)** — Kazakh-only revert; modern Lexicon expansion (+12 roots); Insult intent; smart Unknown handler with noun-hint. Incremental coverage without abandoning thesis.
- **v1.x** — Kazakh corpus engineering. **Committed 4.01 M / local 77.9 M as of v1.5.0 (gap to 100 M = 1.3×)**. Target: **100 M+ Kazakh tokens** from native-text sources (Qazaq Wikipedia, literature, government Kazakh, OCR books). v1.5.5 government sources are expected to close the gap. Pure data engineering, not ML.
- **v2.0** — compact Kazakh LM (transformer or SSM), trained in pure Rust (GGML-style, no PyTorch). Plugged in as the `Intent::Unknown` fallback only. The 26-intent pipeline continues to handle everything it recognises at 0 hallucinations.
- **post-v2.0** — multimodality (speech / vision) only after a truly thinking Kazakh LM exists. Not before.

### Post-v1.1.0 candidates (not promised, not scheduled)

- Native-speaker review of the template set (phrasing, register, naturalness).
- Further modern Lexicon expansion (50+ new professions + tech vocabulary).
- Verb slot expansion (`{root|verb_features}` with verb synthesiser dispatch).
- Additional intents beyond the current 26.

## Out of scope (permanent)

- **Multilingual input and output** (v1.1.0 revert). The v0.9.6 Russian / English triggers were removed. `adam` accepts and produces only Kazakh. Generalisation comes via the v2.0 Kazakh LM, not translation.
- **Speech / multimodal** — deferred until a thinking Kazakh LM exists.
- **Cloud platform work.**
- **50 M+ parameter transformer experiments on current hardware** (M2 8 GB). Target for v2.0 is a compact model that fits on this hardware.
- **Probabilistic / LLM-style free generation in the recognised-intent path.** The 26-intent deterministic backbone never hallucinates. Only the `Unknown`-fallback path (v2.0) is allowed to be generative, and responses from that path will be explicitly marked in traces.
