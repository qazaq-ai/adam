# Training runbook — v6.5 self-learning loop

**Status:** active arc, v6.5.0-rc8+.

The audit cycle that took us through v6.5.0-rc1 → rc7 hit the
"infinite patch-loop" wall the v6.2 strategic pivot warned about.
rc7 closed the cycle with the persistence layer
(`data/mistake_corrections.jsonl`) and explicitly handed the next
move over to learning rather than patching.

This runbook is the canonical playbook for retraining the
intent / coherence / router stack from accumulated rejections.

## Why a runbook (not "just run cargo")

Three forces collide on this machine:

1. **8 GiB unified memory** — same pool as the OS, Chrome, Slack,
   VS Code.  Training a 5 M-param model on `burn-wgpu`+Metal works
   but evicts everything else first.
2. **~228 GiB disk with ~10 GiB free** under typical use — cargo
   target/ + HuggingFace cache eat most of it.  Training builds
   need 15–25 GiB headroom for fresh `target/release/` and
   checkpoint rotation.
3. **Determinism contract** — we do not regenerate the deterministic
   data packs from scratch on every train run.  The runbook commits
   to a fixed input snapshot and rotates only the neural side.

If any of those is ignored, training either OOMs, fails on disk,
or invalidates the rest of the build.  This runbook is the protocol
that keeps all three lined up.

## Pre-flight (every time)

```sh
bash scripts/free_for_training.sh
```

Expected output near the end:

```
=== summary ===
Free before: 9  GiB
Free after:  32 GiB
Recovered:   23 GiB
…
READY
```

If "Free after" is under 20 GiB, the script prints `WARNING`
instead of `READY` — close Chrome / Xcode / VS Code, then re-run.

The script is **idempotent**.  Running it a second time on a clean
machine reports `Recovered: 0 GiB` and exits cleanly.  Safe to
run on every wake-up.

## What can be trained today

| Trainer | Backend | Inputs | Output |
|---------|---------|--------|--------|
| `train_intent_classifier_gpu` | wgpu→Metal | `data/curated/adam_intent_training_pack.json` (1 180 samples × 52 intents) | `data/checkpoints/intent_classifier/` |
| `train_contextual_lm_gpu`     | wgpu→Metal | `data/training/contextual_lm/bpe_sequences.json` (63 362 sequences) | `data/checkpoints/contextual_lm/` |

Each trainer launches a 50-epoch run that completes in 14–60 s
on M2 wall-clock (per `data/checkpoints/intent_classifier_gpu.log`).
Training is short; **the bottleneck is the build, not the training**.
Plan for ~3 min of cold `cargo build --release` before the first
trainer iteration of the day.

## What needs to be added (rc9+ scope)

| Trainer | Inputs needed | Status |
|---------|---------------|--------|
| `synth_corrections_pack` | `data/mistake_corrections.jsonl` (5 records as of rc7 audit) | not built — rc9 deliverable |
| `train_joint_router_gpu` | `data/curated/adam_intent_training_pack.json` + synthesised corrections | not built — rc10 deliverable |

The `mistake_corrections.jsonl` records are the seed.  rc9 expands
each record into ~20–50 synthetic siblings (paraphrase + STT-noise
augmentation) so the joint router sees enough mass to learn from a
handful of audit sessions.

## Standard training procedure

Run order:

```sh
# 1. Pre-flight cleanup + verify free space.
bash scripts/free_for_training.sh

# 2. Sanity-build the trainer (catches dep drift before the long run).
cargo build --release -p adam-agg-model \
  --bin train_intent_classifier_gpu

# 3. Train.  The trainer logs to data/checkpoints/<name>_gpu.log
#    and writes the checkpoint to data/checkpoints/<name>/.
cargo run --release -p adam-agg-model \
  --bin train_intent_classifier_gpu \
  2>&1 | tee data/checkpoints/intent_classifier_gpu.log

# 4. Sanity test: run the affected voice REPL build against a
#    small fixture battery.  Catches "trained but doesn't load"
#    regressions immediately.
cargo test --release -p adam-voice-repl-v6-3

# 5. Live audit.  Per the autonomy memory we don't run the audit
#    ourselves — hand back to the user.
```

## When to retrain

| Signal | Action |
|--------|--------|
| `data/mistake_corrections.jsonl` grew by ≥ 20 lines since last train | retrain intent + joint router |
| Live audit found wrong intent on a high-confidence input | extend pack first, then retrain |
| Coherence false-positive / refuse rate up across a session | retrain coherence side |
| Whisper drift introduces new tokens not in BPE-5188 | rebuild tokenizer first, then everything downstream |

## Anti-patterns

- **Don't** retrain on a single new correction.  Synthesize ≥ 20
  paraphrases first.  One-shot fitting overfits the model to the
  audit transcript.
- **Don't** retrain WITHOUT running `scripts/free_for_training.sh`
  first.  An OOM mid-train can leave the checkpoint dir half-written
  and the trainer hangs on disk-full on the next run.
- **Don't** retrain in two separate terminals at once.  Both jobs
  fight for the GPU and the slower one loses its `vmstat` budget.
- **Don't** delete `data/mistake_corrections.jsonl` to "start fresh".
  Every record is hard-won audit evidence.  Append-only.
- **Don't** commit fresh checkpoints without running the gates from
  the release runbook.  The checkpoint is a binary; clippy / fmt
  won't catch a regression on it.

## Glossary

- **Joint router** (rc10 target) — a single model that does intent
  classification + coherence + cascade-route selection.  Replaces
  three hand-tuned layers with one trained surface.  The hypothesis
  is that the three signals are not independent and a joint model
  reads them together more accurately than the cascade does.
- **Pre-flight** — the cleanup + check sequence that runs before
  every training session.  Not part of the trainer itself; lives in
  `scripts/free_for_training.sh`.
- **Audit-to-data loop** — the rc5–rc7 pipeline that turns live REPL
  rejections into a JSON Lines file.  Closed loop: audit → reject →
  persist → synthesize → train → audit.
