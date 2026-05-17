# adam v6.0 — neural-enabled architecture reference

**Status:** SPECIFICATION (not yet implemented in `main`). Target
release: v6.0.0. Branch of record: `experimental/agglutinative-neural`.
**Audience:** every developer, partner, and downstream system that
needs to know how the v6.0 neural layer fits into the deterministic
kernel without breaking the v3.0–v5.x guarantees.
**Supersedes:** nothing yet. Complements [`architecture_v3.md`](architecture_v3.md)
which describes the deterministic retrieval-and-reasoning core that
remains load-bearing under the v6.0 additions.

---

## 1. Scope of this document

v6.0 introduces a **neural composition layer** between the parser and
the verifier. The deterministic L1 (FST parsing), L2 (semantics +
intent), L3 (retrieval), L4 (reasoning), L5 (dialog templates), L6
(verifier), L7 (ontology audit), L8 (TTS / multimodal) layers from
v3.0–v5.x remain in place. The neural layer is **strictly
additive** — every guarantee that v5.x makes about determinism,
provenance, latency, and hallucination-blocking is preserved.

This document specifies:

1. **Where the neural layer sits** in the request pipeline.
2. **Input / output contracts** between neural and deterministic
   components.
3. **Failure modes** and the deterministic fallbacks for each.
4. **Performance contracts** (latency, RSS, energy budgets).
5. **Audit trail integration** — every neural output is traceable.
6. **Migration path** for existing v5.x deployments.
7. **Versioning + rollback strategy** for neural-layer revisions.

What this document does **not** cover (separate documents):
- Neural-model training procedure → `docs/research/results_real_mix_2026_05_16.md`
  and the `experimental/agglutinative-neural` branch source.
- Lexicon V2 schema → see Lexicon V2 RFC (forthcoming).
- LLM-baseline benchmark methodology → see `docs/bench/llm_baseline.md`
  (forthcoming).
- The "why" — see [`MANIFESTO.md`](MANIFESTO.md).

---

## 2. The v6.0 request pipeline

```
                    ┌─────────────────────────────────────────────────┐
  user input ─────▶ │  L1  parser           (adam-kernel-fst)         │
                    │      FST analyse → list<Analysis>               │
                    └──────────────────────────┬──────────────────────┘
                                               │
                                               ▼
                    ┌─────────────────────────────────────────────────┐
                    │  L2  semantics        (adam-dialog)             │
                    │      interpret_text_with_lexicon → Intent       │
                    └──────────────────────────┬──────────────────────┘
                                               │
                                               ▼
                    ┌─────────────────────────────────────────────────┐
                    │  L3  retrieval        (adam-retrieval)          │
                    │      facts + derived_facts + morpheme_index     │
                    └──────────────────────────┬──────────────────────┘
                                               │
                                               ▼
                    ┌─────────────────────────────────────────────────┐
                    │  L4  reasoning        (adam-reasoning)          │
                    │      10 rules → DerivedFact graph               │
                    └──────────────────────────┬──────────────────────┘
                                               │
                                               ▼
                    ┌─────────────────────────────────────────────────┐
                    │  L5  template          (adam-dialog)            │ ◀── deterministic
                    │      pick canonical template by intent          │     v3.0 path
                    └──────────────────────────┬──────────────────────┘     remains
                                               │                            unchanged
                                               ▼
                    ┌─────────────────────────────────────────────────┐
                    │  L5.5 NEURAL COMPOSITION (NEW IN v6.0)          │ ◀── new in v6.0
                    │      adam-agg-model · FST-constrained decode    │     additive only
                    │      input:  Intent + Retrieval context         │
                    │      output: candidate surface forms            │
                    │      gate:   ALGEBRAICALLY VALID                │
                    └──────────────────────────┬──────────────────────┘
                                               │
                                               ▼
                    ┌─────────────────────────────────────────────────┐
                    │  L6  verifier         (adam-reasoning)          │
                    │      Verifier::check → Pass | Block             │
                    │      gate:   FACTUALLY GROUNDED                 │
                    └──────────────────────────┬──────────────────────┘
                                               │
                                               ▼
                    ┌─────────────────────────────────────────────────┐
                    │  L7  ontology audit   (adam-reasoning)          │
                    │      audit_response · audit_trace_faithfulness  │
                    └──────────────────────────┬──────────────────────┘
                                               │
                                               ▼
                    ┌─────────────────────────────────────────────────┐
                    │  L8  TTS / surface    (adam-voice + dialog)     │
                    └─────────────────────────────────────────────────┘
```

