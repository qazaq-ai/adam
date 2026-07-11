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
//! This is the pilot MVP for the enterprise annual-retraining flow, meant
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
use adam_dialog::briefing_session::{BriefingProtocol, BriefingSession, Lang};
use adam_dialog::procedure_loader::shared_procedures;
use adam_dialog::system_clock::{read_clock, tz_offset_secs_from_env};
use adam_dialog::templates::TemplateRepository;
use adam_dialog::{Conversation, DomainIndex};
use adam_kernel_fst::lexicon::LexiconV1;
use adam_retrieval::MorphemeIndex;
use adam_seal::{SigningKey, sha256, to_hex};
use serde::{Deserialize, Serialize};
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

/// The annual-retraining **program** — the ordered set of briefing
/// topics a worker must clear to be admitted.  A допуск is issued ONLY
/// after every topic's зачёт is passed; failing any one → пересдача.
///
/// Demo default is a short, watchable set; a real deployment lists the
/// full applicable program (edit this constant).  Ids are validated
/// against the loaded corpus at startup.
const PROGRAM: &[&str] = &[
    "kk_labor_vvodnyi_016",           // вводный инструктаж
    "kk_labor_primary_workplace_017", // первичный инструктаж на рабочем месте
    "kk_okhrana_briefing_004",        // периодический (повторный) инструктаж
];

/// Days a worker gets to self-prepare before a пересдача when a зачёт is
/// failed.  30 days (~1 month) mirrors the Kazakhstan practice for a
/// repeat knowledge check (повторная проверка знаний) after an
/// unsatisfactory result.
const RETAKE_DAYS: i64 = 30;

/// One graded topic inside a program run — carries its own signed
/// protocol so every зачёт stays independently verifiable.
#[derive(Clone, Serialize, Deserialize)]
struct TopicRecord {
    procedure_id: String,
    title: String,
    passed_count: u32,
    total: u32,
    admitted: bool,
    sealed: SealedProtocol,
}

/// The aggregate outcome of one worker's full program run — the unit the
/// ИТР dashboard shows and the journal persists (one JSONL line each).
#[derive(Clone, Serialize, Deserialize)]
struct ProgramRecord {
    schema: String,
    worker: String,
    worker_id: String,
    issuance_date: String,
    id_method: String,
    proctor_sha256: String,
    topics: Vec<TopicRecord>,
    topics_passed: u32,
    topics_total: u32,
    questions_correct: u32,
    questions_total: u32,
    /// Admitted only when EVERY topic passed.
    admitted: bool,
    /// Date after which a пересдача is allowed (empty when admitted).
    retake_after: String,
}

/// A live program run: the fixed topic list, which topic is active, the
/// signed outcomes of finished topics, and the current topic's session.
struct Live {
    session: BriefingSession,
    worker: String,
    worker_id: String,
    /// `sha256:<hex>` of the proctoring snapshot, or empty if none.
    proctor_sha256: String,
    lang: Lang,
    program: Vec<String>,
    idx: usize,
    outcomes: Vec<TopicRecord>,
}

/// One person in the enterprise org chart.
#[derive(Clone, Serialize, Deserialize)]
struct OrgWorker {
    id: String,
    name: String,
    position: String,
}

/// A section (участок) within a shop (цех).
#[derive(Clone, Serialize, Deserialize)]
struct OrgSection {
    section: String,
    workers: Vec<OrgWorker>,
}

/// A shop (цех) — the top level of the org chart.
#[derive(Clone, Serialize, Deserialize)]
struct OrgShop {
    shop: String,
    sections: Vec<OrgSection>,
}

/// Built-in org chart (цеха → участки → персонал).  Overridable by
/// dropping a `data/portal/org.json` with the same shape.
const ORG_SEED: &str = include_str!("../../../demo/org_seed.json");
const ORG_FILE: &str = "data/portal/org.json";

fn load_org() -> Vec<OrgShop> {
    if let Ok(s) = fs::read_to_string(ORG_FILE) {
        if let Ok(v) = serde_json::from_str::<Vec<OrgShop>>(&s) {
            return v;
        }
    }
    serde_json::from_str(ORG_SEED).unwrap_or_default()
}

