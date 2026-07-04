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

use adam_dialog::briefing_seal::{SealContext, SealedProtocol};
use adam_dialog::briefing_session::{BriefingProtocol, BriefingSession};
use adam_dialog::procedure_loader::shared_procedures;
use adam_dialog::system_clock::{read_clock, tz_offset_secs_from_env};
use adam_dialog::tts::{NoOpTts, OsTtsBackend, PiperTtsBackend, TtsBackend};
use adam_seal::{SigningKey, generate_signing_key};

/// Bundled Kazakh Piper voice, used when `--voice` is on and
/// `--tts-model` is not overridden.
const DEFAULT_KK_MODEL: &str = "data/tts_models/kk_KZ-issai-high.onnx";

/// This build's engine version, stamped into every sealed protocol.
const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() || args.iter().any(|a| a == "--help" || a == "-h") {
        eprintln!(
            "usage:\n  \
             adam_briefing [--voice] [--tts-backend piper|os] [--tts-model <path>] \
             [--tts-voice <name>] [--worker <name>] [--operator <name>] \
             [--site <id>] [--sign-key <seed.key>] [--seal-out <sealed.json>] \
             <procedure_id>\n  \
             adam_briefing --list\n  \
             adam_briefing keygen [--out <seed.key>]\n  \
             adam_briefing verify <sealed.json> [--expect-key <pubhex>]\n\n\
             example: adam_briefing --worker \"Асан Асанов\" --operator \"ИТҚ Досжан\" \
             --sign-key operator.key --seal-out dopusk.json kk_metallurgy_loto_003"
        );
        return;
    }

    // Subcommands are dispatched on the first bare token.
    match args[0].as_str() {
        "keygen" => {
            run_keygen(&args);
            return;
        }
        "verify" => {
            run_verify(&args);
            return;
        }
        _ => {}
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
    let site = flag_value(&args, "--site");
    let sign_key = flag_value(&args, "--sign-key");
    let seal_out = flag_value(&args, "--seal-out");

    // First non-flag argument is the procedure id.  Skip flag values.
    let id = positional_id(&args);
    let Some(id) = id else {
        eprintln!("no procedure id given. Run with --list to see available ids.");
        return;
    };

    // Load the signing key up front so a bad key path fails before the
    // worker sits through a whole briefing.
    let signer = match sign_key.as_deref() {
        None => None,
        Some(path) => match load_signing_key(path) {
            Ok(k) => Some(k),
            Err(e) => {
                eprintln!("adam_briefing: cannot load --sign-key {path}: {e}");
                return;
            }
        },
    };

    let tts = build_tts(voice, backend_choice.as_deref(), model, os_voice.as_deref());
    let session_ctx = SessionContext {
        worker: worker.as_deref(),
        operator: operator.as_deref(),
        site: site.as_deref(),
        signer: signer.as_ref(),
        seal_out: seal_out.as_deref(),
    };
    run_session(&id, tts.as_ref(), &session_ctx);
}

