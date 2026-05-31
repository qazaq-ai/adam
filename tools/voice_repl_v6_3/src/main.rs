// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! **Voice REPL v6.3** — the full Whisper-free Kazakh voice
//! loop, standalone.
//!
//! ```text
//!   microphone (cpal via adam-audio::record)
//!     ↓ PCM mono 16 kHz
//!     ↓ adam-audio::mfcc
//!     ↓ adam-stt-phoneme::recognise_word + rescore
//!   Vec<Phoneme>
//!     ↓ adam-phoneme::cyrillic::phonemes_to_cyrillic
//!   Cyrillic transcript
//!     ↓ adam-tts-phoneme::synthesise_with_bank
//!     ↓ adam-audio::play
//!   speaker
//! ```
//!
//! ## Usage
//!
//! ```sh
//! # Record 3 s, recognise, print, optionally speak back:
//! cargo run --release -p adam-voice-repl-v6-3 -- --duration 3 --speak
//!
//! # Loop mode (press Enter to record, ^C to quit):
//! cargo run --release -p adam-voice-repl-v6-3 -- --loop
//! ```
//!
//! ## Banks
//!
//! Loads `data/v6_3_phoneme_bank/templates.bin` (MFCC for STT)
//! and `pcm_templates.bin` (PCM for TTS) when present. Both
//! are merged with synth fallback covering uncovered phonemes.

use adam_audio::play::play_blocking;
use adam_audio::record::{RecordConfig, record_fixed_duration, record_until_silence};
use adam_audio::wav::write_wav;
use adam_dialog::Conversation;
use adam_dialog::templates::TemplateRepository;
use adam_kernel_fst::lexicon::LexiconV1;
use adam_phoneme::cyrillic::phonemes_to_cyrillic;
use adam_retrieval::MorphemeIndex;
use adam_stt_phoneme::{PhonemeBank, WordConfig, recognise_word, rescore};
use adam_tts_phoneme::{PcmBank, TtsConfig, synthesise_with_bank};
use clap::Parser;
use std::path::PathBuf;
use std::time::Duration;

// Phase 15f (2026-05-31) — KB / retrieval / reasoning artefact
// paths. Same constants as `adam_chat.rs` so the two REPLs read
// the same artefacts and behave identically on factual queries.
const RETRIEVAL_INDEX_PATH: &str = "data/retrieval/morpheme_index.json";
const FACTS_PATH: &str = "data/retrieval/facts.json";
const DERIVED_FACTS_PATH: &str = "data/retrieval/derived_facts.json";
const WORLD_CORE_DIR: &str = "data/world_core";

