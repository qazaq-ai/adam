# Deploy adam v6.0.0-rc1 on a single Mac (M-series)

**Audience.** Technical lead at the alpha-partner organisation.
**Time budget.** ≈ 30 minutes including first compile.
**Hardware.** MacBook Air / Pro with Apple Silicon M1/M2/M3, ≥ 8 GB RAM, ≥ 5 GB free disk.
**OS.** macOS 14+ (Sonoma) or 15+ (Sequoia). Linux works too but path adjustments are needed; not covered here.

## 1. Prerequisites (10 min)

```bash
# Install Rust if not present:
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install Homebrew if not present:
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Voice stack (optional; required only for the voice REPL):
brew install whisper-cpp

# Download a Whisper model for Kazakh (≈ 1.6 GB):
mkdir -p ~/whisper-models
cd ~/whisper-models
curl -L -O https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin
cd -
```

## 2. Clone + build (15 min)

```bash
git clone https://github.com/qazaq-ai/adam.git
cd adam
git checkout experimental/agglutinative-neural

# Text REPL (no voice, no neural preview):
cargo build --release -p adam-dialog --bin adam_chat

# Voice REPL:
cargo build --release -p adam-dialog --bin adam_chat --features voice

# Voice + L5.5 neural preview:
cargo build --release -p adam-dialog --bin adam_chat --features voice,neural
```

First build pulls ≈ 700 dependencies (~10 min on 100 Mbit Wi-Fi).
Subsequent builds are < 30 s.

## 3. Smoke-test (text REPL) (3 min)

```bash
echo 'Сәлеметсіз бе
Менің атым Дәулет
Қазақстанның астанасы қандай?
2 + 2 қанша?
Сау бол' | ./target/release/adam_chat
```

You should see five reasonable Kazakh-language responses. If the
first line shows only a Cargo error or a missing-file message,
file an issue against `experimental/agglutinative-neural`.

## 4. Smoke-test (voice REPL) (5 min)

```bash
export ADAM_TZ_OFFSET_HOURS=5  # Almaty / Astana / Kostanay time-zone

./target/release/adam_chat \
  --voice-input \
  --tts \
  --whisper-bin /opt/homebrew/bin/whisper-cli \
  --whisper-model ~/whisper-models/ggml-large-v3-turbo.bin \
  --whisper-language kk
```

Press Enter, say «Қазір сағат неші?», pause 1.5 seconds. adam should
speak the current time in Kazakh through macOS Aru voice.

## 5. Optional: enable weather (1 min)

```bash
# Set your city — adam fetches Open-Meteo on demand for AskWeather turns.
export ADAM_WEATHER_CITY=Қостанай

# Restart adam_chat. The «Бүгін ауа райы қандай?» turn now returns
# the live forecast.
```

If you prefer not to share location data with Open-Meteo, leave
`ADAM_WEATHER_CITY` unset and adam will return the honest refusal
template («менде терезе жоқ — қалаңызды айтсаңыз…»).

## 6. Optional: L5.5 neural preview (15 min, one-time training)

```bash
# Train a smoke checkpoint locally (≈ 10 min on M2 CPU):
POC_EPOCHS=1 POC_BATCH=8 POC_CHECKPOINT_DIR=data/checkpoints/smoke \
  cargo run --release -p adam-agg-model --bin poc_kazakh_train

# Or a full checkpoint (≈ 95 min on M2 CPU):
POC_REAL_PACK=data/curated/real_corpus_pairs.json POC_ALPHA=0.5 \
  cargo run --release -p adam-agg-model --bin poc_kazakh_train
# → writes data/checkpoints/poc_kazakh/v6_<UTC-timestamp>/

# Launch adam_chat with the checkpoint:
./target/release/adam_chat \
  --neural-model data/checkpoints/poc_kazakh/v6_<your-timestamp>/
```

Then inside the REPL: `/neural бала` (or any Kazakh root) should
print the morpheme trail + the L6 verifier verdict.

## 7. Daily operation

The REPL exits on Ctrl-D. To restart, re-run the launch command.
There is no background service / daemon — adam is a foreground
process by design.

For long-running evaluation we recommend running adam in `tmux`:

```bash
tmux new -s adam
# ... launch adam_chat inside the tmux session ...
# detach: Ctrl-B then D
# reattach later: tmux attach -t adam
```

## 8. Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| `cargo: command not found` | Rust not on PATH | `source $HOME/.cargo/env` |
| Build fails on `burn` | Old Rust toolchain | `rustup update stable` |
| Voice REPL refuses to launch | `--features voice` missing | Rebuild with `cargo build --release -p adam-dialog --bin adam_chat --features voice` |
| `whisper-cli` not found | Brew install missing | `brew install whisper-cpp` |
| TTS silent on macOS | `say` audio routing | `osascript -e 'set volume output volume 50'` then retest |
| Weather returns refusal even with `ADAM_WEATHER_CITY` set | Curl unavailable / firewall | `which curl` should print `/usr/bin/curl`; if behind a corporate firewall, weather will silently fall back to the refusal template — this is intentional, not a bug |

## 9. Where to file feedback

- GitHub issues (preferred): https://github.com/qazaq-ai/adam/issues
- Email to maintainer: `baimurza.daulet@gmail.com`
- For the 2-week alpha protocol: use the
  [`feedback_form.md`](feedback_form.md) spreadsheet template.
