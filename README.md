<p align="center">
  <img src="assets/shanraq.svg" alt="adam logo" width="160" height="160">
</p>

<h1 align="center">adam</h1>

<p align="center">
  <i>Predictable Kazakh-first dialog, built in pure Rust.</i><br>
  <i>Қазақ тіліне арналған, толық болжамды диалог жүйесі — таза Rust тілінде.</i>
</p>

<p align="center">
  <a href="https://github.com/qazaq-ai/adam/releases"><img src="https://img.shields.io/badge/version-1.4.5-2EA44F?style=for-the-badge" alt="version"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-BUSL%201.1-orange?style=for-the-badge" alt="license"></a>
  <img src="https://img.shields.io/badge/language-Rust-CE412B?style=for-the-badge&logo=rust&logoColor=white" alt="rust">
  <img src="https://img.shields.io/badge/script-Cyrillic-8338EC?style=for-the-badge" alt="cyrillic">
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Linux-lightgrey?style=for-the-badge" alt="platform">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/intents-26-2EA44F?style=flat-square" alt="intents">
  <img src="https://img.shields.io/badge/surface-Kazakh--only-9CCC65?style=flat-square" alt="Kazakh only">
  <img src="https://img.shields.io/badge/lexicon-14%20k%20roots-FBC02D?style=flat-square" alt="lexicon">
  <img src="https://img.shields.io/badge/templates-31%20families-FBC02D?style=flat-square" alt="templates">
  <img src="https://img.shields.io/badge/tests-288%20passing-2EA44F?style=flat-square" alt="tests">
  <img src="https://img.shields.io/badge/hallucinations-0-2EA44F?style=flat-square" alt="hallucinations">
</p>

---

## What is adam?

`adam` is a **predictable, auditable Kazakh dialog system**, built **entirely in Rust**. Every output is produced by a five-layer pipeline you can trace end-to-end:

```
  input ─▶ parser ─▶ semantics ─▶ planner ─▶ realiser ─▶ FST synth ─▶ output
          (Layer 1) (Layer 2)   (Layer 3)   (Layer 4)   (Layer 5)
```

No transformer. No embeddings. No probabilistic generation. For any input, a developer can dump every layer's state and audit why the model chose what it said. The only source of non-determinism is a seed-modulo pick across ≤ 5 interchangeable response templates per intent.

The name *adam* (Kazakh: **адам**) means "human".

**Design principles:**

- **Predictable** — every stage is deterministic or samples from a finite, inspectable set.
- **Auditable** — `adam_chat --trace` dumps every layer per turn.
- **Grammatically correct by construction on the slot path** — `{slot|features}` placeholders go through the FST synthesiser, so no morphologically invalid inflected form can leave the system. Literal template text (e.g. `"сәлем"`, `"қайырлы таң"`) is pre-verified Kazakh in `data/dialog/templates/v1.toml`, audited offline.
- **Small** — runs on a MacBook Air M2 8 GB. No GPU.
- **Kazakh-native** — built on a 14 k-entry curated pre-modern Kazakh Lexicon (no Russian-era loanwords), not translated from English.

## Demo

```
$ cargo run --release -p adam-dialog --bin adam_chat
adam-chat v1.1.0 — пікірлесейік! Қазақ тілінде сөйлесейік; ^D to quit.

> сәлем
сәлем
> менің атым Дәулет
Дәулетпен танысқаныма қуаныштымын
> мен Алматыданмын
Алматы — әдемі қала
> мен бағдарламашымын
бағдарламашыға сәттілік тілеймін         # new modern profession via Lexicon + FST dative
> мен әзірлеушімін
Дәулет, әзірлеуші — құрметті кәсіп
> қалайсыз
жақсы, ал сіз қалайсыз
> кітап оқимын
кітап — қызықты тақырып                  # smart Unknown handler: noun-hint acknowledgement
> сен ақымақсың
сізге ренжімеймін                        # Insult intent: polite non-engagement
> сау бол
сау болыңыз
```

Kazakh-only input and output. Entities (name, age, city, occupation) persist across turns. Personalised templates fire when the session has the right slots filled. When no intent matches, the parser extracts a noun hint so the fallback response can acknowledge the topic instead of blank "түсінбедім".

## Architecture

Seven Rust crates, three layers:

