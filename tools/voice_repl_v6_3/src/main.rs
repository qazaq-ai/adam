// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! **Voice REPL v6.3** — the full Kazakh voice loop, standalone.
//!
//! ## Honest hybrid (2026-06-02 doc audit)
//!
//! Earlier drafts of this docstring claimed a «Whisper-free, pure
//! Rust end-to-end» pipeline. That was the v6.3 design goal but is
//! no longer what the binary does. Live STT accuracy on natural
//! Kazakh speech was too low to be useful with the in-tree DTW
//! recogniser alone, so the production path now runs:
//!
//! ```text
//!   microphone (cpal via adam-audio::record)
//!     ↓ PCM mono 48 kHz
//!     ↓ STT — Whisper.cpp (`whisper-cli`, multilingual ggml model)
//!                  default; in-tree DTW phoneme recogniser as
//!                  `--stt-backend dtw` fallback when model absent.
//!     ↓ token-split merge + Zipf-fuzzy + neural LM rescoring
//!   normalised Cyrillic transcript
//!     ↓ BPE-tokenised neural intent classifier (parallel log)
//!     ↓ Phase 19.G high-confidence override (input rewriting)
//!     ↓ adam-dialog v6.2 router (deterministic, ADAM_V6_2=1)
//!   Kazakh response sentence
//!     ↓ Piper TTS (`kk_KZ-issai-high`) via subprocess
//!     ↓ adam-audio::play
//!   speaker
//! ```
//!
//! **Deterministic vs. neural**: the v6.2 dialog router (intent
//! routing, retrieval, reasoning, realisation) stays 100 % rule-
//! based and inspectable. The neural pieces (Whisper STT, Piper
//! TTS, tiny contextual LM, BPE intent classifier) live at the
//! voice surface only — pre-processing inputs and post-processing
//! outputs around the deterministic core. None of them invent
//! facts; they only normalise audio ↔ text.
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