**The neural layer is L5.5**, gated on both sides by deterministic
components. L5 (template) provides the canonical scaffold; L5.5
chooses among algebraically valid surface realisations; L6 (verifier)
gates the result on factual groundedness before it leaves the system.

Crucially: **the system can answer with L5 alone if L5.5 is
unavailable, slow, or rejected by the verifier.** L5.5 is an
*improvement* layer, not a critical-path layer. See §4.

---

## 3. Input / output contracts

### 3.1 L5 → L5.5 (template scaffold → neural composer)

```rust
pub struct NeuralComposeRequest {
    /// Resolved intent from L2.
    pub intent: Intent,
    /// Retrieval context from L3 (selected facts + supports).
    pub retrieval: RetrievalContext,
    /// The deterministic L5 canonical template, already filled in.
    /// L5.5 may improve this surface but must preserve every
    /// `placeholder_var -> filled_value` binding the template carries.
    pub canonical_surface: String,
    /// Generation budget in milliseconds. If exceeded, L5.5
    /// returns the canonical surface unchanged.
    pub time_budget_ms: u32,
    /// Maximum new tokens the model may emit. Hard cap regardless
    /// of time budget.
    pub max_new_tokens: u16,
}

pub enum NeuralComposeResponse {
    /// Model produced a candidate; ready for L6.
    Candidate {
        surface: String,
        /// Probability mass on the chosen path, for audit + diagnostics.
        log_prob: f32,
        /// Every (token, slot, FST state) emitted, for the audit log.
        trace: Vec<NeuralTraceEntry>,
        elapsed_ms: u32,
    },
    /// Model unavailable, hit the time budget, or violated the
    /// FST constraint at every alternative. Caller MUST fall back
    /// to `canonical_surface` and proceed to L6 with that.
    FallbackToCanonical { reason: FallbackReason },
}

pub enum FallbackReason {
    ModelUnavailable,
    TimeBudgetExceeded,
    NoValidContinuation,
    HardwareError,
}
```

### 3.2 L5.5 → L6 (neural composer → verifier)

The verifier (`adam-reasoning::Verifier`) accepts both the candidate
surface and the trace. It evaluates:

1. **Algebraic validity** (re-check, defensive): every token is a
   morphotactically valid continuation from the previous state.
   The FST already enforced this during decoding; the verifier
   re-checks to catch any model bug that disobeyed the mask.
2. **Factual groundedness**: any noun-phrase that appears to assert
   a fact must trace to a `Fact` or `DerivedFact` in the retrieval
   context. Unsupported assertions are blocked.
3. **Provenance preserved**: every token in the output has a
   non-empty trace entry; missing trace entries fail audit.

A `Block` outcome forces fallback to the canonical surface for that
turn. A `Pass` releases the candidate to L7.

### 3.3 Audit trail

Every L5.5 invocation appends a record to `TurnTrace::neural_calls:
Vec<NeuralCallRecord>`:

```rust
pub struct NeuralCallRecord {
    pub model_id: ModelId,
    pub model_revision: Version,
    pub request_hash: u64,
    pub response: NeuralComposeResponse,
    pub verifier_outcome: VerifierOutcome,
    pub elapsed_ms: u32,
}
```

This is queryable via `adam_inspect` and surfaced in `adam-eval`
benchmark reports. **No neural output reaches the user without an
audit record.**

---

## 4. Failure modes & fallbacks

Every failure mode for the neural layer falls back to a deterministic
behaviour with explicit user-visible status. There is **no failure
mode that silently degrades** to a worse-than-v5.x experience.

