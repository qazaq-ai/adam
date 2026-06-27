# Day 1 — base-model + inference-path decision

**Date:** 2026-06-27
**Branch:** `experiment/hybrid-qlm-verifier`

## Decision

| Question                | Choice                                                              |
| ----------------------- | ------------------------------------------------------------------- |
| Base model              | **Gemma-3 4B (instruction-tuned)**, q4_k_m gguf quantization        |
| Inference path          | **llama.cpp subprocess** via `llama-cli` (mirrors Whisper pattern)  |
| Continue-pretrain corpus| `data/curated` (~300 K KZ sentences, existing) — phase 1; optional KZ Wikipedia / Tatoeba in phase 2 |
| Paraphrase eval source  | Hand-authored from existing `data/eval/real_audit_v6_7/*.json` transcripts — start with ~30 canonical queries × 5 paraphrases each |

## Why Gemma-3 4B + q4_k_m

- **Multilingual coverage.**  Gemma-3 expanded multilingual scope vs Gemma-2; reasonable Kazakh prior (validates by day-3 baseline test).
- **Footprint on M2 8 GB.**  q4_k_m quantized: model ~2.5 GB on disk, ~3 GB working set with KV cache.  Adam runtime + Python eval harness ≈ 1.5 GB.  Net free ≈ 3 GB — enough margin if IDE / browser closed per the «clean RAM before training» discipline.
- **Open-weight + permissive license.**  Gemma terms permit local research use.
- **Architecturally similar to where the field is going.**  Continued-pretrain + LoRA recipes well-documented for Gemma-3 class.

Alternatives considered:

- **Llama-3.1 8B q4_k_m** — ~5 GB working set, leaves < 1 GB margin on 8 GB.  Would force closing every other tool to run.  Rejected for now; revisit if Gemma Kazakh baseline is too weak.
- **Gemma-3 1B** — fits comfortably (~600 MB) but Kazakh capacity at 1B class is too weak per published benchmarks.  Useful as a fallback if 4B inference latency on M2 is unworkable.
- **Mistral 7B / Qwen 2.5 7B** — comparable to Llama 8B class on footprint; no clear advantage for our Kazakh use case.

## Why llama.cpp subprocess

- **Mirrors existing Whisper integration** in voice_repl_v6_3 (`whisper-cli` subprocess).  One operational pattern.
- **No new Rust dependency drag.**  Candle / mistral.rs would force linking ML weight tensors into the workspace; subprocess keeps the deterministic kernel free of LM runtime.
- **Easy to swap base model.**  Change one CLI arg, not a Cargo dependency.
- **Cross-platform.**  Same script works on dev M2 and CI Linux when we get there.

Trade-off: subprocess fork latency ~30–60 ms per call.  For paraphrase / rescore use cases that fire once per turn this is acceptable; if we ever want token-stream generation we revisit.

## Setup (user-side, executed manually — NOT by adam)

```sh
# 1. Install llama.cpp (homebrew route — simplest on macOS).
brew install llama.cpp

# Verify:
which llama-cli            # expected: /opt/homebrew/bin/llama-cli

# 2. Create model dir + download Gemma-3 4B instruction q4_k_m gguf.
mkdir -p data/lm_models
# Place the gguf at:
#   data/lm_models/gemma-3-4b-it-q4_k_m.gguf
# Source: HuggingFace `google/gemma-3-4b-it` or any reputable
# community quantization mirror.  No automated download in this
# repo — model artifacts stay out of git (>50 MB gitignore
# policy).
```

After both steps complete, `experiments/hybrid_qlm/baseline/run_bare_baseline.sh` can be invoked to measure the model alone on the existing eval suite.

## Hard rules carried over from PROTOCOL.md

- Model never speaks factual claims to the user without verifier grounding.
- Production traffic untouched throughout the experiment (`ADAM_HYBRID_LM=1` env-gated).
- If Gemma 4B baseline on Kazakh is materially worse than the simplest paraphrase rules already in `input_normalizer`, archive and pivot to «no LM» conclusion.

## Day 1 deliverable

- `experiments/hybrid_qlm/baseline/run_bare_baseline.sh` (this commit) — measurement harness ready, no execution required from main / production CI.
- This file (`day_1.md`).

Day 2-3 deliverables (when the user has run the manual setup):
- `experiments/hybrid_qlm/baseline/day_3_results.md` — captured baseline.
- Decision: proceed with Gemma 4B, or fall back to a smaller model / a different architecture.
