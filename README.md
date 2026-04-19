<p align="center">
  <img src="assets/shanraq.svg" alt="adam logo" width="160" height="160">
</p>

<h1 align="center">adam</h1>

<p align="center">
  <i>A Kazakh-first foundation language model, built in pure Rust.</i><br>
  <i>Қазақ тіліне арналған тіл моделінің іргетасы — таза Rust тілінде.</i>
</p>

<p align="center">
  <a href="https://github.com/qazaq-ai/adam/releases"><img src="https://img.shields.io/badge/version-0.9.9-blue?style=for-the-badge" alt="version"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-BUSL%201.1-orange?style=for-the-badge" alt="license"></a>
  <img src="https://img.shields.io/badge/language-Rust-CE412B?style=for-the-badge&logo=rust&logoColor=white" alt="rust">
  <img src="https://img.shields.io/badge/script-Cyrillic-8338EC?style=for-the-badge" alt="cyrillic">
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Linux-lightgrey?style=for-the-badge" alt="platform">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/params-24.2M-2EA44F?style=flat-square" alt="params">
  <img src="https://img.shields.io/badge/perplexity-1691.89-2EA44F?style=flat-square" alt="perplexity">
  <img src="https://img.shields.io/badge/corpus-244k%20samples-9CCC65?style=flat-square" alt="corpus">
  <img src="https://img.shields.io/badge/vocab-8192-FBC02D?style=flat-square" alt="vocab">
  <img src="https://img.shields.io/badge/lexicon-211%20roots-FBC02D?style=flat-square" alt="lexicon">
  <img src="https://img.shields.io/badge/sources-7%20packs-9CCC65?style=flat-square" alt="sources">
  <img src="https://img.shields.io/badge/foundation-validated-2EA44F?style=flat-square" alt="foundation">
</p>

---

## What is adam?

`adam` is a foundation language model for Kazakh, built **entirely in Rust** — no Python, no external ML pipelines. The whole stack — lexicon definition, finite-state morphological engine, synthetic corpus generation, BPE tokenizer, model training, and inference — runs on a single MacBook Air M2 8GB.

The name *adam* (Kazakh: **адам**) means "human".

The mission is small but precise: build a culturally and linguistically grounded foundation that the Kazakh NLP community can reuse — from the morphological analyzer up to a working transformer language model.

## Architecture

Three layers, five Rust crates:

| Layer | Crate | Role |
|---|---|---|
| **L0** | [`adam-kernel`](crates/adam-kernel) | Identity + Kazakh FSM morphological engine |
| **L1** | [`adam-tokenizer`](crates/adam-tokenizer) | Pre-tokenizer + BPE trainer + encoder/decoder |
| **L1** | [`adam-corpus`](crates/adam-corpus) | Source acceptance + synthetic sentence generator |
| **L1** | [`adam-eval`](crates/adam-eval) | Evaluation suite + benchmark reports |
| **L2** | [`adam-train`](crates/adam-train) | Transformer model + training loop + inference |

Every layer outputs deterministic, regression-tested JSON artifacts. `bash ./scripts/validate_foundation.sh` runs the full pipeline end-to-end.

## Quickstart

```bash
# 1. Build everything
cargo build --release

# 2. Validate the foundation (all golden artifacts + tests)
bash ./scripts/validate_foundation.sh

# 3. Generate text with the v0.4.0 checkpoint
bash ./scripts/run_generate.sh "жақсы адам" 24 1.0 0 0.9 1.2
#                              prompt        ^^  ^   ^   ^   ^
#                                            new temp tk topp rep_pen
```

## Sample generations

From the v0.4.0 checkpoint (24.2M params, 20,000 training steps on 244k-sample corpus). A mix of nucleus and `temp=0.8` results to show the model's range:

| Prompt | Generated |
|---|---|
| жақсы адам | жақсы адам мағына береді. |
| ол | ол жазады. |
| олар | олар жүреді. |
| үлкен қала | үлкен қала айтады. |
| үлкен жақсы адам | үлкен жақсы адам оқыйды. |
| мектеп туралы | мектеп туралы мәртебе нақтылайды. |
| мен қазір | мен қазір арттырады. |

