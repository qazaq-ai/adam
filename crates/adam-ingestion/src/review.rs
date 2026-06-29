// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Human-review session orchestration.
//!
//! Bridges the validator (which sorts candidates into
//! `AutoAccepted` / `AutoRejected` / `NeedsReview`) and the
//! integrator (which writes approved candidates to
//! `world_core`).  Everything in `NeedsReview` lands in this
//! module's lap.
//!
//! ## Why a [`Reviewer`] trait
//!
//! The session loop (iterate NeedsReview candidates, call
//! something for each one, update the store) is the same
//! whether the reviewer is a human at a TTY or a scripted
//! test fixture.  Trait-shape lets the TTY binary stay a
//! thin wrapper around a stdin reader and lets the unit
//! tests exercise the loop deterministically with a
//! pre-recorded decision sequence.
//!
//! ## Decisions
//!
//! [`ReviewDecision`] is intentionally narrow — four
//! variants, no «edit-in-place» yet.  «Edit» is a future
//! commit; landing it cleanly needs interactive line-edit
//! UI, which is out of scope for this phase.

use crate::candidate::CandidateFact;
use crate::status::IngestionStatus;
use crate::store::{CandidateStore, StoreError};

/// What the reviewer decided about a single candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewDecision {
    /// Promote the candidate to `ApprovedByHuman`.  The
    /// integrator will later pick it up and write into
    /// `world_core` jsonl.
    Approve,
    /// Mark the candidate `RejectedByHuman`.  Terminal
    /// failure — auditable, never re-surfaces.
    Reject,
    /// Leave the candidate as-is and move on.  Useful when
    /// the reviewer wants to come back to it later or
    /// resolve a referenced curated entry first.
    Skip,
    /// End the review session.  Remaining NeedsReview
    /// candidates stay untouched.
    Quit,
}

/// Trait for the «something that decides per candidate»
/// half of the session loop.  Implemented by the TTY
/// binary's stdin reader and by scripted test fixtures.
pub trait Reviewer {
    /// Inspect a candidate and return a decision.  The
    /// session loop does NOT enforce a maximum runtime —
    /// implementations that need a timeout enforce it
    /// themselves.
    fn review_fact(&mut self, fact: &CandidateFact) -> ReviewDecision;
}

/// Aggregate counters returned by [`run_review_session`].
/// Lets a wrapper print a one-line summary at the end of a
/// session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReviewSummary {
    /// How many candidates were examined this session.
    pub examined: usize,
    /// How many were promoted to ApprovedByHuman.
    pub approved: usize,
    /// How many were demoted to RejectedByHuman.
    pub rejected: usize,
    /// How many were left untouched (Skip).
    pub skipped: usize,
    /// Whether the session ended via Quit (vs. ran out of
    /// candidates).
    pub quit: bool,
}

/// Walk every `NeedsReview` fact candidate in `store`,
/// passing each one to `reviewer`.  Decisions persist
/// immediately via [`CandidateStore::update_fact_status`]
/// so a crash mid-session doesn't lose accepted decisions.
///
/// Returns the aggregate [`ReviewSummary`].  Propagates any
/// `StoreError` raised by the underlying store calls — in
/// practice this only fires when the on-disk file is
/// truncated or has malformed JSON, both of which mean the
/// caller wants to abort the session anyway.
pub fn run_review_session(
    store: &CandidateStore,
    reviewer: &mut dyn Reviewer,
) -> Result<ReviewSummary, StoreError> {
    let mut summary = ReviewSummary::default();
    // Snapshot the queue once at session start so a long
    // session sees a stable list — new candidates added by
    // a concurrent extractor mid-session won't be picked
    // up here.  (Pipeline runs stages sequentially in
    // practice, so concurrent extractors aren't expected,
    // but the snapshot avoids the surprise.)
    let initial = store.load_facts()?;
    for fact in initial.into_iter() {
        if fact.status != IngestionStatus::NeedsReview {
            continue;
        }
        summary.examined += 1;
        let decision = reviewer.review_fact(&fact);
        match decision {
            ReviewDecision::Approve => {
                store.update_fact_status(
                    &fact.id,
                    IngestionStatus::ApprovedByHuman,
                    "human review: approve",
                )?;
                summary.approved += 1;
            }
            ReviewDecision::Reject => {
                store.update_fact_status(
                    &fact.id,
                    IngestionStatus::RejectedByHuman,
                    "human review: reject",
                )?;
                summary.rejected += 1;
            }
            ReviewDecision::Skip => {
                summary.skipped += 1;
            }
            ReviewDecision::Quit => {
                summary.quit = true;
                break;
            }
        }
    }
    Ok(summary)
}

