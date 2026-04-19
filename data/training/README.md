# Training Artifacts

Transformer training artifacts from the v0.3.0–v0.4.0 baseline. The
project pivoted to a deterministic FST + dialog layer at v0.7.0, so
these files are **legacy** and not on the v0.9.0+ codepath. They are
kept because:

1. `validate_foundation.sh` still exercises the training manifests as
   regression surface.
2. The v0.4.0 checkpoint (`adam_baseline_checkpoint.safetensors`,
   24.2 M parameters) is the last transformer reference — a future
   small-LM fallback may compare against it.

## Files

| File | Role |
|---|---|
| `adam_baseline_checkpoint.safetensors` | v0.4.0 checkpoint (24.2 M params) |
| `baseline_training_manifest.json` | Configuration for `scripts/run_train_baseline.sh` |
| `baseline_training_assembly_report.json` | Snapshot of which packs fed training |
| `baseline_training_consistency_report.json` | Invariants verified on assembly |
| `baseline_training_delta_report.json` | Drift vs. prior baseline |
| `generation_showcase_report.json` | Multi-prompt × multi-config generation demo |
| `validation_perplexity_report.json` | Held-out perplexity evaluation |
| `clean_training_corpus_report.json` | Statistics on the cleaned corpus |
| `mini_clean_training_*` | v0.3 experiment-matrix (ablation) |
| `tiny_clean_training_profile_*` | v0.3 experiment-matrix (ablation) |

## Which ones CI validates

`validate_foundation.sh` runs these regression assertions (see the
`baseline_training_contracts` test crate). Do not delete without
updating both:

- `baseline_training_assembly_report.json`
- `baseline_training_consistency_report.json`
- `baseline_training_delta_report.json`
- `clean_training_corpus_report.json`
- `mini_clean_training_report.json`
- `mini_clean_training_miss_audit_report.json`
- `tiny_clean_training_report.json`
- `tiny_clean_training_miss_audit_report.json`
- `tiny_clean_training_miss_audit_delta_report.json`
- `tiny_clean_training_profile_*_report.json`

## Regenerating

- Training from scratch: `scripts/run_train_baseline.sh` (~8 h on M2)
- Reports only: `scripts/run_*_report.sh` (fast)
- Evaluation: `scripts/run_eval_perplexity.sh`, `scripts/run_generation_showcase.sh`
