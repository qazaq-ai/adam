<p align="center">
  <img src="assets/shanraq.svg" alt="adam logo" width="160" height="160">
</p>

<h1 align="center">adam</h1>

<p align="center">
  <i>Predictable Kazakh-first dialog, built in pure Rust.</i><br>
  <i>Қазақ тіліне арналған, толық болжамды диалог жүйесі — таза Rust тілінде.</i>
</p>

<p align="center">
  <a href="https://github.com/qazaq-ai/adam/releases"><img src="https://img.shields.io/badge/version-3.6.5-2EA44F?style=for-the-badge" alt="version"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-BUSL%201.1-orange?style=for-the-badge" alt="license"></a>
  <img src="https://img.shields.io/badge/language-Rust-CE412B?style=for-the-badge&logo=rust&logoColor=white" alt="rust">
  <img src="https://img.shields.io/badge/script-Cyrillic-8338EC?style=for-the-badge" alt="cyrillic">
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Linux-lightgrey?style=for-the-badge" alt="platform">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/intents-26-2EA44F?style=flat-square" alt="intents">
  <img src="https://img.shields.io/badge/surface-Kazakh--only-9CCC65?style=flat-square" alt="Kazakh only">
  <img src="https://img.shields.io/badge/lexicon-14%20k%20roots-FBC02D?style=flat-square" alt="lexicon">
  <img src="https://img.shields.io/badge/corpus-77.9%20M%20local%20/%204%20M%20committed-FBC02D?style=flat-square" alt="corpus">
  <img src="https://img.shields.io/badge/retrieval-morpheme%20index-8338EC?style=flat-square" alt="retrieval">
  <img src="https://img.shields.io/badge/tests-413%20passing-2EA44F?style=flat-square" alt="tests">
  <img src="https://img.shields.io/badge/ungrounded%20generation-none%20by%20design-2EA44F?style=flat-square" alt="ungrounded generation">
</p>

---

## Why adam (v3.6.5)

adam is a **neuro-symbolic retrieval system for Kazakh** — the rule-based dialog backbone, the morpheme-indexed retrieval engine, and the forward-chaining reasoner all run together as a single deterministic pipeline. It trades **generalisation for integrity**, and (as of v3.0) adds **rule-derived reasoning** on top of retrieval.

Three things make the trade viable specifically for Kazakh:

- **Agglutinative advantage** — Kazakh's rich morphology means the FST unpacks each word into a typed bundle (root + case + number + possessive + predicate-person), which the retrieval index and reasoner both exploit. What would be a 10⁶-parameter subword model in English is a 14 k-root Lexicon + deterministic rules here.
- **Mathematical determinism** — same input + same session + same seed produces a byte-identical answer across runs. No temperature, no sampling, no GPU.
- **No ungrounded generation by design** — every output is either a template realisation, a corpus quote, or a rule derivation with a full `source_chain`. There is no free-text generator anywhere in the pipeline that could invent content not traceable to its source.

| | adam v3.6.5 | mainstream LLM |
|---|---|---|
| Outputs | template + verbatim quote + FST synthesis + **rule-derived chain** | probabilistic token generation |
| Ungrounded generation | **none by construction** (retrieval quotes verbatim; reasoner derives only from typed facts) | non-zero, non-auditable |
| Inference | ms on laptop CPU | dollars on GPU / datacentre |
| **Reasoning** | **forward-chaining over typed facts, every conclusion has a `rule_id`** | opaque emergent reasoning |
| **Provenance** | **`source_chain: Vec<FactSource>` per derivation; `(pack, sample_id)` per quote** | ~none for free-form output |
| **Inference marker** | **«байланыс-» on every reasoned claim, test-enforced** | — |
| Determinism | byte-identical across runs for same `(input, session, seed)` | temperature-dependent |
| Language coverage | Kazakh only | many, but shallow for low-resource |
| Knowledge depth | bounded by curated corpus + deterministic rules | broad, but fabricated edges |
| Self-improvement | ships by commit, reviewed by humans | parametric updates through training |

adam is **intentionally narrower** than an LLM. In return it is **predictable, cheap, safe, auditable, and — as of v3.0 — capable of deriving conclusions no single corpus sentence states**, while marking every such conclusion with a textual trust signal and a source chain.

