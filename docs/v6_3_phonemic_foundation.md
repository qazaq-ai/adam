# v6.3.0 — Phonemic Foundation

**Status.** 🚧 **Implementation in progress on
`experimental/v6_3_phonemic_foundation`** (cut from `main` at
`b04ea732`). As of 2026-05-27 the arc has shipped phases 1-7, 9
and 10 plus Phase 11 steps 1-3; see the phase table in § 5 for
per-phase status. New crates landed: `adam-phoneme`,
`adam-phonotactics`, `adam-audio`, `adam-stt-phoneme`,
`adam-tts-phoneme`, `adam-kernel-phoneme`, `adam-forced-aligner`,
plus the `voice_repl_v6_3` demonstrator. The full audio path
(audio → MFCC → CMVN → phonemes → Cyrillic → dialog → phoneme
TTS) builds without whisper.cpp or macOS `say`.

**Open risks (2026-05-27 external audit).** STT phoneme accuracy
is not yet measured on a held-out set — only smoke-tested on two
Wikimedia probes (see § 5 Phase 11). A PER/WER report over the
FLEURS test split is the next deliverable. The `data/
v6_3_phoneme_bank` derived corpus (~2.2 GB) is `.gitignore`d and
regenerable; `data/v6_3_corpus/MANIFEST.jsonl` provenance is
still to be reconciled. The workspace `Cargo.toml` version stays
`6.2.0` until the branch merges to `main`.

**Origin.** Voice REPL session-5 (2026-05-26 demo-prep) closed
yet another batch of STT-mishear / listing-fallback bugs. User
signal:

> «Мы опять после каждого диалога находим новые баги и они опять
> не кончаются. … Такое ощущение, что мы стараемся с помощью кода
> предусмотреть все возможные варианты ответов. Но это теоретически
> не возможно. … Поэтому, возможно необходимо начать новую
> исследовательскую ветку v6.3.»

The diagnosis is structural, not tactical: v6.2's router is a
deterministic case-enumeration over an exponential input space
(Whisper mishears × phrasing variants × case forms × context).
Patching cannot win this game. **The graphemes-first foundation
itself is the defect.**

A second user signal landed the same day, after the v6.3
brainstorm:

> «Создатели кириллической и латинской графики казахского создали
> буквы «ы» / «y», которые не имеют казахского звука. Это что-то
> вроде "апострофа" между двумя звуками. … Необходимо создать на
> Rust базу чистых звуков казахского и сформировать базу всех
> слов казахского в аудио двоичном формате. Графический формат
> будет вытекать из двоичного аудио формата, а не наоборот.»

This is the **architectural pivot** v6.3 demands. Phonemes are
the foundation; graphemes are a peripheral, lossy projection.

## 1. Thesis under test

> A **phonemic-first agglutinative architecture** — where the
> sole canonical representation of a Kazakh utterance is a typed
> phoneme stream over an inventory of ~37 units, all higher
> layers (morpheme, word, sentence, dialog) operate over that
> stream, and Cyrillic / Latin orthographies are bidirectional
> renderers — collapses an entire class of audit bugs into
> impossible-by-construction errors, eliminates external C
> dependencies (`whisper.cpp`, macOS `say`), and provides the
> correct substrate for the v6.3 brainstorm's constrained
> decoding / DisCoCat / energy-based verifier directions.

## 2. Why graphemes-first is structurally broken

### 2.1 The «ы» / «y» problem

In Cyrillic Kazakh orthography, the letter **ы** is officially
assigned IPA `/ə/`. In Latin orthography (new alphabets), the
letter **y** plays the same role. In **actual Kazakh speech**,
this segment is in most positions:

- an **epenthetic** reduced vowel — a phonetic glue inserted to
  avoid disallowed consonant clusters, or
- a **purely orthographic marker** of consonant-cluster cohesion,
  with **no acoustic realisation** at normal speech rate.

A native speaker pronouncing «қыз» («girl») produces something
closer to `[qz]` or `[qǝz]` with a minimal, often
non-syllabic transition — not the full `[qɯz]` that
non-Kazakh readers infer from the spelling. The orthography
**promotes a phonetic transition into a full grapheme**, and
non-native speakers (and Whisper, trained on orthographic
text) read it back as a full vowel.

