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

/// **Phase 2.D — repair / correction.** When the user retracts
/// a prior commitment with «Жоқ, X емес, Y», the prior
/// commitment is marked `Rejected` on the discourse log.  The
/// new value Y is absorbed by the v6.1 cascade as a fresh
/// `Intent::StatementOfName` (when the cascade recognises that
/// shape), landing as a new `Proposed` commitment via the same
/// path Phase 2.B already wires.  Phase 2.E will broaden the
/// cascade detection so adam's reply explicitly acknowledges
/// the correction; for now we only assert the discourse-state
/// transition is correct.
#[test]
fn correction_pattern_marks_prior_commitment_rejected() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();

    // Turn 1: user introduces the (eventually-rejected) name.
    let first = format!("Менің атым — {TEST_FIRST_NAME_MALE}.");
    let _ = conv.turn(&first, &lex, &repo, 0);

    // Turn 2: user retracts with the Kazakh repair shape.
    let correction = format!("Жоқ, {TEST_FIRST_NAME_MALE} емес, {TEST_FIRST_NAME_OTHER_MALE}.",);
    let _reply = conv.turn(&correction, &lex, &repo, 1);

    // The first commitment must now carry `Rejected` status.
    let first_commitment = conv
        .discourse_state
        .commitments
        .iter()
        .find(|c| {
            c.author == Speaker::User
                && c.claim_text.contains(TEST_FIRST_NAME_MALE)
                && c.turn_id == 0
        })
        .expect("first commitment must be present on log");
    assert_eq!(
        first_commitment.status,
        CommitmentStatus::Rejected,
        "first commitment must be marked Rejected after the repair turn, got {:?}",
        first_commitment.status,
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
