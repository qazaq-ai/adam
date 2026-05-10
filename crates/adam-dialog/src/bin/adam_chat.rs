//! `adam-chat` — interactive REPL for the adam v4.0 Kazakh dialog pipeline.
//!
//! **Kazakh-only surface** (v1.1.0 revert). Input and output are both Kazakh.
//!
//! Capabilities at v4.0:
//!
//!   - **26 intents** — 25 conversational + Insult for polite non-engagement.
//!   - **Multi-turn session state** (`Conversation`): `name`, `age`, `city`,
//!     `occupation` persist across turns, feeding downstream templates.
//!   - **`{slot|features}` FST templates** with case / number / derivation /
//!     possessive feature tokens — no morphologically invalid form ever
//!     leaves the system.
//!   - **Retrieval fallback** (v1.6.0+) — when no intent matches, we rank
//!     the committed morpheme index by **overlap + pack_purity + length +
//!     loanword-density** and cite the top-1 sample verbatim with
//!     provenance.
//!   - **Session-aware framing** (v1.8.0) — when session has entities, the
//!     template wrapping the citation personalises automatically via
//!     `template_is_fillable`.
//!   - **Opt-in city composition** (v1.9.0, `--compose`) — rewrites city
//!     mentions inside retrieved quotes to the user's session city via
//!     FST feature-preserving synthesis. Biographical-year guarded.
//!   - **Adaptation marker** (v1.9.5) — when a swap happened, the response
//!     frame contains the «бейімд-» stem so the user can always distinguish
//!     a verbatim corpus quote from an adapted one.
//!   - **Rule-derived reasoning chains** (v2.7+) — when committed
//!     `facts.json` + `derived_facts.json` are present and the user probes
//!     a noun that matches a derivation, adam cites the chain (not a
//!     corpus quote) with the «байланыс-» trust marker. At v3.9.5 the
//!     reasoner has 5 active rules (R1 / R2 / R3 / R5 / R6 / R7); the
//!     latter two (LivesIn-via-PartOf, GoesTo-via-PartOf) turn
//!     city-level locations into country-level conclusions through
//!     curated `city PartOf country` chains.
//!   - **World Core curated knowledge** (v3.9.0+) — `data/world_core/*.jsonl`
//!     entries reviewed by `shaman` are merged into `facts.json` with
//!     `ConfidenceKind::HumanApproved`. At v3.9.5: **200 entries / 270
//!     facts** across 6 domains (astronomy, time, geography_kz,
//!     biology_basic, body_parts, society).
//!   - **Dialog closed-class sync** (v3.9.5) — `NOT_A_TOPIC` mirrors
//!     `adam_reasoning::patterns::is_closed_class`, so interrogatives
//!     like «Неліктен?» are correctly treated as function-word input
//!     rather than noun+ablative topics.
//!
//! Architecture reference: [`docs/architecture_v3.md`](../../../docs/architecture_v3.md).
//!
//! Usage:
//!   adam_chat                    — REPL with retrieval on
//!   adam_chat --once "сәлем"     — single-shot, print response (+ trace)
//!   adam_chat --trace            — REPL with full per-turn trace
//!   adam_chat --no-retrieval     — skip retrieval (v1.1.0 behaviour)
//!   adam_chat --compose          — opt into InSampleCitySwap composition
//!   adam_chat --safe             — **investor-safe reasoning mode**
//!                                  (v4.0.3): cite only derivations whose
//!                                  entire `source_chain` is rooted in
//!                                  `data/world_core/*.jsonl` (every
//!                                  supporting fact human-reviewed).
//!                                  Alias: `--curated-only`. Mirrors
//!                                  `adam_demo`'s default since v4.0.2.

use std::{
    io::{self, BufRead, Write},
    process::ExitCode,
    thread,
};

use adam_dialog::{ComposeMode, Conversation, TemplateRepository, tts::TtsBackend};
use adam_kernel_fst::lexicon::LexiconV1;
use adam_kernel_fst::root_affinity::RootAffinity;
use adam_kernel_fst::suffix_priors::SuffixPriors;
use adam_reasoning::{Fact as ReasFact, reasoner::DerivedFact};
use adam_retrieval::MorphemeIndex;

