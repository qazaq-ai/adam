# v6.5 — strategic exit from the patching cycle

**Status:** design (2026-06-08).
**Predecessor:** v6.4.x (rc1–rc12, wellness arc).
**Branch:** `experimental/v6_5_corpus_training` — not yet created.

## Why a separate arc

The v6.4 wellness arc shipped 12 release candidates between 2026-06-04 and 2026-06-08. Each one fixed audit findings. By rc12 the cycle's diminishing returns became visible — user feedback verbatim:

> «продвижения никакого, просто топчемся на месте.»

The recurring failure modes are **not** wellness-arc bugs. They are **upstream** issues — STT noise, intent classifier confusion, retrieval sense ambiguity — that surface in wellness but live in the data + neural surface layers. Patching the wellness arm with vocab additions is sisyphean; v6.5 attacks the root causes.

## Root cause analysis (from rc1–rc12 audits)

| Symptom class | Recurring example | True cause | Patch path (so far) | Limit |
|---|---|---|---|---|
| Operator vocab drift | «кубейт» / «бөль» / «көбей» / «азайыт» | Whisper character substitutions on small operator inventory | Extend `tokenize` match arms (rc12 fixed the duplicate gate) | Adversarial — every audit surfaces a new variant |
| Name DB miss | «даулет» (ә→а) vs DB «дәулет» | Same — Whisper character substitutions on names | Fuzzy DB lookup at 0.85 threshold (rc11) | Brittle on multi-char drift |
| Intent classifier mislabel | «ашуланып» → AskWillingness 0.67 (wrong) | Classifier trained on clean text, sees Whisper-noised text in prod | Promote emotion-content above factual whitelist (rc10) | Doesn't fix the classifier itself |
| Wellness sticky | PostEscalation re-emits 150 even on factual q | Routing predicate too broad | Narrow `is_active` to IFS work stages only (rc9) | Refactor only; doesn't prevent class of bug |
| Sense disambiguation miss | «тіл» = язык/орган/программирование | Retrieval has no contextual sense gate | none yet | Needs corpus expansion + sense tagging |
| TTS prosody | «(1991–2019)» read literally | No TTS pre-processor for parentheticals | rc11 added year-range rewrite | Each new template surface needs a new rewrite |

**Five of six classes resolve to two ROOT issues**:

1. **STT character / vocabulary noise** — production sees Whisper-mangled Kazakh. Vocab patching is a losing race.
2. **Classifier / retrieval trained on clean text, not noise** — every neural component drifts under the data distribution it never trained on.

## Strategic moves (ranked by leverage)

### Move D — Sentence-coherence layer (HIGHEST UX impact, MEDIUM effort)

Single pre-routing gate that combines three signals:

```rust
struct CoherenceScore {
    lm_perplexity:      f32,  // contextual_lm rescorer output
    intent_confidence:  f32,  // neural intent classifier max-class
    morph_coverage:     f32,  // % tokens that FST analyses parse
}

fn is_coherent(s: &CoherenceScore) -> bool {
    s.lm_perplexity < THRESHOLD_PERPLEXITY
        && s.intent_confidence >= 0.40
        && s.morph_coverage >= 0.60
}
```

When `is_coherent` returns false → adam emits **«Кешіріңіз, сұрағыңызды толық түсінбедім. Қайталай аласыз ба?»** instead of guessing. This is the user's «валидировать слова в комплексе всего предложения» from rc8 audit.

**Effect:** the WRONG dict-lookup / WRONG ack template responses (audit T3, T22, multiple math fails) stop happening. adam becomes honest under uncertainty.

**Calibration:** threshold tuning against a real audio batch — NOT heuristic guess. Need ~50 audio recordings categorised as «coherent» vs «noise»; find the threshold that maximises F1.

**Risk:** false negatives on legitimately-noisy production speech. Mitigated by setting thresholds CONSERVATIVELY (favour accept over reject), then tightening only after live audit signals the false-accept rate is acceptable.