| Failure | Detection | Fallback | User-visible effect |
|---|---|---|---|
| Model file missing on disk | startup check | Skip L5.5 entirely | identical to v5.x for the session |
| Model crash mid-inference | runtime panic boundary | `FallbackToCanonical { HardwareError }` | turn uses canonical surface; logged to telemetry |
| Time budget exceeded | watchdog timer | `FallbackToCanonical { TimeBudgetExceeded }` | turn uses canonical surface |
| FST-constraint blocks every candidate | beam-search exhaustion | `FallbackToCanonical { NoValidContinuation }` | turn uses canonical surface; logged as `nopath_event` |
| Verifier blocks neural output | L6 returns `Block` | use canonical surface; record `verifier_block` | turn uses canonical surface |
| OOM | memory budget hit | abort L5.5 for this turn; log `oom_event` | turn uses canonical surface |
| GPU unavailable (where used) | device probe failure | route to CPU backend | latency cost; no functional impact |

**Invariant:** if `data/models/adam_agg_v6.bin` is absent, the
binary boots, runs, and answers correctly. v6.0 is a strict superset
of v5.x.

---

## 5. Performance contracts

These are **release-blocking** contracts. A v6.x release that
violates any of these on M2 8 GB (reference hardware) is reverted.

| Metric | v5.x baseline | v6.0 contract | Source |
|---|---|---|---|
| p50 turn latency, deterministic-only path | 21 ms | ≤ 25 ms (no regression > 20 %) | `crates/adam-dialog/benches/turn_latency.rs` |
| p50 turn latency, neural-enabled path | n/a | ≤ 150 ms | new bench: `crates/adam-agg-model/benches/neural_turn_latency.rs` |
| p99 turn latency, neural-enabled path | n/a | ≤ 400 ms | same bench |
| Resident set size (RSS) without model loaded | 160 MB | ≤ 200 MB | `docs/performance.md` |
| RSS with v6.0 default neural model loaded | n/a | ≤ 320 MB | new metric |
| Energy per turn (J), neural-enabled, M2 CPU only | n/a | ≤ 0.40 J | new bench: `crates/adam-agg-model/benches/neural_energy.rs` |
| GPU dependency | 0 % | 0 % (still CPU-only by default) | architectural invariant |
| Cold-start to first answer | < 600 ms | ≤ 900 ms | `crates/adam-dialog/benches/cold_start.rs` |

The neural-enabled path is selectable per turn via the `time_budget_ms`
field of `NeuralComposeRequest`. A caller that needs the v5.x latency
budget passes `time_budget_ms = 0` and L5.5 is skipped entirely.

---

## 6. Migration path for v5.x users

v6.0 is **strict-superset compatible** with v5.x. There is no
breaking change in user-facing surfaces. Specifically:

1. **All v5.x APIs preserved.** `adam_chat::answer` returns the
   same `Answer` shape with the same fields.
2. **Configuration is opt-in.** Neural layer is **off by default**
   in `Settings::default()`. To enable: `Settings { neural:
   NeuralSettings { enabled: true, model_path: "...", ..default()
   }, ..default() }`.
3. **Same `data/` layout.** No corpus files change shape. The
   `data/lexicon_v1/` files are read-compatible with v6.0; Lexicon
   V2 entries are additive and live alongside V1 entries.
4. **Same eval harness.** `cognitive_eval`, `repl_replay`, and
   foundation eval datasets pass on v6.0 with neural disabled.
5. **Same release artefacts.** `adam_chat` binary, `adam_demo`
   binary, `adam_inspect` are all built. New binary `adam_agg_demo`
   demonstrates the neural composition; not required for normal
   operation.

### 6.1 Upgrade procedure

```bash
# Existing v5.x deployment.
cargo update -p adam_chat                  # → v6.0
# adam_chat runs unchanged. Neural layer is off by default.

# To enable neural composition:
cargo run --release --bin adam_chat -- \
  --neural-model=path/to/adam_agg_v6.bin \
  --neural-time-budget-ms=80
```

### 6.2 Rollback

If a v6.x release misbehaves:

```bash
cargo update -p adam_chat --precise=5.32.0
# Deployment reverts to deterministic-only path; no data migration.
```

