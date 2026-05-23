// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! # v6.1.0 Stage 3 — predicate-aware retrieval regression
//!
//! Pins the contract that with `ADAM_ANSWER_IR=1` set, the dialog
//! kernel routes typed-predicate questions to the typed fact via
//! `predicate_focus::detect` + the predicate-aware retrieval probe
//! wired in `conversation::turn_with_trace` (see Stage 3 block
//! before the v6.0.13 PREDICATE_KEYWORDS fallback).
//!
//! The default-path / cascade-pass-through invariant — that with
//! `ADAM_ANSWER_IR` unset the kernel behaves bit-identically to
//! v6.0.0 — is covered by the 32-case `v6_0_6_audit_regression`
//! suite, which is already green on this branch.
//!
//! These cases exercise the QUERY shapes the Codex 2026-05-22
//! audit flagged as broken under the v6.0.13 keyword hack:
//!
//!   - «Ахмет Байтұрсынұлы қашан туылған?» → date fact (BornIn)
//!   - «КРУ қашан құрылған?» → 1939 fact (FoundedIn)
//!   - «Жасанды интеллект туралы заң қашан күшіне енді?» → 2026
//!     fact (EffectiveFrom)
//!   - «Жасанды интеллект туралы заң қандай санаттарға жіктейді?»
//!     → тәуекел деңгейі (Classifies)
//!   - «Қостанай өңірлік университеті қайда орналасқан?» →
//!     қостанай (LocatedIn)
//!   - «Алаш қозғалысының жетекшілерінің бірі кім?» — Relational
//!     (genitive) currently has no typed mapping; the path falls
//!     back to v6.0.13. Not pinned here.
//!
//! Each case sets `ADAM_ANSWER_IR=1` in the test process for the
//! lifetime of the test and clears it after — adjacent tests must
//! not depend on the env state. Tests are NOT run in parallel by
//! default in adam-dialog's harness, so the env var is shared
//! safely turn-by-turn (a serialised-by-default invariant the
//! existing `cargo_verify` and `factual_eval_100` tests rely on
//! too).

use std::sync::Mutex;

use adam_dialog::{Conversation, TemplateRepository};
use adam_kernel_fst::lexicon::LexiconV1;

/// Env writes serialise across these tests so two cases don't race
/// each other on `ADAM_ANSWER_IR`. Without this guard, parallel
/// test threads could see each other's flag mid-turn.
static ENV_LOCK: Mutex<()> = Mutex::new(());

