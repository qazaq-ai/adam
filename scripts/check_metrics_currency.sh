#!/usr/bin/env bash
# **v4.55.0** — Metrics-currency CI gate.
#
# Cross-checks numeric claims in README.md, data/README.md, and
# docs/foundation_scope.md against the actual values computed from
# the canonical artefacts (intent.rs, world_core/*.jsonl, retrieval/
# facts.json, retrieval/derived_facts.json, Cargo.toml). Fails if any
# claim is stale.
#
# Codex's 2026-05-05 review surfaced ~15 numeric/version drifts that
# had silently accumulated across 15 minor versions; v4.52.5 fixed
# them manually. This gate ensures they don't reaccumulate.
#
# Each subsequent release either updates the docs OR fails this gate.
# Wired into `scripts/validate_foundation.sh` as a final post-test
# step. Run standalone:
#
#   bash scripts/check_metrics_currency.sh
#
# Exits non-zero on the FIRST detected drift (no aggregation). The
# error message tells the maintainer exactly which file:line + what
# claim is stale + what the live value is.

set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

fail() {
    echo "[check_metrics_currency] STALE: $1" >&2
    exit 1
}

ok() {
    echo "[check_metrics_currency] ok: $1"
}

# ── 1. Intent count ────────────────────────────────────────────────
# Live: count top-level `Intent` enum variants in
# `crates/adam-dialog/src/intent.rs`. README badge must match.
intent_count="$(
    awk '
        /^pub enum Intent/ { in_enum = 1; next }
        in_enum && /^}/ { exit }
        in_enum && /^    [A-Z]/ { count++ }
        END { print count }
    ' crates/adam-dialog/src/intent.rs
)"

readme_intent_claim="$(
    grep -oE 'intents-[0-9]+' README.md | head -1 | grep -oE '[0-9]+'
)"

if [[ "$intent_count" != "$readme_intent_claim" ]]; then
    fail "README.md intents badge claims $readme_intent_claim but Intent enum has $intent_count variants"
fi
ok "intent count: $intent_count (README badge matches)"

# ── 2. World-core entry count ──────────────────────────────────────
# Live: total lines across data/world_core/*.jsonl.
# README badge + data/README.md row + foundation_scope.md must match.
entry_count="$(cat data/world_core/*.jsonl | wc -l | tr -d ' ')"

readme_entry_claim="$(
    grep -oE 'world%20core-[0-9]+' README.md | head -1 | grep -oE '[0-9]+$'
)"

if [[ "$entry_count" != "$readme_entry_claim" ]]; then
    fail "README.md world-core badge claims $readme_entry_claim entries but data/world_core/*.jsonl has $entry_count"
fi
ok "world-core entries: $entry_count (README badge matches)"

# ── 3. Total fact count ────────────────────────────────────────────
# Live: facts.json `.facts | length` — the canonical total post-
# extraction (world_core + corpus-extracted patterns). README badge
# cites this number.
fact_count="$(jq '.facts | length' data/retrieval/facts.json)"

# README badge text «N curated / M facts» — extract M.
# Pipeline: pick the "curated%20/%20<digits>%20facts" segment, then
# strip the prefix to surface just the digits. Cannot use one
# `grep -oE '[0-9]+%20facts'` because the leading `%20` would let
# the digit-class match span into the URL-encoded space.
readme_fact_claim="$(
    grep -oE 'curated%20/%20[0-9]+%20facts' README.md | head -1 \
        | sed -E 's/.*curated%20\/%20([0-9]+)%20facts.*/\1/'
)"

if [[ "$fact_count" != "$readme_fact_claim" ]]; then
    fail "README.md world-core badge claims $readme_fact_claim facts but data/retrieval/facts.json has $fact_count"
fi
ok "total facts: $fact_count (README badge matches)"

# World-core .jsonl raw fact count — for sanity vs the README's
# data/README.md row which uses the retrieval/facts.json total too.
worldcore_fact_count="$(
    cat data/world_core/*.jsonl | jq -s 'map(.facts | length) | add'
)"
ok "world-core .jsonl raw facts: $worldcore_fact_count (informational; not gated)"

# ── 4. Derived facts ───────────────────────────────────────────────
derived_count="$(jq '.derived | length' data/retrieval/derived_facts.json)"
ok "derived facts: $derived_count (informational; not gated)"

# ── 5. Workspace version vs Cargo.toml ─────────────────────────────
# Live: [workspace.package] version. Used as the canonical workspace
# version; README badge + performance.md header must match.
workspace_version="$(
    awk '
        /^\[workspace.package\]/ { in_section = 1; next }
        /^\[/ && !/^\[workspace.package\]/ { in_section = 0 }
        in_section && /^version =/ {
            gsub(/[" ]/, "")
            split($0, parts, "=")
            print parts[2]
            exit
        }
    ' Cargo.toml
)"

# **v6.0.0-rc1** — accept shields.io double-dash encoding for the
# SemVer 2.0 pre-release suffix `-rcN`. shields.io URL-encodes a
# single dash as `--` inside badge text, so `version-6.0.0-rc1` is
# served as `version-6.0.0--rc1` in the badge URL. We extract the
# full `MAJOR.MINOR.PATCH[--rcN]` group and collapse `--` back to
# `-` before comparing to the workspace version.
readme_version_claim="$(
    grep -oE 'version-[0-9]+\.[0-9]+\.[0-9]+(--rc[0-9]+)?' README.md \
        | head -1 \
        | sed 's/version-//' \
        | sed 's/--rc/-rc/'
)"

if [[ "$workspace_version" != "$readme_version_claim" ]]; then
    fail "README.md version badge claims $readme_version_claim but workspace Cargo.toml is $workspace_version"
fi
ok "workspace version: $workspace_version (README badge matches)"

perf_version_claim="$(
    head -1 docs/performance.md \
        | grep -oE 'v[0-9]+\.[0-9]+\.[0-9]+(-rc[0-9]+)?' \
        | head -1 \
        | sed 's/^v//'
)"

if [[ "$workspace_version" != "$perf_version_claim" ]]; then
    fail "docs/performance.md header claims v$perf_version_claim but workspace Cargo.toml is $workspace_version"
fi
ok "performance.md header version: $perf_version_claim (matches workspace)"

# ── 6. data/README.md world-core row ───────────────────────────────
# Verify the world_core row in data/README.md cites the live counts.
data_readme_row="$(grep '| `world_core/`' data/README.md || true)"
if [[ -n "$data_readme_row" ]]; then
    if ! grep -qE "$entry_count entries" data/README.md; then
        fail "data/README.md world_core row missing '$entry_count entries' (live count)"
    fi
    if ! grep -qE "$fact_count facts" data/README.md; then
        fail "data/README.md world_core row missing '$fact_count facts' (live count)"
    fi
    ok "data/README.md world_core row: $entry_count entries / $fact_count facts (matches live)"
fi

# ── 7. data/world_core/README.md "Live totals" line ────────────────
if ! grep -qE "$entry_count entries / $fact_count facts" data/world_core/README.md; then
    fail "data/world_core/README.md missing 'Live totals: $entry_count entries / $fact_count facts'"
fi
ok "world_core/README.md Live totals: $entry_count / $fact_count (matches live)"

echo "[check_metrics_currency] all metrics current at workspace version $workspace_version"
