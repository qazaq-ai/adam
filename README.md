<p align="center">
  <img src="assets/shanraq.svg" alt="adam logo" width="160" height="160">
</p>

<h1 align="center">adam</h1>

<p align="center">
  <i>A Kazakh-first foundation language model, built in pure Rust.</i><br>
  <i>Қазақ тіліне арналған тіл моделінің іргетасы — таза Rust тілінде.</i>
</p>

<p align="center">
  <a href="https://github.com/qazaq-ai/adam/releases"><img src="https://img.shields.io/badge/version-0.5.0-blue?style=for-the-badge" alt="version"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-BUSL%201.1-orange?style=for-the-badge" alt="license"></a>
  <img src="https://img.shields.io/badge/language-Rust-CE412B?style=for-the-badge&logo=rust&logoColor=white" alt="rust">
  <img src="https://img.shields.io/badge/script-Cyrillic-8338EC?style=for-the-badge" alt="cyrillic">
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Linux-lightgrey?style=for-the-badge" alt="platform">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/params-24.2M-2EA44F?style=flat-square" alt="params">
  <img src="https://img.shields.io/badge/perplexity-1691.89-2EA44F?style=flat-square" alt="perplexity">
  <img src="https://img.shields.io/badge/corpus-244k%20samples-9CCC65?style=flat-square" alt="corpus">
  <img src="https://img.shields.io/badge/vocab-8192-FBC02D?style=flat-square" alt="vocab">
  <img src="https://img.shields.io/badge/lexicon-211%20roots-FBC02D?style=flat-square" alt="lexicon">
  <img src="https://img.shields.io/badge/sources-7%20packs-9CCC65?style=flat-square" alt="sources">
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

# 3. Generate text with the v0.4.0 checkpoint
bash ./scripts/run_generate.sh "жақсы адам" 24 1.0 0 0.9 1.2
#                              prompt        ^^  ^   ^   ^   ^
#                                            new temp tk topp rep_pen
```

## Sample generations

From the v0.4.0 checkpoint (24.2M params, 20,000 training steps on 244k-sample corpus). A mix of nucleus and `temp=0.8` results to show the model's range:

| Prompt | Generated |
|---|---|
| жақсы адам | жақсы адам мағына береді. |
| ол | ол жазады. |
| олар | олар жүреді. |
| үлкен қала | үлкен қала айтады. |
| үлкен жақсы адам | үлкен жақсы адам оқыйды. |
| мектеп туралы | мектеп туралы мәртебе нақтылайды. |
| мен қазір | мен қазір арттырады. |

Complete grammatical Kazakh sentences now appear consistently at low temperatures — the v0.4.0 fix (literary corpus, Abai integration, no-short-synth filter) gave the model enough signal to finish a clause. Greedy generations still terminate early (the 24M model is capacity-bound on 4.09M training tokens). Reproducible via `bash ./scripts/run_generation_showcase.sh` → `data/training/generation_showcase_report.json`. Coherent chat-level output remains a v0.5.0 target.

## Full training pipeline

```bash
# 1. Fetch authentic Kazakh text sources
bash ./scripts/fetch_tatoeba_kazakh.sh
bash ./scripts/fetch_wikipedia_kz.sh
bash ./scripts/fetch_common_voice_kk.sh
bash ./scripts/fetch_cc100_kk.sh
bash ./scripts/fetch_abai_wikisource.sh

# 2. Process each source into pack JSON
cargo run --release --bin process_tatoeba_kazakh
cargo run --release --bin process_wikipedia_kz -- data/external/wikipedia_kz_plain.txt data/curated/wikipedia_kz_pack.json 100000
cargo run --release --bin process_common_voice_kk
xzcat data/external/cc100_kk.txt.xz | target/release/process_cc100_kk data/curated/cc100_kk_pack.json 50000
cargo run --release --bin process_abai_wikisource

# 3. Generate 100,000 FSM-validated synthetic sentences (min 3 words)
bash ./scripts/run_synth_sentences.sh 100000

# 4. Combine all packs into the unified corpus (dedup included)
bash ./scripts/run_unified_corpus_assembly.sh

# 5. FSM-segment every word into morphemes (char fallback otherwise)
bash ./scripts/run_pretokenize_corpus.sh

# 6. Learn BPE merges (lexicon-seeded vocab, target 8192)
bash ./scripts/run_train_bpe.sh 8192

# 7. Encode with deterministic train/val split
bash ./scripts/run_encode_corpus.sh

# 8. Train the 24M transformer (~8h on M2 Metal; periodic checkpoint every 2000 steps)
bash ./scripts/run_train_baseline.sh 20000 8 128 3e-4 400 500

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

## Stats (v0.4.0)

| Component | Value |
|---|---|
| Lexicon roots | **211** (10 POS) |
| FSM rules | **422** |
| Eval segmentation examples | **464** (100% match rate) |
| Authentic Kazakh sources | **5** (Tatoeba, Wikipedia, Common Voice, CC-100, Abai) |
| Total pack sources | **7** (+ curated clean, proverbs, synthetic FSM) |
| Training samples | **244,625** unique (232,524 train + 12,101 val) |
| Training tokens (encoded) | **4,094,435** (0.00% unknowns, 100.00% roundtrip) |
| BPE vocabulary | **8,192** |
| BPE compression | **3.27×** |
| Model parameters | **24.2M** (hidden 512, layers 5, heads 8, ffn 2048) |
| Wall time (M2 Metal, 20k steps, seq=128 batch=8) | **~8h** |
| Periodic checkpoints | **every 2000 steps** (crash-resilient since v0.4.0) |
| **Validation perplexity** | **1691.89** (12,101 held-out samples, v0.4.0 model) |

## v0.5.0 — FST participles + converbs + vowel-stem coalescence

Expansion of the v0.4.5 FST. Adds participles, converbs, and the vowel-final-stem coalescence rules (оқы + PRES = оқиды). The FST now covers most non-finite forms of Kazakh verbs.

| Component | Value |
|---|---|
| New crate | [`adam-kernel-fst`](crates/adam-kernel-fst/) |
| Unit tests | **68 passing** (up from 55 in v0.4.5) |
| Archiphonemes | 12 |
| Phonological rules | 54 catalogued; 22+ implemented |
| Suffix templates | 30 (+ 3 participles, + 2 converbs since v0.4.5) |
| Lexicon (v1) | **14,296 entries** = 4,454 curated + 11,919 Apertium-imported |
| Roundtrip coverage | **36,238 / 36,238 = 100.0%** (full-lexicon synthesise → analyse) |
| CLI binary | [`adam_fst`](crates/adam-kernel-fst/src/bin/adam_fst.rs) with `synth`, `analyse`, `stats` |

Examples:
```bash
$ target/release/adam_fst synth --root бала --plural --case dat
балаларға

$ target/release/adam_fst synth --root жаз --voice passive --negation --tense past --person 3
жазылмады

$ target/release/adam_fst analyse мектебім
noun: мектеп +P1Sg

$ target/release/adam_fst synth --root оқы --tense present --person 3
оқиды

$ target/release/adam_fst synth --root жаз --tense ParticiplePast
жазған
```

The FST is not a language model; it handles morphology deterministically so a future small LM (v0.5.5+) doesn't need to waste capacity learning suffix patterns.

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
