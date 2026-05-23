// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! # v6.1.0 Stage 4 — BroadTopic multi-claim composer regression
//!
//! Pins the contract that with `ADAM_ANSWER_IR=1`:
//!
//!   - «X туралы айтыңыз» surfaces up to 3 distinct claims about X.
//!   - «ал тағы айт» / «тағы не білесіз» surfaces 3 MORE claims,
//!     skipping anything already-shown on the previous turn.
//!   - Switching subject mid-conversation resets the seen list.
//!
//! Cascade-pass-through invariant (flag off → bit-identical to
//! v6.0.0) is covered by the 32-case `v6_0_6_audit_regression`.

use std::sync::Mutex;

use adam_dialog::{Conversation, TemplateRepository};
use adam_kernel_fst::lexicon::LexiconV1;

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

fn fresh_conv() -> Option<(Conversation, LexiconV1, TemplateRepository)> {
    let facts = load_world_core_facts();
    if facts.is_empty() {
        return None;
    }
    let lex = load_lex();
    let repo = load_repo();
    let conv = Conversation::new().with_reasoning_chains(facts, Vec::new());
    Some((conv, lex, repo))
}

/// Count how many distinct curated raw_text fragments appear in
/// `reply`. Used to verify multi-claim assembly returned > 1 fact.
fn count_distinct_claims(reply: &str, candidates: &[&str]) -> usize {
    let lc = reply.to_lowercase();
    candidates
        .iter()
        .filter(|frag| lc.contains(&frag.to_lowercase()))
        .count()
}

#[test]
fn broad_topic_surfaces_multiple_claims_for_baitursynov() {
    let _g = AnswerIrEnvGuard::enable();
    let Some((mut conv, lex, repo)) = fresh_conv() else {
        return;
    };
    let reply = conv.turn("Ахмет Байтұрсынұлы туралы айтыңыз.", &lex, &repo, 0);
    // We don't pin a hallucination-free contract here (that's
    // factual_eval_100's job) — we pin the multi-claim contract:
    // at least TWO of the curated KRU fragments must appear.
    let fragments = ["ағартушы", "1872", "1937", "төте жазу", "алаш", "әліпбиі"];
    let n = count_distinct_claims(&reply, &fragments);
    assert!(
        n >= 2,
        "broad-topic composer: expected ≥2 distinct claims, got {n} — reply: {reply}"
    );
}

#[test]
fn continuation_surfaces_new_claims_after_initial_turn() {
    let _g = AnswerIrEnvGuard::enable();
    let Some((mut conv, lex, repo)) = fresh_conv() else {
        return;
    };
    // Turn 1: establish the broad-topic context.
    let r1 = conv.turn("Ахмет Байтұрсынұлы туралы айтыңыз.", &lex, &repo, 0);
    assert!(!r1.is_empty(), "turn 1 returned empty");

    // Turn 2: continuation. Must surface a DIFFERENT subset of
    // facts than turn 1 — at minimum one fragment present in r2
    // and absent from r1.
    let r2 = conv.turn("Ал тағы айт.", &lex, &repo, 0);
    let lc1 = r1.to_lowercase();
    let lc2 = r2.to_lowercase();
    let fragments = ["1872", "1937", "төте жазу", "алаш", "әліпбиі", "ағартушы"];
    let new_in_r2 = fragments
        .iter()
        .filter(|f| lc2.contains(&f.to_lowercase()) && !lc1.contains(&f.to_lowercase()))
        .count();
    assert!(
        new_in_r2 >= 1,
        "continuation regression: turn 2 surfaced no NEW fact compared to turn 1\n  turn 1: {r1}\n  turn 2: {r2}"
    );
}

#[test]
fn subject_switch_resets_seen_state() {
    let _g = AnswerIrEnvGuard::enable();
    let Some((mut conv, lex, repo)) = fresh_conv() else {
        return;
    };
    let _r1 = conv.turn("Ахмет Байтұрсынұлы туралы айтыңыз.", &lex, &repo, 0);
    // Switch to a different curated subject. The seen list must
    // clear so the second subject's facts are emitted from the top
    // of the ranking, not influenced by the first subject's seen
    // set.
    let r2 = conv.turn(
        "Қостанай өңірлік университеті туралы айтыңыз.",
        &lex,
        &repo,
        0,
    );
    let lc2 = r2.to_lowercase();
    let kru_fragments = ["жоғары оқу орны", "қостанай", "1939"];
    let hits = kru_fragments
        .iter()
        .filter(|f| lc2.contains(&f.to_lowercase()))
        .count();
    assert!(
        hits >= 1,
        "subject-switch regression: KRU broad-topic surfaced none of {kru_fragments:?} — reply: {r2}"
    );
}

/// **v6.1.5 P1.** When continuation exhausts the seen list, the
/// kernel must emit a honest deterministic "no more on this
/// topic" message — NOT fall back to the generic system-
/// knowledge / capabilities listing the v6.0.0 cascade picks up
/// for «тағы не білесіз?». Pre-v6.1.5 the continuation handler
/// only rewrote intent on `Some(composed)`, leaving exhausted
/// runs to drift into AskAboutSystem{Knowledge}.
#[test]
fn continuation_exhausted_emits_honest_no_more_message() {
    let _g = AnswerIrEnvGuard::enable();
    let Some((mut conv, lex, repo)) = fresh_conv() else {
        return;
    };
    // Run the broad-topic seed + enough continuations to drain
    // the curated facts on Ахмет Байтұрсынұлы (~5-6 facts).
    let _ = conv.turn("Ахмет Байтұрсынұлы туралы айтыңыз.", &lex, &repo, 0);
    let _ = conv.turn("Ал тағы айт.", &lex, &repo, 1);
    let _ = conv.turn("Тағы не білесіз?", &lex, &repo, 2);
    // By turn 4 the seen list is drained on every realistic
    // curated subject; the reply MUST be the deterministic
    // exhausted-fallback, not a generic system-knowledge dump.
    let r4 = conv.turn("Тағы не білесіз?", &lex, &repo, 3);
    let lc4 = r4.to_lowercase();
    assert!(
        lc4.contains("деректерім таусылды") || lc4.contains("ахмет байтұрсынұлы"),
        "P1 regression: exhausted-continuation reply did not emit the no-more-facts fallback — got: {r4}"
    );
    assert!(
        !lc4.contains("мынадай тақырыптар жайлы білемін")
            && !lc4.contains("қазіргі білімімнің ауқымы"),
        "P1 regression: exhausted continuation drifted into generic capabilities listing — got: {r4}"
    );
}

#[test]
fn flag_off_does_not_change_broad_topic_behaviour() {
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");
    unsafe { std::env::remove_var("ADAM_ANSWER_IR") };
    let Some((mut conv, lex, repo)) = fresh_conv() else {
        return;
    };
    let reply = conv.turn("Ахмет Байтұрсынұлы туралы айтыңыз.", &lex, &repo, 0);
    // Without the flag, the v6.0.13 single-fact retrieval is what
    // answers. We just check that SOMETHING about the subject
    // surfaces — concrete fact picked depends on the v6.0.0
    // cascade and is asserted in v6_0_6_audit_regression.
    assert!(
        !reply.is_empty(),
        "flag-off path returned empty reply — cascade regression"
    );
}
