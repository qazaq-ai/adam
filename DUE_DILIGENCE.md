# adam — Due Diligence Pack

**Last updated:** 2026-06-17
**Branch:** `experimental/v6_6_generative_pivot` (release-candidate
for v6.8.0; merge to `main` pending the gaps listed in §8)
**HEAD commit:** `fdf21456` (v6.8.0)
**Workspace version:** `6.8.0`

This document is intentionally a flat list of facts. Numbers come from
commands you can re-run yourself; everything labelled "limitation" or
"known issue" is something we surface rather than hide.

---

## 1. What adam is

A deterministic, offline-first Kazakh-language AI kernel. The dialog
core (v6.2) is rule-based — typed morphological pipeline → retrieval
over a curated fact graph → reasoner → realiser. The voice surface
(v6.3) wraps that core in a microphone → STT → fuzzy/LM rescoring →
intent classifier → router → TTS loop. Neural components live ONLY
at the audio ↔ text boundary; they never invent facts.

- **Language:** Rust (workspace, 37 crates, 160 692 LOC under `src/`).
- **License:** BUSL-1.1.
- **Hardware:** MacBook Air M2, 8 GB RAM (the daily development target).
- **No cloud dependency:** every component runs locally; no inference
  API calls, no telemetry.

## 2. Repository state

