// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `adam_portal` — pilot web portal over the deterministic OT/ТБ briefing
//! engine.
//!
//! A **zero-external-dependency** HTTP server (`std::net` only) that puts
//! the real [`adam_dialog::briefing_session`] engine and the signed
//! [`adam_dialog::briefing_seal`] допуск credential behind a browser UI:
//! a worker completes инструктаж → устный опрос → допуск/недопуск, and the
//! ИТР watches issued протоколы on a live dashboard.  It serves the two
//! pages from `demo/` and exposes a small JSON API.
//!
//! This is the pilot MVP for the ССГПО/ERG annual-retraining flow, meant
//! to be **self-hosted on the enterprise's own servers** (data never
//! leaves the company; no external cloud AI).  Prototype-grade: no TLS, no
//! auth, single process, in-memory state.  Production hardening (corporate
//! SSO, camera proctoring, TLS, persistence) is the next layer.
//!
//! ```sh
//! cargo run -p adam-portal --bin adam_portal        # binds 127.0.0.1:8787
//! # worker:    http://127.0.0.1:8787/
//! # ИТР board: http://127.0.0.1:8787/itr
//! ```

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use adam_dialog::briefing_seal::{SealContext, SealedProtocol};
use adam_dialog::briefing_session::{BriefingSession, Lang};
use adam_dialog::procedure_loader::shared_procedures;
use adam_dialog::system_clock::{read_clock, tz_offset_secs_from_env};
use adam_dialog::templates::TemplateRepository;
use adam_dialog::{Conversation, DomainIndex};
use adam_kernel_fst::lexicon::LexiconV1;
use adam_retrieval::MorphemeIndex;
use adam_seal::{SigningKey, sha256, to_hex};
use serde::Deserialize;
use serde_json::{Value, json};

const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
const ADDR: &str = "127.0.0.1:8787";
const WORKER_HTML: &str = include_str!("../../../demo/portal_worker.html");
const ITR_HTML: &str = include_str!("../../../demo/portal_itr.html");
const DIALOG_HTML: &str = include_str!("../../../demo/portal_dialog.html");
const PITCH_HTML: &str = include_str!("../../../demo/erg_ot_tb_demo.html");

/// Our neural Kazakh voice — Piper model + venv binary.  Override with
/// `ADAM_PORTAL_PIPER_MODEL` / `ADAM_PORTAL_PIPER_BIN`.  When absent,
/// `/api/tts` returns 503 and the browser falls back to its own voice.
const PIPER_MODEL: &str = "data/tts_models/kk_KZ-issai-high.onnx";
const PIPER_BIN: &str = "data/tts_models/.venv/bin/piper";

const RETRIEVAL_INDEX_PATH: &str = "data/retrieval/morpheme_index.json";
const FACTS_PATH: &str = "data/retrieval/facts.json";
const DERIVED_FACTS_PATH: &str = "data/retrieval/derived_facts.json";
const WORLD_CORE_DIR: &str = "data/world_core";

/// Default persistent signing key (hex seed).  Override with `ADAM_PORTAL_KEY`.
const KEY_FILE: &str = "data/portal/operator.key";
/// Default допуск journal — one signed credential per line (JSONL).
/// Override with `ADAM_PORTAL_JOURNAL`.
const JOURNAL_FILE: &str = "data/portal/admissions.jsonl";

/// One issued протокол, kept for the ИТР dashboard.
struct Admission {
    worker: String,
    worker_id: String,
    procedure: String,
    admitted: bool,
    passed_count: u32,
    total: u32,
    issuance_date: String,
    public_key: String,
    valid: bool,
    id_method: String,
    proctor_sha256: String,
}

impl Admission {
    /// Summarise a signed credential for the dashboard, re-verifying its
    /// seal (so a tampered journal line shows as `valid: false`).
    fn from_sealed(sealed: &SealedProtocol) -> Admission {
        let e = &sealed.envelope;
        Admission {
            worker: e.credential_subject.name.clone(),
            worker_id: e.credential_subject.id_ref.clone(),
            procedure: e.procedure.title_kk.clone(),
            admitted: e.admitted,
            passed_count: e.evidence.passed_count,
            total: e.evidence.total,
            issuance_date: e.issuance_date.clone(),
            public_key: sealed.public_key().to_string(),
            valid: sealed.verify().is_valid(),
            id_method: e.credential_subject.id_method.clone(),
            proctor_sha256: e.credential_subject.proctor_sha256.clone(),
        }
    }
}

