# Predictable Dialog Layer

Status: **shipped in v1.0.0** (originally drafted pre-v0.7.0 as a design document, updated progressively from v0.7.0 through v0.9.9; this page now describes the as-built system). Aligned with the user's directive: "we must know the input, know how it thinks through every layer, know the output."

The dialog pipeline is the **L1 product of `adam-dialog`**. Its commitments:

- Every stage is deterministic or samples from a finite, inspectable set.
- For any input, `adam_chat --trace` dumps every layer's state.
- No morphologically invalid word can leave the system — output flows through the FST synthesiser.
- Runs on a MacBook Air M2 8 GB with no GPU.
- Built on a 14 k-entry curated pre-modern Kazakh Lexicon, not translated from English.

## 1. Non-goals

- Matching the fluency of trillion-token LLMs.
- Multi-turn coherence over long contexts.
- Handling arbitrary topics.
- Probabilistic reasoning that emits garbage when out-of-distribution.

## 2. Goals

- **Predictable**: every stage is deterministic OR samples from a finite, inspectable set.
- **Auditable**: for any input, a developer can trace the full decision path.
- **Grammatically correct by construction**: because output goes through the FST synthesiser, no morphologically invalid word can leave the system.
- **Small**: fits on the user's MacBook Air M2 8 GB. No GPU.
- **Kazakh-native**: built on the proto-Kazakh lexicon, not translated from English.

## 3. Pipeline

```
┌────────────────────────────────────────────────────────────────┐
│  INPUT   user_text: String                                     │
└──────────────────────────┬─────────────────────────────────────┘
                           │
                           ▼
┌────────────────────────────────────────────────────────────────┐
│  LAYER 1 — Morphological parser  (adam-kernel-fst::parser)     │
│                                                                │
│  input:  "Мен мектептеміз"                                     │
│  output: [ (мен,   Pronoun, Nom),                              │
│            (мектеп, Noun,   {Plural, Loc, 1PlPoss}) ]          │
│                                                                │
│  determinism: FSM over root lexicon + morphology rules         │
│  complexity:  O(n) per word, O(n·lexicon) for ambiguity        │
└──────────────────────────┬─────────────────────────────────────┘
                           │
                           ▼
┌────────────────────────────────────────────────────────────────┐
│  LAYER 2 — Semantic interpreter  (adam-dialog::semantics)      │
│                                                                │
│  input:  [(root, features)*]                                   │
│  output: IntentClass + EntitySet                               │
│                                                                │
│  example:                                                      │
│    input:  [мен/Pro/Nom, мектеп/N/{PL,Loc,P1Pl}]               │
│    output: Intent::StatementOfLocation {                       │
│              subject: Person::First(Plural),                   │
│              location_root: "мектеп",                          │
│            }                                                   │
│                                                                │
│  determinism: hand-written rules over the (root, features)     │
│  sequence. NO neural net. Rule order is inspectable.           │
└──────────────────────────┬─────────────────────────────────────┘
                           │
                           ▼
┌────────────────────────────────────────────────────────────────┐
│  LAYER 3 — Dialog planner  (adam-dialog::planner)              │
│                                                                │
│  input:  IntentClass + EntitySet + state-so-far                │
│  output: (response_intent, response_slots)                     │
│                                                                │
│  example:                                                      │
│    input:  StatementOfLocation{mentions(мектеп)}               │
│    output: ResponseIntent::CommentLocation {                   │
│              template_id: "location_comment/school_is_good",   │
│              slots: {adj_root: "жақсы"},                       │
│            }                                                   │
│                                                                │
│  determinism: deterministic table lookup with a controlled     │
│  stochastic choice among ≤ 5 applicable templates per intent.  │
│  The stochastic choice is the ONLY randomness in the pipeline  │
│  — everything else is a function.                              │
└──────────────────────────┬─────────────────────────────────────┘
                           │
                           ▼
┌────────────────────────────────────────────────────────────────┐
│  LAYER 4 — Response realiser  (adam-dialog::realiser)          │
│                                                                │
│  input:  template_id + slots                                   │
│  output: [(root, features)*]                                   │
│                                                                │
│  Each template is a linearised list of                         │
│  (root_slot_or_literal, feature_bundle) pairs.                 │
│                                                                │
│  example: template "location_comment/school_is_good" =         │
│    [ (slot("adj_root"),  Adjective),                           │
│      (literal("мектеп"), Noun + Copula_Predicative),           │
│    ]                                                           │
│  with slots { adj_root: "жақсы" } → [                          │
│    (жақсы,  Adj),                                              │
│    (мектеп, Noun + Copula_Predicative),                        │
│  ]                                                             │
└──────────────────────────┬─────────────────────────────────────┘
                           │
                           ▼
┌────────────────────────────────────────────────────────────────┐
│  LAYER 5 — Morphological synthesiser  (adam-kernel-fst)        │
│                                                                │
│  input:  [(root, features)*]                                   │
│  output: surface word-forms                                    │
│                                                                │
│  determinism: FST from v0.4.5                                  │
│  guarantees grammatical output by construction                 │
└──────────────────────────┬─────────────────────────────────────┘
                           │
                           ▼
┌────────────────────────────────────────────────────────────────┐
│  OUTPUT  response_text: String                                 │
└────────────────────────────────────────────────────────────────┘
```

