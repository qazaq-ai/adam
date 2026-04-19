# Changelog

All notable changes are tagged in git as `vX.Y.Z`. Versions before 0.1.0 are foundation work — APIs, schemas, and rules may change between any two releases.

## [0.5.0] — 2026-04-19

Expands the v0.4.5 FST to cover Kazakh non-finite verb forms.

- **Vowel-final-stem aorist coalescence** (Apertium rules 17, 18, 19, 20, 30). Stem-final `ы/і` merge with the aorist `{A}` to produce `и` (e.g. `оқы` + PRES + 3 → `оқиды`, not the previous `*оқыа`). Stems ending in other vowels take a `й`-glide (`сөйле` → `сөйлейді`). Past tense on vowel-final stems (`оқы` + PAST + 1SG → `оқыдым`) continues to work without coalescence.
- **Participles** — three new `Tense` variants:
  - `ParticiplePast` — `-{G}{A}н` (`жазған`, `берген`, `қалған`).
  - `ParticipleHabitual` — `-{A}тын` (`жазатын`, `келетін`).
  - `ParticipleFuture` — `-{A}р` (`жазар`, `келер`).
- **Converbs** — two new `Tense` variants:
  - `ConverbPerfect` — `-{Y}п` (`жазып`, `беріп`).
  - `ConverbImperfect` — `-{A}` (`жаза` without personal ending).

Tests: **68 unit tests passing** in `adam-kernel-fst` (up from 55 in v0.4.5). Workspace totals: 150 passing, 4 ignored, 0 failing.

No changes to v0.4.0 transformer baseline or v0.4.5 FST core code.

## [0.4.5] — 2026-04-19

Introduces **adam-kernel-fst**, a pure-Rust deterministic finite-state transducer for Kazakh morphology. This is Phase 1 of the architecture pivot from stochastic transformers to deterministic morphology + small LM (v1.0.0 track). v0.4.0 transformer stack stays untouched; v0.4.5 adds the new FST layer alongside.

Highlights:

- **New crate `adam-kernel-fst`** — phonology module (12 archiphonemes, 20+ of 54 Apertium twol rules implemented), morphotactics module (25 suffix templates covering noun plural/possessive/case and verb tense/voice/negation/person), parser module (`analyse(surface) → Vec<(root, features)>`), lexicon loader (union of 4,454 curated + 11,919 Apertium-imported entries).
- **55 unit tests + 1 smoke test + 4 ignored slow roundtrips**. Slow tests (manual: `cargo test --test roundtrip -- --ignored`) roundtrip the full 14k lexicon on 4 feature combinations: **36,238 / 36,238 = 100.0 %** success.
- **CLI binary `adam_fst`** — `synth`, `analyse`, `stats` subcommands. Hand-rolled arg parsing (no CLI-framework dep).
- **Apertium-kaz import pipeline** (`import_apertium_lexicon` binary) with POS mapping, loanword filter, and prefix-conflict guard.
- **Grammar study notes**: `docs/kazakh_grammar/00_architecture_v1.md`, `01_phonology.md`, `02_morphology.md`, `03_syntax.md`, `04_lexicon_sources.md`, `05_work_plan.md`, `06_apertium_twol_catalogue.md`.

Not yet:

- Vowel-final-stem edge cases (rule 17 coalescence, semivowel у).
- Participles, converbs, infinitive.
- LM over root + feature-bundle sequences (v0.5+ target).
- Replacement of v0.4.0 pipeline (deliberately left untouched).

Workspace totals: 137 tests passing, 4 ignored, 0 failing. CI green.

## [0.4.0] — 2026-04-17

Corpus and infrastructure maturity release. Adds the first classical-literature source (Abai Qunanbayuly via Wikisource, 146 works, 2,253 samples), the first web-crawl source (CC-100 Kazakh, 50,000 samples filtered for Cyrillic-ratio and repetition), and fixes a data-composition bug in the synthetic generator (1- and 2-word outputs dominated the corpus, teaching the model early EOS). BPE retrained at vocab **8,192** with **3.27× compression** on a 12.5M-token pretokenized corpus. Model rolled back from the v0.4.0-failed experiment (27.3M, H=512 L=6) to **24.2M params** (H=512 L=5) after confirming that the L=6 scale-up was undertrained at 3.9M tokens.

