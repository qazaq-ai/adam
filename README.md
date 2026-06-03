<p align="center">
  <img src="assets/shanraq.svg" alt="adam logo" width="128" height="128">
</p>

<h1 align="center">adam</h1>

<p align="center">
  <i>Neurosymbolic agglutinative AI — typed, deterministic, watch-class fast.</i><br>
  <i>Kazakh-first applied demonstrator of the Qazaq IR architecture.</i><br>
  <i>Қазақ тіліне арналған, толық болжамды диалог жүйесі — таза Rust тілінде.</i>
</p>

<p align="center">
  <b>Why this project exists →</b> <a href="docs/MANIFESTO.md"><code>docs/MANIFESTO.md</code></a><br>
  <b>v6.2 architecture →</b> <a href="docs/v6_2_architectural_redesign.md"><code>docs/v6_2_architectural_redesign.md</code></a><br>
  <b>Due-diligence pack →</b> <a href="DUE_DILIGENCE.md"><code>DUE_DILIGENCE.md</code></a>
</p>

<!-- ─────────────────────────────────────────────────────────────
     v6.3 honest split (2026-06-02 codex audit follow-up).
     Production = v6.2 on `main` (numbers below).
     Research-demo = v6.3 on experimental/v6_3_phonemic_foundation.
     ───────────────────────────────────────────────────────────── -->

> **Two tracks, on purpose.**
>
> - **`main` — v6.2 deterministic core.** Rule-based dialog kernel:
>   morph → frame → QueryIR → retrieval → realiser. **0 MB models, 0
>   GPU, byte-deterministic.** This is the production narrative below.
>
> - **`experimental/v6_3_phonemic_foundation` — v6.3 voice surface.**
>   Wraps the v6.2 core in a microphone → STT → fuzzy / LM rescoring →
>   intent classifier → router → TTS loop. **Honest hybrid:** the
>   dialog core stays 100 % deterministic; neural components (Whisper.cpp
>   STT, Piper TTS, ~1 M-param BPE LM, ~1 M-param intent classifier)
>   live only at the speech surface and never invent facts. Standalone
>   binary at <code>tools/voice_repl_v6_3</code>. Research-demo
>   maturity; not yet on <code>main</code>.
>
> See <a href="DUE_DILIGENCE.md"><code>DUE_DILIGENCE.md</code></a>
> for current test totals, repo state, known limitations and
> reproducibility commands across both tracks.

<p align="center">
  <a href="https://github.com/qazaq-ai/adam/releases"><img src="https://img.shields.io/badge/version-6.3.0-2EA44F?style=for-the-badge" alt="version"></a>
  <a href="https://github.com/qazaq-ai/adam/actions/workflows/rust.yml"><img src="https://img.shields.io/github/actions/workflow/status/qazaq-ai/adam/rust.yml?branch=main&style=for-the-badge&label=CI" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-BUSL%201.1-orange?style=for-the-badge" alt="license"></a>
  <img src="https://img.shields.io/badge/language-Rust-CE412B?style=for-the-badge&logo=rust&logoColor=white" alt="rust">
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Linux-lightgrey?style=for-the-badge" alt="platform">
  <a href="https://github.com/qazaq-ai/adam/commits/main"><img src="https://img.shields.io/github/last-commit/qazaq-ai/adam?style=for-the-badge" alt="last commit"></a>
  <a href="https://github.com/qazaq-ai/adam/stargazers"><img src="https://img.shields.io/github/stars/qazaq-ai/adam?style=for-the-badge" alt="stars"></a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/tests-1477%20passing%20%2F%201%20red-FBC02D?style=flat-square" alt="tests">
  <img src="https://img.shields.io/badge/dialog%20battery-79%2F79%20must--pass-2EA44F?style=flat-square" alt="dialog battery">
  <img src="https://img.shields.io/badge/v6.2%20warm%20p50-917%20ns-2EA44F?style=flat-square" alt="warm latency">
  <img src="https://img.shields.io/badge/v6.2%20core-0%20MB%20%2F%200%20GPU-2EA44F?style=flat-square" alt="v6.2 core">
  <img src="https://img.shields.io/badge/v6.3%20voice%20surface-Whisper%20%2B%20Piper%20%2B%20tiny%20LM-9CCC65?style=flat-square" alt="v6.3 voice">
  <img src="https://img.shields.io/badge/world%20core-3444%20curated%20/%204116%20facts-9CCC65?style=flat-square" alt="world core">
  <img src="https://img.shields.io/badge/lexicon-25.5%20k%20roots-FBC02D?style=flat-square" alt="lexicon">
  <img src="https://img.shields.io/badge/intents-41%20router%20%2F%2052%20neural%20classifier-2EA44F?style=flat-square" alt="intents">
  <img src="https://img.shields.io/badge/hallucinations%20within%20curated%20domains-0-2EA44F?style=flat-square" alt="hallucinations within curated domains">