/// Front-end context threaded into a briefing session: caller-side
/// identities plus the optional signing key / seal destination.
struct SessionContext<'a> {
    worker: Option<&'a str>,
    operator: Option<&'a str>,
    site: Option<&'a str>,
    signer: Option<&'a SigningKey>,
    seal_out: Option<&'a str>,
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
    const VALUED_FLAGS: [&str; 9] = [
        "--tts-backend",
        "--tts-model",
        "--tts-voice",
        "--worker",
        "--operator",
        "--site",
        "--sign-key",
        "--seal-out",
        "--expect-key",
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

/// `UTC±HH:MM` label for a timezone offset in seconds.
fn tz_label(offset_secs: i64) -> String {
    let sign = if offset_secs < 0 { '-' } else { '+' };
    let abs = offset_secs.unsigned_abs();
    format!("UTC{sign}{:02}:{:02}", abs / 3600, (abs % 3600) / 60)
}

/// Caller-injected protocol header: the context the deterministic
/// engine cannot own — wall-clock date/time (local KZ zone) and the
/// worker / operator (ИТҚ) identities.  Printed just above the
/// engine's `render_kk` body (which carries the tamper-evidence hash),
/// so together they form a signable допуск journal entry.
fn print_protocol_header(timestamp: &str, tz: &str, worker: Option<&str>, operator: Option<&str>) {
    let blank = "____________________";
    println!("──────────── ДОПУСК ХАТТАМАСЫ ────────────");
    println!("Күні/уақыты: {timestamp} ({tz})");
    println!("Жұмысшы: {}", worker.unwrap_or(blank));
    println!("ИТҚ/оператор: {}", operator.unwrap_or(blank));
}

fn run_session(id: &str, tts: &dyn TtsBackend, ctx: &SessionContext) {
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
            // Read the wall clock ONCE so the printed header and the
            // sealed envelope carry the same timestamp.
            let offset = tz_offset_secs_from_env();
            let clock = read_clock(offset);
            let timestamp = format!(
                "{:04}-{:02}-{:02} {:02}:{:02}",
                clock.year, clock.month, clock.day, clock.hour, clock.minute
            );
            let tz = tz_label(offset);

            // Stamp the caller-side header, then print the engine's
            // protocol body (feedback + render_kk with the integrity
            // hash) for the ИТР to read and sign.
            println!();
            print_protocol_header(&timestamp, &tz, ctx.worker, ctx.operator);
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
                if let Some(key) = ctx.signer {
                    emit_seal(&p, ctx, &timestamp, &tz, key);
                }
            }
            tts.wait_until_done();
            break;
        }
        println!("\n{}\n", reply.text);
        say(tts, &reply.text);
    }
}

/// Seal the finished protocol and either write it to `--seal-out` or
/// print it to stdout.  A sealing failure is reported but never crashes
/// the session — the human-readable protocol was already produced.
fn emit_seal(
    p: &BriefingProtocol,
    ctx: &SessionContext,
    timestamp: &str,
    tz: &str,
    key: &SigningKey,
) {
    let seal_ctx = SealContext {
        worker: ctx.worker.unwrap_or_default().to_string(),
        operator: ctx.operator.unwrap_or_default().to_string(),
        timestamp: timestamp.to_string(),
        timezone: tz.to_string(),
        site: ctx.site.unwrap_or_default().to_string(),
        prev_record_hash: String::new(),
    };
    let sealed = p.seal_with(&seal_ctx, key, ENGINE_VERSION);
    let json = sealed.to_json();
    match ctx.seal_out {
        Some(path) => match std::fs::write(path, format!("{json}\n")) {
            Ok(()) => eprintln!(
                "adam_briefing: sealed protocol written to {path}\n\
                 adam_briefing: signer public key {}",
                sealed.public_key()
            ),
            Err(e) => eprintln!("adam_briefing: could not write --seal-out {path}: {e}"),
        },
        None => {
            println!("\n──────────── ҚОЛ ҚОЙЫЛҒАН ХАТТАМА (JSON) ────────────");
            println!("{json}");
            eprintln!("adam_briefing: signer public key {}", sealed.public_key());
        }
    }
}

/// Load an Ed25519 signing key from a file holding the 32-byte seed as
/// hex (the format `keygen --out` writes).  Surrounding whitespace is
/// tolerated.
fn load_signing_key(path: &str) -> Result<SigningKey, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    SigningKey::from_seed_hex(raw.trim()).ok_or_else(|| "not a valid 32-byte hex seed".to_string())
}

