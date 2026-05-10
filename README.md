<p align="center">
  <img src="assets/shanraq.svg" alt="adam logo" width="128" height="128">
</p>

<h1 align="center">adam</h1>

<p align="center">
  <i>Deterministic AI research — predictable, cheap, safe.</i><br>
  <i>Kazakh-first applied demonstrator of the Qazaq IR architecture.</i><br>
  <i>Қазақ тіліне арналған, толық болжамды диалог жүйесі — таза Rust тілінде.</i>
</p>

<p align="center">
  <a href="https://github.com/qazaq-ai/adam/releases"><img src="https://img.shields.io/badge/version-5.6.0-2EA44F?style=for-the-badge" alt="version"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-BUSL%201.1-orange?style=for-the-badge" alt="license"></a>
  <img src="https://img.shields.io/badge/language-Rust-CE412B?style=for-the-badge&logo=rust&logoColor=white" alt="rust">
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Linux-lightgrey?style=for-the-badge" alt="platform">
  <a href="https://github.com/qazaq-ai/adam/commits/main"><img src="https://img.shields.io/github/last-commit/qazaq-ai/adam?style=for-the-badge" alt="last commit"></a>
  <a href="https://github.com/qazaq-ai/adam/stargazers"><img src="https://img.shields.io/github/stars/qazaq-ai/adam?style=for-the-badge" alt="stars"></a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/tests-1155%20passing-2EA44F?style=flat-square" alt="tests">
  <img src="https://img.shields.io/badge/p50%20turn%20latency-21%20ms-2EA44F?style=flat-square" alt="latency">
  <img src="https://img.shields.io/badge/RSS-~160%20MB-2EA44F?style=flat-square" alt="rss">
  <img src="https://img.shields.io/badge/GPU-0%25-2EA44F?style=flat-square" alt="gpu">
  <img src="https://img.shields.io/badge/derived%20facts-35%20469-9CCC65?style=flat-square" alt="derived facts">
  <img src="https://img.shields.io/badge/world%20core-3265%20facts%20%2F%2054%20domains-9CCC65?style=flat-square" alt="world core">
  <img src="https://img.shields.io/badge/lexicon-25.5%20k%20roots-FBC02D?style=flat-square" alt="lexicon">
  <img src="https://img.shields.io/badge/intents-41-2EA44F?style=flat-square" alt="intents">
</p>

---

## 30-second pitch

> **adam is a deterministic AI research kernel** — the first applied demonstrator of an alternative to probabilistic large language models, built on the agglutinative morphology of the Kazakh language. Three guarantees by architecture: every output traces to a curated source (**predictability**), runs as a single binary on M2 8GB with 0% GPU and ~21 ms p50 latency (**cheapness**), and is **architecturally incapable of hallucination** because the runtime cannot emit a claim without a backing fact (**safety**). Generalises across ~30 catalogued agglutinative languages (Turkish, Korean, Japanese, Finnish, Hungarian, Tamil, Quechua, Swahili, …). Pure Rust. BUSL-1.1.

> **Reading order:** [MISSION.md](MISSION.md) (thesis) → [RESEARCH.md](RESEARCH.md) (open questions) → [COLLABORATION.md](COLLABORATION.md) (partner terms) → [AGENTS.md](AGENTS.md) (orientation for automated scouts) → [CHANGELOG.md](CHANGELOG.md) (full release history).

## Why deterministic?

Modern LLMs carry three structural problems we treat as **not inevitable**:

| The three diseases of probabilistic AI | adam's target |
|---|---|
| **Black box** — opaque internals, no source attribution, no auditable explanation for any specific output | **Predictability** — every claim traceable to a curated source; every reasoning step inspectable |
| **Resource cost** — billions of parameters, GPU clusters, datacentre inference, kilowatt-scale energy per query | **Cheapness** — single binary on M2 8GB, **0% GPU**, ~21 ms p50 latency, ~300 MB RSS |
| **Hallucination risk** — confident generation of plausible-sounding but factually wrong content, no internal mechanism to flag it | **Safety** — architectural impossibility: the runtime cannot emit a claim without a backing fact in `world_core` or a grounded reasoning chain |

**Hypothesis:** agglutinative languages — Kazakh in particular — exhibit unusually mathematical morphology. Every word decomposes into a root plus a predictable sequence of typed suffixes (case, number, tense, person, possessive, polarity, modality). Composition is **rule-bound, not learned**. That structure is the substrate for a deterministic runtime: FST morphology + typed suffix priors as deterministic prior layers, a curated knowledge graph as the only fact source, templates as the only path from fact to text. **No probabilistic free generation. No retrained-from-scratch behaviour per release.**