struct AppState {
    sessions: HashMap<String, Live>,
    admissions: Vec<ProgramRecord>,
    org: Vec<OrgShop>,
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
        "adam_portal: journal {} — {} запис(ей) допуска restored",
        journal_path.display(),
        admissions.len()
    );

    // Proctoring snapshots live next to the journal.
    let proctor_dir = journal_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("proctor");
    let _ = fs::create_dir_all(&proctor_dir);

    let org = load_org();
    let worker_count: usize = org
        .iter()
        .flat_map(|s| &s.sections)
        .map(|sec| sec.workers.len())
        .sum();
    eprintln!(
        "adam_portal: org chart — {} цех(ов), {} работник(ов)",
        org.len(),
        worker_count
    );
    let state = Arc::new(Mutex::new(AppState {
        sessions: HashMap::new(),
        admissions,
        org,
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
        ("GET", "/api/program") => json_ok(&api_program()),
        ("GET", "/api/org") => json_ok(&api_org(state)),
        ("POST", "/api/worker") => api_worker(body, state),
        ("POST", "/api/proctor") => api_proctor(body, state),
        ("POST", "/api/start") => api_start(body, state),
        ("POST", "/api/answer") => api_answer(body, state),
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

/// The training program's ordered topics with kk/ru titles — the worker
/// page lists these before starting, and shows «Тема N из M» during.
fn api_program() -> Value {
    let procs = shared_procedures();
    let list: Vec<Value> = resolved_program()
        .iter()
        .filter_map(|id| procs.iter().find(|p| &p.id == id))
        .map(|p| {
            json!({
                "id": p.id,
                "title_kk": p.title_kk,
                "title_ru": p.title_ru.clone().unwrap_or_else(|| p.title_kk.clone()),
            })
        })
        .collect();
    Value::Array(list)
}

/// The program a worker runs: the constant `PROGRAM` filtered to ids the
/// loaded corpus actually knows (so a typo can't wedge a session).
fn resolved_program() -> Vec<String> {
    let known: std::collections::HashSet<&str> =
        shared_procedures().iter().map(|p| p.id.as_str()).collect();
    PROGRAM
        .iter()
        .filter(|id| known.contains(*id))
        .map(|s| s.to_string())
        .collect()
}

/// The most recent program run for a worker id, if any.
fn latest_run<'a>(admissions: &'a [ProgramRecord], id: &str) -> Option<&'a ProgramRecord> {
    if id.is_empty() {
        return None;
    }
    admissions.iter().rev().find(|r| r.worker_id == id)
}

/// The kk/ru titles of the program's topics, in order.
fn program_topics_titled() -> Vec<(String, String, String)> {
    let procs = shared_procedures();
    resolved_program()
        .iter()
        .filter_map(|id| procs.iter().find(|p| &p.id == id))
        .map(|p| {
            (
                p.id.clone(),
                p.title_kk.clone(),
                p.title_ru.clone().unwrap_or_else(|| p.title_kk.clone()),
            )
        })
        .collect()
}

/// The org chart with each worker's live допуск status (derived from the
/// journal) plus top-line stats — the ИТР admin dashboard's data source.
fn api_org(state: &Mutex<AppState>) -> Value {
    let st = state.lock().unwrap();
    let program_total = resolved_program().len() as u32;
    let (mut total, mut admitted, mut retake) = (0u32, 0u32, 0u32);

    // Workers with a live program run right now (currently taking a зачёт).
    let online_ids: std::collections::HashSet<String> = st
        .sessions
        .values()
        .map(|l| l.worker_id.clone())
        .filter(|s| !s.is_empty())
        .collect();
    let prog_ids = resolved_program();

    let shops: Vec<Value> = st
        .org
        .iter()
        .map(|shop| {
            let mut shop_total = 0u32;
            let mut shop_admitted = 0u32;
            let sections: Vec<Value> = shop
                .sections
                .iter()
                .map(|sec| {
                    let workers: Vec<Value> = sec
                        .workers
                        .iter()
                        .map(|w| {
                            total += 1;
                            shop_total += 1;
                            let latest = latest_run(&st.admissions, &w.id);
                            let (status, last_date, retake_after, tp, tt) = match latest {
                                Some(r) => {
                                    if r.admitted {
                                        admitted += 1;
                                        shop_admitted += 1;
                                    } else {
                                        retake += 1;
                                    }
                                    (
                                        if r.admitted { "admitted" } else { "retake" },
                                        r.issuance_date.clone(),
                                        r.retake_after.clone(),
                                        r.topics_passed,
                                        r.topics_total,
                                    )
                                }
                                None => ("none", String::new(), String::new(), 0, program_total),
                            };
                            // Per-category status of the assigned program topics,
                            // aligned to program order: "p" passed / "f" failed /
                            // "-" pending — lets the dashboard filter by category.
                            let topic_flags: Vec<&str> = prog_ids
                                .iter()
                                .map(|pid| {
                                    match latest.and_then(|r| {
                                        r.topics.iter().find(|t| &t.procedure_id == pid)
                                    }) {
                                        Some(t) if t.admitted => "p",
                                        Some(_) => "f",
                                        None => "-",
                                    }
                                })
                                .collect();
                            json!({
                                "id": w.id, "name": w.name, "position": w.position,
                                "status": status, "lastDate": last_date, "retakeAfter": retake_after,
                                "topicsPassed": tp, "topicsTotal": tt,
                                "online": online_ids.contains(&w.id),
                                "t": topic_flags,
                                "proctorUrl": latest.map(|r| proctor_url(&r.proctor_sha256)).unwrap_or_default(),
                            })
                        })
                        .collect();
                    json!({ "section": sec.section, "workers": workers })
                })
                .collect();
            json!({
                "shop": shop.shop, "sections": sections,
                "total": shop_total, "admitted": shop_admitted,
            })
        })
        .collect();

    // Per-category (briefing topic) aggregate across every worker's latest
    // run — powers the «по категориям инструктажей» sidebar + chart.
    let prog = program_topics_titled();
    let topic_stats: Vec<Value> = prog
        .iter()
        .map(|(pid, kk, ru)| {
            let (mut passed, mut failed, mut pending) = (0u32, 0u32, 0u32);
            for shop in &st.org {
                for sec in &shop.sections {
                    for w in &sec.workers {
                        let t = latest_run(&st.admissions, &w.id)
                            .and_then(|r| r.topics.iter().find(|t| &t.procedure_id == pid));
                        match t {
                            Some(t) if t.admitted => passed += 1,
                            Some(_) => failed += 1,
                            None => pending += 1,
                        }
                    }
                }
            }
            json!({
                "procedureId": pid, "titleKk": kk, "titleRu": ru,
                "passed": passed, "failed": failed, "pending": pending,
            })
        })
        .collect();

    json!({
        "shops": shops,
        "topics": topic_stats,
        "onlineIds": online_ids.iter().cloned().collect::<Vec<_>>(),
        "stats": {
            "total": total, "admitted": admitted, "retake": retake,
            "notStarted": total - admitted - retake,
            "online": online_ids.len(),
        },
    })
}

/// One worker's personal cabinet: identity + assigned program (with
/// per-topic status from the latest run) + full attempt history.
fn api_worker(body: &[u8], state: &Mutex<AppState>) -> (u16, &'static str, Vec<u8>) {
    let v: Value = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(_) => return json_err("bad json"),
    };
    let id = v["id"].as_str().unwrap_or("").trim();
    if id.is_empty() {
        return json_err("id required");
    }
    let st = state.lock().unwrap();

    // Locate the worker in the org chart.
    let mut found: Option<(String, String, &OrgWorker)> = None;
    for shop in &st.org {
        for sec in &shop.sections {
            if let Some(w) = sec.workers.iter().find(|w| w.id == id) {
                found = Some((shop.shop.clone(), sec.section.clone(), w));
            }
        }
    }
    let Some((shop, section, worker)) = found else {
        return json_err("unknown worker");
    };

    let latest = latest_run(&st.admissions, id);
    // Per-topic status of the assigned program, from the latest run.
    let topics: Vec<Value> = program_topics_titled()
        .iter()
        .map(|(pid, kk, ru)| {
            let t = latest.and_then(|r| r.topics.iter().find(|t| &t.procedure_id == pid));
            let status = match t {
                Some(t) if t.admitted => "passed",
                Some(_) => "failed",
                None => "pending",
            };
            json!({ "procedureId": pid, "titleKk": kk, "titleRu": ru, "status": status,
                    "passedCount": t.map(|t| t.passed_count), "total": t.map(|t| t.total) })
        })
        .collect();

    // Full attempt history, newest first.
    let history: Vec<Value> = st
        .admissions
        .iter()
        .rev()
        .filter(|r| r.worker_id == id)
        .map(|r| {
            json!({
                "issuanceDate": r.issuance_date, "admitted": r.admitted,
                "topicsPassed": r.topics_passed, "topicsTotal": r.topics_total,
                "questionsCorrect": r.questions_correct, "questionsTotal": r.questions_total,
                "retakeAfter": r.retake_after,
            })
        })
        .collect();

    let status = match latest {
        Some(r) if r.admitted => "admitted",
        Some(_) => "retake",
        None => "none",
    };
    json_ok(&json!({
        "id": worker.id, "name": worker.name, "position": worker.position,
        "shop": shop, "section": section,
        "status": status,
        "retakeAfter": latest.map(|r| r.retake_after.clone()).unwrap_or_default(),
        "topics": topics,
        "history": history,
    }))
}