struct AnswerIrEnvGuard {
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl AnswerIrEnvGuard {
    fn enable() -> Self {
        let guard = ENV_LOCK.lock().expect("env lock poisoned");
        unsafe { std::env::set_var("ADAM_ANSWER_IR", "1") };
        Self { _guard: guard }
    }
}

impl Drop for AnswerIrEnvGuard {
    fn drop(&mut self) {
        unsafe { std::env::remove_var("ADAM_ANSWER_IR") };
    }
}

fn load_lex() -> LexiconV1 {
    let curated = "../../data/tokenizer/segmentation_roots.json";
    let apertium = "../../data/lexicon_v1/apertium_imported_roots.json";
    LexiconV1::load(curated, apertium).expect("lexicon load failed")
}

fn load_repo() -> TemplateRepository {
    TemplateRepository::load_default().expect("templates v1.toml must exist")
}

fn load_world_core_facts() -> Vec<adam_reasoning::Fact> {
    let world_core_dir = std::path::Path::new("../../data/world_core");
    if !world_core_dir.exists() {
        return Vec::new();
    }
    adam_reasoning::world_core::load_world_core_facts(world_core_dir)
        .expect("world_core facts must load")
}

fn run(input: &str) -> String {
    let _guard = AnswerIrEnvGuard::enable();
    let lex = load_lex();
    let repo = load_repo();
    let facts = load_world_core_facts();
    if facts.is_empty() {
        // world_core not present (e.g. CI sandbox without
        // data/world_core mount) — skip gracefully.
        return String::new();
    }
    let mut conv = Conversation::new().with_reasoning_chains(facts, Vec::new());
    conv.turn(input, &lex, &repo, 0)
}

#[test]
fn born_in_lands_date_fact_for_baitursynov() {
    let reply = run("Ахмет Байтұрсынұлы қашан туылған?");
    let lc = reply.to_lowercase();
    assert!(
        lc.contains("1872"),
        "BornIn routing failed: reply did not mention 1872 — got: {reply}"
    );
}

#[test]
fn founded_in_lands_1939_for_kru() {
    let reply = run("Қостанай өңірлік университеті қашан құрылған?");
    let lc = reply.to_lowercase();
    assert!(
        lc.contains("1939"),
        "FoundedIn routing failed: reply did not mention 1939 — got: {reply}"
    );
}

#[test]
fn effective_from_lands_2026_for_ai_law() {
    let reply = run("Жасанды интеллект туралы заң қашан күшіне енді?");
    let lc = reply.to_lowercase();
    assert!(
        lc.contains("2026") && (lc.contains("қаңтар") || lc.contains("18")),
        "EffectiveFrom routing failed: reply did not mention 2026 / 18 қаңтар — got: {reply}"
    );
}

#[test]
fn classifies_lands_risk_level_for_ai_law() {
    let reply = run("Жасанды интеллект туралы заң қандай санаттарға жіктейді?");
    let lc = reply.to_lowercase();
    assert!(
        lc.contains("тәуекел"),
        "Classifies routing failed: reply did not mention тәуекел — got: {reply}"
    );
}

#[test]
fn located_in_lands_kostanay_for_kru() {
    let reply = run("Қостанай өңірлік университеті қайда орналасқан?");
    let lc = reply.to_lowercase();
    assert!(
        lc.contains("қостанай"),
        "LocatedIn routing failed: reply did not mention қостанай — got: {reply}"
    );
}

#[test]
fn died_in_lands_1937_for_baitursynov() {
    let reply = run("Ахмет Байтұрсынұлы қашан қайтыс болды?");
    let lc = reply.to_lowercase();
    assert!(
        lc.contains("1937"),
        "DiedIn routing failed: reply did not mention 1937 — got: {reply}"
    );
}

#[test]
fn member_of_lands_alash_for_baitursynov() {
    let reply = run("Ахмет Байтұрсынұлы кімнің мүшесі?");
    let lc = reply.to_lowercase();
    assert!(
        lc.contains("алаш"),
        "MemberOf routing failed: reply did not mention алаш — got: {reply}"
    );
}

/// **v6.1.5 — 2026-05-23 audit fix P0 #1.** Pins the named-after
/// regression. Pre-fix `detect_ask_name` substring-matched «атым»
/// and fired on «атымен» (instrumental — «with [the] name [of]»),
/// routing «X кімнің атымен аталған?» to `Intent::AskName` and
/// emitting the persona reply «Атым — адам.» BEFORE the
/// `PredicateFocus::NamedAfter` retrieval probe got the turn.
/// `contains_word` fix in semantics.rs scopes the match to word
/// boundaries.
#[test]
fn named_after_does_not_collide_with_ask_name() {
    let reply = run("Қостанай өңірлік университеті кімнің атымен аталған?");
    let lc = reply.to_lowercase();
    assert!(
        lc.contains("ахмет байтұрсынұлы"),
        "P0 #1 regression: reply did not surface named_after fact — got: {reply}"
    );
    assert!(
        !lc.contains("атым — адам"),
        "P0 #1 regression: reply mis-fired AskName persona path — got: {reply}"
    );
}

/// **v6.1.5 P0 #1 guard.** Ensure the word-boundary tightening
/// did not regress the legitimate self-recall AskName path.
#[test]
fn ask_name_self_recall_still_works() {
    let _guard = AnswerIrEnvGuard::enable();
    let lex = load_lex();
    let repo = load_repo();
    let facts = load_world_core_facts();
    if facts.is_empty() {
        return;
    }
    let mut conv = Conversation::new().with_reasoning_chains(facts, Vec::new());
    let _ = conv.turn("Менің атым Дәулет.", &lex, &repo, 0);
    let reply = conv.turn("Менің атым кім?", &lex, &repo, 1);
    let lc = reply.to_lowercase();
    assert!(
        lc.contains("дәулет"),
        "AskName self-recall regression after P0 #1 fix — got: {reply}"
    );
}

/// **v6.1.5 P0 #2.** Pin the contrastive-farewell-rejection
/// shape: «Сау болыңыз емес, әлі сөйлесейік» (= "not goodbye,
/// let's keep talking") must NOT emit a Farewell reply. Pre-fix
/// real REPL produced «Аман бол» because
/// `split_compound_utterance` comma-cut the input before
/// `detect_farewell`'s «емес»-token guard could see the
/// continuation. v6.1.5 adds a contrastive-farewell-rejection
/// detector that both bails the splitter AND rewrites `input`
/// at the top of `turn_with_trace` to just the continuation.
#[test]
fn contrastive_farewell_rejection_does_not_fire_farewell() {
    let _guard = AnswerIrEnvGuard::enable();
    let lex = load_lex();
    let repo = load_repo();
    let facts = load_world_core_facts();
    if facts.is_empty() {
        return;
    }
    let mut conv = Conversation::new().with_reasoning_chains(facts, Vec::new());
    let reply = conv.turn("Сау болыңыз емес, әлі сөйлесейік.", &lex, &repo, 0);
    let lc = reply.to_lowercase();
    assert!(
        !lc.contains("аман бол")
            && !lc.contains("сау бол")
            && !lc.contains("қош бол")
            && !lc.contains("кездескенше"),
        "P0 #2 regression: contrastive-farewell-rejection emitted a Farewell — got: {reply}"
    );
}

#[test]
fn flag_off_does_not_change_default_behaviour() {
    // Sanity check: without the flag, the v6.0.13 keyword fallback
    // is what answers — and it already lands the 1872 date for
    // this canonical case, so the cascade-pass-through invariant
    // includes this assertion. Running it here is a self-check
    // that the test harness picks up the env state correctly.
    // We do NOT set ADAM_ANSWER_IR here.
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");
    unsafe { std::env::remove_var("ADAM_ANSWER_IR") };
    let lex = load_lex();
    let repo = load_repo();
    let facts = load_world_core_facts();
    if facts.is_empty() {
        return;
    }
    let mut conv = Conversation::new().with_reasoning_chains(facts, Vec::new());
    let reply = conv.turn("Ахмет Байтұрсынұлы қашан туылған?", &lex, &repo, 0);
    let lc = reply.to_lowercase();
    assert!(
        lc.contains("1872"),
        "v6.0.13 fallback regression: reply did not mention 1872 — got: {reply}"
    );
}
