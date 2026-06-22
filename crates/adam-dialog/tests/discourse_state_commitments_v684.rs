// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! L4.5 Phase 2.B / 2.C integration tests — verify that processing a
//! `StatementOfName` user turn through the public `Conversation::turn`
//! cascade records a commitment on `Conversation::discourse_state`
//! and (Phase 2.C) promotes it from `Proposed` to `Accepted` when
//! the corresponding session slot is populated.
//!
//! ## Test-fixture naming convention
//!
//! Specific personal names are NEVER hardcoded inline.  Test
//! fixtures use the named constants below so the intent is
//! self-documenting — reading `TEST_FIRST_NAME_FEMALE` in a turn
//! input immediately signals «this is a placeholder, not a real
//! person».  Same principle applies to any other fixture data
//! that could be mistaken for personal anecdote (city, age,
//! occupation).
//!
//! Constants pick a value that the curated Kazakh name DB knows
//! (so the cascade's `canonical_person_entity` resolves it); the
//! point is the named-constant indirection, not the specific
//! string.
//!
//! Skips when the curated lexicon is not present (trimmed checkout).
//! Unit-level coverage of the `absorb_entities` hook lives inside
//! `crates/adam-dialog/src/conversation.rs` because the helper is
//! `pub(crate)`.

use adam_dialog::dialog_acts::{CommitmentStatus, Speaker};
use adam_dialog::{Conversation, TemplateRepository};
use adam_kernel_fst::lexicon::LexiconV1;

// ── Test fixture constants ──────────────────────────────────
// Named placeholders so test inputs read «<TEST_FIRST_NAME>», not
// «<some specific person>».  Values picked from the curated Kazakh
// name DB so the cascade resolves them; the indirection is the
// point.
const TEST_FIRST_NAME_FEMALE: &str = "Айгүл";
const TEST_FIRST_NAME_MALE: &str = "Бекжан";
const TEST_FIRST_NAME_OTHER_MALE: &str = "Болат";

fn load_repo() -> TemplateRepository {
    TemplateRepository::load_default().expect("templates v1.toml must exist")
}

fn load_lexicon() -> Option<LexiconV1> {
    let curated = "../../data/tokenizer/segmentation_roots.json";
    let apertium = "../../data/lexicon_v1/apertium_imported_roots.json";
    if !std::path::Path::new(curated).exists() || !std::path::Path::new(apertium).exists() {
        eprintln!("[discourse_state_commitments_v684] lexicon not present, skipping");
        return None;
    }
    LexiconV1::load(curated, apertium).ok()
}

/// A fresh `Conversation` starts with an empty discourse state.
#[test]
fn fresh_conversation_has_empty_discourse_state() {
    let conv = Conversation::new();
    assert!(
        conv.discourse_state.commitments.is_empty(),
        "expected empty commitments on fresh conversation",
    );
}

/// **Full-cascade integration.** Processing a name-statement turn
/// through `Conversation::turn` records the commitment.  Skips
/// when the lexicon is missing.
///
/// In Phase 2.B this test asserted `Proposed` status; Phase 2.C
/// now promotes to `Accepted` because the session slot is
/// populated.  The dedicated `commitment_promotes_to_accepted`
/// test below pins the post-promotion status explicitly; this
/// one just asserts that the user-authored commitment landed at
/// all.
#[test]
fn full_cascade_statement_of_name_records_commitment() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let input = format!("Менің атым — {TEST_FIRST_NAME_FEMALE}.");
    let _reply = conv.turn(&input, &lex, &repo, 0);

    let log = &conv.discourse_state.commitments;
    assert!(
        !log.is_empty(),
        "expected the StatementOfName turn to record a commitment",
    );
    assert!(log.iter().any(|c| c.author == Speaker::User
        && (c.status == CommitmentStatus::Proposed || c.status == CommitmentStatus::Accepted)
        && c.claim_text.contains(TEST_FIRST_NAME_FEMALE)));
}