const RETRIEVAL_INDEX_PATH: &str = "data/retrieval/morpheme_index.json";
const FACTS_PATH: &str = "data/retrieval/facts.json";
const DERIVED_FACTS_PATH: &str = "data/retrieval/derived_facts.json";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let trace = args.iter().any(|a| a == "--trace");
    let no_retrieval = args.iter().any(|a| a == "--no-retrieval");
    let compose = args.iter().any(|a| a == "--compose");
    // v4.0.3 — investor-safe chat mode. When `--safe` (or the
    // longer alias `--curated-only`) is passed, `inject_reasoning_chain`
    // only cites derivations whose full `source_chain` comes from
    // human-reviewed World Core entries. Mirrors the `adam_demo` Part 4
    // investor-safe default added in v4.0.2.
    let safe = args.iter().any(|a| a == "--safe" || a == "--curated-only");
    // **v5.0.0** — TTS output transducer. `--tts` activates a
    // system-native voice synthesiser (macOS `say` / Linux
    // `espeak-ng`) so adam speaks every response in addition to
    // printing it. `--tts-voice <name>` overrides voice detection.
    // **v5.1.0** — `--tts-backend piper --tts-model <path>` opts
    // into the optional Piper neural backend.
    let tts_enabled = args.iter().any(|a| a == "--tts");
    let tts_voice = args
        .windows(2)
        .find(|w| w[0] == "--tts-voice")
        .map(|w| w[1].clone());
    let tts_backend_choice = args
        .windows(2)
        .find(|w| w[0] == "--tts-backend")
        .map(|w| w[1].clone());
    let tts_model = args
        .windows(2)
        .find(|w| w[0] == "--tts-model")
        .map(|w| w[1].clone());

    // **v5.14.0 (V0).** Voice-input transducer flag. Push-to-talk
    // capture + whisper.cpp shell-out → STT → text → kernel. The
    // flag is recognised regardless of `--features voice`; without
    // the feature flag set at build time the binary prints a
    // build-help line and exits cleanly so the architectural
    // boundary stays visible. Companion flags:
    //   --whisper-bin <path>     override env ADAM_WHISPER_BIN
    //   --whisper-model <path>   GGML model file
    //   --whisper-language <kk>  Whisper language code (defaults to kk)
    let voice_input = args.iter().any(|a| a == "--voice-input");
    let whisper_bin_arg = args
        .windows(2)
        .find(|w| w[0] == "--whisper-bin")
        .map(|w| w[1].clone());
    let whisper_model_arg = args
        .windows(2)
        .find(|w| w[0] == "--whisper-model")
        .map(|w| w[1].clone());
    let whisper_language_arg = args
        .windows(2)
        .find(|w| w[0] == "--whisper-language")
        .map(|w| w[1].clone());

    // **v5.6.0** — parallel cold-start load. Heavy I/O resources
    // (retrieval index ~18 MB, derived facts ~22 MB, root affinity
    // ~27 MB, suffix priors ~2.4 MB, world_core directory) are
    // spawned on independent threads so total cold-disk time
    // approaches `max(individual)` instead of `sum`. Pre-v5.6.0
    // sequential loading ran ~1.2 s on a cold filesystem; post-
    // v5.6.0 parallel loading drops that to ~400-500 ms. p50 turn
    // latency unchanged because we still join all threads before
    // accepting input — there is no lazy-load delay on the first
    // user query.
    let retrieval_handle = if no_retrieval {
        None
    } else {
        Some(thread::spawn(load_retrieval_index))
    };
    let reasoning_handle = thread::spawn(load_reasoning_chains);
    let priors_handle = thread::spawn(|| {
        let priors_path = std::path::Path::new("data/retrieval/suffix_chain_priors.json");
        if priors_path.exists() {
            SuffixPriors::load(priors_path).ok()
        } else {
            None
        }
    });
    let affinity_handle = thread::spawn(|| {
        let affinity_path = std::path::Path::new("data/retrieval/root_affinity.json");
        if affinity_path.exists() {
            RootAffinity::load(affinity_path).ok()
        } else {
            None
        }
    });
    let world_core_handle = thread::spawn(|| {
        let world_core_dir = std::path::Path::new("data/world_core");
        adam_reasoning::world_core::load_world_core_dir(world_core_dir).ok()
    });

    // Lexicon + templates load on the main thread — they're small
    // (~1 MB combined) and required before any progress prints make
    // sense.
    let lex = match LexiconV1::load_default() {
        Ok(l) => l,
        Err(e) => {
            eprintln!("cannot load lexicon: {e}");
            return ExitCode::FAILURE;
        }
    };

    let repo = match TemplateRepository::load_default() {
        Ok(r) => {
            eprintln!(
                "adam-chat: loaded {} template families from data/dialog/templates/v1.toml",
                r.len()
            );
            r
        }
        Err(e) => {
            eprintln!("warning: using hardcoded fallback templates ({e})");
            TemplateRepository::hardcoded_fallback()
        }
    };

    // Join the parallel loads in the same logical order as pre-v5.6.0
    // so the progress-print sequence is byte-identical for users
    // running with redirected stderr (CI logs / replay harnesses).
    let index = retrieval_handle.and_then(|h| h.join().ok()).flatten();
    if no_retrieval {
        // explicit no-op — keep symmetry with pre-v5.6.0 silence
    } else {
        match &index {
            Some(idx) => {
                eprintln!(
                    "adam-chat: retrieval on — {} morphemes, {} postings, {} indexed samples",
                    idx.unique_morphemes, idx.total_postings, idx.samples_indexed
                );
            }
            None => {
                eprintln!(
                    "adam-chat: retrieval index not found at {RETRIEVAL_INDEX_PATH} — falling back to v1.1.0 noun-echo"
                );
            }
        }
    }

    let (extracted, derived) = reasoning_handle
        .join()
        .ok()
        .unwrap_or_else(|| (Vec::new(), Vec::new()));
    if !derived.is_empty() {
        eprintln!(
            "adam-chat: reasoning on — {} derived facts available ({} supporting extracted facts)",
            derived.len(),
            extracted.len(),
        );
    }

    let mut conv = Conversation::new();
    if let Some(idx) = index {
        conv = conv.with_morpheme_index(idx);
    }
    if compose {
        conv = conv.with_compose_mode(ComposeMode::InSampleCitySwap);
        eprintln!(
            "adam-chat: compose mode = InSampleCitySwap (v1.9.0 opt-in; adapted quotes marked with «бейімд-»)"
        );
    }
    if safe {
        conv = conv.with_curated_only_reasoning(true);
        eprintln!(
            "adam-chat: --safe mode — reasoning chains filtered to fully-curated (world_core-only) source chains"
        );
    }
    if !derived.is_empty() || !extracted.is_empty() {
        conv = conv.with_reasoning_chains(extracted, derived);
    }

    // **v5.6.0** — domain index built from the world_core load
    // dispatched in parallel above. Build (Vec → HashMap) is
    // single-threaded because the API is non-Send across the
    // intermediate `report.entries` map.
    let domain_idx = match world_core_handle.join().ok().flatten() {
        Some(report) => {
            let entries: Vec<_> = report.entries.into_iter().map(|(e, _)| e).collect();
            let idx = adam_dialog::DomainIndex::build(&entries);
            eprintln!(
                "adam-chat: domain index — {} topics indexed across world_core",
                idx.len()
            );
            idx
        }
        None => {
            eprintln!(
                "adam-chat: world_core load failed; domain index empty (no current_domain inference)"
            );
            adam_dialog::DomainIndex::empty()
        }
    };
    conv = conv.with_domain_index(domain_idx);

    // **v5.6.0** — suffix priors loaded in parallel above. Print +
    // attach here. The Jelinek-Mercer α env override is read on
    // the main thread (unchanged from v4.16.5).
    if let Some(priors) = priors_handle.join().ok().flatten() {
        let n_chains = priors.len();
        let n_bigrams: usize = priors
            .transition_log_prob
            .values()
            .map(|row| row.len())
            .sum();
        let n_tokens = priors.trained_on_tokens;
        eprintln!(
            "adam-chat: suffix priors — {n_chains} chains, {n_bigrams} bigrams over {n_tokens} training tokens"
        );
        conv = conv.with_suffix_priors(priors);
        let alpha = std::env::var("ADAM_PRIORS_ALPHA")
            .ok()
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.3);
        eprintln!("adam-chat: priors alpha = {alpha} (Jelinek-Mercer)");
        conv = conv.with_priors_alpha(alpha);
    }

    // **v5.6.0** — root affinity loaded in parallel above. Print +
    // attach here. Same v4.29.5 SearchGraph reranking semantics.
    if let Some(affinity) = affinity_handle.join().ok().flatten() {
        let n_roots = affinity.root_log_prob.len();
        let n_pairs: usize = affinity.pair_pmi.values().map(|row| row.len()).sum();
        let n_samples = affinity.trained_on_samples;
        eprintln!(
            "adam-chat: root affinity — {n_roots} roots, {n_pairs} pairs over {n_samples} training samples"
        );
        conv = conv.with_root_affinity(affinity);
    }

    // **v5.0.0** — TTS backend init. When `--tts` is on, detect a
    // system-native voice synthesiser (macOS `say` / Linux
    // `espeak-ng`); when not, use the no-op backend so the call
    // sites stay symmetric.
    // **v5.1.0** — `--tts-backend piper` opts into the neural Piper
    // backend (requires `--tts-model <path>` to point at an ONNX
    // voice file). Falls back to the OS backend on detection
    // failure to keep the experience consistent.
    let tts_box: Box<dyn adam_dialog::tts::TtsBackend> = if tts_enabled {
        let chosen = match tts_backend_choice.as_deref() {
            Some("piper") => {
                let model = tts_model.as_ref().map(std::path::PathBuf::from);
                match model
                    .as_deref()
                    .and_then(adam_dialog::tts::PiperTtsBackend::detect)
                {
                    Some(backend) => {
                        eprintln!("adam-chat: TTS on — {}", backend.describe());
                        Some(Box::new(backend) as Box<dyn adam_dialog::tts::TtsBackend>)
                    }
                    None => {
                        eprintln!(
                            "adam-chat: --tts-backend piper requested but `piper` binary, audio player, or model not found; falling back to OS backend"
                        );
                        None
                    }
                }
            }
            _ => None,
        };
        chosen.unwrap_or_else(|| {
            match adam_dialog::tts::OsTtsBackend::detect(tts_voice.as_deref()) {
                Some(backend) => {
                    eprintln!("adam-chat: TTS on — {}", backend.describe());
                    Box::new(backend)
                }
                None => {
                    eprintln!(
                        "adam-chat: --tts requested but no system synthesiser found; falling back to silent text"
                    );
                    Box::new(adam_dialog::tts::NoOpTts)
                }
            }
        })
    } else {
        Box::new(adam_dialog::tts::NoOpTts)
    };
    let tts_handle: Option<&dyn adam_dialog::tts::TtsBackend> =
        if tts_enabled { Some(&*tts_box) } else { None };

    if let Some(pos) = args.iter().position(|a| a == "--once") {
        if let Some(input) = args.get(pos + 1) {
            run_turn(
                &mut conv,
                input,
                &lex,
                &repo,
                trace,
                turn_seed(0),
                tts_handle,
            );
            return ExitCode::SUCCESS;
        } else {
            eprintln!("--once requires an argument");
            return ExitCode::FAILURE;
        }
    }

    eprintln!(
        // **v5.11.5 — Codex follow-up review (B5.5).** Banner version
        // pulled from `CARGO_PKG_VERSION` so the printed line tracks
        // the workspace version automatically. Pre-v5.11.5 the banner
        // was hard-coded as "v5.2" and drifted from the workspace
        // (5.11.0 at the time of the audit) — bad signal for an
        // investor / demo session.
        "adam-chat v{} — пікірлесейік! Қазақ тілінде сөйлесейік; ^D to quit.\n\
         Multi-line code blocks: open with ``` and close with ``` on its own line.\n\
         Voice output: pass --tts (default OS voice; or --tts-backend piper --tts-model <path>).\n\
         Voice input (build with --features voice): pass --voice-input (push-to-talk + whisper.cpp shell-out).",
        env!("CARGO_PKG_VERSION")
    );
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut turn = 0u64;
    // **v4.97.0** — multi-line code-block accumulator (Codex round-2
    // Bug 3). Pre-v4.97.0 every newline ended the turn, so a Rust
    // SubmitSolution like
    //
    //     ```rust
    //     fn main() { println!("hi"); }
    //     ```
    //
    // had to be entered as a single line with `\n` escapes. Now: when
    // a line introduces an unclosed ``` fence, accumulate subsequent
    // lines into a buffer and only fire the turn when the closing
    // fence appears. The fence-count parity (odd → still inside a
    // block; even → block closed) governs the flush.
    // **v5.14.0 (V0).** Voice-input REPL branch. Push-to-talk: user
    // hits Enter, mic records up to 30 s (or until the next Enter
    // closes the cpal stream cleanly), whisper.cpp transcribes, the
    // text path takes over from here. The text REPL below is the
    // default fall-through.
    if voice_input {
        return run_voice_repl(
            &mut conv,
            &lex,
            &repo,
            trace,
            &mut turn,
            tts_handle,
            whisper_bin_arg.as_deref(),
            whisper_model_arg.as_deref(),
            whisper_language_arg.as_deref(),
        );
    }

    let mut block_buf: Option<String> = None;
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if let Some(assembled) = absorb_line(&line, &mut block_buf) {
            turn += 1;
            let seed = turn_seed(turn);
            run_turn(&mut conv, &assembled, &lex, &repo, trace, seed, tts_handle);
            stdout.lock().flush().ok();
        }
    }
    ExitCode::SUCCESS
}

