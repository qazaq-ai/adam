#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

expected_version="${1:-}"

if [[ -z "$expected_version" ]]; then
  echo "usage: verify_release_version.sh <x.y.z>" >&2
  exit 1
fi

if [[ ! "$expected_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "release version must match x.y.z" >&2
  exit 1
fi

workspace_version="$(
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

if [[ "$workspace_version" != "$expected_version" ]]; then
  echo "workspace version mismatch: expected $expected_version, got $workspace_version" >&2
  exit 1
fi

manifest_files=(
  "data/eval/benchmark_manifest.json"
  "data/eval/kazakh_foundation_eval_dataset.json"
  "data/eval/tokenizer_experiment_manifest.json"
  "data/eval/tokenizer_segmentation_eval_dataset.json"
  "data/tokenizer/segmentation_roots.json"
  "data/tokenizer/segmentation_rules.json"
  "data/training/baseline_training_manifest.json"
)

for file in "${manifest_files[@]}"; do
  file_version="$(jq -r '.version' "$file")"
  if [[ "$file_version" != "$expected_version" ]]; then
    echo "manifest version mismatch in $file: expected $expected_version, got $file_version" >&2
    exit 1
  fi
done

lock_packages=(
  "adam-kernel"
  "adam-corpus"
  "adam-eval"
  "adam-tokenizer"
  "adam-train"
)

for package in "${lock_packages[@]}"; do
  package_version="$(
    awk -v package="$package" '
      $0 == "[[package]]" { in_package = 0 }
      $0 == "name = \"" package "\"" { in_package = 1; next }
      in_package && $1 == "version" {
        gsub(/"/, "", $3)
        print $3
        exit
      }
    ' Cargo.lock
  )"

  if [[ "$package_version" != "$expected_version" ]]; then
    echo "Cargo.lock version mismatch for $package: expected $expected_version, got $package_version" >&2
    exit 1
  fi
done

echo "release version $expected_version verified"
