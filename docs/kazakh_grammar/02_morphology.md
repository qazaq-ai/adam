# Kazakh Morphology — Agglutinative Structure

Status: **Week 1-2 skeleton**. This file defines the space the FST must cover. Cells marked `TBD` await corpus verification or standard-grammar cross-check.

## 1. Word structure

Kazakh is strictly agglutinative. A word is built as:

```
ROOT + DERIVATIONAL_SUFFIXES* + INFLECTIONAL_SUFFIXES+
```

Derivational suffixes change the part of speech or nuance of meaning (e.g., `бала → балалы` noun→adjective "having a child"). Inflectional suffixes mark grammatical features and have **strictly fixed order** per POS.

### 1.1 Canonical inflection order (noun)

`ROOT + PLURAL + POSSESSIVE + CASE`

Example: `мектеп+тер+іміз+ге` (school+PL+POSS.1PL+DAT) = "to our schools"

### 1.2 Canonical inflection order (verb)

`ROOT + VOICE + NEGATION + ASPECT/MOOD + TENSE + PERSON+NUMBER`

Example: `жаз+ды+р+ма+й+мыз` (write+CAUS+NEG+PRES+1PL) = "we won't let [it] be written"

(Simplifying: the exact chain order above is the most-common realisation. Alternate combinations exist and are documented per-suffix in section 4 below.)

## 2. Noun system

### 2.1 Plural

Base form: `-лар / -лер` (back / front)

Allomorphs (selected by preceding phoneme):

| context | back | front |
|---|---|---|
| vowel | лар | лер |
| voiced non-nasal | лар | лер |
| voiceless | тар | тер |
| nasal (л, м, н, ң, р, й) | дар | дер |

(Note: traditional grammars split "sonorant" vs "nasal"; we fold them together because the rule output is identical.)

### 2.2 Possessive

| person | back-harmonic | front-harmonic |
|---|---|---|
| 1sg | (ы)м | (і)м |
| 2sg informal | (ы)ң | (і)ң |
| 2sg polite | (ы)ңыз | (і)ңіз |
| 3sg/3pl | (с)ы | (с)і |
| 1pl | (ы)мыз | (і)міз |
| 2pl informal | (ы)ңдар | (і)ңдер |
| 2pl polite | (ы)ңыздар | (і)ңіздер |

Buffer vowels `ы/і` are inserted only after consonant-final stems; after vowel-final stems they are omitted (yielding `бала+м` not `*бала+ым`). The `с` in 3rd person is a **buffer consonant** inserted only after vowel-final stems (`бала+сы` not `*бала+ы`).

### 2.3 Case

Seven cases. Base forms + allomorph matrices:

| case | base form (back / front) | count of allomorphs |
|---|---|---|
| nominative | — (null) | 0 |
| genitive | -ның / -нің | 6 (depending on preceding consonant class) |
| dative | -ға / -ге | 6 |
| accusative | -ны / -ні | 6 |
| locative | -да / -де | 4 |
| ablative | -дан / -ден | 6 |
| instrumental / comitative | -мен / -пен / -бен | 3 |

Full allomorph matrix will be encoded in `data/tokenizer/segmentation_rules.json` v1.

### 2.4 Interaction with possessive

When both possessive and case co-occur, the **buffer `н` intervenes** before the case suffix on certain possessive forms. This is "pronominal н":

- `бала+сы+н+да` (child+POSS.3+BUFFER+LOC) = "at his/her child"
- `бала+ң+да` (child+POSS.2+LOC) = "at your child" (no buffer)

This single rule depends on possessive form and is one of the trickier deterministic constraints. It must be encoded explicitly.

## 3. Verb system

### 3.1 Voice markers (between root and rest)

| voice | suffix | notes |
|---|---|---|
| active | — | unmarked |
| passive | -ыл / -іл / -л | selected by root final |
| reflexive | -ын / -ін / -н | |
| reciprocal | -ыс / -іс / -с | |
| causative | -дыр / -дір / -тыр / -тір / -қыз / -кіз / -ғыз / -гіз | context-dependent |

### 3.2 Negation

Single suffix `-ма/-ме`, with assimilated variants `-ба/-бе`, `-па/-пе` after certain finals. Always follows voice, precedes tense.