</p>

---

## What's new in v6.2.0

**Neurosymbolic agglutinative algebra (env-gated).** The
architectural redesign promised at v6.1.50 lands as the new
[`adam-algebra`](crates/adam-algebra) crate plus an integration
bridge in `adam-dialog::v6_2_router` gated by `ADAM_V6_2=1`. v6.1
cascade is unchanged when the gate is off.

The full v6.2 pipeline runs as **pure typed-data manipulation**:

```text
input → morph lattice → Composition[] → Frame → QueryIR
                                          ↓
                              FrameIndex + math_solver + system_clock
                                          ↓
                                       realiser → output
```

### Headline numbers (release build, M2 Air, single CPU core)

| | adam ARK v6.2 | GPT-class LLM (one call) |
|---|---|---|
| Median warm latency | **274 ns** (Stage 3 pipeline) | ~100–500 ms |
| Median full dialog battery | **791 ns / query** | (same) |
| Throughput | **~1.26M qps / core** | ~2–10 qps |
| Model size | **0 MB** | 5–600 GB |
| GPU required | **No** | Yes |
| Deterministic | **Yes** (byte-identical) | No |

**≈ 126 000× faster** than a fast (100 ms) LLM call on a single
CPU core, with **0 MB model loaded**.

### What ships in `adam-algebra` (8 modules, 195 tests)

- **Stage 1 — `operator` + `root` + `composition`** — typed
  agglutinative algebra. `Root × SuffixOp[]` with slot bookkeeping
  (case / number / possessive / tense / voice / negation / polite).
  Round-trip with `adam_kernel_fst::Analysis` byte-stable.
- **Stage 2 — `frame`** — `Frame { agent, predicate, object,
  modifiers, modality, polarity, evidentiality, tense, aspect }`.
  Subsumes v6.1's scattered `Fact` / `SentenceFrame` /
  `SentenceDecomposition` / `Claim`. 22 typed predicates.
- **Stage 3 — `query`** — typed «frame with a hole»:
  `QueryIR { …, focus, form, answer_shape, sense_hints,
  domain_filter, language_filter }`. Subsumes v6.1's
  `PredicateFocus` + `QuestionShape` + `AnswerShape` + `Intent`
  + slot_inventory.
- **Stage 4 — `index`** — `FrameIndex` with 5 secondary indexes
  (predicate / agent / object / modifier / domain / language).
  Insert O(M), query O(min-index-size). Property-tested against
  brute-force linear filter.
- **Stage 4.5 — `dialog_battery`** — 79 real Kazakh / Russian
  REPL questions across 14 domains (biographical / geographical /
  institutional / legal / sense-ambiguous / МО РК / programming /
  Rust / math / system-clock / Soviet+Kazakh historical dates).
  **CI quality gate: 79/79 must-pass, 0 known gaps, 0 regressions.**
- **Stage 4.6+ — `math_solver`** — pure-function deterministic
  math in Russian + Kazakh + ASCII. Numbers 0-99 in both
  languages, operators (+ − × ÷ × ÷ mod ^ %), trig (sin / cos /
  tan / arcsin / arccos / arctan), constants π / e, square root,
  decimals. Left-to-right chained-imperative semantics.
