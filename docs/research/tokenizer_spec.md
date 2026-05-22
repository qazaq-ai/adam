# Спецификация: морфемный токенайзер `adam-agg-tokenizer`

**Дата:** 2026-05-15 (Phase 0 Day 3).
**Crate:** `crates/adam-agg-tokenizer/` (новый).
**Базируется на:** `adam-kernel-fst` (production FST + Lexicon).

> Цель: дать первую работающую реализацию **типизированного морфемного токенайзера** для казахского, прошедшую FST round-trip-тесты на 100+ словах.

---

## 1. Public API

```rust
pub struct AggTokenizer {
    vocab: Vocab,
    lex: LexiconV1,
}

impl AggTokenizer {
    /// Build tokenizer from lexicon (consumes lexicon for ownership).
    pub fn build(lex: LexiconV1) -> Self;

    /// Tokenize a single surface word into morpheme sequence.
    /// Returns Vec<MorphToken> in canonical FST-analysis order.
    /// First analysis (by determinism contract) is taken; for OOV
    /// words returns single UNK token.
    pub fn tokenize_word(&self, word: &str) -> Vec<MorphToken>;

    /// Tokenize a full sentence (whitespace-split + word-tokenize each).
    /// Service tokens (BOS, EOS, SPACE) inserted at boundaries.
    pub fn tokenize_sentence(&self, sentence: &str) -> Vec<MorphToken>;

    /// Detokenize: morpheme sequence → surface word.
    /// Uses FST synthesise.
    pub fn detokenize_word(&self, tokens: &[MorphToken]) -> Result<String, DetokError>;

    /// Round-trip: tokenize then detokenize. Returns Ok(surface_eq_input)
    /// or Err if word OOV or FST disagreement.
    pub fn round_trip(&self, word: &str) -> Result<bool, RoundTripError>;

    /// Vocab introspection.
    pub fn vocab(&self) -> &Vocab;
}
```

---

## 2. Vocab structure