### 3.3 Tense/aspect

| tense | marker | notes |
|---|---|---|
| past (definite) | -ды/-ді/-ты/-ті | |
| evidential/reported past | -ған/-ген/-қан/-кен | |
| habitual past | -атын/-етін/-йтын/-йтін | |
| present (future-tinged, "aorist") | -а/-е/-й | |
| present progressive | -ып жатыр / -іп жатыр | complex (converb + aux) |
| future (intentional) | -мақ/-мек | |
| future (possible) | -ар/-ер/-с | |
| conditional | -са/-се | |
| imperative | various by person (see 3.5) |

### 3.4 Person+number endings

Different paradigms by tense. Below are the "main" personal endings (attach after definite-past `-ды`):

| person | back | front |
|---|---|---|
| 1sg | -м | -м |
| 2sg informal | -ң | -ң |
| 2sg polite | -ңыз | -ңіз |
| 3 | — | — |
| 1pl | -қ | -к |
| 2pl informal | -ңдар | -ңдер |
| 2pl polite | -ңыздар | -ңіздер |

For non-past tenses different endings apply (`-мын/-мін` for 1sg, etc.). This is a major bookkeeping area.

### 3.5 Imperative paradigm

Separate from other moods; has its own 7-cell matrix:

| person | base example (жаз "write") |
|---|---|
| 2sg informal | жаз |
| 2sg polite | жазыңыз |
| 2pl informal | жазыңдар |
| 2pl polite | жазыңыздар |
| 3 (let him...) | жазсын |
| 1sg volitional | жазайын |
| 1pl volitional | жазайық |

## 4. Suffix ordering constraints — the finite-state spec

The v1.0.0 FST needs an explicit state machine per POS. Below is a compact spec suitable for direct Rust translation:

### 4.1 Noun FST states

```
q0_root → q1_plural? → q2_possessive? → q3_case? → ACCEPT
```

`?` = optional. Legal transitions only forward. No loops.

### 4.2 Verb FST states

```
q0_root → q1_voice? → q2_negation? → q3_tense → q4_person? → ACCEPT
```

Tense is **required** for finite verbs; person is required except for tenses that inherently lack it (e.g., participles, converbs).

Non-finite forms (participles, converbs, infinitive) have their own exit states before q4 and produce deverbal nouns or adjectives that can re-enter the noun FST. This is where **cross-POS feeding** happens and must be designed carefully.

## 5. Allomorphy — the "table of 32 dative suffixes" reality

Each inflectional slot can have between 2 and 8 surface realisations depending on context. For the FST:

- Each slot has one **underlying form**
- Each slot has a **rule table**: (vowel_harmony × preceding_segment_class) → surface string
- Tables are derived mechanically in pre-compute; lookup is O(1) at runtime

Estimated total unique surface strings across all inflectional slots: **~250**. This is tractable to enumerate exhaustively, which is the verification strategy.

## 6. Known difficult phenomena (risk register)

1. **Buffer vowels vs contracted forms** (`бала+м` vs `мектеп+ім`) — conditioning is phonological, must be modelled not rote-listed.
2. **Pronominal `н` after 3rd-person possessive** — one rule, easy to forget.
3. **Russian loanword integration** — some take Kazakh suffixes transparently, others require stem-final reinterpretation (e.g., ending in soft sign `ь` drops the sign before suffix).
4. **Vowel-stem verbs** inserting `й` before some suffixes (`оқы+й+ды` "reads").
5. **`-у` infinitive forms** act as nouns and take noun morphology; this cross-POS path is a source of ambiguity in the parser direction.
6. **Clitics** (`ма/ме` interrogative, `да/де` additive, `ғой/-ғой` emphatic) — attach after the inflected word but before word boundary; must be modelled as post-inflection optional slot or as separate orthographic words.

## 7. Source references

- Ысқақов А., «Қазіргі қазақ тілі: морфология» (Almaty, 1991) — primary source
- Мамaнов Ы., «Қазіргі қазақ тілі» — comprehensive treatment
- Apertium-kaz `kaz.lexc` (lexicon + morphotactics), `kaz.twolc` (phonological rules) — machine-readable reference
- HFST Kazakh grammar repository