/// A live session plus the caller context needed to seal it later.
struct Live {
    session: BriefingSession,
    worker: String,
    worker_id: String,
    /// `sha256:<hex>` of the proctoring snapshot, or empty if none.
    proctor_sha256: String,
}

struct AppState {
    sessions: HashMap<String, Live>,
    admissions: Vec<Admission>,
    key: SigningKey,
    journal_path: PathBuf,
    /// Directory holding proctoring snapshots (`<hex>.jpg`).
    proctor_dir: PathBuf,
}

/// The general-conversation engine — the *real* ADAM deterministic path
/// (v6.2 router + retrieval + world_core; no neural rescorer, so no burn).
/// One shared instance behind a Mutex: dialog context accumulates across
/// turns, which for a demo is desirable (it remembers the name, like the
/// console voice REPL).  Loading is best-effort; if the data packs are
/// absent, `/api/chat` reports that instead of running.
struct ChatEngine {
    conv: Conversation,
    lex: LexiconV1,
    repo: TemplateRepository,
}

fn load_chat_engine() -> Option<ChatEngine> {
    let lex = LexiconV1::load_default().ok()?;
    let repo = TemplateRepository::load_default().ok()?;
    let mut conv = Conversation::new();
    if let Some(idx) = load_retrieval_index() {
        conv = conv.with_morpheme_index(idx);
    }
    let (extracted, derived) = load_reasoning_chains();
    if !extracted.is_empty() || !derived.is_empty() {
        conv = conv.with_reasoning_chains(extracted, derived);
    }
    if let Ok(report) = adam_reasoning::world_core::load_world_core_dir(Path::new(WORLD_CORE_DIR)) {
        let entries: Vec<_> = report.entries.into_iter().map(|(e, _)| e).collect();
        conv = conv.with_domain_index(DomainIndex::build(&entries));
    }
    Some(ChatEngine { conv, lex, repo })
}

fn load_retrieval_index() -> Option<MorphemeIndex> {
    let file = fs::File::open(RETRIEVAL_INDEX_PATH).ok()?;
    let mut idx: MorphemeIndex = serde_json::from_reader(BufReader::new(file)).ok()?;
    idx.refresh_stats();
    Some(idx)
}

fn load_reasoning_chains() -> (
    Vec<adam_reasoning::Fact>,
    Vec<adam_reasoning::reasoner::DerivedFact>,
) {
    #[derive(Deserialize)]
    struct FactsFile {
        facts: Vec<adam_reasoning::Fact>,
    }
    #[derive(Deserialize)]
    struct DerivedFile {
        derived: Vec<adam_reasoning::reasoner::DerivedFact>,
    }
    let extracted = fs::File::open(FACTS_PATH)
        .ok()
        .and_then(|f| serde_json::from_reader::<_, FactsFile>(BufReader::new(f)).ok())
        .map(|f| f.facts)
        .unwrap_or_default();
    let derived = fs::File::open(DERIVED_FACTS_PATH)
        .ok()
        .and_then(|f| serde_json::from_reader::<_, DerivedFile>(BufReader::new(f)).ok())
        .map(|f| f.derived)
        .unwrap_or_default();
    (extracted, derived)
}

