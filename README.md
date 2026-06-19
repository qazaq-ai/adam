<p align="center">
  <img src="assets/shanraq.svg" alt="adam logo" width="128" height="128">
</p>

<h1 align="center">adam</h1>

<p align="center">
  <i>Deterministic Kazakh-first AI kernel — typed, offline, watch-class fast.</i><br>
  <i>An applied demonstrator of neurosymbolic agglutinative reasoning.</i><br>
  <i>Қазақ тіліне арналған, толық болжамды диалог жүйесі — таза Rust тілінде.</i>
</p>

<p align="center">
  <a href="https://github.com/qazaq-ai/adam/releases"><img src="https://img.shields.io/badge/version-6.8.0-2EA44F?style=for-the-badge" alt="version"></a>
  <a href="https://github.com/qazaq-ai/adam/actions/workflows/rust.yml"><img src="https://img.shields.io/github/actions/workflow/status/qazaq-ai/adam/rust.yml?branch=main&style=for-the-badge&label=CI" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-BUSL%201.1-orange?style=for-the-badge" alt="license"></a>
  <img src="https://img.shields.io/badge/language-Rust-CE412B?style=for-the-badge&logo=rust&logoColor=white" alt="rust">
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Linux-lightgrey?style=for-the-badge" alt="platform">
  <a href="https://github.com/qazaq-ai/adam/stargazers"><img src="https://img.shields.io/github/stars/qazaq-ai/adam?style=for-the-badge" alt="stars"></a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/school%20program%20eval-159%2F159-2EA44F?style=flat-square" alt="school program 100%">
  <img src="https://img.shields.io/badge/conv%20dialog%20eval-52%2F52-2EA44F?style=flat-square" alt="conv dialog 100%">
  <img src="https://img.shields.io/badge/safety%20eval-20%2F20-2EA44F?style=flat-square" alt="safety 100%">
  <img src="https://img.shields.io/badge/v6.7%20real%20audit-25%2F26-2EA44F?style=flat-square" alt="real audit 96%">
  <img src="https://img.shields.io/badge/speech%20defect%20baseline-37%2F71-FBC02D?style=flat-square" alt="speech defect baseline 52%">
  <img src="https://img.shields.io/badge/production%20p50-13.6%20ms-2EA44F?style=flat-square" alt="production p50">
  <img src="https://img.shields.io/badge/peak%20RSS-314%20MB-2EA44F?style=flat-square" alt="peak RSS">
  <img src="https://img.shields.io/badge/0%20GPU%20%2F%200%20network-2EA44F?style=flat-square" alt="0 GPU / 0 network">
  <img src="https://img.shields.io/badge/world%20core-3444%20curated%20/%204116%20facts-9CCC65?style=flat-square" alt="world core">
  <img src="https://img.shields.io/badge/intents-41%20router-2EA44F?style=flat-square" alt="intents">
  <img src="https://img.shields.io/badge/hallucinations%20within%20curated%20domains-0-2EA44F?style=flat-square" alt="hallucinations within curated domains">
</p>

<p align="center">
  <b><a href="docs/MANIFESTO.md">Why</a></b> ·
  <b><a href="DUE_DILIGENCE.md">Due diligence</a></b> ·
  <b><a href="docs/COMPARISON.md">vs LLMs</a></b> ·
  <b><a href="CHANGELOG.md">Changelog</a></b> ·
  <b><a href="COLLABORATION.md">Collaborate</a></b>
</p>

---

## TL;DR

**adam is a deterministic Kazakh language AI kernel that fits on a watch.** Every reply traces to a curated source via a typed pipeline (Composition → Frame → QueryIR → FrameIndex → realiser). 314 MB peak RSS. 0 GPU. 0 network. 13.6 ms median production response. **0 hallucinations within curated-domain coverage** by construction — there is no free-text generator in the answer path.

Built on the mathematically regular morphology of Kazakh. Designed to extend across ~30 catalogued agglutinative languages. Pure Rust. BUSL-1.1.

