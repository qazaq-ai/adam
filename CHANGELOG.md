# Changelog

All notable changes are tagged in git as `vX.Y.Z`. Versions before 0.1.0 are foundation work — APIs, schemas, and rules may change between any two releases.

## [0.9.9] — 2026-04-19

Morphology correctness pass + template phrasing polish. The last stretch before the v1.0.0 MVP cut.

### FST Instrumental fix (two bugs, one mechanism)

The `INSTRUMENTAL` suffix template previously used the harmony-alternating archiphoneme `{E}`, but Kazakh Instrumental is actually invariant in vowel — always `-мен/-бен/-пен`, never `-ман/-бан/-пан`. Replaced with a literal `е`:

```diff
- const INSTRUMENTAL: SuffixTemplate = &[Arch(M), Arch(E), Literal('н')];
+ const INSTRUMENTAL: SuffixTemplate = &[Arch(M), Literal('е'), Literal('н')];
```

Separately, `realise_m` flipped `Nasal → 'б'` which produced `мұғалімбен`. Fixed to `Nasal → 'м'`, giving standard `мұғаліммен`. Voiced obstruent → `б` (rare but preserved).

Before / after samples:

| root | pre-v0.9.9 | v0.9.9 |
|---|---|---|
| Алматы | Алматыман ❌ | Алматымен ✓ |
| Астана | Астанаман ❌ | Астанамен ✓ |
| мұғалім | мұғалімбен ❌ | мұғаліммен ✓ |
| Джохн | Джохнбан ❌ | Джохнмен ✓ |
| Дәулет | Дәулетпен ✓ | Дәулетпен ✓ |
| мектеп | мектеппен ✓ | мектеппен ✓ |

### Cleanup

`Archiphoneme::E` and `realise_e` were only used by the Instrumental template and are now dead. Removed both per YAGNI.

### FST regression tests

Added 6 unit tests to `morphotactics` covering every consonant-class path of the new Instrumental + the back/front vowel invariance. These lock in the fix so future archiphoneme refactors can't re-break it.

- `noun_instrumental_front_consonant_final` (Дәулет → пен)
- `noun_instrumental_back_consonant_final` (Джохн → мен)
- `noun_instrumental_back_vowel_stem_stays_е_not_а` (Алматы → мен, regression)
- `noun_instrumental_vowel_final_stem` (бала, тау → мен)
- `noun_instrumental_voiceless_final_gives_пен` (мектеп → пен)
- `noun_instrumental_nasal_final_gives_мен_not_бен` (мұғалім → мен, regression)

### Template polish pass

Dropped awkward / filler templates and replaced with context-specific acknowledgements:

| key | removed | added |
|---|---|---|
| `statement_of_age` | `түсіндім`, `жасыңыз келісті` | `қуатты кезеңіңіз` |
| `statement_of_location` | `түсіндім` | `тамаша өлке` |
| `statement_of_occupation` | `мақтанышпен` (solo) | `мақтанатын жұмыс` |
| `statement_of_weather` | `түсіндім` | `табиғат мезгіліне лайық` |

"түсіндім" as a solo response felt generic/repetitive. Replaced with phrases that match the topic of the user's statement.

### Numbers

- **FST unit tests:** 84 (was 78) — 6 new Instrumental regressions
- **Dialog end-to-end tests:** 81 (unchanged, assertion sets updated)
- **Workspace tests:** **271 passing**, 4 ignored, 0 failing
- **Foundation CI:** passing

### Known v0.9.9 limitations

- Silent English `h` still not special-cased in transliteration (`John → Джохн` rather than the conventional `Джон`). Cosmetic; FST synthesis works fine on either.
- Native-speaker review has NOT been conducted — the polish was a phrasing pass by inspection, not formal review. A real review is queued for post-v1.0.0 refinement.

## [0.9.8] — 2026-04-19

Slot syntax completes the noun-feature surface (Derivation + Possessive), Latin names get transliterated before FST synthesis, and templates gain a layer of cross-slot personalisation that uses multiple remembered entities in a single response.

### Slot syntax: full noun-feature coverage

Adds 11 derivation tokens and 7 possessive tokens to `parse_noun_features`:

```text
{root|agent}           → Agent (-шы/-ші)
{root|abstract}        → Abstract (-лық/-лік)
{root|privative}       → Privative (-сыз/-сіз)
{root|endowed}         → Endowed (-лы/-лі)
{root|similative}      → Similative (-дай/-дей)
{root|comparative}     → Comparative (-рақ/-рек)
{root|verbalnoun}      → VerbalNoun (-у)
{root|actionnoun}      → ActionNoun (-ым/-ім)
{root|diminutive}      → Diminutive (-шық/-шік)
{root|ordinal}         → Ordinal (-ншы/-нші)
{root|collective}      → Collective (-еу/-ау)

{root|p1sg}            → P1Sg (my)
{root|p2sg}            → P2SgPolite (your, polite default)
{root|p2sg_inf}        → P2SgInformal
{root|p3}              → P3 (his/her)
{root|p1pl}            → P1Pl (our)
{root|p2pl}            → P2PlPolite
{root|p2pl_inf}        → P2PlInformal
```

Combinations work as you'd expect: `{name|agent+p1sg+dative}` yields root → Agent derivation → P1Sg possessive → Dative case, all in one synthesis pass.

### Latin → Cyrillic transliteration

New module `adam_dialog::transliteration` converts Latin proper names to Kazakh Cyrillic BEFORE `synthesise_noun` when a template requests morphology on a non-Cyrillic slot value. v0.9.6 guarded against garbled output by falling back to plain substitution; v0.9.8 replaces that fallback with real transliteration so FST inflection actually runs on foreign names.

| Latin input | transliterated |
|---|---|
| `Anna` | Анна |
| `Tom` | Том |
| `John` | Джохн |
| `Zhanna` | Жанна |
| `Sharon` | Шарон |
| `Charlie` | Чарлие |

Conservative single-letter + digraph mapping: `sh/ch/zh/kh/gh/ph/th/ts/yo/ya/yu/ye` as digraphs, rest letter-by-letter (`j → дж`, `c → к`, `x → кс`, `y → й`). Silent `h` in English is **not** special-cased (`John → Джохн`, not `Джон`) — intentionally conservative.

