<p align="center">
  <img src="assets/shanraq.svg" alt="adam logo" width="160" height="160">
</p>

<h1 align="center">adam</h1>

<p align="center">
  <i>A Kazakh-first foundation language model, built in pure Rust.</i><br>
  <i>Қазақ тіліне арналған тіл моделінің іргетасы — таза Rust тілінде.</i>
</p>

<p align="center">
  <a href="https://github.com/qazaq-ai/adam/releases"><img src="https://img.shields.io/badge/version-0.1.0-blue?style=for-the-badge" alt="version"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-BUSL%201.1-orange?style=for-the-badge" alt="license"></a>
  <img src="https://img.shields.io/badge/language-Rust-CE412B?style=for-the-badge&logo=rust&logoColor=white" alt="rust">
  <img src="https://img.shields.io/badge/script-Cyrillic-8338EC?style=for-the-badge" alt="cyrillic">
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Linux-lightgrey?style=for-the-badge" alt="platform">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/params-3.06M-2EA44F?style=flat-square" alt="params">
  <img src="https://img.shields.io/badge/perplexity-129.49-2EA44F?style=flat-square" alt="perplexity">
  <img src="https://img.shields.io/badge/corpus-13.9k%20samples-9CCC65?style=flat-square" alt="corpus">
  <img src="https://img.shields.io/badge/vocab-1390-FBC02D?style=flat-square" alt="vocab">
  <img src="https://img.shields.io/badge/lexicon-211%20roots-FBC02D?style=flat-square" alt="lexicon">
  <img src="https://img.shields.io/badge/rules-422%20FSM-FBC02D?style=flat-square" alt="rules">
  <img src="https://img.shields.io/badge/foundation-validated-2EA44F?style=flat-square" alt="foundation">
</p>

---

## What is adam?

`adam` is a foundation language model for Kazakh, built **entirely in Rust** — no Python, no external ML pipelines. The whole stack — lexicon definition, finite-state morphological engine, synthetic corpus generation, BPE tokenizer, model training, and inference — runs on a single MacBook Air M2 8GB.

The name *adam* (Kazakh: **адам**) means "human".

The mission is small but precise: build a culturally and linguistically grounded foundation that the Kazakh NLP community can reuse — from the morphological analyzer up to a working transformer language model.

## Architecture

Three layers, five Rust crates:

| Layer | Crate | Role |
|---|---|---|
| **L0** | [`adam-kernel`](crates/adam-kernel) | Identity + Kazakh FSM morphological engine |
| **L1** | [`adam-tokenizer`](crates/adam-tokenizer) | Pre-tokenizer + BPE trainer + encoder/decoder |
| **L1** | [`adam-corpus`](crates/adam-corpus) | Source acceptance + synthetic sentence generator |
| **L1** | [`adam-eval`](crates/adam-eval) | Evaluation suite + benchmark reports |
| **L2** | [`adam-train`](crates/adam-train) | Transformer model + training loop + inference |

Every layer outputs deterministic, regression-tested JSON artifacts. `bash ./scripts/validate_foundation.sh` runs the full pipeline end-to-end.

## Quickstart

```bash
# 1. Build everything
cargo build --release

# 2. Validate the foundation (all golden artifacts + tests)
bash ./scripts/validate_foundation.sh

# 3. Generate text with the v0.1.0 checkpoint
bash ./scripts/run_generate.sh "жақсы адам" 24 1.0 0 0.9 1.2
#                              prompt        ^^  ^   ^   ^   ^
#                                            new temp tk topp rep_pen
```

## Sample generations

From the v0.1.0 checkpoint (3.06M params, 7000 training steps), nucleus sampling `top_p=0.9, repetition_penalty=1.2`:

| Prompt | Generated |
|---|---|
| жақсы адам | жақсы адам қолданады. |
| үлкен жақсы адам | үлкен жақсы адам біледі. |
| бала мектепке | бала мектепке сақтайды. |
| ол | ол жасайды. |
| олар | олар жасайды. |
| мен қазір | мен қазір айтады. |
| адам және | адам және оқыйды. |

Reproducible via `bash ./scripts/run_generation_showcase.sh` → `data/training/generation_showcase_report.json`.

## Full training pipeline

```bash
# 1. Generate 18,000 grammatically-validated synthetic sentences
bash ./scripts/run_synth_sentences.sh 18000

# 2. Combine with curated text + classical Kazakh proverbs
bash ./scripts/run_unified_corpus_assembly.sh

# 3. FSM-segment every word into morphemes
bash ./scripts/run_pretokenize_corpus.sh

# 4. Learn BPE merges (lexicon-seeded vocab)
bash ./scripts/run_train_bpe.sh 4096

# 5. Encode with deterministic train/val split
bash ./scripts/run_encode_corpus.sh

# 6. Train the transformer (~25 min on Metal)
bash ./scripts/run_train_baseline.sh 7000

# 7. Evaluate held-out perplexity
bash ./scripts/run_eval_perplexity.sh

# 8. Run the 60-generation showcase
bash ./scripts/run_generation_showcase.sh
```

## Key binaries

| Binary | Crate | Purpose |
|---|---|---|
| `coverage_report` | adam-kernel | Measure FSM coverage on real Kazakh text |
| `synth_sentences` | adam-corpus | Generate FSM-validated synthetic sentences |
| `pretokenize_corpus` | adam-tokenizer | Morpheme-aware pre-tokenization |
| `train_bpe` | adam-tokenizer | BPE trainer over morpheme stream |
| `encode_corpus` | adam-tokenizer | Encode pack to token IDs (with train/val split) |
| `train_baseline` | adam-train | AdamW training loop with grad clipping |
| `eval_perplexity` | adam-train | Held-out perplexity evaluation |
| `generate` | adam-train | Inference with greedy/temperature/top-k/top-p/repetition-penalty |
| `generation_showcase` | adam-train | Multi-prompt × multi-config quality report |

## Stats (v0.1.0)

| Component | Value |
|---|---|
| Lexicon roots | **211** (10 POS) |
| FSM rules | **422** |
| Eval segmentation examples | **464** (100% match rate) |
| Training samples | **13,929** unique |
| Training tokens (encoded) | **60,936** |
| BPE vocabulary | **1,390** |
| Model parameters | **3.06M** |
| Wall time (M2 Metal, 7k steps) | **~25 min** |
| **Validation perplexity** | **129.49** |

## Foundation Policies

- [corpus policy](docs/corpus_policy.md)
- [curation workflow](docs/curation_workflow.md)
- [source classification](docs/source_classification.md)
- [source scoring](docs/source_scoring.md)
- [tokenizer policy](docs/tokenizer_policy.md)
- [tokenizer experiment plan](docs/tokenizer_experiment_plan.md)
- [tokenizer dry run](docs/tokenizer_dry_run.md)
- [tokenizer segmentation eval](docs/tokenizer_segmentation_eval.md)
- [evaluation policy](docs/evaluation_policy.md)
- [training baseline](docs/training_baseline.md)

## Out of scope (foundation phase)

- Multilingual expansion
- Speech / multimodal
- Cloud platform work
- Chat product features

The repo grows from clean data and hard evaluation, not from broad claims.

## License

Business Source License 1.1. Converts automatically to Apache License 2.0 on **2029-01-01**.
See [LICENSE](LICENSE) for full terms.

Non-commercial and research use is unrestricted today. Commercial use is permitted unless it competes directly with Qazna Technologies LLP products or services.

Copyright © 2024–2026 Qazna Technologies LLP.

For commercial licensing inquiries: hello@qazaq.ai
