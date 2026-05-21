# `docs/archive/`

Historical documents preserved for reference. **Not active
specifications.** Files in this folder describe past plans,
prior architecture iterations, or week-specific operational
plans whose time horizon has passed. They are kept here rather
than deleted because:

  - Older CHANGELOG entries link to them by relative path.
  - The `docs/roadmap.md` version history references them.
  - Their content occasionally provides "why was this done
    that way" context for later code archaeology.

Current active documents live one level up in `docs/`.

## Archived items

| File | Era | Reason archived |
|---|---|---|
| `architecture_v2.md` | 2026-04 | Superseded by `architecture_v3.md` and `architecture_neural_v6.md`. |
| `educational_program_2026-W19+1.md` | 2026-W19 (May 5–11) | Week-specific plan, time horizon expired. |
| `source_classification.md` | 2026-04-07 | Early-stage corpus-source planning notes; replaced by `corpus_audit.md` and the live `data/corpus/` manifests. |
| `source_scoring.md` | 2026-04-07 | Same family as `source_classification.md`. |
| `tokenizer_dry_run.md` | 2026-04-07 | Pre-v3.0 tokenizer experiment plan; current tokenizer is `crates/adam-tokenizer/` + the FST in `crates/adam-kernel-fst/`. |
| `tokenizer_experiment_plan.md` | 2026-04-07 | Same family. |
| `training_notes_rc4.md` | 2026-05 (rc4) | rc4-era training observations; current RC is rc5. |
| `weekly_plan_2026-W19.md` | 2026-W19 (May 5–11) | Week-specific plan, time horizon expired. |

## What about `lexicon_gap_candidates.md`?

That file (2.2 MB) was a v3.4.0 mining-pass output. It now lives
under `data/lexicon_gap_candidates.md` because it is **data** —
an auto-generated candidate list — not a hand-authored design
document. Same content, more honest filesystem placement.