fn api_start(body: &[u8], state: &Mutex<AppState>) -> (u16, &'static str, Vec<u8>) {
    let v: Value = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(_) => return json_err("bad json"),
    };
    let worker = v["worker"].as_str().unwrap_or("").trim().to_string();
    let worker_id = v["workerId"].as_str().unwrap_or("").trim().to_string();
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
    let program = resolved_program();
    let Some(first_id) = program.first() else {
        return json_err("empty program");
    };
    let Some(session) = BriefingSession::from_id_in(first_id, lang) else {
        return json_err("unknown procedure");
    };
    let intro = session.begin();
    let title = session.title().to_string();
    let count = program.len();
    let id = next_session_id();
    let mut st = state.lock().unwrap();
    st.sessions.insert(
        id.clone(),
        Live {
            session,
            worker,
            worker_id,
            proctor_sha256,
            lang,
            program,
            idx: 0,
            outcomes: Vec::new(),
        },
    );
    json_ok(&json!({
        "sessionId": id,
        "text": intro,
        "done": false,
        "topicIndex": 1,
        "topicCount": count,
        "topicTitle": title,
    }))
}

/// Seal one finished topic's protocol into an independently-verifiable
/// credential, under the worker/proctor context and the current clock.
fn seal_topic(live: &Live, protocol: &BriefingProtocol, key: &SigningKey) -> SealedProtocol {
    let clock = read_clock(tz_offset_secs_from_env());
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
        site: "Company name".to_string(),
        ..SealContext::default()
    };
    protocol.seal_with(&ctx, key, ENGINE_VERSION)
}

