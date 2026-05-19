# Migration playbook — v5.x → v6.0.0-rcN

**Status:** authoritative for the **v6.0.0-rc1 … rc4 preview** line.
Updated 2026-05-19 (rc4 ships world_core expansion + validator CI
gate + factual_eval_100 baseline; rc1 migration surface unchanged).
The full GA migration (Settings::NeuralSettings struct + automatic
L5.5 integration into the dialog loop + pre-built `.bin` artifacts)
remains a v6.0.5+ task — see §7.

**Audience:** every adam operator running v5.x who wants to evaluate
the v6.0 architecture preview without losing the v5.x guarantees.

**Companion document:** [`architecture_neural_v6.md`](architecture_neural_v6.md)
(architecture contract); [`roadmap_v6_v7.md`](roadmap_v6_v7.md)
(release schedule).

This document is the **rollback-safe** procedure for upgrading from
the deterministic-only v5.x line to the neural-preview-enabled
v6.0.0-rc1 line. Every step is reversible. No production data is
rewritten in place.

---

## 1. Promise of the migration

v6.0.0-rc1 is **strict-superset compatible** with v5.32.x. A v5.x
deployment upgraded to v6.0.0-rc1 will, by default, **behave
identically to v5.32.x**. The neural layer is opt-in through a
**Cargo feature flag** (`--features neural`) and a **CLI argument**
(`--neural-model <checkpoint-dir>`) — without both, the neural code
path is not even linked into the binary.

Specifically, the migration preserves:

1. **All v5.x APIs.** `adam_chat::answer` returns the same `Answer`
   shape with the same fields.
2. **All v5.x behaviour on the deterministic path.** Same templates,
   same intents, same reasoning rules, same world_core, same
   retrieval index.
3. **All v5.x data files on disk.** `data/` layout unchanged.
4. **All v5.x tests.** `cargo test --workspace` passes the same set
   of release-blocker tests on v6.0.0-rc1 as on v5.32.

What v6.0.0-rc1 **adds**:

1. Three new optional Cargo crates: `adam-agg-tokenizer`,
   `adam-agg-synth`, `adam-agg-model`. None are compiled into
   downstream binaries unless explicitly opted into.
2. New optional binary `poc_kazakh_train` (in `adam-agg-model`).
   Used to *produce* L5.5 checkpoints from the FST-generated +
   real-corpus training pairs. Not invoked at runtime; an operator
   runs it once to train a model, then ships the resulting
   checkpoint directory.
3. New optional binary `verifier_demo` (in `adam-agg-model`).
   CLI for inspecting the L6 four-gate behaviour on candidate
   surfaces.
4. New optional feature `neural` on `adam-dialog`. When enabled,
   `adam_chat` accepts `--neural-model <checkpoint-dir>` and
   exposes the `/neural <prompt>` slash command — the **runnable
   preview surface**.
5. New `data/checkpoints/` directory for neural checkpoints. Empty
   by default; populated by running `poc_kazakh_train`. Gitignored
   so checkpoints don't pollute the source tree.

What v6.0.0-rc1 **does not yet add** (deferred to v6.0.5+):

1. A `Settings::NeuralSettings` struct for programmatic config.
   v6.0.0-rc1 uses CLI flags + Cargo features only.
2. Automatic L5.5 invocation inside the dialog loop. The trained
   PoC model is a morpheme-level composer, not a sentence-level
   chatbot; integrating it into the multi-turn dialog graph
   requires (a) a much larger checkpoint trained on conversational
   pairs, (b) routing logic for which intent shapes consult L5.5.
   Both are research-arc work for v6.0.5+.
3. A pre-built `.bin` release artifact. v6.0.0-rc1 expects
   operators to train their own checkpoint from `poc_kazakh_train`
   on M2-class hardware (≈ 95 minutes for the full corpus,
   ≈ 10 minutes for a smoke checkpoint).

---

## 2. Pre-migration checklist