Training: 20,000 steps, batch 8, seq 128, 3e-4 peak lr with cosine decay, 8h on M2 Metal at 0.64–0.70 steps/s throughput. First-class reliability: `train_baseline` now writes a periodic checkpoint every 2000 steps after a reboot lost 13k uncheckpointed steps mid-run.

Validation (honest):
- 12,101 held-out samples (larger/harder distribution vs v0.3.0's 1,939)
- mean_ce: 7.43, **perplexity: 1691.89**
- bits/char: **3.28** (v0.4.0-failed: 3.26; v0.3.0: 3.49 — val sets not directly comparable)

Qualitative:
- Complete grammatical Kazakh sentences now appear in `temp=0.8` and nucleus samples (6 of 30 showcase outputs): `жақсы адам мағына береді`, `ол жазады`, `олар жүреді`, `үлкен жақсы адам оқыйды`, `мектеп туралы мәртебе нақтылайды`.
- Greedy still terminates early — expected for a capacity-bound model (24M params × 4M training tokens is ~25× below Chinchilla-optimal data).

v0.5.0 will address the data bottleneck: curriculum-style FSM expansion (L1/L2/L3 difficulty), larger CC-100 sample (50k → 500k), classical-literature expansion (Ауэзов, Нурпеисов, Бөкей locally), and SFT on translated Alpaca for the first instruction-following pass.

## [0.3.0] — 2026-04-15

First capacity scale-up. `ModelConfig::tiny` grows from 4.28M → **20.0M params** (hidden 224→512, layers 4→5, ffn 896→2048, head_dim 28→64). 15,000 training steps on the 39k unified corpus, 3h 45m on MacBook Air M2 Metal. Validation perplexity drops from **1112.31 → 871.30 (−21.7%)** — first meaningful delta since Phase 6a real-text onset. The 4.28M envelope was saturated at Phase 6d; Wikipedia + Common Voice distribution breadth required more model capacity. Peak RSS ~2.5 GB of 8 GB unified memory — headroom confirmed.

## [0.2.0] — 2026-04-15

First minor release after v0.1.0. Full retrain on the 39k unified corpus assembled across Phases 6a–6d. BPE retrained (3,336 merges, 2.80× compression, **0 unknowns, 100.00% roundtrip**). 4.28M model, 15,000 steps, 1h 48m wall time on M2 Metal. Val PPL 1078.68 → 1112.31 (flat; val set is larger and harder — capacity bottleneck now visible).

## [0.1.6] — 2026-04-15

**Phase 6d — Common Voice KK.** Mozilla Common Voice Kazakh sentence-collector integrated (6,108 accepted, CC0-1.0 text only). Unified corpus grows 32,986 → 39,058 unique (+6,072; 4,282 duplicates dedup'd at assembly). Also fixes `scripts/bump_foundation_version.sh`: Cargo.lock is no longer perl-replaced (corrupted transitive deps under naive substring replace); regenerated by `cargo build` after Cargo.toml bump.

## [0.1.5] — 2026-04-15

**Phase 7.1 — Wikipedia-augmented retrain.** 4.28M baseline retrained on the corpus enlarged with Wikipedia KZ. 14,000 steps, ~2h on M2 Metal. Val PPL 626.81 → 1078.68, reflecting a broader, harder val set (Wikipedia sentences are lexically richer than Tatoeba) — honest baseline on the enlarged distribution, not a regression.

## [0.1.4] — 2026-04-14

**Phase 6c — Kazakh Wikipedia.** Plain-text extracted from the kk.wikipedia.org XML dump (~296k articles → 15,000 clean samples after filter; CC-BY-SA 4.0). Unified corpus 17,986 → 32,986 unique. Infrastructure: `scripts/fetch_wikipedia_kz.sh` (bzcat + perl streaming with UTF-8 fix), `process_wikipedia_kz` binary.

## [0.1.3] — 2026-04-14

**Phase 7 — first real-text baseline.** 4.28M model trained on the unified corpus including authentic Kazakh (Tatoeba): 7,000 steps, 61m on M2 Metal, dropout 0.05, grad clipping max-norm 1.0. Explicit `loss.backward() → clip → opt.step` replaces `opt.backward_step`. First honest perplexity on real text: **626.81** (vs 129.49 on pure synthetic — tells us real Kazakh is harder).

## [0.1.2] — 2026-04-14

BPE vocab size bumped 1390 → **4096**. Char-level fallback + Tatoeba real text saturate the larger target.

## [0.1.1] — 2026-04-14

**Phase 6a/6b — first authentic Kazakh source.** Tatoeba Kazakh integrated (4,058 sentences, CC-BY 2.0 FR) via `fetch_tatoeba_kazakh.sh` + `process_tatoeba_kazakh`. Tokenizer adds char-level fallback for FSM-unknown words and leading-punct ▁ marker placement — brings roundtrip to 100% on mixed real/synthetic text.

## [0.1.0] — 2026-04-14

First minor release. The foundation works end-to-end: a Kazakh-first 3.06M-parameter transformer language model trained from scratch on a self-generated, FSM-validated synthetic corpus, evaluated against held-out perplexity, and serving inference with morpheme-aware BPE encode/decode.

### Brand
- Logo `assets/shanraq.svg` integrated into README header.
- README rewritten with centered title, badges, quickstart, and stats.
- `AUTHORS` file added.

### Quality (cumulative since v0.0.85)
- Validation perplexity: **129.49** on a 699-sample held-out set (mean cross-entropy 4.86 over 2532 tokens).
- All 464 segmentation eval examples match at 10000 bps.
- Foundation validation green across 11 layers.

---

## Phase 5 — Training and inference (v0.0.81 → v0.0.92)

### [0.0.92] — Phase 5i: Generation showcase report
- New `generation_showcase` binary: 20 prompts × 3 sampling configs = 60 generations.
- Report artifact `data/training/generation_showcase_report.json`.
- Foundation validation now requires showcase + perplexity reports.

### [0.0.91] — Phase 5h: Top-p + repetition penalty
- `generate` gains nucleus (top-p) sampling and GPT-2-style repetition penalty.
- Backwards-compatible CLI; defaults are no-ops.

### [0.0.90] — Phase 5g: Hyperparameter tuning
- Dropout 0.10 → 0.05 reduces over-regularization on small corpus.
- Gradient clipping (max-norm 1.0) added to `train_baseline`.
- Training extended to 7000 steps with 300-step warmup.
- **Perplexity: 165.98 → 129.49 (−22%).**

### [0.0.89] — Phase 5f: Model scaling + dropout
- ModelConfig::tiny() bumped: hidden 192 → 224, heads 6 → 8, ffn 768 → 896, +dropout=0.1.
- 2.33M → 3.06M params.
- `forward(ids, train: bool)` added to gate dropout on/off.

### [0.0.88] — Phase 5e: Held-out eval + perplexity
- `encode_corpus` extended with deterministic train/val split (FNV hash of sample id).
- New `eval_perplexity` binary writes structured `validation_perplexity_report.json`.
- First baseline: **165.98 perplexity**.

### [0.0.85] — Phase 5d: Inference binary
- `generate` binary: load checkpoint, autoregressive sampling (greedy/temperature/top-k).
- First sentence generated by the model: "жақсы адам аз көрсетеді."

### [0.0.84] — Phase 5c: Training loop
- `train_baseline` binary: AdamW + linear-warmup + cosine-decay LR + safetensors checkpointing.
- First trained checkpoint, training loss 7.94 → 3.39 in 7m on Metal.

### [0.0.83] — Phase 5b: Data loader
- `DataLoader` reads ids pack, produces shifted (input, target) batches on device.
- End-to-end smoke test: forward + cross-entropy loss.

### [0.0.81] — Phase 5a: Candle integration
- Added candle (HuggingFace Rust ML) with Metal backend on macOS, CPU elsewhere.
- `AdamBaseline` decoder-only transformer (initial 2.21M params).
- M2 Metal smoke test passes.

---

## Phase 4 — Tokenizer (v0.0.78 → v0.0.80)

### [0.0.87] — Phase 4d+4e: Lexicon-seeded BPE
- BPE vocab now seeded with all 211 lexicon roots + all 422 rule forms before counting pairs.
- 0% `<unk>` on any FSM-parseable Kazakh word.

### [0.0.80] — Phase 4c: BPE encoder/decoder
- `bpe::BpeTokenizer` module: load vocab+merges, encode text → ids, decode ids → text.
- `encode_corpus` binary writes a training-ready ids pack.
- 100% round-trip on 7,737 samples.

### [0.0.79] — Phase 4b: BPE trainer
- `train_bpe` binary: iterative most-frequent-pair merging over morpheme stream.
- Skips merges across word boundary (right token starts with ▁).
- 567 merges learned from corpus statistics; 2.12× compression.

### [0.0.78] — Phase 4a: Pre-tokenizer
- `pretokenize(text, lexicon, rules)`: morpheme-aware splitting via FSM.
- SentencePiece-style ▁ marker on word-start morphemes.
- Handles standalone punctuation and whole-word fallback.

---

## Phase 3 — Corpus (v0.0.74 → v0.0.77)

### [0.0.86] — Phase 3e: Full POS coverage
- 15 → 30 templates exercising every POS (adverbs, particles, modals, ол/олар, conjunctions).
- Synthetic corpus 10,000 → 18,000 samples.
- Unified corpus 7,737 → 13,929 unique samples.

### [0.0.77] — Phase 3d: Kazakh proverbs
- Added 80 classical мақал-мәтелдер across 23 themes.
- Proverbs bypass FSM-validation policy (archaic morphology); Cyrillic-only check.

### [0.0.76] — Phase 3c: Unified corpus
- `assemble_unified_corpus` binary: dedup + renumber across packs.
- 7,657 unique samples from 10,094 inputs.

### [0.0.75] — Phase 3b: Rich templates
- Generator templates 6 → 15: pronouns with matched person, conjunctions, multi-argument, etc.
- 10,000 sentences (95% yield).

### [0.0.74] — Phase 3a: Synthetic generator
- `synth_sentences` binary: combines FSM lexicon and rules to produce grammatically valid Kazakh sentences.
- Self-validation: every generated word verified by `deterministic_segment_token`.
- FSM fix: removed vowel from `verb_tense_a/e_from_stem` allowed finals (linguistically correct — `й` handles vowel-final aorist).

---

## Phase 2 — Grammatical foundation (v0.0.66 → v0.0.73)

### [0.0.73] — Phase 2h: Modals
- New `Modal` POS, 6 roots: керек, мүмкін, тиіс, шығар, қажет, лайық.

### [0.0.72] — Phase 2g: Nominal predicate
- 16 predicative personal suffix rules: -мын/мін, -сың/сің, -сыз/сіз, -мыз/міз on noun + adjective.
- 3 copula bare lexemes as Particle: еді, екен, емес.

### [0.0.71] — Phase 2f: Adverbs
- New `Adverb` POS, 19 roots: қазір, бүгін, ертең, кеше, тез, баяу, жоқ, иә, etc.

### [0.0.70] — Phase 2e: Numerals
- New `Numeral` POS, 20 cardinals: бір–жүз, мың.
- 4 ordinal suffix rules: -ншы/нші/-ыншы/інші.

### [0.0.69] — Phase 2d: Conjunctions + Particles
- New `Conjunction` POS, 9 roots: және, бірақ, себебі, өйткені, etc.
- New `Particle` POS, 12 roots: ма/ме, ба/бе, па/пе, ғой, да/де, тек, қана, өте.

### [0.0.68] — Phase 2c: Roots + 3sg aorist
- 29 nouns, 13 verbs, 5 adjectives added.
- Critical FSM fix: `tense → person_3sg` was missing for aorist forms (e.g. береді = бер+е+ді). Added rules for both future and negative_future predecessors.
- "й" connector rule for vowel-final verb stems (жасайды).
- Coverage 19.79% → **73.77%** on educational corpus.

### [0.0.67] — Phase 2b: Postpositions
- New `Postposition` POS, 15 roots: арқылы, үшін, туралы, кейін, etc.

### [0.0.66] — Phase 2a: Adjectives
- New `Adjective` POS, 25 roots, 57 inflection rules (mirror of noun rules).
- Coverage 4.56% → 17.93%.

---

## Pre-Phase 2 — Foundation infrastructure

### [0.0.65] — `normalize_token` for accurate coverage
- `coverage_report` strips trailing punctuation before FSM matching.

### [0.0.64] — adam-kernel L0 crate extraction
- Created `adam-kernel`: identity types + Kazakh FSM morphological engine.
- adam-core merged into adam-kernel.
- New `coverage_report` binary measures FSM coverage on real Kazakh text.

### [0.0.63] and earlier
- Initial corpus / tokenizer / eval / training infrastructure.
- Foundation overview report.
- Tiny clean training pipeline with miss audit.
- See git history (`git log v0.0.63 --oneline`) for details.