**Policy:** plain `{name}` substitution still keeps the user's original spelling ("сәлем John"). Only `{name|features}` triggers transliteration → synthesis.

### Cross-slot templates

New templates reference multiple session entities in one response. Eligible only when every slot is fillable; plain variants stay available otherwise.

| key | new templates |
|---|---|
| `ask_how_are_you` | `"жақсымын {name}, ал сіз қалайсыз"`, `"жақсымын, рахмет {name}"` |
| `statement_of_age` | `"{name}, {age} жастасыз, тамаша"`, `"{name}, {age} жас — керемет кезең"` |
| `statement_of_occupation` | `"{name}, {occupation} — құрметті кәсіп"`, `"{name}, сіз {city|locative} {occupation} екенсіз"` |
| `compliment` | `"рахмет {name}"`, `"рахмет {name}, сіз де тамашасыз"` |

The triple-slot `"{name}, сіз {city|locative} {occupation} екенсіз"` only fires after the user has stated all three entities — "Дәулет, сіз Алматыда әнші екенсіз".

### Tests

81 dialog end-to-end pairs (up from 78), 3 new cross-slot tests covering (name+ask_how_are_you), (name+age), and (name+city+occupation triple). 23 lib-level unit tests (13 slot_syntax + 6 transliteration + 4 planner).

Workspace: **265 passing**, 4 ignored, 0 failing. Foundation CI green.

### Known v0.9.8 limitations

- Silent `h` in English is not special-cased (`John → Джохн` rather than the standard spelling `Джон`). Good enough for FST synthesis; a more polished transliterator is v1.0.0+ work.
- Back-vowel instrumental harmony (`Алматы` → `Алматыман` rather than `Алматымен`) is a pre-existing FST quirk — check `INSTRUMENTAL` template archiphoneme `E` resolution. Out of scope for the dialog layer.
- Derivation + Possessive tokens are fully parseable; the current template set uses only a handful of the 18 feature tokens. Template authors have the full surface available when needed.

## [0.9.7] — 2026-04-19

Lexicon-backed occupation recognition. The fixed 6-form table (`мұғаліммін → мұғалім` and five others) is replaced with generic 1sg-copula stripping + noun lookup against the 14 k-entry Lexicon. Any noun in the Lexicon ending in a 1sg predicate suffix (`-мын/-мін/-пын/-пін/-бын/-бін`) is now recognised.

### What now works

```
$ adam_chat
> мен ақынмын           → сіз ақын екенсіз            (new: ақын is in Lexicon, not in the old table)
> мен әншімін           → әншілер — қажетті мамандық  (FST plural on the new extract)
> мен ғалыммын          → сіз ғалым екенсіз
> мен суретшімін        → сіз суретші екенсіз
> жақсымын              → жақсы екен                  (POS filter → wellbeing, not occupation)
```

### Public API additions

- `adam_dialog::interpret_text_with_lexicon(input, parses, Option<&LexiconV1>) -> Intent`
- `adam_dialog::semantics::interpret_text_with_lexicon` (module-level)

The original `interpret_text(input, parses)` is now a thin wrapper that calls the lexicon-aware variant with `None` — existing callers keep working.

### Implementation detail

```rust
fn strip_copula_and_lookup_noun(tokens: &[String], lex: &LexiconV1) -> Option<String> {
    const COPULA_SUFFIXES: &[&str] = &["мын", "мін", "пын", "пін", "бын", "бін"];
    for t in tokens {
        for suffix in COPULA_SUFFIXES {
            let Some(root) = strip_suffix_chars(t, suffix) else { continue };
            if root.chars().count() < 2 { continue; }
            if let Some(entry) = lex.get(&root) {
                if entry.part_of_speech == "noun" {
                    return Some(root);
                }
            }
        }
    }
    None
}
```

- **POS filter** rejects adjectives (`жақсы`, `жаман`) so "жақсымын" still routes to StatementOfWellbeing.
- **Min-length 2** guards against stripping into short function words.
- **Char-count indexing** keeps UTF-8 boundaries safe.

`respond`, `respond_with_repo`, and `Conversation::turn` all pass the lexicon into the new recogniser automatically.

### Tests

78 dialog end-to-end pairs (up from 73), 5 new:
- 1 positive case covering `ақын` (out-of-table noun)
- 1 bulk test for `әнші / ғалым / суретші`
- 1 adjective-negative-case ensuring `жақсымын` stays wellbeing
- 1 unknown-root case (`xyzzyмын` → must not become occupation)
- 1 multi-turn absorption test (lexicon-derived occupation persists to session)

Workspace: **251 passing**, 4 ignored, 0 failing. Foundation CI green.

### Known v0.9.7 limitations

- Latin→Cyrillic transliteration is intentionally NOT shipped. Latin names ("John") continue to bypass `{name|features}` FST synthesis via the v0.9.6 safety guard. Transliteration lands in v0.9.8 alongside broader slot-syntax enrichment.
- Lexicon coverage is the cap — occupations not in the 14 k-entry Lexicon (`философ`, `программист`) still don't extract. Data-layer expansion is orthogonal work.

## [0.9.6] — 2026-04-19

Multilingual recogniser surface. The model now reads Kazakh, Russian, and English input across all 25 intents and replies exclusively in Kazakh. This is NOT translation — the core pipeline stays deterministic Kazakh-only. The expansion is purely at the recogniser layer: more surface forms map to the same Intent taxonomy.

### Triggers added (per intent)

- **Greeting** (casual/polite/time-of-day): `hi/hello/hey`, `привет`, `здравствуйте`, `доброе утро/день/вечер`, `good morning/afternoon/evening/day`
- **Farewell**: `bye/goodbye/see you`, `до свидания/пока`
- **Affirmation**: `yes/yeah/yep/sure/ok`, `да/конечно/ага`
- **Negation**: `no/nope/nah`, `нет`
- **Thanks**: `thanks/thank you`, `спасибо/большое спасибо`
- **Apology**: `sorry/excuse me`, `извини/извините/прости`
- **AskHowAreYou**: `how are you/how's it`, `как дела/как ты/как вы`
- **StatementOfWellbeing**: `fine/great/i'm good/i'm fine`, `хорошо/нормально/отлично`
- **AskName**: `what is/what's your name`, `как тебя/вас зовут`
- **StatementOfName**: four new patterns —
  - `meня зовут <N>`, `моё имя <N>` (Russian)
  - `my name is <N>`, `call me <N>`, `hi i am <N>` (English; bare "I am X" is ambiguous so requires a leading greet token)