| Layer | Crate | Role |
|---|---|---|
| **L0** | [`adam-kernel`](crates/adam-kernel) | Core identity + foundation contracts |
| **L0** | [`adam-kernel-fst`](crates/adam-kernel-fst) | **FST morphology** — phonology (12 archiphonemes, 22+ twol rules), morphotactics (30 suffix templates), synthesiser + parser, 14 k-entry Lexicon |
| **L1** | [`adam-tokenizer`](crates/adam-tokenizer) | Pre-tokenizer + BPE trainer + encoder |
| **L1** | [`adam-corpus`](crates/adam-corpus) | Source acceptance + synthetic sentence generation |
| **L1** | [`adam-eval`](crates/adam-eval) | Evaluation suite + benchmark reports |
| **L1** | [`adam-dialog`](crates/adam-dialog) | **Dialog pipeline** — intent recognisers, session state, template planner, slot-expanding realiser |
| **L2** | [`adam-train`](crates/adam-train) | Legacy transformer baseline (see [History](#history)) |

Every layer outputs deterministic, regression-tested JSON artifacts. `bash ./scripts/validate_foundation.sh` runs the full foundation validation end-to-end.

## Quick start

```bash
# Build the dialog REPL
cargo build --release -p adam-dialog --bin adam_chat

# Run it (auto-loads data/dialog/templates/v1.toml)
./target/release/adam_chat

# Single-shot
./target/release/adam_chat --once "менің атым Дәулет"

# Full Layer 1..5 trace per turn
./target/release/adam_chat --trace
```

Also available:

```bash
# FST synthesiser + analyser CLI
cargo run --release -p adam-kernel-fst --bin adam_fst -- synth --root бала --plural --case dat
# → балаларға

cargo run --release -p adam-kernel-fst --bin adam_fst -- analyse мектебім
# → noun: мектеп +P1Sg

# Full foundation validation (~30 s on M2)
bash ./scripts/validate_foundation.sh
```

## Capabilities (v1.0.0)

### 26 intents

| family | intents |
|---|---|
| Social | Greeting (Casual / Polite / Morning / Day / Evening), Farewell, Affirmation, Negation, Thanks, Apology, Compliment, Request, WellWishes |
| Conversational | AskHowAreYou, StatementOfWellbeing, AskName, StatementOfName { name } |
| Social topics | AskAge, StatementOfAge { years }, AskLocation, StatementOfLocation { city }, AskOccupation, StatementOfOccupation { occupation }, AskFamily, StatementOfFamily, AskWeather, StatementOfWeather, AskTime |
| Boundary | **Insult** (v1.1.0) — polite non-engagement for rude input |
| Fallback | Unknown { raw_tokens, noun_hint } — v1.1.0 smart handler acknowledges the topic when a noun is parseable |

Every `Statement*` intent with an `Option<T>` payload carries an extracted entity that persists into the session and feeds downstream templates.

### Kazakh-only recogniser (v1.1.0 revert)

v0.9.6 shipped Russian / English trigger phrasings for all 25 intents. Post-v1.0.0 testing showed the multilingual path diluted the Kazakh-first thesis without delivering real generalisation — a Russian speaker typing "Я разработчик" got "түсінбедім" because "разработчик" isn't in the Kazakh Lexicon. **The multilingual surface was removed in v1.1.0.** Non-Kazakh input now falls through to `Intent::Unknown`.

The project's path to handling unbounded inputs is **not translation** — it is training a compact Kazakh LM on a 100 M+ token corpus and plugging it in as the Unknown fallback. See [History](#history).

Self-introduction patterns (Kazakh only):

- `менің атым X`, `атым X`, `мені X деп атайды`, `есімім X`

### Slot syntax for FST-backed templates

Template `{slot|features}` renders via `adam_kernel_fst::morphotactics::synthesise_noun`. Features combine `+`-separated:

| family | tokens |
|---|---|
| Case | `nominative/nom, genitive/gen, dative/dat, accusative/acc, locative/loc, ablative/abl, instrumental/inst` |
| Number | `singular/sg, plural/pl` |
| Derivation | `agent, abstract/abs, privative/priv, endowed/end, similative/sim, comparative/comp, verbalnoun/vnoun, actionnoun/anoun, diminutive/dim, ordinal/ord, collective/coll` |
| Possessive | `p1sg, p2sg/p2sg_pol, p2sg_inf, p3, p1pl, p2pl/p2pl_pol, p2pl_inf` |

Example template: `"{name|instrumental} танысқаныма қуаныштымын"` → `"Дәулетпен танысқаныма қуаныштымын"`. Latin names transliterate to Cyrillic before FST synthesis: `John → Джохн → Джохнмен`.

### Session state (`Conversation`)

```rust
use adam_dialog::{Conversation, TemplateRepository};

let repo = TemplateRepository::load_default()?;
let lex  = adam_kernel_fst::lexicon::LexiconV1::load_default()?;
let mut conv = Conversation::new();

let response = conv.turn("менің атым Дәулет", &lex, &repo, seed);
// conv.session == { "name": "Дәулет" }

// next turn — {name}-referencing templates are now eligible:
let response = conv.turn("сәлем", &lex, &repo, seed);
// possible output: "сәлем Дәулет"
```

### Cross-slot templates

Multi-entity templates fire only when every referenced slot is filled. Eligibility is determined by the template filter; non-fillable templates stay in the repository but aren't picked.

| template | eligibility | example output |
|---|---|---|
| `"сәлем {name}, {city\|ablative} хабар жақсы ма"` | requires name + city | сәлем Дәулет, Алматыдан хабар жақсы ма |
| `"{name}, {age} жас — керемет кезең"` | requires name + age | Дәулет, 30 жас — керемет кезең |
| `"{name}, сіз {city\|locative} {occupation} екенсіз"` | requires all three | Дәулет, сіз Алматыда мұғалім екенсіз |

## Technical specification

| Component | Value |
|---|---|
| Lexicon roots | **14,106** (10 POS, pure pre-modern Kazakh, loanwords purged) |
| Abai Qunanbayuly coverage | **97.8%** (word forms → root prefix match) |
| FST archiphonemes | **11** |
| FST twol phonology rules | **22+** of Apertium's 54 catalogued, all implemented |
| Suffix templates | **30** (7 cases × 2 numbers × 7 possessives × 11 derivations) |
| FST synthesis → analysis roundtrip | **100.0%** on 36,238 forms |
| Dialog intents | **25** |
| Intent recognisers | 25 × ≈3 trigger phrasings per language × 3 languages |
| Template families | **29** (one per intent-key) |
| Slot types | `name`, `age`, `city`, `occupation` |
| Hallucination rate | **0%** (no generative path) |
| Workspace tests | **271 passing**, 4 ignored, 0 failing |
| End-to-end dialog tests | 81 |
| FST unit tests | 84 |

## Directory layout

See [data/README.md](data/README.md) for a top-level map of the `data/` tree, and per-subdirectory READMEs for details:

- [data/dialog/README.md](data/dialog/README.md) — template repository + schema
- [data/curated/README.md](data/curated/README.md) — source packs + manifest hierarchy
- [data/lexicon_v1/README.md](data/lexicon_v1/README.md) — Lexicon provenance
- [data/training/README.md](data/training/README.md) — legacy transformer artifacts

## History

`adam` went through two major architectural eras and a v1.1.0 course-correction:

- **v0.1.0 – v0.4.0 (transformer era)** — authentic Kazakh corpus curation (Tatoeba, Wikipedia KZ, Common Voice KK, CC-100, Abai Wikisource), BPE tokenizer, baseline transformer training. The v0.4.0 checkpoint (24.2 M parameters, PPL 1691.89 on 12 k held-out samples) is preserved in `data/training/` as a regression reference but is not on the current codepath.
- **v0.4.5 – v1.0.0 (FST + dialog era)** — deterministic FST morphology, 14 k-entry pure Kazakh Lexicon, 25-intent dialog pipeline with multi-turn session state, FST-backed slot expansion, trilingual input recogniser (briefly — see v1.1.0).
- **v1.1.0 course-correction** — Post-v1.0.0 testing showed the v0.9.6 multilingual surface was a mistake. Removing it and committing to a Kazakh-only input surface is the honest path toward a thinking Kazakh model. See [roadmap](docs/roadmap.md#post-v10-direction) for the full rationale and path to v2.0 (a compact Kazakh LM trained on 100 M+ tokens, plugged in as the `Unknown`-intent fallback).

See [CHANGELOG.md](CHANGELOG.md) for the full version-by-version history and [docs/roadmap.md](docs/roadmap.md) for the phase-by-phase overview.

## Foundation policies

- [corpus policy](docs/corpus_policy.md)
- [corpus sources](docs/corpus_sources.md)
- [curation workflow](docs/curation_workflow.md)
- [source classification](docs/source_classification.md)
- [source scoring](docs/source_scoring.md)
- [tokenizer policy](docs/tokenizer_policy.md)
- [evaluation policy](docs/evaluation_policy.md)
- [dialog architecture](docs/kazakh_grammar/07_dialog_architecture.md)
- [Kazakh grammar reference](docs/kazakh_grammar/README.md)

## Out of scope

- Multilingual output (response is always Kazakh by design)
- Speech / multimodal
- Cloud platform work
- Arbitrary-topic question answering (the 25-intent surface is the product)

The repo grows from clean data and tight scope, not from broad claims.

## License

Business Source License 1.1. Converts automatically to Apache License 2.0 on **2029-01-01**. See [LICENSE](LICENSE) for full terms.

Non-commercial and research use is unrestricted today. Commercial use is permitted unless it competes directly with Qazna Technologies LLP products or services.

Copyright © 2024–2026 Qazna Technologies LLP.

For commercial licensing inquiries: **hello@qazaq.ai**