/// Build the aggregate program record from the signed per-topic outcomes.
fn build_program_record(live: &Live) -> ProgramRecord {
    // `topics_total` is the WHOLE program, not just attempted topics — a
    // run that stops early on a failed зачёт shows «1/3», making clear the
    // program was not completed.
    let topics_total = live.program.len() as u32;
    let topics_passed = live.outcomes.iter().filter(|t| t.admitted).count() as u32;
    let questions_correct: u32 = live.outcomes.iter().map(|t| t.passed_count).sum();
    let questions_total: u32 = live.outcomes.iter().map(|t| t.total).sum();
    // Admitted only when every program topic was passed.
    let admitted = topics_total > 0 && topics_passed == topics_total;
    let has_proctor = !live.proctor_sha256.is_empty();
    let clock = read_clock(tz_offset_secs_from_env());
    let issuance_date = format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        clock.year, clock.month, clock.day, clock.hour, clock.minute
    );
    let retake_after = if admitted {
        String::new()
    } else {
        let (y, m, d) = add_days(
            clock.year as i64,
            clock.month as i64,
            clock.day as i64,
            RETAKE_DAYS,
        );
        format!("{y:04}-{m:02}-{d:02}")
    };
    ProgramRecord {
        schema: "adam-dopusk-program/1".to_string(),
        worker: live.worker.clone(),
        worker_id: live.worker_id.clone(),
        issuance_date,
        id_method: if has_proctor {
            "camera-proctored".to_string()
        } else {
            "portal-selfservice".to_string()
        },
        proctor_sha256: live.proctor_sha256.clone(),
        topics: live.outcomes.clone(),
        topics_passed,
        topics_total,
        questions_correct,
        questions_total,
        admitted,
        retake_after,
    }
}