Before running the upgrade, verify the following on the production
host:

- [ ] `cargo test --workspace --release` passes on the v5.32.x tree
      (i.e. you have a known-green baseline to compare against).
- [ ] You have a working git checkout (not a binary-only install).
      v6.0 ships as source; you build from `cargo build --release`.
- [ ] Disk space: at least **2 GB free** in the deployment root.
      The neural model binary is on the order of 30–60 MB; the
      build artefacts and dependency download are the larger draw.
- [ ] You have **`Settings::default()` only** in your deployment.
      If you have a custom `Settings` constructor, audit it for
      compatibility with the new `NeuralSettings` field — see §4.
- [ ] You have a recent backup of `data/` (the live corpus and
      knowledge graph). Standard pre-upgrade hygiene; v6.0 does not
      write to `data/` during normal operation but a defensive
      backup is good practice.

If any item above is unchecked, address it before continuing.

---

## 3. Upgrade procedure — deterministic-only (zero risk)

This is the minimum-risk path. The deployment behaves identically
after the upgrade. Use this path if you want to land the v6.0 source
tree in production but defer the neural-layer rollout.

```bash
# In the deployment root:
git fetch origin
git checkout v6.0.0          # or main once v6.0.0 is merged
cargo build --release --workspace
# Run the deterministic-path regression suite:
cargo test --workspace --release
# Restart your adam_chat process:
systemctl restart adam-chat  # or your equivalent
```

Expected runtime impact: **none.** Memory: **same as v5.32**
(neural model not loaded). Latency: **same as v5.32**.

### 3.1 Verification

Compare the post-upgrade response on three test inputs to the
v5.32 baseline:

| Input                           | v5.32 response (must match exactly on v6.0 deterministic path)               |
|---------------------------------|------------------------------------------------------------------------------|
| `сәлем`                         | `Сәлеметсіз бе!` (or your localised greeting; depends on Settings) |
| `мен Алматыда тұрамын`          | `Жақсы, Алматы — әдемі қала.` (or localised confirmation)        |
| `Қазақстан туралы айтшы`        | first-fact retrieval from world_core / geography_kz.jsonl                    |

Any divergence on the deterministic path is a **release blocker**.
Report via repository Issues with the full trace.

---

## 4. Upgrade procedure — neural-enabled (opt-in)

After §3 is green, you may enable the L5.5 neural composer preview.
This is the path that exercises the v6.0 feature set in **preview
mode** — the model loads, generation runs, the L6 verifier gates
the output, and the audit trail is visible. The deterministic
kernel keeps serving normal dialog turns; the neural layer is
accessed only through the explicit `/neural` slash command.

### 4.1 Train (or obtain) a checkpoint

v6.0.0-rc1 expects you to train your own checkpoint. The training
binary lives at `crates/adam-agg-model/src/bin/poc_kazakh_train.rs`.

```bash
# Smoke checkpoint (~10 min on M2 CPU, 1 epoch / batch 8):
POC_EPOCHS=1 POC_BATCH=8 POC_CHECKPOINT_DIR=data/checkpoints/smoke \
  cargo run --release -p adam-agg-model --bin poc_kazakh_train

# Full checkpoint (~95 min on M2 CPU, default 3 epochs):
POC_REAL_PACK=data/curated/real_corpus_pairs.json POC_ALPHA=0.5 \
  cargo run --release -p adam-agg-model --bin poc_kazakh_train
# → writes data/checkpoints/poc_kazakh/v6_<UTC-timestamp>/
```

The checkpoint directory contains `config.json`, `labels.json`,
`training.json`, and `model.mpk` (~3-30 MB depending on hparams).
All four files are required for load.

### 4.2 Enable in adam_chat

The preview is gated by **both** a Cargo feature AND a CLI flag:

```bash
# Build with neural feature (one-time per code change):
cargo build --release -p adam-dialog --bin adam_chat --features neural

# Launch with a checkpoint pointer:
./target/release/adam_chat \
  --neural-model data/checkpoints/poc_kazakh/v6_<timestamp>/
```

The verifier is **strict by default** — the v6.0.0-rc1 contract
inherits the four-gate behaviour from `adam_agg_model::Verifier::new_strict`
(NonKazakhScript → UnkSurface → FstRoundTripFailed → Ungrounded).
There is **no flag to make it permissive**; if you need permissive
mode for offline analysis, use the `verifier_demo` binary
separately.

### 4.3 Smoke-test before serving

```bash
# Boot the REPL with the checkpoint:
./target/release/adam_chat \
  --neural-model data/checkpoints/poc_kazakh/v6_<timestamp>/

# Inside the REPL:
> /neural бала
[neural] BOS R:бала
[neural surface] бала
[neural verdict] PASS surface="бала" root="бала" grounded=true
```

If the boot banner shows `adam-chat: neural preview on — checkpoint
…` and `/neural` produces an audit trail, the preview is live.

### 4.4 Canary roll-out

We recommend a canary deployment with the neural layer enabled on
1–5 % of traffic for at least 48 hours, comparing:

- **Latency.** p50 turn time must remain ≤ 25 ms (deterministic
  path; missing model file path takes this) and ≤ 150 ms (neural
  path).
- **Memory.** RSS must remain ≤ 320 MB with the model loaded.
- **Verifier-block rate.** A spike in `verifier_block` events
  signals either a regression in the neural model or a coverage
  gap in `facts.json`. Both warrant rollback and triage.
- **User-facing quality.** Compare canary vs control on the
  cognitive-eval + repl-replay datasets.

If any of the four signals regress beyond their thresholds, follow
§5.

---

## 5. Rollback procedure

Rollback is **symmetric** with the upgrade. There is no data
migration to undo.

### 5.1 Disable the neural layer (preserve v6.0 source)

The fastest rollback. The v6.0 source stays on disk; only the
neural layer goes dormant.

```bash
# Edit settings to set neural.enabled = false, OR
# remove the model file:
mv data/models/adam_agg_v6_0_0.bin data/models/adam_agg_v6_0_0.bin.disabled
systemctl restart adam-chat
```

The deployment now behaves identically to v5.32 again. Time to
recovery: ~30 seconds.

### 5.2 Full downgrade to v5.32.x

If the v6.0 source itself is implicated in a regression (rare,
since the deterministic path is unchanged):

```bash
git checkout v5.32.0
cargo build --release --workspace
systemctl restart adam-chat
```

Time to recovery: ~3 minutes (build dominated). No data changes
required.

### 5.3 What can go wrong (and how to detect it)

| Symptom                                                | Likely cause                                   | Rollback path |
|--------------------------------------------------------|------------------------------------------------|---------------|
| Latency p50 spike on a subset of turns                 | Neural model loaded, time budget tight         | §5.1 disable  |
| Latency p50 spike on **all** turns                     | v6.0 deterministic path regression (should not happen — release-blocker) | §5.2 full     |
| Memory blow-up                                         | Neural model not freed under load              | §5.1 disable  |
| Verifier-block rate > 5 % on production traffic        | Coverage gap in `facts.json` OR model emitting OOD outputs | §5.1 disable + triage |
| `adam_chat` won't start                                 | Missing model file path with `enabled=true`   | edit settings (§5.1 inverse) |
| `cargo test` fails on v6.0                              | Release artefact tampered with                 | §5.2 full     |

---

## 6. Long-term migration

Once v6.0 is stable in production for at least one minor-version
cycle (i.e. v6.1.x lands), the v5.32 codebase enters maintenance-
only status. Critical security fixes will be backported for
**two minor releases** (so v5.32.x is supported through v6.2.x);
beyond that, upgrade is required.

The model artefact has a separate support window — see
[`architecture_neural_v6.md`](architecture_neural_v6.md) §7.2.

