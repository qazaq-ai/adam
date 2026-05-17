// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! L10-edu — outcome verifier.
//!
//! The single source of truth for declaring a concept mastered.
//! Every other component in the curriculum stack reads mastery
//! state; only this module writes it. That invariant lets us audit
//! "why was concept X marked mastered" with a one-line answer:
//! «outcome::verify_concept declared it on attempt N at threshold
//! T».
//!
//! The rubric is from
//! [`docs/product/qazaq_ai_ustaz_v1.md`](../../../docs/product/qazaq_ai_ustaz_v1.md)
//! «Measurable mastery per concept»:
//!
//! 1. ≥ N test items attempted in a single sitting (N typically 5).
//! 2. Score on that sitting ≥ concept's `mastery_threshold`
//!    (default 0.80; subject experts may set per-concept).
//! 3. The same N-item subset is never re-used for the same learner.
//!    The L9-edu lesson planner enforces this by tracking item ids
//!    consumed; this module trusts the planner's selection.

use crate::concept::{Concept, ConceptGraph, ConceptId};
use crate::error::{CurriculumError, Result};
use crate::learner::LearnerRecord;

/// Minimum items per attempt for it to count toward mastery.
/// Configurable per pillar in a future revision; for v1.0 we use
/// a single global value matching the product-spec default.
pub const MIN_ITEMS_PER_ATTEMPT: u32 = 5;