/// **v5.14.0 (V0).** Voice-input REPL — push-to-talk capture +
/// whisper.cpp shell-out + standard kernel turn. Without
/// `--features voice` this prints a build-help line and exits so the
/// boundary stays visible (no silent fall-through to the text path —
/// the user explicitly asked for voice).
#[cfg(feature = "voice")]
#[allow(clippy::too_many_arguments)]
fn run_voice_repl(
    conv: &mut Conversation,
    lex: &LexiconV1,
    repo: &TemplateRepository,
    trace: bool,
    turn: &mut u64,
    tts_handle: Option<&dyn adam_dialog::tts::TtsBackend>,
    whisper_bin_arg: Option<&str>,
    whisper_model_arg: Option<&str>,
    whisper_language_arg: Option<&str>,
) -> ExitCode {
    use adam_voice::{MicCapture, MicConfig, WhisperCli, write_wav};

    let bin: std::path::PathBuf = match whisper_bin_arg
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os(adam_voice::stt::ADAM_WHISPER_BIN_ENV).map(Into::into))
    {
        Some(p) => p,
        None => {
            eprintln!(
                "adam-chat --voice-input: whisper binary not configured. \
                 Set ADAM_WHISPER_BIN env var or pass --whisper-bin <path>."
            );
            return ExitCode::FAILURE;
        }
    };
    if !bin.exists() {
        eprintln!(
            "adam-chat --voice-input: whisper binary not found at {bin:?} — \
             install whisper.cpp (e.g. `brew install whisper-cpp`) and re-run."
        );
        return ExitCode::FAILURE;
    }
    let mut cli = WhisperCli::new(&bin);
    if let Some(m) = whisper_model_arg {
        cli = cli.with_model(m);
    }
    if let Some(lang) = whisper_language_arg {
        cli = cli.with_language(lang);
    }

    eprintln!(
        "adam-chat --voice-input: push-to-talk active. Press Enter to record \
         (up to 30 s), then Enter again to stop. ^D to quit."
    );
    let stdin = io::stdin();
    let mut prompt_line = String::new();
    loop {
        prompt_line.clear();
        eprint!("[voice] press Enter to record … ");
        io::stderr().lock().flush().ok();
        if stdin.lock().read_line(&mut prompt_line).is_err() || prompt_line.is_empty() {
            return ExitCode::SUCCESS;
        }
        let cap = match MicCapture::start(MicConfig::default()) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[voice] mic start failed: {e}");
                continue;
            }
        };
        eprint!("[voice] recording … press Enter to stop");
        io::stderr().lock().flush().ok();
        let mut stop_line = String::new();
        let _ = stdin.lock().read_line(&mut stop_line);
        let samples = match cap.stop() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[voice] mic stop failed: {e}");
                continue;
            }
        };
        let tmp_dir = std::env::temp_dir();
        let wav_path = tmp_dir.join(format!("adam_voice_turn_{}.wav", *turn + 1));
        if let Err(e) = write_wav(&samples, &wav_path) {
            eprintln!("[voice] wav write failed: {e}");
            continue;
        }
        let transcript = match cli.transcribe(&wav_path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("[voice] whisper transcribe failed: {e}");
                continue;
            }
        };
        let _ = std::fs::remove_file(&wav_path);
        let _ = std::fs::remove_file(format!("{}.txt", wav_path.display()));
        if transcript.text.trim().is_empty() {
            eprintln!("[voice] empty transcript — try again.");
            continue;
        }
        eprintln!("[voice] heard: «{}»", transcript.text);
        *turn += 1;
        let seed = turn_seed(*turn);
        run_turn(conv, &transcript.text, lex, repo, trace, seed, tts_handle);
        io::stdout().lock().flush().ok();
    }
}

