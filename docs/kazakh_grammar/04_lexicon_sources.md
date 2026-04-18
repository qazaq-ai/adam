# Lexicon Sources — where the 10k–50k roots come from

Status: **Week 3 prep**. Catalogue of sources we will draw on; full import scripts are Week 3 deliverables.

## 1. Primary source: Apertium-kaz

**Repository**: https://github.com/apertium/apertium-kaz

**Contents**:
- `apertium-kaz.kaz.lexc` — Lexicon with POS tagging and morphological class; ~30k–50k entries
- `apertium-kaz.kaz.twolc` — Two-level phonological rules (our phonology target)
- `apertium-kaz.kaz.rlx` — Constraint Grammar disambiguation (syntax disambiguation evidence)
- `apertium-kaz.post-kaz.dix` — post-lexical dictionary
- Test corpora

**License**: GPL-3.0. Compatible with our research use. For our release model (BUSL-1.1), we **don't redistribute the lexicon content directly**; we use it as a reference to build our own schema. Individual lemma-POS triples are factual data and should not themselves be copyrightable — but we will document the derivation carefully.

**How we use it**:
- Week 3: Fork + explore repository locally (gitignored path).
- Extract lemma list with basic POS tag.
- Map Apertium's `N`, `V`, `Adj`, `Adv`, `Postp`, etc. to our `SegmentationPartOfSpeech` enum.
- Spot-verify top 2000 most-frequent entries against corpus evidence.
- Generate `data/tokenizer/segmentation_roots.json` entries.

## 2. Secondary source: HFST Kazakh

**Repository**: https://github.com/giellalt/lang-kaz or similar GiellaLT packages.

**Use**: cross-check Apertium rules against HFST's formalisation; especially useful for irregular forms.

## 3. Academic grammars (human-readable)

- Ысқақов А. «Қазіргі қазақ тілі: морфология» (1991) — reference for morpheme inventory
- Маманов Ы. «Қазіргі қазақ тілі» — reference grammar
- Кенесбаев С. «Қазақ тілінің фразеологиялық сөздігі» — idiom dictionary, useful for lexicon edge cases

**Acquisition**: digital scans, published institutional copies. Our study usage is transformative research use.

## 4. Open-access corpora

| source | size | licence | use |
|---|---|---|---|
| Kazakh Wikipedia | ~300k articles | CC-BY-SA 4.0 | frequency data, new roots, NE |
| CC-100 Kazakh | ~500M tokens | CC BY-SA 4.0 | frequency data only |
| HPLT Monolingual Kazakh | large | variable | future consideration |
| Leipzig Corpora (Kazakh) | 1M+ sentences | CC-BY | already used |
| OSCAR Kazakh | ~200M | variable | future |
| Turkic NER & POS corpora | smaller | academic | verification of POS tagging |

## 5. Proper nouns

Kazakh proper nouns (toponyms, person names, historical figures) need a separate sub-lexicon because:
- They don't always follow native phonological harmony (they come from many source languages).
- They take full morphological inflection.
- They dominate real-world text frequency.

Proposed approach (Week 4):
- Extract top 5k proper nouns by frequency from Wikipedia + news corpora.
- Store in `data/tokenizer/proper_nouns.json` separate file.
- FST treats them as a special root class that bypasses harmony check but participates in case/number inflection.

## 6. Loanwords

Russian-origin loanwords are pervasive. Two options:

**Option A — Integrate**: accept that modern Kazakh includes ~30% Russian-origin vocabulary; include these in the main lexicon with correct inflection rules.

**Option B — Exclude (user preference)**: build a "pure Kazakh" model that rejects/skips loanwords, per the user's stated v0.5.x goal.

Tentative plan: support both via a flag. Default v1.0.0 model is Option B (pure Kazakh); a follow-on v1.1.0 can extend to Option A for practical coverage.

## 7. Corpus-derived lexicon (our 113k candidates)

The existing `data/curated/root_candidates_report.json` contains 113,717 unique greedy-stripped candidates from our source packs. These will be **cross-referenced** with Apertium-kaz during Week 3:

- Candidates present in Apertium → import with Apertium's POS/class
- Candidates absent from Apertium → manual review queue (rank by frequency)
- Apertium entries absent from our corpus → add to lexicon but note as "low observed frequency"

This dual-track import gives us both breadth (from Apertium) and depth (our corpus-verified frequency signals).

## 8. Target lexicon sizes

| milestone | root count | verb count | adjective count | proper nouns |
|---|---|---|---|---|
| current v0.5.1 | 4,454 | ~500 | ~50 | 0 (mixed in) |
| Week 3 target | 15,000 | 3,000 | 2,000 | 3,000 |
| Week 4 target | 30,000 | 5,000 | 3,500 | 5,000 |
| v1.0.0 final | 40,000–50,000 | 7,000 | 5,000 | 8,000 |

These figures are bench-marked against Apertium-kaz actual entries. The v1.0.0 lexicon should be a **superset of Apertium's union with our corpus evidence**.
