# v0.7.0+ — Predictable Dialog Layer

Status: **design document**, pre-implementation. Aligns the final stretch of work (v0.7.0 → v1.0.0) around the user's directive: "we must know the input, know how it thinks through every layer, know the output."

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

## 5. Intent taxonomy (v0.7.0 initial set)

Target 15–25 intents for first cut. Each intent is a struct holding the extracted entities.

| intent | user-side example | entities |
|---|---|---|
| `Greeting` | "сәлем", "сәлеметсіз бе", "қайырлы таң" | salutation_kind |
| `Farewell` | "сау бол", "кездескенше" | — |
| `SelfIntroduction` | "менің атым Айбек" | person_name |
| `AskName` | "сенің атың қалай" | — |
| `AskWhere` | "қайдасың", "қайда тұрасың" | subject_person |
| `StatementOfLocation` | "мен қалада тұрамын" | subject_person, location_root |
| `AskHowAreYou` | "қалай сіз", "жағдайыңыз қалай" | — |
| `StatementOfWellbeing` | "жақсымын", "жаман емеспін" | valence |
| `Affirmation` | "иә", "дұрыс", "рас" | — |
| `Negation` | "жоқ", "қате" | — |
| `Thanks` | "рахмет", "көп рахмет" | — |
| `Apology` | "кешіріңіз", "ғафу етіңіз" | — |
| `AskAge` | "нешеге келдің" | subject_person |
| `StatementOfAge` | "мен жиырма жастамын" | subject_person, age_numeral |
| `AskWhatDoing` | "не істеп жатырсың" | subject_person |
| `StatementOfAction` | "оқып жатырмын" | subject_person, verb_root |
| `Unknown` | anything else | raw_input |

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

## 9. Coverage strategy toward v1.0.0

| milestone | intents | templates | rough dialog range |
|---|---|---|---|
| v0.7.0 | 5 (greeting, farewell, affirmation, negation, unknown) | 10 | trivial 2-turn |
| v0.7.5 | 15 | 40 | social intro + basic Q&A |
| v0.8.0 | 25 | 80 | full MVP dialog about weather, food, family, daily |
| v0.9.0 | 25 + corpus polish | 120 | demo-ready |
| v1.0.0 | 30+ | 150+ | investor demo |

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

## 12. File layout

```
crates/
  adam-dialog/
    Cargo.toml                — depends on adam-kernel-fst
    src/
      lib.rs                  — pub API: Intent, interpret, respond
      semantics.rs            — parse → intent rules
      planner.rs              — intent → response intent + template
      realiser.rs             — template + slots → (root, features) list
      templates.rs            — template loader (TOML → structs)
      entities.rs             — entity recognisers (name, number, time)
    src/bin/
      adam_chat.rs            — interactive REPL
    tests/
      end_to_end.rs           — 30+ (input, expected_output_set) pairs

data/
  dialog/
    templates/
      greeting.toml
      farewell.toml
      self_intro.toml
      wellbeing.toml
      location.toml
      ... (one file per intent family)
    intent_dataset.json       — labelled test pairs (input → intent)

docs/
  kazakh_grammar/
    07_dialog_architecture.md — THIS file
    08_intent_taxonomy.md     — detailed intent specs (v0.7.0 deliverable)
    09_template_guide.md      — how to author templates
```

## 13. Success definition

**v1.0.0 is shippable to an investor** when:

1. `adam_chat` binary runs on M2 8 GB in under 200 MB RAM.
2. A demo conversation of ≥ 20 turns flows coherently (native speaker blind-judged ≥ 40/50 natural).
3. Every turn's decision can be traced through Layers 1–5 by running a `--trace` flag.
4. The whole system is pure Rust, no Python runtime, no GPU.
5. Grammar errors in output = 0.
6. The test corpus shows ≥ 80 % intent-recognition coverage.

These six bullets are the gate. Below them: beta / alpha in the 0.x series. Above them: v1.0.0.