#[cfg(not(feature = "voice"))]
#[allow(clippy::too_many_arguments)]
fn run_voice_repl(
    _conv: &mut Conversation,
    _lex: &LexiconV1,
    _repo: &TemplateRepository,
    _trace: bool,
    _turn: &mut u64,
    _tts_handle: Option<&dyn adam_dialog::tts::TtsBackend>,
    _whisper_bin_arg: Option<&str>,
    _whisper_model_arg: Option<&str>,
    _whisper_language_arg: Option<&str>,
) -> ExitCode {
    eprintln!(
        "adam-chat --voice-input requires the `voice` feature. \
         Rebuild with `cargo build --features voice -p adam-dialog --bin adam_chat`."
    );
    ExitCode::FAILURE
}

/// Drive the multi-line code-block accumulator state machine for one
/// input line.
///
/// `block_buf` holds the in-progress code block (None when not inside
/// one). Returns `Some(turn_input)` when a complete utterance is ready
/// to send to `run_turn`, else `None` (line was empty or buffering
/// continues).
///
/// Parity rule: a buffer with **odd** fence count is open; **even**
/// closes it. Inline blocks (one line containing both opening and
/// closing fences) always have even count and bypass the accumulator.
fn absorb_line(raw_line: &str, block_buf: &mut Option<String>) -> Option<String> {
    // Trim trailing whitespace only — leading whitespace inside a
    // code block is significant (indentation).
    let line = raw_line.trim_end();
    if let Some(buf) = block_buf.as_mut() {
        buf.push('\n');
        buf.push_str(line);
        if count_fences(buf) % 2 == 0 {
            let assembled = std::mem::take(buf);
            *block_buf = None;
            return Some(assembled.trim().to_string());
        }
        return None;
    }
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    if count_fences(trimmed) % 2 == 1 {
        // Block opens but doesn't close on this line — start buffering.
        *block_buf = Some(line.to_string());
        return None;
    }
    Some(trimmed.to_string())
}