fn api_answer(body: &[u8], state: &Mutex<AppState>) -> (u16, &'static str, Vec<u8>) {
    let v: Value = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(_) => return json_err("bad json"),
    };
    let sid = v["sessionId"].as_str().unwrap_or("").to_string();
    let input = v["input"].as_str().unwrap_or("");
    let mut st = state.lock().unwrap();
    if !st.sessions.contains_key(&sid) {
        return json_err("unknown session");
    }

    // 1) Advance the current topic's session.
    let reply = {
        let live = st.sessions.get_mut(&sid).unwrap();
        live.session.advance(input.trim())
    };

    // Still inside the current topic → return the next slide/question with
    // program progress so the worker page can show «Тема N из M».
    if !reply.done {
        let live = st.sessions.get(&sid).unwrap();
        return json_ok(&json!({
            "text": reply.text,
            "done": false,
            "isQuestion": reply.is_question,
            "topicIndex": live.idx + 1,
            "topicCount": live.program.len(),
            "topicTitle": live.session.title(),
        }));
    }

    // 2) The current topic's зачёт just finished — seal it (immutable read
    // of the session + the key, disjoint fields).
    let outcome = {
        let live = st.sessions.get(&sid).unwrap();
        let Some(protocol) = live.session.protocol() else {
            return json_err("topic not finished");
        };
        let sealed = seal_topic(live, &protocol, &st.key);
        TopicRecord {
            procedure_id: live.session.procedure_id().to_string(),
            title: live.session.title().to_string(),
            passed_count: protocol.passed_count as u32,
            total: protocol.total as u32,
            admitted: protocol.admitted,
            sealed,
        }
    };

    // 3) Record it.  Advance to the next topic ONLY if this зачёт passed
    //    and more topics remain; a failed зачёт STOPS the program right
    //    here (пересдача), and passing the last topic finalises the допуск.
    let live = st.sessions.get_mut(&sid).unwrap();
    live.outcomes.push(outcome.clone());
    let lang = live.lang;

    if outcome.admitted && live.idx + 1 < live.program.len() {
        live.idx += 1;
        let next_id = live.program[live.idx].clone();
        match BriefingSession::from_id_in(&next_id, lang) {
            Some(next) => {
                let intro = next.begin();
                let title = next.title().to_string();
                let idx = live.idx;
                let count = live.program.len();
                live.session = next;
                let head = topic_transition_head(lang, &outcome, idx, count);
                return json_ok(&json!({
                    "text": format!("{head}\n\n{intro}"),
                    "done": false,
                    "isQuestion": false,
                    "topicComplete": true,
                    "topicIndex": idx + 1,
                    "topicCount": count,
                    "topicTitle": title,
                    "lastTopic": { "title": outcome.title, "passedCount": outcome.passed_count, "total": outcome.total, "admitted": outcome.admitted },
                }));
            }
            None => return json_err("unknown procedure"),
        }
    }

    // 4) Last topic done → build + persist the aggregate program record.
    let record = build_program_record(live);
    append_journal(&st.journal_path, &record);
    st.admissions.push(record.clone());
    st.sessions.remove(&sid);

    json_ok(&json!({
        "done": true,
        "programComplete": true,
        "aggregate": program_record_json(&record),
        "sealed": serde_json::to_value(&record).unwrap_or(Value::Null),
    }))
}

/// Localised one-line verdict shown when a topic's зачёт finishes and the
/// next topic starts.
fn topic_transition_head(lang: Lang, o: &TopicRecord, next_idx: usize, count: usize) -> String {
    let n = next_idx + 1;
    match lang {
        Lang::Ru => {
            let verdict = if o.admitted {
                format!("зачёт сдан ({}/{})", o.passed_count, o.total)
            } else {
                format!("зачёт НЕ сдан ({}/{})", o.passed_count, o.total)
            };
            format!("«{}» — {verdict}.\n— Тема {n} из {count}.", o.title)
        }
        Lang::Kk => {
            let verdict = if o.admitted {
                format!("сынақ тапсырылды ({}/{})", o.passed_count, o.total)
            } else {
                format!("сынақ тапсырылмады ({}/{})", o.passed_count, o.total)
            };
            format!(
                "«{}» — {verdict}.\n— {n}-тақырып, барлығы {count}.",
                o.title
            )
        }
    }
}

