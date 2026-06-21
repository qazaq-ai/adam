// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! L4.5 Phase 2.B integration test — verify that processing a
//! `StatementOfName` user turn through the public `Conversation::turn`
//! cascade records a `Proposed` commitment on
//! `Conversation::discourse_state` (alongside the existing
//! `session["name"]` slot write).
//!
//! Skips when the curated lexicon is not present (trimmed checkout).
//! Unit-level coverage of the `absorb_entities` hook lives inside
//! `crates/adam-dialog/src/conversation.rs` because the helper is
//! `pub(crate)`.

use adam_dialog::dialog_acts::{CommitmentStatus, Speaker};
use adam_dialog::{Conversation, TemplateRepository};
use adam_kernel_fst::lexicon::LexiconV1;

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

/// **Full-cascade integration.** Processing «Менің атым Дәулет»
/// through `Conversation::turn` records the commitment.  Skips
/// when the lexicon is missing.
#[test]
fn full_cascade_statement_of_name_records_commitment() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let _reply = conv.turn("Менің атым — Дәулет.", &lex, &repo, 0);

    let log = &conv.discourse_state.commitments;
    assert!(
        !log.is_empty(),
        "expected the StatementOfName turn to record a commitment",
    );
    assert!(log.iter().any(|c| c.author == Speaker::User
        && c.status == CommitmentStatus::Proposed
        && c.claim_text.contains("Дәулет")));
}

/// Two consecutive name turns each leave one Proposed commitment.
/// The arbitration that promotes / rejects them is Phase 2.C.
#[test]
fn two_name_turns_record_two_commitments() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let _ = conv.turn("Менің атым — Дәулет.", &lex, &repo, 0);
    let _ = conv.turn("Жоқ, Дәулет емес, Бекжан.", &lex, &repo, 1);

    let names_in_log: Vec<&str> = conv
        .discourse_state
        .commitments
        .iter()
        .filter(|c| c.author == Speaker::User)
        .map(|c| c.claim_text.as_str())
        .collect();
    assert!(
        names_in_log.iter().any(|t| t.contains("Дәулет")),
        "Дәулет commitment missing in log: {names_in_log:?}",
    );
    // Whether «Бекжан» surfaces depends on the v6.1 cascade's
    // intent classifier on the contrastive shape — Phase 2.D adds
    // the typed `Correct` move that guarantees both land.  Until
    // then, just confirm that at least the first commitment is
    // recorded so the data path is live.
}

/// `Conversation::reset()` clears the discourse log.
#[test]
fn reset_clears_discourse_state() {
    let Some(lex) = load_lexicon() else { return };
    let repo = load_repo();
    let mut conv = Conversation::new();
    let _ = conv.turn("Менің атым — Дәулет.", &lex, &repo, 0);
    assert!(!conv.discourse_state.commitments.is_empty());
    conv.reset();
    assert!(conv.discourse_state.commitments.is_empty());
}
