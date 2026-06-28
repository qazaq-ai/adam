# `experiment/hybrid-qlm-verifier` — ARCHIVE NOTES

**Branch:** `experiment/hybrid-qlm-verifier`
**Opened:** 2026-06-27 from `main @ bc484254`
**Archived:** 2026-06-28
**Status:** NEGATIVE result, archived without merge.  Branch
            kept on remote for reference; no rebase, no
            production deployment, lessons documented here.

## One-line verdict

A bare 1-4B open-weight LM (Gemma-3 4B q4_k_m specifically),
used as a candidate generator behind a deterministic
verifier, does NOT clear cheap deterministic baselines on
the Kazakh school-tutor / SOP domain at the resource
envelope of an 8 GB M2 with no fine-tuning.

## What was tested — five proposer / verifier configurations

| # | Test                          | Deterministic              | LM-4B base       | Verdict   |
| - | ----------------------------- | -------------------------- | ---------------- | --------- |
| 1 | Bare-LM probe (10 Q&A)        | n/a                        | 7/10 + 1 confident hallucination + 2 honest «не знаю» | LM verifier-mandatory |
| 2 | Paraphrase rewriting (100)    | 89/100 (alias table)       | 85-90/100 (4 LM variants) | NO LM ROI |
| 3 | N-best rescoring (20 × 5)     | 7/20 (vocab score, 1 s)    | 6/20 (74 s)      | NO LM ROI |
| 4 | Classify dialog-act (30 × 5)  | 27/30 (keyword rules)      | 20/30            | NO LM ROI |
| 5 | Verifier layering (paraphrase)| naive: 5/19, smart: 4/19   | unsafe v2: 9/19  | Verifier necessity confirmed, smart heuristic underperformed |

4 / 4 proposer tasks lost to cheap deterministic
alternatives that run 10-100× faster, are 100 %
reproducible, carry zero hallucination risk, and need
zero ML dependencies.

## Commits on this branch (chronological)

  b3ee0b88 — open + PROTOCOL.md
  f335c20f — Day 1: model decision + baseline harness
  87166e82 — Day 2-3: bare-baseline + Metal lesson
  a6acd34f — api-skeleton: adam-hybrid-llm crate (3 proposers)
  fe8cdfa7 — paraphrase-eval: 100-variant pack, baseline 81/100
  32de5a28 — hybrid wiring: +4 pp single-shot
  729e50ef — N-best: +9 pp (unsafe — hallucinations propagate)
  c521f41e — verifier-gate v3: +5 pp naive verifier
  f30c45e3 — smart verifier v4: +4 pp (genitive heuristic flopped)
  91348a87 — DETERMINISTIC alias baseline: +8 pp NO LM ⭐
  0a60aee5 — rescore_n_best baseline: 30 % vs 20 % random
  823a775d — DETERMINISTIC vocab rescorer: 35 % beats LM ⭐
  62faafee — classify_dialog_act: keyword 90 % beats LM 66 % ⭐

⭐ = test that established «deterministic ≥ LM» for that
proposer role.

## What the experiment DID produce — real artefacts

1. **Production win shipped to main as v6.8.33** — 7
   curated Kazakh paraphrase alias rules in
   `crates/adam-dialog/src/input_normalizer.rs`,
   surfaced by the paraphrase-eval phase, validated to
   close 8 / 19 misses with zero ML deps and zero
   regression on the 5 production eval suites.

2. **Architectural pattern catalog** — six verifier
   guards (Surface / Safety / FST / Predicate-alias /
   Frame / AnswerCandidate) sketched in the 2026-06-28
   Codex review.  When / if a future hybrid arc opens,
   start there, not at topic-noun heuristics.

3. **`crates/adam-hybrid-llm`** — env-gated subprocess
   wrapper around `llama-completion` exposing the three
   typed proposer APIs.  Default-off.  Reference
   implementation for any future hybrid attempt; the
   stdout-drain deadlock fix in particular is
   load-bearing infrastructure knowledge that took a
   debugging session to surface.