- **AskAge**: `how old are you`, `сколько тебе/вам лет`
- **AskLocation**: `where are you from / where do you live`, `откуда ты/вы`
- **AskOccupation**: `what do you do / what's your job`, `кем работаешь/занимаешься`
- **AskWeather**: `how's / what's the weather`, `какая погода`
- **AskTime**: `what time is it / what's the time`, `сколько времени/который час`
- **Compliment**: `great/awesome/wonderful/excellent/well done`, `молодец/отлично/здорово`
- **Request**: `please/need help/can you help`, `пожалуйста/помогите/помоги`
- **WellWishes**: `good luck/all the best`, `удачи/всего наилучшего`

### Safety guard for Latin roots

FST phonology is tuned for Kazakh Cyrillic. Feeding `"John"` into `synthesise_noun(..., Case::Instrumental)` would produce garbled `"Johnман"`. The realiser now detects non-Cyrillic roots and falls back to plain substitution — no suffix attached. Output: `"John танысқаныма қуаныштымын"` rather than hallucinated morphology.

### Ordering change

`StatementOfName` is now checked BEFORE `Greeting` in `interpret_text`. This prevents `"hi i am John"` from misfiring as a bare Casual greeting. All StatementOfName patterns (атым/есімім/зовут/my name is/call me/[greet] i am X) are explicit enough to rule out false positives.

### Tests

73 dialog end-to-end pairs (up from 56), 17 new:
- 10 recogniser triggers (greetings × 3, farewell, affirmation, negation, thanks, apology, ask-how-are-you, ask-name)
- 4 self-introduction patterns (Russian `зовут`, English `my name is` / `call me` / `hi i am`)
- 2 output-is-Kazakh invariants (Russian input → Cyrillic-only output; Latin name → no FST suffix)
- 1 multilingual multi-turn conversation flow

Workspace: **245 passing**, 4 ignored, 0 failing. Foundation CI green.

### Known v0.9.6 limitations

- Recogniser catches the common phrasings. Edge cases (British contractions "init", ru-ua mix, Kazakh with Latin transliteration "salem") are not handled.
- Latin names stay un-inflected in templates requesting `{name|features}`. Transliteration to Cyrillic (e.g. `John` → `Джон`) would let the FST synthesise properly — possible future work.
- No output-language switching: Russian / English input still gets Kazakh output by design.

## [0.9.5] — 2026-04-19

FST-backed slot expansion. Templates can now emit `{slot|features}` atoms; the realiser synthesises grammatical forms via `adam_kernel_fst::morphotactics::synthesise_noun` instead of plain text substitution. Cross-slot templates (using multiple slots in one response) drop in naturally because of the v0.8.5 template-fillability filter.

### New slot syntax

```text
{slot}                    — plain: substitute slot value verbatim
{slot|feat1+feat2+...}    — FST: synthesise via morphotactics
```

Feature tokens (case-insensitive, `+`-separated, unknown tokens ignored):

| token | → field |
|---|---|
| `nominative / nom` | `case = Nominative` |
| `genitive / gen` | `case = Genitive` |
| `dative / dat` | `case = Dative` |
| `accusative / acc` | `case = Accusative` |
| `locative / loc` | `case = Locative` |
| `ablative / abl` | `case = Ablative` |
| `instrumental / inst` | `case = Instrumental` |
| `singular / sg` | `number = Singular` |
| `plural / pl` | `number = Plural` |

### Examples of what now works

| template | filled | rendered |
|---|---|---|
| `{city\|locative} тұрасыз ба` | city=Алматы | Алматыда тұрасыз ба |
| `{city\|ablative} хабар жақсы ма` | city=Алматы | Алматыдан хабар жақсы ма |
| `{name\|instrumental} танысқаныма қуаныштымын` | name=Дәулет | Дәулетпен танысқаныма қуаныштымын |
| `{occupation\|plural} — қажетті мамандық` | occupation=мұғалім | мұғалімдер — қажетті мамандық |
| `сәлем {name}, {city\|ablative} хабар жақсы ма` | name=Дәулет, city=Алматы | сәлем Дәулет, Алматыдан хабар жақсы ма |

The last one is a cross-slot template: the planner only considers it when BOTH `name` and `city` are in session.

### Public API additions

- `adam_dialog::slot_syntax::{parse_placeholder, parse_noun_features}`

### TOML changes (v1.toml version → 0.9.5)

- `greeting.casual`, `greeting.polite` each get a cross-slot `{name}+{city|abl/loc}` variant.
- `statement_of_name` gets `{name|instrumental}` variants.
- `statement_of_location` gets 3 FST-backed variants: locative / ablative / dative.
- `statement_of_occupation` gets plural + dative variants.

### Tests

56 dialog end-to-end pairs (up from 52), 4 new covering every FST-backed expansion path + the cross-slot greeting. 7 slot-syntax unit tests + 1 additional planner unit test. 1 doc-test.

Workspace: **229 passing**, 4 ignored, 0 failing.

### Known v0.9.5 limitations

- Feature parser covers noun `case + number` only. Derivation and possessive are v1.0.0 additions.
- Occupation recogniser still uses the fixed 6-form table; generic 1sg-copula stripping via FST lookup is queued for v0.9.8.
- No verb slot expansion — `{root|verb_features}` would need a different synthesiser dispatch.

## [0.9.0] — 2026-04-19

Full entity absorption: every social-topic statement now contributes an extractable entity to session state. Age is parsed from Kazakh numerals (1–99), city from ablative/locative case stripping, occupation from 1sg-copula stripping.

### Intent payload changes (breaking)

- `StatementOfAge` → `StatementOfAge { years: Option<u32> }`
- `StatementOfLocation` → `StatementOfLocation { city: Option<String> }`
- `StatementOfOccupation` → `StatementOfOccupation { occupation: Option<String> }`