### 2.1 Token enum

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MorphToken {
    /// Root morpheme. id = position in Lexicon entries_ordered.
    Root { id: u32, root: String, pos: PartOfSpeech },

    /// Suffix morpheme (one variant per FST feature variant).
    Suffix(SuffixKind),

    /// Service tokens.
    Bos,
    Eos,
    Pad,
    Space,
    Unk { surface: String },

    /// Punctuation (one variant per common KZ punct).
    Punct(char),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SuffixKind {
    // Noun side
    Number(Number),                  // Singular | Plural
    Possessive(Possessive),          // P1Sg | P2SgInformal | ...
    Case(Case),                      // Nominative | Genitive | ...
    Predicate(Predicate),            // P1Sg | P2SgInformal | ... (copula)
    Derivation(Derivation),          // various

    // Verb side
    Voice(Voice),                    // Active | Passive | ...
    Negation,                        // boolean → unary token
    Tense(Tense),                    // PastDefinite | ...
    Person(Person, bool),            // (Person, polite_flag)
    // Number on verbs reuses Number variant above
}
```

### 2.2 Vocab size breakdown

| Category | Variants | Count |
|---|---|---|
| Root tokens | one per Lexicon entry | ~25 500 |
| Number | Singular (implicit/identity, not emitted) + Plural | 1 |
| Possessive | 7 | 7 |
| Case | 8 | 8 |
| Predicate | 7 | 7 |
| Derivation | ~10 | 10 |
| Voice | 5 | 5 |
| Negation | 1 | 1 |
| Tense | 14 | 14 |
| Person | 6 × (polite ∈ {true,false}) | 12 |
| Service (BOS/EOS/PAD/SPACE/UNK) | 5 | 5 |
| Punct | common KZ punctuation | ~20 |
| **Total** | | **~25 590** |

(Closer to my pre-lit estimate of 30k than initially thought — но всё ещё значительно меньше чем BPE-словарь типичной LLM.)

### 2.3 ID assignment

```
0           — Pad
1           — Bos
2           — Eos
3           — Space
4           — Unk (special slot; multiple Unk's share this id, surface stored in token)
5-24        — Punct (top 20 KZ punct chars)
25-99       — Suffix tokens (75 variants, includes all SuffixKind combinations)
100-25600   — Root tokens (one per Lexicon entries_ordered slot, in order)
```

Это даёт стабильное `id → token` mapping, derivable из Lexicon size + suffix enum cardinalities at build time.

---

## 3. Tokenize algorithm

```rust
fn tokenize_word(&self, word: &str) -> Vec<MorphToken> {
    let analyses = adam_kernel_fst::parser::analyse(word, &self.lex);
    
    let first = match analyses.into_iter().next() {
        Some(a) => a,
        None => return vec![MorphToken::Unk { surface: word.to_string() }],
    };

    let mut tokens = Vec::new();
    match first {
        Analysis::Noun { root, features } => {
            tokens.push(MorphToken::Root { id: lookup_root_id(&root), root: root.root.clone(), pos: PartOfSpeech::Noun });
            if let Some(d) = features.derivation { tokens.push(MorphToken::Suffix(SuffixKind::Derivation(d))); }
            if features.number == Some(Number::Plural) { tokens.push(MorphToken::Suffix(SuffixKind::Number(Number::Plural))); }
            if let Some(p) = features.possessive { tokens.push(MorphToken::Suffix(SuffixKind::Possessive(p))); }
            if let Some(c) = features.case { tokens.push(MorphToken::Suffix(SuffixKind::Case(c))); }
            if let Some(p) = features.predicate { tokens.push(MorphToken::Suffix(SuffixKind::Predicate(p))); }
        }
        Analysis::Verb { root, features } => {
            tokens.push(MorphToken::Root { id: lookup_root_id(&root), root: root.root.clone(), pos: PartOfSpeech::Verb });
            if let Some(v) = features.voice { tokens.push(MorphToken::Suffix(SuffixKind::Voice(v))); }
            if features.negation { tokens.push(MorphToken::Suffix(SuffixKind::Negation)); }
            if let Some(t) = features.tense { tokens.push(MorphToken::Suffix(SuffixKind::Tense(t))); }
            if let Some(p) = features.person { tokens.push(MorphToken::Suffix(SuffixKind::Person(p, features.polite))); }
        }
    }
    tokens
}
```

**Key invariants:**

1. **Deterministic:** для одного и того же word → одна и та же последовательность tokens (FST `analyse` уже даёт stable ordering).
2. **Canonical order:** для noun — `root → derivation → number → possessive → case → predicate`; для verb — `root → voice → negation → tense → person`. Совпадает с FST morphotactic stacking.
3. **OOV-safe:** слова не из Lexicon → single Unk token, surface preserved.

---

## 4. Detokenize algorithm

```rust
fn detokenize_word(&self, tokens: &[MorphToken]) -> Result<String, DetokError> {
    let root_token = tokens.first().ok_or(DetokError::Empty)?;
    
    match root_token {
        MorphToken::Root { root, pos: PartOfSpeech::Noun, .. } => {
            let mut features = NounFeatures::default();
            for tok in &tokens[1..] {
                if let MorphToken::Suffix(kind) = tok {
                    match kind {
                        SuffixKind::Derivation(d)  => features.derivation = Some(*d),
                        SuffixKind::Number(n)      => features.number = Some(*n),
                        SuffixKind::Possessive(p)  => features.possessive = Some(*p),
                        SuffixKind::Case(c)        => features.case = Some(*c),
                        SuffixKind::Predicate(p)   => features.predicate = Some(*p),
                        _ => return Err(DetokError::InvalidSuffixForNoun(*kind)),
                    }
                }
            }
            Ok(synthesise_noun(root, features))
        }
        MorphToken::Root { root, pos: PartOfSpeech::Verb, .. } => {
            let mut features = VerbFeatures::default();
            for tok in &tokens[1..] {
                if let MorphToken::Suffix(kind) = tok {
                    match kind {
                        SuffixKind::Voice(v)  => features.voice = Some(*v),
                        SuffixKind::Negation  => features.negation = true,
                        SuffixKind::Tense(t)  => features.tense = Some(*t),
                        SuffixKind::Person(p, polite) => { features.person = Some(*p); features.polite = *polite; }
                        _ => return Err(DetokError::InvalidSuffixForVerb(*kind)),
                    }
                }
            }
            Ok(synthesise_verb(root, features))
        }
        MorphToken::Unk { surface } => Ok(surface.clone()),
        _ => Err(DetokError::NotARoot),
    }
}
```

---

## 5. Round-trip property

```rust
fn round_trip(&self, word: &str) -> Result<bool, RoundTripError> {
    let tokens = self.tokenize_word(word);
    let reconstructed = self.detokenize_word(&tokens)?;
    Ok(reconstructed == word)
}
```

**Test expectation:** на 100+ казахских словах из Lexicon round_trip должен возвращать `Ok(true)` ≥ 95%. Допустимые failures:
- Слова с derivational morphology, которую parser не покрывает (legacy).
- Ambiguous parses, где FST берёт первую analysis, а surface match'ит другую.

5% — workable floor для PoC. В Phase 1 будем поднимать к 99%.

---

## 6. What's NOT in scope for Phase 0 prototype

- **Multi-word phrases / sentences** — tokenize_sentence — реализуем минимально (whitespace split + BOS/EOS/SPACE), но без punctuation handling beyond basic.
- **Stemming для OOV** — Unk остаётся Unk; не пытаемся guess root.
- **Vocab serialization** — `Vocab` строится in-memory из LexiconV1; persist в JSON в Phase 1.
- **Embedding-table** — это уже Phase 2 (training). Tokenizer только outputs token IDs.
- **Subword fallback** — если слово не в Lexicon, всё единый Unk. Byte-level fallback — Phase 1.

---

## 7. Тесты Day 3

Минимум для commit'а:

1. **Vocab size sanity:** `vocab.size() ≥ 25500 + 50`.
2. **Root token round-trip:** ~20 чистых root'ов (бала, ат, кітап, ...) → tokenize → detokenize → equal.
3. **Inflected forms round-trip:** ~50 склонённых форм из existing FST tests (балалар, балаға, кітабым, кітаптарымыз, оқыдым, оқымаймын, ...).
4. **Service tokens:** BOS/EOS preserved through sentence tokenization.
5. **OOV graceful:** «Whatever» → single Unk, не crash.
6. **Determinism:** один word → один результат (5 calls, all equal).

**Target:** все тесты pass + 95%+ round-trip success на 100-word sample из data/lexicon_v1/.

---

## 8. Структура crate'а

```
crates/adam-agg-tokenizer/
├── Cargo.toml
├── src/
│   ├── lib.rs           — public API, AggTokenizer, MorphToken, SuffixKind
│   ├── vocab.rs         — Vocab struct, id assignment
│   ├── tokenize.rs      — tokenize_word, tokenize_sentence
│   ├── detokenize.rs    — detokenize_word
│   └── error.rs         — DetokError, RoundTripError
└── tests/
    ├── round_trip.rs    — 100-word battery
    ├── service.rs       — BOS/EOS/Unk
    └── determinism.rs   — repeated calls
```

Идём кодить.
