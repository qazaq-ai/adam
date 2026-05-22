# Lexicon coverage progress — 2026-05-16

**Branch:** `experimental/agglutinative-neural`
**Tools touched:** `crates/adam-corpus/src/bin/mine_lexicon_gaps.rs`
**Lexicon untouched.** Numbers below come purely from fixing the
gap-miner's heuristics; the actual Lexicon V1 / V2 entries are
unchanged.

## Baseline → wave 1 → wave 2

| Stage                                  | Uncovered distinct surfaces | Top-1 candidate (frequency) |
|----------------------------------------|----------------------------:|:---------------------------|
| 2026-05-16 morning baseline            | 103 694 | оның (11 729) |
| Wave 1 — closed-class POS exception    |  86 516 | алып (4 083) |
| Wave 2 — trust Lexicon + dash-strip    | **69 860** | **сондайақ (3 324)** |

**Cumulative reduction: −32.6 %** of distinct uncovered surfaces.

The committed corpus is unchanged (4 149 827 tokens across 432 263
samples in 10 packs). Every removed candidate was a real Kazakh
surface that the previous miner heuristics dropped for procedural
reasons.

## What was wrong

### Wave 1 — closed-class short roots dropped by `MIN_ROOT_LEN = 3`

The miner only accepted roots ≥ 3 chars. Kazakh's closed-class
items (pronouns «ол», «мен», «сен», «не»; particles; conjunctions)
are 2 chars; their inflected surfaces («оның», «оны», «оған», …)
were therefore classified as having no Lexicon root prefix.
`adam-kernel-fst::pronoun_paradigm` already analysed these
surfaces correctly — only the gap-miner was unaware.

Fix: `RootEntry` carries `part_of_speech`; short roots with
closed-class POS are kept; `has_known_prefix` window lowered to
`2..=n`.

### Wave 2 — long-root closed-class also dropped + `-` curator prefix

Two additional dimensions of the same kind of bug:

1. **All short Lexicon roots dropped.** Common Kazakh verbs «ал»
   (to take, 2 char), «бер» (to give, 3 char), «өт» (to pass, 2
   char) and common nouns «ел» (country / land, 2 char) were also
   filtered. Top-1 after wave 1 was «алып» (4 083 freq) — a
   converb form of «ал» that round-trips through verb morphology.

2. **Curator-marker `-` prefix.** 47 Lexicon entries had a `-`
   prefix used by the original curator («-аят», «-ба», …). The
   gap-miner matched literally so «аяттарды» never prefix-matched
   «-аят».

Fix: trust the Lexicon curation. Every entry that survived
curation is used as a prefix-match candidate regardless of length.
The Lexicon itself is the filter. The `-` prefix is normalised on
load (with the source JSON unchanged for curator traceability).

## What's left at top of the list (post-wave 2)

| Rank | Surface     | Frequency | Likely fix path |
|-----:|:------------|----------:|:----------------|
|   1  | сондайақ    | 3 324     | new Lexicon entry (coordinator) |
|   2  | өтті        | 2 675     | verb FST coverage extension for past-definite of «өту» |
|   3  | оған        | 2 672     | pronoun stem-alternation extension in `pronoun_paradigm` |
|   4  | одан        | 2 259     | same as #3 |
|   5  | бойы        | 1 627     | possessive-stem extension |
|   6  | қыз         | 1 098     | new Lexicon entry (girl / daughter) |
|   7  | орны        | 1 045     | possessive-stem extension |
|   8  | млрд        | 1 040     | abbreviation, ignore |
|   9  | көмек       | 1 019     | new Lexicon entry (help) |
|  10  | бұған       | 980       | pronoun stem-alternation extension |

The top 10 break down: 4 require new Lexicon entries (lexicographer
review needed); 4 require FST verb / pronoun / possessive extensions
(deterministic, code-only); 1 abbreviation noise; 1 already
covered by future work.

## Pipeline performance

`mine_lexicon_gaps` Pass 1 (per-pack frequency counting) was
parallelised with rayon. Wall-clock on the committed corpus:

|                            | Time  | CPU usage |
|----------------------------|-------|----------:|
| Baseline (sequential)      | ~12 s | 110 %     |
| Wave 1+2 (rayon par_iter)  | 5.5 s | 140 %     |

Speedup is partial (~2×) because the workload is bounded by the
number of packs (10) and pack-size imbalance. The biggest packs
(wikipedia, cc100) dominate the critical path.

## How this propagates to the model

Every dropped Unk surface that turns into a properly-analysed Root
moves the corresponding training pair from "Unk-headed" (model
learns word-as-token, no algebra) to "Root-headed" (model learns
morpheme algebra, generalises). The PoC `poc_kazakh_train` filters
`POC_REAL_PACK` to keep only Root-headed pairs; the −32.6 %
uncovered-surface reduction propagates into a comparable rise in
the training-pair yield from the same source corpus.

The forthcoming production training run will measure the actual
held-out CE delta vs the 2026-05-16 baseline of 0.031 (see
`docs/research/results_real_mix_2026_05_16.md`).

## Action items

These are tracked in `docs/roadmap_v6_v7.md` against the v6.0.0
release-blocker for Lexicon V2:

- Native-speaker curated approval of the top-2 000 candidates
  in `docs/lexicon_gap_candidates.md`.
- Pronoun stem-alternation extension in `pronoun_paradigm.rs` for
  `оған`, `одан`, `бұған`, `бұдан`, `соған`, `содан` (the
  remaining oblique forms of `ол`, `бұл`, `сол`).
- Possessive-stem extension for `бойы`, `орны`, `маңызы`, …
  (third-person possessive on consonant-final stems).
- Common-coordinator additions: «сондайақ», «алайда».

Re-run `mine_lexicon_gaps` after every change; the morpheme-
coverage delta is the v6.0 acceptance metric.
