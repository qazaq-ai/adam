// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `adam_briefing` — interactive REPL for the v6.10.0 safety-briefing
//! session engine.  Runs one OT/TB procedure end-to-end:
//! инструктаж → устный опрос → оценка → протокол.
//!
//! ## Usage
//!
//! ```sh
//! # List available procedure ids:
//! cargo run -p adam-dialog --bin adam_briefing -- --list
//!
//! # Run a session (text only):
//! cargo run -p adam-dialog --bin adam_briefing -- kk_metallurgy_loto_003
//!
//! # Run with Kazakh voice output (spoken prompts + verdict):
//! cargo run -p adam-dialog --bin adam_briefing -- --voice kk_metallurgy_loto_003
//! ```
//!
//! At each prompt type the worker's spoken answer (Kazakh); during
//! the instruction phase type any acknowledgement («түсінікті») to
//! advance.  The final protocol prints (and, with `--voice`, speaks)
//! the pass/fail verdict for the ОТ/ТБ ИТР to sign.
//!
//! ## Voice output
//!
//! Voice is a **front-end concern** — the [`BriefingSession`] engine
//! stays pure text-in/text-out, and this binary layers spoken output
//! on top via the existing [`adam_dialog::tts`] backends.  Any future
//! UI reuses the same engine + the same `TtsBackend::speak` path.
//!
//! `--voice` prefers the neural **Piper** backend with the bundled
//! Kazakh voice (`data/tts_models/kk_KZ-issai-high.onnx`); it needs
//! the `piper` CLI + an audio player (`afplay` / `aplay`) on `PATH`.
//! When Piper is unavailable it falls back to the OS synthesiser
//! (macOS `say` / Linux `espeak-ng`), then to silent no-op.
//! Audible output does NOT require the crate's `voice` feature (that
//! feature is for voice *input* / AEC) — Piper shells out to the
//! audio player either way.

use std::io::{self, Write};
use std::path::PathBuf;

use adam_dialog::briefing_session::BriefingSession;
use adam_dialog::procedure_loader::shared_procedures;
use adam_dialog::system_clock::{read_clock, tz_offset_secs_from_env};
use adam_dialog::tts::{NoOpTts, OsTtsBackend, PiperTtsBackend, TtsBackend};

/// Bundled Kazakh Piper voice, used when `--voice` is on and
/// `--tts-model` is not overridden.
const DEFAULT_KK_MODEL: &str = "data/tts_models/kk_KZ-issai-high.onnx";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() || args.iter().any(|a| a == "--help" || a == "-h") {
        eprintln!(
            "usage: adam_briefing [--voice] [--tts-backend piper|os] \
             [--tts-model <path>] [--tts-voice <name>] \
             [--worker <name>] [--operator <name>] <procedure_id> | --list\n\
             example: adam_briefing --voice --worker \"Асан Асанов\" \
             --operator \"ИТҚ Досжан\" kk_metallurgy_loto_003"
        );
        return;
    }

    if args.iter().any(|a| a == "--list") {
        let procs = shared_procedures();
        if procs.is_empty() {
            eprintln!("(no procedures loaded — is data/procedures present?)");
            return;
        }
        eprintln!("{} procedures:", procs.len());
        for p in procs {
            println!("{:34} {}", p.id, p.title_kk);
        }
        return;
    }

    let voice = args.iter().any(|a| a == "--voice");
    let backend_choice = flag_value(&args, "--tts-backend");
    let model = flag_value(&args, "--tts-model");
    let os_voice = flag_value(&args, "--tts-voice");
    let worker = flag_value(&args, "--worker");
    let operator = flag_value(&args, "--operator");

    // First non-flag argument is the procedure id.  Skip flag values.
    let id = positional_id(&args);
    let Some(id) = id else {
        eprintln!("no procedure id given. Run with --list to see available ids.");
        return;
    };

    let tts = build_tts(voice, backend_choice.as_deref(), model, os_voice.as_deref());
    run_session(&id, tts.as_ref(), worker.as_deref(), operator.as_deref());
}

/// Value following `flag` (e.g. `--tts-model foo` → `Some("foo")`).
fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

/// The procedure id — the first bare token that is neither a flag nor
/// a flag's value.
fn positional_id(args: &[String]) -> Option<String> {
    const VALUED_FLAGS: [&str; 5] = [
        "--tts-backend",
        "--tts-model",
        "--tts-voice",
        "--worker",
        "--operator",
    ];
    let mut skip_next = false;
    for (i, a) in args.iter().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }
        if a.starts_with("--") {
            if VALUED_FLAGS.contains(&a.as_str()) {
                skip_next = true;
            }
            continue;
        }
        let _ = i;
        return Some(a.clone());
    }
    None
}