mod coherence;
mod context_corrections;
mod correction_persist;
#[cfg(test)]
mod drift_battery;
mod intent_classifier_runtime;
mod lexicon_validator;
mod multi_act_splitter;
mod neural_override;
mod neural_rescorer;
mod rejection_detector;
mod session_journal;
mod zipf_vocab;

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
    ///
    /// **Phase 15g.C (2026-06-02) — default switched to Shirali
    /// fine-tuned KZ Whisper.** The ISSAI/Shirali model
    /// (`whisper-small-ISSAI_KSC_335RS_v2` from HuggingFace,
    /// converted HF→ggml via `whisper.cpp/models/convert-h5-to-ggml.py`)
    /// is trained on the Kazakh Speech Corpus and produces canonical
    /// Қ/Ғ/Ң/Ө/Ұ/Ү/Һ/І/Ә where multilingual Whisper drifts to К/Г/Н/etc.
    /// File size: 465 MB (whisper-small architecture).
    ///
    /// Fallback alternatives if Shirali file missing or produces
    /// artefacts on a specific audio:
    ///   data/stt_models/ggml-medium.bin   (multilingual, 1.5 GB)
    ///   data/stt_models/ggml-small.bin    (multilingual, 488 MB)
    /// Override at runtime:
    ///   --whisper-model data/stt_models/ggml-medium.bin
    #[arg(long, default_value = "data/stt_models/ggml-shirali-kz.bin")]
    whisper_model: PathBuf,
    /// Initial prompt fed to whisper-cli to bias decoding toward
    /// Kazakh-specific phonotactics. Multilingual Whisper otherwise
    /// substitutes Қ→К, Ғ→Г, Ң→Н, Ө→О, Ұ→У, Ү→У, Һ→Х, І→И, Ә→Е.
    ///
    /// **Phase 15g.A.1 (2026-05-31)** — was a full-sentence prompt;
    /// caused prompt leakage where unclear / noisy audio would have
    /// Whisper hallucinate phrases straight from the prompt
    /// («Ассаламу алейкум» → «Сәлеметсіз бе. Қазір сағат неше?»).
    /// Now a letter-anchor list — one short word per Kazakh-specific
    /// letter. Same biasing effect, no full phrases for Whisper to
    /// regurgitate.
    #[arg(
        long,
        default_value = "Қ: қазақ. Ғ: ғылым. Ң: оның. \
            Ө: өзен. Ұ: ұлы. Ү: үй. \
            Һ: һәм. І: іс. Ә: әке."
    )]
    whisper_prompt: String,
    /// Dialog mode. `echo` (default for now) = TTS re-speaks the
    /// STT output (Phase 13/15 loopback validation). `respond` =
    /// route the recognised text through `adam_dialog::Conversation`
    /// so the system answers instead of echoing. **`wellness`**
    /// (v6.4) = route through the `adam-wellness` IFS state machine
    /// — Kazakh-language reflective companion grounded in evidence-
    /// based therapy frameworks.  NOT a medical treatment system;
    /// see crates/adam-wellness/src/lib.rs for the safety contract.
    #[arg(long, default_value = "respond")]
    mode: String,
    /// Per-session RNG seed for dialog response selection.
    #[arg(long, default_value_t = 42)]
    seed: u64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // **Phase 15g.H (2026-06-01)** — enable the v6.2 router by
    // default in the voice REPL. The router (`adam_dialog::v6_2_router`)
    // handles inventory queries («Қазақстанда қандай таулар бар»
    // → list of mountains, not the «тау = жер бедері» definition
    // lookup the v6.1 stack falls back to), plus a curated geo /
    // history / abai factual battery. It defaults to off elsewhere
    // (opt-in via ADAM_V6_2=1) so existing CI / text REPL stay
    // identical. On the voice path we want the inventory answers
    // by default.
    if std::env::var("ADAM_V6_2").is_err() {
        // SAFETY: env::set_var is `unsafe` since Rust 1.79 due to a
        // documented race with threaded readers — we're calling it
        // BEFORE spawning any worker threads and reading the var,
        // and only when it's not already set, so this single write
        // is sound.
        unsafe { std::env::set_var("ADAM_V6_2", "1") };
        println!("[voice-repl] ADAM_V6_2=1 (v6.2 inventory router enabled)");
    }

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
    // **v6.4.0-rc7 (2026-06-08).**  Load the v6.2 dialog engine for
    // BOTH `respond` AND `wellness` modes.  Prior to rc7, wellness
    // mode skipped the cascade entirely — so factual queries
    // («Қазір сағат неше?», «Бүгін қай күн?», «Екі қосу екі»)
    // got wellness templates even when the intent classifier
    // labelled them with 1.00 confidence as factual.  User audit:
    // «диалог должен быть простым и по запросу пользователя, а не
    // по хотелке модели».  rc7 routes per-turn: factual intents
    // go through the v6.2 cascade, emotion / distress intents go
    // through the wellness arm.
    let (dialog_state, mut conversation): (
        Option<(LexiconV1, TemplateRepository)>,
        Option<Conversation>,
    ) = if args.mode == "respond" || args.mode == "wellness" {
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

    // **Phase 15g.B (2026-06-01)** — Zipf-ranked hot vocabulary.
    //
    // Pre-15g.B used a flat 3155-word list built from
    // `INTENT_VOCAB` ∪ `adam_lexicon_curated::full_lexicon()`.
    // That set had no frequency signal, so on ties fuzzy pulled
    // STT outputs toward whichever canonical happened to be
    // alphabetically first («даулет» → «сәулет»). 15g.B replaces
    // it with a corpus-derived Zipf-ranked list:
    //   * 1 000 most-frequent surface forms from cc100_kk +
    //     Wikipedia + Abai (3.48 M tokens / 243 k distinct),
    //     covering 43.74 % of corpus tokens — the cheapest 80 %
    //     of conversational vocabulary;
    //   * ≈70 explicit overrides (greetings, honorifics, math
    //     ops, gender words) so rare-but-critical canonicals stay
    //     in the hot path.
    // Build once at startup; `best_match` weighs phonetic
    // similarity AND Zipf rank so the tie-break goes to the more
    // frequent canonical, not the alphabetically-first one.
    // Phase 15g.B.2 — vocabulary loader takes a path arg for source
    // compatibility; the corpus-derived Zipf JSON is no longer read.
    let zipf_vocab = zipf_vocab::ZipfVocab::load_or_overrides_only("");

    // **Phase 15g.C.2 step 3 (2026-06-01)** — neural rescorer.
    // Loads the contextual LM trained by `train_contextual_lm`
    // (vocab 5188, ~2 M params, final CE 1.12). If any artefact
    // is missing the loader returns None and rescoring is skipped
    // — voice REPL behaviour is then identical to 15g.B.2.
    let neural = neural_rescorer::NeuralRescorer::load_default();

    // **Phase 19 step E (2026-06-02)** — neural intent classifier
    // runs **alongside** the substring intent matchers in
    // adam-dialog. We don't override the substring decision yet —
    // just log the neural prediction (intent + confidence) so we
    // can see when neural and substring agree / disagree on the
    // same input. Once the discrepancy data shows neural is
    // reliable on a category, we'll switch that category to
    // neural-first routing.
    let intent_classifier = intent_classifier_runtime::IntentClassifierRuntime::load_default();

    // **v6.4 (2026-06-04)** — wellness arc.  When launched with
    // `--mode wellness`, the REPL skips the `respond` cascade
    // entirely and routes user utterances through the IFS state
    // machine in `adam-wellness`.  Session state lives across
    // turns (current stage, focal emotion, turn counter).  When a
    // session closes (graceful abort or final integration) a
    // fresh one is started on the next utterance so the user can
    // do another cycle.  Red-flag escalation inside `ifs::step`
    // clears the session — the next utterance is the user's
    // chance to opt back in.
    // **rc7** — `--mode wellness` no longer hard-locks every turn
    // into the wellness arm.  Instead the session is DORMANT at
    // boot (stage = None); each turn's intent classifier decides
    // whether to route to wellness or fall through to the v6.2
    // cascade.  The session only enters intake (AskingName) when
    // the user surfaces emotion content or explicitly asks to do
    // wellness work.  This matches the user contract from the rc6
    // audit: «диалог по запросу пользователя, не по хотелке модели».
    let mut wellness_session = if args.mode == "wellness" {
        println!(
            "[voice-repl] mode=wellness — wellness arm available on \
             demand.  Factual queries (time, math, name recall) still \
             route through the v6.2 cascade.  Red-flag detector active \
             on every turn regardless of route."
        );
        Some(adam_dialog::wellness::ifs::WellnessSession::default())
    } else {
        None
    };

    // **v6.5.0-rc5 self-learning foundation.**  Session journal
    // captures each completed turn so the rejection detector can
    // spot, on the NEXT turn, that the user is rejecting / rephrasing
    // / correcting the last reply.  rc5 ships detection + log only;
    // rc6 wires this into a persisted `mistake_corrections.jsonl`
    // and rc7 reads that file on the way IN to override the cascade.
    let mut journal = session_journal::SessionJournal::new();

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
                        // **rc5 self-learning probe.**  Before any
                        // downstream processing, ask the rejection
                        // detector whether this turn looks like the
                        // user is rejecting / rephrasing / correcting
                        // adam's previous reply.  rc5 only logs the
                        // observation; rc6 will persist it.
                        if let Some(sig) = rejection_detector::detect(&text, &journal) {
                            println!(
                                "[voice-repl] [journal] REJECTION DETECTED — {}",
                                rejection_detector::render_log(&sig)
                            );
                            // **rc7 self-learning persist.**  Save the
                            // rejected turn + the user's clarifying
                            // input to `data/mistake_corrections.jsonl`.
                            // rc8 will load this file at startup to
                            // override the cascade on matching inputs.
                            // Failure to persist must NEVER crash the
                            // REPL — log + continue.
                            let record =
                                correction_persist::CorrectionRecord::from_signal(&sig, &text);
                            match correction_persist::append(&record) {
                                Ok(()) => println!(
                                    "[voice-repl] [journal] correction persisted → {}",
                                    correction_persist::DEFAULT_CORRECTION_PATH
                                ),
                                Err(e) => eprintln!(
                                    "[voice-repl] [journal] correction persist failed: {e}"
                                ),
                            }
                        }
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
        // **Phase 15g.C.1 (2026-06-02)** — merge Whisper-Shirali
        // token splits («көл дер» → «көлдер», etc.) BEFORE fuzzy /
        // intent classification. This is a pre-step, not a rewrite,
        // so the LM safety net below sees the merged surface as
        // both the «raw» and (typically) the «proposed» state.
        let user_text_merged = merge_whisper_splits(&user_text);
        if user_text_merged != user_text {
            println!("[voice-repl] split-merge → «{user_text_merged}»");
        }

        // **Phase 22 step A (2026-06-03)** — context-aware STT
        // corrections. Pre-step before fuzzy/LM/intent. Codifies live
        // REPL findings where the corrected form is grammatically
        // valid and the original form is unambiguously a STT drift
        // (e.g. «менің атом X» → «менің атым X»). See
        // `context_corrections.rs` for the patch list.
        //
        // **Phase 22 step B (2026-06-03)** — session-aware variant.
        // When adam's previous reply was «Атыңызды айта аласыз ба?»
        // (IntroProposal asking for name), we flip on a more
        // aggressive name-extraction pass that catches a broader set
        // of Whisper drifts. The flag is kept in `conv.session` so
        // it survives across the iteration boundary.
        let awaiting_name = conversation
            .as_ref()
            .and_then(|c| c.session.get("voice_awaiting_name"))
            .is_some();
        let user_text_corrected =
            context_corrections::apply_with_context(&user_text_merged, awaiting_name);
        if user_text_corrected != user_text_merged {
            let tag = if awaiting_name {
                "context-fix (awaiting-name)"
            } else {
                "context-fix"
            };
            println!("[voice-repl] {tag} → «{user_text_corrected}»");
        }
        let user_text_merged = user_text_corrected;

        // **v6.5.0-rc23 — universal per-token lexicon validator.**
        // Walks every word in the input through the FST/LexiconV1.
        // Words that parse pass through unchanged.  Words that
        // don't parse get a UNIQUE edit-distance-1 lexicon
        // neighbour substituted in (e.g. «Қалыңғыз» → «Қалыңыз»,
        // «бөль» → «бөл», «Гейц» → «Гейтс»).  Ambiguous fixes
        // (multiple equidistant neighbours) are skipped to avoid
        // the «қатым/қауын» minimal-pair trap.
        //
        // Runs BEFORE the legacy fuzzy_normalise so the heavier
        // Zipf-fuzzy only sees tokens that the FST itself couldn't
        // recover.  Logged per-turn for diagnostics.
        // **v6.6 generative pivot (2026-06-11)** — lexicon_validator
        // DISABLED. rc25-audit confirmed the edit-distance-1 +
        // hot-vocab approach is net-negative: it rewrites valid
        // tokens («дәулет→сәулет», «тұрамын→тұратын», «Жасым→Жасы»,
        // «толды→толы», «айттым→айтты», «тұрам→тұра», «берші→бері»,
        // «алаған→алған») more often than it repairs Whisper drift.
        // The right place to recover from Whisper noise is a
        // context-aware LM rescorer trained on the full 338k-sentence
        // corpus, not a runtime edit-distance rule. Leaving the
        // module in tree (`mod lexicon_validator;` above) so the
        // tests still serve as a regression spec for any future
        // contextual replacement.
        let user_text_merged = user_text_merged;

        let fuzzy_out = if args.mode == "respond" || args.mode == "wellness" {
            fuzzy_normalise(&user_text_merged, &zipf_vocab)
        } else {
            user_text_merged.clone()
        };

        // **Phase 15g.C.2 step 3 (2026-06-01)** — neural rescorer
        // safety net. When fuzzy proposed a rewrite, check the
        // contextual LM agrees the rewritten sentence is more
        // plausible than the raw Whisper output. If LM disagrees
        // (rewrite has LOWER per-token log-prob than original),
        // revert to the raw output — catches regressions like
        // «даулет → сәулет» that were B.1 / B.2's failure mode.
        // If no rescorer (missing checkpoint), keep fuzzy as-is.
        let normalised = if fuzzy_out != user_text_merged {
            if let Some(r) = neural.as_ref() {
                match (r.score_text(&user_text_merged), r.score_text(&fuzzy_out)) {
                    (Some(orig), Some(rew)) => {
                        // Tolerance: only revert when the rewrite is
                        // clearly worse than the original (≥ 0.05 nats/
                        // token gap). Avoids flapping on numerical noise.
                        if rew + 0.05 < orig {
                            println!(
                                "[voice-repl] fuzzy → «{fuzzy_out}» — LM score \
                                 (orig={orig:.3} vs rew={rew:.3}) reverted to raw"
                            );
                            user_text_merged.clone()
                        } else {
                            println!(
                                "[voice-repl] fuzzy → «{fuzzy_out}» (LM orig={orig:.3} rew={rew:.3})"
                            );
                            fuzzy_out
                        }
                    }
                    _ => {
                        println!("[voice-repl] fuzzy → «{fuzzy_out}»");
                        fuzzy_out
                    }
                }
            } else {
                println!("[voice-repl] fuzzy → «{fuzzy_out}»");
                fuzzy_out
            }
        } else {
            user_text_merged.clone()
        };

        // **Phase 19 step E (2026-06-02)** — parallel-log neural
        // intent classifier prediction.  Runs ALONGSIDE the
        // adam-dialog substring intent layer.  We log:
        //   [voice-repl] intent (neural) → AskTime (conf=0.83)
        // The substring path still drives the final response;
        // this is observability for the Phase 19.F iteration
        // where we'll start trusting neural-first on high-
        // confidence predictions.
        let neural_intent: Option<(String, f32)> = if let Some(ic) = intent_classifier.as_ref() {
            let pred = ic.classify(&normalised);
            if let Some((label, conf)) = pred.as_ref() {
                let marker = if *conf >= 0.70 {
                    "✓"
                } else if *conf >= 0.40 {
                    "~"
                } else {
                    "?"
                };
                println!("[voice-repl] intent (neural) → {label} (conf={conf:.2}) {marker}");
            }
            pred
        } else {
            None
        };

        // **Phase 19 step G (2026-06-02)** — high-confidence neural
        // override. When the neural classifier is strongly sure the
        // utterance is `AskAboutTopic` but the surface form contains
        // substring triggers that misroute (capability handler on
        // «не білесің», language definition on «тіл»), rewrite the
        // input so the substring router falls into the right branch.
        // Conservative: only kicks in at conf ≥ 0.85 and only when a
        // known trigger pattern is present.
        let normalised = match neural_intent.as_ref() {
            Some((label, conf)) if *conf >= 0.85 && label == "AskAboutTopic" => {
                let rewritten = neural_override::rewrite_topic_query(&normalised);
                if rewritten != normalised {
                    println!(
                        "[voice-repl] intent override → «{rewritten}» (neural AskAboutTopic conf={conf:.2})"
                    );
                }
                rewritten
            }
            _ => normalised,
        };

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
        // **v6.4 rc3** — pitch-gender estimation now fires in BOTH
        // `respond` and `wellness` modes, since wellness uses the
        // hint to pick the right honorific (Ағай / Апай / Балам)
        // even though it has no `Conversation` instance.
        let i16_samples: Vec<i16> = pcm
            .data
            .iter()
            .map(|s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
            .collect();
        let pitch_gender = adam_voice::pitch::estimate_pitch_hz(&i16_samples, pcm.sample_rate)
            .and_then(adam_voice::pitch::classify_gender);
        if let Some(g) = pitch_gender {
            let label = match g {
                adam_voice::pitch::PitchGender::Male => "male",
                adam_voice::pitch::PitchGender::Female => "female",
                adam_voice::pitch::PitchGender::Child => "child",
            };
            // Respond-mode path: write into the Conversation session.
            if let Some(conv) = conversation.as_mut() {
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
            // Wellness-mode path: push the hint into the IFS
            // session.  Idempotent — overwrite each turn so the
            // lock-stability logic isn't needed here.
            if let Some(session) = wellness_session.as_mut() {
                let hint = match g {
                    adam_voice::pitch::PitchGender::Male => {
                        adam_dialog::wellness::ifs::GenderHint::Male
                    }
                    adam_voice::pitch::PitchGender::Female => {
                        adam_dialog::wellness::ifs::GenderHint::Female
                    }
                    adam_voice::pitch::PitchGender::Child => {
                        adam_dialog::wellness::ifs::GenderHint::Child
                    }
                };
                session.set_gender_hint(hint);
                println!("[voice-repl] wellness gender hint = {label}");
            }
        }

        // **v6.4.0-rc8 (2026-06-08 audit).**  Intent-aware routing.
        // rc7 made the router fire wellness whenever the session
        // was active — but that meant once you started IFS work,
        // every subsequent turn was hijacked.  Live audit caught
        // «Биллион саны қанша?» (Math 0.72) and «Менім атым кім?»
        // (AskName 0.97) routed through wellness because the
        // session was sitting in IdentifyPart.
        //
        // rc8 — even mid-IFS, if THIS turn is a clearly-factual
        // intent (Math / Time / Date / memory recall / etc.), let
        // the v6.2 cascade answer.  The wellness session sits.
        // The next emotion-content turn resumes the IFS work.
        //
        // Red-flag still always preempts — safety must not be
        // gated by intent classifier confidence.
        // **rc9 (2026-06-08 audit).**  Expanded factual whitelist
        // — rc8 list missed AskActivity (audit hit «таулар бар»
        // 1.00), InventoryQuery (1.00 on enumerations), Affirmation
        // / Negation / StatementOfWellbeing.  User feedback:
        // «пользователь не должен говорить специальное кодовое
        // слово «Жалғастырайық» — это должно происходить по смыслу
        // следующего предложения».
        const FACTUAL_INTENTS: &[&str] = &[
            // Question intents
            "AskTime",
            "AskDate",
            "AskAge",
            "AskName",
            "AskAboutSystem",
            "AskAboutTopic",
            "AskActivity",
            "AskLocation",
            "AskFamily",
            "AskOccupation",
            "AskDefinition",
            "AskExercise",
            "AskWeather",
            "AskHowAreYou",
            "AskCurrentProgress",
            "AskCurriculumContent",
            "AskNextTopic",
            "AskPurpose",
            "AskWillingness",
            // Compute / code / lists
            "MathExpression",
            "CodeRequest",
            "ExplainCompilerError",
            "InventoryQuery",
            "CrossLanguageContrast",
            "SubmitSolution",
            "Request",
            // User identity / state statements
            "StatementOfName",
            "StatementOfAge",
            "StatementOfFamily",
            "StatementOfLocation",
            "StatementOfOccupation",
            "StatementOfActivity",
            "StatementOfWeather",
            "StatementOfWellbeing",
            // Social / civility
            "Greeting",
            "Farewell",
            "Thanks",
            "Apology",
            "WellWishes",
            "IntroProposal",
            "Affirmation",
            "Negation",
        ];
        let intent_is_clearly_factual = neural_intent.as_ref().is_some_and(|(label, conf)| {
            *conf >= 0.60 && FACTUAL_INTENTS.contains(&label.as_str())
        });

        // **rc9** — `PostEscalation` HOLDS intake memory but does
        // NOT block routing.  Live audit: after a 150 escalation,
        // user asked «Биллион саны қанша?», «Қазақстанда қандай
        // таулар бар?» — purely factual — and adam re-emitted
        // hotline reminder instead of answering.  In rc9 only
        // ACTIVELY-engaged IFS stages bias routing toward wellness;
        // PostEscalation / Closed / AskingName / AskingAge pass
        // through to the v6.2 cascade (factual answers flow), with
        // red_flag re-escalating on any fresh crisis input.  This
        // is the «semantic resume» the user asked for — no magic
        // code phrase needed.
        let in_active_ifs_work = wellness_session.as_ref().is_some_and(|s| {
            use adam_dialog::wellness::ifs::WellnessStage as Stage;
            matches!(
                s.stage,
                Some(
                    Stage::AskingProblem
                        | Stage::EmotionCheckIn
                        | Stage::IdentifyPart
                        | Stage::AskRole
                        | Stage::WitnessPain
                        | Stage::Unblending
                        | Stage::Integration
                )
            )
        });

        // **rc10 (2026-06-08 audit).**  Emotion content is now a
        // STRONGER signal than the factual whitelist.  rc9 audit
        // caught «Мен әкеме ашуланып жүрмін» misclassified as
        // AskWillingness (0.67, in factual whitelist) — wellness
        // never engaged.  Several intent labels are noisy on
        // emotion utterances (AskWillingness, AskActivity, even
        // AskAboutTopic), so when the lexicon explicitly names
        // an emotion («ашу», «реніш», «қорқыныш», etc.), wellness
        // wins regardless of the classifier label.
        let has_emotion_content =
            adam_dialog::wellness::ifs::extract_emotion(&normalised).is_some();

        let want_wellness = wellness_session.is_some()
            && (adam_dialog::wellness::red_flags::detect(&normalised).is_some()
                || has_emotion_content
                || (in_active_ifs_work && !intent_is_clearly_factual));

        // **v6.5 Movement D (2026-06-08).**  Sentence-coherence
        // gate — combine LM perplexity + intent confidence + FST
        // morphology parse coverage.  When at least 2 of 3 vote
        // "noise", refuse to route instead of guessing.  Red-flag
        // is the only signal that bypasses this — safety preempts
        // honesty.  See [[project_v6_5_strategic_plan]].
        let coherence = coherence::CoherenceSignals::collect(
            &normalised,
            neural.as_ref(),
            intent_classifier.as_ref(),
            dialog_state.as_ref().map(|(lex, _)| lex),
        );
        println!("[voice-repl] coherence → {}", coherence.render_log());
        let safety_bypass = wellness_session
            .as_ref()
            .is_some_and(|_| adam_dialog::wellness::red_flags::detect(&normalised).is_some());
        let coherence_refuses = !safety_bypass && !coherence.is_coherent();

        // **v6.5 Movement D**: coherence refusal pre-empts both
        // routes (except red-flag safety which is checked above).
        if coherence_refuses {
            println!(
                "[voice-repl] coherence refuse → adam → «{}»",
                coherence::REFUSE_TEMPLATE
            );
        }

        let cyrillic = if coherence_refuses {
            coherence::REFUSE_TEMPLATE.to_string()
        } else {
            match (
                &mut conversation,
                dialog_state.as_ref(),
                wellness_session.as_mut(),
                args.mode.as_str(),
                want_wellness,
            ) {
                (_, _, Some(session), "wellness", true) => {
                    // **wellness route** — red_flag, active session, or
                    // emotion content detected.  If session is dormant,
                    // bootstrap at AskingProblem (not AskingName) so
                    // we don't unilaterally interrogate the user for
                    // name + age when they just want to vent.
                    if session.stage.is_none() {
                        session.stage =
                            Some(adam_dialog::wellness::ifs::WellnessStage::AskingProblem);
                        session.turns_at_stage = 0;
                    }
                    let reply = adam_dialog::wellness::ifs::step(&normalised, session);
                    let action_tag = match reply.action {
                    adam_dialog::wellness::ifs::ReplyAction::Continue => "continue",
                    adam_dialog::wellness::ifs::ReplyAction::Escalate(flag) => match flag {
                        adam_dialog::wellness::red_flags::RedFlag::SuicidalIdeation => {
                            "escalate:suicidal"
                        }
                        adam_dialog::wellness::red_flags::RedFlag::AcuteMedicalSymptom => {
                            "escalate:medical"
                        }
                        adam_dialog::wellness::red_flags::RedFlag::ChildAbuse => {
                            "escalate:child-abuse"
                        }
                        adam_dialog::wellness::red_flags::RedFlag::DomesticViolenceImmediate => {
                            "escalate:dv"
                        }
                        adam_dialog::wellness::red_flags::RedFlag::Psychosis => {
                            "escalate:psychosis"
                        }
                    },
                    adam_dialog::wellness::ifs::ReplyAction::Close => "close",
                };
                    let stage = session
                        .stage
                        .map(|s| format!("{s:?}"))
                        .unwrap_or_else(|| "cleared".into());
                    println!("[voice-repl] wellness → stage={stage} action={action_tag}");
                    println!("[voice-repl] adam → «{}»", reply.text);
                    if !matches!(
                        reply.action,
                        adam_dialog::wellness::ifs::ReplyAction::Continue
                    ) {
                        let was_escalation = matches!(
                            reply.action,
                            adam_dialog::wellness::ifs::ReplyAction::Escalate(_)
                        );
                        if !was_escalation {
                            *session =
                                adam_dialog::wellness::ifs::WellnessSession::resume_after_clearance(
                                    session, false,
                                );
                        }
                    }
                    reply.text
                }
                (Some(conv), Some((lex, repo)), _, "respond", _)
                | (Some(conv), Some((lex, repo)), _, "wellness", false) => {
                    // **rc10 multi-act splitter.**  A trailing farewell
                    // («сау бол / қош бол / көріскенше …») after a
                    // substantive head means the user closed the turn
                    // with a compound utterance (e.g. rc9 audit T36
                    // «Өте жақсы аңгмелестік енді сау бол.»).  Route
                    // only the head through the cascade, then append
                    // the farewell acknowledgement to the reply.  When
                    // the input has NO trailing farewell, this is a
                    // no-op and the cascade runs on `normalised` as
                    // before.
                    let multi_act = multi_act_splitter::split_trailing_farewell(&normalised);
                    let cascade_input: &str = multi_act
                        .as_ref()
                        .map(|s| s.head.as_str())
                        .unwrap_or(&normalised);
                    if let Some(s) = &multi_act {
                        println!(
                            "[voice-repl] multi-act split → head=«{}» tail=«{}»",
                            s.head, s.tail
                        );
                    }
                    let mut reply = conv.turn(cascade_input, lex, repo, args.seed);

                    // **v6.5.0-rc25 — StatementOfName routing safeguard.**
                    // rc22 audit T9 «Менің атым - Дәулет» — the intent
                    // classifier said StatementOfName(1.00) but the
                    // cascade fell into topic search on «ат» (Kazakh
                    // for "name" / "horse") and emitted an Abai poem
                    // about horses arıḳtap.  When the neural intent is
                    // confident on StatementOfName AND the reply
                    // contains the known horse-poem fragment, override
                    // with a generic name-acceptance acknowledgement.
                    //
                    // This is a SAFETY NET, not a fix.  The proper fix
                    // is a cascade-side StatementOfName route that
                    // bypasses topic search; this belt-and-suspenders
                    // override catches the case until then.
                    if let Some((label, conf)) = neural_intent.as_ref()
                        && label == "StatementOfName"
                        && *conf >= 0.85
                        && (reply.contains("Әрі-бері айналса") || reply.contains("аты арықтап"))
                    {
                        reply = "Атыңызды есте сақтадым. Танысқанымызға қуаныштымын!".to_string();
                        println!(
                            "[voice-repl] StatementOfName guard → override topic-search reply"
                        );
                    }

                    if multi_act.is_some() {
                        reply = format!("{reply} {}", multi_act_splitter::FAREWELL_ACK);
                    }
                    println!("[voice-repl] adam → «{reply}»");

                    // **Phase 22 step B (2026-06-03)** — set / clear the
                    // `voice_awaiting_name` session flag based on whether
                    // THIS reply just asked for the user's name. The flag
                    // is read on the NEXT iteration before fuzzy/LM/intent
                    // to drive aggressive name-extraction in
                    // context_corrections::apply_with_context.
                    if context_corrections::reply_asks_for_name(&reply) {
                        conv.session
                            .insert("voice_awaiting_name".into(), "1".into());
                    } else {
                        conv.session.remove("voice_awaiting_name");
                    }

                    reply
                }
                _ => user_text.clone(),
            }
        };

        // **v6.5.0-rc5 self-learning journal.**  Record the
        // completed turn AFTER the reply is finalised but BEFORE
        // TTS playback.  The journal is consulted on the NEXT
        // iteration (top of the loop) by `rejection_detector::detect`,
        // so the order is: append now → next turn reads it.
        let journal_turn_no = journal.append(
            user_text.clone(),
            normalised.clone(),
            neural_intent.as_ref().map(|(lbl, _)| lbl.clone()),
            neural_intent.as_ref().map(|(_, c)| *c),
            cyrillic.clone(),
        );
        if args.loop_mode {
            println!(
                "[voice-repl] [journal] turn #{journal_turn_no} captured \
                 (journal_len={})",
                journal.len()
            );
        }

        // TTS playback. Phase 15f.1 (2026-05-31): in --mode=respond the
        // REPL is a voice dialog, not a STT tester — a silent reply
        // makes no sense. Force speak ON whenever we're responding.
        // `--speak` still works for --mode=echo (loopback debugging).
        let should_speak = args.speak || args.mode == "respond" || args.mode == "wellness";
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
/// **v6.4.0-rc11 (2026-06-08 audit).**  TTS preprocess for year
/// ranges.  Live audit feedback: Piper reads «(1991–2019)»
/// literally as «открывающая скобка одна тысяча девятьсот девяносто
/// один тире две тысячи девятнадцать закрывающая скобка», which
/// breaks the prosody of the surrounding Kazakh.  This helper
/// rewrites the parenthetical date forms into spoken Kazakh:
///
///   «(YYYY–YYYY)» / «(YYYY-YYYY)»  → «YYYY-YYYY жылдары»
///   «(YYYY жылдан бері)»            → «YYYY жылдан бері»
///   «(YYYY)»                        → «YYYY жылы»
///
/// Years are kept as digits — Piper's Kazakh voice reads digit
/// runs correctly («бір мың тоғыз жүз тоқсан бір» for 1991).
fn preprocess_year_ranges_for_tts(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 16);
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'(' {
            // Look for «(YYYY[–—\-]YYYY)» or
            // «(YYYY жылдан бері)» or bare «(YYYY)».
            // Slice from this byte; the rest is UTF-8 safe because
            // we only branched on the ASCII '('.
            let rest = &input[i..];
            if let Some(end) = rest.find(')') {
                let inner = &rest[1..end];
                if let Some(rewritten) = rewrite_paren_date(inner) {
                    out.push(' ');
                    out.push_str(&rewritten);
                    i += end + 1;
                    continue;
                }
            }
        }
        // Default: copy this byte through.  UTF-8 multi-byte chars
        // pass through unchanged because we don't branch inside them.
        out.push(input.as_bytes()[i] as char);
        i += 1;
    }
    // The byte-wise copy above mangles multi-byte UTF-8.  Fall back
    // to a simpler char-based pass when the input contains non-ASCII
    // (which Kazakh always does).  Re-do char-by-char with a single
    // regex-style sweep for «(N–N)» / «(N-N)» / «(N)» blocks.
    rewrite_paren_dates_char_wise(input)
}

/// Char-aware version of [`preprocess_year_ranges_for_tts`].  The
/// byte-wise scanner above is left in place for the ASCII case but
/// Kazakh always has multi-byte chars, so this is the actual
/// implementation used at runtime.
fn rewrite_paren_dates_char_wise(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len() + 16);
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '(' {
            if let Some(close_offset) = chars[i + 1..].iter().position(|&ch| ch == ')') {
                let inner: String = chars[i + 1..i + 1 + close_offset].iter().collect();
                if let Some(rewritten) = rewrite_paren_date(&inner) {
                    if !out.ends_with(' ') && !out.is_empty() {
                        out.push(' ');
                    }
                    out.push_str(&rewritten);
                    i += close_offset + 2; // skip past ')'
                    continue;
                }
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Returns Some(spoken form) when `inner` is a parenthetical date
/// block; None otherwise.
fn rewrite_paren_date(inner: &str) -> Option<String> {
    let trimmed = inner.trim();
    // «YYYY–YYYY» or «YYYY-YYYY»  (en-dash or hyphen)
    for sep in ['\u{2013}', '\u{2014}', '-'] {
        if let Some(dash) = trimmed.find(sep) {
            let left = trimmed[..dash].trim();
            let right = trimmed[dash + sep.len_utf8()..].trim();
            if is_year(left) && is_year(right) {
                return Some(format!("{left}-{right} жылдары"));
            }
        }
    }
    // «YYYY жылдан бері» / «YYYY жылдан»
    if trimmed.ends_with("жылдан бері") || trimmed.ends_with("жылдан") {
        // Drop the parens; let Piper read the inner phrase.
        return Some(trimmed.to_string());
    }
    // Bare «YYYY»
    if is_year(trimmed) {
        return Some(format!("{trimmed} жылы"));
    }
    None
}

fn is_year(s: &str) -> bool {
    !s.is_empty()
        && s.chars().all(|c| c.is_ascii_digit())
        && (1000..=2999).contains(&s.parse::<u32>().unwrap_or(0))
}

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
    // **v6.4.0-rc11 (2026-06-08 audit).**  Piper reads parenthetical
    // year ranges «(1991–2019)» literally as «open paren one nine
    // nine one dash …».  Spell them as date words so TTS sounds
    // natural.  Replaces «(YYYY–YYYY)» / «(YYYY-YYYY)» / «(YYYY)»
    // with «YYYY–YYYY жылдары» / «YYYY жылы» surface forms.
    let trimmed = preprocess_year_ranges_for_tts(trimmed);
    let trimmed = trimmed.as_str();
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

/// **Phase 15g.B (2026-06-01)** — Zipf-aware fuzzy STT normaliser.
///
/// User directive: «Знание ~1000 слов покрывает до 80% разговорных
/// текстов. Чтобы наша модель знала их наизусть и могла мгновенно
/// заменить плохо расслышанные или невнятные слова.»
///
/// Pre-15g.B used a flat 3155-word list (`INTENT_VOCAB ∪
/// adam_lexicon_curated::full_lexicon()`) with no frequency
/// signal — so on ties fuzzy pulled STT outputs to whichever
/// canonical happened to be alphabetically first («даулет» →
/// «сәулет», «тауылар» → «тауарлар»). 15g.B fixes the root cause:
///
///   1. **Zipf-ranked vocabulary** (corpus top-1000 + ~70 explicit
///      overrides) — see [`zipf_vocab::ZipfVocab`]. Built from
///      cc100_kk + Wikipedia + Abai (3.48 M tokens). On ties
///      `best_match` prefers the canonical with higher Zipf rank.
///
///   2. **Context-aware named-entity skip** — when the previous
///      1-2 tokens contain a name-trigger («атым», «есімім»,
///      «менің атым»), the next token is left alone. Whisper-out
///      «менің атым даулет» no longer mangles «даулет» into a
///      hand-curated architecture term.
///
///   3. **Short-numeral alias pass** retained from 15f.3 (kept
///      pre-fuzzy because Whisper drops trailing «і»/«ы»/«ө» on
///      single-syllable numerals where alias is unambiguous).
///
/// Threshold 0.70 (similarity * (1 + zipf_bonus)) — see
/// [`zipf_vocab::ZipfVocab::best_match`].

// **Phase 15g.B (2026-06-01)** — `intent_vocab_static_legacy` is
// retained one release for `git log` archaeology / diff context;
// runtime path no longer touches it. Promoted overrides live in
// `zipf_vocab::OVERRIDES`. Drop after Phase 15g.B.1 ships.
#[allow(dead_code)]
const fn intent_vocab_static_legacy() -> &'static [&'static str] {
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
        // Place + geography (Phase 15g.A.1 — hot-path so fuzzy
        // doesn't pull «тауылар» (горы with stray ы) toward the
        // curated lexicon entry «тауарлар» (товары) when «таулар»
        // is the intended canonical. Geographic plurals also keep
        // questions like «Қазақстанда қандай {таулар/көлдер/...}
        // бар» routing through the right world-core domain.)
        "қазақстан",
        "алматы",
        "астана",
        "нұр-сұлтан",
        "тау",
        "таулар",
        "өзен",
        "өзендер",
        "көл",
        "көлдер",
        "теңіз",
        "теңіздер",
        "қала",
        "қалалар",
        "жер",
        "жерлер",
        "ауыл",
        "облыс",
        "облыстар",
        "ел",
        "елдер",
        "халық",
        "халықтар",
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

/// **Phase 15g.C.1 (2026-06-02)** — Whisper-Shirali token-split merge.
/// Shirali sometimes splits compound Kazakh plural / suffixed forms
/// across a space boundary that the dialog engine's substring
/// matchers don't recognise («көлдер» → «көл дер», «таулар» →
/// «тау лар», «қазақстан» → «қаз ас тан»). The merge pairs below
/// rejoin them BEFORE fuzzy / intent classification runs.
///
/// Each pair is `(split_form, merged_form)` — the substring is
/// safe to merge because none of the split forms is a real
/// Kazakh word collocation (e.g. nobody says «көл дер» as two
/// separate tokens).
fn merge_whisper_splits(text: &str) -> String {
    static SPLIT_MERGES: &[(&str, &str)] = &[
        ("көл дер", "көлдер"),
        ("тау лар", "таулар"),
        ("өзен дер", "өзендер"),
        ("теңіз дер", "теңіздер"),
        ("қала лар", "қалалар"),
        ("қаз ас тан", "қазақстан"),
        ("қазақ стан", "қазақстан"),
        ("қазас тан", "қазақстан"),
        ("таныс айық", "танысайық"),
        ("алды мен", "алдымен"),
    ];
    let mut out = text.to_string();
    for (split, merged) in SPLIT_MERGES {
        if out.contains(split) {
            out = out.replace(split, merged);
        }
    }
    out
}

fn fuzzy_normalise(text: &str, vocab: &zipf_vocab::ZipfVocab) -> String {
    // **Phase 15f.3 (retained 15g.B)** — Whisper's trailing-vowel
    // drop on 3-char numerals («екі»→«ек», «үш»→«уш», «төрт»→
    // «торт», «бес»→«бис», «алты»→«алт», «жеті»→«жет»). These
    // fall under the 0.70 gate even with the Zipf bonus, so an
    // unambiguous alias table fixes them BEFORE best_match runs.
    // Math-only — no legitimate Kazakh word is «ек» / «уш».
    static SHORT_NUMERAL_ALIAS: &[(&str, &str)] = &[
        // Base numerals — Whisper drops trailing short vowels.
        ("ек", "екі"),
        ("уш", "үш"),
        ("торт", "төрт"),
        ("бис", "бес"),
        ("алт", "алты"),
        ("жет", "жеті"),
        ("сегиз", "сегіз"),
        ("тогиз", "тоғыз"),
        // **Phase 15g.I (2026-06-01)** — math-context Whisper drift
        // observed across multiple live REPL sessions. These fall
        // outside the length-floor-5 fuzzy gate AND outside the
        // base-numeral alias above. Each replaces a Whisper-noisy
        // surface form with the canonical Kazakh math root that
        // `discourse::try_evaluate_kazakh_word_math` recognises.
        // Limited to math-unambiguous strings — none of these is a
        // legitimate Kazakh word in its own right.
        //
        // «тоғыз» (9) cases — Whisper drift Ұ↔О: «тұғыз», «туғыз».
        ("тұғыз", "тоғыз"),
        ("туғыз", "тоғыз"),
        ("тұғызды", "тоғызды"),
        ("туғызды", "тоғызды"),
        // «көбейт» (multiply imperative) — Whisper drift Ө→О and
        // /-эйт/ → /-эт/ slur drops the -й-.
        ("кубейт", "көбейт"),
        ("кубет", "көбейт"),
        ("көбет", "көбейт"),
        ("кубетынғыз", "көбейтіңіз"),
        ("кубетыңыз", "көбейтіңіз"),
        // «азайт» (subtract) — observed drift /ай/→/а/ in slurred
        // speech, and /айт/→/айд/ voicing of final consonant.
        ("азаид", "азайт"),
        ("азайд", "азайт"),
        // «бөл» (divide) — removed in v6.5.0-rc3 (2026-06-09).
        // The "бол" → "бөл" alias was too aggressive: «сау бол» (= "be
        // well", canonical Kazakh farewell) was rewritten to «сау бөл»,
        // which the math router then interpreted as a divide operator.
        // Live audit T36 (rc2) ended with adam answering "10" to a
        // farewell.  The math vocab still has the canonical «бөл» plus
        // «бөлу», «боль», «бөль», «бел», «бөлі» (since rc12) — users
        // who actually want to divide can pronounce «бөл» clearly, but
        // slurred «бол» now stays as the verb "to be" and routes to
        // Farewell as intended.
        // «жиырма» (20) — drift /жи/→/же/.
        ("жерма", "жиырма"),
        // Dative «-ке/-қа» suffixed forms that Whisper sometimes
        // mis-attaches («үшке» → «үш ке» split → drift). The
        // dative survives lexicon analysis as long as the root
        // matches a numeral.
        ("түртке", "төртке"),
        ("тортке", "төртке"),
    ];

    // We rebuild a per-word `previous_tokens` slice as we iterate
    // so context-aware skip (named-entity detection) sees the
    // ORIGINAL surface forms — not the rewritten ones. This matters
    // when «атым» itself was rewritten by a prior best_match call.
    let raw_tokens: Vec<String> = text.split_whitespace().map(|s| s.to_string()).collect();
    let mut out_tokens: Vec<String> = Vec::with_capacity(raw_tokens.len());

    for (idx, word) in raw_tokens.iter().enumerate() {
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
            out_tokens.push(word.to_string());
            continue;
        }
        let lower = core.to_lowercase();

        // 1. Explicit short-numeral alias (cheap O(1) per word).
        let mut aliased: Option<String> = None;
        for (drift, canonical) in SHORT_NUMERAL_ALIAS {
            if &lower == drift {
                aliased = Some((*canonical).to_string());
                break;
            }
        }
        if let Some(canonical) = aliased {
            out_tokens.push(format!("{leading}{canonical}{trailing}"));
            continue;
        }

        // 2. Already canonical → skip (cheap O(N) scan over the
        //    Zipf vocab; for the top-1000 case this is fast).
        if vocab.contains(&lower) {
            out_tokens.push(word.to_string());
            continue;
        }

        // 3. **Context-aware named-entity pass.** When the previous
        //    1-2 raw tokens contain a name-trigger («атым»,
        //    «есімім», «менің атым»), the current token is the
        //    proper name itself. Phase 15g.B.2 (2026-06-01) — we
        //    look it up in the Kazakh name DB (211 male + 141
        //    female canonical names) and:
        //       a) replace with the canonical-cased form if similarity
        //          ≥ 0.80 (e.g. «даулет» → «Даулет»);
        //       b) if no DB match, capitalise the first letter so the
        //          downstream dialog engine treats it as a proper noun
        //          rather than a common content word.
        //    Either way fuzzy NEVER rewrites the name to a different
        //    word — the bug in c2de5afa («даулет» → «сәулет») can't
        //    happen here, the only candidate pool is real names.
        let lookback: &[String] = if idx >= 2 {
            &raw_tokens[idx - 2..idx]
        } else {
            &raw_tokens[..idx]
        };
        if zipf_vocab::is_after_name_trigger(lookback) {
            let chosen = match vocab.best_name_match(&lower, 0.80) {
                Some(canonical) => canonical,
                None => {
                    // No DB match → capitalise the input as-is so
                    // the dialog engine's NamedAfter / IsA path can
                    // still pick it up as a name slot.
                    let mut chars = lower.chars();
                    match chars.next() {
                        Some(c) => c.to_uppercase().chain(chars).collect(),
                        None => lower.clone(),
                    }
                }
            };
            out_tokens.push(format!("{leading}{chosen}{trailing}"));
            continue;
        }

        // 4. Phonetic best_match.
        //    **15g.J.1 (2026-06-01)** — back to 0.80 threshold after
        //    LM v4 proved insufficient as gatekeeper (see
        //    `ZipfVocab::best_match` doc). Wide pool stays — the
        //    coverage helps when fuzzy DOES fire — but the bar is
        //    tight so it rarely fires on noisy short surfaces.
        if let Some((canonical, _score)) = vocab.best_match(&lower, 0.80) {
            if canonical != lower {
                out_tokens.push(format!("{leading}{canonical}{trailing}"));
                continue;
            }
        }
        out_tokens.push(word.to_string());
    }

    out_tokens.join(" ")
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
