# Hybrid QLM Verifier — experiment protocol

**Branch:** `experiment/hybrid-qlm-verifier`
**Opened:** 2026-06-27 from `main @ bc484254`
**Deadline:** 2026-07-27 (30 days from open)
**Status:** scoping

## Why this branch exists

ADAM's deterministic kernel has a real ceiling.  Novel queries
outside the curated graph land in `Clarify`.  The user
experiences this as «no intellect, only pre-baked answers».
Strategic review on 2026-06-27 confirmed two things:

  1. Pivoting wholesale to a Kazakh LLM trained from scratch is
     NOT viable on our resource envelope (M2 8 GB / ~300 K
     curated Kazakh sentences / single dev).  Chinchilla
     scaling laws alone settle this — a 1B+ model wants tens
     of billions of tokens to pretrain well, not hundreds of
     thousands of sentences.

  2. A small (1-4B) open-weight LM, used **strictly as a
     candidate generator behind the deterministic verifier**,
     might add the conversational fluency the deterministic
     surface currently lacks — without giving up the
     zero-hallucination contract that ADAM's pilot value
     proposition depends on.

This branch tests (2).

## Architectural contract — non-negotiable

```
small LM         →   proposes candidates (paraphrase / ASR
                     N-best rescore / dialog-act
                     classification / soft suggestions)
adam algebra     →   parses / verifies the candidate
world_core /     →   grounds factual claims
procedure /
solver
realiser         →   emits the surface text
eval harness     →   blocks regression
```

The LM **never** speaks a factual claim directly to the user.
Every utterance the user sees is either:

  * deterministic-cascade output (existing path), OR
  * an LM-generated CANDIDATE that the verifier has
    successfully grounded against `world_core` / procedure /
    safety / clarification rules.

An unsupported LM claim is DROPPED, not surfaced.  This is the
load-bearing invariant of the whole experiment.

## Success criteria (binary, evaluated at day 30)

The experiment is **promoted to mainline integration** ONLY
when ALL three hold:

  1. **No production regression.**  All 5 production evals
     (school_program / conv_dialog / safety / v6_7_real_audit /
     speech_defect) stay at their pre-experiment baselines.
     Currently:
       school_program     159/159 = 100% semantic
       conv_dialog         52/52  = 100% semantic
       safety              22/22  = 100% semantic
       v6_7_real_audit     26/26  = 100% semantic
       speech_defect       54/71  = 76%

  2. **Measurable naturalness / paraphrase coverage gain.**
     New eval pack `paraphrase_coverage_eval.json` introduced
     in this branch.  Tracks how many «X variants» of canonical
     queries reach the same correct cascade answer.  Baseline
     to be measured at experiment day 1.  Promotion requires
     ≥ 20 pp gain.

  3. **Zero factual hallucinations.**  Manual audit of N=100
     novel queries through the hybrid path.  Verifier MUST
     drop unsupported LM claims.  ANY hallucination that
     reaches the user counts as a failure → archive.

If ANY of the three fails at day 30 — **archive the branch
without sentimental rescue**.  This is built into the protocol
on day 1.

## Archive criteria

Branch is **archived without integration** if:
  * Promotion criteria not met at day 30, OR
  * Any factual hallucination reaches the user during the
    experiment window, OR
  * Verifier complexity outpaces the maintenance budget of the
    deterministic kernel (architectural cost > value), OR
  * User direction changes.

«Archived» means: branch stays on the remote for reference,
no rebase / merge to main, no production deployment, lessons
documented in [[project_codex_consultation_2026_06_27]] memory
file.

## Layout convention

```
experiments/hybrid_qlm/
  PROTOCOL.md              ← this file
  baseline/                ← day-1 honest-baseline measurements
                             of the chosen LM on existing evals
  paraphrase_eval/         ← new eval pack tracking paraphrase
                             coverage as the load-bearing
                             success metric
  lora_recipes/            ← LoRA / continued-pretrain configs
  archive_notes.md         ← lessons / decisions / abandoned
                             paths
```

