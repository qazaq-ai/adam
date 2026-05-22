# Collaboration

We are a research-stage company. We are open to collaboration in
every direction. This document is the concrete onboarding framework
per partner class.

For the research thesis itself, see [`MISSION.md`](MISSION.md). For
the four-inversion architectural position, see
[`docs/MANIFESTO.md`](docs/MANIFESTO.md). For the detailed research
roadmap, see [`RESEARCH.md`](RESEARCH.md). For the v6.0 architecture
spec (algebra-anchored neural composition layer added to the
deterministic kernel), see
[`docs/architecture_neural_v6.md`](docs/architecture_neural_v6.md).

## TL;DR — start here

| You are a... | We're looking for... | First step |
|---|---|---|
| **Linguist** | Native-speaker validation of Kazakh G2P / morphology; computational semantics insight; contributors for any of the 30 catalogued agglutinative languages | Open an Issue tagged `linguistics` describing your area |
| **AI researcher** | Deterministic-AI methodology; formal verification co-authors; comparative typology partners | Open an Issue tagged `research-collab` or email |
| **Educational institution** | Pilot deployment of adam in Kazakh-language classrooms (2-3 classrooms / 10-50 students) | Email `baimurza.daulet@gmail.com` |
| **National research agency** | Joint-research grants — Japan (JST/JSPS), South Korea (NRF), Finland (Academy of Finland), Turkey (TÜBİTAK), Hungary (NKFIH), Estonia (ETAg), Uzbekistan, Kyrgyzstan, Mongolia, Tatarstan, others | Email with subject «Grant inquiry — [country/agency]» |
| **Government / defence** | Use-case scoping for offline-capable, auditable Kazakh / Turkic / agglutinative-language AI | Email (private channel preferred) |
| **Investor** | Angel pre-seed / seed-stage capital | Email + pitch deck on request |
| **Software engineer** | Rust contributors; tooling; CI / infrastructure | Open a Pull Request |