#[derive(Debug, Parser)]
#[command(
    name = "voice_repl_v6_3",
    about = "End-to-end v6.3 voice loop: mic → phoneme STT → Cyrillic → phoneme TTS → speaker.",
    version
)]
struct Args {
    /// Fixed recording duration in seconds. When set, --vad is
    /// ignored.
    #[arg(long)]
    duration: Option<u64>,
    /// Use VAD auto-stop (recording stops on 1.5 s of silence).
    /// Default 30 s hard cap.
    #[arg(long)]
    vad: bool,
    /// Loop mode — press Enter to start each recording, ^C to
    /// quit.
    #[arg(long, name = "loop")]
    loop_mode: bool,
    /// Synthesise the recognised phoneme stream back through
    /// the TTS bank and play it.
    #[arg(long)]
    speak: bool,
    /// Optional: save each recording to a WAV file (debugging).
    #[arg(long)]
    save_wav: Option<PathBuf>,
    /// Bank directory.
    #[arg(long, default_value = "data/v6_3_phoneme_bank")]
    bank_dir: PathBuf,
    /// TTS backend. `concat` = the in-tree concatenative
    /// pipeline (PcmBank + parametric fallback, Phase 7).
    /// `piper` = subprocess to the Piper neural TTS CLI using
    /// the ISSAI kk_KZ-issai-high voice from Phase 13.
    /// Piper produces single-speaker, sentence-level, natural-
    /// sounding output and is the recommended default once the
    /// model + venv are present.
    #[arg(long, default_value = "piper")]
    tts_backend: String,
    /// Piper voice .onnx path (used only when --tts-backend=piper).
    #[arg(long, default_value = "data/tts_models/kk_KZ-issai-high.onnx")]
    piper_model: PathBuf,
    /// Path to the Python venv where `piper-tts` is installed
    /// (built per tools/synthesize_piper/README.md).
    #[arg(long, default_value = "data/tts_models/.venv")]
    piper_venv: PathBuf,
    /// STT backend. `dtw` = the in-tree DTW phoneme recogniser
    /// (Phase 13 «human bank v4», FLEURS PER ≈ 76.6 %).
    /// `whisper` = subprocess to whisper.cpp's `whisper-cli`
    /// CLI using a multilingual ggml model — dramatically
    /// better word-level accuracy on natural Kazakh speech.
    /// Default `whisper` once the model is present; auto-falls
    /// back to `dtw` if model/binary missing.
    #[arg(long, default_value = "whisper")]
    stt_backend: String,
    /// Whisper ggml model path (used only when --stt-backend=whisper).
    /// Default: ggml-medium.bin (~1.5 GB, multilingual incl. Kazakh).
    /// Smaller alternative: ggml-small.bin (488 MB) — works but mangles
    /// the distinctive Kazakh letters (Қ/Ғ/Ң/Ө/Ұ/Ү/Һ/І/Ә) more often,
    /// which trips the dialog engine's intent classifier.
    #[arg(long, default_value = "data/stt_models/ggml-medium.bin")]
    whisper_model: PathBuf,
    /// Initial prompt fed to whisper-cli to bias decoding toward
    /// Kazakh-specific phonotactics. Multilingual Whisper otherwise
    /// substitutes Қ→К, Ғ→Г, Ң→Н, Ө→О, Ұ→У, Ү→У, Һ→Х, І→И, Ә→Е.
    /// Providing canonical Kazakh phrases in the prompt re-anchors
    /// the distribution toward those letters (validated 2026-05-31:
    /// «Қалыңыз», «отанымыз», «қазақша сөйл» all recover when the
    /// prompt is set; without it they get butchered).
    #[arg(
        long,
        default_value = "Сәлеметсіз бе. Қалыңыз қалай? Менің атым Даулет. \
            Сен кімсің? Қазір сағат неше? Бүгін қай күн? Танысайық. \
            Алдымен танысайық. Бүгін жексенбі. Қазақша сөйлесейік."
    )]
    whisper_prompt: String,
    /// Dialog mode. `echo` (default for now) = TTS re-speaks the
    /// STT output (Phase 13/15 loopback validation). `respond` =
    /// route the recognised text through `adam_dialog::Conversation`
    /// so the system answers instead of echoing.
    #[arg(long, default_value = "respond")]
    mode: String,
    /// Per-session RNG seed for dialog response selection.
    #[arg(long, default_value_t = 42)]
    seed: u64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Load banks (real templates + synth fallback hybrid).
    let mfcc_bank = load_mfcc_bank(&args.bank_dir, 16_000)?;
    let pcm_bank = load_pcm_bank(&args.bank_dir).ok();
    println!(
        "[voice-repl] MFCC bank covers {} phonemes; PCM bank {}",
        mfcc_bank.len(),
        pcm_bank
            .as_ref()
            .map(|b| format!("covers {} phonemes", b.len()))
            .unwrap_or_else(|| "absent (synth-only TTS)".into()),
    );

    // Phase 16: load dialog engine if --mode=respond. Cheap when off
    // (one Option pair); pricey when on (LexiconV1 + TemplateRepository
    // load ≈ 1 second on a cold filesystem), so we do it once at
    // startup and reuse across turns.
    //
    // Phase 15f (2026-05-31): also load the KB / retrieval / reasoning
    // / world-core artefacts in the same startup phase so factual
    // queries («Қазақстан туралы», «Абай кім», «Алматы туралы»)
    // route through `adam-retrieval`'s morpheme-index O(1) lookup
    // and `adam-reasoning`'s curated fact graph instead of falling
    // through to «Бәлкім, X туралы айтасыз ба».
    //
    // Each loader fails silently — missing files just disable that
    // capability, the REPL still runs.
    let (dialog_state, mut conversation): (
        Option<(LexiconV1, TemplateRepository)>,
        Option<Conversation>,
    ) = if args.mode == "respond" {
        match (
            LexiconV1::load_default(),
            TemplateRepository::load_default(),
        ) {
            (Ok(lex), Ok(repo)) => {
                println!(
                    "[voice-repl] dialog engine: lexicon + {} template families loaded",
                    repo.len()
                );
                let mut conv = Conversation::new();

                // 1. MorphemeIndex from data/retrieval/morpheme_index.json
                if let Some(idx) = load_retrieval_index() {
                    println!(
                        "[voice-repl] retrieval: {} morphemes / {} postings indexed",
                        idx.unique_morphemes, idx.total_postings
                    );
                    conv = conv.with_morpheme_index(idx);
                } else {
                    eprintln!(
                        "[voice-repl] retrieval: {} not found — factual queries deflect",
                        RETRIEVAL_INDEX_PATH
                    );
                }

                // 2. Reasoning chains (facts + derived_facts JSON).
                let (extracted, derived) = load_reasoning_chains();
                if !extracted.is_empty() || !derived.is_empty() {
                    println!(
                        "[voice-repl] reasoning: {} facts + {} derived loaded",
                        extracted.len(),
                        derived.len()
                    );
                    conv = conv.with_reasoning_chains(extracted, derived);
                }

                // 3. DomainIndex from data/world_core/*.jsonl — enables
                //    current-domain inference (which jsonl pack a query
                //    most likely belongs to).
                let domain_idx = match adam_reasoning::world_core::load_world_core_dir(
                    std::path::Path::new(WORLD_CORE_DIR),
                ) {
                    Ok(report) => {
                        // `load_world_core_dir` returns `Vec<(WorldCoreEntry, PathBuf)>`
                        // — DomainIndex::build wants `&[WorldCoreEntry]`, so we
                        // strip the provenance paths here.
                        let entries: Vec<_> = report.entries.into_iter().map(|(e, _)| e).collect();
                        let idx = adam_dialog::DomainIndex::build(&entries);
                        println!(
                            "[voice-repl] world_core: {} domains / {} entries indexed",
                            idx.len(),
                            entries.len()
                        );
                        idx
                    }
                    Err(e) => {
                        eprintln!(
                            "[voice-repl] world_core: load failed ({e}); domain inference disabled"
                        );
                        adam_dialog::DomainIndex::empty()
                    }
                };
                conv = conv.with_domain_index(domain_idx);

                (Some((lex, repo)), Some(conv))
            }
            (lex_res, repo_res) => {
                if let Err(e) = lex_res {
                    eprintln!(
                        "[voice-repl] dialog: lexicon load failed ({e}) — degrading to echo mode"
                    );
                }
                if let Err(e) = repo_res {
                    eprintln!(
                        "[voice-repl] dialog: template repo load failed ({e}) — degrading to echo mode"
                    );
                }
                (None, None)
            }
        }
    } else {
        (None, None)
    };

    let single = !args.loop_mode;
    loop {
        if args.loop_mode {
            println!("[voice-repl] press Enter to record, ^C to quit");
            let mut buf = String::new();
            if std::io::stdin().read_line(&mut buf).is_err() || buf.is_empty() {
                break;
            }
        }

        // **Phase 15f.5 (2026-05-31)** — VAD is now the default.
        // Earlier phases used a fixed N-second cap (3 → 6 → 4),
        // which guillotined long sentences mid-syllable. The right
        // shape — the one we had pre-v6.3 — is: record while the
        // speaker is speaking, stop on a 1.5 s silence trail.
        // `--duration N` still forces a fixed cap; `--vad` is a
        // no-op now that VAD is on by default but kept for muscle
        // memory.
        let pcm = if let Some(d) = args.duration {
            println!("[voice-repl] recording {d} s (fixed)...");
            record_fixed_duration(Duration::from_secs(d))?
        } else {
            println!("[voice-repl] recording (auto-stop on 1.5 s of silence, 30 s max)...");
            record_until_silence(RecordConfig::default())?
        };
        println!(
            "[voice-repl] captured {:.2} s at {} Hz",
            pcm.duration_s(),
            pcm.sample_rate
        );

        if let Some(path) = &args.save_wav {
            write_wav(path, &pcm)?;
            println!("[voice-repl] saved {}", path.display());
        }

        // STT: recognise. Whisper backend takes the WAV directly and
        // returns Cyrillic text; DTW backend produces a phoneme
        // stream that's rendered via phonemes_to_cyrillic.
        // `rescored` is also needed below for the concat-TTS fallback
        // path, so we always run the DTW recogniser for that pathway
        // and use whisper's Cyrillic when --stt-backend=whisper.
        let raw = recognise_word(
            &pcm.data,
            pcm.sample_rate,
            &mfcc_bank,
            &WordConfig::default(),
        );
        let rescored = rescore(&raw);
        let dtw_cyrillic = phonemes_to_cyrillic(&rescored);

        let user_text = match args.stt_backend.as_str() {
            "whisper" => {
                match transcribe_via_whisper(&pcm, &args.whisper_model, &args.whisper_prompt) {
                    Ok(text) => {
                        println!("[voice-repl] you said: «{text}»");
                        text
                    }
                    Err(e) => {
                        eprintln!("[voice-repl] whisper backend failed: {e} — falling back to DTW",);
                        println!("[voice-repl] dtw phonemes (raw):       {raw:?}");
                        println!("[voice-repl] dtw phonemes (rescored):  {rescored:?}");
                        println!("[voice-repl] you said (dtw):           «{dtw_cyrillic}»");
                        dtw_cyrillic.clone()
                    }
                }
            }
            _ => {
                println!("[voice-repl] dtw phonemes (raw):       {raw:?}");
                println!("[voice-repl] dtw phonemes (rescored):  {rescored:?}");
                println!("[voice-repl] you said (dtw):           «{dtw_cyrillic}»");
                dtw_cyrillic.clone()
            }
        };

        // Phase 16: route through dialog engine when --mode=respond,
        // else echo the user's own text back (Phase 13/15 loopback).
        // Phase 15e (2026-05-31): before dispatch, fuzzy-normalise
        // STT noise — substitute K→Қ, Г→Ғ, Н→Ң, И→Й, И→І etc. when
        // the input token is a near-neighbour of a canonical
        // intent-trigger word. Built on adam_dialog::kazakh_fuzzy's
        // phonetic-aware edit distance (PHONETIC_PAIRS table).
        let normalised = if args.mode == "respond" {
            fuzzy_normalise(&user_text)
        } else {
            user_text.clone()
        };
        if normalised != user_text {
            println!("[voice-repl] fuzzy → «{normalised}»");
        }

        // Phase 17 (2026-05-31): voice-derived gender hint. Same
        // pattern as `adam-dialog/src/bin/adam_chat.rs:1045+`:
        // run YIN F0 over the recorded segment, classify into
        // male/female/child, write it (and a stability lock) into
        // the Conversation session so any greeting / vocative
        // template that interpolates the addressee uses the
        // correct Kazakh honorific («Ағай» / «Апай» / «Балам»).
        //
        // The lock prevents per-turn flapping when a speaker's F0
        // straddles the male/female boundary (~165 Hz). After two
        // consecutive estimates pick the same label, the session
        // becomes immutable for the rest of the run.
        if let Some(conv) = conversation.as_mut() {
            let i16_samples: Vec<i16> = pcm
                .data
                .iter()
                .map(|s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
                .collect();
            if let Some(g) = adam_voice::pitch::estimate_pitch_hz(&i16_samples, pcm.sample_rate)
                .and_then(adam_voice::pitch::classify_gender)
            {
                let label = match g {
                    adam_voice::pitch::PitchGender::Male => "male",
                    adam_voice::pitch::PitchGender::Female => "female",
                    adam_voice::pitch::PitchGender::Child => "child",
                };
                let locked = conv.session.get("voice_gender_locked").is_some();
                if !locked {
                    let counter_key = format!("voice_gender_count_{label}");
                    let prior_count: u32 = conv
                        .session
                        .get(&counter_key)
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                    let new_count = prior_count + 1;
                    conv.session.insert(counter_key, new_count.to_string());
                    conv.session
                        .insert("voice_gender_hint".into(), label.to_string());
                    if new_count >= 2 {
                        conv.session
                            .insert("voice_gender_locked".into(), label.to_string());
                        println!("[voice-repl] pitch-gender locked = {label}");
                    } else {
                        println!("[voice-repl] pitch-gender hint = {label} ({new_count}/2)");
                    }
                }
            }
        }

        let cyrillic = match (&mut conversation, dialog_state.as_ref(), args.mode.as_str()) {
            (Some(conv), Some((lex, repo)), "respond") => {
                let reply = conv.turn(&normalised, lex, repo, args.seed);
                println!("[voice-repl] adam → «{reply}»");
                reply
            }
            _ => user_text,
        };

        // TTS playback. Phase 15f.1 (2026-05-31): in --mode=respond the
        // REPL is a voice dialog, not a STT tester — a silent reply
        // makes no sense. Force speak ON whenever we're responding.
        // `--speak` still works for --mode=echo (loopback debugging).
        let should_speak = args.speak || args.mode == "respond";
        if should_speak {
            let tts_out = match args.tts_backend.as_str() {
                "piper" => {
                    match synthesise_via_piper(&cyrillic, &args.piper_model, &args.piper_venv) {
                        Ok(pcm) => {
                            println!(
                                "[voice-repl] piper synthesised {:.2} s @ {} Hz, playing back",
                                pcm.duration_s(),
                                pcm.sample_rate,
                            );
                            pcm
                        }
                        Err(e) => {
                            eprintln!(
                                "[voice-repl] piper backend failed: {e} — falling back to concat",
                            );
                            let pcm = synthesise_with_bank(
                                &rescored,
                                pcm_bank.as_ref(),
                                &TtsConfig::default(),
                            );
                            println!(
                                "[voice-repl] concat synthesised {:.2} s, playing back",
                                pcm.duration_s(),
                            );
                            pcm
                        }
                    }
                }
                _ => {
                    let pcm =
                        synthesise_with_bank(&rescored, pcm_bank.as_ref(), &TtsConfig::default());
                    println!(
                        "[voice-repl] concat synthesised {:.2} s, playing back",
                        pcm.duration_s(),
                    );
                    pcm
                }
            };
            play_blocking(&tts_out)?;
        }

        if single {
            break;
        }
        println!();
    }

    Ok(())
}

