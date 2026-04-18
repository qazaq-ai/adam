# Adam v1.0.0 Architecture — Deterministic Agglutinative Language Model

Status: **proposal** (Phase 0 — pre-implementation study)
Target: v1.0.0 (replaces the v0.3-v0.5 pure-transformer lineage after honest-assessment pivot on 2026-04-18)

## 1. Premise — What's wrong with the v0.3-v0.5 approach

The v0.3–v0.5 line of work treated Kazakh as a generic string to be modelled by a transformer trained on BPE tokens. This failed consistently:

| run | params | data | val PPL | generations |
|---|---|---|---|---|
| v0.3.0 | 20M | 606k tokens (synth-heavy) | 871 | 3-4 complete sentences / 30 |
| v0.4.0 | 24M | 4.09M tokens (mixed) | 1692 | 6 complete sentences / 30 |
| v0.5.0 failed | 27M | 3.9M tokens | 1811 | 0 complete sentences / 30 |
| v0.5.0 shipped-no | 24M | 4.09M (cleaner) | 1502 | 6, with fragment noise |
| v0.5.1 fast-iter | 3.4M | 848k (FSM≥70%) | **103.94** | **0 complete** (memorised `-ды` trick) |

Pattern: stochastic transformers on small Kazakh corpora either under-train (lumpy garbage) or over-memorise narrow patterns (PPL looks great, generations are trivial). Throwing hand-crafted lexicon work at the problem from 211 → 4454 roots did not close the gap.

## 2. Root cause — a theoretical observation

Kazakh morphology is **deterministic**. Given a root and a feature bundle `{number, possessive, case, tense, person, voice, negation}`, the surface form is computable by a finite-state transducer — one valid output, O(length). This has been textbook computational linguistics since Koskenniemi (1980s); Apertium, HFST, and FOMA are production tools that demonstrate it at scale for many languages including Turkic.

Transformers trained from scratch waste model capacity **re-learning** what morphology books already encode:
- Vowel harmony (front/back, sometimes rounded/unrounded)
- Consonant assimilation at morpheme boundaries
- Suffix ordering constraints (fixed canonical sequence)
- Voicing alternations, devoicing before voiceless consonants

These are **functions**, not probability distributions. Any time our transformer produces `жылнеор` instead of a well-formed Kazakh word, it's fighting against rules we could just encode directly.

**The v0.5.1 "-ды trick" is the smoking gun**: a 3.4M model on 848k heavily-filtered tokens converged on a single pattern (emit `-ды` and stop). Because the tokeniser hid morpheme boundaries behind BPE subwords, the model could not see the real structure; it saw high-frequency endings and over-fit them.

## 3. Proposed architecture — hybrid FST + minimal LM

```
                    ┌────────────────────────────────────────┐
                    │          INPUT: Kazakh text            │
                    └────────────────────┬───────────────────┘
                                         │
             ┌───────────────────────────▼───────────────────────────┐
             │  FST PARSER (deterministic, O(n), pure Rust)          │
             │  input:  "мектептерімізде"                            │
             │  output: (root=мектеп, noun,                          │
             │           [plural=ler, poss_1pl=імізде,               │
             │            case=locative])                            │
             └───────────────────────────┬───────────────────────────┘
                                         │
             ┌───────────────────────────▼───────────────────────────┐
             │  SEQUENCE: list of (root_id, feature_bundle) tuples   │
             │  This is the real "vocabulary" for the LM.            │
             └───────────────────────────┬───────────────────────────┘
                                         │
             ┌───────────────────────────▼───────────────────────────┐
             │  LM OVER ROOTS (small transformer, 5-20M params)      │
             │  vocab = #roots (10-50k) + #feature_combos (~500)     │
             │  trains on a factorised representation, not chars     │
             └───────────────────────────┬───────────────────────────┘
                                         │
             ┌───────────────────────────▼───────────────────────────┐
             │  FST SYNTHESISER (deterministic, O(1), pure Rust)     │
             │  input:  (root=бала, noun, [plural, dat_case])        │
             │  output: "балаларға"  ← guaranteed morphologically    │
             │                         correct by construction       │
             └───────────────────────────┬───────────────────────────┘
                                         │
                    ┌────────────────────▼───────────────────┐
                    │        OUTPUT: Kazakh text             │
                    └────────────────────────────────────────┘
```

### Why this is fundamentally different