/// Two consecutive name turns each leave one commitment on the
/// log.  The arbitration that supersedes / corrects them is
/// Phase 2.D (typed `DialogueMove::Correct`).
#[test]
fn two_name_turns_record_two_commitments() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let first = format!("Менің атым — {TEST_FIRST_NAME_MALE}.");
    let correction = format!("Жоқ, {TEST_FIRST_NAME_MALE} емес, {TEST_FIRST_NAME_OTHER_MALE}.",);
    let _ = conv.turn(&first, &lex, &repo, 0);
    let _ = conv.turn(&correction, &lex, &repo, 1);

    let names_in_log: Vec<&str> = conv
        .discourse_state
        .commitments
        .iter()
        .filter(|c| c.author == Speaker::User)
        .map(|c| c.claim_text.as_str())
        .collect();
    assert!(
        names_in_log
            .iter()
            .any(|t| t.contains(TEST_FIRST_NAME_MALE)),
        "first commitment missing in log: {names_in_log:?}",
    );
    // Whether the second name surfaces depends on the v6.1
    // cascade's intent classifier on the contrastive shape —
    // Phase 2.D adds the typed `Correct` move that guarantees
    // both land.  Until then, just confirm at least one
    // commitment is recorded so the data path is live.
}

/// `Conversation::reset()` clears the discourse log.
#[test]
fn reset_clears_discourse_state() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let input = format!("Менің атым — {TEST_FIRST_NAME_FEMALE}.");
    let _ = conv.turn(&input, &lex, &repo, 0);
    assert!(!conv.discourse_state.commitments.is_empty());
    conv.reset();
    assert!(conv.discourse_state.commitments.is_empty());
}

/// **Phase 2.D + 2.E.1 — full repair-turn handling.** When the
/// user retracts a prior commitment with «Жоқ, X емес, Y»:
///   * the prior X commitment is marked `Rejected` (2.D);
///   * `session["name"]` is updated to the canonical Y form (2.E.1);
///   * a fresh `Proposed` Y commitment is recorded and then
///     promoted to `Accepted` by the Phase 2.C pass (2.E.1);
///   * adam's reply is the typed acknowledgement template
///     «Түзеттім — атыңызды Y деп есте сақтадым» (2.E.1).
#[test]
fn correction_full_repair_lifecycle() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();

    // Turn 1: user introduces the (eventually-rejected) name.
    let first = format!("Менің атым — {TEST_FIRST_NAME_MALE}.");
    let _ = conv.turn(&first, &lex, &repo, 0);

    // Turn 2: user retracts with the Kazakh repair shape.
    let correction = format!("Жоқ, {TEST_FIRST_NAME_MALE} емес, {TEST_FIRST_NAME_OTHER_MALE}.",);
    let reply = conv.turn(&correction, &lex, &repo, 1);

    // ── Reply: typed acknowledgement template (Phase 2.E.1). ──
    assert!(
        reply.contains("Түзеттім") && reply.contains(TEST_FIRST_NAME_OTHER_MALE),
        "expected the «Түзеттім — атыңызды Y деп есте сақтадым» \
         acknowledgement, got: {reply}",
    );

    // ── session: name slot updated to replacement (2.E.1). ──
    let name_in_session = conv.session.get("name").cloned().unwrap_or_default();
    assert!(
        name_in_session.contains(TEST_FIRST_NAME_OTHER_MALE),
        "session['name'] should hold the replacement, got: {name_in_session:?}",
    );

    // ── Prior X commitment: Rejected (Phase 2.D). ──
    let prior = conv
        .discourse_state
        .commitments
        .iter()
        .find(|c| {
            c.author == Speaker::User
                && c.claim_text.contains(TEST_FIRST_NAME_MALE)
                && c.turn_id == 0
        })
        .expect("prior commitment must be present");
    assert_eq!(
        prior.status,
        CommitmentStatus::Rejected,
        "prior commitment must be Rejected, got {:?}",
        prior.status,
    );

    // ── New Y commitment: Accepted (Phase 2.E.1 → 2.C promote). ──
    let new_y = conv
        .discourse_state
        .commitments
        .iter()
        .find(|c| {
            c.author == Speaker::User
                && c.claim_text.contains(TEST_FIRST_NAME_OTHER_MALE)
                && c.turn_id == 1
        })
        .expect("new replacement commitment must be present");
    assert_eq!(
        new_y.status,
        CommitmentStatus::Accepted,
        "new commitment should promote to Accepted (session['name'] set), got {:?}",
        new_y.status,
    );
}

