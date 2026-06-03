# `synthesize_piper` — Phase 13 step 1 (2026-05-31)

User listening verdict (2026-05-31) after iterating on concatenative
PCM/morpheme banks:

> «Я прослушал все 9 файлов все отлично звучат…»

after switching from our pure-Rust concatenative approach to a
pre-trained neural TTS model. This directory documents the
end-to-end pipeline that produced the **committed reference
listening set** in `data/listening_tests/phase13_neural_piper/`.

The next step (Phase 13b) is to replace the Python `piper` CLI with
a pure-Rust ONNX inference adapter, so the whole runtime path stays
inside the workspace's no-C-dependency pledge.

## Why neural

Concatenative TTS (Phases 7, 12) reached a hard wall because the
underlying audio (kaz-tili.kz drill recordings) is recorded by
**multiple speakers across multiple sessions**. Concatenating
phoneme/morpheme PCM slices from different speakers produces audible
timbre / pitch / breath discontinuities at every join. The user
called this exactly:

> «Такое ощущение, что слово произносится, где каждую букву
>  озвучивали разные люди (мужчина, женщина, ребёнок и робот).»

A single-speaker neural TTS model — once trained — encodes one
consistent voice across the entire phoneme inventory, so
synthesised speech is internally coherent.

## Asset

- **Voice:** Piper TTS `kk_KZ-issai-high`
  - Trained from scratch on the ISSAI KazakhTTS2 corpus
    (271h / 5 speakers; https://github.com/IS2AI/Kazakh_TTS)
  - License: CC BY 4.0
  - Sample rate: 22 050 Hz mono
  - Inventory: 130 IPA + punctuation symbols
- **Files:**
  - `data/tts_models/kk_KZ-issai-high.onnx` — 128 MB, gitignored
    (regenerable from the canonical URL — see `.gitignore`).
  - `data/tts_models/kk_KZ-issai-high.onnx.json` — phoneme-ID map +
    inference hyper-parameters. Tracked.
  - `data/tts_models/kk_KZ-issai-high.MODEL_CARD` — provenance.
    Tracked.

## Build-time tools

The pipeline relies on two external tools at build time; both are
single-binary, well-maintained, and align with the existing precedent
for `ffmpeg` / `yt-dlp`:

- **eSpeak-NG 1.52.0** — `brew install espeak-ng`. Phonemises Kazakh
  Cyrillic → IPA (used by Piper internally).
- **Piper TTS 1.4.2** — installed via Python venv at
  `data/tts_models/.venv/`. Bundles ONNX Runtime for inference.

Reproduce the venv:

```bash
python3 -m venv data/tts_models/.venv
source data/tts_models/.venv/bin/activate
pip install piper-tts
```

## Per-word synthesis configuration

Empirically tuned during the 2026-05-31 listening session, the
default Piper config sometimes loses a phoneme on short utterances
(attention-decay artifact). Per-word overrides that produced the
committed reference WAVs:

| Word | Cyrillic input | length-scale | Notes |
|---|---|---|---|
| Сәлем. | `Сәлем.` | 1.1 | trim trailing silence ≥200 ms below -55 dB |
| Қазақстан. | `Қазақстан.` | 1.1 | same |
| Балалар. | `Балалар.` | 1.1 | same |
| Мен. | `Мен.` | 1.2 | same |
| Сен. | `Сен.` | 1.1 | same |
| Адам. | `Адам.` | 1.2 | stress falls on the first «а» (model artifact, accepted) |
| Бала. | `Бала.` | 1.4 | needs slower speech so the first «а» isn't compressed away |
| Бұл. | `Бұл.` | 1.3 | same — short word needs attention buffer |
| **Олар.** | `Олар.` | 1.0 (default) | **no trim, no slowdown — original config is the cleanest** |

The «no trim» override on `Олар` is the key learning: trimming with
`silenceremove` can eat the trailing tap of «р» and leave either a
click or a phantom «е» echo. When the un-trimmed output is already
clean, leave it alone.

## Synthesis command (per word)

```bash
source data/tts_models/.venv/bin/activate
echo "${capitalized_word}." | piper \
  --model data/tts_models/kk_KZ-issai-high.onnx \
  --length-scale ${scale} \
  --sentence-silence 0.0 \
  --output-file out.wav

# Optional trim (skip for Олар):
ffmpeg -y -i out.wav \
  -af "areverse,silenceremove=start_periods=1:start_silence=0.08:start_threshold=-55dB:start_duration=0.2,areverse" \
  out_trimmed.wav
```

## Known limitations

1. **Stress placement.** eSpeak's IPA for «Адам.» is `ɑdˈɑm` (stress
   on the second syllable, correct for native Kazakh), but the neural
   model renders prominence on the first vowel. This is baked into
   the model's training corpus and not fixable from the input side.
   Trying the alternative `kk_KZ-iseke-x_low` or `kk_KZ-raya-x_low`
   voices is a Phase 13b experiment.
2. **Short-utterance attention.** Words of 3-4 phonemes can lose
   their initial consonant when `length-scale = 1.0`. Workaround is
   the per-word table above. A `kk_KZ-iseke` ablation might handle
   short words differently.

## Phase 13b — pure-Rust path

The current build-time stack:

```
text → eSpeak-NG (C) → IPA → piper (Python, onnxruntime C++) → WAV
```

The Rust-native path that closes the no-C-runtime pledge:

```
text → adam-phoneme/cyrillic_to_phonemes_prayer_aware → IPA mapping →
       tract-onnx (pure Rust) → WAV
```

Mapping our 37 Kazakh phonemes onto Piper's 130-IPA inventory is a
small static table. The `tract-onnx` crate handles ONNX inference
without any C dependency. Phase 13b ships as a new
`adam-tts-neural` crate exposing `synthesise_neural(text) →
PcmSamples`, with the per-word `length-scale` overrides above
captured as a `TtsConfig::word_override(text, scale)` API.