This is the same class of defect as French «Renault»: 7 letters,
4 phonemes (`[ʁə.no]`). The orthography is **lossy in both
directions**, and the loss is asymmetric — different segments
get different amounts of artificial padding.

### 2.2 What this costs us in v6.2

The v6.2 morphological FST operates on **graphemes**, not
phonemes. Every rule must handle orthographic artifacts:
the «ы»-epenthesis cases, the «у» that flips between `/w/` and
`/uw/` based on context, the digraph «и» that resolves to
`/ij/` or `/əj/`. Every Whisper-mishear handler in
`v6_2_router::stt_fold` is a **graphical hack for a
phonetically clean phenomenon** — «қобейту»/«кубит»/«кубейт»
are all the same phoneme sequence `[k̪ø.bɛj.tʉ]` rendered
differently by a noisy ASR.

In a phoneme-first system these collapse to **one match**, not
six string-replace rules.

## 3. Architecture

### 3.1 The five layers

```
Layer 0a  Phoneme Alphabet         (~37 units, typed, discrete)
   ↓
Layer 0b  Acoustic Realisation     (MFCC templates + diphone bank)
   ↓
Layer 0c  Phonotactic FST          (harmony, clusters, syllable)
   ↓
Layer 0d  Bidirectional Renderer   (phoneme ↔ Cyrillic ↔ Latin)
   ↓
Layer 1+  Morpheme / Lexicon / Frame / Dialog
          (existing v6.2 stack, lifted to phonemes)
```

Each layer is a separate Rust crate. Layers 0a–0d are new;
Layer 1+ is the v6.2 stack with its string-typed boundaries
replaced by `Vec<Phoneme>`.

### 3.2 Layer 0a — Phoneme alphabet

A typed enum (~37 variants) plus an attribute table. Each
phoneme carries:

| field            | values                                          |
|------------------|--------------------------------------------------|
| `id`             | `Q`, `K`, `Aa`, `Ae`, `Oo`, `Oe`, …             |
| `ipa`            | `q`, `k`, `a`, `æ`, `o`, `ø`, …                  |
| `class`          | Vowel ∣ Consonant ∣ Glide                        |
| `place`          | (consonants) labial ∣ dental ∣ velar ∣ uvular ∣ … |
| `manner`         | (consonants) stop ∣ fricative ∣ nasal ∣ trill ∣ …|
| `voicing`        | voiced ∣ voiceless                               |
| `harmony_class`  | (vowels) Front ∣ Back                             |
| `height`         | (vowels) Close ∣ Mid ∣ Open                       |
| `rounding`       | (vowels) Rounded ∣ Unrounded                      |
| `length`         | Short ∣ Long                                      |
| `is_epenthetic`  | `bool` — true for reduced `[ə]` / `[ɪ]` in
                     positions where Cyrillic writes «ы» / «і»
                     but speech omits or minimally realises it.  |

**Finalised inventory: 37 units** (cross-checked against
`docs/kazakh_grammar/01_phonology.md` and standard Turkic
phonological descriptions).

### Vowels — 10 units (8 full + 2 epenthetic)

| id  | IPA  | harmony | height | rounding   | length | epenthetic | Cyrillic |
|-----|------|---------|--------|------------|--------|------------|----------|
| `A` | /a/  | Back    | Open   | Unrounded  | Short  | no         | а        |
| `Ä` | /æ/  | Front   | Open   | Unrounded  | Short  | no         | ә        |
| `O` | /o/  | Back    | Open   | Rounded    | Short  | no         | о        |
| `Ö` | /ø/  | Front   | Open   | Rounded    | Short  | no         | ө        |
| `U` | /ʊ/  | Back    | Close  | Rounded    | Short  | no         | ұ        |
| `Ü` | /y/  | Front   | Close  | Rounded    | Short  | no         | ү        |
| `E` | /e/  | Front   | Mid    | Unrounded  | Short  | no         | е        |
| `I` | /i/  | Front   | Close  | Unrounded  | Long   | no         | и (= Ɨ+J) |
| `Ǝ` | /ɯ/  | Back    | Close  | Unrounded  | Short  | **yes**    | ы        |
| `Ɨ` | /ɪ/  | Front   | Close  | Unrounded  | Short  | **yes**    | і        |

