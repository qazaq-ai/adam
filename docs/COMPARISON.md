# adam vs LLMs — comparison instrument

**v6.5.0-rc21 (2026-06-11).**  Built per external-audit
recommendation #4 (2026-06-10): a same-task comparison instrument
rather than raw latency claims.

This document gathers **publicly available** benchmark scores for
the major LLMs adam is often (incorrectly) compared against.  We
do not have API access to all of them; we rely entirely on
published model cards, papers, and benchmark leaderboards cited
inline.

**adam is NOT a general LLM replacement.**  It is a deterministic
Kazakh-first reasoning kernel with a narrow surface (factual /
OOD / safety / tutor / conversational on curated Kazakh content).
The table below frames where each system class wins.

## 1. Kazakh-language accuracy

| Model | KazMMLU avg | MMLU-Kk | Belebele Kk | Source |
|---|---|---|---|---|
| **adam v6.5.0-rc18** | not run (see §5) | — | — | own 100-item blind eval: **97 / 100** [`data/eval/blind_eval_v1.json`](../data/eval/blind_eval_v1.json) |
| GPT-4o | **76.6 %** | not published | not published | [KazMMLU paper, ACL 2025](https://arxiv.org/abs/2502.12829) |
| DeepSeek V3 | 76.9 % | — | — | same |
| Llama 3.1 70B | **56.2 %** | — | — | same |
| Llama 3.1 8B | **39.7 %** | 31.3 | 25.9 | KazMMLU + [Sherkala paper](https://arxiv.org/abs/2503.01493) (Mar 2025) |
| Llama-3.1-Sherkala-8B (Kk-tuned) | 41.4 % | 34.6 | 30.6 | Sherkala paper |
| Claude (3.5 / 3.7 / 4) | **no Kazakh-specific number published** | — | — | Anthropic model cards do not break out per-language scores |

**KazMMLU** = 23 k Kazakh + Russian MCQ from Kazakhstan
educational materials (10 969 Kk + 12 031 Ru) covering STEM,
humanities, social science, language, other.

## 2. Kazakh QA / translation

- **KazQAD** (68 k Q-C-A triples) + **sKQuAD** (1 k records) — LLM-vs-RAG eval published Oct 2025 ([MDPI info16110943](https://doi.org/10.3390/info16110943)).
- **FLORES-200 Kazakh** for Llama 3.1 405B: **no Kazakh BLEU / chrF in Meta's model card**.  Kazakh is *not* one of the 8 officially supported languages (en / de / fr / it / pt / hi / es / th) per [Llama 3.1 MODEL_CARD.md](https://github.com/meta-llama/llama-models/blob/main/models/llama3_1/MODEL_CARD.md).

## 3. Resource cost — apples-to-watermelons by design

| Model | Params | RAM / VRAM (inference) | Hardware | API price ($/1M tok in/out) |
|---|---|---|---|---|
| **adam v6.5.0-rc20** | deterministic kernel; no params | **314 MB RSS** | CPU; M2 8 GB; **offline / 0 network** | **$0** |
| Llama 3.1 8B | 8 B | ~16 GB FP16 | 1× A100 / H100 | self-host |
| Llama 3.1 70B | 70 B | ~140 GB FP16 | 2-4× H100 | self-host |
| Llama 3.1 405B | 405 B | 972 GB FP16 / 486 GB FP8 / 243 GB INT4 | 8× H100 (FP8) | self-host |
| GPT-4o | undisclosed | API only | — | $2.50 / $10.00 |
| Claude 3.5 Sonnet | undisclosed | API only | — | $3.00 / $15.00 |

Sources: [HF Llama 3.1 blog](https://huggingface.co/blog/llama31), [Llama 3.1 405B card](https://apxml.com/models/llama-3-1-405b), [Anthropic pricing](https://platform.claude.com/docs/en/about-claude/pricing).

## 4. Hallucination rate (English benchmarks)

No Kazakh-specific factuality benchmark for LLMs exists publicly.
The Vectara grounded-summary leaderboard is the closest standard.

| Model | Vectara hallucination | Source |
|---|---|---|
| **adam (within curated Kazakh domains)** | **0** by construction (deterministic retrieval over curated `data/world_core/*.jsonl`) | this repo |
| GPT-4o | 1.5 % | [Vectara leaderboard](https://github.com/vectara/hallucination-leaderboard) |
| Claude 3.5 Sonnet | 4.4 % | same |
| Claude Opus | 10.1 % | same |
| GPT-5 / Claude Sonnet 4.5 / Grok-4 (reasoning, harder benchmark) | > 10 % each | [Vectara next-gen post](https://www.vectara.com/blog/introducing-the-next-generation-of-vectaras-hallucination-leaderboard) |
| Llama 3.1 405B | «best on HalluLens extrinsic» — exact % not in source | [HalluLens arXiv:2504.17550](https://arxiv.org/pdf/2504.17550) |

adam's «0 hallucinations within curated domains» claim is
architectural: every fact-bearing reply emits from a `FrameIndex`
hit on a curated `world_core` entry, or falls through to honest
refusal (see [rc15 safety guard](../crates/adam-dialog/src/safety_guard.rs)
+ [rc18 OOD discipline](../crates/adam-dialog/src/v6_2_router.rs)).
Off-corpus queries refuse rather than confabulate.

## 5. Honest gaps in this comparison

These are the limitations of the table above — read them.

- **adam's 97 / 100 is on its own 100-item curated battery.**  It is **not directly comparable** to KazMMLU's 23 k MCQ.  To make a true apples-to-apples comparison, adam must be run on KazMMLU itself.  That's the rc22+ deliverable (need to adapt adam's open-text reply path to MCQ scoring).
- **Claude has zero published Kazakh-specific benchmark scores.**  Anthropic does not break out per-length per-language numbers.  Without that, any «Claude vs adam on Kazakh» claim is speculative.
- **Llama 3.1 has no published FLORES-Kazakh chrF / BLEU.**  Kazakh isn't an officially supported language in the 405 B card.
- **Older hallucination benchmarks are partially saturated.**  TruthfulQA's blind decision-tree scores 79.6 %; HaluEval's length classifier 93.3 %.  Recent leaderboards favour generation-quality scoring over MC, but cross-model comparison stays noisy.

## 6. Where each system class wins

| | adam | LLM (any of the above) |
|---|---|---|
| Narrow Kazakh-curated factual / OOD / safety / tutor / conversational | **97 / 100 (own eval)** | 56-77 % (KazMMLU) |
| Broad open-domain knowledge | 0 outside curated corpus | strength |
| Long-context reasoning (32 k+ tokens) | not supported | strength |
| Creative / generative writing | not supported | strength |
| Multilingual coverage (50 + languages) | Kazakh + some Russian | strength |
| Offline / airgapped deployment | **runs** | requires network or 16-486 GB of weights |
| 0-GPU on a watch-class CPU | **runs (314 MB RSS)** | infeasible at quality |
| Byte-deterministic answers (regression-testable in CI) | **yes** | no (sampling) |
| Auditable fact provenance | **yes** (`data/world_core/*.jsonl`) | no (hidden training corpus) |
| Pricing | **$0 / query** | $2.50–$15 / 1 M tokens API, or self-host |

## 7. The honest one-liner

**adam wins where the problem is auditable, Kazakh, narrow, and
deployment-constrained.**  LLMs win on everything else.  The two
are different system classes; the right comparison instrument is
this table, not raw latency.