/// Count occurrences of the ` ``` ` markdown code-fence token. Used by
/// the v4.97.0 multi-line accumulator to decide when a code block has
/// closed (parity flips even).
fn count_fences(s: &str) -> usize {
    s.matches("```").count()
}

/// **v5.6.0** — stream-parse JSON via a buffered `File` reader so the
/// raw file contents are never held as a 27 MB / 18 MB / 22 MB Rust
/// `String` in addition to the parsed struct. Pre-v5.6.0
/// `read_to_string` + `from_str` peaked at `len(file) + sizeof(struct)`;
/// `BufReader::new(File::open)` + `from_reader` peaks closer to
/// `sizeof(struct)`. Saves ~70 MB peak across the three large
/// artefacts (root_affinity / morpheme_index / derived_facts).
fn load_retrieval_index() -> Option<MorphemeIndex> {
    let file = std::fs::File::open(RETRIEVAL_INDEX_PATH).ok()?;
    let reader = std::io::BufReader::new(file);
    let mut idx: MorphemeIndex = serde_json::from_reader(reader).ok()?;
    idx.refresh_stats();
    Some(idx)
}

/// Load the v2.7 reasoning artefacts — `facts.json` + `derived_facts.json`.
/// Silently returns empty vectors for any missing / malformed file so
/// embedders running in trimmed checkouts get v2.6-identical behaviour.
fn load_reasoning_chains() -> (Vec<ReasFact>, Vec<DerivedFact>) {
    #[derive(serde::Deserialize)]
    struct FactsFile {
        facts: Vec<ReasFact>,
    }
    #[derive(serde::Deserialize)]
    struct DerivedFile {
        derived: Vec<DerivedFact>,
    }
    let extracted = std::fs::File::open(FACTS_PATH)
        .ok()
        .map(std::io::BufReader::new)
        .and_then(|reader| serde_json::from_reader::<_, FactsFile>(reader).ok())
        .map(|f| f.facts)
        .unwrap_or_default();
    let derived = std::fs::File::open(DERIVED_FACTS_PATH)
        .ok()
        .map(std::io::BufReader::new)
        .and_then(|reader| serde_json::from_reader::<_, DerivedFile>(reader).ok())
        .map(|d| d.derived)
        .unwrap_or_default();
    (extracted, derived)
}