Complete grammatical Kazakh sentences now appear consistently at low temperatures — the v0.4.0 fix (literary corpus, Abai integration, no-short-synth filter) gave the model enough signal to finish a clause. Greedy generations still terminate early (the 24M model is capacity-bound on 4.09M training tokens). Reproducible via `bash ./scripts/run_generation_showcase.sh` → `data/training/generation_showcase_report.json`. Coherent chat-level output remains a v0.5.0 target.

## Full training pipeline

```bash
# 1. Fetch authentic Kazakh text sources
bash ./scripts/fetch_tatoeba_kazakh.sh
bash ./scripts/fetch_wikipedia_kz.sh
bash ./scripts/fetch_common_voice_kk.sh
bash ./scripts/fetch_cc100_kk.sh
bash ./scripts/fetch_abai_wikisource.sh

# 2. Process each source into pack JSON
cargo run --release --bin process_tatoeba_kazakh
cargo run --release --bin process_wikipedia_kz -- data/external/wikipedia_kz_plain.txt data/curated/wikipedia_kz_pack.json 100000
cargo run --release --bin process_common_voice_kk
xzcat data/external/cc100_kk.txt.xz | target/release/process_cc100_kk data/curated/cc100_kk_pack.json 50000
cargo run --release --bin process_abai_wikisource

# 3. Generate 100,000 FSM-validated synthetic sentences (min 3 words)
bash ./scripts/run_synth_sentences.sh 100000

# 4. Combine all packs into the unified corpus (dedup included)
bash ./scripts/run_unified_corpus_assembly.sh

# 5. FSM-segment every word into morphemes (char fallback otherwise)
bash ./scripts/run_pretokenize_corpus.sh

# 6. Learn BPE merges (lexicon-seeded vocab, target 8192)
bash ./scripts/run_train_bpe.sh 8192

# 7. Encode with deterministic train/val split
bash ./scripts/run_encode_corpus.sh

# 8. Train the 24M transformer (~8h on M2 Metal; periodic checkpoint every 2000 steps)
bash ./scripts/run_train_baseline.sh 20000 8 128 3e-4 400 500

# 9. Evaluate held-out perplexity
bash ./scripts/run_eval_perplexity.sh

# 10. Run the 60-generation showcase
bash ./scripts/run_generation_showcase.sh
```

## Key binaries

| Binary | Crate | Purpose |
|---|---|---|
| `coverage_report` | adam-kernel | Measure FSM coverage on real Kazakh text |
| `synth_sentences` | adam-corpus | Generate FSM-validated synthetic sentences |
| `pretokenize_corpus` | adam-tokenizer | Morpheme-aware pre-tokenization |
| `train_bpe` | adam-tokenizer | BPE trainer over morpheme stream |
| `encode_corpus` | adam-tokenizer | Encode pack to token IDs (with train/val split) |
| `train_baseline` | adam-train | AdamW training loop with grad clipping |
| `eval_perplexity` | adam-train | Held-out perplexity evaluation |
| `generate` | adam-train | Inference with greedy/temperature/top-k/top-p/repetition-penalty |
| `generation_showcase` | adam-train | Multi-prompt × multi-config quality report |

## Stats (v0.4.0)

| Component | Value |
|---|---|
| Lexicon roots | **211** (10 POS) |
| FSM rules | **422** |
| Eval segmentation examples | **464** (100% match rate) |
| Authentic Kazakh sources | **5** (Tatoeba, Wikipedia, Common Voice, CC-100, Abai) |
| Total pack sources | **7** (+ curated clean, proverbs, synthetic FSM) |
| Training samples | **244,625** unique (232,524 train + 12,101 val) |
| Training tokens (encoded) | **4,094,435** (0.00% unknowns, 100.00% roundtrip) |
| BPE vocabulary | **8,192** |
| BPE compression | **3.27×** |
| Model parameters | **24.2M** (hidden 512, layers 5, heads 8, ffn 2048) |
| Wall time (M2 Metal, 20k steps, seq=128 batch=8) | **~8h** |
| Periodic checkpoints | **every 2000 steps** (crash-resilient since v0.4.0) |
| **Validation perplexity** | **1691.89** (12,101 held-out samples, v0.4.0 model) |

## v0.9.9 — Morphology correctness + phrasing polish

Last stretch before the v1.0.0 MVP cut.

**Fixed two FST morphology bugs** in the Instrumental suffix:

| root | pre-v0.9.9 | v0.9.9 |
|---|---|---|
| Алматы | Алматы**ман** ❌ | Алматы**мен** ✓ |
| мұғалім | мұғалім**бен** ❌ | мұғалім**мен** ✓ |
| Джохн | Джохн**бан** ❌ | Джохн**мен** ✓ |
| Дәулет | Дәулетпен ✓ | Дәулетпен ✓ |

- The Instrumental suffix is invariant in vowel (`-мен/-бен/-пен` regardless of harmony) — the old template used harmony-alternating archiphoneme `{E}` which produced wrong `-ман/-бан/-пан` on back-vowel stems. Changed to literal `е`.
- `realise_m` flipped nasal consonants to `б` instead of `м`, so `мұғалімен` became `мұғалімбен`. Fixed to preserve `м` after nasals.

6 new FST unit tests lock in the fix across every consonant-class path.

**Template polish pass.** Dropped generic filler ("түсіндім" appeared in 3 statement families) in favour of topic-specific acknowledgements:

- `statement_of_age` → `қуатты кезеңіңіз` (vigorous period)
- `statement_of_location` → `тамаша өлке` (wonderful region)
- `statement_of_occupation` → `мақтанатын жұмыс` (work to be proud of)
- `statement_of_weather` → `табиғат мезгіліне лайық` (fitting for the season)

Workspace totals: **271 passing**, 4 ignored, 0 failing. Foundation CI green.

## v0.9.8 — Full slot syntax + transliteration + cross-slot templates

Three improvements that together push the dialog layer toward demo-ready quality:

**1. Slot syntax covers all four NounFeature fields.** v0.9.5 parsed `case + number`; v0.9.8 adds `derivation` (11 tokens: agent / abstract / privative / endowed / similative / comparative / verbalnoun / actionnoun / diminutive / ordinal / collective) and `possessive` (7 tokens: p1sg / p2sg / p2sg_inf / p3 / p1pl / p2pl / p2pl_inf). `{name|agent+p1sg+dative}` is now a single synthesis pass.

**2. Latin → Cyrillic transliteration.** When the template requests morphology on a Latin root (say `my name is John` → session.name = "John"), the realiser now transliterates to Cyrillic (`Джохн`) before `synthesise_noun`, then the FST inflects naturally (`Джохнбен…`). Plain `{name}` substitution still preserves the user's original Latin spelling ("сәлем John").

**3. Cross-slot templates.** Multiple session entities in one response — fires only when every slot is fillable:

```
$ adam_chat
> менің атым Дәулет
Дәулетпен танысқаныма қуаныштымын
> мен Алматыданмын
Алматы — әдемі қала
> мен әншімін
Дәулет, сіз Алматыда әнші екенсіз          ← triple-slot template
> қалайсыз
жақсымын Дәулет, ал сіз қалайсыз           ← cross-slot with name
```

81 dialog end-to-end tests (up from 78). 23 lib-level unit tests. Workspace totals: **265 passing**, 4 ignored, 0 failing.

## v0.9.7 — Lexicon-backed occupation recognition

The hand-written 6-form occupation table is replaced with generic 1sg-copula stripping against the 14 k-entry Lexicon. Any noun in the Lexicon ending in `-мын / -мін / -пын / -пін / -бын / -бін` now routes to `StatementOfOccupation` — Lexicon coverage becomes the cap, not the recogniser.

```
$ adam_chat
> мен ақынмын              сіз ақын екенсіз            ← new: ақын was NOT in old table
> мен әншімін              әншілер — қажетті мамандық  ← FST plural on new extract
> мен ғалыммын             сіз ғалым екенсіз
> мен суретшімін           сіз суретші екенсіз
> мен мұғаліммін           жақсы кәсіп
> жақсымын                 жақсы екен                  ← POS filter → wellbeing
```

### POS filter

The Lexicon entry for `жақсы` is tagged `adjective`, so `жақсымын` is rejected from occupation extraction and routes correctly to `StatementOfWellbeing`. Same guard against `жаман` and other common adjective-predicate confounds.

### Public API addition

```rust
pub fn interpret_text_with_lexicon(
    input: &str,
    parses: &[Analysis],
    lexicon: Option<&LexiconV1>,
) -> Intent
```

The original `interpret_text(input, parses)` is now a thin wrapper that calls the lexicon-aware variant with `None` — existing callers keep working.

78 dialog end-to-end tests (up from 73). Workspace totals: **251 passing**, 4 ignored, 0 failing.