## Quick start

```bash
# Build the dialog REPL
cargo build --release -p adam-dialog --bin adam_chat

# Run interactively (auto-loads data/dialog/templates/v1.toml)
./target/release/adam_chat

# Single-shot
./target/release/adam_chat --once "Қасқыр — тірі ме?"

# Full Layer 1..5 trace per turn
./target/release/adam_chat --trace

# Voice output (macOS Aru / Linux espeak-ng / optional Piper)
./target/release/adam_chat --tts

# FST synthesiser + analyser CLI
cargo run --release -p adam-kernel-fst --bin adam_fst -- synth --root бала --plural --case dat
# → балаларға

# Full foundation validation (~30 s on M2)
bash ./scripts/validate_foundation.sh
```

## Architecture — ARK (Agglutinative Reasoning Kernel)

Three pillars:

- **A**gglutinative — Kazakh morphology decomposes deterministically (root + typed suffixes); composition is rule-bound, not learned.
- **R**easoning — a curated knowledge graph (`data/world_core/*.jsonl`) + a forward-chaining reasoner (10 active rules) produces every fact-bearing claim. Every output cites a source.
- **K**ernel — system-runtime, not a probabilistic estimator. ARK has small trained components (selection-weights perceptron, suffix-chain priors, root-affinity PMI) but they sit inside the kernel as inspectable layers, not at the centre.

```
input ─▶ parser ─▶ semantics ─▶ [ retrieval + compose ] ─▶ planner ─▶ realiser ─▶ FST synth ─▶ output
        (Layer 1) (Layer 2)       (Layer 2.5–2.75)       (Layer 3)   (Layer 4)   (Layer 5)
```

No transformer. No embeddings. No probabilistic generation. For any input, a developer can dump every layer's state and audit why the model chose what it said.

### Crates

| Layer | Crate | Role |
|---|---|---|
| **L0** | [`adam-kernel`](crates/adam-kernel) | Core identity + foundation contracts |
| **L0** | [`adam-kernel-fst`](crates/adam-kernel-fst) | FST morphology — phonology, morphotactics, synthesiser + parser, 25.5 k-entry Lexicon |
| **L1** | [`adam-tokenizer`](crates/adam-tokenizer) | Pre-tokenizer + BPE trainer + encoder |
| **L1** | [`adam-corpus`](crates/adam-corpus) | Source acceptance, streaming processors, synthetic generator, `corpus_audit`, `morpheme_coverage` |
| **L1** | [`adam-eval`](crates/adam-eval) | Evaluation suite + benchmark reports |
| **L1** | [`adam-dialog`](crates/adam-dialog) | Dialog pipeline — 41 intents, multi-turn session + DST, template planner, slot-expanding realiser, voice output transducer |
| **L1** | [`adam-retrieval`](crates/adam-retrieval) | Retrieval engine — morpheme inverted index, deterministic ranking, in-sample composition |
| **L1** | [`adam-reasoning`](crates/adam-reasoning) | Reasoning engine — typed-fact graph, 10 active forward-chaining rules, `extract_facts` / `build_lexical_graph` / `run_reasoner` binaries |
| **L1** | [`adam-scaling`](crates/adam-scaling) | Tier-by-tier scaling bench across the corpus |
| **L1** | [`adam-train`](crates/adam-train) | Legacy transformer baseline, preserved as regression reference |

Every layer outputs deterministic, regression-tested JSON artifacts. `bash ./scripts/validate_foundation.sh` runs the full foundation validation end-to-end. See [`docs/architecture_v3.md`](docs/architecture_v3.md) for the canonical architecture reference.

## Demo

```
$ cargo run --release -p adam-dialog --bin adam_chat
adam-chat: loaded 114 template families
adam-chat: reasoning on — 35 469 derived facts (3 265 supporting extracted facts)

> Қасқыр — тірі ме?                          # bare yes/no IsA, v5.4.0
Қасқыр — тірі. Бұл қасқыр → тірі тізбегі арқылы расталады.

> жер туралы айтшы                            # reasoning probe
жер туралы мынадай байланыс анықтадым:
қорытынды: жер — аспан денесі (байланысты ой-тізбек арқылы)
# R1_is_a_transitivity: (жер IsA ғаламшар) ∧ (ғаламшар IsA аспан денесі)

> Абай жайында не дейсің                      # retrieval fallback (verbatim quote)
абай жайында осындай мысал бар:
«Абай Құнанбайұлы (10 тамыз 1845 — 6 шілде 1904)»
# byte-identical quote from wikipedia_kz_pack.json / wiki_kz_0000190

> менің атым Дәулет                           # entity extraction → session.name
қош келдіңіз Дәулет

> сау бол
сау бол
```