- **Stage 5 — sense disambiguation** — cross-shape
  `TimeAnchor::Year(N)` ↔ phrase «N жыл» matching; bilingual
  language-tag filter for same-root same-domain ambiguity
  («гравитация» / «фотосинтез» / «днк» KZ vs RU).
- **Stage 7.1 — `corpus_loader`** — `load_world_core(path)`
  reads `data/world_core/*.jsonl`. **66 files → 3461 entries →
  4044 frames → 0 dropped**.
- **Stage 7.2 — `realiser`** — typed `Frame → Kazakh surface`.
  Pure function. Every v6.1 NLG rule family expressible.
- **Stage 7.3 — `system_clock`** — OS wall-clock provider with
  hand-rolled Kazakh calendar (no chrono dep).

### What ships in `adam-dialog::v6_2_router`

- `is_v6_2_active()` — reads `ADAM_V6_2` env var (default off).
- `answer(input) -> Option<String>` — top-level entry. Routes:
  math_solver / system_clock / FrameIndex + realiser.
- Lazy `OnceLock` corpus from `data/world_core/*.jsonl` +
  battery-augmented Russian aliases.
- `tests/v6_2_integration.rs`: 11 real Kazakh / Russian questions
  end-to-end. All pass.

### Live REPL demo

```bash
# Interactive (math, time, retrieval — all routed):
cargo run --release --example chat -p adam-algebra

# Latency bench + dialog battery report:
cargo run --release --example bench_pipeline -p adam-algebra

# Quality gate:
cargo test -p adam-algebra dialog_battery_meets_quality_gate -- --nocapture

# Integration through adam-dialog router (env-gated):
ADAM_V6_2=1 cargo test -p adam-dialog --test v6_2_integration
```

### Default-off discipline

`ADAM_V6_2` defaults OFF. Every existing v6.1 user / CI run sees
no behavioural change. Stage 8 (HumanDialogEval v1) will decide
when to flip the default after a curated 100-prompt battery
passes ≥ 90 % relevance.

See [`CHANGELOG.md` § 6.2.0](CHANGELOG.md) for the full inventory
and [`docs/v6_2_architectural_redesign.md`](docs/v6_2_architectural_redesign.md)
for the design doc that preceded the work.

---

## 30-second pitch

> **adam is a neurosymbolic AI research kernel** built on the
> agglutinative morphology of Kazakh. Every output traces to a
> curated source via a typed pipeline (Composition → Frame →
> QueryIR → FrameIndex → realiser). Three structural advantages
> over LLMs by **construction**:
>
> 1. **Predictability** — every claim cites a curated `(pack,
>    sample_id)` provenance; the cascade is byte-identical given
>    `(input, seed, world_core)`.
> 2. **Cheapness** — single Rust binary, **0 MB model**, **0 %
>    GPU**, **791 ns warm median latency** on one M2 core
>    (~1.26M qps). Watch-class deployable.
> 3. **Architectural hallucination-freedom within curated domains** —
>    every fact-bearing reply emits from a `FrameIndex` hit + Stage 7
>    realiser, or falls through to v6.1 cascade / honest refusal. No
>    probabilistic free generation anywhere in the answer path.
>    *Scope caveat:* «0 hallucinations» holds for queries the
>    curated corpus covers; off-corpus queries route to v6.1
>    fallback (which can still misroute on edge cases — see the
>    open Stage 8 backlog) or to «нет данных».
>
> Designed to extend across ~30 catalogued agglutinative
> languages — currently demonstrated on Kazakh; cross-language
> ports are a research goal, not a shipped capability. Pure Rust.
> BUSL-1.1.

