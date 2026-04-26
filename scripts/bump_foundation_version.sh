#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

target_version="${1:-}"

if [[ -z "$target_version" ]]; then
  echo "usage: bump_foundation_version.sh <x.y.z>" >&2
  exit 1
fi

if [[ ! "$target_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "target version must match x.y.z" >&2
  exit 1
fi

current_version="$(
  awk '
    $0 == "[workspace.package]" { in_section = 1; next }
    /^\[/ && $0 != "[workspace.package]" { in_section = 0 }
    in_section && $1 == "version" {
      gsub(/"/, "", $3)
      print $3
      exit
    }
  ' Cargo.toml
)"

if [[ "$current_version" == "$target_version" ]]; then
  echo "workspace is already at version $target_version"
else
  # Files where the workspace version string is safe to replace as a plain
  # substring (either it appears only as "version = \"x.y.z\"" or it is the
  # only x.y.z triple in a data file). Do NOT add Cargo.lock here: it contains
  # transitive dep versions (num-integer 0.1.46, etc.) whose suffixes collide
  # with short workspace versions like 0.1.4 under a naive substring replace.
  versioned_files=(
    "Cargo.toml"
    "crates/adam-eval/src/lib.rs"
    "crates/adam-tokenizer/src/lib.rs"
    "crates/adam-train/src/lib.rs"
    "data/eval/benchmark_manifest.json"
    "data/eval/kazakh_foundation_eval_dataset.json"
    "data/eval/tokenizer_experiment_manifest.json"
    "data/eval/tokenizer_segmentation_eval_dataset.json"
    "data/tokenizer/segmentation_roots.json"
    "data/tokenizer/segmentation_rules.json"
    "data/training/baseline_training_manifest.json"
  )

  cargo run --quiet -p adam-train --bin bump_foundation_version -- \
    "$current_version" "$target_version" "${versioned_files[@]}"

  # Regenerate Cargo.lock so workspace-member version entries (adam-corpus,
  # adam-eval, etc.) match the new Cargo.toml without disturbing transitive
  # dep versions.
  cargo build --quiet --workspace >/dev/null
fi

bash ./scripts/verify_release_version.sh "$target_version"
bash ./scripts/validate_foundation.sh

echo "foundation version bumped to $target_version"
echo "next:"
echo "  git status --short"
echo "  git add Cargo.toml Cargo.lock crates/adam-eval/src/lib.rs crates/adam-tokenizer/src/lib.rs crates/adam-tokenizer/tests/tokenizer_experiment_contracts.rs crates/adam-train/src/lib.rs data/eval/benchmark_manifest.json data/eval/kazakh_foundation_eval_dataset.json data/eval/tokenizer_experiment_manifest.json data/eval/tokenizer_segmentation_eval_dataset.json data/tokenizer/segmentation_roots.json data/tokenizer/segmentation_rules.json data/training/baseline_training_manifest.json docs/tokenizer_policy.md .github/workflows/release.yml scripts/bump_foundation_version.sh scripts/cut_release.sh scripts/verify_release_version.sh scripts/README.md README.md"
echo "  git commit -m \"release: prepare v${target_version}\""
echo "  git push"
echo "  git tag -a v${target_version} -m \"Release v${target_version}\""
echo "  git push origin v${target_version}"
