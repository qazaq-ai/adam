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

## Phase 8 — Capacity scale-up (current)

| Release | Model | Corpus | Steps | Val PPL |
|---|---|---|---|---|
| v0.3.0 | **20.0M** (H=512, L=5) | 39k | 15,000 | **871.30** (−21.7%) |

First non-flat PPL delta since real-text onset. 20M is the largest config that fits MacBook Air M2 8GB training comfortably (peak RSS ~2.5 GB of 8 GB unified memory).

## Near-term

- **v0.4.0** — quality work on the 20M model (longer training, schedule tuning) and/or first public inference surface.
- **v0.5.0** — working Kazakh chat-bot. Minimum viable: coherent short-turn dialogue with instruction-following on common Kazakh prompts. This is the current project target before considering hardware upgrades (M5 24 GB) or investor conversations.

## Out of near-term scope

- Multilingual expansion
- Speech / multimodal
- Cloud platform work
- 25M+ parameter models on M2 8GB (plausible but untested; future work on M5 24GB)