> **Reading order:** [MISSION.md](MISSION.md) (thesis) →
> [v6.2 design doc](docs/v6_2_architectural_redesign.md) (current
> architecture) → [RESEARCH.md](RESEARCH.md) (open questions) →
> [COLLABORATION.md](COLLABORATION.md) (partner terms) →
> [AGENTS.md](AGENTS.md) (orientation for automated scouts) →
> [CHANGELOG.md](CHANGELOG.md) (full release history).

## Why neurosymbolic (not LLM)

Modern LLMs carry three structural problems we treat as **not
inevitable**. v6.2's *neurosymbolic* architecture means: neural
components produce **typed closed-set** outputs only (sense
disambiguation candidates, focus detection, intent class); a
deterministic verifier owns truth. No free text generation
anywhere in the answer path.

| The three diseases of probabilistic AI | adam's target | How v6.2 enforces it |
|---|---|---|
| **Black box** — opaque internals, no source attribution | **Predictability** — every claim traceable | `FrameIndex.query` returns a `RankedFrame` with `FrameId`; `world_core/*.jsonl` is the only fact source; every realised sentence is a function of `(Frame, focus, slot)` |
| **Resource cost** — billions of params, GPU clusters | **Cheapness** — single binary | 0 MB model, 0 % GPU, 791 ns p50 dialog battery on M2 single core |
| **Hallucination risk** — confident generation of plausible-sounding wrong content | **Safety** — architectural impossibility | Realiser is a typed pure function over a curated Frame; if the index returns `None`, the router falls through to v6.1 cascade (no invention) |

**Hypothesis:** agglutinative languages — Kazakh in particular —
exhibit unusually mathematical morphology. Every word decomposes
into a root + a typed suffix chain (case, number, tense, person,
possessive, polarity, modality). Composition is **rule-bound**.
That structure becomes the substrate for a typed runtime: FST
morphology produces `Composition`, the algebra layer lifts it
into `Frame`, retrieval typed via `QueryIR`, output realised as
pure-function `Frame → surface`. **No probabilistic free
generation. No retrained-from-scratch behaviour per release.**

## Quick start

```bash
# v6.2 live REPL demo (math + clock + retrieval routed):
cargo run --release --example chat -p adam-algebra

# v6.2 latency bench + dialog battery quality report:
cargo run --release --example bench_pipeline -p adam-algebra

# v6.2 integration test through adam-dialog router:
ADAM_V6_2=1 cargo test -p adam-dialog --test v6_2_integration

# v6.1 cascade (default, unchanged):
cargo build --release -p adam-dialog --bin adam_chat
./target/release/adam_chat
./target/release/adam_chat --once "Қасқыр — тірі ме?"
./target/release/adam_chat --trace
./target/release/adam_chat --tts

# FST synthesiser + analyser:
cargo run --release -p adam-kernel-fst --bin adam_fst -- synth --root бала --plural --case dat
# → балаларға

# Full foundation validation (~30 s on M2):
bash ./scripts/validate_foundation.sh
```

## Architecture — ARK (Agglutinative Reasoning Kernel)

Three pillars:

- **A**gglutinative — Kazakh morphology decomposes deterministically (root + typed suffixes); composition is rule-bound, not learned.
- **R**easoning — a curated knowledge graph ([`data/world_core/*.jsonl`](data/world_core)) + a forward-chaining reasoner (10 active rules) produces every fact-bearing claim. Every output cites a source.
- **K**ernel — system-runtime, not a probabilistic estimator. ARK has small trained components (selection-weights perceptron, suffix-chain priors, root-affinity PMI) but they sit inside the kernel as inspectable layers, not at the centre.

### v6.2 typed pipeline (`adam-algebra`)

```text
input ─▶ FST lattice ─▶ Composition ─▶ Frame ─▶ QueryIR ─▶ FrameIndex ─▶ Realiser ─▶ output
        (kernel-fst)    (operator +    (semantic  (frame    (typed       (Frame →
                         root +         record)    with a    retrieval)   Kazakh)
                         composition)              hole)
```

### v6.1 cascade (default, unchanged)

