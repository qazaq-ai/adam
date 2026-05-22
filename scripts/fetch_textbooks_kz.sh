#!/usr/bin/env bash
# fetch_textbooks_kz.sh — manifest-driven Kazakh school textbook fetcher.
#
# Reads scripts/textbooks_kz_manifest.json. For each entry:
#   - If `url` is empty: skip with TODO warning. Edit the manifest to
#     add the official source URL (opiq.kz / okulyk.kz / publisher page).
#   - If `local_seed` is present and the seed file exists: copy it into
#     the target tree without re-downloading. Used to migrate the
#     pre-existing data/external/*.pdf files into the per-grade layout.
#   - Otherwise: curl the URL with resume + retry, compute sha256,
#     verify size, write back into manifest in-place.
#
# Output tree:   data/external/textbooks_kz/grade_NN/<filename>.pdf
# Manifest:      scripts/textbooks_kz_manifest.json (updated on success)
#
# Legal note: only fill the `url` field with sources you are LEGALLY
# allowed to redistribute under their terms. The PDFs are gitignored,
# so they never leave the local checkout — but you still need the right
# to fetch them in the first place.
#
# Dependencies: jq, curl, shasum (built into macOS), stat.

set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
manifest="$repo_root/scripts/textbooks_kz_manifest.json"
target_root="$repo_root/data/external/textbooks_kz"

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
n_seeded=0
n_already=0
n_failed=0

# Portable sha256 + size helpers (macOS Darwin has BSD stat).
sha256() { shasum -a 256 "$1" | awk '{print $1}'; }
file_size() { stat -f%z "$1" 2>/dev/null || stat -c%s "$1"; }

for (( i = 0; i < n_total; i++ )); do
  entry=$(jq ".sources[$i]" "$manifest")
  grade=$(jq -r '.grade'   <<<"$entry")
  subject=$(jq -r '.subject'  <<<"$entry")
  title=$(jq -r '.title'    <<<"$entry")
  url=$(jq -r '.url // ""'  <<<"$entry")
  filename=$(jq -r '.filename' <<<"$entry")
  local_seed=$(jq -r '.local_seed // ""' <<<"$entry")
  recorded_sha=$(jq -r '.sha256 // ""'   <<<"$entry")
  recorded_size=$(jq -r '.size_bytes // 0' <<<"$entry")

  grade_dir=$(printf "%s/grade_%02d" "$target_root" "$grade")
  mkdir -p "$grade_dir"
  out_path="$grade_dir/$filename"
  short_path="${out_path#"$repo_root/"}"

  # Case 1: file already present locally and matches recorded sha.
  if [[ -f "$out_path" && -n "$recorded_sha" ]]; then
    actual_sha=$(sha256 "$out_path")
    if [[ "$actual_sha" == "$recorded_sha" ]]; then
      n_already=$((n_already + 1))
      continue
    fi
  fi

  # Case 2: local seed present (migrate existing data/external/*.pdf).
  if [[ -n "$local_seed" && -f "$repo_root/$local_seed" ]]; then
    cp -f "$repo_root/$local_seed" "$out_path"
    sha=$(sha256 "$out_path")
    sz=$(file_size "$out_path")
    tmp=$(mktemp)
    jq ".sources[$i].sha256 = \"$sha\" | .sources[$i].size_bytes = $sz" "$manifest" >"$tmp"
    mv "$tmp" "$manifest"
    echo "  [seed]    $short_path  (from $local_seed, $sz bytes)"
    n_seeded=$((n_seeded + 1))
    continue
  fi

  # Case 3: no URL set — TODO, skip with warning.
  if [[ -z "$url" ]]; then
    n_skip_no_url=$((n_skip_no_url + 1))
    continue
  fi

  # Case 4: download.
  echo "  [fetch]   grade $grade · $subject · $title"
  echo "            $url"
  if ! curl --fail --location \
        --retry 3 --retry-delay 2 \
        --connect-timeout 30 --max-time 600 \
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
printf "  seeded from local:    %d\n" "$n_seeded"
printf "  already present:      %d\n" "$n_already"
printf "  TODO (no url yet):    %d\n" "$n_skip_no_url"
printf "  failed:               %d\n" "$n_failed"

if (( n_failed > 0 )); then
  exit 1
fi