fn main() {
    // Enable the v6.2 router before building the Conversation engine —
    // without it the math / clock / FrameIndex stack stays gated off.
    if std::env::var("ADAM_V6_2").is_err() {
        // SAFETY: single-threaded here, before any thread is spawned.
        unsafe { std::env::set_var("ADAM_V6_2", "1") };
    }

    let key_path = env_path("ADAM_PORTAL_KEY", KEY_FILE);
    let journal_path = env_path("ADAM_PORTAL_JOURNAL", JOURNAL_FILE);
    ensure_parent_dir(&key_path);
    ensure_parent_dir(&journal_path);

    // Persistent operator signing key: loaded from disk, or minted once and
    // saved so every restart signs with the SAME key (past допуска stay
    // verifiable).
    let key = match load_or_create_key(&key_path) {
        Ok(k) => k,
        Err(e) => {
            eprintln!(
                "adam_portal: signing key error ({}): {e}",
                key_path.display()
            );
            return;
        }
    };
    eprintln!(
        "adam_portal: signer public key {} (key: {})",
        key.public_key_hex(),
        key_path.display()
    );

    // Restore the допуск journal (signed credentials, JSONL) from disk.
    let admissions = load_journal(&journal_path);
    eprintln!(
        "adam_portal: journal {} — {} протокол(ов) restored",
        journal_path.display(),
        admissions.len()
    );

    // Proctoring snapshots live next to the journal.
    let proctor_dir = journal_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("proctor");
    let _ = fs::create_dir_all(&proctor_dir);

    let state = Arc::new(Mutex::new(AppState {
        sessions: HashMap::new(),
        admissions,
        key,
        journal_path,
        proctor_dir,
    }));

    // General-conversation engine (real ADAM, deterministic path).
    let chat: Arc<Option<Mutex<ChatEngine>>> = Arc::new(load_chat_engine().map(Mutex::new));
    if chat.is_some() {
        eprintln!("adam_portal: chat engine ready — general dialog at /dialog");
    } else {
        eprintln!(
            "adam_portal: chat engine NOT loaded (data/retrieval or world_core missing) — /dialog reports unavailable"
        );
    }

    let listener = match TcpListener::bind(ADDR) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("adam_portal: cannot bind {ADDR}: {e}");
            return;
        }
    };
    eprintln!("adam_portal: pitch   → http://{ADDR}/pitch");
    eprintln!("adam_portal: dialog  → http://{ADDR}/dialog");
    eprintln!("adam_portal: worker  → http://{ADDR}/");
    eprintln!("adam_portal: ИТР     → http://{ADDR}/itr");

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                let state = Arc::clone(&state);
                let chat = Arc::clone(&chat);
                thread::spawn(move || {
                    if let Err(e) = handle(s, &state, &chat) {
                        eprintln!("adam_portal: connection error: {e}");
                    }
                });
            }
            Err(e) => eprintln!("adam_portal: accept error: {e}"),
        }
    }
}

fn handle(
    mut stream: TcpStream,
    state: &Mutex<AppState>,
    chat: &Option<Mutex<ChatEngine>>,
) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    if reader.read_line(&mut request_line)? == 0 {
        return Ok(()); // client closed
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("/").to_string();

    // Headers → find Content-Length.
    let mut content_length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            content_length = v.trim().parse().unwrap_or(0);
        }
    }
    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body)?;
    }

    let (status, ctype, payload) = route(&method, &path, &body, state, chat);
    write_response(&mut stream, status, ctype, &payload)
}

fn route(
    method: &str,
    path: &str,
    body: &[u8],
    state: &Mutex<AppState>,
    chat: &Option<Mutex<ChatEngine>>,
) -> (u16, &'static str, Vec<u8>) {
    // Proctoring snapshots are served from /proctor/<hex>.jpg.
    if method == "GET" && path.starts_with("/proctor/") {
        return serve_proctor(path, state);
    }
    match (method, path) {
        ("GET", "/") => (200, "text/html; charset=utf-8", WORKER_HTML.into()),
        ("GET", "/itr") => (200, "text/html; charset=utf-8", ITR_HTML.into()),
        ("GET", "/dialog") => (200, "text/html; charset=utf-8", DIALOG_HTML.into()),
        ("GET", "/pitch") => (200, "text/html; charset=utf-8", PITCH_HTML.into()),
        ("GET", "/api/procedures") => json_ok(&api_procedures()),
        ("POST", "/api/proctor") => api_proctor(body, state),
        ("POST", "/api/start") => api_start(body, state),
        ("POST", "/api/answer") => api_answer(body, state),
        ("POST", "/api/seal") => api_seal(body, state),
        ("POST", "/api/chat") => api_chat(body, chat),
        ("POST", "/api/tts") => api_tts(body),
        ("GET", "/api/admissions") => json_ok(&api_admissions(state)),
        _ => (
            404,
            "application/json; charset=utf-8",
            b"{\"error\":\"not found\"}".to_vec(),
        ),
    }
}

fn api_procedures() -> Value {
    let list: Vec<Value> = shared_procedures()
        .iter()
        .map(|p| json!({ "id": p.id, "title": p.title_kk }))
        .collect();
    Value::Array(list)
}