The `is_epenthetic = true` flag on `Ǝ` and `Ɨ` is the technical
core of the v6.3 thesis: in most positions where Cyrillic
writes «ы» / «і», the segment is acoustically null or minimal.
The Layer 0d Cyrillic→Phoneme renderer applies the rule in §9
OQ4 (resolved below) to decide when these letters become real
phonemes vs. orthographic markers.

### Consonants — 27 units

| id    | IPA  | place        | manner      | voicing    | native/loan | Cyrillic |
|-------|------|--------------|-------------|------------|-------------|----------|
| `P`   | /p/  | Bilabial     | Stop        | Voiceless  | native      | п        |
| `B`   | /b/  | Bilabial     | Stop        | Voiced     | native      | б        |
| `M`   | /m/  | Bilabial     | Nasal       | Voiced     | native      | м        |
| `T`   | /t/  | Dental       | Stop        | Voiceless  | native      | т        |
| `D`   | /d/  | Dental       | Stop        | Voiced     | native      | д        |
| `S`   | /s/  | Alveolar     | Fricative   | Voiceless  | native      | с        |
| `Z`   | /z/  | Alveolar     | Fricative   | Voiced     | native      | з        |
| `N`   | /n/  | Alveolar     | Nasal       | Voiced     | native      | н        |
| `L`   | /l/  | Alveolar     | Lateral     | Voiced     | native      | л        |
| `R`   | /r/  | Alveolar     | Trill       | Voiced     | native      | р        |
| `Š`   | /ʃ/  | Postalveolar | Fricative   | Voiceless  | native      | ш        |
| `Ž`   | /ʒ/  | Postalveolar | Fricative   | Voiced     | native      | ж        |
| `J`   | /j/  | Palatal      | Glide       | Voiced     | native      | й        |
| `K`   | /k/  | Velar        | Stop        | Voiceless  | native      | к        |
| `G`   | /g/  | Velar        | Stop        | Voiced     | native      | г        |
| `Ŋ`   | /ŋ/  | Velar        | Nasal       | Voiced     | native      | ң        |
| `X`   | /x/  | Velar        | Fricative   | Voiceless  | native      | х        |
| `Q`   | /q/  | Uvular       | Stop        | Voiceless  | native      | қ        |
| `Ğ`   | /ʁ/  | Uvular       | Fricative   | Voiced     | native      | ғ        |
| `H`   | /h/  | Glottal      | Fricative   | Voiceless  | native      | һ        |
| `W`   | /w/  | Labiovelar   | Approximant | Voiced     | native      | у (cons.) |
| `F`   | /f/  | Labiodental  | Fricative   | Voiceless  | **loan**   | ф        |
| `V`   | /v/  | Labiodental  | Fricative   | Voiced     | **loan**   | в        |
| `C`   | /ts/ | Alveolar     | Affricate   | Voiceless  | **loan**   | ц        |
| `Č`   | /tʃ/ | Postalveolar | Affricate   | Voiceless  | **loan**   | ч        |
| `Šč`  | /ɕɕ/ | Alveolopalatal | Fricative | Voiceless  | **loan**   | щ        |
| `ʔ`   | /ʔ/  | Glottal      | Stop        | Voiceless  | (boundary) | (none)   |

`ʔ` is a phoneme-stream **internal marker** for glottal-stop
boundaries (compound-word junctions, hiatus avoidance); it has
no Cyrillic glyph and is inserted/elided by the phonotactic
FST (Layer 0c).

**Loan-only consonants** (`F`, `V`, `C`, `Č`, `Šč`) are
permitted in the alphabet but flagged so lexicon load-time
validation can warn on their appearance in claimed-native
roots.

**Total: 10 vowels + 27 consonants = 37 phonemes.**

### 3.3 Layer 0b — Acoustic realisation

