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
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use adam_dialog::briefing_seal::SealContext;
use adam_dialog::briefing_session::BriefingSession;
use adam_dialog::procedure_loader::shared_procedures;
use adam_dialog::system_clock::{read_clock, tz_offset_secs_from_env};
use adam_seal::SigningKey;
use serde_json::{Value, json};

const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
const ADDR: &str = "127.0.0.1:8787";
const WORKER_HTML: &str = include_str!("../../../demo/portal_worker.html");
const ITR_HTML: &str = include_str!("../../../demo/portal_itr.html");

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
}

/// A live session plus the caller context needed to seal it later.
struct Live {
    session: BriefingSession,
    worker: String,
    worker_id: String,
}

struct AppState {
    sessions: HashMap<String, Live>,
    admissions: Vec<Admission>,
    key: SigningKey,
}

fn main() {
    // Ephemeral operator key for the prototype.  A real deployment loads a
    // persistent per-operator key (see adam_briefing keygen).
    let key = match adam_seal::generate_signing_key() {
        Ok(k) => k,
        Err(e) => {
            eprintln!("adam_portal: cannot read OS randomness for signing key: {e}");
            return;
        }
    };
    eprintln!("adam_portal: signer public key {}", key.public_key_hex());
    eprintln!("adam_portal: (ephemeral prototype key — regenerated each start)");

    let state = Arc::new(Mutex::new(AppState {
        sessions: HashMap::new(),
        admissions: Vec::new(),
        key,
    }));

    let listener = match TcpListener::bind(ADDR) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("adam_portal: cannot bind {ADDR}: {e}");
            return;
        }
    };
    eprintln!("adam_portal: worker  → http://{ADDR}/");
    eprintln!("adam_portal: ИТР     → http://{ADDR}/itr");

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                let state = Arc::clone(&state);
                thread::spawn(move || {
                    if let Err(e) = handle(s, &state) {
                        eprintln!("adam_portal: connection error: {e}");
                    }
                });
            }
            Err(e) => eprintln!("adam_portal: accept error: {e}"),
        }
    }
}

fn handle(mut stream: TcpStream, state: &Mutex<AppState>) -> std::io::Result<()> {
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

    let (status, ctype, payload) = route(&method, &path, &body, state);
    write_response(&mut stream, status, ctype, &payload)
}

fn route(
    method: &str,
    path: &str,
    body: &[u8],
    state: &Mutex<AppState>,
) -> (u16, &'static str, Vec<u8>) {
    match (method, path) {
        ("GET", "/") => (200, "text/html; charset=utf-8", WORKER_HTML.into()),
        ("GET", "/itr") => (200, "text/html; charset=utf-8", ITR_HTML.into()),
        ("GET", "/api/procedures") => json_ok(&api_procedures()),
        ("POST", "/api/start") => api_start(body, state),
        ("POST", "/api/answer") => api_answer(body, state),
        ("POST", "/api/seal") => api_seal(body, state),
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
    if worker.is_empty() {
        return json_err("worker name required");
    }
    let Some(session) = BriefingSession::from_id(proc_id) else {
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
    let ctx = SealContext {
        worker: live.worker.clone(),
        worker_id: live.worker_id.clone(),
        // Remote self-service: identity is portal-asserted for now; camera
        // proctoring upgrades this to a stronger id_method in the next layer.
        worker_id_method: "portal-selfservice".to_string(),
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

    let admission = Admission {
        worker: live.worker.clone(),
        worker_id: live.worker_id.clone(),
        procedure: sealed.envelope.procedure.title_kk.clone(),
        admitted: sealed.envelope.admitted,
        passed_count: sealed.envelope.evidence.passed_count,
        total: sealed.envelope.evidence.total,
        issuance_date: sealed.envelope.issuance_date.clone(),
        public_key: sealed.public_key().to_string(),
        valid: verify.is_valid(),
    };

    let resp = json!({
        "admitted": admission.admitted,
        "passedCount": admission.passed_count,
        "total": admission.total,
        "publicKey": admission.public_key,
        "verify": {
            "signatureValid": verify.signature_valid,
            "digestMatches": verify.digest_matches,
            "algKnown": verify.alg_known,
            "issuerBound": verify.issuer_bound,
        },
        "sealed": serde_json::to_value(&sealed).unwrap_or(Value::Null),
    });

    st.admissions.push(admission);
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
            })
        })
        .collect();
    Value::Array(rows)
}

// ------------------------------------------------------------- helpers

fn next_session_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    format!("s{}", COUNTER.fetch_add(1, Ordering::Relaxed))
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