/// `keygen [--out <path>]` — mint a fresh Ed25519 signing key.
///
/// The public key is printed to stdout; distribute it to whoever must
/// verify seals.  The secret seed is the private material: with `--out`
/// it is written to a `0600` file, otherwise printed to stderr with a
/// warning so it never lands in a piped stdout by accident.
fn run_keygen(args: &[String]) {
    let key = match generate_signing_key() {
        Ok(k) => k,
        Err(e) => {
            eprintln!("adam_briefing: keygen failed to read OS randomness: {e}");
            return;
        }
    };
    println!("{}", key.public_key_hex());
    match flag_value(args, "--out") {
        Some(path) => {
            if let Err(e) = write_secret_seed(&path, &key.seed_hex()) {
                eprintln!("adam_briefing: could not write secret key to {path}: {e}");
                return;
            }
            eprintln!(
                "adam_briefing: secret seed written to {path} (keep it private — anyone with \
                 this file can sign as you).\nadam_briefing: public key printed above; share it \
                 with verifiers."
            );
        }
        None => {
            eprintln!(
                "adam_briefing: SECRET SEED (store securely, do NOT commit): {}\n\
                 adam_briefing: re-run with `--out <path>` to write it to a 0600 file instead.",
                key.seed_hex()
            );
        }
    }
}

/// Write the secret seed with owner-only permissions where the OS
/// supports it.
fn write_secret_seed(path: &str, seed_hex: &str) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        f.write_all(seed_hex.as_bytes())?;
        f.write_all(b"\n")?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, format!("{seed_hex}\n"))
    }
}

/// `verify <sealed.json> [--expect-key <pubhex>]` — check a sealed
/// protocol's Ed25519 seal and print a допуск-ready verdict.  Exits
/// non-zero when the seal does not hold, so scripts can gate on it.
fn run_verify(args: &[String]) {
    let expect_key = flag_value(args, "--expect-key");
    // Skip the `verify` subcommand token when hunting for the path.
    let Some(path) = positional_id(&args[1..]) else {
        eprintln!("usage: adam_briefing verify <sealed.json> [--expect-key <pubhex>]");
        std::process::exit(2);
    };
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("adam_briefing: cannot read {path}: {e}");
            std::process::exit(2);
        }
    };
    let sealed = match SealedProtocol::from_json(&raw) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("adam_briefing: {path} is not a valid sealed protocol: {e}");
            std::process::exit(2);
        }
    };

    let v = sealed.verify();
    let env = &sealed.envelope;
    println!("──────────── SEAL ТЕКСЕРУ ────────────");
    println!("Рәсім: {} ({})", env.procedure_title_kk, env.procedure_id);
    println!("Жұмысшы: {}", env.worker);
    println!("ИТҚ/оператор: {}", env.operator);
    println!("Күні/уақыты: {} ({})", env.timestamp, env.timezone);
    println!("Қозғалтқыш нұсқасы: {}", env.engine_version);
    println!(
        "Нәтиже: {}/{} — {}",
        env.passed_count,
        env.total,
        if env.admitted {
            "допущен"
        } else {
            "не допущен"
        }
    );
    println!("Қол қойған кілт (public): {}", sealed.public_key());
    println!("  алгоритм танылды: {}", yes_no(v.alg_known));
    println!("  дайджест сәйкес:  {}", yes_no(v.digest_matches));
    println!("  қолтаңба жарамды: {}", yes_no(v.signature_valid));

    let mut ok = v.is_valid();
    if let Some(expected) = expect_key {
        let matches = expected.trim().eq_ignore_ascii_case(sealed.public_key());
        println!("  күтілген кілт:    {}", yes_no(matches));
        ok = ok && matches;
    }

    if ok {
        println!("НӘТИЖЕ: ЖАРАМДЫ — хаттама бүтін, қол қойылған.");
        std::process::exit(0);
    } else {
        println!("НӘТИЖЕ: ЖАРАМСЫЗ — хаттамаға сенуге болмайды.");
        std::process::exit(1);
    }
}

fn yes_no(b: bool) -> &'static str {
    if b { "иә" } else { "ЖОҚ" }
}