## v0.9.6 — Multilingual recogniser surface

The model now reads Kazakh, Russian, and English input across all 25 intents — and replies exclusively in Kazakh. This is **not** translation: the core pipeline stays deterministic Kazakh-only. Only the recogniser layer widens; more surface forms map to the same Intent.

```
$ adam_chat
> hello
сәлем
> how are you
жақсымын, рахмет
> my name is John
John танысқаныма қуаныштымын            ← Latin name stays bare (FST is Kazakh-only)
> меня зовут Дәулет
Дәулетпен сөйлесу — құрмет              ← Cyrillic root → full FST synthesis
> спасибо
ештеңе емес
> bye
сау болыңыз
```

### Latin root safety

FST phonology is tuned for Kazakh Cyrillic. Feeding `"John"` into `synthesise_noun(..., Case::Instrumental)` would produce garbled `"Johnман"`. The realiser now detects non-Cyrillic roots and falls back to plain substitution — no suffix is attached, no hallucinated morphology.

### Ordering change

`StatementOfName` is checked BEFORE `Greeting` so `"hi i am John"` doesn't misfire as a bare Casual greeting. All self-introduction patterns (атым / есімім / зовут / my name is / call me / [greet] i am X) are explicit enough to rule out false positives.

73 dialog end-to-end tests (up from 56). Workspace totals: **245 passing**, 4 ignored, 0 failing.

## v0.9.5 — FST-backed slot expansion

Templates can now request grammatical forms via `{slot|features}`; the realiser synthesises them through `adam_kernel_fst::morphotactics::synthesise_noun`. Kazakh case marking becomes FST-guaranteed — no more hand-written agreement.

```
$ adam_chat
> менің атым Дәулет
Дәулетпен танысқаныма қуаныштымын       ← {name|instrumental} → Дәулетпен
> мен Алматыданмын
Алматыда тұрасыз ба                     ← {city|locative}  → Алматыда
> мен мұғаліммін
мұғалімдер — қажетті мамандық           ← {occupation|plural} → мұғалімдер
> сәлем
сәлем Дәулет, Алматыдан хабар жақсы ма  ← cross-slot: {name} + {city|ablative}
```

### Feature spec

Slot features are case-insensitive and `+`-separated:

| token | field set |
|---|---|
| `nominative`/`nom`, `genitive`/`gen`, `dative`/`dat`, `accusative`/`acc`, `locative`/`loc`, `ablative`/`abl`, `instrumental`/`inst` | `case` |
| `singular`/`sg`, `plural`/`pl` | `number` |

Unknown tokens are silently ignored — templates stay forward-compatible as new feature types arrive (`derivation`, `possessive` in v1.0.0).

### Cross-slot templates

The v0.8.5 template-fillability filter handles multi-slot templates for free: a template referencing both `{name}` and `{city|ablative}` is only eligible when both are in session. Plain single-slot and literal templates stay available for sessions that don't have all entities — no regression.

56 dialog end-to-end tests (up from 52). Workspace totals: **229 passing**, 4 ignored, 0 failing.

## v0.9.0 — Full entity absorption (age, city, occupation)

Every MVP social-topic statement now contributes an extractable entity that persists across turns. The user tells the model their age once — it remembers. Says where they're from — remembers. Names their occupation — remembers. Templates personalise accordingly.

```
$ adam_chat
> менің атым Дәулет
сәлем Дәулет
> менің жасым отыз
30 жас — тамаша кезең
> мен Алматыданмын
Алматы — әдемі қала
> мен мұғаліммін
мұғалім — құрметті кәсіп
```

Session now carries `{name, age, city, occupation}`.

### Kazakh numeral parser

`semantics::parse_kazakh_age` parses 1–99:

| form | value |
|---|---|
| `бір` | 1 |
| `он` | 10 |
| `отыз` | 30 |
| `отыз бес` | 35 (compound) |
| `тоқсан тоғыз` | 99 |
| `"30"` | 30 (literal digits also accepted) |

### Entity extraction rules

- **Age** — numeral parsed from any token in a 1st-person age statement.
- **City** — ablative+copula or locative stripped from the token:
  - `Алматыданмын` → `Алматы`
  - `астанада тұрамын` → `астана`
