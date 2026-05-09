# Research arc

This document is the detailed research roadmap for **Qazaq IR** — the
deterministic AI kernel built on agglutinative-language morphology.
The high-level mission is in [`MISSION.md`](MISSION.md); this document
zooms into open research questions, methodology, and milestones.

## Open research questions

### Q1. Can a deterministic kernel match LLMs on conversational coherence?

**Status.** Partially answered. adam (v5.3.x) demonstrates multi-turn
dialog with belief-state tracking, contradiction recovery, anaphora
resolution, and curriculum-aware self-recall — all without
neural-style generation. The remaining gap is **scope**: adam covers
~41 intent variants and a curated `world_core` of ~3000 facts, far
narrower than what an LLM is trained on.

**Open sub-questions:**
- How does the curated-graph approach scale as the fact base grows
  10× / 100× / 1000×?
- Does coherence degrade gracefully when topics fall outside curated
  domains? (Current honest-fallback templates suggest yes, but at what
  point does refusal-rate become user-hostile?)
- Can the morpheme-decomposition core handle code-switching (Kazakh +
  Latin tech tokens, Kazakh + Russian)?

### Q2. Where is the boundary between «kernel» and «trainable component»?

**Status.** Working hypothesis: **tiny ML lives, large ML doesn't**.
adam ships a 24-byte selection-weights perceptron (6 f32) for template
ranking, suffix-chain bigram priors (~31 k bigrams), and root-affinity
PMI (~10 k roots). All are statistical but inspectable; none is a
neural network in the LLM sense.

**Open sub-questions:**
- What is the largest trainable component compatible with the
  kernel-determinism guarantee? E.g. could a small (~10 MB) neural
  classifier sit *inside* the kernel as a confidence-band oracle
  without breaking auditability?
- Can we **formally verify** the determinism property end-to-end?
  Candidate framework: TLA+ specifications of the dialog FSM with
  invariants stating «for any (input, seed, facts), output is uniquely
  determined».
- What happens if we replace the template-ranking perceptron with a
  rule-based scoring function? (Would simplify the kernel but possibly
  hurt naturalness.)

### Q3. How does the architecture generalise across agglutinative languages?

