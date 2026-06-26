#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

tmp_acceptance_report="$(mktemp)"
trap 'rm -f "$tmp_acceptance_report"' EXIT

# **2026-06-02** — kernel-only fallback for clean CI checkouts.
# Commit `17e7fce1` («chore: remove regenerable corpus + phoneme
# bank source files (-11 GB)») pulled the source corpus manifest
# (`adam_training_corpus_manifest.json`) out of git so the repo
# fits under GitHub's storage budget. The corpus-pipeline checks
# below depend on it; on a clean checkout they would fail at the
# first `cargo run --bin assemble_unified_corpus` call.
#
# Strategy: if the manifest is missing, run the kernel-side gates
# (cargo fmt + validate_world_core + metrics-currency) and exit
# success. Dev machines that hold the full corpus on disk still
# get the deep validation. DUE_DILIGENCE.md § 6 + § 8 disclose the
# gap explicitly.
if [[ ! -f data/curated/adam_training_corpus_manifest.json ]]; then
    echo "[validate_foundation] SKIP corpus pipeline: data/curated/adam_training_corpus_manifest.json missing"
    echo "[validate_foundation] running kernel-only validation"
    cargo fmt --all --check
    cargo run --release -p adam-reasoning --bin validate_world_core
    cargo run --release -p adam-algebra --bin validate_procedures
    bash scripts/check_metrics_currency.sh
    echo "kernel validation passed (corpus pipeline skipped — manifest absent on this checkout)"
    exit 0
fi

# Regenerate derived corpus artifacts if missing. The unified pack (72 MB),
# pretokenized pack (227 MB), and encoded ids pack (104 MB) exceed GitHub's
# size limits and are `.gitignore`d — CI (and a clean checkout) must rebuild
# them from the committed source packs + BPE artifacts before validation.
# The BPE vocab/merges are committed, so no `run_train_bpe.sh` is needed here.
if [[ ! -f data/curated/adam_training_corpus_pack.json ]]; then
    echo "regenerating adam_training_corpus_pack.json..."
    bash scripts/run_unified_corpus_assembly.sh
fi
if [[ ! -f data/curated/adam_pretokenized_corpus_pack.json ]]; then
    echo "regenerating adam_pretokenized_corpus_pack.json..."
    bash scripts/run_pretokenize_corpus.sh
fi
if [[ ! -f data/curated/adam_training_ids_pack.json ]]; then
    echo "regenerating adam_training_ids_pack.json + validation ids..."
    bash scripts/run_encode_corpus.sh
fi

