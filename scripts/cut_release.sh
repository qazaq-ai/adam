#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

target_version="${1:-}"

if [[ -z "$target_version" ]]; then
  echo "usage: cut_release.sh <x.y.z>" >&2
  exit 1
fi

if [[ ! "$target_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "release version must match x.y.z" >&2
  exit 1
fi

if [[ -n "$(git status --short)" ]]; then
  echo "cut_release.sh requires a clean worktree" >&2
  exit 1
fi

bash ./scripts/bump_foundation_version.sh "$target_version"

git add \
  Cargo.toml \
  Cargo.lock \
  crates/adam-eval/src/lib.rs \
  crates/adam-tokenizer/src/lib.rs \
  crates/adam-tokenizer/tests/tokenizer_experiment_contracts.rs \
  crates/adam-train/src/lib.rs \
  data/eval/benchmark_manifest.json \
  data/eval/kazakh_foundation_eval_dataset.json \
  data/eval/tokenizer_experiment_manifest.json \
  data/eval/tokenizer_segmentation_eval_dataset.json \
  data/tokenizer/segmentation_roots.json \
  data/tokenizer/segmentation_rules.json \
  data/training/baseline_training_manifest.json \
  docs/tokenizer_policy.md \
  .github/workflows/release.yml \
  scripts/bump_foundation_version.sh \
  scripts/cut_release.sh \
  scripts/verify_release_version.sh \
  scripts/README.md \
  README.md

git commit -m "release: prepare v${target_version}"
git push
git tag -a "v${target_version}" -m "Release v${target_version}"
git push origin "v${target_version}"

echo "release v${target_version} pushed"
