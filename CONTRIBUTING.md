# Contributing to adam

Short version of the rules every change in this repo must follow. The
long version lives in `docs/roadmap.md` (release-by-release history),
the per-crate READMEs, and the `feedback_*` / `project_*` discussions
linked from individual decisions.

## Stack policies (contract-tested)

- **Rust-only.** No Python / Node / TypeScript / other-language source
  files. Enforced by `crates/adam-eval/tests/rust_only_contracts.rs`.
- **Graph-first.** No external graph DB, no Cypher / Gremlin / SPARQL —
  the graph layer is Rust-native. Enforced by
  `crates/adam-eval/tests/graph_first_contracts.rs`.
- **POSIX shell scripts** in `scripts/` are thin wrappers around
  `cargo run` / `cargo test` only.
- **Kazakh-only surface.** User-facing reply text is Cyrillic Kazakh.
  English / Russian only appears as Latin technical tokens explicitly
  whitelisted in the v4.3.4 quality gate (`adam`, `Nano`, `Language`,
  `Model`, `NLM`).

## Pre-push checklist

1. `cargo fmt --all --check` — CI rejects even one-line rustfmt diffs.
2. `cargo test --workspace` — every test must pass; the cognitive eval
   harness is part of this and treats canonical-scenario failures as
   hard CI red.
3. `bash scripts/verify_release_version.sh <x.y.z>` — workspace
   `Cargo.toml` and the seven foundation manifests must agree on
   version. Bump them together or not at all.
4. README numeric claims (test count, fact / derivation counts, corpus
   size, version refs) must be current. The pre-push README audit is
   a hard gate.

## Versioning

Strict `x.y.z` only — no `-alpha` / `-beta` / `-rc` suffixes; the
verifier rejects them.

- **Patch** `x.y.z` — small / incremental: bug fixes, template tweaks,
  detector reorderings, hygiene, single-file refactors, small
  Lexicon additions. Patch numbers increment by 1, not by 5.
- **Minor** `x.y.0` — significant: new corpus source, new intent
  family, new recogniser layer, new cognitive capability, completed
  multi-release milestone, new `Action` variant.
- **Major** `x.0.0` — paradigm shift.

Sequential numbering is mandatory — never skip a number to "reserve"
it for a future release.

## Cognitive eval policy

**Every dialog defect ships with at least one new cognitive
scenario.**

This is the load-bearing policy of the test layer. A defect surfaced
by an external review (Codex 2026-04-27 caught two), a real REPL
trace, or a user report is not "fixed" until the scenario that
reproduces it lives in `data/eval/cognitive_dialog_dataset.json`
under the right category and the harness asserts the post-fix
behaviour. The 54 canonical scenarios that exist today were earned
this way over many releases — they are the floor of regressions, not
a synthetic batch authored cold.

Authoring rules for new scenarios:

- Pick the category that already exists if at all possible
  (`action_routing`, `belief_revision`, `contradiction_handling`,
  `contradiction_recovery`, `direct_answer`, `epistemic_routing`,
  `goal_continuity`, `parse_failure`, `topic_switch`,
  `verification_gating`). Add a new category only when the failure
  mode is genuinely uncovered.
- Always include a `description` that names the release and the bug
  it locks down. Future readers should be able to reconstruct *why*
  the scenario exists from the description alone.
- Prefer trace-signal assertions (`action`, `epistemic_status`,
  `task_status`, `belief_contradictions_count`) when the bug is in
  classification or planning.
- Add `output_contains_lower_any` / `output_not_contains_lower`
  assertions when the bug is in the surface text (e.g. v4.4.5's
  CheckContradiction renderer leak — the trace was right, only the
  reply was wrong).
- Mark a scenario `expected_failing: true` if it documents a known
  gap that no shipped release satisfies yet. The harness reports
  PASSes on aspirational scenarios as "ready to promote" without
  failing CI on their FAILs.

## REPL replay policy (v4.4.6+)

**Every surface-text defect ships with at least one new REPL replay
dialog.**

The cognitive eval checks trace signals; the REPL replay
(`crates/adam-dialog/tests/repl_replay.rs` over
`data/eval/repl_dialogs.jsonl`) checks what the user *actually
sees*. The two complement each other — Codex's 2026-04-27 catch
was invisible to the cognitive eval because the trace was correct,
only the rendered text was wrong; a REPL replay over the same
two-turn sequence would have caught it directly.

Authoring rules: same shape as the cognitive eval, but each turn
records `user` input and either `output_contains_lower_any` /
`output_not_contains_lower` substring assertions or an exact
`expected_output` (the latter is brittle but appropriate for
locked-down surface invariants).

## Commits

- Subject line under 72 chars. The `release: vX.Y.Z — <one-line
  scope>` form is preferred for tagged commits.
- Body explains *why*. The "what" is in the diff.
- Co-authorship on Claude-assisted commits: append
  `Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>`
  (or whatever model line was active).
- Never `--amend` a published commit. Never force-push to `main`.
  Tag re-pointing on a brand-new tag (release CI fix) is acceptable;
  on a tag that's been live for hours, use a fresh patch number
  instead.

## Performance regression policy (v4.4.7+)

Performance regressions are release blockers. Before tagging a
release that touches dialog runtime
(`crates/adam-dialog/src/`), re-run

```
cargo bench -p adam-dialog --bench turn_latency
```

on the same M2-baseline reference machine and compare against the
numbers in [docs/performance.md](docs/performance.md). A p50
regression **> 20 %** on any scenario must either be:

1. justified in the release notes (a new capability landed that
   explains the cost), with `docs/performance.md` updated to
   reflect the new baseline, or
2. rolled back before tagging.

`/usr/bin/time -l ./target/release/adam_chat --once "сәлем"` is
the secondary check for max RSS regressions; the same
> 20 % rule applies.

## Documentation refresh on every release

`README.md`, `CHANGELOG.md`, and `docs/roadmap.md` must be refreshed
**as part of the release commit**, not as a follow-up. Per-crate
READMEs and `lib.rs` / bin docstrings get audited on every release
too — not just the top-level three. Stale numeric claims in any of
these is a release blocker.