/// Dashboard/verification view of one program record: per-topic seal
/// re-verification (a tampered journal line shows `valid:false`) plus the
/// aggregate the ИТР board renders.
fn program_record_json(r: &ProgramRecord) -> Value {
    let topics: Vec<Value> = r
        .topics
        .iter()
        .map(|t| {
            json!({
                "title": t.title,
                "procedureId": t.procedure_id,
                "passedCount": t.passed_count,
                "total": t.total,
                "admitted": t.admitted,
                "valid": t.sealed.verify().is_valid(),
            })
        })
        .collect();
    let all_valid = r.topics.iter().all(|t| t.sealed.verify().is_valid());
    let public_key = r
        .topics
        .first()
        .map(|t| t.sealed.public_key().to_string())
        .unwrap_or_default();
    // The topic that stopped the program (first failed зачёт), if any.
    let stopped_at = r
        .topics
        .iter()
        .find(|t| !t.admitted)
        .map(|t| t.title.clone())
        .unwrap_or_default();
    json!({
        "worker": r.worker,
        "workerId": r.worker_id,
        "issuanceDate": r.issuance_date,
        "idMethod": r.id_method,
        "proctorSha256": r.proctor_sha256,
        "proctorUrl": proctor_url(&r.proctor_sha256),
        "topics": topics,
        "topicsPassed": r.topics_passed,
        "topicsTotal": r.topics_total,
        "attempted": r.topics.len(),
        "stoppedAt": stopped_at,
        "questionsCorrect": r.questions_correct,
        "questionsTotal": r.questions_total,
        "admitted": r.admitted,
        "retakeAfter": r.retake_after,
        "valid": all_valid,
        "publicKey": public_key,
    })
}

fn api_admissions(state: &Mutex<AppState>) -> Value {
    let st = state.lock().unwrap();
    Value::Array(st.admissions.iter().map(program_record_json).collect())
}

/// Whether `y` is a leap year (Gregorian).
fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// Days in month `m` of year `y`.
fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        2 => {
            if is_leap(y) {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Add `n` days to `(y, m, d)`, handling month/year rollover.
fn add_days(mut y: i64, mut m: i64, mut d: i64, mut n: i64) -> (i64, i64, i64) {
    while n > 0 {
        d += 1;
        if d > days_in_month(y, m) {
            d = 1;
            m += 1;
            if m > 12 {
                m = 1;
                y += 1;
            }
        }
        n -= 1;
    }
    (y, m, d)
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
    // Russian turns go through the bounded peripheral adapter (greeting,
    // small-talk, identity, arithmetic, capitals; honest RU fallback
    // otherwise) — the Kazakh truth path stays untouched, and this works
    // even without the chat engine loaded.  The general-dialog page sets
    // `lang` from its kz/ru selector.
    if v["lang"].as_str() == Some("ru") {
        let reply = adam_dialog::lang_bridge::respond_ru(&text, v["voice_gender"].as_str());
        return json_ok(&json!({ "reply": reply }));
    }
    let Some(engine) = chat else {
        return json_ok(&json!({
            "reply": "Диалог қозғалтқышы жүктелмеген (data/retrieval немесе world_core жоқ)."
        }));
    };
    let mut e = engine.lock().unwrap();
    let ChatEngine { conv, lex, repo } = &mut *e;
    // Voice-derived gender hint from the browser's pitch (F0) estimate —
    // lets the engine open with the correct Kazakh vocative («Ағай» for a
    // male voice, «Апай» for a female one).  For industrial safety this is
    // also an anti-impersonation signal: the worker cannot pass a male
    // voice off as female.  Only trusted values are forwarded.
    if let Some(g @ ("male" | "female" | "child")) = v["voice_gender"].as_str() {
        conv.session
            .insert("voice_gender_hint".to_string(), g.to_string());
    }
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
fn load_journal(path: &Path) -> Vec<ProgramRecord> {
    let mut out = Vec::new();
    if let Ok(content) = fs::read_to_string(path) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(record) = serde_json::from_str::<ProgramRecord>(line) {
                out.push(record);
            }
        }
    }
    out
}

/// Append one aggregate program record to the journal as a compact JSON
/// line (append-only JSONL).
fn append_journal(path: &Path, record: &ProgramRecord) {
    let Ok(line) = serde_json::to_string(record) else {
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
