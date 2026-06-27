# Day 3 — bare-baseline summary

**Date:** 2026-06-27
**Branch:** `experiment/hybrid-qlm-verifier`
**Models tested:** Gemma-3 1B-it q4_k_m, Gemma-3 4B-it q4_k_m
**Inference path:** `llama-completion` via Metal (`-ngl 99`)

## Hardware + Metal — critical finding

Apple Silicon `llama.cpp` REQUIRES `-ngl N` to use the Metal GPU.
Default behaviour without `-ngl` is **CPU-only** on M-class
hardware, which causes catastrophic swap thrashing as the
model spills past the ~5 GB usable working set on an 8 GB
M2 (the 4B q4_k_m gguf is ~2.3 GB on disk + KV cache + context
+ inference scratch reaches ~3 GB working set; on CPU it's
all going through main memory).  With `-ngl 99` the weight
tensors live in unified-memory GPU pages and the inference
loop runs through Metal compute kernels — no swap thrashing,
no I/O stalls.

Numbers:
  * 1B q4_k_m, 10 probes, Metal: **13.5 s total** (~1.3 s/probe)
  * 4B q4_k_m, 10 probes, Metal: **28.0 s total** (~2.8 s/probe)
  * 4B q4_k_m, CPU-only earlier attempt: **>1 hour, never completed**

This is a one-off lesson: every llama.cpp invocation on this
hardware MUST set `-ngl 99` (or `-ngl auto` once the build
supports it).  The `run_bare_baseline.sh` harness now does
this by default.

Also: `llama-cli` in llama.cpp build 9820 (~late 2026) became
**interactive-only**.  Single-shot generation moved to
`llama-completion`.  Old `-no-cnv` flag is rejected with
«--no-conversation is not supported by llama-cli — please use
llama-completion instead».

## Probe results — head-to-head

| #  | Query                                  | 1B answer                                  | 4B answer                                                   | 1B | 4B |
| -- | -------------------------------------- | ------------------------------------------ | ----------------------------------------------------------- | -- | -- |
|  1 | Қазақстанның астанасы қай қала?        | Алматы                                     | Астана                                                      | ✗  | ✓  |
|  2 | Темірдің химиялық таңбасы қандай?      | Ксенон                                     | Fe                                                          | ✗  | ✓  |
|  3 | Күмістің химиялық таңбасы қандай?      | Cu                                         | Cu                                                          | ✗  | ✗  |
|  4 | Судың формуласы қандай?                | 27                                         | H₂O                                                         | ✗  | ✓  |
|  5 | Жүрек не үшін керек?                   | тілеу, қатысу                              | өмірде қажет                                                | ✗  | ✓  |
|  6 | Ахмет Байтұрсынұлы кім?                | Республикасының әже                        | Қазақ ғалымы, этнограф, тарихшы                             | ✗  | ✓  |
|  7 | Қазақстанның ұлттық валютасы.          | Қазақстаны                                 | Тенге                                                       | ✗  | ✓  |
|  8 | Алматы туралы қысқаша айтшы.           | құшбас қаймасы (Turkish drift)             | «Қазақстанның астанасы, ең үлкен қаласы. Шығыс Қазақстанда» | ✗  | ✗  |
|  9 | СИЗ беру тәртібі қандай?               | hand-wave                                  | Білмеймін                                                   | ✗  | ✓* |
| 10 | Хазах деген не?                        | көкжие, көкжие                             | Білмеймін                                                   | ✗  | ✓* |

`✓*` = honest «I don't know» (system prompt asked for this).

**Score: 1B = 0/10.  4B = 7/10 correct + 2 honest «не знаю» + 1
confident hallucination.**

## What this validates

1. **adam-kernel verifier is non-negotiable, not theoretical.**
   The 4B output on probe 3 («Cu» for «silver») and probe 8
   («Алматы — астана» — confidently wrong on TWO claims in one
   sentence — Almaty is NOT the capital and is in south-east,
   not east, of Kazakhstan) confirms that even the much
   stronger 4B baseline hallucinates confidently 10-20 % of
   the time.  Any pipeline that lets the LM speak facts to
   the user without grounding WILL hallucinate.  Verifier
   gate per the PROTOCOL.md «load-bearing invariant» is the
   only safe deployment shape.

2. **1B is too weak to be useful as a Kazakh candidate
   generator.**  100 % gibberish on factual probes, with
   syntactically Kazakh-looking but semantically empty
   output.  Worse, it doesn't even honour the
   «біле алмасаң — білмеймін» system-prompt instruction —
   the system prompt clears the 4B but not 1B.  Decision:
   **drop 1B from the experiment.**

3. **4B is workable as candidate generator behind verifier.**
   28 s for 10 probes ≈ 2.8 s per call.  That's tolerable for
   paraphrase / dialog-act / ASR rescore pre-cascade work
   (one call per turn).  Real-time response budget on the
   text REPL stays under 5 s.

4. **System prompt «біле алмасаң — білмеймін» measurably
   reduces hallucination on the 4B base.**  2/10 «Білмеймін»
   responses on probes the LM genuinely doesn't have
   coverage for (industrial SOP, defective speech token).
   This gives the verifier a CHEAP first signal — if the LM
   itself says «don't know», don't even attempt grounding.

5. **Cu / Ag confusion (probe 3) is the canonical
   verification target.**  This is exactly the kind of
   «sounds right, isn't right» error that adam-kernel
   would catch via world_core's `chemistry_school` domain
   (Ag is silver, Cu is copper — both are first-class facts
   in our graph).  Plan the verifier API to expose this kind
   of cross-check.

## Decision for Day 4-7

Proceed with **Gemma-3 4B-it q4_k_m** as the experiment's
base model.  1B is archived as «too weak; documented for
reference».

Day 4-7 phasing per PROTOCOL.md:
  * Design + ship `crates/adam-hybrid-llm` skeleton with
    three proposer APIs: `propose_paraphrase`,
    `rescore_n_best`, `classify_dialog_act`.
  * Wire as `ADAM_HYBRID_LM=1` env-gated path.
  * Verifier integration uses the existing AnswerCandidate +
    ProofRef layer (v6.8.26) — LM-proposed text becomes an
    `AnswerCandidate` with `ProofRef::LegacyCascade`
    initially; verifier promotes to `CuratedFact` only when
    world_core grounds the claim.

## Reproducibility

```sh
# All probes run from repo root, fresh RAM.
sudo purge
bash experiments/hybrid_qlm/baseline/run_bare_baseline.sh
# 1B by default; 4B via:
LM_MODEL=data/lm_models/gemma-3-4b-it-q4_k_m.gguf \
OUT_FILE=experiments/hybrid_qlm/baseline/day_3_results_4b.md \
  bash experiments/hybrid_qlm/baseline/run_bare_baseline.sh
```

Raw outputs in `day_3_results.md` (1B) and
`day_3_results_4b.md` (4B).
