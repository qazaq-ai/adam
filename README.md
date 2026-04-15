<p align="center">
  <img src="assets/shanraq.svg" alt="adam logo" width="160" height="160">
</p>

<h1 align="center">adam</h1>

<p align="center">
  <i>A Kazakh-first foundation language model, built in pure Rust.</i><br>
  <i>Қазақ тіліне арналған тіл моделінің іргетасы — таза Rust тілінде.</i>
</p>

<p align="center">
  <a href="https://github.com/qazaq-ai/adam/releases"><img src="https://img.shields.io/badge/version-0.3.0-blue?style=for-the-badge" alt="version"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-BUSL%201.1-orange?style=for-the-badge" alt="license"></a>
  <img src="https://img.shields.io/badge/language-Rust-CE412B?style=for-the-badge&logo=rust&logoColor=white" alt="rust">
  <img src="https://img.shields.io/badge/script-Cyrillic-8338EC?style=for-the-badge" alt="cyrillic">
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Linux-lightgrey?style=for-the-badge" alt="platform">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/params-20.0M-2EA44F?style=flat-square" alt="params">
  <img src="https://img.shields.io/badge/perplexity-871.30-2EA44F?style=flat-square" alt="perplexity">
  <img src="https://img.shields.io/badge/corpus-39.1k%20samples-9CCC65?style=flat-square" alt="corpus">
  <img src="https://img.shields.io/badge/vocab-4096-FBC02D?style=flat-square" alt="vocab">
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

# 3. Generate text with the v0.3.0 checkpoint
bash ./scripts/run_generate.sh "жақсы адам" 24 1.0 0 0.9 1.2
#                              prompt        ^^  ^   ^   ^   ^
#                                            new temp tk topp rep_pen
```

## Sample generations

From the v0.3.0 checkpoint (20.0M params, 15,000 training steps), nucleus sampling `top_p=0.9, repetition_penalty=1.2`:

| Prompt | Generated |
|---|---|
| жақсы адам | жақсы адамніңгеем сал кісі |
| үлкен қала | үлкен қала ме.. |
| бала мектепке | бала мектепке мінеді.. |
| ол | ол өте |
| олар | олар тұрады. |
| мен қазір | мен қазірі. |
| адам және | адам және тапты.тар батыр» қара |

Reproducible via `bash ./scripts/run_generation_showcase.sh` → `data/training/generation_showcase_report.json`. Generation quality reflects a 20M foundation trained on a 39k-sample mixed corpus; coherent chat-level output is a v0.5.0 target.

## Full training pipeline

```bash
# 1. Fetch authentic Kazakh text sources (Tatoeba, Wikipedia, Common Voice)
bash ./scripts/fetch_tatoeba_kazakh.sh
bash ./scripts/fetch_wikipedia_kz.sh
bash ./scripts/fetch_common_voice_kk.sh

# 2. Process each source into pack JSON
cargo run --release --bin process_tatoeba_kazakh
cargo run --release --bin process_wikipedia_kz
cargo run --release --bin process_common_voice_kk

# 3. Generate 18,000 FSM-validated synthetic sentences
bash ./scripts/run_synth_sentences.sh 18000

# 4. Combine all packs into the unified corpus (dedup included)
bash ./scripts/run_unified_corpus_assembly.sh

# 5. FSM-segment every word into morphemes (char fallback otherwise)
bash ./scripts/run_pretokenize_corpus.sh

# 6. Learn BPE merges (lexicon-seeded vocab, target 4096)
bash ./scripts/run_train_bpe.sh 4096

# 7. Encode with deterministic train/val split
bash ./scripts/run_encode_corpus.sh

# 8. Train the 20M transformer (~3h 45m on M2 Metal)
bash ./scripts/run_train_baseline.sh 15000

# 9. Evaluate held-out perplexity
bash ./scripts/run_eval_perplexity.sh

# 10. Run the 60-generation showcase
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

## Stats (v0.3.0)

| Component | Value |
|---|---|
| Lexicon roots | **211** (10 POS) |
| FSM rules | **422** |
| Eval segmentation examples | **464** (100% match rate) |
| Authentic Kazakh sources | **3** (Tatoeba, Wikipedia, Common Voice) |
| Training samples | **39,058** unique (37,119 train + 1,939 val) |
| Training tokens (encoded) | **606,416** (0.00% unknowns, 100.00% roundtrip) |
| BPE vocabulary | **4,096** |
| Model parameters | **20.0M** (hidden 512, layers 5, heads 8, ffn 2048) |
| Wall time (M2 Metal, 15k steps) | **~3h 45m** |
| **Validation perplexity** | **871.30** (1939 held-out samples) |

## Foundation Policies

- [corpus policy](docs/corpus_policy.md)
- [corpus sources](docs/corpus_sources.md)
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