fn api_start(body: &[u8], state: &Mutex<AppState>) -> (u16, &'static str, Vec<u8>) {
    let v: Value = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(_) => return json_err("bad json"),
    };
    let worker = v["worker"].as_str().unwrap_or("").trim().to_string();
    let worker_id = v["workerId"].as_str().unwrap_or("").trim().to_string();
    let proc_id = v["procedureId"].as_str().unwrap_or("");
    // Proctoring hash from a prior /api/proctor upload (empty if none).
    let proctor_sha256 = v["proctorSha256"].as_str().unwrap_or("").to_string();
    // Delivery language (kz default; ru mandatory for ССГПО).
    let lang = match v["lang"].as_str() {
        Some("ru") => Lang::Ru,
        _ => Lang::Kk,
    };
    if worker.is_empty() {
        return json_err("worker name required");
    }
    let Some(session) = BriefingSession::from_id_in(proc_id, lang) else {
        return json_err("unknown procedure");
    };
    let intro = session.begin();
    let id = next_session_id();
    let mut st = state.lock().unwrap();
    st.sessions.insert(
        id.clone(),
        Live {
            session,
            worker,
            worker_id,
            proctor_sha256,
        },
    );
    json_ok(&json!({ "sessionId": id, "text": intro, "done": false }))
}

fn api_answer(body: &[u8], state: &Mutex<AppState>) -> (u16, &'static str, Vec<u8>) {
    let v: Value = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(_) => return json_err("bad json"),
    };
    let sid = v["sessionId"].as_str().unwrap_or("");
    let input = v["input"].as_str().unwrap_or("");
    let mut st = state.lock().unwrap();
    let Some(live) = st.sessions.get_mut(sid) else {
        return json_err("unknown session");
    };
    let reply = live.session.advance(input.trim());
    json_ok(&json!({ "text": reply.text, "done": reply.done }))
}

fn api_seal(body: &[u8], state: &Mutex<AppState>) -> (u16, &'static str, Vec<u8>) {
    let v: Value = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(_) => return json_err("bad json"),
    };
    let sid = v["sessionId"].as_str().unwrap_or("").to_string();
    let mut st = state.lock().unwrap();
    let Some(live) = st.sessions.get(&sid) else {
        return json_err("unknown session");
    };
    let Some(protocol) = live.session.protocol() else {
        return json_err("session not finished");
    };

    let clock = read_clock(tz_offset_secs_from_env());
    // Camera proctoring, when a snapshot was captured, upgrades the
    // identity method and binds the face-image hash into the credential.
    let has_proctor = !live.proctor_sha256.is_empty();
    let ctx = SealContext {
        worker: live.worker.clone(),
        worker_id: live.worker_id.clone(),
        worker_id_method: if has_proctor {
            "camera-proctored".to_string()
        } else {
            "portal-selfservice".to_string()
        },
        proctor_sha256: live.proctor_sha256.clone(),
        operator: "ADAM ОТ/ТБ порталы".to_string(),
        timestamp: format!(
            "{:04}-{:02}-{:02} {:02}:{:02}",
            clock.year, clock.month, clock.day, clock.hour, clock.minute
        ),
        timezone: "UTC+05:00".to_string(),
        site: "ССГПО".to_string(),
        ..SealContext::default()
    };
    let sealed = protocol.seal_with(&ctx, &st.key, ENGINE_VERSION);
    let verify = sealed.verify();

    let resp = json!({
        "admitted": sealed.envelope.admitted,
        "passedCount": sealed.envelope.evidence.passed_count,
        "total": sealed.envelope.evidence.total,
        "publicKey": sealed.public_key(),
        "verify": {
            "signatureValid": verify.signature_valid,
            "digestMatches": verify.digest_matches,
            "algKnown": verify.alg_known,
            "issuerBound": verify.issuer_bound,
        },
        "sealed": serde_json::to_value(&sealed).unwrap_or(Value::Null),
    });

    // Persist the signed credential (append-only journal), then update the
    // in-memory dashboard view.
    append_journal(&st.journal_path, &sealed);
    st.admissions.push(Admission::from_sealed(&sealed));
    st.sessions.remove(&sid);
    json_ok(&resp)
}

fn api_admissions(state: &Mutex<AppState>) -> Value {
    let st = state.lock().unwrap();
    let rows: Vec<Value> = st
        .admissions
        .iter()
        .map(|a| {
            json!({
                "worker": a.worker,
                "workerId": a.worker_id,
                "procedure": a.procedure,
                "admitted": a.admitted,
                "passedCount": a.passed_count,
                "total": a.total,
                "issuanceDate": a.issuance_date,
                "publicKey": a.public_key,
                "valid": a.valid,
                "idMethod": a.id_method,
                "proctorSha256": a.proctor_sha256,
                "proctorUrl": proctor_url(&a.proctor_sha256),
            })
        })
        .collect();
    Value::Array(rows)
}