/// Pretty-print a candidate for a TTY reviewer.  Pure
/// function — returns the string instead of writing to
/// stdout, so test fixtures can assert against it.
pub fn render_fact_for_review(fact: &CandidateFact) -> String {
    let mut s = String::new();
    s.push_str(&format!("─── candidate {} ───\n", fact.id));
    s.push_str(&format!("  subject   : {}\n", fact.subject));
    s.push_str(&format!("  predicate : {}\n", fact.predicate));
    s.push_str(&format!("  object    : {}\n", fact.object));
    if !fact.source_sentence.is_empty() {
        s.push_str(&format!("  context   : {}\n", fact.source_sentence));
    }
    s.push_str(&format!(
        "  source    : {:?} {} (line {:?})\n",
        fact.source.kind, fact.source.identifier, fact.source.line
    ));
    s.push_str(&format!("  confidence: {:.2}\n", fact.confidence));
    s.push_str(&format!("  status    : {:?}\n", fact.status));
    if !fact.notes.is_empty() {
        s.push_str(&format!("  notes     : {}\n", fact.notes));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceRef;
    use std::collections::VecDeque;

    /// Scripted reviewer — pre-recorded decisions, FIFO.
    /// Defaults to Skip when the script is exhausted so
    /// tests can't deadlock the session loop.
    struct ScriptedReviewer {
        decisions: VecDeque<ReviewDecision>,
        seen: Vec<String>,
    }

    impl ScriptedReviewer {
        fn new(seq: &[ReviewDecision]) -> Self {
            Self {
                decisions: seq.iter().copied().collect(),
                seen: Vec::new(),
            }
        }
    }

    impl Reviewer for ScriptedReviewer {
        fn review_fact(&mut self, fact: &CandidateFact) -> ReviewDecision {
            self.seen.push(fact.id.clone());
            self.decisions.pop_front().unwrap_or(ReviewDecision::Skip)
        }
    }

    fn tmp_root(tag: &str) -> std::path::PathBuf {
        let tid = format!("{:?}", std::thread::current().id()).replace([':', ' ', '(', ')'], "_");
        let dir = std::env::temp_dir().join(format!("adam-review-test-{tag}-{tid}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn pending_fact(id: &str, status: IngestionStatus) -> CandidateFact {
        CandidateFact {
            id: id.into(),
            subject: "алматы".into(),
            predicate: "is_a".into(),
            object: "қала".into(),
            source_sentence: "Алматы — қала.".into(),
            source: SourceRef::manual("shaman"),
            status,
            confidence: 0.7,
            created_at: "2026-06-29".into(),
            notes: String::new(),
        }
    }

    #[test]
    fn approve_promotes_to_approved_by_human() {
        let root = tmp_root("approve");
        let store = CandidateStore::open(&root).expect("open");
        store
            .save_facts(&[pending_fact("a", IngestionStatus::NeedsReview)])
            .expect("save");
        let mut reviewer = ScriptedReviewer::new(&[ReviewDecision::Approve]);
        let summary = run_review_session(&store, &mut reviewer).expect("session ok");
        assert_eq!(summary.examined, 1);
        assert_eq!(summary.approved, 1);
        assert_eq!(summary.rejected, 0);
        let loaded = store.load_facts().expect("load");
        assert_eq!(loaded[0].status, IngestionStatus::ApprovedByHuman);
        assert!(loaded[0].notes.contains("approve"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn reject_demotes_to_rejected_by_human() {
        let root = tmp_root("reject");
        let store = CandidateStore::open(&root).expect("open");
        store
            .save_facts(&[pending_fact("a", IngestionStatus::NeedsReview)])
            .expect("save");
        let mut reviewer = ScriptedReviewer::new(&[ReviewDecision::Reject]);
        let summary = run_review_session(&store, &mut reviewer).expect("session ok");
        assert_eq!(summary.rejected, 1);
        let loaded = store.load_facts().expect("load");
        assert_eq!(loaded[0].status, IngestionStatus::RejectedByHuman);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn skip_leaves_status_unchanged() {
        let root = tmp_root("skip");
        let store = CandidateStore::open(&root).expect("open");
        store
            .save_facts(&[pending_fact("a", IngestionStatus::NeedsReview)])
            .expect("save");
        let mut reviewer = ScriptedReviewer::new(&[ReviewDecision::Skip]);
        let summary = run_review_session(&store, &mut reviewer).expect("session ok");
        assert_eq!(summary.skipped, 1);
        let loaded = store.load_facts().expect("load");
        assert_eq!(loaded[0].status, IngestionStatus::NeedsReview);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn quit_stops_loop() {
        let root = tmp_root("quit");
        let store = CandidateStore::open(&root).expect("open");
        store
            .save_facts(&[
                pending_fact("a", IngestionStatus::NeedsReview),
                pending_fact("b", IngestionStatus::NeedsReview),
                pending_fact("c", IngestionStatus::NeedsReview),
            ])
            .expect("save");
        let mut reviewer = ScriptedReviewer::new(&[ReviewDecision::Approve, ReviewDecision::Quit]);
        let summary = run_review_session(&store, &mut reviewer).expect("session ok");
        assert_eq!(summary.examined, 2);
        assert_eq!(summary.approved, 1);
        assert!(summary.quit);
        // `b` was the one we Quit on — should still be
        // NeedsReview.  `c` was never examined.
        let loaded = store.load_facts().expect("load");
        assert_eq!(loaded[0].status, IngestionStatus::ApprovedByHuman);
        assert_eq!(loaded[1].status, IngestionStatus::NeedsReview);
        assert_eq!(loaded[2].status, IngestionStatus::NeedsReview);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn non_needs_review_candidates_skipped() {
        let root = tmp_root("nonreview");
        let store = CandidateStore::open(&root).expect("open");
        store
            .save_facts(&[
                pending_fact("a", IngestionStatus::Pending),
                pending_fact("b", IngestionStatus::AutoAccepted),
                pending_fact("c", IngestionStatus::AutoRejected),
                pending_fact("d", IngestionStatus::NeedsReview),
            ])
            .expect("save");
        let mut reviewer = ScriptedReviewer::new(&[ReviewDecision::Approve]);
        let summary = run_review_session(&store, &mut reviewer).expect("session ok");
        // Only d should be examined.
        assert_eq!(summary.examined, 1);
        assert_eq!(summary.approved, 1);
        assert_eq!(reviewer.seen, vec!["d".to_string()]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn render_fact_for_review_includes_key_fields() {
        let fact = pending_fact("test-id", IngestionStatus::NeedsReview);
        let rendered = render_fact_for_review(&fact);
        assert!(rendered.contains("test-id"));
        assert!(rendered.contains("алматы"));
        assert!(rendered.contains("is_a"));
        assert!(rendered.contains("қала"));
        assert!(rendered.contains("Алматы — қала."));
        assert!(rendered.contains("NeedsReview"));
    }

    #[test]
    fn empty_queue_yields_empty_summary() {
        let root = tmp_root("empty");
        let store = CandidateStore::open(&root).expect("open");
        let mut reviewer = ScriptedReviewer::new(&[]);
        let summary = run_review_session(&store, &mut reviewer).expect("session ok");
        assert_eq!(summary, ReviewSummary::default());
        let _ = std::fs::remove_dir_all(&root);
    }
}