Each phoneme is associated with **two** acoustic artefacts,
stored in a Rust binary format (`bincode` or `rkyv`):

1. **MFCC template** — a `[N_frames × 13]` matrix of mel-cepstral
   coefficients extracted from a clean reference recording.
   Used as the matching target in STT (DTW or Viterbi alignment).

2. **Waveform sample** — the source PCM (16-bit, 16 kHz mono) of
   the reference recording. Used as the synthesis primitive in
   TTS (concatenated with diphone transitions).

In addition, a **diphone bank** (~400 high-frequency phoneme→phoneme
transitions, out of 37×37 = 1369 possible) provides smooth
junctions for synthesis. Recording strategy:

- 2 speakers (1 male, 1 female) for pitch invariance.
- ~30 minutes of total recording yields ~5 instances of each
  phoneme and the ~400 most common diphones.
- All recording done with the same `cpal`-based recorder, so
  the spectral characteristics match production STT input.

### 3.4 Layer 0c — Phonotactic FST

A small FST over the phoneme alphabet enforces:

- **Vowel harmony**: a Front-class vowel may not co-occur with a
  Back-class vowel within a single morphological domain. Loan
  exceptions are flagged at lexicon-load time, not at runtime.
- **Allowed consonant clusters**: a closed list (e.g. `st`, `nt`,
  `rk` are common; `tk`, `pq` are not).
- **Syllable structure**: `(C)V(C)(C)` with restricted final
  cluster set.
- **Assimilation rules**: voicing assimilation at morpheme
  boundaries (the 8-way matrix already documented in
  `01_phonology.md` § 3 — now lifted from grapheme rules to
  phoneme rules).

This FST is the **constraint surface** for any future
constrained-decoding LM (the Spike A direction from the v6.3
brainstorm). A neural component producing one phoneme at a
time is masked by this FST: illegal next-phonemes get `-∞`
logit. **Morphological ill-formedness becomes impossible by
construction.**

### 3.5 Layer 0d — Bidirectional graphics renderer

Two-way mapping between phoneme streams and orthographic forms:

**Phoneme → Cyrillic** (lossy, but deterministic):

```rust
fn phonemes_to_cyrillic(stream: &[Phoneme]) -> String
```

Rules:
- Each non-epenthetic phoneme has a 1-to-1 Cyrillic glyph.
- Each epenthetic phoneme (`Ǝ`, `Ɨ`) renders as «ы» or «і»
  **only when orthographic convention requires it** (consonant
  cluster boundaries).
- Compound digraphs like «и» (= `Ɨ`+`J`) and «у»-as-vowel
  (= `Ʊ`+`W`) are resolved per orthographic context.

**Phoneme → Latin** — analogous, against the post-2025 official
Latin alphabet (or whichever is current in the corpus).

**Cyrillic → Phoneme** (the harder direction — must undo the
orthographic padding):

```rust
fn cyrillic_to_phonemes(s: &str) -> Vec<Phoneme>
```

Rules:
- Each Cyrillic glyph has a default phoneme, **but** «ы» and
  «і» are demoted to epenthetic when adjacent to consonant
  clusters in positions where speakers omit them.
- Default-with-override table populated from corpus evidence.

**This is where the «ы»-problem is properly absorbed**: not as a
runtime hack in the dialog router, but as a one-time projection
at the orthography boundary. Once converted to phonemes, all
downstream code is free of the orthographic artefact.

### 3.6 Lifting layers 1+

The existing morphological FST (`adam-kernel-fst`), lexicon
(`adam-tokenizer`), frame index (`adam-algebra`) and dialog
router (`adam-dialog`) currently key on `&str` (grapheme
sequences). v6.3 replaces those `&str` boundaries with
`&[Phoneme]`.

The transition can be done **incrementally**:

- Phase 1–3 deliver Layers 0a–0d with **no integration**.
- Phase 4+ wires phoneme STT into `adam_chat` as an optional
  backend (env-gate `ADAM_V6_3=1`).
- Phase 5+ lifts each higher layer to phonemes one at a time,
  keeping graphemes as the in-memory form until the lift is
  complete for that layer.