/// General conversation: run the visitor's text through the real ADAM
/// deterministic engine and return its Kazakh reply.
fn api_chat(body: &[u8], chat: &Option<Mutex<ChatEngine>>) -> (u16, &'static str, Vec<u8>) {
    let v: Value = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(_) => return json_err("bad json"),
    };
    let text = v["text"].as_str().unwrap_or("").trim().to_string();
    if text.is_empty() {
        return json_err("empty text");
    }
    let Some(engine) = chat else {
        return json_ok(&json!({
            "reply": "Диалог қозғалтқышы жүктелмеген (data/retrieval немесе world_core жоқ)."
        }));
    };
    let mut e = engine.lock().unwrap();
    let ChatEngine { conv, lex, repo } = &mut *e;
    let reply = conv.turn(&text, lex, repo, 42);
    json_ok(&json!({ "reply": reply }))
}

/// Text-to-speech with **our** neural Kazakh voice (Piper).  Returns a
/// 22.05 kHz mono WAV the browser plays directly — far better Kazakh than
/// the browser's own speech engine.  503 when Piper isn't set up (the page
/// then falls back to the browser voice).
fn api_tts(body: &[u8]) -> (u16, &'static str, Vec<u8>) {
    let v: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
    let text = v["text"].as_str().unwrap_or("").trim();
    if text.is_empty() {
        return json_err("empty text");
    }
    match synth_piper(text) {
        Some(wav) => (200, "audio/wav", wav),
        None => (
            503,
            "application/json; charset=utf-8",
            br#"{"error":"tts unavailable"}"#.to_vec(),
        ),
    }
}

/// Synthesise `text` to WAV bytes via the Piper venv, or `None` if Piper is
/// unavailable / fails.  Shells out per request (a demo, not a hot path).
fn synth_piper(text: &str) -> Option<Vec<u8>> {
    let model = env_path("ADAM_PORTAL_PIPER_MODEL", PIPER_MODEL);
    let bin = env_path("ADAM_PORTAL_PIPER_BIN", PIPER_BIN);
    if !model.exists() || !bin.exists() {
        return None;
    }
    let prompt = tts_prompt(text);
    if prompt.is_empty() {
        return None;
    }
    static TTS_COUNTER: AtomicU64 = AtomicU64::new(1);
    let n = TTS_COUNTER.fetch_add(1, Ordering::Relaxed);
    let out = std::env::temp_dir().join(format!("adam_portal_tts_{}_{n}.wav", std::process::id()));

    let mut child = Command::new(&bin)
        .arg("--model")
        .arg(&model)
        .arg("--length-scale")
        .arg("1.0")
        .arg("--sentence-silence")
        .arg("0.2")
        .arg("--output-file")
        .arg(&out)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(prompt.as_bytes());
    }
    let ok = child.wait().ok()?.success();
    let bytes = if ok { fs::read(&out).ok() } else { None };
    let _ = fs::remove_file(&out);
    bytes
}

/// Canonical TTS sentence shape: capitalise the first letter and add a
/// trailing period so Piper reads it as a full sentence.
fn tts_prompt(text: &str) -> String {
    let t = text.trim();
    let mut chars = t.chars();
    let Some(head) = chars.next() else {
        return String::new();
    };
    let cap: String = head.to_uppercase().collect::<String>() + chars.as_str();
    if cap.ends_with(['.', '!', '?']) {
        cap
    } else {
        format!("{cap}.")
    }
}

/// Store a proctoring snapshot (raw JPEG body): hash it, save `<hex>.jpg`,
/// and return the `sha256:<hex>` the worker page then passes to /api/start.
fn api_proctor(body: &[u8], state: &Mutex<AppState>) -> (u16, &'static str, Vec<u8>) {
    if body.is_empty() {
        return json_err("empty image");
    }
    let hex = to_hex(&sha256(body));
    let dir = { state.lock().unwrap().proctor_dir.clone() };
    let path = dir.join(format!("{hex}.jpg"));
    if let Err(e) = fs::write(&path, body) {
        eprintln!(
            "adam_portal: cannot write proctor image {}: {e}",
            path.display()
        );
        return json_err("cannot store image");
    }
    json_ok(&json!({ "proctorSha256": format!("sha256:{hex}") }))
}

