# Privacy & data-handling — adam v6.0.0-rc1 alpha

> **Read this before deploy.** Sign with the maintainer at kick-off.

## TL;DR

adam runs entirely on the partner's hardware. The maintainer
never has access to user inputs, dialog logs, or the feedback
file unless the partner explicitly shares them. There is **no
phone-home** under any circumstance.

## Network traffic (allowlist)

The only outbound network connections adam makes:

| When | Destination | Purpose | Opt-in |
|---|---|---|---|
| `Intent::AskWeather` AND `ADAM_WEATHER_CITY` (or env LAT/LON) set | `api.open-meteo.com` | Fetch current weather | Yes — partner sets the env var |
| `--tts` build flag enabled | (none — local `say` binary) | TTS | N/A — local |
| `--voice-input` build flag enabled | (none — local `whisper-cli` binary) | STT | N/A — local |

**Everything else stays local.** No telemetry, no crash reports
sent, no usage stats, no model-update checks, no analytics. We
checked: `lsof -i -P -n | grep adam` should show only the
weather call when AskWeather fires and `ADAM_WEATHER_*` is set.

If the partner blocks `api.open-meteo.com` at the network layer,
adam silently falls back to the honest refusal template
(«менде терезе жоқ») — no data is exfiltrated. This is the
**air-gap default**.

## Data adam stores on disk

| File / dir | What | When written | Cleanup |
|---|---|---|---|
| `data/*` | Curated knowledge graph + corpus. Shipped in git, immutable at runtime. | Never written at runtime | N/A |
| `data/retrieval/morpheme_index.json` | Retrieval index. Immutable. | Built once via `cargo run -p adam-retrieval --bin build_morpheme_index`; never updated by `adam_chat`. | N/A |
| `data/checkpoints/*` | L5.5 neural checkpoints. | Only when partner runs `poc_kazakh_train` locally. Never auto-downloaded. | `rm -rf data/checkpoints/` |
| stdout / stderr | Dialog log. | Per-session, only if partner pipes through `tee`. | `rm <log-file>` |

**adam_chat does not write to `~/.cache`, `~/Library`, or any
user-directory location.** All state lives inside the repo clone.

## Data the maintainer receives

Only what the partner deliberately sends:

1. **The feedback spreadsheet** (`feedback_form.md` format). The
   partner anonymises tester identities before sharing (e.g.
   "Aigerim" → "T01"). The maintainer receives the CSV by email
   or by uploading to the partner's preferred file-share.

2. **Optional**: GitHub issues filed by the partner against
   `qazaq-ai/adam`. The partner controls the issue content;
   nothing is auto-filed by adam.

The maintainer **does not** receive:
- Raw stdout logs (unless partner explicitly shares).
- Voice recordings (Whisper writes a temp WAV that adam_chat
  deletes after transcription — see `crates/adam-voice/src/stt.rs`).
- User identities, names, locations, occupations — these stay in
  the partner's spreadsheet at the partner's discretion.

## What we ask the partner to keep on file

For the alpha duration:

1. Per-tester consent that their dialog input may be reviewed by
   the partner's own technical lead. (Internal to the partner;
   not seen by the maintainer.)
2. A reasonable retention policy for the feedback spreadsheet
   (we suggest: delete after the alpha closes + 30 days; keep
   only the aggregated weekly summary).

We do not ask for personal data of the testers. Tester IDs in the
spreadsheet are pseudonyms.

## GDPR / Kazakhstan personal-data law

Kazakhstan's «Личных данных» law (closely modelled on GDPR)
applies. adam is a tool the partner deploys; the partner is the
data controller. The maintainer is not a processor — receives no
personal data.

If a tester later requests deletion of their feedback rows, the
partner deletes those rows from the spreadsheet. The maintainer
has nothing to delete because nothing flowed upstream.

## Signed at kick-off

Both maintainer and partner sign-off on this document at alpha
kick-off (electronic signature OK). A copy stays with each side.

---

**Maintainer**: Daulet Baimurza · Qazna Technologies LLP ·
`baimurza.daulet@gmail.com` · Date: ________________

**Partner organisation**: ________________ ·
Technical lead: ________________ · Date: ________________