### Current state (v3.6.5 — honest numbers)

v3.0 is **proof of mechanism, not proof of scale.** The reasoning pipeline is end-to-end and test-locked, but the fact set is deliberately small while the matchers mature.

| | value |
|---|---|
| Dialog intents | 26 |
| Lexicon roots | 14 247 |
| Corpus (committed / local) | **4.57 M** (v3.5.0: 10 textbooks) / 77.9 M words across 9 committed source packs |
| Morpheme coverage over committed corpus | 79.48 % |
| Workspace tests | **413 passing, 0 failing, 0 warnings** |
| Pattern matchers | **11** — v2.x baseline (4) + v3.5.0 (6) + **v3.5.5 structural_part_of** |
| Reasoning rules active | **4/5** — R1 IsA-transitivity, R2 Has-inheritance, **R3 Has-inheritance via PartOf (v3.5.5)**, R5 shared-IsA → RelatedTo (R4 IsA-symmetry is curator-warning only) |
| Predicates defined | **11** — IsA, LivesIn, Has, GoesTo, PartOf, RelatedTo, Causes, After, HasQuantity, DoesTo, InDomain |
| **Lexicon gap candidates queued for review (v3.4.0)** | **200** pre-tagged roots in `docs/lexicon_gap_candidates.md` (top-ranked of 104 657 distinct uncovered surfaces across the 4.32 M-word committed pool) |
| Reasoning rules active | 3 (R1 IsA-transitivity, R2 Has-inheritance, R5 shared-IsA → RelatedTo) |
| Extracted facts (committed runtime) | **13 345** (v3.6.5: T4_200k-scale committed pool via `extract_facts --bench-order --max-total 200000`; ×53 over v3.6.0). Full bench T5_1M = 67 806 facts (reference ceiling) |
| Rule-derived facts (committed runtime) | **207** (v3.6.5) — **first release where all 4 active rules fire simultaneously on the committed runtime pool** that `adam_chat` / `adam_demo` load: R1=33, R2=116, R3=2, R5=56 |
| Fact-graph nodes / edges | **2 974 / 11 813** (committed v3.6.5); most-connected content nouns: адам (237), ел (209), ат (186), жер (176), қазақ (170) |
| **Predicate coverage (v3.6.5)** | **7 / 11 = 63.6 %** — IsA, Has, PartOf, RelatedTo, After, HasQuantity, DoesTo fire on committed runtime. Not yet firing: LivesIn, GoesTo, Causes, InDomain (require specific surface patterns not dense in current corpus) |
| Iteration harness (v3.1.0) | `--time-budget <SEC>`, `--progress-interval <SEC>`, SIGINT→graceful-commit; Rayon par_iter on extract hot loop |
| Scaling bench (v3.3.0) | `adam-scaling::scaling_bench` + `audit_precision` — emits `data/scaling/scaling_report.json` + `docs/scaling_report.md` + `docs/precision_audit.md`. Budget-aware `run_tier_with_budget` (chunked at 128 samples, SIGINT / `--time-budget` stops within ~1 s). Normalized metrics per tier: `facts_per_10k_words`, `derivations_per_fact`, `predicate_coverage_pct`, `duplicate_fact_rate_pct`. **Measured scaling on 4.32 M-word committed pool (textbooks + wiki + Abai)**: T3_10k (19 facts, 0 deriv) → T4_50k (120 facts, 51 deriv) — reasoning activates once graph density crosses threshold. |
| Determinism (v3.2.0 + v3.3.0) | dual-storage Lexicon (`HashMap` get + `entries_ordered: Vec<RootEntry>` for `analyse`). Fixes a 2-year latent non-determinism where `analyse().next()` returned different first analyses across runs for ambiguous surfaces. **4 regression tests** guard the invariant, including expected-order assertions that fail ≈ 50 % on pre-v3.2.0 code. |
| Lexicon mining (v3.4.0) | `adam-corpus::mine_lexicon_gaps` scans all 9 committed packs, finds uncovered tokens, ranks globally by frequency, auto-tags (vowel harmony + final-sound class), extracts 3 context sentences per candidate. Produces `docs/lexicon_gap_candidates.md` for native-speaker review. First scan: top-5 candidates **validated against the v1.5.5-era `project_morpheme_coverage_baseline` memory** — exact match on all 5 predicted gaps (`деп, оның, осы, деген, пен`). |
| Gold corpus (v3.3.0) | 3 Kazakh secondary-school textbooks OCR'd via tesseract-kaz @ 200 DPI (pdftotext drops Қ/Ң/Ғ/Ө/Ү/Ұ/Һ on custom-font PDFs). **108 913 raw words → 8 421 samples** in `kazakh_textbooks_pack.json`, per-book provenance. 7 more textbooks staged for v3.4. |