/// Serve a proctoring snapshot from `/proctor/<hex>.jpg`.  The filename is
/// constrained to hex + `.jpg` so it cannot escape the directory.
fn serve_proctor(path: &str, state: &Mutex<AppState>) -> (u16, &'static str, Vec<u8>) {
    let name = path.trim_start_matches("/proctor/");
    let stem = name.strip_suffix(".jpg").unwrap_or(name);
    if stem.is_empty() || !stem.chars().all(|c| c.is_ascii_hexdigit()) {
        return (404, "text/plain; charset=utf-8", b"not found".to_vec());
    }
    let dir = { state.lock().unwrap().proctor_dir.clone() };
    match fs::read(dir.join(format!("{stem}.jpg"))) {
        Ok(bytes) => (200, "image/jpeg", bytes),
        Err(_) => (404, "text/plain; charset=utf-8", b"not found".to_vec()),
    }
}

/// `/proctor/<hex>.jpg` URL for a `sha256:<hex>` proctoring hash, or empty.
fn proctor_url(proctor_sha256: &str) -> String {
    match proctor_sha256.strip_prefix("sha256:") {
        Some(hex) if !hex.is_empty() => format!("/proctor/{hex}.jpg"),
        _ => String::new(),
    }
}

// ------------------------------------------------------------- helpers

fn next_session_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    format!("s{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}

/// Path from an env var, or the given default.
fn env_path(var: &str, default: &str) -> PathBuf {
    env::var(var)
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(default))
}

/// Create the parent directory of `path` if it does not exist.
fn ensure_parent_dir(path: &Path) {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = fs::create_dir_all(parent);
        }
    }
}

/// Load the operator signing key from `path`, or mint a fresh one and save
/// it (owner-only where the OS supports it) so restarts reuse the same key.
fn load_or_create_key(path: &Path) -> std::io::Result<SigningKey> {
    if path.exists() {
        let hex = fs::read_to_string(path)?;
        return SigningKey::from_seed_hex(hex.trim()).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "not a valid 32-byte hex seed",
            )
        });
    }
    let key = adam_seal::generate_signing_key()?;
    write_secret_seed(path, &key.seed_hex())?;
    eprintln!("adam_portal: minted a new signing key → {}", path.display());
    Ok(key)
}

/// Write a secret seed with owner-only permissions where supported.
fn write_secret_seed(path: &Path, seed_hex: &str) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        f.write_all(seed_hex.as_bytes())?;
        f.write_all(b"\n")
    }
    #[cfg(not(unix))]
    {
        fs::write(path, format!("{seed_hex}\n"))
    }
}

/// Restore the допуск journal — one signed credential per line.  Malformed
/// lines are skipped; each survivor is re-verified for the dashboard.
fn load_journal(path: &Path) -> Vec<Admission> {
    let mut out = Vec::new();
    if let Ok(content) = fs::read_to_string(path) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(sealed) = SealedProtocol::from_json(line) {
                out.push(Admission::from_sealed(&sealed));
            }
        }
    }
    out
}

/// Append one signed credential to the journal as a compact JSON line.
fn append_journal(path: &Path, sealed: &SealedProtocol) {
    let Ok(line) = serde_json::to_string(sealed) else {
        return;
    };
    match fs::OpenOptions::new().create(true).append(true).open(path) {
        Ok(mut f) => {
            if let Err(e) = writeln!(f, "{line}") {
                eprintln!("adam_portal: journal append failed: {e}");
            }
        }
        Err(e) => eprintln!("adam_portal: cannot open journal {}: {e}", path.display()),
    }
}

fn json_ok(v: &Value) -> (u16, &'static str, Vec<u8>) {
    (
        200,
        "application/json; charset=utf-8",
        serde_json::to_vec(v).unwrap_or_default(),
    )
}

fn json_err(msg: &str) -> (u16, &'static str, Vec<u8>) {
    (
        400,
        "application/json; charset=utf-8",
        serde_json::to_vec(&json!({ "error": msg })).unwrap_or_default(),
    )
}

fn write_response(
    stream: &mut TcpStream,
    status: u16,
    ctype: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "OK",
    };
    let header = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}