**Status.** Theoretical claim made, not yet validated. The
30-language catalogue in [`MISSION.md`](MISSION.md#agglutinative-languages)
identifies candidates, but adam currently only exists for Kazakh.

**Open sub-questions:**
- What is the actual porting cost from Kazakh to Karakalpak (≈ closest
  language)? Estimate: ~1-2 weeks of Lexicon adaptation + per-suffix
  phonology rule deltas. To be measured.
- Where do the differences live? E.g. Hungarian has 18 cases vs
  Kazakh's 7 — does the FST architecture handle that purely through
  data, or does it need code changes?
- What about non-Eurasian agglutinative languages (Tamil, Quechua,
  Swahili)? Phonology assumptions baked into the Kazakh G2P module
  may not transfer cleanly.

### Q4. What is the cost-quality frontier for kernel-pure voice?

**Status.** v5.0.0 ships TTS via OS-bundled voices (`Aru` on macOS).
v5.1.0 adds optional Piper neural backend. v5.2.0 ships the Kazakh
G2P module as substrate for kernel-pure concatenative TTS.

**Open sub-questions:**
- How does naturalness-perception scale with phoneme-bank quality?
  (One native speaker recording each of 33 phonemes vs studio-quality
  multi-speaker corpus.)
- Is concatenative kernel-pure TTS preferable to neural Piper for
  educational deployment? Trade-off: determinism / offline / no model
  vs naturalness.

### Q5. What architectural extensions does the curriculum surface need?

**Status.** v4.98.0 → v4.99.5 shipped a 5-stage Rust curriculum tree
with adaptive difficulty + auto-advance + student-side query intents.
Closed the Codex round-2 audit of educational use case.

**Open sub-questions:**
- Beyond Rust: can the same curriculum tree generalise to other
  programming languages? Mathematics? Natural science?
- What's the right granularity for «adaptive difficulty»? Currently
  binary (Easy / Hard) thresholded on pass/fail counts; could become
  multi-dimensional (concept × variation × prior knowledge).
- How does the cargo-check verifier loop generalise to non-Rust
  languages with different toolchain expectations?

## Methodology

The research is empirical-engineering, not theoretical. We follow
this loop:

1. **Hypothesis** — formulate a claim about what the kernel can or
   cannot do (e.g. «multi-turn anaphora can resolve via DialogContext
   without neural attention»).
2. **Implementation** — build the smallest version of the feature
   that tests the hypothesis.
3. **Live testing** — exercise it in a real REPL session with novel
   Kazakh phrasings (not templated holdouts; per
   `feedback_real_human_testing_with_memory`).
4. **Audit** — submit transcripts to Codex (third-party AI auditor)
   for adversarial review. Fold findings into next iteration.
5. **Regression test** — every audited bug becomes a permanent test
   case so it cannot reoccur silently.
6. **Release** — bundle 1-7 innovations per release, version per
   significance (`feedback_versioning_post_1_0`).

Three Codex audit rounds completed so far (2026-04-29 / 2026-05-07 /
2026-05-08), each surfacing 7-8 issues. Investor-readiness
self-assessment: 7/10 after v5.3.0; ongoing.

## Milestones

### Q2 2026 — Demonstrator stability ✅ shipping

- ✅ Multi-turn dialog with belief-state tracking
- ✅ Contradiction recovery + explicit-pick resolution
- ✅ Anaphora resolution with overcarry guard
- ✅ Voice output via OS-bundled TTS
- ✅ Kazakh-only refusal for non-Kazakh inputs
- ✅ Curriculum tree with adaptive difficulty (Rust track)
- ✅ Codex round-3 audit closed (architectural pass)
- ✅ Public repository (2026-05-08), BUSL-1.1 license

### Q3 2026 — Kernel-pure voice + first port

- 🟡 Phoneme bank: hand-record native-speaker WAVs for the 33 Kazakh
  phonemes
- 🟡 `PhonemeBankTtsBackend`: load + splice via existing G2P module
- 🟡 Validate the architecture's portability: prototype Karakalpak
  Lexicon adaptation (closest-language test)
- 🟡 First school pilot in Almaty / Astana (MVP deployment in 2-3
  classrooms)

### Q4 2026 — Multi-language extension

- 🟡 Choose second language for full port: candidates Kyrgyz (Kipchak,
  closest after Karakalpak) or Turkish (largest existing NLP base for
  comparison)
- 🟡 Document the porting cost honestly; identify which architectural
  pieces are language-agnostic vs language-specific
- 🟡 Publish a comparative paper (or technical blog post)

### 2027 — Formal verification

- 🟡 Specify ARK's deterministic guarantees in TLA+ (or similar)
- 🟡 Machine-check key invariants:
  - «For any (input, seed, facts), output is uniquely determined»
  - «No claim is emitted without a backing fact in `world_core` or a
    grounded reasoning chain»
  - «Belief state has at most one Active fact per (subject, predicate)»
- 🟡 Publish verified-kernel artefact

### 2027+ — Vertical applications

Per the parent Qazna Technologies vision (FinTech / DefenseTech /
HealthTech / EdTech), explore non-educational applications of the
deterministic kernel:

- **FinTech** — auditable compliance assistants, regulatory document
  analysis (Kazakh-language)
- **DefenceTech** — offline-capable, deterministic operator-support
  AI for restricted environments
- **HealthTech** — symptom-triage with traceable reasoning chains
  (no hallucination tolerance)
- **EdTech** — adam continues as the educational vertical;
  potentially expand to other subjects beyond Rust

## How this is funded

The research is currently self-funded (founder time + minimal
infrastructure costs — 0% GPU).

We are pursuing **two parallel funding tracks**:

1. **Angel pre-seed / seed-stage private capital** — to accelerate
   Q3 2026 milestones (phoneme bank recording, first port, school
   pilots). Target: $200K–300K for 12 months.

2. **State research grants and academic joint-research partnerships**
   — every state in the 30-language agglutinative catalogue has a
   direct strategic interest in deterministic AI for its own national
   language. Priority partners: Japan (JST/JSPS), South Korea (NRF),
   Finland (Academy of Finland), Turkey (TÜBİTAK), Uzbekistan,
   Hungary (NKFIH), Estonia (ETAg), Mongolia, Kyrgyzstan, Tatarstan.
   See [`COLLABORATION.md`](COLLABORATION.md#international--agglutinative-language-alignment)
   for engagement terms.

These tracks are complementary, not competing: state grants fund
research milestones (ports to new languages, formal verification,
phoneme-bank recording with native speakers), while private capital
funds applied product / pilot deployment. We pursue both.

See [`COLLABORATION.md`](COLLABORATION.md) for the full collaboration
framework, including investor-engagement terms.

## Publications and external references

- [adam GitHub repository](https://github.com/qazaq-ai/adam) — source
  of truth (BUSL-1.1)
- (Planned) Habr / Medium technical blog post — Q2 2026
- (Planned) Comparative paper on Karakalpak port — Q4 2026
- (Planned) TLA+ verified-kernel artefact — 2027