---

## 7. Frequently asked

**Q. Can I run v6.0 in air-gapped / offline mode?**
A. Yes. The neural model binary is a single file on disk; no
network call is made at inference time. Build artifacts are also
all-local (`cargo build --frozen --offline` once dependencies are
cached).

**Q. Can I use a custom-trained neural model?**
A. Yes, provided it matches the `TinyAgtConfig` schema. See
`crates/adam-agg-model/src/lib.rs` for the layout. Custom models
must be paired with a custom model card under `data/models/`.

**Q. Will v6.0 affect the v5.x scaling-bench numbers?**
A. With the neural layer disabled, no. With the neural layer
enabled, latency increases by ~2 ms p50 and ~3 ms p99 per neural-
enabled turn (see [`bench/neural_inference_2026_05_16.md`](bench/neural_inference_2026_05_16.md)).

**Q. What if I can't allocate the extra 120 MB of RSS for the
neural model?**
A. Disable the neural layer (`neural.enabled = false`). v6.0 in
deterministic-only mode has the same RSS profile as v5.32.

**Q. How do I roll out the neural layer to a fraction of traffic
instead of all of it?**
A. In v6.0.0-rc1 the neural surface is opt-in **per turn** via the
`/neural <prompt>` slash command — there is no automatic
invocation, so traffic split is implicit (only turns that begin
with `/neural` consult the model). Programmatic
`Settings::neural.enabled` per-request gating + `TurnTrace::
neural_calls` audit are deferred to v6.0.5 (see §7 "Deferred to
v6.0.5+").

---

## 7. Deferred to v6.0.5+ (post-rc1 roadmap)

The following items were promised by earlier drafts of this
document but are intentionally **not** in v6.0.0-rc1. They are
tracked in `docs/roadmap_v6_v7.md` and unblock once the alpha
feedback from rc1 is incorporated.

| Item | Reason deferred |
|---|---|
| `Settings::NeuralSettings` struct (programmatic config) | CLI-flag + feature-flag surface is enough for alpha. Settings struct ossifies an API; we want alpha learnings first. |
| Automatic L5.5 invocation in the dialog loop | Requires a much larger conversational-pair training set than v6.0.0-rc1 ships. The PoC checkpoint composes morphemes, not turns. |
| Pre-built `.bin` release artifact | Distributing trained weights raises licensing questions for derivative data. Postponed until counsel review. |
| `TurnTrace::neural_calls` audit field | Auto-routing into the loop is the prerequisite; once L5.5 fires on dialog turns we add the audit shape that the architecture spec describes. |
| `--neural-time-budget-ms` / `--neural-verifier-strict` flags | rc1 hard-codes the budget at the model's natural inference time (~1.7 ms for 6 tokens) and verifier-strict at always-on. Tunable per-request comes with the Settings struct. |

The architecture spec
([`architecture_neural_v6.md`](architecture_neural_v6.md)) describes
the **target** API surface; this migration doc describes what
**actually exists in v6.0.0-rc1**. The two converge in v6.0.5.

---

## 8. Pre-registered alpha-partner deployment

> **rc1 status (2026-05-18).** The alpha-partner kit lives at
> `docs/alpha_onboarding_kit/` once published. The kit includes:
> deploy guide for M2-class hardware (no GPU); 2-week validation
> protocol with 300-turn target; success/abort criteria; feedback
> spreadsheet template; privacy/data-handling agreement; and a
> risk disclaimer noting v6.0.0-rc1 is research-grade.


The v6.0 acceptance criterion #6 in
[`architecture_neural_v6.md`](architecture_neural_v6.md) §9 requires
the migration to be validated against an **external alpha
deployment** for at least one week. The alpha-partner contract
template lives at `docs/outreach/alpha_partner_contract.md`
(forthcoming). Any organisation interested in being the alpha
partner should email `baimurza.daulet@gmail.com` with their
deployment topology.
