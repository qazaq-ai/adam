# Roadmap

The roadmap records phases as executed, plus the near-term target. Earlier foundation phases (1–5) built the deterministic layers — corpus policy, FSM lexicon, synthetic corpus, tokenizer, first model. From Phase 6 onward the project started ingesting authentic Kazakh text.

## Phase 6 — Authentic Kazakh sources

| Sub-phase | Release | Source | Size | License |
|---|---|---|---|---|
| 6a | v0.1.1 | Tatoeba Kazakh | 4,058 | CC-BY 2.0 FR |
| 6b | v0.1.2 | *(tokenizer: char fallback + leading-punct ▁, 4096 vocab)* | — | — |
| 6c | v0.1.4 | Kazakh Wikipedia | 15,000 | CC-BY-SA 4.0 |
| 6d | v0.1.6 | Mozilla Common Voice KK | 6,108 | CC0-1.0 |

Unified corpus after Phase 6d: **39,058 unique samples**, three authentic Kazakh sources plus synthetic + proverbs. Lossless tokenization confirmed (0 unknowns, 100% roundtrip).

## Phase 7 — Baselines on real text

| Sub-phase | Release | Model | Corpus | Steps | Val PPL |
|---|---|---|---|---|---|
| 7 | v0.1.3 | 4.28M | 17k (Tatoeba added) | 7,000 | 626.81 |
| 7.1 | v0.1.5 | 4.28M | 32k (Wikipedia added) | 14,000 | 1078.68 |
| 7.2 | v0.2.0 | 4.28M | 39k (Common Voice added) | 15,000 | 1112.31 |

By v0.2.0 the 4.28M parameter envelope was saturated — PPL stopped improving with more data. Capacity, not data, was the bottleneck.

## Phase 8 — Capacity scale-up

| Release | Model | Corpus | Steps | Val PPL |
|---|---|---|---|---|
| v0.3.0 | 20.0M (H=512, L=5, vocab 4k) | 39k / 606k tokens | 15,000 | 871.30 |

First non-flat PPL delta since real-text onset. 20M is the largest config that fits MacBook Air M2 8GB training comfortably (peak RSS ~2.5 GB of 8 GB unified memory).

## Phase 9 — Data + infra maturity (current)

| Release | Model | Corpus | Steps | Val PPL | Notes |
|---|---|---|---|---|---|
| v0.4.0 attempted | 27.3M (H=512, L=6, vocab 8k) | 214k / 3.9M tokens | 20,000 | 1811.34 | rolled back — too aggressive scale-up |
| **v0.4.0 (shipped)** | **24.2M (H=512, L=5, vocab 8k)** | **244k / 4.09M tokens** | **20,000** | **1691.89** | 7 packs incl. Abai + CC-100 |

Key v0.4.0 additions:
- Abai Qunanbayuly's public-domain works (Wikisource, 2,253 samples) — first literary source
- CC-100 Kazakh web-crawl (50,000 filtered samples) — first web source
- Synthetic generator minimum length raised to 3 words (was dominated by 2-word noise)
- BPE vocab 4k → 8k with 2.80× → 3.27× compression
- Periodic checkpoint every 2,000 steps (crash recovery)

Capacity limit confirmed: 24M params × 4M tokens ≈ 25× below Chinchilla-optimal data. Further improvement needs an order-of-magnitude more training data, not more parameters.

## Phase 10 — Deterministic FST (current)

| Release | Scope | Result |
|---|---|---|
| v0.4.5 | `adam-kernel-fst` crate — phonology + morphotactics + parser + lexicon (4,454 curated + 11,919 Apertium) + CLI | 55 unit tests, 100% roundtrip on 36,238 full-lexicon cycles |
| v0.5.0 | Participles (`-{G}{A}н`, `-{A}тын`, `-{A}р`), converbs (`-{Y}п`, `-{A}`), vowel-final-stem aorist coalescence | 68 unit tests, covers most non-finite Kazakh verb forms |
| v0.5.5 | Pure Kazakh lexicon — drop 1,500 loanwords, add 500 Abai-attested classical roots | 14,106 entries, Abai coverage 88.8% → 97.8% |
| v0.6.0 | Derivational morphology — 11 word-formation suffixes (agent, abstract, privative, similative, ordinal, diminutive, verbal-noun, …) | 78 unit tests; root→derived→inflected chains work end-to-end |
| v0.7.0 | First dialog layer — 5-intent MVP (`adam-dialog` crate + `adam_chat` CLI + 15 end-to-end tests) | 175 workspace tests; predictable 5-layer pipeline documented |
| v0.7.5 | Dialog widening — 10 intents (+Thanks, Apology, AskHowAreYou, StatementOfWellbeing, AskName), templates moved to `data/dialog/templates/v1.toml` | 183 workspace tests; data-driven template repo replaces hardcoded planner arrays |
| v0.8.0 | Dialog widening — 25 intents (+15: age, location, occupation, family, weather, time, compliment, request, well-wishes, statement-of-name), PersonName extraction + slot expansion | 201 workspace tests; first entity extraction lands — user's name is pulled from self-introduction and substituted via `{name}` slot |
| v0.8.5 | Multi-turn session — `Conversation` struct; name persists across turns; templates filtered by slot availability; greetings gain `{name}` variants | 204 workspace tests; "Say your name once, get greeted by name forever" |
| v0.9.0 | Full entity absorption — Kazakh numeral parser (1–99), ablative/locative city stripping, 1sg-copula occupation stripping; `{age}/{city}/{occupation}` slots; `StatementOf*` variants carry `Option<T>` payloads | 215 workspace tests; every social-topic statement contributes a remembered entity |
| v0.9.5 | FST-backed slot expansion — `{slot\|features}` syntax parsed and rendered via `synthesise_noun`; cross-slot templates | 229 workspace tests; Kazakh case/number agreement becomes FST-guaranteed |
| v0.9.6 | Multilingual recogniser surface — Kazakh / Russian / English input triggers for all 25 intents; name extraction from 3 languages; Latin-root safety guard in realiser | 245 workspace tests; input any language, response always Kazakh |
| v0.9.7 | Lexicon-backed occupation recognition — generic 1sg-copula stripping + POS=noun lookup against 14 k Lexicon replaces the fixed 6-form table | 251 workspace tests; any Lexicon noun works, adjectives correctly rejected |

Phase 10 pivots the project from pure-stochastic transformers (v0.3–v0.4 line) to a deterministic morphology layer + small LM-over-roots. The v0.4.0 transformer baseline stays as reference; new work layers on top.

## Near-term

- **v0.9.8** — Derivation + Possessive in slot syntax (`{name|p1sg+dative}` = "менің Дәулетіме"); more cross-slot templates exploiting age + city + occupation combinations; Latin→Cyrillic transliteration helper (optional FST synthesis on Latin names).
- **v0.9.9** — native-speaker review of the MVP template set (~100 templates × 2–5 variants); correct phrasing, tighten politeness/register.
- **v1.0.0** — investor-demoable MVP: 25-intent trilingual-input predictable Kazakh dialog, multi-turn session state, FST-guaranteed morphology, native-speaker-reviewed templates, end-to-end Rust stack.

## Out of near-term scope

- Multilingual expansion
- Speech / multimodal
- Cloud platform work
- 50M+ parameter models on M2 8GB (plausible but untested; future work on M5 24GB or cloud GPU rental)