The scale-up path is explicit: scale coverage of the four existing matchers to the full 77.9 M-word corpus, add `PartOf` / `Causes` extractors, activate R3/R4. Nothing in the architecture is gated on more data — the engine already produces derivations with full provenance.

### The v3.0 trust stack

```
 template realisation            →  recognised intent, 0% fabrication
 verbatim quote «…»              →  corpus citation, byte-identical to source
 «бейімд-» adaptation marker      →  quote was rewritten (v1.9.5)
 «байланыс-» reasoning marker     →  derivation, not a quote — v3.0 addition
```

Every marker is test-enforced in both directions: it fires when and only when the underlying path fired.

The name *adam* (Kazakh: **адам**) means "human".

## What is adam?

A **predictable, auditable Kazakh dialog system**, built **entirely in Rust**. Every output is produced by a five-layer pipeline you can trace end-to-end:

```
  input ─▶ parser ─▶ semantics ─▶ [ retrieval + compose ] ─▶ planner ─▶ realiser ─▶ FST synth ─▶ output
          (Layer 1) (Layer 2)       (Layer 2.5–2.75)       (Layer 3)   (Layer 4)   (Layer 5)
```

No transformer. No embeddings. No probabilistic generation. For any input, a developer can dump every layer's state and audit why the model chose what it said.

**Design principles:**

- **Predictable** — every stage is deterministic or samples from a finite, inspectable set.
- **Auditable** — `adam_chat --trace` dumps every layer per turn; every corpus citation names its `(pack, sample_id)`.
- **Grammatically correct by construction** on the slot path — `{slot|features}` placeholders go through the FST synthesiser, so no morphologically invalid inflected form can leave the system.
- **No ungrounded generation by default** — the retrieved quote is byte-identical to the corpus. Adaptation (`ComposeMode::InSampleCitySwap`) is opt-in and every adapted response is explicitly marked with «бейімд-» so the user always knows.
- **Small** — runs on a MacBook Air M2 8 GB. No GPU.
- **Kazakh-native** — built on a 14 k-entry curated pre-modern Kazakh Lexicon and a 77.9 M-word local corpus, not translated from English.

See [**`docs/architecture_v3.md`**](docs/architecture_v3.md) for the single canonical architecture reference ([`architecture_v2.md`](docs/architecture_v2.md) remains as a v2.0–v2.3 snapshot).

## Demo

### Scripted 15-turn walkthrough

The fastest way to see adam end-to-end. Fully deterministic, safe to record for a presentation.

```
$ cargo run --release -p adam-dialog --bin adam_demo
```

Four parts (v3.0):
- **Part 1** — all 12 canonical turns with retrieval on, `ComposeMode::Verbatim` (default). Every cited quote is byte-identical to the corpus.
- **Part 2** — same 12 turns with `ComposeMode::InSampleCitySwap`. On the real corpus, the safety guards refuse most swaps — this is the *safe case* (marker fires only when a swap actually happened).
- **Part 3** — synthetic sample explicitly triggering the swap path, so the v1.9.5 «бейімд-» marker is visible in action.
- **Part 4** — loads committed `facts.json` + `derived_facts.json`, surfaces the rule-derived chain with its `source_chain` provenance, runs a user probe across 4 deterministic seeds. Every response cites the **reasoned** chain (not a quote) and carries the v2.7 «байланыс-» trust marker.

### Interactive REPL