fn load_mfcc_bank(bank_dir: &std::path::Path, sample_rate: u32) -> std::io::Result<PhonemeBank> {
    let path = bank_dir.join("templates.bin");
    let synth = PhonemeBank::synthetic(sample_rate);
    if path.exists() {
        match PhonemeBank::load_from_file(&path) {
            Ok(real) => Ok(real.merged_with_fallback(&synth)),
            Err(e) => {
                eprintln!(
                    "[voice-repl] WARN: failed to load {}: {} — using synth-only bank",
                    path.display(),
                    e,
                );
                Ok(synth)
            }
        }
    } else {
        eprintln!(
            "[voice-repl] note: no real MFCC bank at {} — using synth-only",
            path.display()
        );
        Ok(synth)
    }
}

fn load_pcm_bank(bank_dir: &std::path::Path) -> Result<PcmBank, Box<dyn std::error::Error>> {
    let path = bank_dir.join("pcm_templates.bin");
    Ok(PcmBank::load_from_file(path)?)
}

/// Phase 13 (2026-05-31) Piper neural TTS backend.
///
/// Shells out to the Piper CLI installed in the Python venv at
/// `venv_path`. Input is the recognised Cyrillic text with the
/// canonical formatting we tuned during the listening session:
/// **capitalise first letter + append "."** so the model gets a
/// proper sentence shape (otherwise short utterances lose their
/// initial consonant). Output is a 22 050 Hz mono WAV which
/// `play_blocking` accepts directly.
///
/// This is the «integration bridge» implementation. Phase 13b
/// will replace it with a pure-Rust `tract-onnx` adapter that
/// runs the same model in-process, eliminating the Python +
/// onnxruntime C++ dependencies at runtime.
fn synthesise_via_piper(
    cyrillic_text: &str,
    model_path: &std::path::Path,
    venv_path: &std::path::Path,
) -> Result<adam_audio::PcmSamples, Box<dyn std::error::Error>> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    if !model_path.exists() {
        return Err(format!(
            "piper model not found at {} — fetch via the URL in .gitignore",
            model_path.display()
        )
        .into());
    }
    let piper_bin = venv_path.join("bin/piper");
    if !piper_bin.exists() {
        return Err(format!(
            "piper CLI not found at {} — set up the venv per tools/synthesize_piper/README.md",
            piper_bin.display()
        )
        .into());
    }

    // Canonical sentence shape: capitalise first char + trailing period.
    let trimmed = cyrillic_text.trim();
    if trimmed.is_empty() {
        return Err("piper backend: empty input text".into());
    }
    let mut chars = trimmed.chars();
    let head = chars.next().unwrap();
    let head_upper: String = head.to_uppercase().collect();
    let cap = format!("{head_upper}{}", chars.as_str());
    let needs_period = !cap.ends_with(['.', '!', '?']);
    let prompt = if needs_period { format!("{cap}.") } else { cap };

    let tmp_dir = std::env::temp_dir();
    let pid = std::process::id();
    let tmp_out = tmp_dir.join(format!("voice_repl_piper_{pid}.wav"));

    let mut child = Command::new(&piper_bin)
        .arg("--model")
        .arg(model_path)
        .arg("--length-scale")
        .arg("1.0")
        .arg("--sentence-silence")
        .arg("0.2")
        .arg("--output-file")
        .arg(&tmp_out)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(prompt.as_bytes())?;
    }
    let output = child.wait_with_output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "piper exited with {:?}: {}",
            output.status.code(),
            stderr.trim()
        )
        .into());
    }
    if !tmp_out.exists() {
        return Err(format!("piper produced no output at {}", tmp_out.display()).into());
    }
    let raw_pcm = adam_audio::wav::read_wav(&tmp_out)?;
    let _ = std::fs::remove_file(&tmp_out);

    // adam-audio's `play_blocking` does NOT resample — it feeds
    // raw samples at the device's preferred rate. On macOS the
    // default output device is typically 48 kHz; Piper produces
    // 22 050 Hz; without resampling the 22 050 Hz buffer plays
    // at 48 kHz device rate → 2.18× too fast → exactly the
    // «скоростной неразборчивый звук» the user heard
    // (2026-05-31 test, captured 48000 Hz mic, played 22050 Hz
    // Piper output, audio chipmunked). Resample to the device's
    // rate via ffmpeg before handing off. Phase 13b will land a
    // pure-Rust resample inside `play_blocking` itself.
    let device_rate = preferred_output_sample_rate();
    if raw_pcm.sample_rate == device_rate {
        return Ok(raw_pcm);
    }
    let in_path = tmp_dir.join(format!("voice_repl_piper_pre_{pid}.wav"));
    let out_path = tmp_dir.join(format!("voice_repl_piper_rs_{pid}.wav"));
    adam_audio::wav::write_wav(&in_path, &raw_pcm)?;
    let status = Command::new("ffmpeg")
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .arg("-y")
        .arg("-i")
        .arg(&in_path)
        .arg("-ar")
        .arg(device_rate.to_string())
        .arg("-ac")
        .arg("1")
        .arg("-c:a")
        .arg("pcm_s16le")
        .arg(&out_path)
        .status()?;
    let _ = std::fs::remove_file(&in_path);
    if !status.success() {
        return Err("ffmpeg resample failed".into());
    }
    let resampled = adam_audio::wav::read_wav(&out_path)?;
    let _ = std::fs::remove_file(&out_path);
    Ok(resampled)
}

