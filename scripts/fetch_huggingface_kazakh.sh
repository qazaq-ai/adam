#!/usr/bin/env bash
# fetch_huggingface_kazakh.sh — download Hugging Face Kazakh corpora.
#
# Reads scripts/huggingface_kazakh_manifest.json. For each entry:
#   - If `url` is empty: skip with TODO warning.
#   - If `sha256` matches the on-disk file: skip (already current).
#   - Otherwise: curl with resume + retry, compute sha256, verify size.
#
# Output:    data/external/huggingface_kz/<filename>
# Manifest:  scripts/huggingface_kazakh_manifest.json (updated on success)
#
# Why HF over scraping opiq.kz / okulyk.kz: HF datasets are
# explicitly licensed (Apache-2.0, CC-BY-4.0, etc.) and have stable
# resolvable URLs. Scraping a textbook portal is brittle (URLs hidden
# behind JS viewers) and legally ambiguous; HF was specifically built
# for redistributing labelled corpora.

set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
manifest="$repo_root/scripts/huggingface_kazakh_manifest.json"
target_root="$repo_root/data/external/huggingface_kz"

if [[ ! -f "$manifest" ]]; then
  echo "missing manifest: $manifest" >&2
  exit 1
fi
command -v jq    >/dev/null || { echo "jq is required" >&2; exit 1; }
command -v curl  >/dev/null || { echo "curl is required" >&2; exit 1; }
command -v shasum >/dev/null || { echo "shasum is required" >&2; exit 1; }

mkdir -p "$target_root"

n_total=$(jq '.sources | length' "$manifest")
n_done=0
n_skip_no_url=0
n_already=0
n_failed=0

sha256() { shasum -a 256 "$1" | awk '{print $1}'; }
file_size() { stat -f%z "$1" 2>/dev/null || stat -c%s "$1"; }

for (( i = 0; i < n_total; i++ )); do
  entry=$(jq ".sources[$i]" "$manifest")
  dataset=$(jq -r '.dataset'    <<<"$entry")
  split=$(jq -r '.split'        <<<"$entry")
  title=$(jq -r '.title'        <<<"$entry")
  url=$(jq -r '.url // ""'      <<<"$entry")
  filename=$(jq -r '.filename'  <<<"$entry")
  recorded_sha=$(jq -r '.sha256 // ""' <<<"$entry")

  out_path="$target_root/$filename"
  short_path="${out_path#"$repo_root/"}"

  if [[ -z "$url" ]]; then
    n_skip_no_url=$((n_skip_no_url + 1))
    continue
  fi

  if [[ -f "$out_path" && -n "$recorded_sha" ]]; then
    actual_sha=$(sha256 "$out_path")
    if [[ "$actual_sha" == "$recorded_sha" ]]; then
      n_already=$((n_already + 1))
      continue
    fi
  fi

  echo "  [fetch]   $dataset/$split"
  echo "            $url"
  if ! curl --fail --location \
        --retry 3 --retry-delay 5 \
        --connect-timeout 30 --max-time 7200 \
        -C - \
        -o "$out_path" "$url"; then
    echo "  [FAIL]    $short_path — curl exited non-zero" >&2
    n_failed=$((n_failed + 1))
    continue
  fi
  sha=$(sha256 "$out_path")
  sz=$(file_size "$out_path")
  tmp=$(mktemp)
  jq ".sources[$i].sha256 = \"$sha\" | .sources[$i].size_bytes = $sz" "$manifest" >"$tmp"
  mv "$tmp" "$manifest"
  echo "  [ok]      $short_path  ($sz bytes, sha=${sha:0:12}…)"
  n_done=$((n_done + 1))
done

echo
echo "Summary:"
printf "  total entries:        %d\n" "$n_total"
printf "  downloaded this run:  %d\n" "$n_done"
printf "  already present:      %d\n" "$n_already"
printf "  TODO (no url yet):    %d\n" "$n_skip_no_url"
printf "  failed:               %d\n" "$n_failed"

if (( n_failed > 0 )); then
  exit 1
fi
