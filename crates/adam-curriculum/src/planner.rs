// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! L9-edu — lesson planner.
//!
//! Builds a deterministic sequence of next lessons for a learner:
//! the concepts they are ready to attempt (all prerequisites
//! mastered), prioritised by depth in the curriculum DAG.
//!
//! Pure function: same `(graph, learner, pillar?)` → same plan.
//! No spaced-repetition stochasticity; SRS is implemented as a
//! discrete schedule index that the planner reads but does not
//! roll dice on.

use std::collections::HashSet;

use crate::concept::{ConceptGraph, ConceptId, Pillar};
use crate::error::{CurriculumError, Result};
use crate::learner::LearnerRecord;

/// A single planner output: the next concept to teach plus an
/// explanation that the UI may surface to the learner.
#[derive(Debug, Clone, PartialEq)]
pub struct NextLesson {
    pub concept: ConceptId,
    pub rationale: LessonRationale,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LessonRationale {
    /// First concept in the learner's path through this pillar.
    Foundation,
    /// All prerequisites mastered; learner is ready for new
    /// material.
    PrerequisitesMet { unlocked_by: Vec<ConceptId> },
    /// The previous attempt fell below mastery threshold —
    /// re-attempt this concept rather than advance.
    RetryAfterFailure { latest_score: f32 },
}

/// Plan the next lesson within a scope (optionally limited to one
/// pillar). Returns `Err(AllConceptsMastered)` when no concept in
/// scope is unmastered AND ready — caller flips UI to a "course
/// complete" state in that case.
pub fn plan_next(
    graph: &ConceptGraph,
    learner: &LearnerRecord,
    scope: Option<Pillar>,
) -> Result<NextLesson> {
    // Strategy: walk every concept in scope; categorise into
    // (mastered, ready, blocked, retry). Prefer:
    //   1. retry-after-failure (consolidate before advancing)
    //   2. ready-foundation (no prereqs)
    //   3. ready-with-prereqs-met (advance)
    // Within each tier pick the lexicographically smallest id for
    // determinism.

    let mut retries: Vec<NextLesson> = Vec::new();
    let mut foundations: Vec<NextLesson> = Vec::new();
    let mut readies: Vec<NextLesson> = Vec::new();

    for concept in graph.iter() {
        if let Some(p) = scope {
            if concept.pillar != p {
                continue;
            }
        }
        if learner.is_mastered(&concept.id) {
            continue;
        }
        // Compute prereq state.
        let unmastered_prereqs: HashSet<&ConceptId> = concept
            .prerequisites
            .iter()
            .filter(|p| !learner.is_mastered(p))
            .collect();
        if !unmastered_prereqs.is_empty() {
            continue; // blocked
        }
        // Concept is reachable. Is it a retry after a failure?
        if let Some(m) = learner.mastery.get(&concept.id) {
            if !m.attempts.is_empty() {
                let latest = m.latest_score();
                if latest < concept.mastery_threshold {
                    retries.push(NextLesson {
                        concept: concept.id.clone(),
                        rationale: LessonRationale::RetryAfterFailure {
                            latest_score: latest,
                        },
                    });
                    continue;
                }
            }
        }
        // Otherwise — fresh ready concept.
        if concept.prerequisites.is_empty() {
            foundations.push(NextLesson {
                concept: concept.id.clone(),
                rationale: LessonRationale::Foundation,
            });
        } else {
            readies.push(NextLesson {
                concept: concept.id.clone(),
                rationale: LessonRationale::PrerequisitesMet {
                    unlocked_by: concept.prerequisites.clone(),
                },
            });
        }
    }

    // Tier 1: retries.
    retries.sort_by(|a, b| a.concept.cmp(&b.concept));
    if let Some(r) = retries.into_iter().next() {
        return Ok(r);
    }
    // Tier 2: foundations.
    foundations.sort_by(|a, b| a.concept.cmp(&b.concept));
    if let Some(r) = foundations.into_iter().next() {
        return Ok(r);
    }
    // Tier 3: ready-with-prereqs-met.
    readies.sort_by(|a, b| a.concept.cmp(&b.concept));
    if let Some(r) = readies.into_iter().next() {
        return Ok(r);
    }
    Err(CurriculumError::AllConceptsMastered)
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
            "A",
            Pillar::Mathematics,
            "5",
            "ескерту",
        ))
        .unwrap();
        g.insert(
            Concept::new("math::b", "B", Pillar::Mathematics, "5", "ескерту")
                .with_prerequisites(vec![ConceptId::new("math::a")]),
        )
        .unwrap();
        g.insert(Concept::new(
            "rust::a",
            "A",
            Pillar::RustProgramming,
            "vuz_year_1",
            "ескерту",
        ))
        .unwrap();
        (g, LearnerRecord::new("test"))
    }

    #[test]
    fn first_call_picks_a_foundation_alphabetically() {
        let (g, l) = fixture();
        let next = plan_next(&g, &l, None).unwrap();
        assert_eq!(next.concept, ConceptId::new("math::a"));
        assert_eq!(next.rationale, LessonRationale::Foundation);
    }

    #[test]
    fn after_mastering_a_picks_b() {
        let (g, mut l) = fixture();
        l.mark_mastered(&ConceptId::new("math::a"));
        // also mastered rust::a so b is the only unmastered with prereqs met.
        l.mark_mastered(&ConceptId::new("rust::a"));
        let next = plan_next(&g, &l, Some(Pillar::Mathematics)).unwrap();
        assert_eq!(next.concept, ConceptId::new("math::b"));
        assert!(matches!(
            next.rationale,
            LessonRationale::PrerequisitesMet { .. }
        ));
    }

    #[test]
    fn retry_takes_precedence_over_new_material() {
        let (g, mut l) = fixture();
        // Failed attempt on math::a (low score).
        l.record_attempt(
            ConceptId::new("math::a"),
            AttemptRecord {
                timestamp: "t".into(),
                items_attempted: 5,
                items_correct: 2,
                time_taken_s: 30,
            },
        );
        let next = plan_next(&g, &l, None).unwrap();
        // Even though rust::a is also a Foundation candidate, the
        // retry on math::a takes precedence.
        assert_eq!(next.concept, ConceptId::new("math::a"));
        assert!(matches!(
            next.rationale,
            LessonRationale::RetryAfterFailure { .. }
        ));
    }

    #[test]
    fn pillar_scope_filters_candidates() {
        let (g, l) = fixture();
        let next = plan_next(&g, &l, Some(Pillar::RustProgramming)).unwrap();
        assert_eq!(next.concept, ConceptId::new("rust::a"));
    }

    #[test]
    fn all_mastered_returns_specific_error() {
        let (g, mut l) = fixture();
        l.mark_mastered(&ConceptId::new("math::a"));
        l.mark_mastered(&ConceptId::new("math::b"));
        l.mark_mastered(&ConceptId::new("rust::a"));
        let err = plan_next(&g, &l, None).unwrap_err();
        assert!(matches!(err, CurriculumError::AllConceptsMastered));
    }
}