- v6.2 remains the production main branch throughout.

## 4. Pure-Rust audio stack

All audio I/O and DSP done with crates that are pure Rust (no
C dependencies):

| concern              | crate         | notes                          |
|----------------------|---------------|--------------------------------|
| Microphone I/O       | `cpal`        | cross-platform, used by Bevy   |
| Speaker I/O          | `cpal`        | same                            |
| WAV read/write       | `hound`       | mature, ubiquitous              |
| FFT                  | `rustfft`     | fastest pure-Rust FFT           |
| DSP primitives       | `dasp`        | filters, windowing              |
| Resampling           | `rubato`      | high-quality, pure Rust         |
| Pitch detection      | (in-house)    | YIN / cepstral, ~200 LoC         |
| MFCC extraction      | (in-house)    | rustfft + mel filter bank        |
| Voice Activity Det.  | (in-house)    | energy + zero-crossing rate      |

**No** dependency on `whisper.cpp`, `libfvad`, macOS `say`,
`espeak`, or any external binary. The entire audio pipeline is
`cargo build` away.

## 5. Implementation phases

| phase | scope                                                   | gate / acceptance                          | status      |
|-------|---------------------------------------------------------|--------------------------------------------|-------------|
| 1     | Layer 0a + binary phoneme format                        | inventory frozen + round-trip tests pass    | **shipped** |
| 2a    | Corpus collection (see § 6)                              | manifest tracks every source                | **shipped** |
| 2b    | FLEURS-kk_kz ingest (text normalisation built-in)        | 5441 entries, 21.6 h, 40/40 native Cyr     | **shipped** |
| 2c    | Forced alignment for per-phoneme timestamps              | Viterbi DP via Phase 10's aligner           | **shipped** |
| 2d    | MFCC template + diphone bank extraction                  | binary bank files (~12 KB MFCC + 1.8 MB PCM)| **shipped** |
| 2e    | Bank serialisation in pure-Rust format                   | runtime loads bank without MFA / Python     | **shipped** |
| 3     | Layer 0c — phonotactic FST                               | rejects synthetic ill-formed, accepts corpus| **shipped** |
| 4     | Layer 0d — bidirectional renderer (Cyrillic / Latin)     | corpus round-trip ≥ 99%                     | **shipped** |
| 5     | Pure-Rust audio I/O (cpal-based)                         | replaces `voice` feature                    | **shipped** |
| 6     | Phoneme-level STT (DTW + Viterbi)                        | ≥ 70% phoneme-level accuracy on clean Kk    | **infra shipped, accuracy gate NOT met** (PER 84% on FLEURS test, 61% on synth-on-synth; see § 11.1) |
| 7     | Phoneme-level TTS (concatenative + PSOLA)                | MOS within 0.5 of macOS `say`               | **shipped** |
| 8     | Wire `adam_chat` to phoneme STT/TTS                      | v6.3 demo viable; voice-REPL audits pass    | partial     |
| 9     | Lift `adam-kernel-fst` to phoneme input                  | morphology on phonemes; world_core lifted   | **shipped** |
| 10    | Pure-Rust forced aligner (replaces MFA bootstrap)        | T×N Viterbi DP with self-loops              | **shipped** |
| 11    | Speaker normalisation (CMVN / VTLN / per-speaker bank)   | qazaq Wikimedia-probe recovers Q phoneme    | next        |

**Sizing.** No calendar estimates. Each phase ships when its
acceptance gate is met; the order can be shuffled if a phase
blocks (e.g. a corpus source is slow to acquire).

**Phase 11 backlog.** The Phase 10 commit identified speaker
mismatch — not alignment — as the residual blocker on the
`qazaq_pipeline_with_rescore` test. The Wikimedia probe is one
voice; the FLEURS-trained bank is means over 2773 F + 2653 M
speakers, and the probe's Q frames land closer to FLEURS-Z in
cepstral distance than to FLEURS-Q. Three candidate fixes:
cepstral mean/variance normalisation (CMVN — cheapest), vocal-
tract-length normalisation (VTLN), or a per-speaker bank
selected by speaker-ID at recognition time.