4. **Methodology checklist** — every future LM-side
   experiment should:
     * Run the DETERMINISTIC baseline FIRST before
       investing in LM.  Codex called this out and we
       still skipped it for 3 phases; the cost showed.
     * Use canonical answer / proof id, not substring
       match, for scoring.  «Алгоритм» → «Қадамдар
       тізбегі» is semantically correct, substring-strict
       graders count it wrong.
     * Set the promotion bar AFTER measuring baseline,
       not before.  The «≥ 20 pp» bar was mathematically
       unachievable on a baseline-81 % pack.
     * Stratify eval packs by handler type for n ≥ 300.
       n = 100 was too small to distinguish 4 pp from
       9 pp noise.
     * Measure latency budget + reproducibility
       explicitly — every LM call broke determinism on
       this branch and the temperature seed wasn't
       fixed.

## Why archive, not pivot to LoRA

LoRA was the last untested lever.  The honest cost-benefit:

  cost   : 10-50 h M2 training, bespoke (clean, defective)
           Kazakh paraphrase corpus assembly (non-trivial),
           4B-LoRA checkpoint maintenance, latency stays
           multi-second per call.
  best-case: + 10-20 pp on each of the 4 tests = + 6-10
           cases recovered absolute.  Still doesn't
           dominate the deterministic baselines that
           cost ~0 to author and ~0 to run.
  worst-case: indistinguishable from bare 4B (Kazakh
           Q&A data scarce; LoRA needs ≥ 5 K clean
           pairs to converge on a small model).

Opportunity cost:
  * Codex 2026-06-27 named DATA INGESTION PIPELINE as
    the missing piece for ADAM growth — raw KZ text /
    PDF / SOP → candidate facts → validator → human
    review → world_core / procedure DB.  This is where
    HIGHER ROI lives.
  * Industrial pilot direction (ССГПО / Allur / KIA)
    needs SOP coverage at the 50-100 procedure scale,
    not LM polish on the 15 we already have.
  * Speech-defect ceiling at 76 % is bounded by data,
    not by proposer cleverness — more curated
    speech-defect cases close more failures than any LM.

Archiving honest negative result and reallocating dev
attention to higher-ROI tracks IS the right move.

## What this branch did NOT prove (out of scope)

  * Whether a 7B+ model would behave differently.
    Hardware doesn't fit.
  * Whether multilingual LM with stronger Kazakh
    pre-training (Aya, Mistral-multilingual) would
    dominate.  Same hardware constraint.
  * Whether a TRUE Whisper N-best (rather than the
    synthetic 5-candidate sets used here) would yield
    different rescoring numbers.  whisper.cpp doesn't
    expose alternatives per segment.

These are reasonable future arcs IF the resource
envelope changes (better hardware, dedicated Kazakh LM
budget).  Today's branch closes with the data we have.

## How to read this branch

If you want to:

  * Understand WHY the experiment closed negative:
    read this file + the per-phase result markdown
    files under
    `experiments/hybrid_qlm/{baseline,paraphrase_eval,rescore_eval,classify_eval}/`.

  * Reuse the LM-side machinery in a future arc:
    `crates/adam-hybrid-llm/` is self-contained,
    default-off, harmless if linked but unused.
    The chat-template strip + worker-thread stdout
    drain are the load-bearing utilities.

  * Reuse the deterministic baselines in production:
    paraphrase aliases already live in
    `crates/adam-dialog/src/input_normalizer.rs` as of
    v6.8.33 on main.  Vocab-rescore + keyword
    classify are easy lifts when there's a
    production caller.

  * Audit the methodology before opening a similar
    experimental branch: «Methodology checklist»
    section above.

See also memory file
`project_codex_hybrid_qlm_review_2026_06_28.md` for the
mid-arc review that anticipated this verdict.