jq empty data/curated/corpus_manifest.json
jq empty data/curated/source_acceptance_report.json
jq empty data/curated/source_acceptance_summary_report.json
jq empty data/curated/source_acceptance_delta_report.json
jq empty data/curated/clean_training_corpus_manifest.json
jq empty data/curated/clean_training_corpus_pack.json
jq empty data/curated/clean_general_core_pack.json
jq empty data/curated/clean_general_extension_pack.json
jq empty data/curated/clean_reference_extension_pack.json
jq empty data/curated/clean_education_extension_pack.json
jq empty data/curated/tokenizer_dry_run_pack.json
jq empty data/curated/tiny_clean_training_manifest.json
jq empty data/curated/tiny_clean_training_selection_manifest.json
jq empty data/curated/tiny_clean_training_profile_suite_manifest.json
jq empty data/curated/tiny_clean_training_profile_baseline_manifest.json
jq empty data/curated/tiny_clean_training_profile_strategy_manifest.json
jq empty data/curated/tiny_clean_training_profile_promotion_manifest.json
jq empty data/curated/tiny_clean_training_profile_experiment_matrix_manifest.json
jq empty data/curated/tiny_clean_training_profile_experiment_matrix_policy_manifest.json
jq empty data/curated/tiny_clean_general_pack.json
jq empty data/curated/tiny_clean_reference_pack.json
jq empty data/curated/tiny_clean_education_pack.json
jq empty data/curated/tiny_clean_training_pack.json
jq empty data/curated/mini_clean_training_manifest.json
jq empty data/curated/mini_clean_training_pack.json
jq empty data/curated/synthetic_sentences_pack.json
jq empty data/curated/kazakh_proverbs_pack.json
jq empty data/curated/tatoeba_kazakh_pack.json
jq empty data/curated/wikipedia_kz_pack.json
jq empty data/curated/common_voice_kk_pack.json
jq empty data/curated/adam_training_corpus_manifest.json
jq empty data/curated/adam_training_corpus_pack.json
jq empty data/curated/adam_pretokenized_corpus_pack.json
jq empty data/curated/adam_training_ids_pack.json
jq empty data/curated/adam_validation_ids_pack.json
# validation_perplexity_report.json + generation_showcase_report.json regenerate
# whenever the model is retrained; they are dropped between corpus-only releases.
# Phase 7.1 (v0.1.5) will retrain on the enlarged Wikipedia-augmented corpus.
jq empty data/raw/source_registry.json
jq empty data/raw/source_scoring_rules.json
jq empty data/eval/benchmark_manifest.json
jq empty data/eval/benchmark_report.json
jq empty data/eval/benchmark_delta_report.json
jq empty data/eval/kazakh_foundation_eval_dataset.json
jq empty data/eval/tokenizer_segmentation_eval_dataset.json
jq empty data/eval/tokenizer_experiment_manifest.json
jq empty data/eval/tokenizer_experiment_report.json
jq empty data/eval/tokenizer_experiment_delta_report.json
jq empty data/tokenizer/segmentation_roots.json
jq empty data/tokenizer/segmentation_rules.json
jq empty data/tokenizer/bpe_vocab.json
jq empty data/tokenizer/bpe_merges.json
jq empty data/training/baseline_training_manifest.json
jq empty data/training/baseline_training_assembly_report.json
jq empty data/training/baseline_training_consistency_report.json
jq empty data/training/baseline_training_delta_report.json
jq empty data/training/clean_training_corpus_report.json
jq empty data/training/tiny_clean_training_profile_suite_report.json
jq empty data/training/tiny_clean_training_profile_comparison_report.json
jq empty data/training/tiny_clean_training_profile_baseline_report.json
jq empty data/training/tiny_clean_training_profile_baseline_delta_report.json
jq empty data/training/tiny_clean_training_profile_strategy_report.json
jq empty data/training/tiny_clean_training_profile_strategy_delta_report.json
jq empty data/training/tiny_clean_training_profile_promotion_report.json
jq empty data/training/tiny_clean_training_profile_promotion_delta_report.json
jq empty data/training/tiny_clean_training_profile_experiment_matrix_report.json
jq empty data/training/tiny_clean_training_profile_experiment_matrix_delta_report.json
jq empty data/training/tiny_clean_training_profile_experiment_matrix_policy_report.json
jq empty data/training/tiny_clean_training_profile_experiment_matrix_policy_delta_report.json
jq empty data/training/tiny_clean_training_report.json
jq empty data/training/tiny_clean_training_miss_audit_report.json
jq empty data/training/tiny_clean_training_miss_audit_delta_report.json
jq empty data/training/mini_clean_training_report.json
jq empty data/training/mini_clean_training_miss_audit_report.json
jq empty data/foundation/foundation_overview_report.json
jq empty data/foundation/foundation_overview_delta_report.json
cargo fmt --all --check
cargo test -p adam-corpus --tests -- --nocapture
cargo test -p adam-tokenizer --tests -- --nocapture
cargo test -p adam-eval --tests -- --nocapture
cargo test -p adam-train --tests -- --nocapture
./scripts/generate_source_acceptance_report.sh "$tmp_acceptance_report"
cmp -s "$tmp_acceptance_report" data/curated/source_acceptance_report.json
./scripts/run_source_acceptance_summary.sh
./scripts/run_source_acceptance_delta.sh
./scripts/run_tokenizer_dry_run.sh
./scripts/run_eval_benchmark_report.sh
./scripts/run_eval_benchmark_delta.sh
./scripts/run_tokenizer_segmentation_eval.sh
./scripts/run_tokenizer_experiment.sh
./scripts/run_tokenizer_experiment_delta.sh
./scripts/run_training_baseline_plan.sh
./scripts/run_training_baseline_assembly.sh
./scripts/run_training_baseline_consistency.sh
./scripts/run_training_baseline_delta.sh
./scripts/run_clean_training_corpus_assembly.sh
./scripts/run_clean_training_corpus_report.sh
./scripts/run_tiny_clean_training_assembly.sh
./scripts/run_mini_clean_training_assembly.sh
./scripts/run_tiny_training_profile_suite.sh
./scripts/run_tiny_training_profile_comparison.sh
./scripts/run_tiny_training_profile_baseline.sh
./scripts/run_tiny_training_profile_baseline_delta.sh
./scripts/run_tiny_training_profile_strategy.sh
./scripts/run_tiny_training_profile_strategy_delta.sh
./scripts/run_tiny_training_profile_experiment_matrix.sh
./scripts/run_tiny_training_profile_experiment_matrix_delta.sh
./scripts/run_tiny_training_profile_experiment_matrix_policy.sh
./scripts/run_tiny_training_profile_experiment_matrix_policy_delta.sh
./scripts/run_tiny_training_profile_promotion.sh
./scripts/run_tiny_training_profile_promotion_delta.sh
./scripts/run_tiny_clean_training.sh
./scripts/run_tiny_training_miss_audit.sh
./scripts/run_tiny_training_miss_audit_delta.sh
./scripts/run_mini_clean_training.sh
./scripts/run_mini_training_miss_audit.sh
./scripts/run_foundation_overview.sh
./scripts/run_foundation_overview_delta.sh

# **v6.0.0-rc4** — world_core integrity gate. Cross-checks every
# `data/world_core/*.jsonl` entry deserialises (predicates must come
# from the closed 11-variant Predicate enum), ids are globally
# unique, and `kk` text passes the Kazakh-only purity audit. Closes
# the v3.9.0 roadmap promise that put this in CI. Any unknown
# predicate (e.g. `produces`, `author`, `birth_year`) or stray
# Latin character outside backticks fails the gate.
cargo run --release -p adam-reasoning --bin validate_world_core

# **v6.8.28** — procedure-data CI gate.  Validates every JSONL
# line under `data/procedures/`: schema, `check_invariants()`
# (id / title / step monotonicity / `source.version_date` non-
# empty), unique-id collisions across files, and reports
# freshness + trilingual coverage as INFO.  Codex 2026-06-25 #4
# Phase 2 — gates curatorial growth from 15 toward the 50-100
# target without the schema rot a hand-validated data set
# accumulates.
cargo run --release -p adam-algebra --bin validate_procedures

# **v4.55.0** — metrics-currency CI gate. Cross-checks numeric
# claims in README.md / data/README.md / data/world_core/README.md
# / docs/performance.md against actual values from intent.rs +
# world_core/*.jsonl + retrieval/facts.json + Cargo.toml. Fails on
# the first detected drift. Each subsequent release either updates
# docs OR fails this gate.
bash scripts/check_metrics_currency.sh

echo "foundation validation passed"