**MFA policy.** MFA was originally planned as a Phase 2c
build-time tool, never as a runtime dependency. **Phase 10 has
shipped the pure-Rust replacement** ([adam-forced-aligner](
../crates/adam-forced-aligner/)) — Viterbi DP over a T × N
state machine with self-loops, six unit tests, zero external
deps. The pipeline never installed MFA; the bootstrap path
went straight from equipartition baseline to the pure-Rust
aligner.

## 6. Corpus

**Directive.** Self-recording is **not** the path: studio-grade
dictation is not available to the project and the user does
not want to gate work on finding paid voice talent. Instead,
the bank is built from **all available Kazakh audio in the
public sphere**, regardless of copyright status, treated as
research-fair-use under the non-commercial scope of v6.3.

(See memory: `project_v6_3_corpus_directive`. Licensing
clearance is deferred to the post-result phase; phoneme banks
are **derived** artefacts and do not redistribute the source
audio, so internal use is safe.)

### 6.1 Source classes

- **Mozilla Common Voice Kazakh** — CC0; ~tens of hours;
  pre-paired audio + transcript.
- **ISSAI Kazakh Speech Corpus / KSC2** — open academic
  licence; hundreds of hours of high-quality dictation.
- **kazneb.kz** — Kazakh national digital library; audio books
  with PDF transcripts.
- **YouTube** — «қазақша аудиокітап», «қазақ ертегілері аудио»,
  news anchor channels, theatre recordings. Variable quality;
  manual curation pass on top creators.
- **Radio Шалқар / Qazaq Radiosy** — archived broadcasts.
- **archive.org** — older Kazakh recordings.
- **Public-domain literature** — Abai, Şäkärim, Mahjan,
  Altynsarin (read-aloud editions widely available).

### 6.2 Selection priorities

For phoneme-bank suitability (Phase 2d MFCC averaging), prefer
sources in **this** order:

1. **News anchor / documentary narration** — clearest spectral
   profile, conversational pace.
2. **Modern prose audiobooks** — closer to natural speech than
   poetry.
3. **Children's tales (ертегі)** — clear enunciation, often
   slower pace; good for diphone bank.
4. **Classical poetry** — gives clean phoneme exemplars but
   prosody is hyper-expressive; use as supplement, not primary.

### 6.3 Manifest

Every collected source is tracked in
`data/v6_3_corpus/MANIFEST.jsonl`:

```json
{"source_url": "...", "collected_at": "2026-05-26",
 "format": "mp3", "duration_s": 1843.2, "speaker_count": 1,
 "speaker_gender": "male", "transcript_available": true,
 "transcript_url": "...", "licence_status": "cc0|academic|unknown",
 "used_in_bank": false}
```

`used_in_bank` flips to `true` only after a source passes
the Phase 2c forced-alignment confidence filter
(per-phoneme alignment confidence ≥ 0.7).

### 6.4 Bank size targets

| artefact         | target                                           |
|------------------|--------------------------------------------------|
| Total raw audio  | ≥ 50 hours (well-aligned subset)                 |
| Per-phoneme MFCC | ≥ 100 instances per phoneme, averaged            |
| Diphone bank     | ≥ 400 high-frequency phoneme→phoneme transitions |
| Final binary     | ~5–10 MB serialised                              |

## 7. Acceptance metrics

- **Phase 1:** all 37 phonemes round-trip through Cyrillic and
  Latin renderers losslessly **except** for `is_epenthetic`
  cases (where round-trip may collapse).
- **Phase 3:** vowel-harmony FST rejects 100% of synthetic
  ill-formed sequences and accepts 100% of corpus-evidenced
  well-formed words.
