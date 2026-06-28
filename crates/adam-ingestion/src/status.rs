// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Ingestion-queue status state machine.
//!
//! Every candidate moves through a strict subset of allowed
//! transitions.  Out-of-order transitions return
//! [`StatusTransitionError`] — the integrator can NOT mark a
//! candidate `IntegratedIntoWorldCore` without it first being
//! `ApprovedByHuman`, the human reviewer can NOT promote a
//! `Pending` candidate to `Approved` without it having
//! cleared the validator (`AutoAccepted` or `NeedsReview`).
//!
//! This is the deterministic-kernel discipline applied to
//! the curation queue — every state transition is auditable.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Where in the queue a candidate currently sits.
///
/// State diagram (legal transitions only):
///
/// ```text
///           Pending
///         /    |    \
///        v     v     v
/// AutoAccepted | AutoRejected
///        |     |
///        |     v
///        | NeedsReview
///        |     |   \
///        v     v    v
/// ApprovedByHuman   RejectedByHuman
///        |
///        v
/// IntegratedIntoWorldCore
/// ```
///
/// `RejectedByHuman` and `AutoRejected` are terminal — once
/// a candidate is dropped, it stays dropped (audit trail).
/// `IntegratedIntoWorldCore` is terminal on the success
/// side — the candidate has been written into the
/// world_core jsonl and is now production data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestionStatus {
    /// Fresh candidate, has not yet been through the
    /// validator.  Default state on creation.
    Pending,
    /// Validator passed all gates AND assigned high
    /// enough confidence to skip human review.  May be
    /// integrated directly.
    AutoAccepted,
    /// Validator rejected outright (duplicate of existing
    /// world_core fact, contradicts curated truth, schema
    /// invalid).  Terminal failure state.
    AutoRejected,
    /// Validator could not decide — confidence between the
    /// auto-accept and auto-reject floors, OR conflicts
    /// with a curated entry that doesn't dominate.  Waits
    /// for a human.
    NeedsReview,
    /// Human reviewer marked OK to integrate.
    ApprovedByHuman,
    /// Human reviewer rejected.  Terminal failure state.
    RejectedByHuman,
    /// Integrator has written this candidate into the
    /// production world_core jsonl.  Terminal success
    /// state.
    IntegratedIntoWorldCore,
}

impl IngestionStatus {
    /// Whether `self → next` is a legal transition per the
    /// state diagram above.  Used by `CandidateStore`
    /// updates so the pipeline can't silently corrupt the
    /// queue.
    pub fn can_transition(self, next: IngestionStatus) -> bool {
        use IngestionStatus::*;
        matches!(
            (self, next),
            (Pending, AutoAccepted)
                | (Pending, AutoRejected)
                | (Pending, NeedsReview)
                | (AutoAccepted, ApprovedByHuman)
                | (AutoAccepted, IntegratedIntoWorldCore)
                | (NeedsReview, ApprovedByHuman)
                | (NeedsReview, RejectedByHuman)
                | (ApprovedByHuman, IntegratedIntoWorldCore)
        )
    }

    /// Whether this status is terminal (no further
    /// transitions allowed).  `RejectedByHuman` /
    /// `AutoRejected` / `IntegratedIntoWorldCore`.
    pub fn is_terminal(self) -> bool {
        use IngestionStatus::*;
        matches!(
            self,
            AutoRejected | RejectedByHuman | IntegratedIntoWorldCore
        )
    }
}

/// Returned when `CandidateStore::update_status` is called
/// with a transition the state machine forbids.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("illegal ingestion-status transition: {from:?} → {to:?}")]
pub struct StatusTransitionError {
    pub from: IngestionStatus,
    pub to: IngestionStatus,
}

#[cfg(test)]
mod tests {
    use super::*;
    use IngestionStatus::*;

    #[test]
    fn legal_transitions() {
        assert!(Pending.can_transition(AutoAccepted));
        assert!(Pending.can_transition(NeedsReview));
        assert!(Pending.can_transition(AutoRejected));
        assert!(AutoAccepted.can_transition(IntegratedIntoWorldCore));
        assert!(NeedsReview.can_transition(ApprovedByHuman));
        assert!(NeedsReview.can_transition(RejectedByHuman));
        assert!(ApprovedByHuman.can_transition(IntegratedIntoWorldCore));
    }

    #[test]
    fn illegal_transitions() {
        // Can't skip review.
        assert!(!Pending.can_transition(ApprovedByHuman));
        assert!(!Pending.can_transition(IntegratedIntoWorldCore));
        // Can't unreject.
        assert!(!RejectedByHuman.can_transition(ApprovedByHuman));
        assert!(!AutoRejected.can_transition(ApprovedByHuman));
        // Can't reintegrate.
        assert!(!IntegratedIntoWorldCore.can_transition(Pending));
        // No self-loops permitted (would mask bugs).
        assert!(!Pending.can_transition(Pending));
    }

    #[test]
    fn terminal_states() {
        assert!(AutoRejected.is_terminal());
        assert!(RejectedByHuman.is_terminal());
        assert!(IntegratedIntoWorldCore.is_terminal());
        assert!(!Pending.is_terminal());
        assert!(!NeedsReview.is_terminal());
        assert!(!AutoAccepted.is_terminal());
        assert!(!ApprovedByHuman.is_terminal());
    }
}