`None` means the intent matched on keywords but the entity wasn't parseable ("жасым жасырын").

### Numeral parser

`semantics::parse_kazakh_age` handles:

- Bare tens: он (10), жиырма (20), отыз (30), қырық (40), елу (50), алпыс (60), жетпіс (70), сексен (80), тоқсан (90)
- Bare units: бір (1) … тоғыз (9)
- Compound forms: "отыз бес" (35), "жиырма екі" (22)
- Literal digit strings: "30"

### Entity extraction

- `StatementOfLocation`: strips ablative+copula (`-данмын/-денмін/-танмын/-тенмін`) or locative (`-да/-де/-та/-те`) to recover the city root. Preserves original casing: "Алматыданмын" → "Алматы"; "астанада тұрамын" → "астана".
- `StatementOfOccupation`: matches a fixed table of 1sg-copula forms and emits the stripped noun root: "мұғаліммін" → "мұғалім".

### Session wiring

- `Conversation::absorb_entities` and `planner::extract_slots` both consume the new fields and populate `{age}`, `{city}`, `{occupation}` slots (in addition to `{name}` from v0.8.5).
- Once absorbed, the entities persist across turns just like `{name}` does.

### Templates (TOML v0.9.0)

New personalised variants in `statement_of_age`, `statement_of_location`, `statement_of_occupation`:

- `statement_of_age`: `"{age} жас — тамаша кезең"`, `"жасыңыз {age} екен"`
- `statement_of_location`: `"{city} — әдемі қала"`, `"{city} туралы көп естідім"`
- `statement_of_occupation`: `"{occupation} — құрметті кәсіп"`, `"сіз {occupation} екенсіз"`

Only eligible when the slot can be filled; untouched by templates stay canonical for utterances without extractable entities.

### Tests

52 dialog end-to-end pairs (up from 44), 8 new:

- 3 intent tests covering age numeral parsing (bare, compound, none)
- 2 location extraction tests (ablative + locative)
- 1 occupation extraction test
- 1 multi-turn absorption test (age+city+occupation into session)
- 1 multi-turn personalisation test (numeral appears in response)

Workspace: **215 passing**, 4 ignored, 0 failing.

### Known v0.9.0 limitations

- Occupation extraction uses a fixed 6-form table. Regular 1sg-copula stripping via FST lookup lands in v0.9.5 together with `{root|features}` slot expansion.
- Location extraction is surface-pattern only — no FST lookup yet, so misspelt or inflected cities ("Қызылордаданмын") get a raw root rather than normalised lexicon lemma.
- No cross-slot templates yet (`"{name}, сіз {age} жастасыз ба?"` — possible but unwritten).

## [0.8.5] — 2026-04-19

First session state in the dialog layer. The new [`Conversation`] struct accumulates entities across turns, so a user who introduces themselves once gets greeted by name on every subsequent turn.

- `Conversation { session: HashMap<String, String> }` with `new()`, `turn(input, lex, repo, seed) -> String`, `reset()`.
- `planner::plan_response_with_session(intent, seed, repo, session)` merges per-turn slots with session slots (per-turn wins on collision) and filters candidate templates down to those whose every `{slot}` reference is satisfiable. If filtering empties the pool, falls back to the full pool (visible `{slot}` is better than a crash).
- `plan_response_with_repo` is now defined in terms of `plan_response_with_session(…, &HashMap::new())` — no behaviour change for existing callers.
- Greeting families get `{name}` variants:
  - `greeting.casual`: сәлем / сәлем достым / **сәлем {name}**
  - `greeting.polite`: сәлеметсіз бе / армысыз / **сәлеметсіз бе {name}**
  - `greeting.morning` / `.day` / `.evening` all get a corresponding `{name}` variant.
- `adam_chat` CLI now holds a single `Conversation` for the whole REPL session; `--trace` mode dumps the live session map.
- Ordering: `Conversation::turn` absorbs entities BEFORE planning, so the SAME turn that says "менің атым X" can already receive a response containing `{name}` substituted to X.

Tests: 44 dialog end-to-end pairs (+3 session tests covering persistence, non-persistence when no name said, and `reset()`). 3 planner unit tests for `template_is_fillable`. Workspace: **204 passing**, 4 ignored, 0 failing.

Known v0.8.5 limitations:

- Only `name` is persisted across turns — `age`, `location`, `occupation`, `family` recognition exists but their entities aren't yet extracted into session. That lands in v0.9.0 together with numeric extraction.
- No context-aware responses: the model doesn't say "мен сізді Дәулет деп атадым, иә?" to confirm, or disambiguate "Дәулет" the name from "дәулет" the concept.

## [0.8.0] — 2026-04-19

Dialog layer widened from 10 to **25 intents**. First entity extraction lands: the user's name is pulled out of self-introduction patterns and substituted into the response template.

New intents (+15, recognisers in `semantics.rs`):

- `StatementOfName { name }` — "менің атым X" / "мені X деп атайды" / "есімім X"
- `AskAge` / `StatementOfAge` — жасың неше / менің жасым отыз
- `AskLocation` / `StatementOfLocation` — қайда тұрасыз / мен Алматыданмын
- `AskOccupation` / `StatementOfOccupation` — немен айналысасың / мен мұғаліммін
- `AskFamily` / `StatementOfFamily` — балаларың бар ма / менің балам бар
- `AskWeather` / `StatementOfWeather` — ауа райы қалай / бүгін суық
- `AskTime` — сағат неше
- `Compliment` — жарайсың / керемет / тамаша
- `Request` — өтінемін / көмектесіңізші
- `WellWishes` — сәттілік / жақсы күн тілеймін

Entity extraction + slot expansion:

- `semantics::detect_statement_of_name` extracts the PersonName from three surface patterns (атым / мені X деп атайды / есімім) with case-preserving capitalisation.
- `ResponsePlan` gains a `slots: HashMap<String, String>` field populated by the planner from the Intent.
- `realiser::realise` substitutes `{slot}` placeholders in the chosen template; templates like `"қош келдіңіз {name}"` now personalise.

Ordering subtlety: Statement-of-X is checked BEFORE Ask-of-X in every topic pair — a 1st-person marker ("келдім", "тұрамын", "жасым") unambiguously identifies the user as stating, not asking. Without this, "қайдан келдім" would hit `AskLocation` first (because of "қайдан").

