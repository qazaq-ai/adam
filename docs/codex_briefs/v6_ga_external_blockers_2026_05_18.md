# Codex brief — v6.0 GA external blockers (2026-05-18)

> **Context.** The `adam` project (Agglutinative Reasoning Kernel,
> Kazakh-first deterministic AI) has a `experimental/agglutinative-neural`
> research branch that closed all internal v6.0.0 acceptance criteria.
> Three criteria remain — all of them require **work outside the
> repository** (linguist review / academic peer review / live alpha
> partner). This brief asks for a concrete plan to close each.
>
> **What we want from Codex.** Analysis + recommendations + draft
> deliverables. No code changes — the maintainer will implement
> whatever you suggest, you are not expected to push.
>
> **Repository access.** Public:
> `https://github.com/qazaq-ai/adam/tree/experimental/agglutinative-neural`.
> No credentials required.
>
> **Maintainer.** Daulet Baimurza, solo developer (Kostanay,
> Kazakhstan). Native Kazakh speaker. Budget: bootstrapped, no
> commercial funding yet. Hardware: MacBook Air M2 8 GB.

---

## Criterion #3 — Lexicon V2 native-speaker review

### Status

- The corpus-mining pipeline (`adam-corpus/src/bin/mine_lexicon_gaps.rs`)
  surfaces every Kazakh surface that the current 25.5k-root Lexicon
  cannot decompose. Phase A waves 1-3 dropped uncovered surfaces
  from **103,694 → 69,808 (-33.1%)**.
- The remaining gap is **~70k surfaces** that need either a root
  addition or a confirmed exclusion (loanword / OCR error / typo).
- A **top-2000 review queue** (highest frequency uncovered surfaces)
  lives at:
    `docs/lexicon_gap_candidates.md`
  Each candidate has: surface, frequency, sample corpus sentences,
  morphological hint, source pack.

### Acceptance bar (from `docs/architecture_neural_v6.md` §9)

> Lexicon V2 ≥ **70 %** Root coverage on the canonical Kazakh
> real-corpus eval set.

Current coverage: **86.21 %** prefix-match over 3.87 M committed
words *with* fragment matching. **Without** fragment matching (true
root coverage) the number is ≈ 63 % — short of 70%.

### Concrete asks for Codex

1. **Read** `docs/lexicon_gap_candidates.md` (35k-line markdown,
   sorted by frequency). Each section is one candidate root with
   surrounding evidence.

2. **Cluster the 2000 candidates** into:
   - **Auto-approve** (high-confidence: regular Kazakh nouns / verbs
     / adjectives with clear morphology, ≥ 5 corpus citations).
   - **Auto-exclude** (loanwords with Russian / English origin
     visible in the surface; OCR garbage; numerals already covered;
     proper nouns belonging to `geo_kz` / `notable_kazakhstanis`
     domains).
   - **Needs native-speaker** (everything else).

   Report cluster sizes + 10 sample entries from each cluster.

3. **Estimate review effort**: if a native speaker spends 30 s per
   "Needs native-speaker" entry, how many hours total? What's the
   minimum subset that gets us past the 70 % threshold?

4. **Draft a review workflow** — a 1-page protocol the linguist can
   follow without reading the source code. Input: a CSV / spreadsheet
   per cluster. Output: marked rows (approve / exclude / edit-then-
   approve / discuss). Should run in Google Sheets or LibreOffice
   Calc — no custom tooling.

5. **Suggest 5-10 candidate linguists / institutions in Kazakhstan**
   appropriate for this review. The maintainer already has an active
   contact at КРУ им. Байтұрсынұлы (Кафедра ФиПЛ, проф. Әбсадық
   А. А.). Add other candidates from:
   - Қазақ ұлттық университеті (КазНУ Алматы)
   - А. Байтұрсынұлы атындағы Тіл білімі институты (Алматы)
   - Әл-Фараби атындағы ҚазҰУ — кафедра қазақ тілі
   - Pop Lit Group at А. Байтұрсынұлы Institute of Linguistics
   - Anyone else you know of who reviews Kazakh corpus / NLP data.

6. **Optional**: suggest paid review options. Approximate cost in
   tenge if a native-speaker linguist charges ₸1500-3000 per
   reviewed candidate. Funding sources adam could approach:
   Astana Hub Kazakh-language AI grants, IT-HUB Қостанай.

### Deliverables

- `lexicon_v2_clusters.csv` — 2000 rows × {surface, cluster, freq, sample}
- `lexicon_v2_review_protocol.md` — 1-page linguist protocol
- `lexicon_v2_outreach_candidates.md` — table of 5-10 contacts
- One-line answer: **how many native-speaker hours to clear 70 % bar?**

---

## Criterion #6 — arXiv preprint accepted / under review

### Status

- Draft committed at `docs/preprint/arxiv_v0_draft.md`.
- Architecture position: «algebra-anchored neural composition for
  agglutinative languages». Key contributions: FST-guided constrained
  decoding + L6 verifier + Kazakh-first eval.
- Empirical results: Train CE 0.368, Held-out CE 0.416, exact-match
  17% on 290k pairs (M2 CPU, 95 min training, 1.17M params).
- **Not yet submitted** anywhere.

