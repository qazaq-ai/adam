# Repository Layout

## Crates (9 total)

- `adam-kernel`
  shared identity, versioning, and foundation contracts (L0)
- `adam-kernel-fst`
  **FST morphology** (L0): phonology (11 archiphonemes + 22+ twol rules; v2.3 glide-vowel fix), morphotactics (36 suffix templates incl. v1.4.0 predicate-person copula), synthesiser + inverse parser, Lexicon loader, `adam_fst` CLI
- `adam-tokenizer`
  pre-tokenizer + BPE trainer + encoder / decoder (L1)
- `adam-corpus`
  source acceptance, streaming processors (Wikipedia KZ, CC-100, Tatoeba, Common Voice, classics), `corpus_audit`, `morpheme_coverage` (v1.5.5), synthetic-sentence generation (L1)
- `adam-eval`
  evaluation suite, benchmark manifests, delta reports (L1)
- `adam-dialog`
  **dialog pipeline** (L1): 26-intent recognisers, `Conversation` session state + DST (`active_intent`, `intent_history`), session-aware template planner, slot-expanding realiser, `adam_chat` REPL + `adam_demo` scripted walkthrough
- `adam-retrieval` (v1.6.0+)
  **retrieval engine** (L1): `MorphemeIndex` inverted index, composite deterministic `rank`, `compose::compose_with_city` opt-in in-sample swap, `build_morpheme_index` binary
- `adam-reasoning` (v2.1+)
  **ILMRR reasoning bootstrap** (L1): structured `Fact` extraction via 3 pattern matchers (copula → IsA, locative-existential → LivesIn, possessive-existence → Has), Lexical Graph projection (v2.3), `extract_facts` + `build_lexical_graph` binaries
- `adam-train`
  legacy transformer baseline (v0.1 – v0.4); kept for CI regression, not on the current codepath (L2)

## Data

- `data/raw/`
  source registry, scoring rules (policy inputs only, no raw text)
- `data/external/`
  raw fetched text from authentic Kazakh sources (gitignored if > 50 MB)
- `data/curated/`
  per-source Kazakh-only packs + training / validation manifests (see `data/curated/README.md`)
- `data/lexicon_v1/`
  the authoritative Lexicon: Apertium import + curated roots (~16.4 k total; v2.2 purged 87 intervocalic-voicing-duplicate Apertium pollutions; see `data/lexicon_v1/README.md`)
- `data/tokenizer/`
  BPE vocab + merges + segmentation rules + curated root list (4.4 k)
- `data/dialog/`
  dialog-layer template repository (`templates/v1.toml`, 31 families incl. v1.8.0 session-aware evidence templates and v1.9.5 adapted-evidence family; see `data/dialog/README.md`)
- `data/retrieval/`
  morpheme inverted index (v1.6.0+), committed `facts.json` (v2.1+), committed `lexical_graph.json` (v2.3+)
- `data/eval/`
  benchmark + tokenizer-experiment manifests, held-out datasets, delta reports
- `data/training/`
  legacy transformer artifacts (v0.4.0 checkpoint + assembly / consistency / delta reports); see `data/training/README.md`
- `data/foundation/`
  top-level foundation-overview report aggregating every layer

See [data/README.md](../data/README.md) for load-bearing / generatable flagging per subdirectory.

## Scripts

- `scripts/validate_foundation.sh` — full CI-level validation; runs every layer's regression tests.
- `scripts/bump_foundation_version.sh <x.y.z>` — workspace version bump with drift verification.
- `scripts/cut_release.sh` — create / tag / push a release from a clean working tree.
- `scripts/verify_release_version.sh` — enforce strict `x.y.z` tag format.
- `scripts/fetch_*.sh` — fetch external sources (Tatoeba, Wikipedia KZ, Common Voice KK, CC-100, Abai Wikisource).
- `scripts/run_*.sh` — per-stage regenerators (synth sentences, BPE training, encode, evaluation, foundation-overview assembly).

## Documentation

- `docs/roadmap.md` — version-by-version history, lifecycle view of the two eras.
- `docs/kazakh_grammar/` — linguistic reference (phonology / morphology / syntax / lexicon sources / work plan / Apertium twol catalogue) + the dialog architecture spec.
- `docs/corpus_policy.md`, `docs/corpus_sources.md`, `docs/curation_workflow.md` — corpus curation.
- `docs/tokenizer_policy.md`, `docs/tokenizer_experiment_plan.md` — tokenizer.
- `docs/evaluation_policy.md`, `docs/eval_baseline.md` — evaluation.
- `docs/training_baseline.md` — legacy transformer baseline context.
- `docs/foundation_scope.md` — overall project scope.
- `docs/source_classification.md`, `docs/source_scoring.md` — source-quality framework.