- **Phase 6:** phoneme STT achieves ≥ 70% phoneme-level
  accuracy on a 100-utterance clean-speech test set. (Whisper
  baseline for reference, not as competitor.)
  - **Status 2026-05-29: gate NOT met.** Infrastructure
    shipped (`tools/stt_eval` PER harness; multi-template
    bank with K=50 exemplars; CMVN, forced aligner,
    sonorant-nucleus phonotactics, formant synthesiser, 312-
    entry curated lexicon). Measured PER on FLEURS test
    --max 100 = **84.0 %** (target ≤ 30 %), `hyp/ref` ratio
    0.58 (under-segments). On the homogeneous synth-on-synth
    corpus PER = 61.0 % (also short of 30 %). Two
    architectural negative results recorded: diagonal-
    Gaussian scoring ties Euclidean, K > 50 over-fits synth
    without lifting FLEURS. **Branch should be read as
    "phonemic foundation + STT research" until PER ≤ 30 %.**
    Next planned lever per codex review (2026-05-29):
    lexicon-constrained decoding + phonotactic transition
    mask + duration priors; only fall back to a small
    CTC/Conformer (output: phoneme lattice + confidence)
    if the pure-template path plateaus above 50 % PER.
- **Phase 7:** in a blind subjective MOS test (5 listeners),
  the concatenative TTS scores within 0.5 MOS points of macOS
  `say` with the Aru voice on 10 representative Kazakh
  sentences.
- **Phase 8 (v6.3 viable):** the existing voice-REPL audit
  battery (sessions 1–5 combined) passes with `ADAM_V6_3=1`
  with at most 2 regressions vs `ADAM_V6_2=1`.

## 8. Risks and honest limitations

1. **STT accuracy will trail Whisper at first.** Whisper-large-v3-turbo
   was trained on thousands of hours; our phoneme-level DTW will be
   trained on ~30 minutes. On clean speech this is fine; on noisy
   or accented speech it will lose. **Compliance framing:** "works
   locally, no cloud, no C dependencies, deterministic." That is
   genuine value, but it does not survive being framed as "better
   than Whisper."
2. **Diphone synthesis sounds robotic.** PSOLA (Phase 7) buys
   prosody but no naturalness beyond classic concatenative TTS.
   Native-grade naturalness requires neural TTS (e.g. VITS, FastSpeech)
   which is outside the v6.3 deterministic scope. Accept the
   trade-off or scope-extend Phase 7.
3. **Self-recorded corpus is one speaker pair.** Generalisation to
   other accents and timbres is limited until Phase 6 extends with
   Common Voice. Demo videos with the recording speaker will
   sound much better than demos with arbitrary users.
4. **Phase 9 (lifting morphology to phonemes) touches the entire
   stack.** This is where the actual engineering risk concentrates:
   all curated `data/world_core/*.jsonl` content is currently
   Cyrillic-graphemic. The lift must convert (or dual-key) every
   curated fact. Plan a parallel migration script in Phase 8.
5. **v6.2 must stay alive as production main throughout.** All v6.3
   work happens on `experimental/v6_3_phonemic_foundation`; only
   when Phase 8 passes its acceptance gate does it become merge-eligible.

## 9. Open questions

1. ~~Self-recorded vs Common Voice for Phase 2?~~ **Resolved
   2026-05-26.** Decision: public-domain + free-access Kazakh
   audio at scale (Common Voice + KSC + audiobooks + radio
   archives), with MFA bootstrap forced alignment. See § 6.
2. ~~Latin or Cyrillic as primary renderer in v6.3 demos?~~
   **Resolved 2026-05-26.** Decision: **Cyrillic primary**.
   Reasons: (a) current official orthography of the Republic
   of Kazakhstan in 2026; (b) the entire `data/world_core/*.jsonl`
   curated corpus is in Cyrillic; (c) the post-2025 official
   Latin alphabet is not yet stabilised. The Latin renderer
   ships in Layer 0d for completeness but is not the demo
   default. v6.3 demo screens render Cyrillic.
3. **Phoneme-level STT: pure DTW vs. small Conformer/CTC neural
   net?** Pure DTW is fully deterministic but plateaus around
   75–80% accuracy. A 5M-param Conformer trained on the much
   larger corpus from §6 (Common Voice + KSC + audiobooks
   adds up to ~1000+ hours) would lift accuracy to 90%+ and
   introduces a learned component compatible with the v6.3
   brainstorm's Spike A (constrained decoding LM) and Spike C
   (energy-based verifier) — both already assume a small
   neural component. **Recommended path:** Phase 6a delivers
   pure DTW for the deterministic floor; Phase 6b adds a tiny
   Conformer trained on the aligned corpus, guarded by the
   phonotactic FST so output remains type-safe.