## 4. Why this is not a chatbot lottery

- Every layer is a function, except Layer 3 which picks among ≤ 5 applicable templates.
- That single stochastic choice is bounded and inspectable — a user can enumerate every response the system could give for a given intent.
- There is no free generation, no "hallucinate a fact", no unbounded attention over arbitrary text.
- If the parser doesn't recognise the input, we emit a clearly-tagged `Intent::Unknown` and respond with a fallback ("түсінбедім"), not with a confabulation.

## 5. Intent taxonomy (v1.0.0 shipped set — 25 intents)

Each intent is a variant of `adam_dialog::Intent`. Variants that carry a payload use `Option<T>` so the intent matches on keywords alone (`years = None`) even when the entity wasn't extractable.

| intent | Kazakh trigger example | entity payload |
|---|---|---|
| `Greeting::Casual` | сәлем | — |
| `Greeting::Polite` | сәлеметсіз бе | — |
| `Greeting::TimeOfDay(Morning)` | қайырлы таң | — |
| `Greeting::TimeOfDay(Day)` | қайырлы күн | — |
| `Greeting::TimeOfDay(Evening)` | қайырлы кеш | — |
| `Farewell` | сау бол, кездескенше | — |
| `Affirmation` | иә, дұрыс, рас, мақұл | — |
| `Negation` | жоқ, қате, емес | — |
| `Thanks` | рахмет, көп рахмет | — |
| `Apology` | кешіріңіз, ғафу ет | — |
| `AskHowAreYou` | қалайсыз, жағдайыңыз қалай | — |
| `StatementOfWellbeing` | жақсымын, жаман емеспін | — |
| `AskName` | атың кім, есімің қалай | — |
| `StatementOfName { name }` | менің атым Дәулет | `name: String` |
| `AskAge` | жасың неше | — |
| `StatementOfAge { years }` | менің жасым отыз | `years: Option<u32>` |
| `AskLocation` | қайда тұрасыз | — |
| `StatementOfLocation { city }` | мен Алматыданмын | `city: Option<String>` |
| `AskOccupation` | немен айналысасың | — |
| `StatementOfOccupation { occupation }` | мен мұғаліммін | `occupation: Option<String>` |
| `AskFamily` | балаларың бар ма | — |
| `StatementOfFamily` | менің балам бар | — |
| `AskWeather` | ауа райы қалай | — |
| `StatementOfWeather` | бүгін суық | — |
| `AskTime` | сағат неше | — |
| `Compliment` | жарайсың, керемет | — |
| `Request` | өтінемін, көмектесіңізші | — |
| `WellWishes` | сәттілік, жақсы күн тілеймін | — |
| `Unknown { raw_tokens }` | anything else | raw tokens |

