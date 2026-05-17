// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! End-to-end integration: build a tiny graph, simulate a learner's
//! journey through plan→diagnose→attempt→verify, assert the
//! mastery state evolves correctly.
//!
//! Mirrors the production loop the «Qazaq AI Ұстаз» runtime will
//! drive every turn. If this test breaks, the L7-L10-edu contract
//! has shifted.

use adam_curriculum::{
    AttemptRecord, Concept, ConceptGraph, ConceptId, Diagnosis, LearnerRecord, LessonRationale,
    NotYetReason, Pillar, VerifyOutcome, diagnose, plan_next, verify_concept,
};

fn build_curriculum() -> ConceptGraph {
    let mut g = ConceptGraph::new();
    g.insert(Concept::new(
        "morphology::noun::plural",
        "Көптік жалғау",
        Pillar::KazakhMorphology,
        "5",
        "«-лар/-лер» жұрнағы",
    ))
    .unwrap();
    g.insert(
        Concept::new(
            "morphology::noun::dative",
            "Барыс септігі",
            Pillar::KazakhMorphology,
            "5",
            "«-ға/-ге» жұрнағы",
        )
        .with_prerequisites(vec![ConceptId::new("morphology::noun::plural")]),
    )
    .unwrap();
    g
}

#[test]
fn full_journey_plan_diagnose_attempt_verify() {
    let graph = build_curriculum();
    let mut learner = LearnerRecord::new("test-uuid");

    // Step 1: planner picks the foundation concept.
    let next = plan_next(&graph, &learner, Some(Pillar::KazakhMorphology)).unwrap();
    assert_eq!(next.concept.as_str(), "morphology::noun::plural");
    assert!(matches!(next.rationale, LessonRationale::Foundation));

    // Step 2: learner attempts and passes.
    let cid = next.concept.clone();
    learner.record_attempt(
        cid.clone(),
        AttemptRecord {
            timestamp: "2026-05-17T10:00:00Z".into(),
            items_attempted: 10,
            items_correct: 9,
            time_taken_s: 180,
        },
    );
    let outcome = verify_concept(&graph, &mut learner, &cid).unwrap();
    assert!(matches!(outcome, VerifyOutcome::Mastered { .. }));
    assert!(learner.is_mastered(&cid));

    // Step 3: planner now unlocks the dependent concept.
    let next = plan_next(&graph, &learner, Some(Pillar::KazakhMorphology)).unwrap();
    assert_eq!(next.concept.as_str(), "morphology::noun::dative");
    assert!(matches!(
        next.rationale,
        LessonRationale::PrerequisitesMet { .. }
    ));

    // Step 4: learner fails — score below threshold.
    let cid = next.concept.clone();
    learner.record_attempt(
        cid.clone(),
        AttemptRecord {
            timestamp: "2026-05-17T10:15:00Z".into(),
            items_attempted: 10,
            items_correct: 5,
            time_taken_s: 200,
        },
    );
    let outcome = verify_concept(&graph, &mut learner, &cid).unwrap();
    assert!(matches!(
        outcome,
        VerifyOutcome::NotYet {
            reason: NotYetReason::BelowThreshold,
            ..
        }
    ));
    assert!(!learner.is_mastered(&cid));

    // Step 5: diagnostic confirms prereqs are mastered (the failure
    // is a misconception in the target itself).
    let diag = diagnose(&graph, &learner, &cid);
    assert_eq!(diag, Diagnosis::Ready);

    // Step 6: planner sends learner back to retry.
    let next = plan_next(&graph, &learner, Some(Pillar::KazakhMorphology)).unwrap();
    assert_eq!(next.concept, cid);
    assert!(matches!(
        next.rationale,
        LessonRationale::RetryAfterFailure { .. }
    ));

    // Step 7: learner retries and passes.
    learner.record_attempt(
        cid.clone(),
        AttemptRecord {
            timestamp: "2026-05-17T10:30:00Z".into(),
            items_attempted: 10,
            items_correct: 9,
            time_taken_s: 150,
        },
    );
    let outcome = verify_concept(&graph, &mut learner, &cid).unwrap();
    assert!(matches!(outcome, VerifyOutcome::Mastered { .. }));
    assert!(learner.is_mastered(&cid));

    // Step 8: planner reports the pillar is complete.
    let err = plan_next(&graph, &learner, Some(Pillar::KazakhMorphology)).unwrap_err();
    assert!(matches!(
        err,
        adam_curriculum::CurriculumError::AllConceptsMastered
    ));

    // The learner's record retains the full attempt history.
    assert_eq!(learner.mastery[&cid].attempts.len(), 2);
    assert_eq!(learner.mastered_count(), 2);
}
