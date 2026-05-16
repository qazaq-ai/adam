# Migration playbook — v5.x → v6.0

**Status:** authoritative for the v6.0.0 GA release.
**Audience:** every adam operator running v5.x in production who
needs to plan the upgrade to v6.0.
**Companion document:** [`architecture_neural_v6.md`](architecture_neural_v6.md)
(architecture contract); [`roadmap_v6_v7.md`](roadmap_v6_v7.md)
(release schedule).

This document is the **rollback-safe** procedure for upgrading from
the deterministic-only v5.x line to the neural-enabled v6.0 line.
Every step is reversible. No production data is rewritten in place.

---

## 1. Promise of the migration

v6.0 is **strict-superset compatible** with v5.32.x. A v5.x deployment
upgraded to v6.0 will, by default, **behave identically to v5.32.x**.
The neural layer is opt-in per `Settings::NeuralSettings { enabled,
.. }`.

Specifically, the migration preserves:

1. **All v5.x APIs.** `adam_chat::answer` returns the same `Answer`
   shape with the same fields.
2. **All v5.x behaviour on the deterministic path.** Same templates,
   same intents, same reasoning rules, same world_core, same
   retrieval index.
3. **All v5.x data files on disk.** `data/` layout unchanged.
4. **All v5.x tests.** `cargo test --workspace` passes the same set
   of release-blocker tests on v6.0 as on v5.32.

What v6.0 **adds**:

1. A new optional binary `adam_agg_demo` (no impact unless invoked).
2. A new optional Cargo crate `adam-agg-model` (no impact unless a
   downstream binary depends on it).
3. New optional fields in `Settings::NeuralSettings`. The
   `Default::default()` of `Settings` has these all set so the
   neural layer is **disabled by default**.
4. New `data/models/` directory for neural-model binaries. Empty by
   default; populated only when an operator chooses to enable the
   neural layer.

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

After §3 is green, you may enable the L5.5 neural composition layer.
This is the path that exercises the v6.0 feature set.

### 4.1 Place the neural model artefact

```bash
mkdir -p data/models
curl -L -o data/models/adam_agg_v6_0_0.bin \
  https://github.com/qazaq-ai/adam/releases/download/v6.0.0/adam_agg_v6_0_0.bin
sha256sum data/models/adam_agg_v6_0_0.bin
# Verify against the SHA published in data/models/adam_agg_v6_0_0.md.
```

### 4.2 Enable in Settings

In your deployment's configuration code (or `adam_chat` CLI flags):

```rust
let settings = Settings {
    neural: NeuralSettings {
        enabled: true,
        model_path: "data/models/adam_agg_v6_0_0.bin".into(),
        time_budget_ms: 80,    // hard timeout per turn
        verifier_strict: true, // block on missing factual grounding
        ..NeuralSettings::default()
    },
    ..Settings::default()
};
```

Or via CLI:

```bash
adam_chat \
  --neural-model=data/models/adam_agg_v6_0_0.bin \
  --neural-time-budget-ms=80 \
  --neural-verifier-strict
```

### 4.3 Smoke-test before serving

```bash
cargo test --workspace --release \
  --features adam-agg-model/neural-runtime
```

The neural-runtime feature gate exercises L5.5 in the integration
tests. If green, the deployment is ready to serve.

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
A. Use the `Settings::neural.enabled` flag at the request level
(supported from v6.0.0). The `TurnTrace::neural_calls` field tells
you for each turn whether L5.5 fired.

---

## 8. Pre-registered alpha-partner deployment

The v6.0 acceptance criterion #6 in
[`architecture_neural_v6.md`](architecture_neural_v6.md) §9 requires
the migration to be validated against an **external alpha
deployment** for at least one week. The alpha-partner contract
template lives at `docs/outreach/alpha_partner_contract.md`
(forthcoming). Any organisation interested in being the alpha
partner should email `baimurza.daulet@gmail.com` with their
deployment topology.