Because the neural layer is additive and the underlying corpus +
Lexicon are forward-compatible, downgrade is symmetric and safe.

---

## 7. Versioning + rollback strategy for the neural layer

Neural-model revisions are first-class versioned artefacts.

- Model binary path: `data/models/adam_agg_v<major>_<minor>_<patch>.bin`
- Model card path: `data/models/adam_agg_v<major>_<minor>_<patch>.md`
  (training data manifest, eval scores, license, sha256, size).
- `Cargo.toml` of `adam-agg-model` records the **default** model
  version it expects. A binary built against expected `v6.0.0`
  refuses to load `v6.1.0` unless `--allow-newer-model` is passed.

### 7.1 Release cadence

Patch releases (`v6.0.x`) for bug fixes that keep the trained model
unchanged. Minor releases (`v6.x.0`) when the model is retrained
without architectural change. Major releases (`v7.0.0`) when the
architecture changes (e.g. multi-language extension, larger
parameter count beyond the published contract).

### 7.2 Model deprecation

A model artefact remains supported for **two minor releases** after
its replacement. So `v6.0.0` model is supported through `v6.2.x`;
`v6.3.x` may require an upgrade. Deprecation is announced one minor
release in advance via `data/models/DEPRECATIONS.md`.

---

## 8. Open questions (resolved before v6.0.0 GA)

These are tracked in `docs/roadmap.md` under the v6.0 milestone.

1. **GPU optional backend.** burn supports WGPU + Metal. Should
   adam-agg-model expose a feature flag for them in v6.0 or wait for
   v6.1? Current stance: wait, ship CPU-only first to keep the
   watch-battery promise visible.
2. **Multi-language model packing.** When Lexicon V2 supports Kyrgyz
   and Tatar, do we ship one cross-language model or one per language?
   Current stance: per-language until cross-lingual benefit is
   measured.
3. **Differential privacy on training data.** Real-corpus pack
   contains content from many sources. Per the BUSL clause we are
   non-competing-commercial-OK, but we have not formally audited
   for PII. Required before any release that uses user-supplied
   training data.
4. **Energy bench methodology.** `Watt-hours per turn` is harder
   to measure on macOS than on Linux. Reference methodology TBD.

---

## 9. Acceptance criteria for v6.0.0 GA

The v6.0.0 release ships when **all** of the following are true.
Anything missing reverts to v5.x.

- [ ] All v5.x release-blocker tests pass (foundation, cognitive_eval,
  repl_replay, benchmark_delta).
- [ ] All performance contracts in §5 are met on M2 8 GB reference
  hardware.
- [ ] Lexicon V2 yields ≥ 70 % Root coverage on the canonical
  Kazakh real-corpus eval set.
- [ ] Verifier integration: 0 hallucinations on the 100-prompt
  factual eval set; every neural output has a complete audit
  record.
- [ ] Characteristics comparison published. adam v6.0's measured
  numbers on the six v6.0 axes (latency / memory / cost /
  determinism / hallucination / audit) are published alongside
  well-attested published LLM numbers in
  [`docs/bench/our_numbers_vs_published_llm.md`](bench/our_numbers_vs_published_llm.md).
  Updated 2026-05-17: this replaces the former "run head-to-head
  API benchmark" criterion. Rationale: a paid head-to-head bench
  is premature before our own numbers are stable and adds $90-
  $200 + 2-3 weeks of evaluator work that grants / commercial
  funding cover better post-GA. The full head-to-head benchmark
  is deferred per docs/bench/our_numbers_vs_published_llm.md
  "When a full head-to-head bench will be needed".
- [ ] arXiv preprint accepted (or under review with a stable DOI).
- [ ] Migration plan validated against an external alpha deployment.
- [ ] No new dependency adds proprietary or cloud-only components.
  (Build-time + run-time dependency tree audited.)
- [ ] CHANGELOG.md + docs/roadmap.md + docs/MANIFESTO.md all
  reflect v6.0.0.

---

*This document is the v6.0.0 architecture contract. Substantive
changes are tracked in pull requests against this file; the current
version is in the git history under `docs/architecture_neural_v6.md`.*
