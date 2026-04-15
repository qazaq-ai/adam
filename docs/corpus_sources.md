# Corpus sources

Accepted external data sources for the `adam` training corpus and their licenses.

## Policy

The `adam` project is licensed BUSL-1.1 (converts to Apache-2.0 on 2029-01-01). Training corpus sources must have licenses compatible with **training a language model and redistributing its weights**. Source text itself is **not** redistributed unchanged — only FSM-validated pretokenized sequences and encoded ids derived from it.

Required actions for every external source:

1. **License recorded** in this file with URL to the canonical license text.
2. **Attribution** preserved in the JSON pack `source_id` field.
3. **No binary dump** of the raw source committed to git; a `scripts/fetch_*.sh` script describes how to obtain it reproducibly, and `data/external/` is gitignored.

Sources incompatible with the policy are documented in [corpus_policy.md](corpus_policy.md) and excluded.

## Accepted sources

### Tatoeba (CC-BY 2.0 FR)

- **Scope**: sentence-per-line human-curated translations.
- **Kazakh subset**: sentences where `lang_code == "kaz"`.
- **Download**: `scripts/fetch_tatoeba_kazakh.sh` → `data/external/tatoeba_kazakh_sentences.tsv`.
- **License**: Creative Commons Attribution 2.0 FR — <https://creativecommons.org/licenses/by/2.0/fr/deed.en>.
- **Attribution**: "Tatoeba.org (CC-BY 2.0 FR) — <https://tatoeba.org>".
- **source_id** in packs: `tatoeba_kazakh_<yyyymmdd>`.
- **Added in**: v0.1.1 (Phase 6a).

### Kazakh classical proverbs (public domain)

- **Scope**: 80 classical мақал-мәтелдер, pre-1923 or otherwise public domain.
- **Redistributed**: yes — embedded directly in `data/curated/kazakh_proverbs_pack.json`.
- **License**: public domain (oral tradition, no enforceable copyright).
- **source_id** in packs: `classical_kazakh_proverbs`.
- **Added in**: v0.0.77 (Phase 3d).

### Synthetic (generated in-repo)

- **Scope**: FSM-validated sentences from `synth_sentences` binary over our own lexicon + rules.
- **License**: Same as this repo (BUSL-1.1 → Apache-2.0 on 2029-01-01).
- **source_id** in packs: `generated_template_pool_v1`.
- **Added in**: v0.0.74 (Phase 3a).

### Curated hand-written (internal)

- **Scope**: small (~100) hand-written example sentences for domain-specific packs.
- **License**: Same as this repo.
- **source_id** in packs: e.g. `curated_general_kazakh`.

### Kazakh Wikipedia (CC-BY-SA 4.0)

- **Scope**: Kazakh-language Wikipedia article body text, plain-text extracted from the XML dump.
- **Download**: `scripts/fetch_wikipedia_kz.sh` → `data/external/wikipedia_kz_plain.txt` (~668 MB).
- **License**: Creative Commons Attribution-ShareAlike 4.0 — <https://creativecommons.org/licenses/by-sa/4.0/>.
- **Attribution**: "Wikipedia contributors, Kazakh Wikipedia (CC-BY-SA 4.0) — <https://kk.wikipedia.org>".
- **source_id** in packs: `wikipedia_kz_article_<n>`.
- **Added in**: v0.1.4 (Phase 6c).

### Common Voice Kazakh (CC0-1.0)

- **Scope**: Mozilla Common Voice Kazakh sentence-collector prompts (text only; audio is not used).
- **Download**: `scripts/fetch_common_voice_kk.sh` → `data/external/common_voice_kk_sentences.txt`.
- **License**: CC0-1.0 (public domain dedication) — <https://github.com/common-voice/common-voice/blob/main/LICENSE>.
- **Attribution**: none required; acknowledgement retained as good practice — "Mozilla Common Voice contributors".
- **source_id** in packs: `common_voice_kk_line_<n>`.
- **Added in**: v0.1.6 (Phase 6d).

## Under consideration (not yet integrated)

### Adilet.zan.kz government documents

- **Status**: license unclear; awaiting clarification. Kazakh government documents are typically public domain but direct use terms are not explicit.
- **Planned**: v0.1.4+ pending legal review.

## Rejected sources

None so far.

## Adding a new source — checklist

1. Verify license is compatible with model training + weight redistribution.
2. Add a `scripts/fetch_<source>.sh` that downloads deterministically (versioned URL when possible).
3. Add a processor binary (or extend an existing one) that produces our pack JSON schema.
4. Document in this file above — scope, license URL, attribution, source_id.
5. Include in `data/curated/adam_training_corpus_manifest.json`.
6. Re-run `scripts/run_unified_corpus_assembly.sh`.
7. Re-pretokenize + retrain BPE if vocabulary significantly grows.
8. Add `jq empty` validation to `scripts/validate_foundation.sh`.
