# Real-corpus mix proof — 2026-05-16

**Branch:** `experimental/agglutinative-neural`
**Commit context:** after `d131bc1` (MANIFESTO + SPDX), before result commit
**Hardware:** M2 8 GB, CPU only, pure Rust
**Author:** `shaman` <baimurza.daulet@gmail.com>

## What was tested

Three training configurations of the same `TinyAgt` model (vocab-sized
to the data, d_model=64, n_layers=2, batch=32, lr=3e-3, 3 epochs):

1. **synth-nouns** — FST-generated noun inflections only (~53 k pairs).
2. **synth-mixed** — adds FST verb inflections (~12 k pairs).
3. **synth + real** — adds 44 194 Root-decomposed pairs from the
   real-corpus pack (`data/curated/real_corpus_pairs.json`, built
   from 15 committed Kazakh-language packs + 500 books slice of
   `huggingface_kz/kazakhBooks.csv`).

All three measure:
- Train-end cross-entropy
- Held-out (10 % stride) teacher-forced CE
- Held-out exact-match rate (does greedy reproduce the gold
  continuation token-for-token?)

## Results

| Config            | Train CE | Held-out CE | Gap   | Exact-match (greedy) |
|-------------------|---------:|------------:|------:|---------------------:|
| synth-nouns       |    0.297 |       0.493 | 0.196 |                 0 %  |
| synth-mixed       |    0.355 |       0.486 | 0.131 |                 0 %  |
| **synth + real**  |    0.384 |       0.415 | **0.031** |          **15 %** |

## Reading the numbers

**Generalisation gap (held-out CE − train CE) collapsed 6.3×.**
A model trained only on FST-synth had a 0.196 gap; mixing in 44 k
real-corpus pairs dropped it to 0.031. The model is learning a
distribution that *transfers* to unseen (root, feature) combinations,
not just memorising the training samples.

**Exact-match went from 0 % to 15 %.** With synth-only training, the
model's greedy generation from `[BOS, R]` would deterministically pick
the most common path — *some* valid noun inflection, but never the
specific gold one in the held-out set. With real data mixed in, the
model has internalised enough of the conditional distribution
P(suffix | root) to reproduce the gold continuation 15 % of the time
on a 100-prefix held-out set.

**Train CE went up slightly (0.297 → 0.384), not down.** This is the
expected sign of a *harder* training distribution: real-corpus pairs
expose the model to inflection patterns the synth pipeline doesn't
generate. The model can no longer overfit to the narrow synth shape.

## Why this is the proof we were after

This is the **third-path empirical proof** the research charter was
written for. Recall the four ARK inversions from
[`docs/MANIFESTO.md`](../MANIFESTO.md):

1. **Algebra, not statistics** — ✅ the FST + AggTokenizer
   produced 44 194 Root-decomposed real pairs from the 16 850-entry
   Lexicon. The model trains on morpheme-typed tokens, not BPE
   sub-word pieces.
2. **CPU, not cloud** — ✅ 2.05 M parameters, 39 minutes on M2 CPU,
   pure Rust via burn (ndarray backend), no Python, no GPU.
3. **Verifier, not RLHF** — ✅ FST-constrained decoding hits 100 %
   transition-validity on held-out prefixes; the architectural gate
   blocks every morpho-invalid continuation by construction.
4. **Agglutinative-first, not English-first** — ✅ Kazakh is the
   first-class language; every morpheme is typed by its FST role
   (Number / Case / Possessive / Tense / Person / Voice).

The collapse of the generalisation gap (0.196 → 0.031) is the
*quantitative* evidence that neural training **inside the algebraic
envelope** behaves the way the hypothesis predicts: the model learns
the algebra, not the table.

## What is NOT proven by this run

- This is still a **PoC scale**: 2 M parameters, 3 epochs, 100-prefix
  evaluation set. Production-scale claims will require at least 10 M
  parameters, more epochs, and a properly stratified eval split.
- The 21 % Root yield from the real corpus is bounded by Lexicon V1
  coverage (16 850 entries). Scaling Lexicon up will scale the
  real-pair yield up proportionally.
- Latency and energy numbers haven't been audited at production
  scale yet.

## What's next

1. **Scale the Lexicon** (v1 → v2): closes the 79 % Unk gap in real
   corpus. Bumps real-pair yield from 44 k to ~200 k+.
2. **Multi-language extension**: same architecture on Kyrgyz, Tatar,
   Uyghur — the morphology FSTs need work but the rest is invariant.
3. **Algebraic loss A/B at scale**: previous A/B (v6 of the PoC binary)
   showed +15 pp on unconstrained validity at synth scale; re-run at
   real-corpus scale.
4. **Verifier-bounded factuality demo**: introduce world_core / facts
   knowledge graph as a generation-time grounded verifier; measure
   hallucination rate against an LLM baseline on a held-out fact set.

## Reproducibility

```bash
# 1. Build the real-corpus pair set (CPU, ~5 min).
MAX_PAIRS=300000 MAX_BOOKS_CSV=500 \
  cargo run --release -p adam-agg-synth --bin build_real_corpus_pairs

# 2. Train + evaluate (CPU, ~40 min).
POC_REAL_PACK=data/curated/real_corpus_pairs.json \
POC_SKIP_ALG=1 \
  cargo run --release -p adam-agg-model --bin poc_kazakh_train
```

Output is written to stderr and includes the table above verbatim.
The `kazakhBooks.csv` source (~3.97 GB) is gitignored; fetch via
`./scripts/fetch_huggingface_kazakh.sh`.