- **Occupation** — 1sg copula stripped from a fixed table of known occupation forms (extensible): `мұғалім, дәрігер, студент, инженер, оқушы, жұмысшы`.

### Intent payload changes (breaking)

```rust
Intent::StatementOfAge { years: Option<u32> }
Intent::StatementOfLocation { city: Option<String> }
Intent::StatementOfOccupation { occupation: Option<String> }
```

52 dialog end-to-end tests (up from 44). Workspace totals: **215 passing**, 4 ignored, 0 failing.

## v0.8.5 — Multi-turn session state

The dialog pipeline now has memory. A `Conversation` struct accumulates entities extracted from previous turns and feeds them back into the planner as slot values. Say your name once, get greeted by name on every subsequent turn.

```
$ adam_chat
adam-chat v0.8.5 — пікірлесейік! Type a Kazakh sentence; ^D to quit.
> менің атым Дәулет
сәлем Дәулет
> сәлем
сәлем Дәулет            ← the model remembers
> қайырлы таң
қайырлы таң Дәулет
> сәлеметсіз бе
сәлеметсіз бе Дәулет
```

API surface:

```rust
use adam_dialog::{Conversation, TemplateRepository};

let mut conv = Conversation::new();
let response = conv.turn(input, &lexicon, &repo, seed); // absorbs + plans + realises
// conv.session is a HashMap<String, String> — {name: "Дәулет"} after a self-intro.
conv.reset(); // clear state
```

**Template filtering.** `plan_response_with_session` only considers templates whose every `{slot}` is satisfiable from the merged (session ∪ per-turn) slot map. When no templates are fillable, it falls back to the unfiltered pool rather than crash. This means adding a `сәлем {name}` variant doesn't break sessions where the user never introduced themselves — the template simply isn't eligible.

**Ordering.** `Conversation::turn` absorbs entities BEFORE planning, so the same turn that says "менің атым X" can already receive a personalised response.

44 dialog end-to-end tests (3 new multi-turn tests). Workspace totals: **204 passing**, 4 ignored, 0 failing.

## v0.8.0 — 25 intents + PersonName extraction

Dialog layer widened from 10 to **25 intents** covering full MVP social-conversation topics: introductions, age, location, occupation, family, weather, time, compliments, requests, well-wishes. First entity extraction lands: the user's name is pulled from self-introduction patterns and substituted into the response via `{name}` slot placeholders.

New intents (+15):

| intent | example input | example response |
|---|---|---|
| `StatementOfName { name }` | `менің атым Дәулет` | `сәлем Дәулет` / `қош келдіңіз Дәулет` |
| `AskAge` | `жасың неше` | `мен әлі жаспын` / `менің жасым адамзат жасындай` |
| `StatementOfAge` | `менің жасым отыз` | `түсіндім` / `жақсы жас` |
| `AskLocation` | `қайда тұрасыз` | `мен сандық әлемде тұрамын` |
| `StatementOfLocation` | `мен Алматыданмын` | `жақсы жер` / `әдемі аймақ` |
| `AskOccupation` | `немен айналысасың` | `мен тілдерді үйренемін` |
| `StatementOfOccupation` | `мен мұғаліммін` | `жақсы кәсіп` |
| `AskFamily` | `балаларың бар ма` | `мен жалғызбын` |
| `StatementOfFamily` | `менің балам бар` | `отбасыңыз аман болсын` |
| `AskWeather` | `ауа райы қалай` | `менде терезе жоқ` |
| `StatementOfWeather` | `бүгін суық` | `ауа райы мейірімді болсын` |
| `AskTime` | `сағат неше` | `уақыт — асыл қазына` |
| `Compliment` | `жарайсың` | `рахмет` / `сіз де өте жақсысыз` |
| `Request` | `көмектесіңіз` | `әрине, айтыңыз` / `тыңдап тұрмын` |
| `WellWishes` | `сәттілік` | `сізге де` / `тілегіңіз қабыл болсын` |

Entity extraction + slot expansion:

- `ResponsePlan` gains `slots: HashMap<String, String>` populated from the Intent (e.g., `{"name": "Дәулет"}`)
- `realiser::realise` substitutes `{slot}` placeholders in the chosen template
- PersonName is extracted from three surface patterns: `атым X`, `мені X деп атайды`, `есімім X` — case preserved and title-cased on output