Code paths land in:

```
crates/adam-hybrid-llm/    ← new crate.  candidate generator
                             API: propose_paraphrase,
                             rescore_n_best,
                             classify_dialog_act.
crates/adam-dialog/        ← verifier-side gate (existing
                             AnswerCandidate + ProofRef layer
                             extended to accept hybrid
                             candidates and reject unsupported
                             ones).
```

## What is NOT in scope for this branch

  * Training from scratch.  Continue-pretrain / LoRA on a
    pretrained Gemma-3 4B / Llama-3.1 8B base ONLY.
  * Multilingual generation.  Kazakh + Russian + English are
    handled via the existing `lang_bridge` peripheral; this
    branch does not extend that.
  * Image / audio multimodality.
  * Direct production traffic.  Branch outputs are gated
    behind an env var (`ADAM_HYBRID_LM=1`); default-off in
    every binary.

## Phases (thematic, not calendar)

Cadence is per-commit, not per-day — this work moves in hours,
not weeks.  Each phase is one or more commits on this branch.

  baseline           — pick base model, write baseline harness,
                       capture honest bare-LM scores on existing
                       eval probes.  **Done — 87166e82.**
                       Result: 4B = 7/10 + 2 honest «не знаю»,
                       1B = 0/10 (dropped).
  api-skeleton       — design + ship `crates/adam-hybrid-llm`.
                       Three proposer APIs (`propose_paraphrase`,
                       `rescore_n_best`, `classify_dialog_act`)
                       behind `ADAM_HYBRID_LM=1` env gate.
                       Real `llama-completion` subprocess wiring.
  paraphrase-eval    — author `paraphrase_coverage_eval.json`
                       (~100 canonical queries × 5 paraphrases).
                       Measure deterministic-only baseline so
                       the hybrid lift is comparable.
  lora-recipe        — LoRA / continued-pretrain on native
                       Kazakh corpora (`data/curated` +
                       Wikipedia kk + adam-dialog session
                       journal extracts).  Re-measure with
                       hybrid path on.
  verifier-hardening — 100-case manual audit for hallucination
                       leakage.  Tighten the
                       `adam-kernel`-verifier gate at every
                       place an ungrounded claim slipped
                       through.
  final-measurement  — re-run all three promotion criteria
                       (production no-regression, paraphrase
                       coverage ≥ 20 pp gain, zero
                       hallucinations).  Write
                       `archive_notes.md`.
  promote-or-archive — single decision commit.  Either merge
                       integration plan into main, or archive
                       branch without rebase.

## Open questions (decide in first 3 days)

  * Base model: Gemma-3 4B vs Llama-3.1 8B vs something
    smaller?  M2 8 GB constraint argues 4B quantized
    (q4_k_m ~2.5 GB inference RAM).
  * Inference path: llama.cpp subprocess (mirrors existing
    Whisper integration) vs in-process rust binding (mistral.rs
    / candle)?  Subprocess is simpler.
  * Continue-pretrain corpus: just `data/curated`, or also pull
    KZ Wikipedia + Tatoeba?  Latter is more data but may dilute
    the school-tutor / SOP focus.
  * Paraphrase eval source: hand-authored 100 cases or extract
    from existing real_audit sessions?

These get answered on the branch.  No commitment from main.

## Out-of-scope distractions to refuse

  * «Make adam answer arbitrary novel queries» — not the
    experiment.  Hybrid LM augments coverage of EXISTING
    domains via paraphrase / rescore, not adds new factual
    domains.
  * «Train Kazakh GPT competitor» — not the experiment.
    Strategic review explicitly rejected this.
  * «Use LM for safety routing» — not the experiment.  Safety
    stays 100 % deterministic.

If anyone (including future-me) proposes any of the above on
this branch — they get pointed at this section.