```bash
# Try it — 60 seconds, no GPU, no network:
cargo build --release --bin adam_chat
ADAM_V6_2=1 ./target/release/adam_chat
# Қазақстанның астанасы қандай?
# > Қазақстанның елордасы — Астана қаласы (1997 жылдан бастап).
```

---

## What's new in v6.8.0

**Production hybrid stack + five-suite eval infrastructure.** Consolidates the v6.6 generative pivot and v6.7 staged training breakthrough into a single shipped release with verifiable trust signals. Five production eval suites measured against the full `respond_full` cascade (`ADAM_V6_2=1`):

| Suite | Accepted | Probes | Score |
|---|---:|---:|---:|
| `school_program_eval` (14 subjects) | 159 | 0 | **100 %** |
| `conv_dialog_eval` (real voice REPL + scripted) | 52 | 31 | **100 %** |
| `safety_eval` (13 categories, Codex release gate) | 20 | 32 | **100 %** |
| `v6_7_real_audit_eval` | 26 | 0 | **96 %** |
| `speech_defect_eval` (8 defect categories) | 71 | 9 | 52 % honest baseline |

The headline shift is **stack measurement honesty**: the LM-only `respond` binary reports the v6.7 generative head ceiling alone (school 78 %); the `respond_full` binary runs the full deterministic cascade + world_core domain index (school **100 %**). All five suites are measured against `respond_full` — no LM-only scoring in the dashboard.

