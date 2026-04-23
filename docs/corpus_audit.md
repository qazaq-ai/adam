# Corpus Audit

This page documents the Kazakh corpus position across releases. **Current as of v3.9.5**: **4.57 M committed / 77.9 M local words** across 9 committed source packs. The v1.x expansion plan below is **fully executed** (see "Historical baseline" section). v3.3.0 – v3.5.0 added 10 Kazakh high-school textbooks OCR'd via tesseract-kaz, contributing 28 110 samples / 434 k words.

> **Note on v2.0 direction.** The original plan targeted a 100 M+ word corpus to *train* a compact neural LM as the `Intent::Unknown` fallback. v2.0 committed to a different architecture: **retrieval + composition**, not trained-neural. v3.9.0+ added a third facet: human-authored **World Core** knowledge packs (`data/world_core/*.jsonl`) merged into `facts.json` with `ConfidenceKind::HumanApproved`. The corpus is fuel for the morpheme-indexed retrieval engine (`adam-retrieval`) and the text-pattern fact extractor (`adam-reasoning`), not for gradient descent. See [`project_retrieval_not_neural_v2`](roadmap.md#post-v10-direction).

Run `cargo run --release -p adam-corpus --bin corpus_audit` to regenerate `data/corpus_audit_report.json` (committed corpus) or add `--local` to include gitignored Wikipedia + CC-100 shards.

## Current position (v3.9.5, committed)

| pack | samples | words | purity | added |
|---|---:|---:|---:|---|
| tatoeba_kazakh | 4,058 | 24,643 | 98.12 % | v0.1.1 |
| wikipedia_kz (shard 01, committed) | 150,036 | 1,613,306 | 99.99 % | v1.3.0 re-extract |
| common_voice_kk | 6,108 | 36,397 | 99.91 % | v0.1.6 |
| **cc100_kk (shard 01, committed)** | **140,000** | **1,140 k** | **98.36 %** | **v1.5.0 re-extract** |
| abai_wikisource | 2,253 | 20,303 | 99.81 % | v0.4.0 |
| kazakh_proverbs | 80 | 349 | 100.00 % | v0.4.0 |
| synthetic_sentences | 100,000 | 403,558 | 98.79 % | v0.4.0 |
| kazakh_classics | 111 | 926 | 100.00 % | v1.2.0 |
| **kazakh_textbooks (10 books OCR'd)** | **28,110** | **434,581** | **high** | **v3.3.0 – v3.5.0** |
| **TOTAL (committed)** | **~430 k** | **~4.57 M** | **≥ 98 %** | |

### Local corpus (with gitignored shards)

33 additional CC-100 shards (v1.5.0) + 9 Wikipedia shards (v1.3.5) are gitignored and regenerable via `--full` mode on their processors. Local totals:

- **~77.9 M words** across all packs + shards
- **1.72 M unique roots** (prefix-match basis)
- Gap to nominal 100 M target: **~1.3×** (v1.5.0 baseline)

### Morpheme coverage (v1.5.5 baseline, still current at v2.3)

- **79.48 %** of committed-corpus words begin with a Lexicon root (≥ 3 chars). See [morpheme_coverage_baseline](../CHANGELOG.md) memory for top uncovered items.

### Structured facts (v2.1 → v2.3)

- **15 extracted facts**, 2 predicates (`IsA`, `Has`), 0 imprecisions, deterministic provenance. See `data/retrieval/facts.json`.
- **Lexical Graph**: 29 nodes, 15 edges. See `data/retrieval/lexical_graph.json`.

## File size constraint

The full reprocess of `wikipedia_kz_plain.txt` (638 MB source) yields **1,395,801 samples / ~15 M words**, but that JSON file exceeds GitHub's 100 MB hard limit. v1.3.0 commits the first 150 k samples (~49 MB) as the canonical pack. The remaining ~1.25 M samples will be exposed via sharding (`wikipedia_kz_shard_N_pack.json`, each ≤ 50 MB) in v1.3.5. Local users can regenerate the uncapped pack any time:

```bash
cargo run --release -p adam-corpus --bin process_wikipedia_kz
# → data/curated/wikipedia_kz_pack.json (~440 MB, do not commit)
```

## Interpretation

- **Data volume is the real bottleneck**, not model size. Chinchilla-optimal training for a 24 M-parameter model is ~480 M tokens; we need at least the 100 M target before any LM training becomes worthwhile.
- **Abai + Common Voice + Proverbs are small but pristine** (>99 % purity) — the "literary core" we want the LM to weight highly.
- **CC-100 and Wikipedia carry the volume but have 3–4 % loanword contamination**. v1.2.0+ ingestion should tighten the purity filter on these sources before passing further text through.
- **Synthetic sentences** plateau at ~16k unique vocabulary despite 100k samples — expected, since the generator combines a fixed template set with a bounded root Lexicon. Not a path to vocabulary growth; useful only for morphological coverage.
- **Wikipedia is the single biggest single-source opportunity** — the full Kazakh Wikipedia dump (~200k articles × ~500 words average) could approach the 100 M target alone, though purity-gated samples will cut that substantially.

## Historical expansion plan (v1.1.5 → v1.5.0 — delivered)

| release | what landed | committed/local |
|---|---|---|
| v1.1.5 | audit + baseline | 2.24 M committed |
| v1.2.0 | Kazakh classical literature (Ыбырай + Мағжан public-domain PD) | +0.9 k samples (cleaner than planned; OCR deferred) |
| v1.3.0 | full Wikipedia re-extract, streaming + 10 % loanword filter | 2.85 M committed |
| v1.3.5 | Wikipedia sharding (committed shard 01 + gitignored 02–10) | 16 M local |
| v1.4.0 | FST-NER + DST + predicate-copula (not corpus work) | — |
| v1.4.5 | Lexicon +20 modern nouns | — |
| v1.5.0 | CC-100 re-extract, streaming + sharding | 4.01 M committed / **77.9 M local** |
| v1.5.5 | morpheme-coverage audit (79.48 % baseline) | — |
| v1.5.5→v2.0 | committed corpus stable; focus shifted to retrieval engine | — |

**Government Kazakh sources** (akorda.kz, egov.kz, bnews.kz) — originally v1.4.0 plan — were de-prioritised once retrieval-over-corpus delivered the quality the planned volume was meant to support. Still on the post-v2.x shortlist.

## Purity filter (for all new ingestions)

Any new source MUST pass the Kazakh-purity filter before landing in `data/curated/`:

1. **Drop tokens with Russian-only letters** (ё, ф, ц, ч, щ, ъ, ь, э) — strong loanword signal.
2. **Drop tokens ending in loanword suffixes** (-ция, -логия, -графия, -тика, -изм, -альный, …).
3. **Drop samples with high loanword density** (> 10 % of words flagged).
4. **Audit after ingestion** with `corpus_audit` — report purity delta and confirm the pack raises, not lowers, the overall Kazakh-purity score.

See the `project_corpus_purity_directive` memory for the full rationale.