| Metric | Value |
| --- | --- |
| HEAD branch | `main` (v6.8.0 merged 2026-06-17; post-release patches at HEAD) |
| HEAD commit | `4595db45` |
| `school_program_eval` (14 subjects, production) | **159 / 159 = 100 %** semantic |
| `conv_dialog_eval` (44 real voice REPL + 39 scripted) | **52 / 52 = 100 %** semantic (31 probes document gaps) |
| `safety_eval` (13 categories, Codex #3 release gate) | **20 / 20 = 100 %** semantic (34 probes; v6.8.0 shipped at 16 / 16, then v6.8.2 patches promoted 4 dangerous probes (suicide method, severe bleeding, account intrusion) — all now pass) |
| `v6_7_real_audit_eval` | **25 / 26 = 96 %** semantic (v6.8.0 shipped at 24 / 26 = 92 %; v6.8.2 cascade-order inversion improved by 1) |
| `speech_defect_eval` (8 defect categories) | 37 / 71 = 52 % semantic (honest baseline; v7 milestone) |
| Voice REPL live session (45 turns, 2026-06-16 evening) | 43 / 45 ≈ 96 % |
| Legacy `blind_eval_v1` (kept as historical reference) | 97 / 100 (rc18 baseline; no longer the production gate) |
| Workspace test files | 274 (across 28 crates + 10 tools) |
| Largest crate (LOC) | `adam-dialog` (extended via v6.5 safety guard + v6.8 cascade patches + wellness::pain_support) |
| World-core curated entries | 3 444 in `data/world_core/*.jsonl` (65 domains, unchanged since v6.3) |
| Reasoning facts | 4 116 extracted + 37 991 derived |
| Retrieval index | 5 543 morphemes / 124 477 postings |
| Hand-curated corpus lines | 7 749 869 (`data/curated/`) plus the v6.7 / v6.8 generative-pivot packs |
| `.git` directory size | ~460 MB |
| `data/` directory size | 5.7 GB tracked + ~700 MB untracked v6.8 training corpora |

Repository is monorepo-style. `crates/*` is the kernel; `tools/*`
holds binaries (voice REPL, chat REPL, evaluators); `data/*` ships
curated facts, corpora, and (NOT in git) model checkpoints.

## 3. How to build and test

Pre-requisites: Rust 1.83+ (stable channel), pkg-config, ALSA / CoreAudio
headers. Whisper.cpp and Piper are optional (voice REPL only).

```sh
git clone <repo>
cd adam
cargo fmt --all --check         # passes on HEAD
cargo build --release --workspace
cargo test --release --workspace
```

### Current test totals on HEAD

```
cargo test --release --workspace
# 274 test files across the workspace; run locally to get the
# definitive pass/fail/ignored breakdown.
```

The ignored tests are flagged with `#[ignore]` for documented reasons
(slow Whisper integration tests; tests requiring missing data assets;
deferred fix tracking). They are inventory, not hidden failures.

### Production eval suite totals on HEAD

The dashboard below is the single source of truth for capability
claims. All five suites are measured via the `respond_full` binary
(Phase 16 deterministic cascade + Phase 17 world_core domain index +
`ADAM_V6_2=1`), which mirrors the production voice REPL pipeline.

```sh
cargo build --release --bin respond_full
./target/release/respond_full data/eval/school_program_eval.json
./target/release/respond_full data/eval/conv_dialog_eval.json
./target/release/respond_full data/eval/safety_eval.json
./target/release/respond_full data/eval/v6_7_real_audit_eval.json
./target/release/respond_full data/eval/speech_defect_eval.json
```

Each invocation prints `strict` (exact-token match) and `semantic`
(content-roots overlap ≥ 0.70) scores, plus per-subject and
per-topic rollups. The numbers reported in §2 above are semantic
scores at HEAD `fdf21456`.

The contrast with `respond` (LM-only) is intentional: `respond` runs
the v6.7 generative head WITHOUT world_core / reasoning / domain
inference, so its school_program score (78 %) reports the LM ceiling
alone, not the production ceiling.

### The 1 failing test (`rust_book_chapter_15_holdout`)

```sh
cargo test --release -p adam-dialog --test rust_book_chapter_15
# → 1/18 cases fails: ch15_smart_pointer
# Query: «Ақылды сілтеме деген не?» → got: «Рахмет»
# Expected any of: «қосымша мінез-құлық», «weak<t>», «interior mutability»
```

**This test passed in v4.85.5** (commit `6eaa579c`, the release that
shipped chapter 15). Some intervening refactor across ~100 commits
broke routing for that one query — substring «рахмет» (thanks) fires
instead of the smart-pointer concept lookup. This is the only red
regression test in the workspace; tracked for a focused fix.

### Replay battery (voice REPL transcripts)

```sh
cargo test --release -p adam-dialog --test replayed_voice_repl
# → 4/4 pass on HEAD (full_replay_battery_passes covers 27 transcript cases)
```

This battery pins exact Whisper-transcribed user utterances from real
audit sessions to required substring answers. New voice REPL findings
become permanent regression entries (memory feedback rule).

## 4. Architecture in one diagram

```
voice in (cpal mic → 48 kHz PCM)
        ↓
  STT layer (Whisper.cpp default; in-tree DTW fallback)         ┐
        ↓                                                       │  v6.3
  Token-split merge + Zipf fuzzy + neural LM rescoring          │  voice surface
        ↓                                                       │  (controlled
  BPE neural intent classifier (1 M params, parallel log)       │   neural
        ↓                                                       │   boundaries)
  Phase 19.G high-conf override (input rewriting)               ┘
        ↓
  adam-dialog v6.2 router  ────────────────────────┐
        ↓                                          │   v6.2
  Typed QueryIR + FrameIndex retrieval             │   deterministic
        ↓                                          │   core
  Reasoner + Realiser + math solver + clock        │   (rule-based,
        ↓                                          │    inspectable,
  Kazakh response sentence                         ┘    auditable)
        ↓
  Piper TTS (kk_KZ-issai-high subprocess) → speaker      ┐ v6.3
                                                         ┘ neural
```

The architectural commitment we ship under is **honest hybrid**: the
dialog core is 100 % rule-based; neural components live exclusively
at the speech surface and never participate in deciding what's true.

## 5. Reproducing the published numbers

### `cargo fmt` clean

```sh
cargo fmt --all -- --check    # exit code 0 on HEAD
```

### Voice REPL live loop

```sh
cargo build --release -p adam-voice-repl-v6-3
./target/release/voice_repl_v6_3 --loop-mode --mode respond
```

Startup log on a fully-configured machine prints:

```
[voice-repl] dialog engine: lexicon + 151 template families loaded
[voice-repl] retrieval: 5543 morphemes / 124477 postings indexed
[voice-repl] reasoning: 4116 facts + 37991 derived loaded
[voice-repl] world_core: 3736 domains / 3444 entries indexed
[voice-repl] hot vocab: 82 overrides + 3065 curated lexicon = 3147 total
[voice-repl] neural rescorer: ready (vocab=5188, ckpt=data/checkpoints/contextual_lm)
[voice-repl] intent classifier: ready (vocab=5188, n_intents=52)
```

If models are missing the loaders print `missing checkpoint files` and
the voice REPL runs without that subsystem (dialog still works).

### Latency on the M2 reference machine — three classes

Honest split — do not conflate when comparing to other systems.

- **Stage 3 typed-kernel micro-path**: ~470 ns avg (`adam-algebra`
  Stage 3 — algebraic Frame→Index→answer only; excludes router /
  retrieval / reasoning / realiser).
- **Production cascade** (the real number for text queries through
  `adam_chat`): **13.6 ms p50 / 19.6 ms p95 / 314 MB peak RSS** on
  the 30-query production battery (`adam_resource_bench`).
- **Voice REPL** end-to-end: 3–8 s wall (Whisper STT + Piper TTS
  dominate; the dialog cascade itself is the 13.6 ms number above).

The Stage 3 micro-path latency is useful for comparing kernel
versions to each other.  It is NOT a fair head-to-head against
full NLP systems (Llama 3.1 / Claude / GPT) — those are a
different system class (128K–1M context, broad open-domain).
adam's narrow deterministic Kazakh scope is what makes the
small footprint possible; same-task blind eval is the right
comparison instrument, not raw latency.

## 6. Data assets and where they come from

Not everything in `data/` is in git. The 5.7 GB on a developer
machine breaks down as:

| Subtree | Size | In git? | Origin |
| --- | --- | --- | --- |
| `data/world_core/*.jsonl` | < 20 MB | ✅ | hand-curated facts |
| `data/curated/*` | ~ 100 MB | ✅ | cleaned Kazakh corpora |
| `data/checkpoints/` | 61 MB | ❌ | trained in-house from `data/curated` |
| `data/stt_models/` (Whisper.cpp) | 4.0 GB | ❌ | upstream releases (multilingual ggml + Shirali fine-tune) |
| `data/tts_models/` (Piper kk_KZ) | 1.1 GB | ❌ | upstream Piper voice catalog |
| `data/v6_3_phoneme_bank/*.bin` | 40 MB | ❌ | derived from `data/curated` audio |
| `data/v6_3_corpus/` | 8 KB | ⚠ manifest only | placeholder; bank still needs MANIFEST work |

The model checkpoints we ship in-house (`data/checkpoints/`,
`data/v6_3_phoneme_bank/`) are reproducible from the curated corpus
plus the training binaries in `crates/adam-agg-model/src/bin/`. The
Whisper / Piper models are upstream third-party artefacts; we do not
redistribute them, the user downloads them once.

A fetch-script for the third-party models is a planned deliverable
under codex review recommendation 5.

## 7. Test discipline

- **Unit / integration tests:** `cargo test --release --workspace`
  runs the whole suite in ~30 s on M2.
- **Bench targets:** `cargo bench -p adam-dialog` for the router;
  output captured in `docs/v6_2_*.md`.
- **Voice transcript replay:** every voice REPL audit finding lands
  in `crates/adam-dialog/tests/replayed_voice_repl.rs` as a permanent
  case. Phase 21 (relative-day handler) added 6 such cases.
- **Holdouts:** `data/eval/*.json` holds blind evaluation sets. The
  `rust_book_chapter_15_holdout.json` and `live_holdout_*.json` files
  feed dedicated integration tests with 100 % pass requirements.

## 8. Known limitations / honest gaps

### Open at HEAD (post-v6.8.0 patches)

1. **`speech_defect_eval` at 52 %** (37 / 71 semantic). The eval
   establishes an honest baseline against eight defect categories
   (rhotacism / sigmatism / lambdacism / kappacism / nasalisation /
   stuttering / elderly / whisper). The architectural fix is the
   v7 milestone: candidate rescoring + FST fuzzy match BEFORE
   `Conversation::turn`. `lookup_possessive_property` substring
   matching is the brittle surface; v6.8 added an edit-distance
   ≤ 1 fallback to recover the easy cases.
2. **`v6_7_real_audit_eval` at 96 %** (25 / 26 semantic, was 24 / 26
   at v6.8.0 release; the cascade-order inversion in the v6.8.2
   safety patches closed one of the three known deviations). One
   deviation remains.
3. **BTC live-data routing** — «Қазір BTC бағасы қанша?» — «қазір»
   triggers a date-query intent BEFORE `detect_safety_topic` runs;
   cascade-order hoisting is the fix. Documented as a probe in
   `safety_eval`; counted against the v7 release.
4. **Identity templating** — «Сен адамсың ба?» — production gives
   a `дерек жоқ` refusal. No identity-template family yet;
   ideal answer would be «мен тілдік модельмін».
5. **Eval comparator `match_type` taxonomy** — current semantic
   comparator can false-positive on fallback responses («дұрыс
   түсінбедім» passes when expected substring overlaps with a
   stop-word in the fallback). User audit 2026-06-17 flagged
   this; a typed `match_type` (`exact_value` / `contains_answer`
   / `safety_refusal` / `no_data` / `dialog_state` / `math_scalar`)
   refactor is the v7 fix.
6. **Fuzzy lock behind intent + entity parse** — current `fuzzy_normalise`
   can run pre-cascade and rewrite a query before the disambiguator
   sees it («Байтта неше біт **баланың аты** бар?» → «Бір байтта 8
   бит бар» false win). Gating fuzzy behind a high-confidence
   intent + entity parse is the v7 fix.

### v6.3-era gaps still open

5. **Voice STT word-error-rate on natural Kazakh** is high
   (`stt_eval` harness target). Whisper.cpp default mishears
   Kazakh-specific letters (Қ, Ғ, Ң) without the Shirali
   fine-tune; v6.3 produced fuzzy + LM rescoring to recover but
   it's not perfect. v6.6 drift-aware BPE + LM lifted recovery
   TP from 25 % to 75 % on the rc25 audit eval.
6. **STT-eval reproducibility** — `data/v6_3_phoneme_bank/MANIFEST.jsonl`
   and `data/v6_3_corpus/MANIFEST.jsonl` are not in git (size +
   the derivation-vs-redistribution policy). Reproducing the
   published PER numbers needs the corpus fetch + bank rebuild
   scripts.
7. **«Заученность» (response monotony)** — Phase 20 paraphrase
   variants cover high-frequency static responses; topic queries
   («Қазақстан туралы айтшы») still return short fact lists.
   Phase 20.5 knowledge-aware multi-fact rendering is the
   planned fix.
8. **Name resilience** — Whisper mishears «Дәулет» as «Дауыл» /
   «Дауылед» / «Атом», breaking the name-recall path. The 2026-
   06-16 voice REPL audit confirmed the gender-filtered name DB
   path (pitch detector + gender-labeled lexicon) is wired up
   but not threaded into `best_name_match` — that wiring is the
   PropertyQueryIR work item.
9. **`rust_book_chapter_15_holdout` 17 / 18** — `ch15_smart_pointer`
   fails (see § 3). Tracked.
10. **GitHub repo size (~ 460 MB `.git`)** — most weight is `.git`
    history from the curated corpus growing over time. Pruning
    history or moving large files to LFS is on the 14-day plan.

## 9. License + dependency posture

- adam itself: BUSL-1.1 (Business Source License; converts to
  Apache-2.0 after 4 years per file).
- Direct third-party runtime deps (selection):
  - `burn` (neural framework, Apache-2.0 / MIT)
  - `cpal`, `hound`, `rustfft` (audio I/O, Apache-2.0 / MIT)
  - `serde`, `serde_json`, `thiserror` (Apache-2.0 / MIT)
  - `rayon` (CPU parallelism, Apache-2.0 / MIT)
- Out-of-tree models:
  - Whisper.cpp + ggml binaries: MIT.
  - Piper voices: MIT (catalog) / individual voice licenses vary.
  - We download upstream releases; we do NOT bundle redistribute.

The kernel directive is **pure Rust + surgical deps**: no `chrono`
(rolled our own 6-function calendar), no `regex` in the hot path, no
HTTP client in the dialog crates.

## 10. Hardware + runtime profile

- **Reference machine:** MacBook Air M2, 8 GB RAM, macOS 15.6.
- **Voice REPL RSS:** ~ 1.5–2 GB peak (Whisper + Piper dominate;
  dialog core < 200 MB).
- **Watch-target architecture:** the v6.2 deterministic core is
  designed to run on ARM Cortex-M / Apple Watch class hardware; the
  Phase 21+ work has not regressed that envelope (~ 200 MB RSS).
- **No GPU required** in production. GPU (Metal via wgpu) is used
  only for in-house training (intent classifier, contextual LM).

## 11. What's NOT in this document

- Commercial terms (ask amount, equity split, runway).
- Partner / customer pipeline (covered separately).
- Roadmap dates beyond the immediate phase backlog.
- Marketing claims about "better than X".

If a reader wants any of those, request a separate `INVESTOR_BRIEF.md`
— deliberately kept out of this technical pack so this file stays
verifiable from `cargo` output alone.

## 12. Contact + repo

- GitHub: github.com/qazaq-ai/adam (mirror) /
  github.com/qazaq-ai/agglutinative-foundation (history).
- Maintainer: Daulet Baimurza (`baimurza.daulet@gmail.com`).
- Branch convention: `experimental/v<N>_<arc-name>` for in-progress
  arcs; `main` is the deterministic core, currently at v6.2.
