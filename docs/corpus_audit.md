# Corpus Audit — v1.1.5 Baseline

This page documents the starting position for the v1.x corpus expansion toward a 100 M+ word Kazakh corpus that can realistically train a compact LM as the v2.0 `Intent::Unknown` fallback.

Run `cargo run --release -p adam-corpus --bin corpus_audit` to regenerate `data/corpus_audit_report.json`.

## Baseline (current)

| pack | samples | words | unique words | Kazakh purity | dup samples | added |
|---|---:|---:|---:|---:|---:|---|
| tatoeba_kazakh | 4,058 | 24,643 | 9,709 | 98.12 % | 0 | v0.1.1 |
| **wikipedia_kz** | **150,036** | **1,613,306** | **155,917** | **99.99 %** | **0** | **v1.3.0 (re-extract)** |
| common_voice_kk | 6,108 | 36,397 | 10,575 | 99.91 % | 0 | v0.1.6 |
| cc100_kk | 50,000 | 602,144 | 74,333 | 96.59 % | 0 | v0.4.0 |
| abai_wikisource | 2,253 | 20,303 | 8,209 | 99.81 % | 0 | v0.4.0 |
| kazakh_proverbs | 80 | 349 | 245 | 100.00 % | 0 | v0.4.0 |
| synthetic_sentences | 100,000 | 403,558 | 15,880 | 98.79 % | 207 | v0.4.0 |
| kazakh_classics | 111 | 926 | 710 | 100.00 % | 0 | v1.2.0 |
| **TOTAL** | **312,646** | **2,851,629** | **224,301** | **97.99 %** | **207** | |

- **Target:** 100 M words
- **Gap:** 97.15 M words
- **Expansion factor needed:** **~35×** (was 45× at v1.1.5 baseline)
- **v1.3.0 contribution:** +613k words from re-extracting the full Wikipedia dump with the loanword-density filter; Wikipedia purity jumped from 95.92 % → 99.99 % because the filter now catches high-density Russian-loanword articles that slipped through before.

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

## Expansion plan (v1.2.0 → v1.5.0)

| release | source | target added | cumulative |
|---|---|---:|---:|
| v1.1.5 (here) | audit + baseline | — | 2.24 M |
| v1.2.0 | Kazakh classical literature — OCR of Әуезов, Жамбыл, Ыбырай, other early-20c poets (public domain) | +10–15 M | ~15 M |
| v1.2.5 | cleanup + dedup of v1.2.0 ingestion | — | ~15 M |
| v1.3.0 | full Qazaq Wikipedia dump (beyond the current 100k sample subset) | +30 M | ~45 M |
| v1.3.5 | Wikipedia purity tightening + de-loanwording | — | ~45 M |
| v1.4.0 | Kazakh government corpora (egov.kz, akorda.kz, bnews.kz — select long-form content) | +15–20 M | ~60 M |
| v1.4.5 | Lexicon auto-extraction from the expanded corpus (new curated roots) | — | ~60 M |
| v1.5.0 | reach 100 M+ via additional classical literature or filtered news corpora | +40 M | **100+ M** |

## Purity filter (for all new ingestions)

Any new source MUST pass the Kazakh-purity filter before landing in `data/curated/`:

1. **Drop tokens with Russian-only letters** (ё, ф, ц, ч, щ, ъ, ь, э) — strong loanword signal.
2. **Drop tokens ending in loanword suffixes** (-ция, -логия, -графия, -тика, -изм, -альный, …).
3. **Drop samples with high loanword density** (> 10 % of words flagged).
4. **Audit after ingestion** with `corpus_audit` — report purity delta and confirm the pack raises, not lowers, the overall Kazakh-purity score.

See the `project_corpus_purity_directive` memory for the full rationale.