```text
input ─▶ parser ─▶ semantics ─▶ [ retrieval + compose ] ─▶ planner ─▶ realiser ─▶ FST synth ─▶ output
        (Layer 1) (Layer 2)       (Layer 2.5–2.75)       (Layer 3)   (Layer 4)   (Layer 5)
```

No transformer. No embeddings. No probabilistic generation. For any input, a developer can dump every layer's state and audit why the model chose what it said.

### Crates

| Layer | Crate | Role |
|---|---|---|
| **L0** | [`adam-kernel`](crates/adam-kernel) | Core identity + foundation contracts |
| **L0** | [`adam-kernel-fst`](crates/adam-kernel-fst) | FST morphology — phonology, morphotactics, synthesiser + parser, 25.5 k-entry Lexicon |
| **L0.5** | [`adam-algebra`](crates/adam-algebra) | **v6.2** — typed neurosymbolic stack: agglutinative algebra (Composition / Frame / QueryIR), FrameIndex retrieval, realiser, math_solver, system_clock, corpus_loader. Source of the 79-case real-Kazakh dialog battery + CI quality gate. |
| **L1** | [`adam-tokenizer`](crates/adam-tokenizer) | Pre-tokenizer + BPE trainer + encoder |
| **L1** | [`adam-corpus`](crates/adam-corpus) | Source acceptance, streaming processors, synthetic generator, `corpus_audit`, `morpheme_coverage` |
| **L1** | [`adam-eval`](crates/adam-eval) | Evaluation suite + benchmark reports |
| **L1** | [`adam-dialog`](crates/adam-dialog) | Dialog pipeline — 41 intents, multi-turn session + DST, template planner, slot-expanding realiser, voice output transducer. **v6.2:** `v6_2_router` integration bridge (env-gated). |
| **L1** | [`adam-retrieval`](crates/adam-retrieval) | Retrieval engine — morpheme inverted index, deterministic ranking, in-sample composition |
| **L1** | [`adam-reasoning`](crates/adam-reasoning) | Reasoning engine — typed-fact graph, 10 active forward-chaining rules, `extract_facts` / `build_lexical_graph` / `run_reasoner` binaries |
| **L1** | [`adam-scaling`](crates/adam-scaling) | Tier-by-tier scaling bench across the corpus |
| **L1** | [`adam-train`](crates/adam-train) | Legacy transformer baseline, preserved as regression reference |

Every layer outputs deterministic, regression-tested JSON artifacts. `bash ./scripts/validate_foundation.sh` runs the full foundation validation end-to-end. See [`docs/architecture_v3.md`](docs/architecture_v3.md) for the canonical v6.1 architecture reference and [`docs/v6_2_architectural_redesign.md`](docs/v6_2_architectural_redesign.md) for the v6.2 design.

## Demo — v6.2 live REPL

```
$ cargo run --release --example chat -p adam-algebra
=== adam ARK — live REPL ===
(deterministic; CPU-only; 0 MB model; no LLM)
Курс: 65 curated facts. Print a question on each line; type «exit» to quit.

? Ахмет Байтұрсынұлы қашан туылған?
> 1872     (25 417 ns)

? Двадцать пять умножь на 7 раздели на два прибавь три
> 90.5     (15 667 ns)

? Корень из шестнадцати
> 4        (5 334 ns)

? Что такое гравитация?
> сила притяжения масс       (14 458 ns)        # Russian — language-filtered

? Гравитация деген не?
> массалардың бір-бірін тарту күші   (12 208 ns) # Kazakh — language-filtered

? Бүгін айдың нешесі?
> 25       (live clock)

? Қазір қандай ай?
> мамыр    (live clock)
```

**Every answer is typed-data manipulation** — no template, no
free generation, no LLM call, no GPU. Bilingual disambiguation
works on same-root words («гравитация» appears in both KZ and RU
corpus with different definitions; `language_filter` resolves
which surfaces).

For a full evidence dump on any Kazakh root, run [`adam_inspect`](crates/adam-dialog/src/bin/adam_inspect.rs).