TOML repository: +15 families → 29 families total, version = "0.8.0".

Tests: 41 dialog end-to-end pairs (up from 23), 18 new covering recognition, slot substitution, and planner coverage for every new intent. Workspace: **201 passing**, 4 ignored, 0 failing.

Known v0.8.0 limitations (by design, not bugs):

- No session state: the model doesn't remember the user's name across turns. Adding a `Conversation` struct lands in v0.8.5.
- Numeric extraction (age, time) is a v0.9.0 concern; StatementOfAge templates acknowledge generically.
- Templates are still literal phrases with optional `{slot}` text replacement. FST-backed `{root|features}` atoms land in v0.9.0.
- Templates have not been native-speaker reviewed — stiffness is expected; v0.9.0 tightens phrasing.

## [0.7.5] — 2026-04-19

Dialog layer widened from 5 to **10 intents** and templates externalised to TOML.

New intents (+recognisers in `semantics.rs`):

- `Thanks` — рахмет / көп рахмет / рақмет → оқасы жоқ, ештеңе емес, ризамын
- `Apology` — кешіріңіз / ғафу ет → ештеңе емес, мейлі, түк етпейді
- `AskHowAreYou` — қалайсың / қалайсыз / жағдайыңыз қалай → жақсымын рахмет, жаман емеспін, жақсы ал сіз қалайсыз
- `StatementOfWellbeing` — жақсымын / жаман емес → жақсы екен, қуанамын, ал сіз қалайсыз
- `AskName` — атың кім / есіміңіз қалай → менің атым адам, мені адам деп атайды

Templates are now loaded from `data/dialog/templates/v1.toml` (14 families, one per intent-key), not hardcoded in `planner.rs`. `TemplateRepository::load_default()` auto-discovers the TOML file; `hardcoded_fallback()` preserves MVP guarantees when the file is missing.

Public API additions:

- `adam_dialog::TemplateRepository` + `TemplateError`
- `adam_dialog::respond_with_repo(input, lex, repo, seed)` — explicit-repo variant of `respond`
- `adam_dialog::plan_response_with_repo(intent, seed, repo)` + `intent_key(intent)`

`adam_chat` REPL now loads the TOML repo at startup (falls back to hardcoded if missing) and prints family count on stderr.

Ordering subtlety in the semantic dispatcher: `Thanks`/`Apology` are checked BEFORE `Affirmation` so "рахмет" (thanks) can't accidentally fall into affirmation if later extended.

Tests: 23 dialog end-to-end pairs (up from 15), 8 new covering all 5 new intents. Workspace totals: **183 passing**, 4 ignored, 0 failing.

Known v0.7.5 limitations (by design, not bugs):

- Templates are still literal phrases; slotted templates with `(root, features)` atoms land in v0.8.0.
- No entity extraction (own name from "менің атым X" → greeting back by name).
- 10 intents cover greetings + basic social politeness; v0.8.0 widens to 25.
- No multi-turn state.

## [0.7.0] — 2026-04-19

First iteration of the predictable dialog layer. New crate `adam-dialog` implements a 5-layer pipeline (FST parser → semantics → planner → realiser → FST synthesiser) against the architectural spec in `docs/kazakh_grammar/07_dialog_architecture.md`.

Recognises 5 intents from raw Kazakh input:
- `Greeting` with kind `Casual` / `Polite` / `TimeOfDay(Morning|Day|Evening)`
- `Farewell`
- `Affirmation`
- `Negation`
- `Unknown` (fallback)

Each intent has 2–4 hand-written response variants; planner picks one by seeded PRNG mod count. The entire output space is enumerable per input — no free generation.

New binary `adam_chat` with three modes:
- `--once "<input>"` — single-shot stdout response
- default — interactive REPL over stdin
- `--trace` — dump each layer's state (parses, intent, trace lines, output)

Tests: 15 end-to-end pairs cover the full pipeline. Workspace totals: 175 passing, 4 ignored, 0 failing.

Known v0.7.0 limitations (by design, not bugs):
- Only 5 social intents; ~150 templates needed for v1.0.0 MVP.
- Templates are hardcoded in `planner.rs`, not data-driven TOML (v0.7.5).
- No morphological info used for intent classification yet (v0.7.5+).
- No multi-turn state.

## [0.6.0] — 2026-04-19

Derivational morphology — the "word-formation layer" the user flagged as a v1.0.0-path requirement. The FST now transforms a root into a new root via a derivational suffix before applying inflection. Eleven derivation types covered:

- `Agent` `-шы/-ші` (жазу → жазушы)
- `Abstract` `-лық/-лік` (жақсы → жақсылық)
- `Privative` `-сыз/-сіз` (тұз → тұзсыз)
- `Endowed` `-лы/-лі` (күш → күшті)
- `Similative` `-дай/-дей` (тау → таудай)
- `Comparative` `-рақ/-рек` (жақсы → жақсырақ)
- `VerbalNoun` `-у` (жаз → жазу)
- `ActionNoun` `-ым/-ім` (айт → айтым)
- `Diminutive` `-шық/-шік` (үй → үйшік)
- `Ordinal` `-ншы/-нші` (бір → бірінші)
- `Collective` `-еу/-ау` (бір → біреу)

`NounFeatures` gains a `derivation: Option<Derivation>` field; `synthesise_noun` applies the derivation BEFORE inflection so the two pipelines chain correctly (жазу → Agent → жазушы → Dative → жазушыға).

Tests added: 10. `adam-kernel-fst` lib now at **78 passing**. Workspace at **160 passing**, 4 ignored, 0 failing.

No other code changes.

## [0.5.5] — 2026-04-19

Pure Kazakh lexicon milestone. Enforces the "no loanwords" directive at the lexicon level and augments coverage from classical 19th-century sources.

Pipeline:

1. **Purity audit** (`lexicon_purity_audit` binary) — classified all 16,373 entries from v0.4.0 curated + v0.4.5 Apertium-imported against strict pre-modern-Kazakh criteria (Russian-only letters, loanword suffixes, no Kazakh-specific letter).
2. **Pure Kazakh build** (`build_pure_kazakh_lexicon`) — filtered out 1,500 contaminated entries (824 Russian letters, 128 loanword suffixes, 681 no-Kazakh-signal). Retained 13,606.
3. **Abai gap analysis** (`extract_abai_gap`) — identified 715 unique root candidates missing from the lexicon but present as word forms in Abai's corpus.
4. **Augmentation** (`augment_lexicon_from_abai`) — automatically classified the top 500 gap candidates (393 nouns + 107 verbs) with POS, vowel harmony, and final sound class. Output: `data/lexicon_v1/abai_augmented_roots.json`.

Result:

| metric | v0.5.0 | v0.5.5 |
|---|---|---|
| Lexicon entries (pure) | n/a | 14,106 |
| Loanwords dropped | 0 | 1,500 |
| Abai vocabulary coverage | 88.8% | **97.8%** (+9 pp) |

Missing-vocabulary examples added (each backed by corpus frequency):
- `сөз` (word, speech — 123× in Abai)
- `бой`, `қан`, `қол`, `қар`, `жау`, `жат`, `жет`, `түс`, `қыс`, `жай`
- `надан` (ignorant — Abai's key philosophical concept)

These are fundamental proto-Kazakh vocabulary items the Apertium import had zero entries for.

No changes to the FST code, phonology, or morphotactics modules. The augmented lexicon file lives alongside the v0.4.5 imports and can be unioned into the active lexicon at load time.

Workspace totals: 150 tests passing, 4 ignored, 0 failing.

## [0.5.0] — 2026-04-19

Expands the v0.4.5 FST to cover Kazakh non-finite verb forms.

- **Vowel-final-stem aorist coalescence** (Apertium rules 17, 18, 19, 20, 30). Stem-final `ы/і` merge with the aorist `{A}` to produce `и` (e.g. `оқы` + PRES + 3 → `оқиды`, not the previous `*оқыа`). Stems ending in other vowels take a `й`-glide (`сөйле` → `сөйлейді`). Past tense on vowel-final stems (`оқы` + PAST + 1SG → `оқыдым`) continues to work without coalescence.
- **Participles** — three new `Tense` variants:
  - `ParticiplePast` — `-{G}{A}н` (`жазған`, `берген`, `қалған`).
  - `ParticipleHabitual` — `-{A}тын` (`жазатын`, `келетін`).
  - `ParticipleFuture` — `-{A}р` (`жазар`, `келер`).
- **Converbs** — two new `Tense` variants:
  - `ConverbPerfect` — `-{Y}п` (`жазып`, `беріп`).
  - `ConverbImperfect` — `-{A}` (`жаза` without personal ending).

Tests: **68 unit tests passing** in `adam-kernel-fst` (up from 55 in v0.4.5). Workspace totals: 150 passing, 4 ignored, 0 failing.

No changes to v0.4.0 transformer baseline or v0.4.5 FST core code.

## [0.4.5] — 2026-04-19

Introduces **adam-kernel-fst**, a pure-Rust deterministic finite-state transducer for Kazakh morphology. This is Phase 1 of the architecture pivot from stochastic transformers to deterministic morphology + small LM (v1.0.0 track). v0.4.0 transformer stack stays untouched; v0.4.5 adds the new FST layer alongside.

Highlights:

- **New crate `adam-kernel-fst`** — phonology module (12 archiphonemes, 20+ of 54 Apertium twol rules implemented), morphotactics module (25 suffix templates covering noun plural/possessive/case and verb tense/voice/negation/person), parser module (`analyse(surface) → Vec<(root, features)>`), lexicon loader (union of 4,454 curated + 11,919 Apertium-imported entries).
- **55 unit tests + 1 smoke test + 4 ignored slow roundtrips**. Slow tests (manual: `cargo test --test roundtrip -- --ignored`) roundtrip the full 14k lexicon on 4 feature combinations: **36,238 / 36,238 = 100.0 %** success.
- **CLI binary `adam_fst`** — `synth`, `analyse`, `stats` subcommands. Hand-rolled arg parsing (no CLI-framework dep).
- **Apertium-kaz import pipeline** (`import_apertium_lexicon` binary) with POS mapping, loanword filter, and prefix-conflict guard.
- **Grammar study notes**: `docs/kazakh_grammar/00_architecture_v1.md`, `01_phonology.md`, `02_morphology.md`, `03_syntax.md`, `04_lexicon_sources.md`, `05_work_plan.md`, `06_apertium_twol_catalogue.md`.

Not yet:

- Vowel-final-stem edge cases (rule 17 coalescence, semivowel у).
- Participles, converbs, infinitive.
- LM over root + feature-bundle sequences (v0.5+ target).
- Replacement of v0.4.0 pipeline (deliberately left untouched).

Workspace totals: 137 tests passing, 4 ignored, 0 failing. CI green.

## [0.4.0] — 2026-04-17

Corpus and infrastructure maturity release. Adds the first classical-literature source (Abai Qunanbayuly via Wikisource, 146 works, 2,253 samples), the first web-crawl source (CC-100 Kazakh, 50,000 samples filtered for Cyrillic-ratio and repetition), and fixes a data-composition bug in the synthetic generator (1- and 2-word outputs dominated the corpus, teaching the model early EOS). BPE retrained at vocab **8,192** with **3.27× compression** on a 12.5M-token pretokenized corpus. Model rolled back from the v0.4.0-failed experiment (27.3M, H=512 L=6) to **24.2M params** (H=512 L=5) after confirming that the L=6 scale-up was undertrained at 3.9M tokens.

Training: 20,000 steps, batch 8, seq 128, 3e-4 peak lr with cosine decay, 8h on M2 Metal at 0.64–0.70 steps/s throughput. First-class reliability: `train_baseline` now writes a periodic checkpoint every 2000 steps after a reboot lost 13k uncheckpointed steps mid-run.

Validation (honest):
- 12,101 held-out samples (larger/harder distribution vs v0.3.0's 1,939)
- mean_ce: 7.43, **perplexity: 1691.89**
- bits/char: **3.28** (v0.4.0-failed: 3.26; v0.3.0: 3.49 — val sets not directly comparable)

Qualitative:
- Complete grammatical Kazakh sentences now appear in `temp=0.8` and nucleus samples (6 of 30 showcase outputs): `жақсы адам мағына береді`, `ол жазады`, `олар жүреді`, `үлкен жақсы адам оқыйды`, `мектеп туралы мәртебе нақтылайды`.
- Greedy still terminates early — expected for a capacity-bound model (24M params × 4M training tokens is ~25× below Chinchilla-optimal data).

v0.5.0 will address the data bottleneck: curriculum-style FSM expansion (L1/L2/L3 difficulty), larger CC-100 sample (50k → 500k), classical-literature expansion (Ауэзов, Нурпеисов, Бөкей locally), and SFT on translated Alpaca for the first instruction-following pass.

## [0.3.0] — 2026-04-15

First capacity scale-up. `ModelConfig::tiny` grows from 4.28M → **20.0M params** (hidden 224→512, layers 4→5, ffn 896→2048, head_dim 28→64). 15,000 training steps on the 39k unified corpus, 3h 45m on MacBook Air M2 Metal. Validation perplexity drops from **1112.31 → 871.30 (−21.7%)** — first meaningful delta since Phase 6a real-text onset. The 4.28M envelope was saturated at Phase 6d; Wikipedia + Common Voice distribution breadth required more model capacity. Peak RSS ~2.5 GB of 8 GB unified memory — headroom confirmed.

## [0.2.0] — 2026-04-15

First minor release after v0.1.0. Full retrain on the 39k unified corpus assembled across Phases 6a–6d. BPE retrained (3,336 merges, 2.80× compression, **0 unknowns, 100.00% roundtrip**). 4.28M model, 15,000 steps, 1h 48m wall time on M2 Metal. Val PPL 1078.68 → 1112.31 (flat; val set is larger and harder — capacity bottleneck now visible).

## [0.1.6] — 2026-04-15

**Phase 6d — Common Voice KK.** Mozilla Common Voice Kazakh sentence-collector integrated (6,108 accepted, CC0-1.0 text only). Unified corpus grows 32,986 → 39,058 unique (+6,072; 4,282 duplicates dedup'd at assembly). Also fixes `scripts/bump_foundation_version.sh`: Cargo.lock is no longer perl-replaced (corrupted transitive deps under naive substring replace); regenerated by `cargo build` after Cargo.toml bump.

## [0.1.5] — 2026-04-15

**Phase 7.1 — Wikipedia-augmented retrain.** 4.28M baseline retrained on the corpus enlarged with Wikipedia KZ. 14,000 steps, ~2h on M2 Metal. Val PPL 626.81 → 1078.68, reflecting a broader, harder val set (Wikipedia sentences are lexically richer than Tatoeba) — honest baseline on the enlarged distribution, not a regression.

## [0.1.4] — 2026-04-14

**Phase 6c — Kazakh Wikipedia.** Plain-text extracted from the kk.wikipedia.org XML dump (~296k articles → 15,000 clean samples after filter; CC-BY-SA 4.0). Unified corpus 17,986 → 32,986 unique. Infrastructure: `scripts/fetch_wikipedia_kz.sh` (bzcat + perl streaming with UTF-8 fix), `process_wikipedia_kz` binary.

## [0.1.3] — 2026-04-14

**Phase 7 — first real-text baseline.** 4.28M model trained on the unified corpus including authentic Kazakh (Tatoeba): 7,000 steps, 61m on M2 Metal, dropout 0.05, grad clipping max-norm 1.0. Explicit `loss.backward() → clip → opt.step` replaces `opt.backward_step`. First honest perplexity on real text: **626.81** (vs 129.49 on pure synthetic — tells us real Kazakh is harder).

## [0.1.2] — 2026-04-14

BPE vocab size bumped 1390 → **4096**. Char-level fallback + Tatoeba real text saturate the larger target.

## [0.1.1] — 2026-04-14

**Phase 6a/6b — first authentic Kazakh source.** Tatoeba Kazakh integrated (4,058 sentences, CC-BY 2.0 FR) via `fetch_tatoeba_kazakh.sh` + `process_tatoeba_kazakh`. Tokenizer adds char-level fallback for FSM-unknown words and leading-punct ▁ marker placement — brings roundtrip to 100% on mixed real/synthetic text.

## [0.1.0] — 2026-04-14

First minor release. The foundation works end-to-end: a Kazakh-first 3.06M-parameter transformer language model trained from scratch on a self-generated, FSM-validated synthetic corpus, evaluated against held-out perplexity, and serving inference with morpheme-aware BPE encode/decode.

### Brand
- Logo `assets/shanraq.svg` integrated into README header.
- README rewritten with centered title, badges, quickstart, and stats.
- `AUTHORS` file added.

### Quality (cumulative since v0.0.85)
- Validation perplexity: **129.49** on a 699-sample held-out set (mean cross-entropy 4.86 over 2532 tokens).
- All 464 segmentation eval examples match at 10000 bps.
- Foundation validation green across 11 layers.

---

## Phase 5 — Training and inference (v0.0.81 → v0.0.92)

### [0.0.92] — Phase 5i: Generation showcase report
- New `generation_showcase` binary: 20 prompts × 3 sampling configs = 60 generations.
- Report artifact `data/training/generation_showcase_report.json`.
- Foundation validation now requires showcase + perplexity reports.

### [0.0.91] — Phase 5h: Top-p + repetition penalty
- `generate` gains nucleus (top-p) sampling and GPT-2-style repetition penalty.
- Backwards-compatible CLI; defaults are no-ops.

### [0.0.90] — Phase 5g: Hyperparameter tuning
- Dropout 0.10 → 0.05 reduces over-regularization on small corpus.
- Gradient clipping (max-norm 1.0) added to `train_baseline`.
- Training extended to 7000 steps with 300-step warmup.
- **Perplexity: 165.98 → 129.49 (−22%).**

### [0.0.89] — Phase 5f: Model scaling + dropout
- ModelConfig::tiny() bumped: hidden 192 → 224, heads 6 → 8, ffn 768 → 896, +dropout=0.1.
- 2.33M → 3.06M params.
- `forward(ids, train: bool)` added to gate dropout on/off.

### [0.0.88] — Phase 5e: Held-out eval + perplexity
- `encode_corpus` extended with deterministic train/val split (FNV hash of sample id).
- New `eval_perplexity` binary writes structured `validation_perplexity_report.json`.
- First baseline: **165.98 perplexity**.

### [0.0.85] — Phase 5d: Inference binary
- `generate` binary: load checkpoint, autoregressive sampling (greedy/temperature/top-k).
- First sentence generated by the model: "жақсы адам аз көрсетеді."

### [0.0.84] — Phase 5c: Training loop
- `train_baseline` binary: AdamW + linear-warmup + cosine-decay LR + safetensors checkpointing.
- First trained checkpoint, training loss 7.94 → 3.39 in 7m on Metal.

### [0.0.83] — Phase 5b: Data loader
- `DataLoader` reads ids pack, produces shifted (input, target) batches on device.
- End-to-end smoke test: forward + cross-entropy loss.

### [0.0.81] — Phase 5a: Candle integration
- Added candle (HuggingFace Rust ML) with Metal backend on macOS, CPU elsewhere.
- `AdamBaseline` decoder-only transformer (initial 2.21M params).
- M2 Metal smoke test passes.

---

## Phase 4 — Tokenizer (v0.0.78 → v0.0.80)

### [0.0.87] — Phase 4d+4e: Lexicon-seeded BPE
- BPE vocab now seeded with all 211 lexicon roots + all 422 rule forms before counting pairs.
- 0% `<unk>` on any FSM-parseable Kazakh word.

### [0.0.80] — Phase 4c: BPE encoder/decoder
- `bpe::BpeTokenizer` module: load vocab+merges, encode text → ids, decode ids → text.
- `encode_corpus` binary writes a training-ready ids pack.
- 100% round-trip on 7,737 samples.

### [0.0.79] — Phase 4b: BPE trainer
- `train_bpe` binary: iterative most-frequent-pair merging over morpheme stream.
- Skips merges across word boundary (right token starts with ▁).
- 567 merges learned from corpus statistics; 2.12× compression.

### [0.0.78] — Phase 4a: Pre-tokenizer
- `pretokenize(text, lexicon, rules)`: morpheme-aware splitting via FSM.
- SentencePiece-style ▁ marker on word-start morphemes.
- Handles standalone punctuation and whole-word fallback.

---

## Phase 3 — Corpus (v0.0.74 → v0.0.77)

### [0.0.86] — Phase 3e: Full POS coverage
- 15 → 30 templates exercising every POS (adverbs, particles, modals, ол/олар, conjunctions).
- Synthetic corpus 10,000 → 18,000 samples.
- Unified corpus 7,737 → 13,929 unique samples.

### [0.0.77] — Phase 3d: Kazakh proverbs
- Added 80 classical мақал-мәтелдер across 23 themes.
- Proverbs bypass FSM-validation policy (archaic morphology); Cyrillic-only check.

### [0.0.76] — Phase 3c: Unified corpus
- `assemble_unified_corpus` binary: dedup + renumber across packs.
- 7,657 unique samples from 10,094 inputs.

### [0.0.75] — Phase 3b: Rich templates
- Generator templates 6 → 15: pronouns with matched person, conjunctions, multi-argument, etc.
- 10,000 sentences (95% yield).

### [0.0.74] — Phase 3a: Synthetic generator
- `synth_sentences` binary: combines FSM lexicon and rules to produce grammatically valid Kazakh sentences.
- Self-validation: every generated word verified by `deterministic_segment_token`.
- FSM fix: removed vowel from `verb_tense_a/e_from_stem` allowed finals (linguistically correct — `й` handles vowel-final aorist).

---

## Phase 2 — Grammatical foundation (v0.0.66 → v0.0.73)

### [0.0.73] — Phase 2h: Modals
- New `Modal` POS, 6 roots: керек, мүмкін, тиіс, шығар, қажет, лайық.

### [0.0.72] — Phase 2g: Nominal predicate
- 16 predicative personal suffix rules: -мын/мін, -сың/сің, -сыз/сіз, -мыз/міз on noun + adjective.
- 3 copula bare lexemes as Particle: еді, екен, емес.

### [0.0.71] — Phase 2f: Adverbs
- New `Adverb` POS, 19 roots: қазір, бүгін, ертең, кеше, тез, баяу, жоқ, иә, etc.

### [0.0.70] — Phase 2e: Numerals
- New `Numeral` POS, 20 cardinals: бір–жүз, мың.
- 4 ordinal suffix rules: -ншы/нші/-ыншы/інші.

### [0.0.69] — Phase 2d: Conjunctions + Particles
- New `Conjunction` POS, 9 roots: және, бірақ, себебі, өйткені, etc.
- New `Particle` POS, 12 roots: ма/ме, ба/бе, па/пе, ғой, да/де, тек, қана, өте.

### [0.0.68] — Phase 2c: Roots + 3sg aorist
- 29 nouns, 13 verbs, 5 adjectives added.
- Critical FSM fix: `tense → person_3sg` was missing for aorist forms (e.g. береді = бер+е+ді). Added rules for both future and negative_future predecessors.
- "й" connector rule for vowel-final verb stems (жасайды).
- Coverage 19.79% → **73.77%** on educational corpus.

### [0.0.67] — Phase 2b: Postpositions
- New `Postposition` POS, 15 roots: арқылы, үшін, туралы, кейін, etc.

### [0.0.66] — Phase 2a: Adjectives
- New `Adjective` POS, 25 roots, 57 inflection rules (mirror of noun rules).
- Coverage 4.56% → 17.93%.

---

## Pre-Phase 2 — Foundation infrastructure

### [0.0.65] — `normalize_token` for accurate coverage
- `coverage_report` strips trailing punctuation before FSM matching.

### [0.0.64] — adam-kernel L0 crate extraction
- Created `adam-kernel`: identity types + Kazakh FSM morphological engine.
- adam-core merged into adam-kernel.
- New `coverage_report` binary measures FSM coverage on real Kazakh text.

### [0.0.63] and earlier
- Initial corpus / tokenizer / eval / training infrastructure.
- Foundation overview report.
- Tiny clean training pipeline with miss audit.
- See git history (`git log v0.0.63 --oneline`) for details.
