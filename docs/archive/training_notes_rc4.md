# rc4 training sweep — notes for rc5

**Date:** 2026-05-19
**Hardware:** MacBook Air M2, 8 GB RAM, 8 CPU cores (4P + 4E), 8-core GPU
**Backend:** `burn-ndarray` (CPU only, single-thread per process default)
**Status:** 3 / 4 checkpoints completed; 1 (D mixed-real) crashed in eval phase.

## Goal

Compare four architecture/data combinations on the L5.5 agglutinative
composer (rc1 PoC baseline = 1.17 M params). Establish a calibrated
rc5 baseline.

## Sweep configuration

All jobs: 1 epoch, batch 64, lr 0.003, FST round-trip target.
Data split: 90 % train / 10 % held-out, deterministic seed 42.

| Job | Architecture | Train data | Notes |
|---|---|---|---|
| **A** small fst | d=128, l=3, h=4, ff=512 | 65 104 FST-synth pairs | ~2.8 M params |
| **B** medium fst | d=192, l=3, h=6, ff=768 | same FST-only | ~4.7 M params |
| **C** large fst | d=256, l=4, h=8, ff=1024 | same FST-only | ~7.6 M params (≈ rc4 GA target) |
| **D** medium mixed | d=192, l=3, h=6, ff=768 | 65 104 FST + 50 625 real (α=0.5) | crashed in eval |

## Headline results

| Job | params | held-out CE | train-end CE | gap | wall-clock |
|---|---|---:|---:|---:|---:|
| **A** | 2.81 M | **0.687** | 0.281 | 0.406 | 23.8 min |
| B | 4.65 M | 0.726 | 0.274 | 0.452 | 32.8 min |
| C | 7.58 M | 0.720 | 0.278 | 0.442 | 22.6 min |
| D | 7.63 M | (crashed) | end 0.383 > mid 0.354 | — | ~32 min before crash |
| _rc1 PoC reference_ | _1.17 M_ | _0.416_ | _0.368_ | _0.048_ | _~95 min_ |

(Per-job FST round-trip validity on synth output: **500 / 500 = 100 %** for
A / B / C. Verifier L6 check on 100 sampled training surfaces: 100 / 100
passed for all completed jobs, 0 hallucinations.)

**Best held-out CE: A (2.8 M params, FST-only).**

## Three findings worth carrying into rc5

### 1. Bigger models do not generalise better on FST-only data

A (2.8 M) < C (7.6 M) < B (4.7 M) on held-out CE. The
train-CE band is narrow (0.274–0.281) — all three memorise the FST
surface roughly equally well — but the smaller model is the only one
that retains useful inductive bias for unseen prefix combinations.
Bigger = more aggressive overfit, **not** better algebra learning.

**rc5 implication:** stop scaling model. Either scale **data** (D was
the right direction, but unstable) or improve the training **schedule**
(see #2). The rc1 PoC (1.17 M) is still on the Pareto frontier —
its gap of 0.048 is far better than rc4's smallest model's 0.406,
suggesting the rc4 sweep is in an over-fit regime even at A's size.

### 2. Real-corpus mixing is the right direction but the schedule is wrong

D's loss curve: `start=8.979 → mid=0.354 → end=0.383`. Loss **went up**
in the second half of the epoch. This is the first non-monotonic curve
in any rc1–rc4 run. Combined with the eval-phase crash (likely OOM under
the larger vocab + real data buffers), it points to:

- **lr=0.003 is too aggressive** for the mixed-distribution batches.
  Once the model has absorbed the clean FST shape, real-corpus batches
  inject high-magnitude gradients that push it out of the minimum.
- **alpha=0.5 may be too much real** in a one-epoch schedule.
  Recommend a curriculum: pure FST warmup → introduce real linearly.

**rc5 schedule recommendation:**
- lr warmup (200 steps to 0.003, then cosine decay to 3e-4)
- alpha=0.1 first half-epoch, ramp to 0.5 second half
- 2 epochs minimum so the higher-alpha second pass has time to recover
- Adam optimizer state explicitly persisted between epochs

### 3. `burn-ndarray` is single-thread per process — parallel processes beat one big run

Smoke run of a single CPU job: 20 % CPU (≈ 1.5 of 8 cores). Four
parallel jobs: ≈ 360-400 % CPU (≈ 4 cores) — Accelerate BLAS
auto-threads when memory bandwidth permits. RAM-pressure point:
≈ 2.5 GB working set per medium job, so **3 mixed jobs = swap thrash**
on 8 GB M2. Two jobs + browser/IDE closed = stable.

**rc5 implication for hardware:** if we stay on CPU + M2, the realistic
unit is 2 parallel jobs of ≤ 7 M params each. Bigger sweeps need
`burn-wgpu` (Metal) — but burn 0.17's wgpu Metal backend is
historically unstable; needs a smoke test before committing the
sweep budget to it.

## What did not get measured

- **factual_eval_100 with neural backend.** `adam_chat` with
  `--features neural --neural-model <dir>` would integrate any
  A/B/C checkpoint into the dialog turn, but rc4's
  `factual_eval_100.rs` runner does not exercise that surface yet
  — it measures the deterministic kernel + verifier, which is the
  rc4 gate (34 hallucinations baseline). Neural-aware eval is a
  rc5 deliverable.
- **D held-out CE.** D crashed before eval. Re-running with the
  rc5 schedule (above) should produce a clean number.

## Operational notes for rc5 sweeps

1. **macOS swap autosize works.** Initial swap cap was 2 GB; after
   pressure macOS expanded to 14 GB. Don't manually fix the cap —
   just close the browser before starting the sweep.
2. **/tmp is wiped on reboot.** Logs live there; metrics survive
   because `training.json` is written into the checkpoint dir
   (which is in `data/checkpoints/...`, persistent).
3. **`scripts/compare_rc4_sweep.sh`** parses checkpoint `training.json`
   directly — no /tmp dependency.
4. **Per-batch progress logging is absent.** `poc_kazakh_train` only
   prints stage boundaries; during the 20-30 min `[5/6]` phase you
   cannot tell if it's making progress without sampling `ps` for
   CPU usage. Adding step-by-step log lines (every 100 steps) is a
   one-line rc5 fix.

## rc5 plan summary

1. **Backend smoke test:** wire `burn-wgpu` Metal feature behind
   `--features neural-gpu`. Run d=128 / 1 M params smoke; if it
   completes without crash, baseline the next sweep on it.
2. **Schedule rewrite:** lr warmup + cosine decay + alpha curriculum
   (see finding #2). Keep model size at 2-5 M params (finding #1).
3. **Re-run D with new schedule:** if held-out CE drops below 0.687
   (current A best), real-corpus mixing is validated and rc5 ships
   that checkpoint as the neural-preview default. If not, scale
   real-corpus pool from 50 k to 100 k pairs before scaling model.
4. **Wire neural backend into `factual_eval_100`** so the gate
   measures end-to-end neural + deterministic + verifier — not
   just deterministic.