### Move B — Intent classifier retrain with STT-noise augmentation (HIGH impact, HIGH effort)

Current classifier:
- 52 intents
- 3 389 examples = **~65 per intent average**
- Trained on **clean** text

Production:
- Input is Whisper-noised
- Confidence drops to 0.3–0.7 on common phrasings the classifier «should» know
- Audit: «ашуланып» → AskWillingness 0.67; «Биллион саны» → Greeting 0.32

**Augmentation pipeline** (v6.5):

```
seed examples (clean Kazakh)
       ↓
apply N synthetic perturbations:
  - drop kazakh-specific chars (ә→а, қ→к, ғ→г, ө→о, ұ→у, ү→у, і→и, ң→н)
  - drop word-final consonants (көбейт → көбей)
  - swap soft/hard pairs (бөль ↔ бөл)
  - insert filler vowels (алты → алтыа)
  - merge adjacent tokens (Whisper occasionally merges)
       ↓
target: 500 examples per intent, ~30% augmented
       ↓
retrain → new ckpt at data/checkpoints/intent_classifier_v2
```

**Effect:** intent confidence on real production noise jumps from 0.3–0.7 to 0.8+. Wellness routing becomes more accurate. Math doesn't get mislabelled as Farewell.

### Move C — STT corpus acquisition (HIGHEST root impact, HIGHEST effort)

Current Kazakh STT model is Shirali (ISSAI/KSC, 335 hours). Industry-standard ASR needs 1000+ hours. We can **fine-tune** Shirali on additional in-domain Kazakh audio to drop the production WER without training from scratch.

**Acquisition targets**:

| Source | License | Hours estimate | Effort |
|---|---|---|---|
| KazNU public lectures (YouTube) | educational | 100+ | scrape + transcribe |
| Bilim TV broadcast archive | public-broadcast | 200+ | mirror + align |
| Parliament hearings | public-domain | 500+ | mirror + align |
| Public-domain Kazakh literature audiobooks | various | 100+ | mirror + align |

Total target: **1 000 hours** of Kazakh audio with transcripts (auto-aligned via existing v6.3 forced-aligner).

**Fine-tune** Shirali on this expanded corpus → new ggml model → drop into voice REPL. WER should drop from ~15–20% to ~5–10%.

### Move A — Data hygiene (low-risk, immediate)

DONE 2026-06-08 — 5.7G → 4.1G freed:
- Deleted `data/stt_models/_convert/venv` (757M)
- Deleted `data/stt_models/_convert/shirali/pytorch_model.bin` (922M, source for converted ggml)
- Deleted old `data/checkpoints/contextual_lm_v1-v5` (45M)

NOT deleted (production runtime depends on it):
- `data/tts_models/.venv` (1.0G) — Piper Python venv, referenced by `voice_repl_v6_3 --piper-venv` default.

## Sequencing recommendation

1. **Now → 1 week**: implement Move D (sentence-coherence layer). Calibrate against real audio. Ship as v6.5.0-alpha.
2. **Parallel, 2 weeks**: write the synthetic-augmentation pipeline (Move B). Retrain intent classifier. Ship as v6.5.0-beta with new checkpoint.
3. **Background, 4–6 weeks**: STT corpus acquisition script + ingestion pipeline. Fine-tune Shirali. Ship as v6.5.0 stable.
4. **Never**: continue patching operator vocabs / name DBs in the v6.4.x line. Bugs of that class are now design defects that need Move D to be the safety net.

## Versioning

- v6.4.0 line: **frozen** after rc12. Only patches for safety-critical regressions.
- v6.5.0: this arc. Lives on `experimental/v6_5_corpus_training` branch until Move D ships.
- v6.3.x stable: independent track. rc15 awaits live confirmation.

## Success criteria (predeclared)