- **No possibility of morphologically invalid output.** The synthesiser only accepts legal feature bundles; if the LM emits a nonsense bundle, the synthesiser rejects it and the LM re-samples. No `адамніңгеем` can leave the system.
- **LM capacity goes to semantics, not orthography.** With morphology externalised, a 5M model has roughly the effective capacity of a 100M transformer that had to learn suffix patterns from scratch.
- **O(n) inference, not O(n²).** Attention is optional in the root-LM — a GRU or small transformer suffices because the sequence is short (each step is a whole morphological word, not 2-5 subtokens).
- **Predictability & safety.** Grammatical hallucination is impossible. Semantic hallucination becomes a bounded LM problem on a small, curated root vocabulary.
- **Small disk / small compute.** Entire pipeline fits on MacBook Air M2 8GB for both training and inference. No GPU cluster required.

## 4. Engineering scope

| component | size | effort |
|---|---|---|
| FST phonological rules (vowel harmony, assimilation) | ~30 rule templates × ~5 variants | 1-2 weeks |
| Root lexicon | 10k-50k entries | 2-4 weeks (importable from Apertium-kaz + manual curation) |
| Suffix inventory (noun + verb + other) | ~150-200 morphemes | 1 week |
| Suffix ordering / cooccurrence grammar | ~50 state-machines per POS | 1-2 weeks |
| Training data generation (pairs: surface ↔ analysed) | derived from corpus via FST parser | 1 week |
| Root-LM (Rust, candle or custom) | 5-20M params | 1 week model code + training cycles |
| Integration + end-to-end inference | — | 1 week |
| **Total** | | **~8-12 weeks calendar** |

## 5. Dependency on existing work

Keep:
- `adam-kernel` FSM foundations — extend, don't discard
- `adam-tokenizer` morpheme-aware entry points — repurpose as FST parser
- Current 2585-root curated lexicon — seed for expansion
- `data/tokenizer/segmentation_rules.json` — seed grammar rules
- Existing corpus and evaluation infrastructure

Discard (for v1.0.0 line):
- BPE-as-primary-tokeniser — BPE becomes optional fallback for unknown words only
- End-to-end transformer trained on subword ids — replaced by root-LM
- Treating morphological correctness as a soft objective

## 6. Reference architectures to study

- **Apertium** — rule-based MT for ~50 languages including `kaz`. Shows production FST at scale.
- **HFST (Helsinki Finite-State Toolkit)** — academic FST infra, Kazakh morphology modules exist.
- **FOMA** — simpler FST compiler.
- **Koskenniemi 1983, "Two-Level Morphology"** — foundational text.
- **Beesley & Karttunen 2003, "Finite State Morphology"** — standard reference.
- **AKO Corpus / Kazakh Academy of Sciences grammar** — authoritative source.

None of these need to ship — we study them to inform our Rust implementation.

## 7. What the first four weeks look like (daily work, 10h Astana time)

**Week 1 — Phonology + survey**
- Read standard grammars (Ysqakov, Mamanov)
- Write `01_phonology.md` with complete vowel harmony + consonant assimilation tables
- Catalogue Apertium-kaz rule files and extract `.lexc`/`.twolc` reference

**Week 2 — Morphology inventory**
- Write `02_morphology.md` with all noun/verb suffixes + ordering constraints
- Compare against our current 422 segmentation rules; plan additions
- Identify irregular forms and allomorphy patterns

**Week 3 — Lexicon**
- Import Apertium-kaz lemma list; map to our JSON schema
- Manual pass to verify / correct ~1000 most-frequent entries
- Write `03_lexicon_sources.md` with attribution

**Week 4 — FST design**
- Design Rust types for two-level morphology
- Spec: `02_fst_rust_design.md`
- Unit-test skeleton for rule validators
- No training yet — just solid foundations

Weeks 5-8 implement; weeks 9-12 train + iterate.

## 8. Non-goals for v1.0.0

- Hitting parity with ChatGPT-class models. Not the target.
- Supporting scripts other than Cyrillic.
- Multilingual capability.
- Speech or vision.

v1.0.0 target is a **predictable, grammatically-correct, deployable-on-laptop Kazakh language model** for short-context generation and analysis. Everything else is v2.0.0+.

## 9. Success criteria

- 100% grammatical outputs (by construction — no ill-formed morphology possible)
- Val perplexity on **root-level** task (not char-level) meaningful — the old PPL numbers become obsolete as a comparison
- Generation of coherent multi-word phrases ≥10 words in 80%+ of prompts
- Inference latency <50ms/word on M2 CPU
- Lexicon: 10k+ verified roots
- Coverage: 90%+ of real Kazakh text segmentable via FST (no char-fallback)