fn run_turn(
    conv: &mut Conversation,
    input: &str,
    lex: &LexiconV1,
    repo: &TemplateRepository,
    trace: bool,
    seed: u64,
    tts: Option<&dyn adam_dialog::tts::TtsBackend>,
) {
    if trace {
        // v4.0.25 — trace through the REAL runtime path via
        // `turn_with_trace`. Pre-v4.0.25 this branch manually
        // re-implemented turn() but stopped before
        // `inject_retrieval_example` / `inject_reasoning_chain`, so
        // `--trace` output was materially false for v4.0.20+ features
        // (Codex v4.0.23 re-review #2). Now trace prints the
        // post-injection intent — the exact state the planner saw.
        let (out, trace) = conv.turn_with_trace(input, lex, repo, seed);
        println!("┌─ input:    {input}");
        println!("├─ parses:   {:#?}", trace.parses);
        // **v4.31.5** — first SemFrame consumer migration. Render
        // the unified morphemic-logical IR per parse. Compact one-
        // line-per-frame format: `{root}/{pos} case={case?}
        // tense={tense?} polarity={polarity:?} evidence={ev?}`. Any
        // feature that's `None` is omitted to keep the line scannable
        // — most frames have only 2-3 non-None features. The full
        // `Debug` form is reachable via `trace.sem_frames` from a
        // calling Rust consumer; this is the human-readable variant.
        if trace.sem_frames.is_empty() {
            println!("├─ sem_frames: (none)");
        } else {
            println!("├─ sem_frames:");
            for (i, frame) in trace.sem_frames.iter().enumerate() {
                let mut features = Vec::new();
                if let Some(c) = frame.case {
                    features.push(format!("case={c:?}"));
                }
                if let Some(n) = frame.number {
                    features.push(format!("number={n:?}"));
                }
                if let Some(p) = frame.possessive {
                    features.push(format!("poss={p:?}"));
                }
                if let Some(p) = frame.predicate {
                    features.push(format!("pred={p:?}"));
                }
                if let Some(d) = frame.derivation {
                    features.push(format!("deriv={d:?}"));
                }
                if let Some(t) = frame.tense {
                    features.push(format!("tense={t:?}"));
                }
                if let Some(p) = frame.person {
                    features.push(format!("person={p:?}"));
                }
                if let Some(v) = frame.voice {
                    features.push(format!("voice={v:?}"));
                }
                if frame.polarity != adam_kernel_fst::Polarity::Affirmative {
                    features.push(format!("polarity={:?}", frame.polarity));
                }
                if frame.polite {
                    features.push("polite".into());
                }
                if let Some(m) = frame.modality {
                    features.push(format!("mod={m:?}"));
                }
                if let Some(e) = frame.evidence {
                    features.push(format!("evid={e:?}"));
                }
                if let Some(r) = frame.relation {
                    features.push(format!("rel={r:?}"));
                }
                let suffix = if features.is_empty() {
                    String::new()
                } else {
                    format!(" {}", features.join(" "))
                };
                println!("│   [{i}] {}/{:?}{suffix}", frame.root, frame.pos);
            }
        }
        println!("├─ intent:   {:?}", trace.intent_after_injection);
        println!("├─ session:  {:?}", trace.session_snapshot);
        // v4.0.27 — belief snapshot (Codex v4.0.26 roadmap Phase 1).
        let d = trace.belief_digest;
        println!(
            "├─ belief:   entities={} facts={} active={} contested={} pending={} conflicts={}",
            d.entities,
            d.facts_total,
            d.facts_active,
            d.facts_contested,
            d.pending_questions,
            d.contradictions
        );
        if !trace.belief_snapshot.contradictions.is_empty() {
            for c in &trace.belief_snapshot.contradictions {
                println!(
                    "├─ belief conflict: {} {}: fact[{}] vs fact[{}] @ turn {}",
                    c.subject, c.predicate, c.fact_a_index, c.fact_b_index, c.detected_at_turn
                );
            }
        }
        // v4.0.29 — task layer snapshot (Codex Phase 2).
        let td = trace.task_digest;
        println!(
            "├─ task:     goal={} variant={} subgoals={} status={:?} set_at={:?}",
            td.has_goal,
            td.goal_variant.unwrap_or("—"),
            td.subgoals,
            td.status,
            td.goal_age_turns
        );
        // v4.0.31 — action planner (Codex Phase 3).
        let ad = trace.action_digest;
        println!(
            "├─ action:   {:?} → {:?} (rationale×{})",
            ad.action, ad.expected_output, ad.rationale_count
        );
        for r in &trace.action_plan.rationale {
            println!("├─ action rationale: {r}");
        }
        // v4.0.32 — verifier (Codex Phase 4).
        println!(
            "├─ verify:   supported={} evidence={} issues={:?}",
            trace.verification.supported,
            trace.verification.evidence_count,
            trace.verification.issues
        );
        if !trace.verification.supported {
            println!("├─ verify:   GATE fired — evidence stripped before rendering");
        }
        // v4.0.33 — epistemic status (Codex Phase 5 part 1).
        println!("├─ epistem:  {:?}", trace.epistemic_status);
        // v4.0.37 substrate / v4.0.38 audit-mode dispatch.
        if trace.tool_calls.is_empty() {
            println!("├─ tools:    none dispatched");
        } else {
            println!("├─ tools:    {} audit call(s)", trace.tool_calls.len());
            for r in &trace.tool_calls {
                let tag = match &r.call {
                    adam_dialog::ToolCall::SearchBelief { subject, predicate } => {
                        format!("SearchBelief({subject}, {predicate:?})")
                    }
                    adam_dialog::ToolCall::SearchGraph { subject, predicate } => {
                        format!("SearchGraph({subject}, {predicate:?})")
                    }
                    adam_dialog::ToolCall::SearchRetrieval { morphemes } => {
                        format!("SearchRetrieval({} morphemes)", morphemes.len())
                    }
                    adam_dialog::ToolCall::RunLocalReasoner {
                        topic,
                        curated_only,
                    } => {
                        format!("RunLocalReasoner({topic}, curated_only={curated_only})")
                    }
                };
                println!(
                    "├─ tool: {tag} success={} findings={}",
                    r.success,
                    r.findings.len()
                );
            }
        }
        for t in &trace.plan_trace {
            println!("├─ {t}");
        }
        println!("└─ output:   {out}");
        if let Some(tts) = tts
            && let Err(e) = tts.speak(&out)
        {
            eprintln!("[tts] speak failed: {e}");
        }
    } else {
        let out = conv.turn(input, lex, repo, seed);
        println!("{out}");
        if let Some(tts) = tts
            && let Err(e) = tts.speak(&out)
        {
            eprintln!("[tts] speak failed: {e}");
        }
    }
}