/// Decision returned by `verify_concept`. `Mastered` means the
/// learner record has been updated to reflect the new state;
/// `NotYet` carries a reason the rubric was not satisfied.
#[derive(Debug, Clone, PartialEq)]
pub enum VerifyOutcome {
    Mastered {
        score: f32,
        threshold: f32,
        items_attempted: u32,
    },
    NotYet {
        reason: NotYetReason,
        score: f32,
        threshold: f32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotYetReason {
    /// Attempt had fewer than `MIN_ITEMS_PER_ATTEMPT` items.
    TooFewItems,
    /// Score on the attempt fell below the concept's threshold.
    BelowThreshold,
    /// One or more prerequisites are not yet mastered. The learner
    /// can still attempt this concept, but mastery doesn't get
    /// granted until prereqs are in.
    PrerequisitesUnmastered,
}

/// Apply the rubric to the latest attempt and update the learner
/// record if it passes. Returns the decision either way.
pub fn verify_concept(
    graph: &ConceptGraph,
    learner: &mut LearnerRecord,
    concept_id: &ConceptId,
) -> Result<VerifyOutcome> {
    let concept = graph
        .get(concept_id)
        .ok_or_else(|| CurriculumError::UnknownConcept(concept_id.0.clone()))?;
    let mastery = learner.mastery.get(concept_id).cloned().unwrap_or_default();
    let last = match mastery.attempts.last() {
        Some(a) => a.clone(),
        None => {
            return Ok(VerifyOutcome::NotYet {
                reason: NotYetReason::TooFewItems,
                score: 0.0,
                threshold: concept.mastery_threshold,
            });
        }
    };
    let score = last.score();
    let threshold = concept.mastery_threshold;

    if last.items_attempted < MIN_ITEMS_PER_ATTEMPT {
        return Ok(VerifyOutcome::NotYet {
            reason: NotYetReason::TooFewItems,
            score,
            threshold,
        });
    }
    if !prerequisites_mastered(graph, learner, concept) {
        return Ok(VerifyOutcome::NotYet {
            reason: NotYetReason::PrerequisitesUnmastered,
            score,
            threshold,
        });
    }
    if score >= threshold {
        learner.mark_mastered(concept_id);
        Ok(VerifyOutcome::Mastered {
            score,
            threshold,
            items_attempted: last.items_attempted,
        })
    } else {
        Ok(VerifyOutcome::NotYet {
            reason: NotYetReason::BelowThreshold,
            score,
            threshold,
        })
    }
}

/// True iff every prerequisite of `concept` is marked mastered in
/// `learner`. Empty prerequisite list ⇒ trivially true.
fn prerequisites_mastered(
    graph: &ConceptGraph,
    learner: &LearnerRecord,
    concept: &Concept,
) -> bool {
    for prereq in &concept.prerequisites {
        if !learner.is_mastered(prereq) {
            // Prerequisite is unmastered. Defensive: also require
            // its prereqs in case the graph was extended after the
            // learner mastered an earlier version.
            if graph.get(prereq).is_some() {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::concept::{Concept, Pillar};
    use crate::learner::AttemptRecord;

    fn fixture() -> (ConceptGraph, LearnerRecord) {
        let mut g = ConceptGraph::new();
        g.insert(Concept::new(
            "math::a",
            "Тест A",
            Pillar::Mathematics,
            "5",
            "ескерту",
        ))
        .unwrap();
        g.insert(
            Concept::new("math::b", "Тест B", Pillar::Mathematics, "5", "ескерту")
                .with_prerequisites(vec![ConceptId::new("math::a")]),
        )
        .unwrap();
        let learner = LearnerRecord::new("test");
        (g, learner)
    }

    fn attempt(items: u32, correct: u32) -> AttemptRecord {
        AttemptRecord {
            timestamp: "2026-05-17T00:00:00Z".into(),
            items_attempted: items,
            items_correct: correct,
            time_taken_s: 60,
        }
    }

    #[test]
    fn no_attempts_means_not_yet() {
        let (g, mut l) = fixture();
        let r = verify_concept(&g, &mut l, &ConceptId::new("math::a")).unwrap();
        assert!(matches!(
            r,
            VerifyOutcome::NotYet {
                reason: NotYetReason::TooFewItems,
                ..
            }
        ));
        assert!(!l.is_mastered(&ConceptId::new("math::a")));
    }

    #[test]
    fn too_few_items_blocks_mastery() {
        let (g, mut l) = fixture();
        let cid = ConceptId::new("math::a");
        l.record_attempt(cid.clone(), attempt(3, 3));
        let r = verify_concept(&g, &mut l, &cid).unwrap();
        assert!(matches!(
            r,
            VerifyOutcome::NotYet {
                reason: NotYetReason::TooFewItems,
                ..
            }
        ));
        assert!(!l.is_mastered(&cid));
    }

    #[test]
    fn below_threshold_blocks_mastery() {
        let (g, mut l) = fixture();
        let cid = ConceptId::new("math::a");
        l.record_attempt(cid.clone(), attempt(5, 3)); // 60%
        let r = verify_concept(&g, &mut l, &cid).unwrap();
        assert!(matches!(
            r,
            VerifyOutcome::NotYet {
                reason: NotYetReason::BelowThreshold,
                ..
            }
        ));
        assert!(!l.is_mastered(&cid));
    }

    #[test]
    fn unmastered_prereq_blocks_mastery_even_with_high_score() {
        let (g, mut l) = fixture();
        let cid = ConceptId::new("math::b");
        l.record_attempt(cid.clone(), attempt(10, 10)); // 100% but a not mastered
        let r = verify_concept(&g, &mut l, &cid).unwrap();
        assert!(matches!(
            r,
            VerifyOutcome::NotYet {
                reason: NotYetReason::PrerequisitesUnmastered,
                ..
            }
        ));
        assert!(!l.is_mastered(&cid));
    }

    #[test]
    fn passing_rubric_grants_mastery() {
        let (g, mut l) = fixture();
        let cid = ConceptId::new("math::a");
        l.record_attempt(cid.clone(), attempt(10, 9)); // 90% on 10 items
        let r = verify_concept(&g, &mut l, &cid).unwrap();
        assert!(matches!(r, VerifyOutcome::Mastered { .. }));
        assert!(l.is_mastered(&cid));
    }

    #[test]
    fn second_concept_unlocks_after_first_mastered() {
        let (g, mut l) = fixture();
        let a = ConceptId::new("math::a");
        let b = ConceptId::new("math::b");
        l.record_attempt(a.clone(), attempt(10, 9));
        verify_concept(&g, &mut l, &a).unwrap();
        l.record_attempt(b.clone(), attempt(10, 9));
        let r = verify_concept(&g, &mut l, &b).unwrap();
        assert!(matches!(r, VerifyOutcome::Mastered { .. }));
        assert!(l.is_mastered(&b));
    }
}