v6.8 also closes three production routing gaps (global `red_flags::detect` hoist at `Conversation::turn()`, weapon / harm-other marker expansion, past-tense math verbs), surfaced by the new suites and patched in [c67b37b3](https://github.com/qazaq-ai/adam/commit/c67b37b3).

Full release history: [CHANGELOG.md](CHANGELOG.md).

---

## Two product directions

The strategic focus for the post-v6.8 arc is **not** a Kazakh ChatGPT clone — adam's architecture wins where LLMs structurally lose:

### 1. Kazakh school tutor

Curriculum-verified answers, offline, 0 hallucinations within the curated scope. The `school_program_eval` suite (14 subjects × 159 cases at 100 % semantic) is the trust foundation; the curriculum SQLite DB at [`data/curriculum/curriculum.db`](data/curriculum/curriculum.db) holds the productionised Q&A pipeline.

### 2. Industrial knowledge assistant (read-only, voice-first)

Target verticals: SOPs / охрана труда / training manuals / maintenance / incident reporting, with a voice interface for hands-occupied environments. Adam's deterministic core + auditable provenance + Kazakh-native morphology + 314 MB RSS + offline mode form a moat where general LLMs cannot follow:

| Property | adam | General LLM |
|---|---|---|
| Per-answer traceability | ✓ ProofObject + world_core | ✗ |
| Offline operation | ✓ 314 MB RSS · 0 GPU · 0 network | ✗ |
| Byte-deterministic | ✓ rule-based core | ✗ |
| Low-end hardware (M2 8 GB) | ✓ | ✗ |
| Native Kazakh | ✓ FST + 3 444 facts | weak |
| Closed-set safety control | ✓ red_flags + safety_guard | hard |

---

## Quick start

```bash
# Build the production binaries (5–10 min on M2 Air):
cargo build --release --bin adam_chat --bin voice_repl_v6_3 --bin respond_full

# 1) Text REPL — production dialog cascade:
ADAM_V6_2=1 ./target/release/adam_chat

# 2) Voice REPL — Whisper STT → cascade → Piper TTS:
ADAM_V6_2=1 ./target/release/voice_repl_v6_3 --loop-mode --mode respond

# 3) Run any production eval suite:
./target/release/respond_full data/eval/safety_eval.json
./target/release/respond_full data/eval/school_program_eval.json
./target/release/respond_full data/eval/conv_dialog_eval.json

# 4) Full foundation validation (~30 s):
bash ./scripts/validate_foundation.sh
```

The voice loop auto-downloads no models. If `data/stt_models/ggml-shirali-kz.bin` (Shirali Kazakh-fine-tuned Whisper, 465 MB) is missing, voice REPL falls back to the in-tree DTW phoneme STT.

---

## Architecture

```text
                          ┌───────── voice REPL (optional) ─────────┐
                          │                                          │
mic ──cpal──▶ Whisper STT ──▶ fuzzy + LM rescore ──▶ intent classifier ─┐
                          │                                          │ │
                          └──── 100 % deterministic dialog core ─────┘ │
                                                                      ▼
                  ┌─── adam-kernel-fst ──────────────────────────────────┐
input (text) ────▶│ FST morphology · 25 k-root Kazakh Lexicon            │
                  └─── ▼ Analysis ───────────────────────────────────────┘
                  ┌─── adam-algebra ─────────────────────────────────────┐
                  │ Composition (root + typed suffix chain)              │
                  │ ─▶ Frame (agent · predicate · object · modifiers)    │
                  │ ─▶ QueryIR (frame with a hole + focus + shape)       │
                  └─── ▼ ────────────────────────────────────────────────┘
                  ┌─── adam-dialog::v6_2_router ─────────────────────────┐
                  │ math_solver · system_clock · FrameIndex · realiser   │
                  │  ↑                                                    │
                  │  └── world_core (3 444 curated facts · 65 domains)   │
                  └─── ▼ ────────────────────────────────────────────────┘
                                                                      │
output (Kazakh sentence) ────────────────────────────────────────────  ┘
                                                                      ▼
                  ┌─── voice loop: Piper TTS (Kazakh ISSAI voice) ───────┐
                  └──────────────────────────────────────────────────────┘
```

**No transformer in the answer path. No embeddings. No probabilistic free generation.** For any input, a developer can dump every layer's state and audit why the kernel chose what it said. Neural components (Whisper STT, Piper TTS, ~1 M-param BPE LM rescorer, ~1 M-param intent classifier) live exclusively at the speech surface around the deterministic core; they never invent facts.

Three latency classes (release build, M2 Air):

| Class | What it measures | Median | p95 | RSS |
|---|---|---:|---:|---:|
| **Typed kernel micro-path** | algebra-only Frame → Index → answer | ~470 ns | ~600 ns | < 50 MB |
| **Production dialog cascade** | 30-query battery, morph → router → retrieval → reasoning → realiser | **13.6 ms** | 19.6 ms | **314 MB** |
| **Full voice loop** | mic → Whisper → cascade → Piper | interactive | — | + 2.3 GB STT + 1.1 GB TTS on disk |

---

## Why neurosymbolic, not LLM

Modern LLMs carry three structural problems we treat as **not inevitable**. adam's neurosymbolic stack means neural components — where they exist — produce **typed closed-set** outputs only (intent class, sense disambiguation candidates, retrieval ranker score). A deterministic verifier owns truth.

| LLM disease | adam's target | How it is enforced |
|---|---|---|
| **Black box** — opaque internals, no source attribution | **Predictability** — every claim traceable | `FrameIndex.query` returns a `RankedFrame` with `FrameId`; `data/world_core/*.jsonl` is the only fact source |
| **Resource cost** — billions of params, GPU clusters | **Cheapness** — single binary | 314 MB RSS · 0 % GPU · 0 network · 13.6 ms p50 on M2 Air |
| **Hallucination risk** — confident plausible-sounding wrongness | **Architectural impossibility within curated coverage** | Realiser is a typed pure function over a curated Frame; index miss → honest «нет данных», never invention |

**Hypothesis:** agglutinative languages — Kazakh in particular — exhibit unusually mathematical morphology. Every word decomposes into a root + a typed suffix chain (case, number, tense, person, possessive, polarity, modality). Composition is **rule-bound**, not learned. That structure becomes the substrate for a typed runtime.

---

## How adam compares to LLMs

KazMMLU ([arXiv:2502.12829](https://arxiv.org/abs/2502.12829), 23 k Kazakh + Russian MCQ across STEM / humanities / social science) is the most-cited Kazakh academic benchmark. adam has not yet been run on KazMMLU itself (that's a v7 deliverable); the table compares published scores and the resource envelope:

| Model | KazMMLU | Resource cost |
|---|---|---|
| **adam v6.8.0** | own 5-suite production dashboard (`school_program` 159 / 159 · `conv_dialog` 52 / 52 · `safety` 20 / 20 · `real_audit` 24 / 26 · `speech_defect` 37 / 71 honest baseline) | **314 MB RSS · 0 GPU · $0** |
| GPT-4o | 76.6 % | API only · $2.50 / $10 per 1 M tok |
| DeepSeek V3 | 76.9 % | self-host or API |
| Llama 3.1 70B | 56.2 % | ~140 GB FP16 · 2–4× H100 |
| Llama 3.1 8B | 39.7 % | ~16 GB FP16 · 1× A100 |
| Claude (3.5 / 3.7 / 4) | not published per-language | API only · $3 / $15 per 1 M tok |

See [`docs/COMPARISON.md`](docs/COMPARISON.md) for the full table (hallucination rate, FLORES coverage, Sherkala-tuned Llama) plus honest gaps.

---

## What's measurable

| Metric | Value |
|---|---|
| Workspace test files | 274 (across 28 crates + 10 tools; run `cargo test --release --workspace --locked` for live counts) |
| `school_program_eval` | **159 / 159 = 100 %** semantic (14 subjects via `respond_full`) |
| `conv_dialog_eval` | **52 / 52 = 100 %** semantic (44 real voice REPL turns + 39 scripted; 31 probes) |
| `safety_eval` | **20 / 20 = 100 %** semantic (13 categories, Codex release gate; 32 probes) |
| `v6_7_real_audit_eval` | 25 / 26 = 96 % semantic |
| `speech_defect_eval` | 37 / 71 = 52 % semantic (honest baseline; 8 defect categories) |
| v6.2 dialog battery | 79 / 79 must-pass, 0 gaps (CI quality gate `dialog_battery_meets_quality_gate`) |
| Production cascade latency (M2 Air, 30-query battery) | **13.6 ms p50** · 19.6 ms p95 · **314 MB peak RSS** |
| Algebra micro-path latency | ~470 ns avg · ~600 ns p95 (algebra core only — not a fair head-to-head against full NLP) |
| Throughput (single core) | ~1.26 M queries / sec |
| Model size | **0 MB** (pure typed-data manipulation) |
| GPU usage | **0 %** |
| Hallucination rate (curated-domain coverage) | **0 %** architectural |
| Lexicon roots | 25 525 (13 606 pure Kazakh + 11 919 Apertium imports) |
| Curated `world_core/` | 3 444 entries · 4 116 facts · 65 domains |
| Derived facts | 37 991 from 10 forward-chaining rules |
| Curriculum DB | 91 chemistry Q&A across 8 topics (`data/curriculum/curriculum.db`) — school-tutor foundation |

See [`docs/performance.md`](docs/performance.md) for the full performance report.

---

## Repository layout

```
crates/                Rust workspace (28 crates)
  adam-algebra/        Typed neurosymbolic stack (Composition / Frame / QueryIR / FrameIndex / realiser / math_solver / system_clock / corpus_loader)
  adam-dialog/         Dialog pipeline + v6_2_router + wellness::pain_support + safety_guard + red_flags
  adam-kernel-fst/     FST morphology — phonology, morphotactics, synthesiser + parser, 25 k-root Lexicon
  adam-reasoning/      Typed-fact graph + 10 forward-chaining rules
  adam-retrieval/      Morpheme inverted index + deterministic ranking
  adam-eval/           Evaluation suite + rust_only_contracts gate
  …                    (24 more — see Cargo.toml workspace.members)
tools/                 Binaries (10) — voice_repl_v6_3, intent_dataset, build_human_bank, …
data/world_core/       65 .jsonl files · 3 444 typed-fact entries · CI-validated
data/eval/             Five production suites + legacy holdouts
data/curated/          Training packs (small in git; > 30 MB derived corpora gitignored)
data/curriculum/       SQLite curriculum DB (school tutor foundation)
docs/                  Architecture, roadmap, performance, foundation policies
scripts/               validate_foundation.sh + Python builders (off-runtime, gated)
```

See [`data/README.md`](data/README.md) for a top-level map of `data/`.

---

## FAQ

**Is this a wrapper around an LLM?** No. No LLM, no neural network in the answer path, no API call to OpenAI / Anthropic / Google. Inference is FST + forward-chaining reasoner + typed Composition → Frame → QueryIR → FrameIndex → realiser.

**Is it really deterministic?** Yes. The answer path is pure-function: `FrameIndex.query(QueryIR)` returns frames in `(score desc, frame_id asc)` order, `realiser::realise(frame, focus, slot)` is a pure mapping. Same `(input, seed, world_core)` → byte-identical surface.

**Why Kazakh?** Kazakh's agglutinative morphology is exceptionally regular — every word decomposes into root + typed suffixes, each contributing a known operator. Composition is rule-bound. This is the cleanest substrate we know of for a deterministic AI runtime.

**Will it generalise to other languages?** The architecture is *designed* for it but not yet *demonstrated* on a second language. ~30 candidate agglutinative languages are catalogued in [MISSION.md](MISSION.md). First port (Karakalpak or Kyrgyz) is on the v7 research roadmap with measured porting cost as a deliverable.

**Who built this?** [Daulet Baimurza](https://github.com/DauletBai), founder of Qazna Technologies. Solo development since 2026-04-07. Repository public since 2026-05-08.

**How do I cite this work?** See [CITATION.cff](CITATION.cff) and [codemeta.json](codemeta.json). GitHub renders the citation file as a «Cite this repository» button.

---

## Open to collaboration

- **Linguists** — agglutinative morphology, formal phonology, computational semantics
- **AI researchers** — deterministic / neurosymbolic alternatives, formal verification of language systems
- **Educational institutions** — pilot deployments with Kazakh-language students (current focus: Almaty / Astana / Kostanay schools)
- **Industrial partners** — read-only SOP / safety / training / maintenance assistants for production environments
- **National research agencies** — joint grants from agglutinative-language country agencies (Japan JST/JSPS, South Korea NRF, Finland Academy of Finland, Turkey TÜBİTAK, Hungary NKFIH, Estonia ETAg)
- **Investors** — angel pre-seed / seed who share the thesis that probabilistic AI is not the only path forward

Contact: **baimurza.daulet@gmail.com** · [LinkedIn](https://www.linkedin.com/in/daulet-baimurza-4b3506211)

See [COLLABORATION.md](COLLABORATION.md) for per-class engagement terms.

---

## License

[Business Source License 1.1](LICENSE). Converts automatically to Apache License 2.0 on **2029-01-01**.

Non-commercial and research use is unrestricted today. Commercial use is permitted unless it competes directly with Qazna Technologies LLP products or services. For commercial licensing inquiries: **baimurza.daulet@gmail.com**.

Copyright © 2026 Qazna Technologies LLP.

---

<p align="center">
  <a href="MISSION.md">MISSION</a> ·
  <a href="docs/v6_2_architectural_redesign.md">ARCHITECTURE</a> ·
  <a href="RESEARCH.md">RESEARCH</a> ·
  <a href="COLLABORATION.md">COLLABORATION</a> ·
  <a href="DUE_DILIGENCE.md">DUE DILIGENCE</a> ·
  <a href="CHANGELOG.md">CHANGELOG</a> ·
  <a href="docs/roadmap_v6_v7.md">ROADMAP</a> ·
  <a href="CITATION.cff">CITE</a>
</p>
