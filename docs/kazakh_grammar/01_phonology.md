# Kazakh Phonology — Foundations for FST Rules

Status: **Week 1 skeleton**. Tables populated from standard references, to be refined with corpus evidence in week 2.

## 1. Alphabet (Cyrillic orthography, current official standard)

42 letters. The 9 letters marked ★ are Kazakh-specific (not present in Russian):

| letter | IPA | type |
|---|---|---|
| а | /a/ | vowel, back |
| ә ★ | /æ/ | vowel, front |
| б | /b/ | consonant, voiced stop |
| в | /v/ | consonant, voiced fricative (mostly in loanwords) |
| г | /g/ | consonant, voiced stop |
| ғ ★ | /ʁ/ | consonant, voiced uvular fricative |
| д | /d/ | consonant, voiced stop |
| е | /je/ or /e/ | vowel, front |
| ё | /jo/ | vowel, back (loanwords) |
| ж | /ʒ/ | consonant, voiced fricative |
| з | /z/ | consonant, voiced fricative |
| и | /ij/ or /əj/ | diphthong-like (loanwords / digraph) |
| й | /j/ | consonant, glide |
| к | /k/ | consonant, voiceless stop |
| қ ★ | /q/ | consonant, voiceless uvular stop |
| л | /l/ | consonant, voiced liquid |
| м | /m/ | consonant, voiced nasal |
| н | /n/ | consonant, voiced nasal |
| ң ★ | /ŋ/ | consonant, voiced nasal (velar) |
| о | /o/ | vowel, back |
| ө ★ | /ø/ | vowel, front |
| п | /p/ | consonant, voiceless stop |
| р | /r/ | consonant, voiced trill |
| с | /s/ | consonant, voiceless fricative |
| т | /t/ | consonant, voiceless stop |
| у | /w/ or /uw/ | consonant/vowel (context dependent) |
| ұ ★ | /ʊ/ | vowel, back |
| ү ★ | /y/ | vowel, front |
| ф | /f/ | consonant, voiceless fricative (loanwords) |
| х | /x/ | consonant, voiceless fricative |
| һ ★ | /h/ | consonant, voiceless glottal fricative (Arabic loanwords) |
| ц | /ts/ | affricate (loanwords) |
| ч | /tʃ/ | affricate (loanwords) |
| ш | /ʃ/ | consonant, voiceless fricative |
| щ | /ɕɕ/ | consonant (loanwords) |
| ъ | — | hard sign (Russian orthographic) |
| ы | /ə/ | vowel, back |
| і ★ | /ɪ/ | vowel, front |
| ь | — | soft sign (Russian orthographic) |
| э | /e/ | vowel, front (loanwords) |
| ю | /ju/ | vowel (loanwords) |
| я | /ja/ | vowel (loanwords) |

**Note on loanwords**: letters в, ё, ф, ц, ч, щ, ъ, ь, э, ю, я are used almost exclusively in words borrowed from Russian. Native Kazakh roots use the remaining 30 letters. This is a **filter signal** for loanword detection.

## 2. Vowel classes

| class | back | front |
|---|---|---|
| open unrounded | а | ә |
| close unrounded | ы | і |
| close rounded | ұ | ү |
| open rounded | о | ө |
| mid (rare/loan) | у, и | е, э |

**Harmony principle**: within a native Kazakh word, all vowels belong to the same class (back OR front). This is the **foundational determinism** for agglutination.

Exceptions (to be catalogued):
- Compound words may have mixed harmony at compound boundary.
- Russian-origin loanwords often violate harmony (intact).
- Some native words have settled irregularities (e.g., the `и` digraph).

## 3. Consonant classes (for assimilation)

| class | members |
|---|---|
| voiceless | к, қ, п, с, т, ф, х, һ, ц, ч, ш, щ |
| voiced (non-sonorant) | б, в, г, ғ, д, ж, з |
| voiced sonorant / nasal | л, м, н, ң, р, й |

**Assimilation principle**: at morpheme boundaries, consonants of an attaching suffix change voicing and place of articulation to match the preceding segment.

Key alternation patterns (to be formalised as FST rules):

1. **Plural suffix**: base form `-лар/-лер`. After voiceless-ending root → `-тар/-тер`. After nasal-ending root → `-дар/-дер`. Front/back harmony independently.
2. **Locative suffix**: `-да/-де` after voiced, `-та/-те` after voiceless.
3. **Ablative suffix**: `-дан/-ден` after voiced, `-тан/-тен` after voiceless, `-нан/-нен` after nasal.
4. **Dative suffix**: `-ға/-ге` after voiced non-guttural, `-қа/-ке` after voiceless non-guttural, `-на/-не` after possessive.
5. **Genitive suffix**: `-дың/-дің/-тың/-тің/-ның/-нің` (selected by harmony × preceding segment class).

This is the **8-way matrix** (2 harmonies × 4 preceding contexts) that defines most of the noun-case system. Each cell is one deterministic function.

## 4. Consequence for FST design

Every morpheme in our lexicon will be stored as an **abstract underlying form** with feature requirements. The realisation function `(feature_bundle, preceding_context) → surface_string` is encoded in a small number of 2-level rules. This gives:

- Exactly one surface form per (root, feature_bundle)
- Exactly one analysis per well-formed surface string (or a rejection if ill-formed)

The phoneme-level tables above feed directly into these rules. Week 2 work: finalise these tables against corpus evidence and publish as machine-readable JSON.

## 5. Source references

- Кеңесбаев С., «Қазіргі қазақ тілінің фонетикасы» (Almaty, 1974)
- Джусупов М., «Звуковые системы русского и казахского языков. Слог. Интерференция. Обучение произношению» (Tashkent, 1991)
- Ысқақов А., «Қазіргі қазақ тілі: морфология» (Almaty, 1991) — ch. 1 on phonology
- Apertium-kaz `phon.twolc` two-level rules (import in Week 2)

## 6. Known gaps / research items

- [ ] Resolve `и` and `у` as digraphs vs phonemes in FST input (current adam code treats them as single chars; verify correctness)
- [ ] Verify exact allomorph selection for `1st-person singular possessive` on vowel-final roots (may require special rule for `й`-insertion)
- [ ] Loanword behaviour — do Russian loans take Kazakh suffixes according to final phoneme of Russian stem, or according to assumed phonetic adaptation?
- [ ] Compound-word harmony: dominant-first rule vs. dominant-last?