4. ~~What does «ы» do in dictionary-form lookup?~~ **Resolved
   2026-05-26, tightened 2026-05-29.** The original 2026-05-26
   resolution drew a three-clause line (non-initial + between
   consonants + native root). Implementation hit a 2026-05-29
   user pushback that the carve-out for initial-syllable «ы»
   was the same orthographic illusion native speakers actually
   collapse — «қыз» is articulated as `/qz/`, with a fleeting
   schwa-ish glide if any, and «қызыл» as `/qzl/`. The Renault
   analogy (7 letters, 4 phonemes) applies symmetrically: «ы»
   and «і» are **orthographic markers, not phonemes**, and the
   parser must not emit them.

   **Final v6.3 rule (this is what the code does):** for any
   native Kazakh root, **every** «ы» and «і» drops from the
   phonemic stream, regardless of syllable position. Loan
   words (`is_native_root = false`) keep both glyphs because
   the loan source language treats them as full vowels.

   | word         | analysis                          | parsed phonemes      |
   |--------------|-----------------------------------|----------------------|
   | қыз          | native                            | `[Q, Z]`             |
   | қызыл        | native                            | `[Q, Z, L]`          |
   | жұмыс        | native                            | `[Zh, U, M, S]`      |
   | жұмыссыз     | native, two «ы»s                  | `[Zh, U, M, S, S, Z]`|
   | білім        | native, two «і»s                  | `[B, L, M]`          |
   | байтұрсынұлы | native, two «ы»s incl. final      | `[B, A, J, T, U, R, S, N, U, L]` |
   | бизнес       | loan                              | `[B, I, Z, N, E, S]` |

   `Phoneme::Y` / `Phoneme::Yi` stay in the inventory enum (for
   loan-word parsing and back-compat with suffix tables in
   `adam-kernel-phoneme`), but the Cyrillic→Phoneme and Latin→
   Phoneme parsers never emit them in native mode. Consonant-
   only words like `[Q, Z]` validate via the **sonorant /
   last-consonant nucleus fallback** added in the same change
   (see `adam-phonotactics::syllable::nucleus_indices`).

## 10. References

- Поппе Н. Н., *Введение в алтайское языкознание* (1965) —
  classic treatment of Turkic phonology incl. epenthetic vowels.
- Баскаков Н. А., *Тюркские языки* (1960) — comparative
  framework.
- Кайдар А., *Структура односложных корней и основ в казахском
  языке* (1986) — Kazakh-specific phonotactics.
- ISSAI / Nazarbayev University Kazakh Speech Corpus (KSC) —
  https://issai.nu.edu.kz/kz-speech-corpus/
- Common Voice Kazakh — https://commonvoice.mozilla.org/kk
- Existing v6.x phonology notes — `docs/kazakh_grammar/01_phonology.md`
- v6.2 architectural pivot — `docs/v6_2_architectural_redesign.md`

## 11. Sign-off

**Status 2026-05-26:** All four open questions in §9 are
resolved. The phoneme inventory (§3.2) is finalised at 37
units. This document is the **signed design contract** for
v6.3 implementation.

**Next actions** (no calendar — gates only):
1. Cut branch `experimental/v6_3_phonemic_foundation` from
   `main` at the design-sign-off commit.
2. Begin **Phase 1**: create `crates/adam-phoneme/` with the
   typed phoneme enum, the attribute table, the
   `Phoneme::cyrillic_default()` projection, the «ы»/«і»
   epenthetic rule from §9 OQ4, and round-trip unit tests
   against a small word list from `data/world_core`.
3. Begin **Phase 2a** in parallel (no Rust dependency): start
   acquiring corpus per §6.1 priorities, populate
   `data/v6_3_corpus/MANIFEST.jsonl`.

The remaining phases (2b–10) gate sequentially on their
predecessors' acceptance criteria. v6.2 stays production main
throughout; v6.3 is merge-eligible only after Phase 8 passes.
