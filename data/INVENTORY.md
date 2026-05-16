# Data Inventory — single map of all corpora & artefacts

Generated 2026-05-16 for the agglutinative-neural research arc. Audit
everything that lives under `data/` so contributors don't rediscover
forgotten corpora the hard way (the way `data/raw/rust_book_kk/` was
re-discovered mid-research on 2026-05-16).

Re-run an audit any time you touch `data/`. The README.md in each
subdir is the local authority; this file is the bird's-eye view.

Schema for each row:

- **Path** — absolute repository path
- **Items** — sentence count / entry count / row count
- **Size** — on-disk bytes (rounded)
- **Used by** — which crate/binary or pipeline stage consumes it
- **Status** — `live` (read by current code), `frozen` (read but never
  rewritten), `legacy` (kept for archaeology), `todo` (placeholder)

## Tier 1 — Raw Kazakh-language corpora (training fuel)

Pre-curated text in JSON packs. All sentences passed the per-source
ToS check; loanword-filtered variants live alongside the raw ones with
a `filtered_` prefix.

| Path | Items | Size | Used by | Status |
|---|---|---|---|---|
| `curated/wikipedia_kz_pack.json` | 150 000 sentences | 49 MB | morpheme_index, retrieval, adam_chat | live |
| `curated/cc100_kk_pack.json` | 140 000 sentences | 49 MB | morpheme_index, retrieval | live |
| `curated/synthetic_sentences_pack.json` | 100 000 sentences | 23 MB | retrieval | frozen |
| `curated/filtered_synthetic_sentences_pack.json` | 55 983 sentences | 14 MB | retrieval (loanword-filtered) | frozen |
| `curated/kazakh_textbooks_pack.json` | 28 110 sentences | 9.6 MB | retrieval, eval | live |
| `curated/common_voice_kk_pack.json` | 6 108 sentences | — | retrieval | live |
| `curated/filtered_wikipedia_kz_pack.json` | 5 524 sentences | — | retrieval (loanword-filtered) | live |
| `curated/tatoeba_kazakh_pack.json` | 4 058 sentences | — | retrieval | live |
| `curated/filtered_cc100_kk_pack.json` | 3 251 sentences | — | retrieval | live |
| `curated/abai_wikisource_pack.json` | 2 253 sentences | — | retrieval, eval | live |
| **`curated/rust_book_kk_pack.json`** | **1 543 sentences** | — | adam_chat (technical Kazakh) | live |
| `curated/filtered_common_voice_kk_pack.json` | 930 | — | retrieval | live |
| `curated/filtered_tatoeba_kazakh_pack.json` | 733 | — | retrieval | live |
| `curated/filtered_abai_wikisource_pack.json` | 161 | — | retrieval | live |
| `curated/kazakh_classics_pack.json` | 111 | — | retrieval | live |
| `curated/kazakh_proverbs_pack.json` | 80 | — | retrieval | live |
| `curated/clean_*_core/extension_pack.json` (6 packs) | 9–13 each | small | CI golden samples | frozen |

**Sub-total (live Kazakh text):** ≈ **430 000 sentences** ready to be
consumed; ≈ 200 MB.

## Tier 2 — Source text in `raw/` and `external/`

Pre-pack stage, kept for traceability + regeneration.

| Path | Contents | Size | Status |
|---|---|---|---|
| `raw/rust_book_kk/` | 20 chapters Markdown — full Kazakh translation of *The Rust Programming Language* | 460 KB / 10 277 lines | **live** (processed into `rust_book_kk_pack.json`) |
| `raw/source_registry.json` | Curation policy: which sources are allowed | — | live |
| `raw/source_scoring_rules.json` | Per-source quality weights | — | live |
| `external/textbooks_kz/grade_NN/*.pdf` | School textbooks per grade — **7/95 present, 88 TODO** | 91 MB | partial (see `scripts/textbooks_kz_manifest.json`) |
| `external/3к-20…`, `Алгебра 7…`, `Биология 8…` (loose PDFs) | Same 7 PDFs as above, pre-migration. **Can be deleted once migration is verified.** | 91 MB | duplicate |

## Tier 3 — Derived training artefacts (`curated/`)

Produced by the v0.4 transformer pipeline. The adam-agg-model
research arc does NOT use these yet.

| Path | Items | Used by | Status |
|---|---|---|---|
| `curated/adam_training_corpus_pack.json` | 66 648 | legacy transformer | frozen, gitignored (>50 MB) |
| `curated/adam_pretokenized_corpus_pack.json` | 66 648 | legacy transformer | frozen, gitignored |
| `curated/adam_training_ids_pack.json` | 63 362 | legacy transformer | frozen, gitignored |
| `curated/adam_validation_ids_pack.json` | 3 286 | legacy transformer | frozen, gitignored |
| `curated/shards/` | sharded pre-tokenised wiki | 1.4 GB | gitignored, regenerable |
| `training/adam_baseline_checkpoint.safetensors` | v0.4 transformer weights | — | legacy, frozen |

## Tier 4 — Knowledge graphs (`world_core/` + `retrieval/`)

Powers adam_chat's deterministic factual layer.