## What's measurable

| Metric | Value | Notes |
|---|---|---|
| Workspace tests | **1 904 passing** | default + `--features neural` builds both green; v6.2.0 adds 169 (algebra + router + integration); v5.x release-blocker invariants preserved |
| Release cadence | **540+ versioned releases since 2026-04-07** | every release CI-verified |
| v6.2 dialog battery | **79/79 must-pass, 0 gaps** | 14 real-Kazakh domains; CI quality gate via `dialog_battery_meets_quality_gate` |
| v6.2 warm p50 latency (release, M2 single core) | **791 ns** (full dialog) / **274 ns** (Stage 3) | vs LLM ~100ms — ≈ 126 000× faster |
| v6.2 throughput (single core) | **~1.26 M queries / sec** | scales linearly across cores |
| v6.2 model size | **0 MB** | pure typed-data manipulation |
| v6.1 cascade p50 turn latency | **~21 ms** | vs Llama-3 8B fp16 800–1500 ms; vs GPT-4 50–200 ms |
| Memory footprint | **~300 MB RSS** | both cascades; vs LLM 16+ GB VRAM |
| GPU usage | **0 %** | vs LLM dedicated GPU |
| Hallucination rate (curated-domain coverage) | **0 %** architectural | verified by graph admissibility tests + Stage 4 quality gate. Off-corpus queries fall through to v6.1 cascade or honest «нет данных» — they cannot invent facts, but they can misroute. Stage 8 (HumanDialogEval) measures the off-corpus surface |
| Lexicon roots | **25.5 k** | 13.6 k pure Kazakh + 11.9 k Apertium imports |
| Curated entries (`world_core/`) | **3 461 entries / 4 044 frames** | across 66 domains; `validate_world_core` enforced in CI |
| Curated facts total (incl. v6.1 augmentation) | **4 116** | `world_core` + bilingual aliases + МО РК + historical |
| Derived facts | **37 991** | from 10 forward-chaining rules over the curated graph |
| Dialog intents (v6.1 cascade) | **41** | template planner with `{slot\|features}` FST-aware syntax |

See [`docs/performance.md`](docs/performance.md) for the full performance report and [`docs/scaling_report.md`](docs/scaling_report.md) for the per-tier scaling bench.

## FAQ

**Is this a wrapper around an LLM?** No. There is no LLM, no neural network at the answer path, no API call to OpenAI / Anthropic / Google. v6.1 inference is FST + forward-chaining reasoner over a typed-fact graph; v6.2 inference is the typed Composition → Frame → QueryIR → FrameIndex → realiser pipeline.

**Is it really deterministic?** Yes. In v6.1, the only source of randomness is `planner::choose_template` (pin the seed → byte-identical output). In v6.2, the answer path is entirely pure-function: `FrameIndex.query(QueryIR)` returns frames in `(score desc, frame_id asc)` order, `realiser::realise(frame, focus, slot)` is a pure mapping; same input → byte-identical surface.

**What is "neurosymbolic" supposed to mean here?** Per the [v6.2 design doc](docs/v6_2_architectural_redesign.md): neural components — when they exist (Stage 6, not in v6.2.0) — produce **typed closed-set** outputs only (intent class, sense disambiguation candidates, retrieval ranker score). They never generate free text. The verifier owns truth and rejects any candidate without a `FrameIndex` provenance. v6.2.0 ships the **symbolic half** of the architecture (typed algebra + retrieval + realiser); Stage 6 will add learned closed-set components inside this scaffolding.

**Why Kazakh?** Kazakh's agglutinative morphology is exceptionally regular: every word decomposes into root + typed suffixes (case, number, tense, person, possessive, polarity, modality), each contributing a known operator. Composition is rule-bound, not learned. This is the cleanest substrate we know of for a deterministic AI runtime.