/// Query cpal's default output device for its preferred sample
/// rate. Falls back to 48 kHz on any error (the most common
/// macOS / Linux default).
fn preferred_output_sample_rate() -> u32 {
    use cpal::traits::{DeviceTrait, HostTrait};
    match cpal::default_host().default_output_device() {
        Some(dev) => match dev.default_output_config() {
            Ok(cfg) => cfg.sample_rate().0,
            Err(_) => 48_000,
        },
        None => 48_000,
    }
}

/// Phase 15 (2026-05-31) whisper.cpp STT backend.
///
/// Shells out to the `whisper-cli` binary (brew install
/// whisper-cpp) with a ggml-format multilingual model and the
/// Kazakh language hint. Input is the recorded `PcmSamples`;
/// we write it as a 16 kHz mono WAV in `/tmp/` (whisper-cli
/// reads from disk), invoke whisper-cli, strip its progress
/// banner from the stdout, and return the recognised Cyrillic
/// text.
///
/// Pairs symmetrically with `synthesise_via_piper`: both are
/// subprocess wrappers around well-maintained C/C++ neural
/// runtimes that ship as `brew` binaries. Phase 13b will
/// replace BOTH with pure-Rust `tract-onnx` adapters that run
/// the same models in-process.
fn transcribe_via_whisper(
    pcm: &adam_audio::PcmSamples,
    model_path: &std::path::Path,
    prompt: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    use std::process::Command;

    if !model_path.exists() {
        return Err(format!(
            "whisper ggml model not found at {} — fetch with:\n  \
             curl -L -o {} https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
            model_path.display(),
            model_path.display()
        )
        .into());
    }
    // whisper-cli expects 16 kHz mono WAV on disk.
    let tmp_dir = std::env::temp_dir();
    let pid = std::process::id();
    let tmp_wav = tmp_dir.join(format!("voice_repl_whisper_in_{pid}.wav"));
    adam_audio::wav::write_wav(&tmp_wav, pcm)?;

    let mut cmd = Command::new("whisper-cli");
    cmd.arg("-m")
        .arg(model_path)
        .arg("-l")
        .arg("kk")
        .arg("-f")
        .arg(&tmp_wav)
        .arg("-nt") // no timestamps
        .arg("--print-progress")
        .arg("false");
    if !prompt.is_empty() {
        cmd.arg("--prompt").arg(prompt);
    }
    let output = cmd.stderr(std::process::Stdio::piped()).output()?;

    let _ = std::fs::remove_file(&tmp_wav);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "whisper-cli exited with {:?}: {}",
            output.status.code(),
            stderr.trim()
        )
        .into());
    }

    // Parse stdout: whisper-cli prints lots of init noise before
    // the transcription. The transcription itself is on lines
    // that aren't bracket/log-prefixed. Keep the longest non-log
    // line as the transcript.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut best: String = String::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty()
            || line.starts_with('[')
            || line.starts_with("whisper_")
            || line.starts_with("ggml_")
            || line.starts_with("load_")
            || line.starts_with("main")
            || line.starts_with("system_info")
        {
            continue;
        }
        if line.len() > best.len() {
            best = line.to_string();
        }
    }
    if best.is_empty() {
        return Err(format!("whisper-cli produced no recognisable output:\n{stdout}").into());
    }
    Ok(best)
}