Ordering rule: Statement-of-X checked BEFORE Ask-of-X. A 1st-person marker ("келдім", "тұрамын", "жасым") unambiguously identifies the user as stating, not asking.

41 dialog end-to-end tests (up from 23). Workspace totals: **201 passing**, 4 ignored, 0 failing.

## v0.7.5 — 10 intents + TOML templates

Widens the v0.7.0 dialog layer to 10 intents and moves template content out of Rust code into [`data/dialog/templates/v1.toml`](data/dialog/templates/v1.toml). Adding a new response phrase no longer requires recompiling.

| intent | example input | example response |
|---|---|---|
| `Greeting` (Casual) | `сәлем` | `сәлем` / `сәлем достым` |
| `Greeting` (Polite) | `сәлеметсіз бе` | `сәлеметсіз бе` / `армысыз` |
| `Greeting` (TimeOfDay) | `қайырлы таң` | `қайырлы таң` / `қайырлы таң болсын` |
| `Farewell` | `сау бол` | `сау бол` / `кездескенше` / `аман бол` / `сау болыңыз` |
| `Affirmation` | `иә` / `дұрыс` | `иә` / `дұрыс айтасыз` / `рас` / `мақұл` |
| `Negation` | `жоқ` | `жоқ` / `дұрыс емес` |
| `Thanks` **(new)** | `рахмет` / `көп рахмет` | `оқасы жоқ` / `ештеңе емес` / `ризамын` |
| `Apology` **(new)** | `кешіріңіз` | `ештеңе емес` / `мейлі` / `түк етпейді` |
| `AskHowAreYou` **(new)** | `қалайсыз` | `жақсымын, рахмет` / `жаман емеспін` / `жақсы, ал сіз қалайсыз` |
| `StatementOfWellbeing` **(new)** | `жақсымын` | `жақсы екен` / `қуанамын` / `ал сіз қалайсыз` |
| `AskName` **(new)** | `атың кім` / `атыңыз кім` | `менің атым адам` / `мені адам деп атайды` |
| `Unknown` (fallback) | `xyz` | `түсінбедім` / `қайта айтыңызшы` |

CLI:

```bash
cargo build --release -p adam-dialog --bin adam_chat
./target/release/adam_chat          # REPL (auto-loads data/dialog/templates/v1.toml)
./target/release/adam_chat --once "қалайсыз"
./target/release/adam_chat --trace  # full Layer 1..5 pipeline trace
```

Public API additions: `TemplateRepository`, `respond_with_repo`, `plan_response_with_repo`, `intent_key`.

23 dialog end-to-end tests (up from 15) verify the pipeline against the v1.0.0 lexicon. Workspace totals: **183 passing**, 4 ignored, 0 failing.

## v0.7.0 — First dialog layer

Initial version of the MVP dialog pipeline, 5 intents, templates hardcoded in `planner.rs`. See [`docs/kazakh_grammar/07_dialog_architecture.md`](docs/kazakh_grammar/07_dialog_architecture.md) for the architectural spec.

## v0.6.0 — Derivational morphology

Adds the "word-formation" layer per the user's directive. The FST can now derive new word classes from roots before applying inflection, closing the `root → new root → inflected form` pipeline.

| suffix | meaning | example |
|---|---|---|
| `-шы / -ші` | agent noun | жазу → жазушы (writer) |
| `-лық / -лік` | abstract noun | жақсы → жақсылық (goodness) |
| `-сыз / -сіз` | privative (without) | тұз → тұзсыз (saltless) |
| `-лы / -лі` | endowed-with | күш → күшті (strong) |
| `-дай / -дей` | similative | тау → таудай (mountain-like) |
| `-рақ / -рек` | comparative | жақсы → жақсырақ (better) |
| `-у` | verbal noun | жаз → жазу (writing) |
| `-ым / -ім` | action-result | айт → айтым (saying) |
| `-шық / -шік` | diminutive | үй → үйшік (little house) |
| `-ншы / -нші` | ordinal | бір → бірінші (first) |
| `-еу / -ау` | collective | бір → біреу (someone) |

These derivations chain cleanly with existing inflection (e.g., жазу → жазушы → жазушыға, "writer" in dative).

`adam-kernel-fst` unit tests: **78 passing** (up from 68 in v0.5.0). Workspace totals: 160 tests passing, 4 ignored, 0 failing.

## v0.5.5 — Pure Kazakh lexicon