| Path | Items | Used by | Status |
|---|---|---|---|
| `world_core/*.jsonl` (61 files) | 3 244 entries / 3 650 facts / 61 domains | adam_chat, extract_facts | live |
| `retrieval/facts.json` | 3 650 facts (merged from world_core) | adam_chat, adam_demo | live |
| `retrieval/derived_facts.json` | 37 062 derivations (10 active rules) | adam_chat reasoning | live |
| `retrieval/morpheme_index.json` | full-corpus morpheme→sentence index | retrieval | live |
| `retrieval/lexical_graph.json` | root-↔-suffix graph | retrieval, adam_chat | live |
| `retrieval/root_affinity.json` | per-root suffix priors | adam_chat | live |
| `retrieval/suffix_chain_priors.json` | bigram chain priors (v4.15+) | adam_chat | live |

## Tier 5 — Linguistic resources

The deterministic backbone shared by everything.

| Path | Contents | Used by | Status |
|---|---|---|---|
| `lexicon_v1/pure_kazakh_roots.json` | 13 606 curated Kazakh roots | adam-kernel-fst, adam-agg-tokenizer, adam-agg-synth | **live** |
| `lexicon_v1/apertium_imported_roots.json` | 11 919 Apertium-derived roots | adam-kernel-fst | live |
| `lexicon_v1/abai_augmented_roots.json` | extra Abai-corpus roots | lexicon merge | frozen |
| `lexicon_v1/dropped_loanwords.json` | rejected-loanword audit trail | CI purity gate | frozen |
| `tokenizer/bpe_vocab.json` + `bpe_merges.json` | 8 192-token BPE for v0.4 transformer | legacy transformer | legacy |
| `tokenizer/segmentation_roots.json` | curated segmentation roots (v0.3-v0.5 era) | adam-kernel-fst, adam-agg-tokenizer | **live** |
| `tokenizer/segmentation_rules.json` | morphotactic rule snapshot | adam-kernel-fst (smoke check) | live |
| `dialog/templates/` | dialog-layer TOML templates | adam_chat | live |

## Tier 6 — Evaluation & benchmarking

| Path | Items | Used by | Status |
|---|---|---|---|
| `eval/cognitive_dialog_dataset.json` | 54 canonical scenarios | adam_chat eval | live |
| `eval/benchmark_manifest.json` | coverage + contract benchmark | CI | live |
| `eval/benchmark_report.json` | latest benchmark result | release docs | live |
| `eval/live_holdout_*.json` (20+) | per-release real-user holdouts | release validation | live |
| `eval/kazakh_foundation_eval_dataset.json` | foundation eval dataset | foundation CI | live |
| `eval/adversarial_dialog_v1.json` | adversarial test cases | adam_chat eval | live |
| `foundation/foundation_report.json` | aggregated foundation score | release docs | live |

## What adam-agg-model (research arc) actually uses

As of commit `33176e5` on `experimental/agglutinative-neural`:

| Source | Used? | Why or why not |
|---|---|---|
| `lexicon_v1/{pure,apertium}_roots.json` | **Yes** | 16 850 roots feed `SynthGenerator` |
| `tokenizer/segmentation_roots.json` | **Yes** | curated path of `AggTokenizer` |
| Live FST synth (53–65 k sequences) | **Yes** | only training source so far |
| `curated/rust_book_kk_pack.json` | **No** | 1 543 sentences of technical Kazakh sitting unused |
| `curated/wikipedia_kz_pack.json` | **No** | 150 k sentences sitting unused |
| `curated/cc100_kk_pack.json` | **No** | 140 k sentences sitting unused |
| `curated/kazakh_textbooks_pack.json` | **No** | 28 k sentences sitting unused |
| `curated/abai_wikisource_pack.json` | **No** | 2 253 sentences sitting unused |
| `world_core/*.jsonl` | **No** | 3 650 typed facts sitting unused |
| `external/textbooks_kz/**.pdf` | **No** | no PDF→text→morpheme pipeline yet |

**The gap.** The research arc trains on ~50 k synth sequences while
**~430 000 real Kazakh sentences sit in `data/curated/`** never seen
by the model. Closing this gap (Tier 1 → adam-agg-model) is the
single highest-ROI next step.

## Pipeline missing pieces

1. **`real_corpus_to_morpheme.rs`** — load a list of pack files,
   tokenise every sentence through `AggTokenizer`, emit
   `Vec<Vec<i64>>` sequences. One binary; reads the same packs the
   v0.4 retrieval pipeline already reads.
2. **`pdf_to_text_kk.rs`** — extract Kazakh text from `external/
   textbooks_kz/**.pdf` (born-digital → `pdftotext`; scans →
   `tesseract -l kaz`). Output joins `curated/textbooks_kz_pack.json`.
3. **Manifest fill-in** — `scripts/textbooks_kz_manifest.json` has
   88 TODO URLs. Sources to scan: opiq.kz, okulyk.kz, bilim-all.kz,
   publisher portals.

## How to grow this file

When adding any new corpus / pack / artefact:

1. Decide which tier it belongs to.
2. Add a row in the relevant table.
3. Make sure the subdir's own README.md links back here.
4. If the new source is significant (> 1 000 items), add a
   one-line note to the "What adam-agg-model uses" table so the
   research arc can quickly see whether it's been wired up.