Every intent also accepts Russian and English trigger phrasings (see [recogniser surface](#recogniser-surface)). Triggers map to the same Intent; the response is always Kazakh.

## 6. Data structures (Rust sketch)

```rust
// Lives in crate `adam-dialog`.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    Greeting { salutation: GreetingKind },
    Farewell,
    SelfIntroduction { person_name: String },
    AskName,
    AskWhere { subject: SubjectPerson },
    StatementOfLocation { subject: SubjectPerson, location_root: String },
    // ...
    Unknown { raw_tokens: Vec<String> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubjectPerson {
    First(Number),
    Second(Number, Politeness),
    Third(Number),
}

pub fn interpret(
    parses: &[adam_kernel_fst::parser::Analysis],
) -> Intent {
    // rule-based dispatch over the parse sequence
    // e.g. if parses start with a question-word pronoun {қайда/қалай/кім/не}
    // → it's a question intent; pick which one by the main verb.
}
```

## 7. Template repository

Stored as TOML or JSON in `data/dialog/templates/*.toml`. Each file is one intent family with 2–8 alternative realisations.

```toml
# data/dialog/templates/statement_of_wellbeing.toml
intent = "StatementOfWellbeing"

[[templates]]
id = "wellbeing/good_thanks"
slots = []
atoms = [
  { root = "жақсы", features = "Adj" },
  { root = "__dummy__", features = "VerbCop_1SgPres" },  # -мын
  { literal = "," },
  { root = "рахмет", features = "Noun" },
]
# realises to: "жақсымын, рахмет"

[[templates]]
id = "wellbeing/neutral"
slots = []
atoms = [
  { root = "жаман", features = "Adj" },
  { root = "емес", features = "Particle" },
  { root = "__dummy__", features = "VerbCop_1SgPres" },
]
# realises to: "жаман емеспін" (not bad)
```

Key property: the template repository is **plain data** the user / reviewer can read, audit, and extend. No embedded code. No learned weights.

## 8. Stochastic component

The planner's ONLY randomness is `choose_template(intent, applicable_templates)`:

```rust
fn choose_template(
    intent: &Intent,
    applicable: &[&Template],
    rng: &mut impl Rng,
) -> &Template {
    // Uniform pick from the applicable set. Seed-able for reproducibility.
    &applicable[rng.gen_range(0..applicable.len())]
}
```

This is the single point where "predictability" has a bounded exception. We argue that is not "unpredictable" in the LLM sense — for any input, the set of possible outputs is enumerable and finite. A test can pin the RNG seed to make it a pure function.

## 9. Coverage realised (v0.7.0 → v1.0.0)

| release | intents | template families | capability delta |
|---|---|---|---|
| v0.7.0 | 5 | 8 (hardcoded in planner) | first pipeline, trivial 2-turn |
| v0.7.5 | 10 | 14 (TOML) | social politeness surface |
| v0.8.0 | 25 | 25 | full MVP social-topic surface; first slot expansion (`{name}`) |
| v0.8.5 | 25 | 29 | session state, greeting `{name}` variants |
| v0.9.0 | 25 | 29 | `{age}`, `{city}`, `{occupation}` slots with entity absorption |
| v0.9.5 | 25 | 29 | `{slot\|features}` FST-backed expansion + cross-slot templates |
| v0.9.6 | 25 | 29 | trilingual recogniser (kk / ru / en) |
| v0.9.7 | 25 | 29 | Lexicon-backed occupation recognition |
| v0.9.8 | 25 | 29 | full slot syntax (case + number + derivation + possessive) + transliteration |
| v0.9.9 | 25 | 29 | FST Instrumental harmony fix + phrasing polish |
| **v1.0.0** | **25** | **29** | **MVP cut** — no new features; documentation refresh |

## Recogniser surface

| intent family | Kazakh triggers | Russian triggers | English triggers |
|---|---|---|---|
| Greeting | сәлем / сәлеметсіз бе / қайырлы таң/күн/кеш | привет / здравствуйте / доброе утро / добрый день / добрый вечер | hi / hello / hey / good morning / good afternoon / good evening / good day |
| Farewell | сау бол / кездескенше / қош | до свидания / пока | bye / goodbye / see you |
| Affirmation | иә / дұрыс / рас / мақұл | да / конечно / ага | yes / yeah / yep / sure / ok |
| Negation | жоқ / қате / емес | нет | no / nope / nah |
| Thanks | рахмет / көп рахмет | спасибо / большое спасибо | thanks / thank you |
| Apology | кешіріңіз / ғафу ет | извини / извините / прости | sorry / excuse me |
| AskHowAreYou | қалайсың / қалайсыз / жағдайыңыз қалай | как дела / как ты / как вы | how are you / how's it |
| AskName | атың кім / есіміңіз қалай | как тебя / вас зовут | what is / what's your name |
| StatementOfName | атым X / мені X деп атайды / есімім X | меня зовут X / моё имя X | my name is X / call me X / hi i am X |
| AskAge | жасың неше | сколько тебе / вам лет | how old are you |
| StatementOfAge | жасым N / N жастамын | — | — |
| AskLocation | қайда тұрасыз / қай жерден / қайдан | откуда ты / вы, где живёшь | where are you from / where do you live |
| StatementOfLocation | X-данмын / X-да тұрамын | — | — |
| AskOccupation | немен айналысасың / жұмысың не | кем работаешь / чем занимаешься | what do you do / what's your job |
| StatementOfOccupation | X-мын / X-пын / X-бын (Lexicon-backed noun stripping) | — | — |
| AskFamily | балаларың бар ма / отбасың бар ма | — | — |
| AskWeather | ауа райы қалай | какая погода | how's / what's the weather |
| AskTime | сағат неше | сколько времени / который час | what time is it / what's the time |
| Compliment | жарайсың / керемет / тамаша | молодец / отлично / здорово | great / awesome / wonderful / well done |
| Request | өтінемін / көмектесіңізші | пожалуйста / помогите / помоги | please / need help / can you help |
| WellWishes | сәттілік / жақсы күн тілеймін | удачи / всего наилучшего | good luck / all the best |

## 10. Measurement

Dialog quality for this architecture is measured by:

- **Coverage rate**: fraction of user inputs (drawn from a test corpus of 500 Kazakh one-turn exchanges) that are parsed → recognised → responded to without falling through to `Unknown`.
- **Grammaticality of responses**: 100 % by construction (FST synthesiser).
- **Template diversity**: average number of applicable templates per recognised intent (higher is better; prevents robotic repetition).
- **Review by native speaker**: N out of 50 responses judged "natural-sounding".

Target for v1.0.0: ≥ 80 % coverage, ≥ 4 templates/intent average, ≥ 40/50 naturalness score.

## 11. Deliberately out of scope for v1.0.0

- Reasoning: no chain-of-thought, no question-answering-from-facts.
- Memory: every turn is stateless except for the conversation history seen by Layer 2.
- Factual knowledge: system does not know "what year is it" or "what is the capital of France". All responses are social / locutionary.
- Instruction-following beyond the enumerated intents.

This is the smallest possible system that can plausibly hold a short conversation in Kazakh while remaining 100 % auditable. It is NOT a general-purpose assistant. It IS a foundation that an LM layer can later augment without replacing — because the structural guarantees (grammar, intent, template) continue to hold as a safety net.

## 12. File layout (v1.0.0 as-built)

```
crates/
  adam-dialog/
    Cargo.toml                — depends on adam-kernel-fst + toml + serde
    src/
      lib.rs                  — pub API: respond, respond_with_repo, Conversation,
                                plan_response_with_session, interpret_text_with_lexicon, …
      semantics.rs            — intent recognisers + numeral parser +
                                PersonName/city/occupation extractors
      intent.rs               — 25-variant Intent enum + NounFeatures payload types
      planner.rs              — intent_key mapping, extract_slots, template-fillability
                                filter, plan_response_with_session
      realiser.rs             — {slot|features} expansion via synthesise_noun
      templates.rs            — TemplateRepository loader (TOML)
      slot_syntax.rs          — parse_placeholder + parse_noun_features (v0.9.5+)
      transliteration.rs      — Latin → Cyrillic for foreign names (v0.9.8+)
      conversation.rs         — multi-turn session state
    src/bin/
      adam_chat.rs            — interactive REPL with --once / --trace
    tests/
      end_to_end.rs           — 81 (input, expected_output_set) + multi-turn tests

data/
  dialog/
    templates/
      v1.toml                 — single TOML file, 29 template families
    README.md                 — schema + versioning

docs/
  kazakh_grammar/
    07_dialog_architecture.md — THIS file
```

The "one file per intent family" plan was simplified to a single `v1.toml` during v0.7.5 — schema evolution will use `v2.toml` etc. alongside (see `data/dialog/README.md`).

## 13. Success definition — v1.0.0 delivered

**v1.0.0 ships when:**

1. `adam_chat` runs on M2 8 GB in under 200 MB RAM — ✅ delivered
2. Predictable multi-turn conversation across 25 intents — ✅ delivered
3. Every turn's decision traceable through Layers 1–5 via `--trace` — ✅ delivered
4. Pure Rust, no Python runtime, no GPU — ✅ delivered
5. Grammar errors in output = 0 (FST-guaranteed) — ✅ delivered
6. Trilingual input recogniser (kk / ru / en) — ✅ delivered (v0.9.6+)
7. Regression test suite covers every layer — ✅ **271 passing**, 0 failing

Native-speaker naturalness review (originally target 40/50) is queued for post-v1.0.0; the v0.9.9 polish pass is an in-team inspection, not a formal review. This does not gate the MVP cut.