Phase v0.5.5 enforces the "pure pre-modern Kazakh" directive at the lexicon level. The combined v0.4.5 lexicon (16,373 entries) was audited against strict purity criteria, filtered to the 13,606-entry pure-Kazakh subset, and then augmented with 500 missing classical roots extracted from Abai Qunanbayuly's corpus.

| step | result |
|---|---|
| Total entries (v0.4.5) | 16,373 |
| Dropped: Russian-only letters (ё,ф,ц,ч,щ,ъ,ь,э) | 824 |
| Dropped: loanword suffix (-ция, -изм, -лог, …) | 128 |
| Dropped: no Kazakh signal | 681 |
| **Pure Kazakh retained** | **13,606** |
| Augmented from Abai corpus | +500 |
| **Final v1 lexicon** | **14,106** |
| **Abai coverage** (word forms → root prefix match) | **97.8%** (was 88.8%) |

Missing-vocabulary highlights that were added from Abai:
- `сөз` (word, speech — used 123× in Abai!)
- `бой` (body, self), `қан` (blood), `қол` (hand), `қар` (snow)
- `жау` (enemy / rain), `жат` (lie down / stranger), `жет` (reach)
- `надан` (ignorant — Abai's key concept)
- `сал`, `қал`, `түс`, `қыс`, `жай` and ~480 others

These are fundamental proto-Kazakh vocabulary that the Apertium import had NO entries for — it over-indexes modern technical terminology and under-indexes the literary semantic core.

## v0.5.0 — FST participles + converbs + vowel-stem coalescence

Expansion of the v0.4.5 FST. Adds participles, converbs, and the vowel-final-stem coalescence rules (оқы + PRES = оқиды). The FST now covers most non-finite forms of Kazakh verbs.

| Component | Value |
|---|---|
| New crate | [`adam-kernel-fst`](crates/adam-kernel-fst/) |
| Unit tests | **68 passing** (up from 55 in v0.4.5) |
| Archiphonemes | 12 |
| Phonological rules | 54 catalogued; 22+ implemented |
| Suffix templates | 30 (+ 3 participles, + 2 converbs since v0.4.5) |
| Lexicon (v1) | **14,296 entries** = 4,454 curated + 11,919 Apertium-imported |
| Roundtrip coverage | **36,238 / 36,238 = 100.0%** (full-lexicon synthesise → analyse) |
| CLI binary | [`adam_fst`](crates/adam-kernel-fst/src/bin/adam_fst.rs) with `synth`, `analyse`, `stats` |

Examples:
```bash
$ target/release/adam_fst synth --root бала --plural --case dat
балаларға

$ target/release/adam_fst synth --root жаз --voice passive --negation --tense past --person 3
жазылмады

$ target/release/adam_fst analyse мектебім
noun: мектеп +P1Sg

$ target/release/adam_fst synth --root оқы --tense present --person 3
оқиды

$ target/release/adam_fst synth --root жаз --tense ParticiplePast
жазған
```

The FST is not a language model; it handles morphology deterministically so a future small LM (v0.5.5+) doesn't need to waste capacity learning suffix patterns.

## Foundation Policies

- [corpus policy](docs/corpus_policy.md)
- [corpus sources](docs/corpus_sources.md)
- [curation workflow](docs/curation_workflow.md)
- [source classification](docs/source_classification.md)
- [source scoring](docs/source_scoring.md)
- [tokenizer policy](docs/tokenizer_policy.md)
- [tokenizer experiment plan](docs/tokenizer_experiment_plan.md)
- [tokenizer dry run](docs/tokenizer_dry_run.md)
- [tokenizer segmentation eval](docs/tokenizer_segmentation_eval.md)
- [evaluation policy](docs/evaluation_policy.md)
- [training baseline](docs/training_baseline.md)

## Out of scope (foundation phase)

- Multilingual expansion
- Speech / multimodal
- Cloud platform work
- Chat product features

The repo grows from clean data and hard evaluation, not from broad claims.

## License

Business Source License 1.1. Converts automatically to Apache License 2.0 on **2029-01-01**.
See [LICENSE](LICENSE) for full terms.

Non-commercial and research use is unrestricted today. Commercial use is permitted unless it competes directly with Qazna Technologies LLP products or services.

Copyright © 2024–2026 Qazna Technologies LLP.

For commercial licensing inquiries: hello@qazaq.ai