```
$ cargo run --release -p adam-dialog --bin adam_chat
adam-chat v3.6.5 — пікірлесейік! Қазақ тілінде сөйлесейік; ^D to quit.

> сәлем
сәлем
> менің атым Дәулет
Дәулетпен танысқаныма қуаныштымын
> мен Алматыдамын
Алматы — әдемі қала                      # v1.8.5 fix: locative+P1Sg now classified as location, not occupation
> мен бағдарламашымын
бағдарламашыға сәттілік тілеймін
> қалайсыз
жақсы, ал сіз қалайсыз
> бала туралы бірдеңе айт
Алматыда тұратын сізге бала туралы мынадай дерек:
«Кім сендерді балалар, сүйе-тұғын, Қуанышыңа қуанып, қайғыңа күйе-тұғын»
                                         # v1.6.0 retrieval + v1.7.0 ranking + v1.8.0 session-aware frame + v1.8.5 FST locative
> Абай жайында не дейсің
абай жайында осындай мысал бар: «Абай Құнанбайұлы (10 тамыз 1845 — 6 шілде 1904)»
                                         # quote from data/retrieval/morpheme_index.json → Wikipedia KZ
> сен ақымақсың
сізге ренжімеймін                        # Insult intent (v1.1.0): polite non-engagement
> сау бол
сау болыңыз
```

Kazakh-only input and output. Entities (name, age, city, occupation) persist across turns. When no intent matches, the retrieval engine looks up content morphemes in the committed morpheme index, ranks matches by **overlap + pack-purity + length-goodness − loanword-density**, and cites the top-ranked sample verbatim (guaranteed to exist in the corpus — zero fabrication). When the session has remembered entities, the frame around the citation personalises automatically via `template_is_fillable`.

## Architecture

Nine Rust crates, three layers:

| Layer | Crate | Role |
|---|---|---|
| **L0** | [`adam-kernel`](crates/adam-kernel) | Core identity + foundation contracts |
| **L0** | [`adam-kernel-fst`](crates/adam-kernel-fst) | **FST morphology** — phonology (11 archiphonemes, 22+ twol rules), morphotactics (36 suffix templates incl. v1.4.0 predicate-person copula), synthesiser + parser, 14 k-entry Lexicon |
| **L1** | [`adam-tokenizer`](crates/adam-tokenizer) | Pre-tokenizer + BPE trainer + encoder |
| **L1** | [`adam-corpus`](crates/adam-corpus) | Source acceptance, streaming processors (Wikipedia, CC-100, classics, Common Voice, Tatoeba), synthetic generator, `corpus_audit`, `morpheme_coverage` (v1.5.5) |
| **L1** | [`adam-eval`](crates/adam-eval) | Evaluation suite + benchmark reports |
| **L1** | [`adam-dialog`](crates/adam-dialog) | **Dialog pipeline** — intent recognisers (26 intents), multi-turn session + DST, template planner with `{slot\|features}` syntax, slot-expanding realiser |
| **L1** | [`adam-retrieval`](crates/adam-retrieval) | **Retrieval engine** (v1.6.0+) — morpheme inverted index (`MorphemeIndex`), deterministic `rank(input_morphemes, config)` with overlap + pack-purity + length + loanword scoring (v1.7.0), `SampleRef` provenance, `sample_texts` for direct quoting, `compose::compose_with_city` (v1.9.0) for opt-in in-sample city swap |
| **L1** | [`adam-reasoning`](crates/adam-reasoning) | **Reasoning bootstrap** (v2.1+) — structured-fact extraction over FST parses + lexical graph projection + forward-chaining rule reasoner. `Fact { subject, predicate, object, pattern, source, confidence, raw_text }`, typed `ConfidenceKind` (grammar / curated / repeated / human / rule-inferred — **not an LLM probability**), `Predicate { IsA, LivesIn, Has, GoesTo, PartOf, RelatedTo }`. Four deterministic pattern matchers. v2.3: `LexicalGraph` with `from_facts` / `outgoing` / `incoming` — nodes + typed edges with full provenance. v2.4: `reasoner::run` forward-chaining with explicit `rule_id` + `source_chain` on every `DerivedFact`. v2.5: dative-motion pattern + `GoesTo` predicate. v2.6: `PartOf` + `RelatedTo` predicates, R5 rule active → first real derivation (`кітап RelatedTo ілім`). Binaries: `extract_facts`, `build_lexical_graph`, `run_reasoner`. Implementation of **ILMRR** — Intelligent Lexical-Morphemic Retrieval & Reasoning |
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