/// Phase 15e (2026-05-31) — fuzzy STT-output normaliser.
///
/// User directive (live REPL feedback 2026-05-31):
/// > «Используя математические графы, найти и извлечь из памяти
/// >  ближайшие похожие слова на этот «Калымыз қалай!», а не
/// >  отвечать, что не понял.»
///
/// Multilingual Whisper substitutes the Kazakh-specific letters
/// (Қ→К, Ғ→Г, Ң→Н, Ө→О, Ұ→У, Ү→У, Һ→Х, І→И, Ә→Е) on ambiguous
/// audio. The Phase 15d initial-prompt fix anchors most of these,
/// but live REPL still showed «Калыңыз» / «Танысаиық» (1-letter
/// drift). This normaliser closes that long-tail by mapping each
/// noisy token to its nearest canonical intent-trigger word
/// under `kazakh_edit_distance` (Kazakh-aware Levenshtein with
/// phonetic-pair substitution cost = 0.4 for K↔Қ etc.).
///
/// Conservative thresholding: only substitute when the
/// similarity score ≥ 0.75 AND there IS a non-identity change
/// (i.e. the input is genuinely different from the canonical).
/// Otherwise pass through. Punctuation around tokens is
/// preserved by stripping/re-attaching at the boundary.
///
/// The vocabulary is the union of:
///   - identity-question triggers («кімсің», «боласың», «екенсің»,
///     «өзіңіз», «менің», «атым», …)
///   - common interrogatives («қалай», «неше», «қай»)
///   - time/date keywords («бүгін», «қазір», «сағат», «күн»)
///   - greeting/introduction («сәлеметсіз», «сәлем», «танысайық»)
///   - polite-2nd-person variants («қалыңыз», «жайыңыз», «сіз»)
///
/// We deliberately keep this list SHORT to avoid over-correction
/// on out-of-domain inputs (we'd rather pass through unknown
/// words than pull them toward a wrong canonical).
fn fuzzy_normalise(text: &str) -> String {
    use adam_dialog::kazakh_fuzzy::best_match;
    use std::sync::OnceLock;

    static VOCAB_OWNED: OnceLock<Vec<String>> = OnceLock::new();
    let vocab: &[String] = VOCAB_OWNED.get_or_init(|| {
        intent_vocab_static()
            .iter()
            .map(|s| s.to_string())
            .collect()
    });

    const fn intent_vocab_static() -> &'static [&'static str] {
        &[
            // Greeting
            "сәлем",
            "сәлеметсіз",
            "ассалаумағалейкум",
            // How-are-you
            "қалай",
            "қалайсыз",
            "қалыңыз",
            "жайыңыз",
            "жағдайыңыз",
            // Identity
            "сен",
            "сіз",
            "өзің",
            "өзіңіз",
            "кімсің",
            "кімсіз",
            "кімсін",
            "боласың",
            "боласыз",
            "боласын",
            "екенсің",
            "екенсіз",
            "адам",
            // Name
            "менің",
            "атым",
            "есімім",
            "кім",
            // Time/Date — note: «бүгінгі» helps the fuzzy match
            // reach «бүгін» from 4-letter STT drift «бұғың».
            "бүгін",
            "бүгінгі",
            "қазір",
            "сағат",
            "неше",
            "күн",
            "қай",
            "қайсы",
            "ертең",
            "кеше",
            // Place
            "қазақстан",
            "алматы",
            "астана",
            "нұр-сұлтан",
            // Discourse
            "танысайық",
            "танысалық",
            "алдымен",
            "туралы",
            "айтшы",
            "айтыңыз",
            "айтасыз",
            "ба",
            "ма",
            // Common short particles
            "иә",
            "жоқ",
            "рахмет",
            "кешіріңіз",
            // **Phase 15f.4 (2026-05-31)** — gender / person words.
            // Live REPL turn «Мен еркек.» got fuzzy-mangled to
            // «Мен ертең.» because «еркек» was missing from vocab
            // and the 0.70 best_match gate pulled it to the
            // closest canonical form. Adding both gender words
            // and the negative copula keeps fuzzy honest on
            // identity-correction utterances like:
            //   «Мен апа емеспін.»  / «Мен еркек.»  / «Мен әйел.»
            "еркек",
            "әйел",
            "емес",
            "емеспін",
            "емессіз",
            "емессің",
            "жоқпын",
            "жасым",
            "жасыңыз",
            "жасы",
            // **Phase 15e.next (2026-05-31)** — math operators
            // + numerals. Live REPL turns 11–14 surfaced
            // «кубей» (Whisper) for «көбейт» (multiply), «азаид»
            // for «азайт» (subtract), «жерма» for «жиырма» (20),
            // «бісті» for «бесті» (acc. of 5), «түртке» for
            // «төртке» (dat. of 4). Adding the canonical roots
            // + frequent case-marked forms lets fuzzy_normalise
            // repair them before the dialog engine's math
            // handler runs.
            "қосу",
            "қос",
            "плюс",
            "көбейту",
            "көбейт",
            "көбейтіңіз",
            "азайту",
            "азайт",
            "азайтыңыз",
            "минус",
            "бөлу",
            "бөл",
            "бөліңіз",
            "тең",
            "нәтиже",
            "есепте",
            "қанша",
            "болады",
            // Numerals — base forms
            "бір",
            "екі",
            "үш",
            "төрт",
            "бес",
            "алты",
            "жеті",
            "сегіз",
            "тоғыз",
            "он",
            "жиырма",
            "отыз",
            "қырық",
            "елу",
            "алпыс",
            "жетпіс",
            "сексен",
            "тоқсан",
            "жүз",
            "мың",
            // Frequent case forms used in math («екіні бөл»,
            // «бесті көбейт», «отызға тең»):
            "екіге",
            "үшке",
            "төртке",
            "беске",
            "екіні",
            "үшті",
            "төртті",
            "бесті",
        ]
    }

    // **Phase 15f.3 (2026-05-31)** — Whisper's «і»/«ы»/«ө»-drop on
    // short numerals («екі»→«ек», «үш»→«уш», «төрт»→«торт»,
    // «бес»→«бис», «алты»→«алт», «жеті»→«жет»). These fall just
    // under the 0.70 best_match gate (one-char delete on a 3-char
    // word ≈ 0.67 similarity), so they get repaired by explicit
    // alias BEFORE the general fuzzy pass. Math-only — restoring
    // these is unambiguous: «ек» is never a real Kazakh word.
    // Live REPL turn 11 surfaced this when «Екі қосу екі» came
    // back as «Ек қосу ек» and the dialog engine routed to a
    // definition_lookup of «қосу» instead of arithmetic_eval.
    static SHORT_NUMERAL_ALIAS: &[(&str, &str)] = &[
        ("ек", "екі"),
        ("уш", "үш"),
        ("торт", "төрт"),
        ("бис", "бес"),
        ("алт", "алты"),
        ("жет", "жеті"),
        ("сегиз", "сегіз"),
        ("тогиз", "тоғыз"),
    ];

    text.split_whitespace()
        .map(|word| {
            // Strip leading/trailing punctuation, keep core token.
            let leading: String = word.chars().take_while(|c| !c.is_alphabetic()).collect();
            let trailing: String = word
                .chars()
                .rev()
                .take_while(|c| !c.is_alphabetic())
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            let core: String = word
                .chars()
                .skip_while(|c| !c.is_alphabetic())
                .collect::<String>()
                .trim_end_matches(|c: char| !c.is_alphabetic())
                .to_string();
            if core.is_empty() {
                return word.to_string();
            }
            let lower = core.to_lowercase();
            // Phase 15f.3: explicit short-numeral alias pass before
            // any other normalisation. Cheap O(n) over an 8-entry
            // table; runs once per word.
            for (drift, canonical) in SHORT_NUMERAL_ALIAS {
                if &lower == drift {
                    return format!("{leading}{canonical}{trailing}");
                }
            }
            // Already canonical → skip (saves a scan).
            if vocab.iter().any(|c| c == &lower) {
                return word.to_string();
            }
            // best_match returns (canonical, score) when score ≥
            // threshold. 0.70 catches up-to-3 phonetically-close
            // substitutions (e.g. «бұғың»→«бүгін»: ұ↔ү + ы↔і +
            // ң↔н with phonetic cost 0.4 each ≈ similarity 0.76);
            // a lower threshold risks pulling out-of-domain words
            // to wrong canonicals. Tuned 2026-05-31 from 0.75
            // after live REPL surfaced the 4-substitution case
            // «Бұғың→Бүгін» falling just short of the 0.75 gate.
            if let Some((canonical, _score)) = best_match(&lower, vocab, 0.70) {
                if canonical != lower {
                    return format!("{leading}{canonical}{trailing}");
                }
            }
            word.to_string()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Phase 15f loader — same logic as `adam_chat.rs`'s
/// `load_retrieval_index` (kept in sync so both REPLs read the
/// same JSON artefact). The index ships at
/// `data/retrieval/morpheme_index.json`.
fn load_retrieval_index() -> Option<MorphemeIndex> {
    let file = std::fs::File::open(RETRIEVAL_INDEX_PATH).ok()?;
    let reader = std::io::BufReader::new(file);
    let mut idx: MorphemeIndex = serde_json::from_reader(reader).ok()?;
    idx.refresh_stats();
    Some(idx)
}

/// Phase 15f loader — extracted + derived facts. Missing files
/// return empty vectors so the REPL stays usable on a trimmed
/// checkout.
fn load_reasoning_chains() -> (
    Vec<adam_reasoning::Fact>,
    Vec<adam_reasoning::reasoner::DerivedFact>,
) {
    #[derive(serde::Deserialize)]
    struct FactsFile {
        facts: Vec<adam_reasoning::Fact>,
    }
    #[derive(serde::Deserialize)]
    struct DerivedFile {
        derived: Vec<adam_reasoning::reasoner::DerivedFact>,
    }
    let extracted = std::fs::File::open(FACTS_PATH)
        .ok()
        .and_then(|f| serde_json::from_reader::<_, FactsFile>(std::io::BufReader::new(f)).ok())
        .map(|f| f.facts)
        .unwrap_or_default();
    let derived = std::fs::File::open(DERIVED_FACTS_PATH)
        .ok()
        .and_then(|f| serde_json::from_reader::<_, DerivedFile>(std::io::BufReader::new(f)).ok())
        .map(|f| f.derived)
        .unwrap_or_default();
    (extracted, derived)
}
