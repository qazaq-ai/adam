# Roadmap — v6.0.0 → v7.0.0

**Status:** active, last updated 2026-05-16.
**Branch of record:** `experimental/agglutinative-neural` until v6.0.0
GA, then `main`.
**Architecture spec:** [`architecture_neural_v6.md`](architecture_neural_v6.md)
**Position / philosophy:** [`MANIFESTO.md`](MANIFESTO.md)
**Empirical baseline:** [`research/results_real_mix_2026_05_16.md`](research/results_real_mix_2026_05_16.md)

This roadmap is the **public commitment** of what ships under each
release tag from v6.0.0 through v7.0.0. Items inside a release block
are release-blockers unless flagged otherwise; the release does not
ship until they are all green.

For per-release history of v0.x–v5.x see [`roadmap.md`](roadmap.md).
For the v6.0 architecture contract that every release here honours
see [`architecture_neural_v6.md`](architecture_neural_v6.md) §5
(performance contracts) and §9 (GA acceptance criteria).

---

## v6.0.0 — Algebra-anchored Neural Composition Layer (target Q3 2026)

The headline release. Adds the L5.5 neural composition layer between
the deterministic template (L5) and the verifier (L6). Strictly
additive: a v5.x deployment can upgrade and run with the neural layer
disabled and observe zero functional change.

### Release-blocking items

| # | Item | Status | Est. duration | Owner |
|---|------|--------|---------------|-------|
| 1 | Lexicon V2 — Root yield on real corpus ≥ 70 % | review queue ready (2 000 candidates) | 3–6 weeks (native-speaker bound) | linguist + shaman |
| 2 | Verifier integration — FST + world_core + facts + audit trail | not started | 2–3 weeks | shaman |
| 3 | Latency + energy benchmark — multi-hardware (M2 / M1 / Intel / ARM), CI-enforced | M2 bench landed; multi-hw pending | 1 week after Lexicon V2 done | shaman |
| 4 | Characteristics comparison (`docs/bench/our_numbers_vs_published_llm.md`) — adam's measured six-axis numbers + well-attested published LLM numbers. Head-to-head bench deferred until grant funding. | landed 2026-05-17 | done | shaman |
| 5 | Test suite expansion — workspace tests ≥ 500 including property-based + fuzzing | currently ~1442 (production) + 19 (research arc); structure ok, depth needs work | 2–3 weeks | shaman |
| 6 | Migration plan validated against an external alpha deployment | not started | 1 week + alpha-partner window | shaman + alpha partner |
| 7 | arXiv preprint accepted (or under review with stable DOI) | results note ready as draft | 1–2 weeks writing + ~4-12 weeks peer review | shaman |
| 8 | All v5.x release-blocker tests still pass (foundation, cognitive_eval, repl_replay) | green | continuous | CI |
| 9 | Documentation: every doc reflects v6.0 | wave 1+2 done; tier 3 docs pending | 2-3 days | shaman |

### Out of scope for v6.0.0

- Multi-language extension (Kyrgyz / Tatar / Uyghur). Deferred to v6.1.x.
- GPU backend (WGPU / Metal feature flag in burn). Deferred to v6.1.x.
- Differential privacy audit of training data. Required only if/when
  v6.x consumes user-supplied training data; current v6.0 trains on
  public corpora only.
- Voice / TTS regression against v5.32 baseline. Out-of-scope but
  must not regress.

### v6.0.0 GA Acceptance — all of:

- All nine items above closed green.
- Architecture spec §5 performance contracts met on reference
  hardware. The bench harness lives in
  `crates/adam-agg-model/benches/neural_inference.rs` and the
  numbers are checked-in under `docs/performance.md`.
- One external alpha-partner deployment has run for ≥ 1 week
  without rollback.

---

## v6.1.x — Multi-language extension (target Q4 2026)

First post-GA series. Extends ARK from Kazakh-only to multi-Turkic.

### v6.1.0