**Every line above is traceable** to one of: a template realisation, a verbatim corpus quote with `(pack, sample_id)` provenance, an FST-synthesised slot fill, a rule-derived chain with `rule_id` + non-empty `source_chain`, or a curated World Core fact with a named reviewer. Nothing else can leave the system. Zero free-form generation, zero LLM calls, zero GPU.

For a full evidence dump on any Kazakh root, run [`adam_inspect`](crates/adam-dialog/src/bin/adam_inspect.rs).

## What's measurable

| Metric | Value | Notes |
|---|---|---|
| Workspace tests | **1 155 passing** | + 17 ignored slow integration; CI on every release |
| Release cadence | **461+ versioned releases in 1 month** | every release CI-verified |
| p50 turn latency | **21 ms** | vs Llama-3 8B fp16 800–1500 ms; vs GPT-4 50–200 ms |
| Memory footprint | **~300 MB RSS** | vs LLM 16+ GB VRAM |
| GPU usage | **0%** | vs LLM dedicated GPU |
| Hallucination rate | **0%** (architectural) | verified by graph admissibility tests |
| Lexicon roots | **25.5 k** | 13.6 k pure Kazakh + 11.9 k Apertium imports |
| Curated facts | **3 265** | across 54 world_core domains |
| Derived facts | **35 469** | from 10 forward-chaining rules over the curated graph |
| Dialog intents | **41** | template planner with `{slot\|features}` FST-aware syntax |

See [`docs/performance.md`](docs/performance.md) for the full performance report and [`docs/scaling_report.md`](docs/scaling_report.md) for the per-tier scaling bench.

## FAQ

**Is this a wrapper around an LLM?** No. There is no LLM, no neural network at the core, no API call to OpenAI / Anthropic / Google. Inference happens via a finite-state transducer + a forward-chaining reasoner over a typed-fact graph.

**Is it really deterministic?** Yes. The only source of randomness is `planner::choose_template`, which picks uniformly from ≤ 5 applicable templates given a `(rng_seed, intent)` pair. Pin the seed, pin the input, pin the world_core — output is bit-identical.

**Why Kazakh?** Kazakh's agglutinative morphology is exceptionally regular: every word decomposes into root + typed suffixes (case, number, tense, person, possessive, polarity, modality), each contributing a known operator. Composition is rule-bound, not learned. This is the cleanest substrate we know of for a deterministic AI runtime.