Contact: **baimurza.daulet@gmail.com** ·
[LinkedIn](https://www.linkedin.com/in/daulet-baimurza-4b3506211/) ·
[GitHub](https://github.com/DauletBai)

## Detailed onboarding by partner class

### 1. Linguists

We need help on the following:

**Kazakh-specific:**
- **G2P validation** — the [`adam_dialog::phoneme`](crates/adam-dialog/src/phoneme.rs)
  module ships a 33-phoneme inventory with rule-based grapheme-to-phoneme
  mapping. Native-speaker review of edge cases (loanword phonology,
  vowel harmony in compound words, stress placement) would directly
  improve quality.
- **Morphological corner cases** — every commit lands FST regression
  tests, but the corpus we test against is finite. Native-speaker
  contribution of «hard» test cases (rare suffix combinations,
  archaisms, dialectal variation) is welcome.
- **`world_core` curation** — `data/world_core/*.jsonl` is the curated
  knowledge graph adam grounds responses against. We accept
  pull-request additions reviewed by `shaman` (the founder).

**Cross-language:**
- **Karakalpak / Kyrgyz speakers** — closest-language ports are the
  first concrete generalisation test. Native-speaker collaboration
  on Lexicon adaptation is highly valuable.
- **Turkic / Mongolic / Uralic / Bantu specialists** — comparative
  typology research, identifying where the architecture's assumptions
  break.

**How to engage:** open a GitHub Issue tagged `linguistics`, or email
the founder. If contributing data files, please include a clear
licence statement (we are BUSL-1.1; contributed data should be
compatible).

### 2. AI researchers

We are interested in:

- **Deterministic alternatives to neural inference.** Are there
  hybrid architectures where small ML components sit inside a
  deterministic kernel without compromising its guarantees? See
  [Q2 in `RESEARCH.md`](RESEARCH.md#q2-where-is-the-boundary-between-kernel-and-trainable-component).
- **Formal verification of language models.** Can we machine-check
  the determinism property end-to-end? TLA+ specifications of the
  dialog FSM are on the roadmap (planned 2027).
- **Comparative typology in NLP.** How does the morpheme-decomposition
  approach hold up across the 30 languages catalogued in
  [`MISSION.md`](MISSION.md#agglutinative-languages-global-research-frontier)?
- **Hallucination measurement methodology.** Our claim is
  «architectural impossibility of hallucination because every claim
  cites a source». We want to formalise the measurement and
  benchmark against probabilistic baselines.

**How to engage:** open a GitHub Issue tagged `research-collab` with
a brief proposal, or email the founder. We are open to co-authorship
on papers. We do not require co-authorship on independent research
that uses adam as a baseline.

### 3. Educational institutions

We are running pilot deployments of **adam** as a Kazakh-language
Rust-programming tutor in Kazakhstani schools.

**Current pilot scope (Q3 2026):**
- 2-3 classrooms (Almaty / Astana)
- 10-50 students per classroom, ages 14-18
- 5-stage Rust curriculum: ownership → borrow → lifetime → traits →
  async (~3 months end-to-end)
- Voice + text interaction (macOS or Linux laptops with `--tts`)
- Per-student progress tracking + adaptive difficulty
- Weekly transcripts review for live-corpus refinement

**What we provide:**
- adam binary + setup instructions
- Teacher onboarding (~2 hours)
- Direct line to founder for issues
- Per-pilot performance report (anonymised)

**What we ask:**
- Permission to log conversations for research purposes (no PII)
- Teacher feedback after each session
- Open communication on what works / fails

**How to engage:** email **baimurza.daulet@gmail.com** with school
context (city / age range / current curriculum). We respond within
48 hours.

### 4. Governments and academic research grants

The research direction speaks directly to **state-level priorities
in every country whose national language is agglutinative**. We are
actively seeking both state grants and academic joint-research
partnerships — this is a more natural funding path than private VC
because each state has a **direct strategic interest** in
deterministic AI for its own language.

#### Kazakhstan — home base

- **Digital sovereignty.** A Kazakhstan-built deterministic AI that
  does not depend on external LLM providers (OpenAI, Anthropic,
  Google) for inference. Works offline; no data egress.
- **Kazakh-language preservation and modernisation.** State-supported
  Kazakhization in IT meets a serious gap on the AI-tooling side.
- **Auditable AI for high-stakes domains.** No hallucination by
  architecture; every claim traceable to a source. Matches regulatory
  expectations for defence / healthcare / legal use.

Mechanism: Astana Hub, Ministry of Digital Development, Nazarbayev
University, NIS schools.

#### International — agglutinative-language alignment

Each country listed below has (a) a national language that is
agglutinative, (b) a research-grant ecosystem that funds
deterministic / formal AI work, and (c) strategic interest in
alternatives to American / Chinese LLMs. Our research is directly
applicable to their own language stack.

| Country | Agglutinative language | Research-grant mechanism | Strategic alignment |
|---|---|---|---|
| **Japan** | Japanese | JST (Japan Science and Technology Agency), JSPS (Japan Society for the Promotion of Science) | Strong existing computational-linguistics tradition; NEDO funds non-American AI alternatives |
| **South Korea** | Korean | NRF (National Research Foundation), IITP (Institute for Information & Communications Technology Planning) | Major industrial R&D investment (Samsung, LG, Naver); LLM-skeptic research direction |
| **Finland** | Finnish | Academy of Finland, Business Finland | Strong tradition of formal-methods AI; small-state digital-sovereignty model |
| **Hungary** | Hungarian | National Research, Development and Innovation Office (NKFIH) | EU-aligned research funding; Uralic computational linguistics base |
| **Estonia** | Estonian | Estonian Research Council (ETAg) | Digital-state pioneer; auditable-AI deployment in e-government context |
| **Turkey** | Turkish | TÜBİTAK (Scientific and Technological Research Council) | Strongest existing NLP base for any agglutinative language; natural research-collaboration substrate |
| **Uzbekistan** | Uzbek | Ministry of Innovative Development; cultural alignment with Kazakhstan | Closely-related Turkic; joint-Turkic research agenda; bilateral programmes with Kazakhstan |
| **Kyrgyzstan** | Kyrgyz | Kyrgyz Academy of Sciences | Closest neighbour; closely-related Turkic; rapid-port substrate |
| **Mongolia** | Mongolian | Mongolian Academy of Sciences | Mongolic-language research collaborator; offline-AI strategic interest |
| **Tatarstan (Russia)** | Tatar | Tatarstan Academy of Sciences; Kazan Federal University | Regional-level research; Tatar-language preservation programmes |

…and other agglutinative-language states from the 30-language
catalogue in [`MISSION.md`](MISSION.md#agglutinative-languages-global-research-frontier).
We are open to engagement with any of them.

#### What we offer state / academic partners

- **Open-source kernel** (BUSL-1.1) as substrate for joint research
- **Co-authorship** on resulting publications (we don't require
  single-author claims)
- **Direct technical collaboration** — founder is responsive and the
  codebase is auditable end-to-end
- **Native-speaker validation channel** for any agglutinative-language
  port (we connect language experts on both sides)

#### What we ask

- **Grant funding** for engineer hires, phoneme-bank recording, pilot
  deployments
- **Academic affiliation** for credibility and access to graduate
  students / native-speaker linguists
- **Joint-research scope agreements** with clear publication and IP
  terms (BUSL-1.1 compatible)

**How to engage:** email **baimurza.daulet@gmail.com** with subject
line «Grant inquiry — [country / agency name]». Specify the funding
mechanism you have in mind and the language(s) of interest. We
respond within 48 hours and can prepare country-specific proposals.

### 5. Investors

We are at **angel pre-seed / seed stage**. We are looking for
partners who share the thesis that probabilistic AI is not the only
path forward.

**Stage and ask (current):**
- Angel pre-seed: **$200K–300K** for 12 months
- Use of funds: 2 engineers (Rust + content), phoneme bank recording,
  pilot deployment infrastructure, legal (Delaware C-Corp + KZ ИП)
- Runway target: first cohort of student users + measurable retention
  metrics by end of 12-month horizon, leading to seed round in 2027

**Why now:**
- Working demonstrator (adam) with 1 300+ tests, 487+ versioned
  releases, public repo with green CI
- 30-language generalisation frontier already mapped
- Clear measurable goals (predictability / cheapness / safety) with
  current metrics (300 MB / 21 ms p50 / 0% GPU)
- 5-month solo development pace demonstrates founder execution
  discipline

**Available on request:**
- Pitch deck (12 slides)
- Demo video (3 minutes — REPL + voice)
- Financial model + 18-month roadmap
- Repository access for technical due diligence (already public)
- Live demo via Zoom or in-person in Almaty

**How to engage:** email **baimurza.daulet@gmail.com** with subject
line «Investment inquiry — adam». We respond within 48 hours.

### 6. Software engineers

The code is BUSL-1.1 (source-available; commercial use by
permission). Pull requests are welcome on:

- **Rust performance optimisations** (we run on M2 8GB; tighter
  loops always welcome)
- **CI / tooling** (GitHub Actions workflows, lint setups)
- **Test coverage** (current: 1 300+ tests; we don't reject test-only
  PRs)
- **Documentation** (typos, clarifications, examples)
- **New language ports** (per linguist collaboration above)
- **Bug fixes** (with regression tests)

**Style:**
- All Rust code passes `cargo fmt --check` + `cargo clippy -- -D warnings`
- Each release runs `verify_release_version.sh` + `check_metrics_currency.sh`
- We document non-obvious decisions inline

**How to engage:** fork → branch → PR. Reviewer is the founder; turnaround
~48 hours. For larger architectural changes, open a discussion Issue
first.

## Licence and IP framework

- **Source code:** [BUSL-1.1](LICENSE) (Business Source License). Source
  is visible; commercial use requires explicit permission from the
  founder.
- **Curated data** (`data/world_core/*`, `data/dialog/curriculum/*`):
  same licence as the code; contributions accepted with attribution.
- **First commit:** 2026-04-07 (priority record for authorship
  disputes).
- **Patents:** none filed; relying on copyright + BUSL.
- **Trade secrets:** none claimed; everything is in the open
  repository.

## What we will *not* do

To set expectations clearly:

- **We will not white-label.** We do not build «your-brand-here AI»
  for resale. The Qazaq IR research mission is independent of any
  single application.
- **We will not pivot to LLMs.** The thesis of this project is
  *deterministic alternative* — adopting probabilistic LLMs would
  defeat the research arc.
- **We will not commit to investor terms that compromise BUSL-1.1.**
  The licence is non-negotiable; the open-research-with-commercial-
  protection framing is structural to the project.

## Questions

Open a GitHub Discussion or email **baimurza.daulet@gmail.com**.
We are responsive.
