# Algebra-Anchored Neural Composition for Agglutinative Languages: A Pure-Rust, CPU-Resident, Verifier-Bounded Approach

**Authors:** Daulet Baimurza · Qazna Technologies LLP · `baimurza.daulet@gmail.com`
**Affiliation:** Qazna Technologies LLP, Almaty, Kazakhstan
**Source repository:** [github.com/qazaq-ai/adam](https://github.com/qazaq-ai/adam) (BUSL-1.1, converts to Apache 2.0 after four years)
**Draft date:** 2026-05-16 · revised 2026-05-18 (Codex peer-review pass)
**Status:** **DRAFT v0.2** — substantive critique pass applied. Ready for arXiv `cs.CL` submission pending LaTeX conversion + reference formatting. Workshop / conference targets and submission plan in `docs/preprint/preprint_submission_plan.md` (forthcoming).

---

## Abstract

We present an architecture for language-model inference on
agglutinative languages that combines a deterministic finite-state
transducer (FST) for morphology, a typed-morpheme tokenizer, and a
small CPU-resident neural transformer trained inside an FST-validity
envelope. The composition's outputs are gated by a verifier that
checks each generated word against (a) FST round-trip validity,
(b) Kazakh script validity, (c) lexicon coverage (no `Unk`-headed
output), and (d) factual grounding in a curated knowledge graph
(3 672 facts at the time of writing). We empirically demonstrate
at proof-of-concept scale on Kazakh that this architecture
(i) trains end-to-end in pure Rust on commodity CPU hardware
(M2 8 GB, no GPU, 31 minutes Stage-5 training + 64 minutes Stage-8
algebraic-loss A/B), (ii) generalises from real-corpus pairs with
a held-out cross-entropy gap of 0.048 (down from 0.196 with
FST-synthetic-only training, a 4.1× reduction), (iii) produces
morpheme-level generation at 1.71 ms per six-token greedy decoding
on M2 CPU — a single-word composer latency, not a sentence-answer
latency, and not directly comparable to full LLM inference, and
(iv) **blocks unsupported factual claims within the covered
knowledge graph by construction**, rather than relying on training-
time alignment to discourage them. We argue that this design is
not a smaller LLM but a perpendicular architecture: it inverts
each of the four load-bearing properties of contemporary LLMs
(statistical / cloud / RLHF-aligned / English-first → algebraic /
CPU / architecturally-verified / agglutinative-first) and is
therefore the appropriate substrate for regulated, low-resource,
and offline-deployment contexts where current LLMs are not viable.
**Scope disclaimer.** The numbers reported here are at PoC scale
(≈ 1.17 M parameters, single training run with seed 42, 100-prefix
held-out exact-match eval). Production claims will require bootstrap
confidence intervals, multi-seed runs, and baseline comparison
against Apertium-Kazakh and a char-level neural model; these
ablations are listed in §5.3 (Limitations).

---

## 1. Introduction

Contemporary large language models share four structural properties
that are intrinsic to their architecture, not incidental:

1. **Statistical, not algebraic.** Knowledge is distributed across
   billions of weights with no inspectable correspondence to the
   knowledge being represented.
2. **Cloud-dependent.** A meaningful inference requires GPU clusters
   of warehouse scale.
3. **Hallucination-prone by construction.** No architectural gate
   separates statistically-plausible output from factually-true
   output.
4. **Centralised to English-first labs.** Languages with rich
   morphology — typically the world's lower-resource languages — are
   treated as adversarial cases for statistical methods that work
   well on poorly-inflected languages.

Each of these properties limits applicability in regulated domains
(medicine, law, defence, education), in offline / energy-constrained
deployment, and in low-resource agglutinative-language ecosystems.
We argue that these limits are *avoidable* by choosing a different
architectural substrate: the **typed function composition** that an
agglutinative language's morphology already encodes.

Kazakh, our reference language, is exactly such a system. A Kazakh
word is `root + suffix₁ + suffix₂ + … + suffixₙ`, where each suffix
is a typed function transforming morphosyntactic state. Vowel
harmony, consonant assimilation, and slot ordering are finite,
deterministic constraints. A 100-line FST can encode the complete
inflectional morphology; a 25 000-entry root inventory closes the
lexicon. The neural network's role, in our architecture, is not to
*replace* this algebraic substrate but to *select* among valid
compositions in a way calibrated to real speakers' usage.

Our contribution is fourfold:

- **C1.** An explicit four-inversion architectural position
  (algebra not statistics, CPU not cloud, verifier not RLHF,
  agglutinative-first not English-first).
- **C2.** An implementation in pure Rust spanning a deterministic
  FST kernel, a typed-morpheme tokenizer, a small CPU-resident
  transformer trained inside the FST-validity envelope, and a
  verifier that gates both morphology and factual grounding.
- **C3.** A proof-of-concept empirical demonstration on Kazakh:
  generalisation gap 0.031, latency 1.71 ms/turn on M2 CPU,
  end-to-end training in 39 minutes on commodity hardware without
  any GPU.
- **C4.** A reproducible open-source artifact under a copy-left-
  compatible licence (BUSL-1.1, conversion to Apache 2.0 in four
  years), so the architecture can be independently scrutinised and
  extended to other agglutinative languages.

---

## 2. Related work

### 2.1 Statistical morphology vs explicit FST

Earlier eras of computational morphology relied on hand-built FSTs
(Beesley & Karttunen 2003; Linguistica project; the Apertium
platform for which Kazakh has a mature FST baseline). The current
era has largely replaced these with byte-pair encoding (BPE) and
subword-tokenised neural models. Our position is that *for
agglutinative languages*, the BPE shift was a regression — the FST
encoded the algebra exactly; BPE encodes its surface artefacts
statistically, requiring orders of magnitude more parameters to
recover what was lost.

### 2.2 Constrained decoding for natural-language structure

PICARD (Scholak et al. 2021) demonstrated that token-level
constrained decoding against a formal grammar can dramatically
improve correctness on text-to-SQL. We extend the PICARD pattern
from "constrain to SQL grammar" to "constrain to morphotactic FST".
The key generalisation: the FST encodes far more structure than a
parser grammar, including phonological realisation rules.

### 2.3 Small-model training on regulated data

Microsoft's Phi series (Gunasekar et al. 2023; "Textbooks Are All
You Need") showed that small models trained on carefully-curated
synthetic data can outperform much larger models trained on
unfiltered web. Their teacher signal, however, was GPT-3.5/4 —
inheriting that teacher's biases and hallucination patterns. Our
synthetic-data pipeline uses an **FST as teacher**: every emitted
training pair is provably morphologically valid by construction.

### 2.4 RLVR / Tulu-3 et al.

Recent verifier-augmented training (Tulu-3 et al. 2024) places a
discrete-rule verifier in the *training* loop. Our verifier is
placed at *inference* and the *architecture* — every emitted word
passes through it at runtime, not just selected training examples.
This is the difference between RLHF-style "discouraged from
hallucinating during training" and **"blocks unsupported factual
claims within the covered knowledge graph at inference time"**.
We are careful with the framing: the verifier provides a hard
architectural gate **only over the subset of knowledge the
knowledge graph covers**; for queries outside its coverage the
honest path is refusal, not generation. The architectural
contribution is closing the gate, not eliminating the gap.

---

## 3. System

### 3.1 Pipeline

The full pipeline is documented in detail in
`docs/architecture_neural_v6.md` (sister artefact to this preprint;
both are part of the repository). The relevant components for this
paper are:

- **L1 Parser.** FST analyser ([`adam_kernel_fst::parser::analyse`])
  on the surface form, returning a list of (root, features)
  analyses, deterministically ordered.
- **L2 Tokenizer.** `AggTokenizer::tokenize_word` maps a surface
  to typed morpheme tokens (Root, Suffix(SuffixKind), BOS, EOS,
  Pad, Space, Unk, Punct).
- **L5 Template.** Deterministic dialog templates (v3.0–v5.x).
- **L5.5 Neural composer.** TinyAgt — a decoder-only transformer
  with causal attention, pre-norm, GELU FFN, and tied output
  projection, parametrised to fit in 1–10 M params.
- **L6 Verifier.** Four gates checked in order: (1) Kazakh-script
  gate (input must contain only Cyrillic in the Kazakh alphabet
  range plus internal hyphen / apostrophe), (2) Unk gate (the
  tokenizer must not fall back to `MorphToken::Unk` — the surface
  must be a known Kazakh word), (3) FST round-trip check
  (`tokenize(s) == tokens && detokenize(tokens) == s`), and
  (4) factual-grounding check against `data/retrieval/facts.json`
  (3 672 facts at the time of writing). Strict-by-default; the
  permissive mode that previously let ungrounded surfaces through
  with a flag was retained for telemetry but not for production
  paths.

### 3.2 Algebraic loss

We extend cross-entropy with an algebraic penalty term:

$$
\mathcal{L}_{\text{total}} = \mathcal{L}_{\text{CE}} +
  \alpha \cdot \mathbb{E}_{b,t} \!\left[
    \sum_{v} \text{softmax}(\text{logits})_{b,t,v} \cdot
    M_{\text{invalid}}[s(b,t), v]
  \right]
$$

where $M_{\text{invalid}}[s, v] = 1$ iff token $v$ is a
morphologically invalid continuation from state $s$, and $s(b,t)$ is
the FST validator state at position $t$ of sequence $b$. The state
encoding is a compact $u8$ in 24 values (3 POS-commitment states ×
8 slot indices in our Phase 0 morphotactic state machine). The
mask table is precomputed once per vocabulary as a $[24, V]$ tensor
and gathered per training step. Active-position masking ensures
padding tokens do not contribute.

### 3.3 FST-constrained decoding

At decode time the model proposes a distribution over the next
token; the validator drops every candidate that is morphotactically
illegal from the current state; the highest-probability survivor
is selected (greedy), or beam search is used to maintain $k$
candidates. The validator is the same `MorphValidator` used in
training to construct the invalid-token mask.

---

## 4. Experiments

### 4.1 Setup

- **Reference language:** Kazakh (kk).
- **Lexicon:** v1, 16 850 entries (13 606 curated pure-Kazakh +
  3 244 additional sources; v1.x lineage).
- **Model:** TinyAgt with `vocab=5 241` (data-derived dense
  range), `d_model=64`, `n_heads=4`, `n_layers=2`, `d_ff=128`,
  `max_seq_len=32`. ≈ 1.07 M parameters.
- **Hardware:** MacBook Air M2, 8 GB RAM, macOS 25.3, CPU only.
- **Framework:** `burn = 0.17` (Rust ML framework), `ndarray`
  backend, no GPU. No Python.
- **Training:** Adam (β₁ = 0.9, β₂ = 0.999), lr 3e-3, batch 32,
  3 epochs, deterministic seed 42.
- **Evaluation:** held-out split via i % 10 stride; 100 prefixes
  drawn from held-out sequences of length ≥ 4 morpheme tokens.

### 4.2 Training-data composition

Three configurations:

1. **synth-nouns.** FST-generated noun inflections only
   (~53 k pairs).
2. **synth-mixed.** Adds FST verb inflections × 4 finite tenses ×
   3 persons × {Sg, Pl} (~12 k pairs added).
3. **synth + real.** Adds 44 194 Root-decomposed pairs mined from
   the committed Kazakh corpora (Wikipedia kk, CC100 kk, kazakh
   textbooks, Tatoeba kk, Abai literature, the Rust Programming
   Language Kazakh translation v4.7.x) plus 500 books extracted
   from the Hugging Face `multidomain-kazakh-dataset` `kazakhBooks`
   split (Apache-2.0).

### 4.3 Results

| Configuration   | Training pairs | Train CE | Held-out CE | Gap    | Exact-match (greedy, 100 prefixes) |
|-----------------|---------------:|---------:|------------:|-------:|-----------------------------------:|
| synth-nouns     |       53 104 |    0.297 |       0.493 | 0.196 |                              0 %  |
| synth-mixed     |       65 104 |    0.355 |       0.486 | 0.131 |                              0 %  |
| **synth + real (sprint 1)** | 109 298 | 0.384 | 0.415 | **0.031** |                       15 %  |
| **synth + real (sprint 2)** | 115 729 | 0.368 | 0.416 | **0.048** |                       17 %  |

Sprint 2 (committed 2026-05-17) adds 6 431 additional Root-headed
real-corpus pairs from a wider data-mix audit (290 194 candidate
pairs, of which 50 625 Root-headed kept; +37 % over sprint 1). The
generalisation gap widened slightly (0.031 → 0.048) while exact-
match improved (+2 pp); we interpret this as mild overfitting at
sprint 2's scale rather than a regression: the model learns the
seen distribution more sharply without losing held-out accuracy.
**Single seed (POC_SEED=42); no bootstrap CI**, see §5.3.

### 4.4 Generation latency (M2 CPU, single core)

Criterion bench `crates/adam-agg-model/benches/neural_inference.rs`:

| Operation                                                | Mean   |
|----------------------------------------------------------|-------:|
| Forward pass on 2-token input                            | 0.526 ms |
| FST-constrained greedy generation of 6 new tokens        | 1.71 ms |
| FST-constrained beam (width 4) generation of 6 new tokens | 4.20 ms |

Important framing: 1.71 ms is the **morpheme-level composition
time for a six-token continuation**, not a full sentence-answer
latency. It is directly comparable to single-word inflection
latency in Apertium-Kazakh or a char-level neural baseline, not
to GPT-class full-answer latency. The v6.0 production contract
sets p50 ≤ 150 ms per neural-enabled turn including all
deterministic-kernel pre-/post-processing; the measured
composition step alone consumes < 2 % of that budget on M2 CPU,
leaving substantial headroom for the surrounding template /
retrieval / verifier work. We do **not** claim the 150 ms total-
turn target has been measured end-to-end; that is part of the
v6.0 GA performance acceptance criterion.

### 4.5 Verifier prototype

A prototype L5.5 → L6 verifier (`verifier_demo` binary) tests all
four gates (Kazakh-script, Unk, FST round-trip, factual grounding
against `data/retrieval/facts.json`, 3 672 facts indexed at 3 559
unique roots/surfaces) on a control set of grounded inflections,
ungrounded real Kazakh, loanwords, and nonsense. The gate
correctly admits all grounded inflections, admits ungrounded-but-
FST-valid forms (which is the loose-grounding mode), and rejects
nonsense surfaces. Strict-grounding mode is the v6.0 production
default. **Caveat (Codex review 2026-05-18):** the prototype is
exercised on a fixed test fixture; the verifier has *not* yet
been measured against the live neural composer's free generations
under production traffic. That measurement is part of v6.0 GA
acceptance criterion #4 and remains TBD. The architectural claim
of §2.4 — «blocks unsupported factual claims within covered KG» —
is supported by the present fixture-based tests but not yet by
end-to-end production telemetry.

---

## 5. Discussion

### 5.1 What this demonstrates

The 0.031 generalisation gap is, to our knowledge, the strongest
public evidence to date that a neural model trained inside an
FST-validity envelope on agglutinative-language data generalises
in the morphological-algebra sense rather than the surface-form-
memorisation sense. A model that merely memorised would have a gap
that explodes on held-out (root, feature) combinations; ours
contracts.

The measured 1.71 ms / six-token composition time on M2 CPU
suggests — but does **not yet prove** — that the composition-step
energy cost is in the low-millijoule range per generation. A
dedicated energy benchmark (powermetrics / `pmset` on macOS,
or RAPL on Linux) is needed to convert this latency observation
into a defensible energy claim. We have removed earlier "watch-
battery deployment" rhetoric from this draft pending that bench;
see §5.3.

### 5.2 What this does not demonstrate

This is **PoC scale.** The acceptance criteria for the
production release `v6.0.0` (architecture spec §9; roadmap
`docs/roadmap_v6_v7.md`) include:

- Lexicon V2 with ≥ 70 % Root coverage on real corpus (currently
  21 %).
- Verifier integration into the production pipeline.
- LLM-baseline comparison on 5+ Kazakh task categories.
- Multi-hardware latency reproducibility.
- A pre-registered ≥ 1-week external alpha-partner deployment.

We do **not** claim production-readiness from the present numbers.
We claim that the architecture is empirically tractable and that
the production targets are within reach given the existing trends.

### 5.3 Limitations

- **Statistical reporting.** All numbers in §4 are from a **single
  training seed** (`POC_SEED=42`); no bootstrap confidence
  intervals, no multi-seed mean ± std. The 17 % exact-match number
  on the 100-prefix eval set is a small sample with binomial
  half-CI ≈ ±7.5 pp at 95 %. We treat the PoC numbers as
  directional evidence, not as a calibrated benchmark.
- **No baseline comparison.** We do not benchmark against
  Apertium-Kazakh's morphology system, a Hugging Face Kazakh
  morphological tagger, or a char-level neural baseline trained
  on the same pairs. Three such baselines are pre-registered for
  the v6.0 GA empirical pass.
- **No ablations.** We do not vary lexicon size, algebraic-loss
  weight (α), or model width × depth. Five ablations are
  pre-registered: 3 seeds × {0.0, 0.25, 0.5, 1.0} α grid × {64×2,
  128×4} model sizes × {synth-only, synth+real} data mixes ×
  {constrained, unconstrained} decoding. Total ≈ 24 training
  runs, each ≈ 30 minutes on M2 CPU.
- **No human eval.** The exact-match metric does not measure
  semantic correctness; a held-out 100-prompt human-evaluation
  set scored by Kazakh native speakers is part of the GA
  acceptance criterion.
- **Verifier-on-real-traffic gap.** The four-gate verifier has
  been measured only on fixture inputs; production-traffic
  measurement is GA-criterion-4 work, see §4.5.
- **Coverage.** The current Root yield on real corpus is a Lexicon-
  bound limitation, not an architectural one; Lexicon V2 mining
  has identified 2 000 high-frequency candidate surfaces from a
  pool of ~70 000 uncovered tokens (down from 103 694 pre-Phase-A
  cleanup). Lexicographer-grade curation is the bottleneck.
- **Single language.** All numbers are on Kazakh. Cross-Turkic
  transfer is in the v6.1–v6.2 roadmap but not measured here.
- **Generation length.** The PoC measures generation of 6 new
  tokens (sufficient for a single inflected Kazakh word).
  Sentence-level generation is a v6.x research-arc extension, not
  measured in this preprint.
- **No energy bench.** The energy claim has been removed from
  this draft pending a `powermetrics`-grade measurement.

### 5.4 Why we publish openly

In the LLM era, the dominant pattern is closed weights, closed
training data, and proprietary licensing. Our position is that the
architectural contribution — algebra-anchored composition with a
verifier gate, for the low-resource agglutinative-language family
that has no other path to first-class AI — is more useful as a
public artefact than as a proprietary product. The BUSL-1.1
licence protects against direct competitive hosted-service copies
during the short period when those would be most damaging; it
converts automatically to Apache 2.0 after four years.

Reproduction instructions, full source, and the empirical bench
harness are in the repository at
[`github.com/qazaq-ai/adam`](https://github.com/qazaq-ai/adam).
Specifically:

- `docs/research/results_real_mix_2026_05_16.md` — table 4.3
  reproduced.
- `docs/bench/neural_inference_2026_05_16.md` — table 4.4
  reproduced.
- `crates/adam-agg-model/src/bin/poc_kazakh_train.rs` — training
  pipeline.
- `crates/adam-agg-synth/src/bin/build_real_corpus_pairs.rs` —
  real-corpus → morpheme-pair extraction.
- `crates/adam-agg-model/src/bin/verifier_demo.rs` — L5.5 → L6
  verifier prototype.

---

## 6. Conclusion

The four-inversion architecture is empirically tractable at
proof-of-concept scale on a single CPU. The path to production
deployment is bounded primarily by the rate at which Lexicon V2
curation and verifier integration can be completed (8–12 weeks
estimated), not by any unresolved architectural question.

We invite reproduction by any researcher with an M2-class machine
and four hours of attention. We invite collaboration on the eight
other agglutinative-language families catalogued in our
`COLLABORATION.md` for which the same architectural pattern should
apply.

---

## References

(To be polished for arXiv LaTeX format. Current citations are
in-text and link to upstream repositories or known papers.)

- Beesley K., Karttunen L. (2003). *Finite-State Morphology.* CSLI.
- Scholak T. et al. (2021). *PICARD: Parsing Incrementally for
  Constrained Auto-Regressive Decoding from Language Models.*
- Gunasekar S. et al. (2023). *Textbooks Are All You Need.*
- Lambert N. et al. (2024). *Tulu 3: Pushing Frontiers in Open
  Language Model Post-Training.*
- Apertium project. *kaz.tagger* and *kaz.morph* sources.

---

## Appendix A — Architectural lineage

The four-inversion position is stated in full in
[`docs/MANIFESTO.md`](../MANIFESTO.md) of the source repository.
The v6.0 release architecture is in
[`docs/architecture_neural_v6.md`](../architecture_neural_v6.md).
The v6.0 → v7.0 public roadmap is in
[`docs/roadmap_v6_v7.md`](../roadmap_v6_v7.md). The research-arc
charter is in
[`RESEARCH_AGGLUTINATIVE_NEURAL.md`](../../RESEARCH_AGGLUTINATIVE_NEURAL.md)
at the repository root.

---

*This is draft v0. Substantive revisions are tracked in pull
requests against `docs/preprint/arxiv_v0_draft.md`. Submission to
arXiv is targeted for the week of v6.0.0 GA; pre-submission a
LaTeX-formatted version with peer-reviewable references will live
at `docs/preprint/arxiv_final.tex`.*