- **Move D** ships when, in a 100-utterance audit, adam either answers correctly OR says «толық түсінбедім» — never gives a confidently-wrong answer.
- **Move B** ships when intent classifier max-class confidence ≥ 0.80 on 80%+ of audit utterances.
- **Move C** ships when WER on a held-out 1-hour Kazakh test set drops below 10%.

These criteria are publicly declared so we don't slip into a new patching cycle disguised as a different arc.

## Industry best practices we ARE adopting

User asked (2026-06-08): «почему сами, когда индустрия уже сделала?» The honest answer is — we DO adopt industry tools at the peripheral layers (Whisper.cpp / Piper / Burn). The dialog core stays deterministic by project design — see `project_deterministic_directive_confirmed` (2026-05-11, user reconfirmed). What we missed and now add:

### For Move C (STT corpus + retrain)

- **Compare alternative Kazakh STT models** before commiting to fine-tuning Shirali: download MMS-Kazakh (Meta, 1107-lang) and NeMo Conformer if Kazakh checkpoints exist. Run identical 1-hour Kazakh test set through all three; pick the lowest WER as the base.
- **Audio quality QA pipeline (NEW required gate)** — user feedback verbatim: «прежде чем скачивать аудио файлы, прослушай их с целью выяснения качества и пригодности».  
  Pipeline:
  1. Download 10 random samples per source.
  2. Automated metrics: SNR (reject < 15 dB), voice-activity ratio (reject < 60 %), per-utterance duration (1–30 s window), sample rate ≥ 16 kHz.
  3. Manual listen on 5 samples: reject if background music, multi-speaker overlap, heavy regional dialect (unless target), or any commercial / paywalled content.
  4. Whisper test on the 10 samples — does the output read as plausible Kazakh?
  5. If ≥ 30 % fail → discard the source entirely.
  6. Only at ≥ 70 % pass → schedule bulk acquisition.
- **SpecAugment** during fine-tuning — frequency + time masking for robustness. Standard NeMo / fairseq practice; ~5 % WER drop typical.

### For Move B (intent classifier retrain)

- **Sentence-transformers fallback** when our small classifier confidence < 0.5: run the input through a pre-trained multilingual encoder (LaBSE or `paraphrase-multilingual-MiniLM-L12-v2`), nearest-neighbour against the labeled training set, take the consensus.  This is a safety net for the long tail of intents the small classifier was never going to nail with 65 examples.
- **WER-style metric** for intent classification: track macro-F1 and per-intent precision/recall in CI on a held-out 200-utterance test set.

### For Move D (sentence coherence)

- **Calibrate the threshold against a real audio batch**, not a heuristic guess. Method: collect 100 utterances; manually label each «coherent» / «noise»; pick the threshold that maximises F1.
- **Standard signals** — perplexity from contextual_lm, max-class confidence from intent classifier, FST parse coverage. Industry would also add a length / OOV-word-ratio signal; consider for v6.6 if D leaves false negatives.

## Why we DON'T just route everything through a frontier LLM

The project's USP is BYTE-DETERMINISTIC dialog. A frontier LLM:

- Hallucinates biographies of nonexistent Kazakh people; we refuse with «нақты дерегім жоқ».
- Runs at LLM-class latency (~100 ms+); we run the production cascade at **13.6 ms p50** / 19.6 ms p95 on M2 Air (text-only — voice loop adds STT/TTS on top).
- Requires multi-GB weights + GPU; we run with **314 MB peak RSS, 0 % GPU, 0 network**.
- Has no transparency — we can't show an auditor WHY a specific answer was given.
- Treats Kazakh as < 0.01 % of training data; we treat it as 100 %.
- We are NOT a general LLM replacement — narrow deterministic Kazakh reasoning surface with 97 % on the curated blind-eval battery (rc18). Different system class, fair comparison is the same-pack eval, not raw latency.

For peripheral perception tasks (STT, TTS) we DO use the best open models. For the dialog kernel, the deterministic kernel IS the product. See `project_deterministic_directive_confirmed`, `project_engineering_framing`, `project_retrieval_not_neural_v2` in memory for the full rationale.