**Will it generalise to other languages?** The architecture is designed for it. ~30 candidate agglutinative languages catalogued in [MISSION.md](MISSION.md#agglutinative-languages--global-research-frontier). Closest port: Karakalpak (~1–2 weeks of Lexicon adaptation estimated). Furthest: Quechua, Swahili (different phonology assumptions; full architectural validation needed).

**What is the funding model?** Two parallel tracks: angel pre-seed private capital ($200K–300K target) and state research grants from agglutinative-language country research agencies (Japan JST/JSPS, South Korea NRF, Finland Academy of Finland, Turkey TÜBİTAK, Hungary NKFIH, Estonia ETAg, Uzbekistan, Kyrgyzstan, Mongolia, Tatarstan). See [COLLABORATION.md](COLLABORATION.md).

**Who built this?** [Daulet Baimurza](https://github.com/DauletBai), founder of Qazna Technologies. Solo development since 2026-04-07. Repository public since 2026-05-08. License: BUSL-1.1 (source-available; commercial use by permission).

**How do I cite this work?** See [CITATION.cff](CITATION.cff) and [codemeta.json](codemeta.json). GitHub renders the citation file as a "Cite this repository" button on the right sidebar.

## Recent releases

**v5.4.0 — Bridge data + bare yes/no IsA route.** First user-facing payoff of the v5.3.x repositioning arc. World-graph bridge data closes 25+ dead-end abstract hubs (`тіршілік иесі`, `тірі ағза`, `құрал`, `тағам`, `қазақ ханы`, …): derived facts **30 892 → 35 469 (+4 577)** at 228 derivations per bridge fact. Bare «<X> — <Y> ме?» pattern now routes through `find_isa_chain` over extracted + derived facts; «Қасқыр — тірі ме?» went from a tangential «Қасқыр — жыртқыш» to «Қасқыр — тірі. Бұл қасқыр → тірі тізбегі арқылы расталады.» Live dialog test of 8 cases at 100 % pass.

**v5.3.6 – v5.3.10 — Repositioning arc.** [MISSION.md](MISSION.md) (research thesis), [RESEARCH.md](RESEARCH.md) (open questions + milestones), [COLLABORATION.md](COLLABORATION.md) (partner-class onboarding), [AGENTS.md](AGENTS.md) (orientation for AI-scout discovery), [CITATION.cff](CITATION.cff) + [codemeta.json](codemeta.json) (academic citation + structured data).

**v5.0.0 – v5.2.0 — Voice + multimodal.** OS-native TTS transducer (macOS `Aru`, Linux `espeak-ng`), optional Piper backend, Kazakh G2P module — substrate for kernel-pure concatenative speech.

For full release history (461 releases since 2026-04-07), see [CHANGELOG.md](CHANGELOG.md). For the phase-by-phase roadmap, see [`docs/roadmap.md`](docs/roadmap.md).

## Open to collaboration

We are open to collaboration in every direction:

- **Linguists** — agglutinative morphology, formal phonology, computational semantics
- **AI researchers** — deterministic alternatives to neural inference, formal verification of language models
- **Educational institutions** — pilot deployments with Kazakh-language students (current focus: Almaty / Astana schools)
- **National research agencies** — joint research grants from agglutinative-language country agencies
- **Government / defence** — offline-capable, auditable language AI for Kazakh and related languages
- **Investors** — angel pre-seed / seed stage who share the thesis that probabilistic AI is not the only path forward

Contact: **baimurza.daulet@gmail.com** · [LinkedIn](https://www.linkedin.com/in/daulet-baimurza-4b3506211)

See [COLLABORATION.md](COLLABORATION.md) for full per-class engagement terms.

## Repository layout

```
crates/                Rust workspace (10 crates, L0–L1)
data/world_core/       Curated typed-fact graph (jsonl, by domain)
data/dialog/           Template repository + curriculum
data/retrieval/        Morpheme index + extracted facts + derived facts
data/eval/             Live holdouts + cognitive eval datasets
data/lexicon_v1/       Apertium-imported roots
data/tokenizer/        Curated segmentation roots
docs/                  Architecture, roadmap, performance, foundation policies
scripts/               validate_foundation.sh + release tooling
```

See [`data/README.md`](data/README.md) for a top-level map of `data/`, and per-subdirectory READMEs for details.

## Foundation policies

[corpus](docs/corpus_policy.md) · [sources](docs/corpus_sources.md) · [curation](docs/curation_workflow.md) · [classification](docs/source_classification.md) · [scoring](docs/source_scoring.md) · [tokenizer](docs/tokenizer_policy.md) · [evaluation](docs/evaluation_policy.md) · [dialog architecture](docs/kazakh_grammar/07_dialog_architecture.md) · [Kazakh grammar reference](docs/kazakh_grammar/README.md)

## Out of scope

- **Multilingual input and output** — adam accepts and produces only Kazakh. Generalisation comes via the retrieval engine over the 77.9 M-word Kazakh corpus, not translation.
- **Probabilistic / LLM-style free generation** — every response is a template realisation, a verbatim corpus quote, or a rule derivation over typed facts with a full `source_chain`. Nothing invented.
- **Trained neural LM components in the answer path** — small ML lives inside the kernel as inspectable layers (selection weights, suffix priors, PMI); no transformer, no embeddings.
- **Cloud platform work** — adam runs as a single offline binary.

### Graph-First Policy

The graph layer of `adam` is **Rust-native and repository-native**. No external graph database as a required runtime; no Cypher / Gremlin / SPARQL query layer in the core pipeline; no Python graph stack hidden behind scripts. The canonical graph representation, traversal, and artifact builders live in Rust crates inside this repository. Shell scripts may orchestrate graph builds only as thin wrappers around `cargo run`.

## License

[Business Source License 1.1](LICENSE). Converts automatically to Apache License 2.0 on **2029-01-01**.

Non-commercial and research use is unrestricted today. Commercial use is permitted unless it competes directly with Qazna Technologies LLP products or services.

For commercial licensing inquiries: **hello@qazaq.ai**

Copyright © 2026 Qazna Technologies LLP.

---

<p align="center">
  <a href="MISSION.md">MISSION</a> ·
  <a href="RESEARCH.md">RESEARCH</a> ·
  <a href="COLLABORATION.md">COLLABORATION</a> ·
  <a href="AGENTS.md">AGENTS</a> ·
  <a href="CHANGELOG.md">CHANGELOG</a> ·
  <a href="docs/roadmap.md">roadmap</a> ·
  <a href="CITATION.cff">cite</a>
</p>