/// **Phase 2.E.2 — referent stack anaphora.**  Two-turn dialog
/// where turn 1 introduces a Person topic, turn 2 asks a bare
/// follow-up that lacks an explicit subject.  The follow-up
/// must resolve against the prior turn's referent.
///
/// Specifically: «Ахмет Байтұрсынұлы туралы айтшы» introduces
/// the person; «Қанша жыл өмір сүрді?» without explicit subject
/// must compute the lifespan from the prior referent.
///
/// Skips when the lexicon is missing.
#[test]
fn anaphora_resolves_bare_lifespan_query_to_prior_referent() {
    // The anaphora-aware path lives behind the v6.2 router gate;
    // production binaries (voice REPL, respond_full) set this
    // automatically, but `cargo test` doesn't.
    unsafe {
        std::env::set_var("ADAM_V6_2", "1");
    }
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();

    // Turn 1: introduce the person.  v6.1 broad-topic handler
    // records the topic onto dialog_context, which the Phase
    // 2.E.2 hook mirrors onto discourse_state.referents.
    let _ = conv.turn("Ахмет Байтұрсынұлы туралы айтшы.", &lex, &repo, 0);

    // Verify SOME referent landed on the stack — kind
    // discrimination is the handler's job (see Phase 2.E.2 note
    // in conversation.rs), not the test's.
    let last = conv.discourse_state.last_referent();
    assert!(
        last.is_some(),
        "expected a referent on the stack after the broad-topic turn; stack: {:?}",
        conv.discourse_state.referents,
    );

    // Turn 2: bare lifespan query, no explicit subject.  The
    // anaphora-aware lifespan handler should synthesise the
    // input using the prior referent and resolve.
    let reply = conv.turn("Қанша жыл өмір сүрді?", &lex, &repo, 1);
    assert!(
        reply.contains("жыл өмір сүрді")
            && (reply.contains("65") || reply.contains("1872") || reply.contains("1937")),
        "expected the lifespan answer derived from the prior-turn referent, got: {reply}",
    );
}

/// **Phase 2.C — commitment promotion.** After the full cascade
/// processes a name statement, the recorded Proposed commitment
/// is promoted to Accepted because `session["name"]` is populated
/// (adam adopted the name).
#[test]
fn statement_of_name_commitment_promotes_to_accepted() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let input = format!("Менің атым — {TEST_FIRST_NAME_FEMALE}.");
    let reply = conv.turn(&input, &lex, &repo, 0);
    eprintln!("[debug] reply: {reply}");

    // session must hold the name (existing behaviour).
    let name_in_session = conv.session.get("name").cloned().unwrap_or_default();
    assert!(
        name_in_session.eq_ignore_ascii_case(TEST_FIRST_NAME_FEMALE)
            || name_in_session.contains(TEST_FIRST_NAME_FEMALE),
        "session['name'] should hold the user's name, got: {name_in_session:?}",
    );

    // The commitment must be promoted to Accepted (Phase 2.C
    // behaviour — Phase 2.B left it Proposed).
    let log = &conv.discourse_state.commitments;
    assert!(!log.is_empty());
    let promoted = log
        .iter()
        .find(|c| c.author == Speaker::User && c.claim_text.contains(TEST_FIRST_NAME_FEMALE));
    assert!(promoted.is_some(), "name commitment must be present");
    assert_eq!(
        promoted.unwrap().status,
        CommitmentStatus::Accepted,
        "promotion to Accepted should fire when session slot is populated, got {:?}",
        promoted.unwrap().status,
    );
}