### Concrete asks for Codex

1. **Read** `docs/preprint/arxiv_v0_draft.md` + the linked
   research-arc results:
   - `docs/research/results_clean_full_ab_2026_05_17.md` (sprint 2)
   - `docs/research/results_real_mix_2026_05_16.md` (sprint 1)
   - `docs/bench/our_numbers_vs_published_llm.md` (vs published LLMs)
   - `docs/architecture_neural_v6.md` (the v6.0 architecture spec)

2. **Academic-rigor critique**: identify
   - Claims not backed by experiments shown
   - Methodology weaknesses
   - Missing ablations
   - Statistical reporting gaps (no confidence intervals on CE / EM)
   - Reproducibility blockers

3. **Recommend submission targets**, ranked:
   - arXiv (immediate, gives DOI within 24 h — closes the criterion
     literally if "stable DOI" is enough)
   - Workshop submission (Workshop on Low-Resource NLP at EMNLP /
     ACL? Eurali / Turkic NLP workshop?) — fastest peer review
   - Full journal (TACL / Computational Linguistics) — 6-12 month
     turnaround but highest credibility

4. **Identify what extra experiments would make the draft
   significantly stronger** without requiring more than 1-2 days of
   M2 compute (no GPUs available). Budget: ≤ 4 hours of additional
   training per ablation.

5. **Draft a cover letter** for the strongest recommended target.

### Deliverables

- `preprint_critique.md` — section-by-section academic review
- `preprint_submission_plan.md` — ranked targets, timelines, costs
- `preprint_followup_experiments.md` — 3-5 cheap ablations that
  measurably strengthen the empirical claim
- `preprint_cover_letter.md` — ready-to-paste cover letter

---

## Criterion #7 — Migration validated against external alpha

### Status

- Migration playbook committed at `docs/migration_v5_to_v6.md`.
- Defines the rollback-safe v5.x → v6.0 upgrade path: feature flag
  for L5.5, verifier always strict, kernel deterministic by default.
- **No external organisation has deployed adam yet.**

### Concrete asks for Codex

1. **Read** `docs/migration_v5_to_v6.md` + `docs/architecture_neural_v6.md`.

2. **Define "validated against external alpha" concretely**: what
   has to happen for the criterion to be marked done?
   - Minimum: one external org runs `adam_chat` against ≥ N
     real-user turns; produces a feedback file.
   - Realistic: N = ? turns? duration = ? weeks?
   - Success criteria: % turns rated acceptable / 0 hallucinations
     / what else?
   - Failure criteria: when does the alpha kill the v6.0 release?

3. **Identify 3-5 candidate alpha-partner organisations in Kazakhstan**
   appropriate for a Kazakh school-tutor / deterministic NLP
   product. Adam's product position is «Qazaq AI Ұстаз» — Kazakh
   school tutor (see `docs/product/qazaq_ai_ustaz_v1.md`):
   - Republican Centre «Daryn» (gifted-children school network)
   - Bilim Innovation Lyceums
   - Astana Hub portfolio companies in EdTech
   - Republican Centre for Distance Learning under MES KZ
   - Костанайский региональный университет (active contact, see
     `project_kru_baitursynov_partnership` memory)

4. **Draft an alpha-partner onboarding kit**:
   - One-page pitch (Kazakh + Russian)
   - Deploy instructions (single-machine, M2-class hardware)
   - Feedback collection protocol (Google Form? spreadsheet?)
   - Privacy / data-handling agreement (adam does not phone home;
     opt-in weather and STT shells out locally)
   - Risk disclaimer (adam is research-grade, not production
     certified)

5. **Define rollback / abort criteria**: if the alpha surfaces a
   critical bug, when does the maintainer pause v6.0 release?

### Deliverables

- `alpha_validation_spec.md` — concrete success / failure / abort
  criteria
- `alpha_outreach_candidates.md` — 3-5 candidates with rationale
- `alpha_onboarding_kit/` — directory of pitch + deploy + feedback
  + privacy + risk-disclaimer documents

---

## Cross-cutting question for Codex

**Is the v6.0.0 GA release plan realistic for a solo maintainer
without commercial funding?** Look at:
- Time required to close #3 + #6 + #7 (your own estimate)
- Time required for the linguist / peer reviewers / alpha partner
  to do their part (cannot be compressed)
- Maintainer's track record: solo, M2 8 GB, full-time on adam since
  v1.0.0 (≈ 6 weeks ago)

If GA is unrealistic in ≤ 8 weeks, recommend the next-best
release line: `v6.0.0-rc1` preview release on the main branch +
parallel research-arc work on the external criteria.

---

## How to read this repo if you're new to it

- `README.md` — the public-facing intro.
- `MISSION.md` — what adam is and isn't.
- `docs/MANIFESTO.md` — the four architectural inversions of v6.0.
- `docs/architecture_neural_v6.md` — the v6.0 architecture contract.
- `docs/roadmap.md` — release history, latest at top.
- `CHANGELOG.md` — every release entry with thematic label.

The branch this brief lives on is `experimental/agglutinative-neural`.
Diff against main is 57 commits / 52k lines / 5 new crates. Code is
production-grade Rust, no Python, no cloud dependencies.