fn turn_seed(turn: u64) -> u64 {
    let mut s = turn.wrapping_mul(0x9E3779B97F4A7C15);
    s ^= s >> 33;
    s = s.wrapping_mul(0xFF51AFD7ED558CCD);
    s ^= s >> 33;
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_fences_zero_in_plain_text() {
        assert_eq!(count_fences("сәлем"), 0);
        assert_eq!(count_fences(""), 0);
    }

    #[test]
    fn count_fences_one_in_opening_line() {
        assert_eq!(count_fences("```rust"), 1);
        assert_eq!(count_fences("```"), 1);
    }

    #[test]
    fn count_fences_two_in_inline_block() {
        // Bug 4 / v4.96.0 single-line case: ```rust X```
        assert_eq!(count_fences("```rust let x = 5; ```"), 2);
    }

    #[test]
    fn count_fences_two_for_full_multiline_block() {
        let buf = "```rust\nfn main() {}\n```";
        assert_eq!(count_fences(buf), 2);
    }

    /// Parity invariant: an in-progress (still-open) block always has
    /// odd fence count, a closed one always even. This is the contract
    /// the REPL accumulator relies on.
    #[test]
    fn fence_parity_governs_block_state() {
        // Just opened — odd → still buffering.
        assert_eq!(count_fences("```rust") % 2, 1);
        // After one body line — still odd.
        let mut buf = String::from("```rust\nfn main() {}");
        assert_eq!(count_fences(&buf) % 2, 1);
        // Closing fence appended — even → ready to flush.
        buf.push_str("\n```");
        assert_eq!(count_fences(&buf) % 2, 0);
    }

    /// Drive the full multi-line state machine via `absorb_line` to
    /// verify the open-→-buffer-→-close-→-flush sequence.
    #[test]
    fn absorb_line_assembles_multiline_block() {
        let mut buf: Option<String> = None;
        // Opening fence — buffers, no flush.
        assert!(absorb_line("```rust", &mut buf).is_none());
        assert!(buf.is_some());
        // Body line — still buffering.
        assert!(absorb_line("fn main() {", &mut buf).is_none());
        assert!(absorb_line("    println!(\"hi\");", &mut buf).is_none());
        assert!(absorb_line("}", &mut buf).is_none());
        // Closing fence — flushes complete block.
        let assembled = absorb_line("```", &mut buf).expect("block should flush");
        assert!(buf.is_none(), "buffer must be empty after flush");
        assert!(assembled.starts_with("```rust"), "got: {assembled:?}");
        assert!(assembled.ends_with("```"), "got: {assembled:?}");
        assert!(assembled.contains("println!"), "got: {assembled:?}");
    }

    /// A single line that contains BOTH fences (inline block, the
    /// v4.96.0 Bug 4 case) bypasses the accumulator.
    #[test]
    fn absorb_line_inline_block_no_buffering() {
        let mut buf: Option<String> = None;
        let assembled = absorb_line("```rust let x = 5; ```", &mut buf)
            .expect("inline block must flush immediately");
        assert!(buf.is_none(), "no buffer for inline block");
        assert_eq!(assembled, "```rust let x = 5; ```");
    }

    /// Non-code lines outside a buffer flush as-is.
    #[test]
    fn absorb_line_plain_text_passthrough() {
        let mut buf: Option<String> = None;
        let assembled =
            absorb_line("Сәлеметсіз бе.", &mut buf).expect("plain text must flush immediately");
        assert!(buf.is_none());
        assert_eq!(assembled, "Сәлеметсіз бе.");
    }

    /// Empty / whitespace-only lines outside a buffer are skipped.
    #[test]
    fn absorb_line_empty_lines_skipped_outside_buffer() {
        let mut buf: Option<String> = None;
        assert!(absorb_line("", &mut buf).is_none());
        assert!(absorb_line("   ", &mut buf).is_none());
        assert!(buf.is_none());
    }

    /// Empty lines INSIDE a code block are preserved (a Rust function
    /// can have blank separator lines).
    #[test]
    fn absorb_line_empty_lines_inside_block_preserved() {
        let mut buf: Option<String> = None;
        absorb_line("```rust", &mut buf);
        absorb_line("fn a() {}", &mut buf);
        absorb_line("", &mut buf); // blank separator
        absorb_line("fn b() {}", &mut buf);
        let assembled = absorb_line("```", &mut buf).expect("close fence flushes");
        // Both functions and the blank separator must survive.
        assert!(assembled.contains("fn a()"));
        assert!(assembled.contains("fn b()"));
        // A blank line should appear between them.
        assert!(
            assembled.contains("\n\n") || assembled.matches('\n').count() >= 4,
            "expected blank-line separator preserved; got: {assembled:?}"
        );
    }
}