- Lexicon for **Kyrgyz** (~25 k roots; closest cousin to Kazakh).
- Per-language FST template wiring under
  `crates/adam-kernel-fst/src/morphotactics_kg.rs`.
- Eval harness extended: `data/eval/kyrgyz_holdout.json`.

### v6.1.5

- **Tatar** Lexicon (~25 k roots). Tatar diverges more on phonology
  than Kyrgyz — `adam-kernel-fst::phonology_tt.rs` work.

### v6.2.0

- **Uyghur** Lexicon. Different script (Arabic vs Cyrillic) — script
  routing in `adam-kernel-fst::script`.
- Cross-language transfer benchmark: does a model pretrained on
  Kazakh transfer to Kyrgyz / Tatar / Uyghur with what fraction of
  the training-from-scratch cost? Published as a benchmark report
  under `docs/bench/multi_turkic_transfer.md`.

### v6.2.5

- **Yakut** + **Tuvan** (Siberian Turkic). Strategic anchor for
  Russian-Federation Turkic-language sovereignty interests.

---

## v6.3.x — Local-GPU optional backend (target Q1 2027)

The CPU-only contract from v6.0 stays the default. v6.3.x adds an
**optional** GPU backend behind a Cargo feature flag for users who
want lower latency on local hardware. No cloud GPU dependency.

### v6.3.0

- `adam-agg-model` Cargo feature `gpu-metal` — burn's Metal backend
  for Apple Silicon. Verified to produce bit-for-bit identical
  outputs to the CPU path (modulo FP non-associativity, documented).
- `adam-agg-model` Cargo feature `gpu-wgpu` — burn's WGPU backend
  for cross-vendor GPUs.
- CI: both feature flags compile-tested on every PR, run-tested
  weekly on the relevant hardware.

### v6.3.5

- KV-cache implementation for decoder inference. Cuts greedy-
  generation cost roughly N× for N-token output. Released as a
  performance bump only; no API change.

---

## v7.0.0 — Verifier-bounded factuality across the whole pipeline (target Q3 2027)

The major release that closes the gap between "morphologically
valid" and "factually grounded" at every layer. Every neural output
already goes through the v6.0 verifier; v7.0.0 extends the verifier
to also gate:

- **Reasoning rules.** Derived facts that contradict a prior derived
  fact require reconciliation via the v4.4.0 dismiss-contradiction
  pathway.
- **Tool outputs.** Any `ToolResult` consumed by the dialog layer
  carries a typed-evidence proof that the verifier checks.
- **Cross-session facts.** A fact learned in session A and recalled
  in session B must trace back to session A's evidence chain.

### v7.0.0 acceptance

- 0 hallucinations on a 1 000-prompt adversarial factual eval set
  (versus the LLM-baseline rates on the same set, published).
- p50 turn latency ≤ 200 ms with verifier extension active.
- All v6.x tests still pass.

---

## What this roadmap is and is not

**Is.** The public contract for what we plan to ship. Substantive
changes are tracked in pull requests against this file.

**Is not.** A guarantee against scope adjustments based on what the
empirical results show. If Lexicon V2 yield plateaus below 70 %,
v6.0.0 is **postponed**, not shipped with a lower bar.

**Is not.** A commitment to chase any specific LLM benchmark
number. Our LLM-baseline benchmark (release-blocker #4) is **published
once and frozen**; we don't retune to game it.

---

## Versioning conventions

Same as v0.x–v5.x — see [`roadmap.md`](roadmap.md) under "Versioning
cadence":

- Patch increments `x.y.{1, 2, 3, …}` — text, docs, positioning.
- Patch milestone `x.y.5` — small functional changes.
- Minor `x.y.0` — significant capability changes; kernel-signature
  features.
- Major `x.0.0` — architecture-defining; everyone re-reads the
  architecture document for that release line.

The neural-model artefact is versioned separately from the binary;
see [`architecture_neural_v6.md`](architecture_neural_v6.md) §7.