## Capabilities

### 26 intents

| family | intents |
|---|---|
| Social | Greeting (Casual / Polite / Morning / Day / Evening), Farewell, Affirmation, Negation, Thanks, Apology, Compliment, Request, WellWishes |
| Conversational | AskHowAreYou, StatementOfWellbeing, AskName, StatementOfName { name } |
| Social topics | AskAge, StatementOfAge { years }, AskLocation, StatementOfLocation { city }, AskOccupation, StatementOfOccupation { occupation }, AskFamily, StatementOfFamily, AskWeather, StatementOfWeather, AskTime |
| Boundary | **Insult** (v1.1.0) — polite non-engagement for rude input |
| Fallback | Unknown { raw_tokens, noun_hint, example } — v1.6.5+ smart handler retrieves a corpus sample for the topic and cites it verbatim |

Every `Statement*` intent with an `Option<T>` payload carries an extracted entity that persists into the session and feeds downstream templates.

### Retrieval engine (v1.6.0–v1.9.5)

When no intent matches, `adam` falls back to **retrieve → rank → compose**:

1. Parse the user's input through the FST; extract every **content root** (no pronouns, no closed-class tokens).
2. Look those morphemes up in the committed `MorphemeIndex` (`data/retrieval/morpheme_index.json`, built offline from `tatoeba`, `wikipedia_kz`, `common_voice_kk`, `cc100_kk`, `abai_wikisource`, `kazakh_proverbs`, `synthetic_sentences`, and `kazakh_classics`).
3. Rank the candidate samples by a **deterministic composite score**:
   ```
   score = 0.40 · overlap_ratio          // main "smart" signal
         + 0.30 · pack_purity            // Abai 1.00, Wikipedia 0.85, CC-100 0.75
         + 0.15 · length_goodness        // Gaussian μ=8 words, σ=6
         − 0.15 · loanword_density       // native-Kazakh thesis
   ```
4. Quote the top-1 hit **verbatim** — guaranteed to exist in the corpus. Every quote carries a `(pack, sample_id)` provenance.
5. Choose a **session-aware template** (v1.8.0) to frame the quote — `template_is_fillable` auto-activates personalised variants when the session has `name` / `city` / `age` / `occupation`. FST-aware placeholders like `{city|locative}` (v1.8.5) render with correct vowel-harmonic suffixes.

This path is:

- **Deterministic** — rank has zero randomness; ties break on `(pack, sample_id)`. Same input + same index → byte-identical output.
- **Traceable** — every response cites its source.
- **No ungrounded generation** — we quote, never invent. The retrieved sentence is always a real sentence from a real source.

### Opt-in in-sample composition (v1.9.0+)

By default, the cited quote is **byte-identical** to the corpus sample — zero fabrication. Embedders who want composition can opt into `ComposeMode::InSampleCitySwap`:

```rust
use adam_dialog::{ComposeMode, Conversation};

let conv = Conversation::new()
    .with_morpheme_index(idx)
    .with_compose_mode(ComposeMode::InSampleCitySwap);
```

With swap mode on **and** the session carrying a known Kazakh city, city mentions inside the cited quote are rewritten to the user's city, feature-preserving via the FST (locative stays locative, etc.). Safety guards:

- **Closed list of 20 cities** — only roots in `adam_retrieval::compose::PLACE_NAMES` are swappable.
- **User's city must be in the list** — otherwise the FST can't re-synthesise reliably.
- **Biographical-year guard** — quotes containing a 4-digit year in [1500, 2100] are refused outright, so biographies like "Абай 1845 жылы Қарқаралыда туған" are never rewritten.
- **No name or number swaps** — those are the highest-fabrication-risk categories and are explicitly out of scope for v1.9.0.

**Trust contract — when we adapt, we say so (v1.9.5).** The planner routes any adapted response through the `unknown.with_adapted_evidence` template family, whose every template contains the Kazakh stem «бейімд-» ("adapt-"). Two invariants are test-enforced: when a swap happened the marker MUST fire, and when no swap happened the marker MUST NOT fire. A user can always distinguish a verbatim corpus quote from an adapted one at the textual level alone.

