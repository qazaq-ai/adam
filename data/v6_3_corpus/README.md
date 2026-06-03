# `data/v6_3_corpus/` — Kazakh audio corpus for the v6.3 phoneme bank

Status: **Phase 2a started 2026-05-26.** Directory exists; manifest
schema is defined; first audio sources are pending acquisition.

This directory holds the **audio corpus** from which Layer 0b
(phoneme MFCC templates + diphone bank) is built by the Phase 2c
forced-alignment pass and Phase 2d extraction pass.

The corpus is **not gitignored** — but the actual audio files are
not committed; only their `MANIFEST.jsonl` entries are. Audio
storage layout (see § "Layout" below) is local-only.

## Authoritative reference

- Design doc — [`docs/v6_3_phonemic_foundation.md`](../../docs/v6_3_phonemic_foundation.md) §6.
- Corpus directive (memory) — `project_v6_3_corpus_directive`
  (research-fair-use posture; collect all available Kazakh audio,
  licensing handled post-result).
- Source priorities — design doc §6.2: news anchors > prose >
  tales > poetry.

## Manifest schema

`MANIFEST.jsonl` is the authoritative record of every audio
source the corpus contains. One JSON object per line. Schema:

```jsonc
{
  // Stable identifier — kebab-case slug, unique within the
  // corpus. Used as the local filename prefix.
  "id": "common-voice-kk-validated-2026-05",

  // Where the audio was obtained.
  "source_url": "https://commonvoice.mozilla.org/kk/datasets",

  // Local-disk path RELATIVE to this directory, when downloaded.
  // Null until the audio is actually present locally.
  "local_path": null,

  // ISO-8601 date the manifest entry was created.
  "collected_at": "2026-05-26",

  // Audio file format ("mp3" / "wav" / "flac" / "opus" / ...).
  "format": "mp3",

  // Total duration in seconds (post-acquisition). Null until
  // acquired and measured.
  "duration_s": null,

  // Number of distinct speakers in the recording.
  "speaker_count": 1,

  // "male" / "female" / "mixed" / "unknown".
  "speaker_gender": "unknown",

  // Whether a transcript is bundled / available.
  "transcript_available": true,
  "transcript_url": "https://commonvoice.mozilla.org/kk/datasets",

  // Provenance category, drawn from a closed vocabulary:
  //   "common-voice"     | "ksc" | "kazneb"
  //   "youtube"          | "radio" | "archive-org"
  //   "public-domain"    | "self-recorded" | "other"
  "source_class": "common-voice",

  // Licensing posture:
  //   "cc0"             — explicit public domain
  //   "academic"        — KSC-style open academic
  //   "public-domain"   — copyright expired (Abai, Shakarim, ...)
  //   "youtube-channel" — case-by-case per uploader
  //   "unknown"         — uncategorised, default
  "licence_status": "cc0",

  // Becomes true after the source passes Phase 2c forced-alignment
  // confidence filter (per-phoneme alignment confidence ≥ 0.7).
  // Only "used" entries contribute to the final phoneme bank.
  "used_in_bank": false,

  // Free-text. Optional. For Whisper / MFA / curation notes.
  "notes": ""
}
```

Fields are validated by the Phase 2 pipeline; unknown fields are
allowed and preserved (forward-compat).

## Layout

```
data/v6_3_corpus/
├── README.md                  ← this file
├── MANIFEST.jsonl             ← authoritative record
├── audio/                     ← downloaded audio (gitignored)
│   ├── common-voice/
│   ├── ksc/
│   ├── kazneb/
│   ├── youtube/
│   ├── radio/
│   └── public-domain/
└── transcripts/               ← paired transcript files (gitignored
    │                            if large; otherwise kept)
    └── ...
```

Audio and large transcript files are kept locally only. The
manifest plus the derived bank (Phase 2d output, `~5-10 MB`) is
what ships in git.

## Source-acquisition order (Phase 2a working plan)

1. **Mozilla Common Voice Kazakh** — CC0; pre-aligned audio +
   transcript; lowest licensing friction.
   <https://commonvoice.mozilla.org/kk/datasets>
2. **ISSAI Kazakh Speech Corpus / KSC2** — open academic licence;
   ~1200 h; highest single-source volume.
   <https://issai.nu.edu.kz/kz-speech-corpus/>
3. **kazneb.kz audio books** — Kazakh national digital library;
   variable provenance per item.
4. **YouTube** «қазақша аудиокітап» / «қазақ ертегілері аудио»
   selected channels with consistent narrators.
5. **Radio Шалқар** / **Qazaq Radiosy** archives.
6. **archive.org** Kazakh-tagged recordings.
7. **Public-domain literature** — Abai, Şäkärim, Mahjan,
   Altynsarin read-aloud editions.

Each acquired source is appended to `MANIFEST.jsonl` as a new
line; `used_in_bank` flips to `true` after Phase 2c validates
alignment quality.

## Storage discipline

- `audio/` and `transcripts/` are excluded from git (added to
  `.gitignore` at the repo level). Only the manifest entries
  describing them are committed.
- This file (`README.md`) and `MANIFEST.jsonl` are version-
  controlled.
- The derived phoneme bank (Phase 2e output) is **its own
  artefact** in `data/v6_3_phoneme_bank/`, not nested under
  this corpus directory.

## Status / next actions

- ✅ Directory exists.
- ✅ Manifest schema documented.
- ⏳ `MANIFEST.jsonl` — to be populated as sources are acquired.
- ⏳ `audio/` — to be created when first source is downloaded.
- ⏳ Phase 2b text normalisation script — design pending.
- ⏳ Phase 2c MFA invocation script — pending.