**Will it generalise to other languages?** The architecture is *designed* for it but not yet *demonstrated* on a second language. ~30 candidate agglutinative languages are catalogued in [MISSION.md](MISSION.md#agglutinative-languages--global-research-frontier); first port (Karakalpak or Kyrgyz) is on the v6 research roadmap with measured porting cost as a deliverable. Treat the multi-language story as a research goal, not a current product capability.

**What is the funding model?** Two parallel tracks: angel pre-seed private capital ($200K–300K target) and state research grants from agglutinative-language country research agencies (Japan JST/JSPS, South Korea NRF, Finland Academy of Finland, Turkey TÜBİTAK, Hungary NKFIH, Estonia ETAg, Uzbekistan, Kyrgyzstan, Mongolia, Tatarstan). See [COLLABORATION.md](COLLABORATION.md).

**Who built this?** [Daulet Baimurza](https://github.com/DauletBai), founder of Qazna Technologies. Solo development since 2026-04-07. Repository public since 2026-05-08. License: BUSL-1.1 (source-available; commercial use by permission).

**How do I cite this work?** See [CITATION.cff](CITATION.cff) and [codemeta.json](codemeta.json). GitHub renders the citation file as a "Cite this repository" button on the right sidebar.

## Recent releases

**v6.2.0 — Neurosymbolic agglutinative algebra (env-gated).** The
architectural redesign promised at v6.1.50. Lands the new
`adam-algebra` crate (8 modules, 195 tests) + `adam-dialog::v6_2_router`
integration bridge. v6.1 cascade unchanged when the gate is off.
Workspace tests 1735 → 1904. Real-Kazakh dialog battery 79/79
must-pass, 0 known gaps. Median warm latency 791 ns (release, M2
single core). See [CHANGELOG.md § 6.2.0](CHANGELOG.md).

**v6.1.50 — v6.1-series freeze.** Time-unit Count + Disagreement
answer-shape + voice aliases + doc refresh. Final v6.1.x release;
patch strategy reached its ceiling after 11 releases across two days.

**v6.1.0 — AnswerIR + predicate-aware retrieval (opt-in behind
`ADAM_ANSWER_IR=1`).** Typed `PredicateFocus` enum + 11 new typed
predicates (`BornIn` / `DiedIn` / `FoundedIn` / `EffectiveFrom` /
`Classifies` / `LocatedIn` / `Authored` / etc.) — closes the
2026-05-22 Codex audit relevance/completeness gap.

**v6.0.0 — Technical GA.** L5.5 TinyAgt neural composer preview
(opt-in), E1 discriminative intent classifier (95.95 % accuracy
at 600× lower latency than full cascade), E2 slot extractor,
Whisper.cpp voice input, KRU / Baitursynuly / AI-Law domain,
seven rounds of user-driven audit fixes.

**v5.0.0 – v5.4.0 — Voice, multimodal, World-Graph bridge data.**
OS-native TTS (macOS `Aru`, Linux `espeak-ng`), optional Piper
backend, Kazakh G2P. v5.4.0: 25+ dead-end abstract hubs closed,
derived facts +4 577, bare yes/no IsA route.

For full release history (540+ releases since 2026-04-07), see
[CHANGELOG.md](CHANGELOG.md). For the phase-by-phase roadmap,
see [`docs/roadmap.md`](docs/roadmap.md).

## Open to collaboration

We are open to collaboration in every direction:

- **Linguists** — agglutinative morphology, formal phonology, computational semantics
- **AI researchers** — deterministic / neurosymbolic alternatives to neural inference, formal verification of language models
- **Educational institutions** — pilot deployments with Kazakh-language students (current focus: Almaty / Astana schools)
- **National research agencies** — joint research grants from agglutinative-language country agencies
- **Government / defence** — offline-capable, auditable language AI for Kazakh and related languages; aligned with Kazakhstan's [AI Law of 18 January 2026](data/world_core/kru_baitursynov.jsonl) for defense-relevant high-risk categories
- **Investors** — angel pre-seed / seed stage who share the thesis that probabilistic AI is not the only path forward

Contact: **baimurza.daulet@gmail.com** · [LinkedIn](https://www.linkedin.com/in/daulet-baimurza-4b3506211)

See [COLLABORATION.md](COLLABORATION.md) for full per-class engagement terms.

## Repository layout

```
crates/                Rust workspace (11 crates, L0–L1)
  adam-algebra/        v6.2 typed neurosymbolic stack (algebra / Frame / QueryIR / FrameIndex / realiser / math_solver / system_clock / corpus_loader)
  adam-kernel-fst/     FST morphology — phonology, morphotactics, synthesiser + parser
  adam-dialog/         Dialog pipeline (v6.1 cascade) + v6_2_router (integration bridge)
  …                    (see Crates table above)
data/world_core/       Curated typed-fact graph (jsonl, by domain)
data/dialog/           Template repository + curriculum
data/retrieval/        Morpheme index + extracted facts + derived facts
data/eval/             Live holdouts + cognitive eval datasets
data/lexicon_v1/       Apertium-imported roots
data/tokenizer/        Curated segmentation roots
docs/                  Architecture, roadmap, performance, foundation policies
  v6_2_architectural_redesign.md  v6.2 design doc (signed off 2026-05-24)
scripts/               validate_foundation.sh + release tooling
```

See [`data/README.md`](data/README.md) for a top-level map of `data/`, and per-subdirectory READMEs for details.

## Foundation policies

[corpus](docs/corpus_policy.md) · [sources](docs/corpus_sources.md) · [curation](docs/curation_workflow.md) · [classification](docs/source_classification.md) · [scoring](docs/source_scoring.md) · [tokenizer](docs/tokenizer_policy.md) · [evaluation](docs/evaluation_policy.md) · [dialog architecture](docs/kazakh_grammar/07_dialog_architecture.md) · [Kazakh grammar reference](docs/kazakh_grammar/README.md)

## Out of scope

- **Probabilistic / LLM-style free generation** — every response is a curated fact retrieved via `FrameIndex` (v6.2) or a template realisation / verbatim corpus quote / rule derivation (v6.1). Nothing invented.
- **Multilingual input** — Russian queries work for terms whose surface differs from Kazakh (`«вода» / «скорость света» / «столица казахстана»`); same-root same-domain bilingual ambiguity is resolved via `language_filter`. Other languages are research-direction, not currently shipped.
- **Trained neural LM components in the answer path** — small ML lives inside the kernel as inspectable layers (selection weights, suffix priors, PMI, E1 intent classifier); no transformer, no embeddings, no free generation. Stage 6 (closed-set neural components) is design-only as of v6.2.0.
- **Cloud platform work** — adam runs as a single offline binary.

### Graph-First Policy

The graph layer of `adam` is **Rust-native and repository-native**. No external graph database as a required runtime; no Cypher / Gremlin / SPARQL query layer in the core pipeline; no Python graph stack hidden behind scripts. The canonical graph representation, traversal, and artifact builders live in Rust crates inside this repository. Shell scripts may orchestrate graph builds only as thin wrappers around `cargo run`.

## License

[Business Source License 1.1](LICENSE). Converts automatically to Apache License 2.0 on **2029-01-01**.

Non-commercial and research use is unrestricted today. Commercial use is permitted unless it competes directly with Qazna Technologies LLP products or services.

For commercial licensing inquiries: **baimurza.daulet@gmail.com**

Copyright © 2026 Qazna Technologies LLP.

---

<p align="center">
  <a href="MISSION.md">MISSION</a> ·
  <a href="docs/v6_2_architectural_redesign.md">v6.2 ARCH</a> ·
  <a href="RESEARCH.md">RESEARCH</a> ·
  <a href="COLLABORATION.md">COLLABORATION</a> ·
  <a href="AGENTS.md">AGENTS</a> ·
  <a href="CHANGELOG.md">CHANGELOG</a> ·
  <a href="docs/roadmap.md">roadmap</a> ·
  <a href="CITATION.cff">cite</a>
</p>
