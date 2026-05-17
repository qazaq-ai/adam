// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! L8-edu — diagnostic engine.
//!
//! Given a target concept the learner failed, find the deepest
//! unmastered prerequisite. This is the «name the specific
//! misconception» step from the product spec — instead of «you got
//! it wrong», the system says «you got Cylinder volume wrong because
//! you have not yet mastered Multiplying with units».
//!
//! Deterministic: same `(graph, learner, target)` → same diagnosis,
//! every time. No statistical heuristics.

use crate::concept::{ConceptGraph, ConceptId};
use crate::learner::LearnerRecord;

/// What the diagnostic engine returns. `Ready` means the learner is
/// cleared to attempt the target — all prerequisites are mastered;
/// failure on the target is a target-itself misconception, not a
/// prerequisite gap. `BlockedOn` carries the deepest unmastered
/// prerequisite; the lesson planner should route the learner there
/// next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Diagnosis {
    Ready,
    BlockedOn(ConceptId),
}

/// Walk the prerequisite chain depth-first and surface the deepest
/// concept the learner has not yet mastered. "Deepest" = furthest
/// from the target in the prereq DAG, so remediation starts at the
/// root cause, not at a mid-chain symptom.
pub fn diagnose(graph: &ConceptGraph, learner: &LearnerRecord, target: &ConceptId) -> Diagnosis {
    // Walk prereqs depth-first. The deepest unmastered concept we
    // encounter is the root cause. If we find none, the learner is
    // ready.
    let mut deepest: Option<(usize, ConceptId)> = None;
    walk(graph, learner, target, 0, &mut deepest);
    match deepest {
        None => Diagnosis::Ready,
        Some((_, id)) => Diagnosis::BlockedOn(id),
    }
}

fn walk(
    graph: &ConceptGraph,
    learner: &LearnerRecord,
    cur: &ConceptId,
    depth: usize,
    deepest: &mut Option<(usize, ConceptId)>,
) {
    let Some(concept) = graph.get(cur) else {
        return;
    };
    for prereq in &concept.prerequisites {
        if !learner.is_mastered(prereq) {
            // Found an unmastered prereq; record if deeper than
            // current candidate.
            let new_depth = depth + 1;
            match deepest {
                Some((d, _)) if *d >= new_depth => {}
                _ => *deepest = Some((new_depth, prereq.clone())),
            }
        }
        // Always descend — the deepest unmastered prereq might be
        // a transitive dependency.
        walk(graph, learner, prereq, depth + 1, deepest);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::concept::{Concept, Pillar};

    fn linear_chain(n: usize) -> (ConceptGraph, Vec<ConceptId>) {
        let mut g = ConceptGraph::new();
        let mut ids = Vec::new();
        for i in 0..n {
            let id = format!("math::c{i}");
            let prereqs = if i == 0 {
                vec![]
            } else {
                vec![ConceptId::new(format!("math::c{}", i - 1))]
            };
            let c = Concept::new(
                &id,
                format!("Тест {i}"),
                Pillar::Mathematics,
                "5",
                "ескерту",
            )
            .with_prerequisites(prereqs);
            g.insert(c).unwrap();
            ids.push(ConceptId::new(id));
        }
        (g, ids)
    }

    #[test]
    fn ready_when_no_unmastered_prereqs() {
        let (g, ids) = linear_chain(3);
        let mut l = LearnerRecord::new("test");
        // Master everything up to c2.
        l.mark_mastered(&ids[0]);
        l.mark_mastered(&ids[1]);
        let d = diagnose(&g, &l, &ids[2]);
        assert_eq!(d, Diagnosis::Ready);
    }

    #[test]
    fn blocked_on_deepest_unmastered_in_chain() {
        let (g, ids) = linear_chain(4);
        let l = LearnerRecord::new("test"); // mastered nothing
        // Target = c3 (deepest). Deepest unmastered prereq = c0.
        let d = diagnose(&g, &l, &ids[3]);
        assert_eq!(d, Diagnosis::BlockedOn(ids[0].clone()));
    }

    #[test]
    fn blocked_on_mid_chain_when_lower_mastered() {
        let (g, ids) = linear_chain(4);
        let mut l = LearnerRecord::new("test");
        l.mark_mastered(&ids[0]); // c0 done
        // c1 still unmastered, so diagnosis points there.
        let d = diagnose(&g, &l, &ids[3]);
        assert_eq!(d, Diagnosis::BlockedOn(ids[1].clone()));
    }

    #[test]
    fn unknown_target_returns_ready() {
        let (g, _) = linear_chain(1);
        let l = LearnerRecord::new("test");
        let d = diagnose(&g, &l, &ConceptId::new("math::missing"));
        // Unknown target has no prereqs to fail on → vacuously ready.
        assert_eq!(d, Diagnosis::Ready);
    }
}