Every swap produces provenance via `Composition::trace()` — `[2] Алматыда → Шымкентте (root=шымкент, case=Some(Locative))` — so `adam_chat --trace` can explain every change.

### Kazakh-only recogniser (v1.1.0 revert)

v0.9.6 shipped Russian / English trigger phrasings for all 25 intents. Post-v1.0.0 testing showed the multilingual path diluted the Kazakh-first thesis without delivering real generalisation — a Russian speaker typing "Я разработчик" got "түсінбедім" because "разработчик" isn't in the Kazakh Lexicon. **The multilingual surface was removed in v1.1.0.** Non-Kazakh input now falls through to `Intent::Unknown`, which since v1.6.5 routes through the retrieval engine above.

The project's path to handling unbounded inputs is **not translation and not a trained neural LM** — it is the retrieval engine above, scaled to a ~100 M-token Kazakh corpus. See [History](#history) and [roadmap](docs/roadmap.md#post-v10-direction) for the architectural rationale.

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
| Lexicon roots | **14,247** (≥ 3 chars, curated + Apertium, pure Kazakh) |
| Abai Qunanbayuly coverage | **97.8%** (word forms → root prefix match) |
| Committed corpus words | **4.57 M** across 9 source packs (v3.5.0: 10 textbooks in `kazakh_textbooks_pack.json` — 434 581 raw words / 28 110 samples; earlier was 3.84 M / 8 packs pre-textbook) |
| Local corpus words (with Wikipedia + CC-100 shards) | **77.9 M** |
| Morpheme-coverage baseline (v1.5.5 historical) | **79.48%** prefix-match over 3.84 M committed words at v1.5.5; to be re-run on every Lexicon PR (see `project_morpheme_coverage_baseline` memory) |
| FST archiphonemes | **11** |
| FST twol phonology rules | **22+** of Apertium's 54 catalogued, all implemented |
| Suffix templates | **36** (cases × numbers × possessives × derivations × predicate-person copula) |
| FST synthesis → analysis roundtrip | **100.0%** on 36,238 forms |
| FST parser throughput | **1.155 ms / word** single-threaded M2 |
| Dialog intents | **26** (v1.1.0 added Insult) |
| Template families | **34** (v3.0 added `unknown.with_derived_chain`) |
| Slot types (session) | `name`, `age`, `city`, `occupation` (plus `{slot\|features}` FST-aware variants) |
| Committed morpheme index | **3,191 samples → 3,082 distinct morphemes → 16,262 postings** at the v2.5.0 build (`data/retrieval/morpheme_index.json`, ~2.1 MB; textbooks corpus added in v3.3.0 is not yet in the index — rebuild pending a v3.4+ release) |
| Full local morpheme index | rebuildable via `build_morpheme_index -- --full` (~10 min, ~700 MB, gitignored) |
| Pattern matchers | **11** — v2.x (4) + v3.5.0 (6) + **v3.5.5 structural_part_of** (X Y-нің бөлігі / X Y-нің құрамында) |
| Reasoning rules active | **4** — R1 IsA-transitivity, R2 Has-inheritance, **R3 Has-inheritance via PartOf (v3.5.5)**, R5 shared-IsA → RelatedTo |
| Predicates defined | **11** — IsA, LivesIn, Has, GoesTo, PartOf, RelatedTo, Causes, After, HasQuantity, DoesTo, InDomain |
| Extracted / derived facts (committed runtime) | **13 345 / 207** (v3.6.5: committed pool scaled to T4_200k equivalent via `extract_facts --bench-order --max-total 200000`. All 4 active rules fire: R1=33 IsA-transitivity, R2=116 Has-inheritance, R3=2 Has-via-PartOf, R5=56 shared-IsA) |
| Ungrounded generation rate | **none by construction** (retrieval quotes verbatim; reasoner derives only from typed facts) |
| Workspace tests | **413 passing**, 0 failing, 0 warnings |
| Extraction throughput (v3.1.0) | **~3 000 samples / 12 s** on M2 8-core (Rayon) — ~3.5× over v3.0 sequential; 20 M-word full-corpus run fits in the 3 h iteration budget |

## Directory layout

See [data/README.md](data/README.md) for a top-level map of the `data/` tree, and per-subdirectory READMEs for details:

- [data/dialog/README.md](data/dialog/README.md) — template repository + schema
- [data/curated/README.md](data/curated/README.md) — source packs + manifest hierarchy
- [data/lexicon_v1/README.md](data/lexicon_v1/README.md) — Lexicon provenance
- [data/training/README.md](data/training/README.md) — legacy transformer artifacts

## History

`adam` went through three major architectural eras and a v1.1.0 course-correction:

- **v0.1.0 – v0.4.0 (transformer era)** — authentic Kazakh corpus curation (Tatoeba, Wikipedia KZ, Common Voice KK, CC-100, Abai Wikisource), BPE tokenizer, baseline transformer training. The v0.4.0 checkpoint (24.2 M parameters, PPL 1691.89 on 12 k held-out samples) is preserved in `data/training/` as a regression reference but is **not** on the current codepath.
- **v0.4.5 – v1.0.0 (FST + dialog era)** — deterministic FST morphology, 14 k-entry pure Kazakh Lexicon, 25-intent dialog pipeline with multi-turn session state, FST-backed slot expansion.
- **v1.1.0 course-correction** — post-v1.0.0 testing showed the v0.9.6 multilingual surface was a mistake. Removing it and committing to a Kazakh-only input surface is the honest path toward a thinking Kazakh model.
- **v1.5.0 – v1.8.5 (retrieval era)** — the path to v2.0 is **retrieval**, not a trained neural LM. v1.5.0 re-extracted CC-100 into a 77.9 M-word local corpus. v1.5.5 measured the 79.48 % morpheme-coverage baseline. v1.6.0 shipped `adam-retrieval` with the morpheme inverted index. v1.6.5 wired retrieval into `Intent::Unknown` so dialog cites Abai / Wikipedia / CC-100 verbatim. v1.7.0 added deterministic ranking (overlap + purity + length + loanword density). v1.8.0 introduced **session-aware composition (option A)** — the retrieved quote stays verbatim, the frame around it personalises via the session. v1.8.5 fixed the `-мын` greedy-strip bug and wired FST-aware `{city|locative}` into session-aware templates.
- **v1.9.0 (option B entry)** — first step where the retrieved quote is no longer guaranteed byte-identical. `ComposeMode::InSampleCitySwap` (opt-in; default stays `Verbatim`) rewrites city mentions inside the cited quote to the user's session city via feature-preserving FST synthesis. Safety guards: closed 20-city list, biographical-year refusal (any year 1500–2100), no name/number swaps. Grammaticality FST-guaranteed; semantic truthfulness now a trade-off, explicitly marked in the mode setter.

See [CHANGELOG.md](CHANGELOG.md) for the full version-by-version history and [docs/roadmap.md](docs/roadmap.md) for the phase-by-phase overview, including the v1.9.0+ roadmap toward in-sample slot swap (option B/C territory, with semantic sanity guards).

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

- **Multilingual input and output** (v1.1.0 revert). The v0.9.6 Russian / English triggers were removed; `adam` accepts and produces only Kazakh. Generalisation comes via the retrieval engine over the 77.9 M-word Kazakh corpus, not translation.
- **Speech / multimodal** — deferred until the retrieval engine is a solid baseline.
- **Cloud platform work.**
- **Probabilistic / LLM-style free generation.** Every response is either a template realisation (26-intent path), a verbatim corpus quote (retrieval path), or a rule derivation over typed facts with a full `source_chain` (reasoning path). Nothing invented.
- **50 M+ parameter transformer experiments on current hardware** (M2 8 GB). v2.0 will **not** be a trained neural LM — it will be the retrieval engine above, extended with pattern-based composition and ranking polish. See [`project_retrieval_not_neural_v2`](docs/roadmap.md#post-v10-direction).

The repo grows from clean data, tight scope, and deterministic composition. Not from broad claims, and not from gradient descent.

## License

Business Source License 1.1. Converts automatically to Apache License 2.0 on **2029-01-01**. See [LICENSE](LICENSE) for full terms.

Non-commercial and research use is unrestricted today. Commercial use is permitted unless it competes directly with Qazna Technologies LLP products or services.

Copyright © 2024–2026 Qazna Technologies LLP.

For commercial licensing inquiries: **hello@qazaq.ai**