/// Build the TTS backend for the session front-end.  Off → silent
/// no-op (identical to the pre-voice behaviour).  On → prefer Piper
/// (bundled Kazakh voice) and degrade gracefully to the OS
/// synthesiser, then no-op, printing which backend won.
fn build_tts(
    enabled: bool,
    backend_choice: Option<&str>,
    model: Option<String>,
    os_voice: Option<&str>,
) -> Box<dyn TtsBackend> {
    if !enabled {
        return Box::new(NoOpTts);
    }
    let want_os_only = backend_choice == Some("os");
    let model_path = PathBuf::from(model.unwrap_or_else(|| DEFAULT_KK_MODEL.to_string()));

    if !want_os_only {
        if let Some(piper) = PiperTtsBackend::detect(&model_path) {
            eprintln!("adam_briefing: voice on — {}", piper.describe());
            return Box::new(piper);
        }
        eprintln!(
            "adam_briefing: piper unavailable (needs `piper` CLI + audio player + \
             model at {}); falling back to OS voice.",
            model_path.display()
        );
    }
    if let Some(os) = OsTtsBackend::detect(os_voice) {
        eprintln!(
            "adam_briefing: voice on — {} (note: OS voice may not speak Kazakh well; \
             install `piper` for the bundled kk voice).",
            os.describe()
        );
        return Box::new(os);
    }
    eprintln!("adam_briefing: no usable voice backend found — running silent.");
    Box::new(NoOpTts)
}

/// Speak `text`, swallowing synth errors — a TTS hiccup must never
/// break or abort the briefing session.
fn say(tts: &dyn TtsBackend, text: &str) {
    if let Err(e) = tts.speak(text) {
        eprintln!("(tts error, continuing silently: {e})");
    }
}

/// Caller-injected protocol header: the context the deterministic
/// engine cannot own — wall-clock date/time (local KZ zone) and the
/// worker / operator (ИТҚ) identities.  Printed just above the
/// engine's `render_kk` body (which carries the tamper-evidence hash),
/// so together they form a signable допуск journal entry.
fn print_protocol_header(worker: Option<&str>, operator: Option<&str>) {
    let clock = read_clock(tz_offset_secs_from_env());
    let blank = "____________________";
    println!("──────────── ДОПУСК ХАТТАМАСЫ ────────────");
    println!(
        "Күні/уақыты: {:04}-{:02}-{:02} {:02}:{:02} (жергілікті)",
        clock.year, clock.month, clock.day, clock.hour, clock.minute
    );
    println!("Жұмысшы: {}", worker.unwrap_or(blank));
    println!("ИТҚ/оператор: {}", operator.unwrap_or(blank));
}

fn run_session(id: &str, tts: &dyn TtsBackend, worker: Option<&str>, operator: Option<&str>) {
    let Some(mut session) = BriefingSession::from_id(id) else {
        eprintln!("procedure `{id}` not found. Run with --list to see available ids.");
        return;
    };

    let intro = session.begin();
    println!("{intro}\n");
    say(tts, &intro);

    let stdin = io::stdin();
    loop {
        print!("> ");
        let _ = io::stdout().flush();
        let mut line = String::new();
        if stdin.read_line(&mut line).unwrap_or(0) == 0 {
            eprintln!("\n(input closed — session aborted before completion)");
            return;
        }
        let reply = session.advance(line.trim());
        if reply.done {
            // Stamp the caller-side header, then print the engine's
            // protocol body (feedback + render_kk with the integrity
            // hash) for the ИТР to read and sign.
            println!();
            print_protocol_header(worker, operator);
            println!("\n{}\n", reply.text);
            if let Some(p) = session.protocol() {
                let verdict = if p.admitted {
                    "Жұмысқа жіберілді."
                } else {
                    "Жіберілмеді, қайта нұсқаулық қажет."
                };
                say(
                    tts,
                    &format!(
                        "Тексеру аяқталды. {} сұрақтың {}-і дұрыс. {verdict}",
                        p.total, p.passed_count
                    ),
                );
            }
            tts.wait_until_done();
            break;
        }
        println!("\n{}\n", reply.text);
        say(tts, &reply.text);
    }
}
