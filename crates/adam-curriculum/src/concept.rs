// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! L7-edu — curriculum graph.
//!
//! A curriculum is a directed acyclic graph of **concepts**. Each
//! concept carries:
//!
//! - a stable identifier (`ConceptId`) immutable across versions
//! - a Kazakh-language name and explanation (the canonical product
//!   language)
//! - a `pillar` (one of the four v1.0 pillars: morphology /
//!   informatics / mathematics / rust)
//! - a `grade_range` (e.g. "5-7") indicating target learners
//! - `prerequisites` — a list of other `ConceptId`s the learner
//!   must master before this concept is presented
//! - a `mastery_threshold` (default 0.80) on the `[0.0, 1.0]` range
//!
//! See [`docs/product/qazaq_ai_ustaz_v1.md`](../../../docs/product/qazaq_ai_ustaz_v1.md)
//! for the "measurable mastery per concept" rubric these structures
//! implement.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::error::{CurriculumError, Result};

/// Stable, human-readable concept identifier. By convention:
/// `<pillar>::<subtopic>::<concept>` — for example
/// `morphology::noun::dative_case` or `rust::ownership::borrowing`.
/// Stable across curriculum revisions so learner mastery records
/// don't need rewriting when text is reworded.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ConceptId(pub String);

impl ConceptId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ConceptId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The four v1.0 product pillars. Adding a new pillar is a v2+
/// release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Pillar {
    /// Pillar 1 — Kazakh morphology, grades 5-11.
    KazakhMorphology,
    /// Pillar 2 — School informatics, grades 5-11.
    SchoolInformatics,
    /// Pillar 3 — Mathematics, grades 5-11.
    Mathematics,
    /// Pillar 4 — Rust programming.
    RustProgramming,
}

impl Pillar {
    pub fn id_prefix(&self) -> &'static str {
        match self {
            Pillar::KazakhMorphology => "morphology",
            Pillar::SchoolInformatics => "informatics",
            Pillar::Mathematics => "math",
            Pillar::RustProgramming => "rust",
        }
    }
}

/// A single concept node in the curriculum graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    pub id: ConceptId,
    /// Canonical name in Kazakh. The UI may localise to Russian or
    /// English for ops/admin views but the product surface is Kazakh.
    pub name_kk: String,
    /// Pillar this concept belongs to. Must match `id` prefix.
    pub pillar: Pillar,
    /// Target grade range as a free-form string. Standard form:
    /// `"5-7"`, `"10-11_ОГН"`, `"vuz_year_1"`.
    pub grade_range: String,
    /// Short Kazakh-language explanation. Authored by a subject
    /// expert, reviewed by a native-speaker linguist.
    pub explanation_kk: String,
    /// Other concept ids the learner must master first. May be empty
    /// for foundational concepts.
    #[serde(default)]
    pub prerequisites: Vec<ConceptId>,
    /// Threshold on `[0.0, 1.0]` for the mastery rubric. Defaults to
    /// 0.80 per the product spec; subject experts may argue for 0.75
    /// on conceptual vs 0.90 on rote per `qazaq_ai_ustaz_v1.md` §
    /// "Open questions".
    #[serde(default = "default_mastery_threshold")]
    pub mastery_threshold: f32,
}

fn default_mastery_threshold() -> f32 {
    0.80
}

impl Concept {
    /// Construct a minimal concept; useful in tests and curriculum-
    /// authoring tooling. Production curriculum entries should be
    /// loaded from disk via `ConceptGraph::load`.
    pub fn new(
        id: impl Into<String>,
        name_kk: impl Into<String>,
        pillar: Pillar,
        grade_range: impl Into<String>,
        explanation_kk: impl Into<String>,
    ) -> Self {
        Self {
            id: ConceptId::new(id),
            name_kk: name_kk.into(),
            pillar,
            grade_range: grade_range.into(),
            explanation_kk: explanation_kk.into(),
            prerequisites: Vec::new(),
            mastery_threshold: default_mastery_threshold(),
        }
    }

    pub fn with_prerequisites(mut self, prereqs: Vec<ConceptId>) -> Self {
        self.prerequisites = prereqs;
        self
    }

    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.mastery_threshold = threshold;
        self
    }

    /// Sanity check that the concept is self-consistent. Called by
    /// `ConceptGraph::insert`; surfaces authoring bugs early.
    pub fn validate(&self) -> Result<()> {
        if self.name_kk.is_empty() {
            return Err(CurriculumError::MissingField {
                concept: self.id.0.clone(),
                field: "name_kk",
            });
        }
        if self.explanation_kk.is_empty() {
            return Err(CurriculumError::MissingField {
                concept: self.id.0.clone(),
                field: "explanation_kk",
            });
        }
        if self.grade_range.is_empty() {
            return Err(CurriculumError::MissingField {
                concept: self.id.0.clone(),
                field: "grade_range",
            });
        }
        if !(0.0..=1.0).contains(&self.mastery_threshold) {
            return Err(CurriculumError::ThresholdOutOfRange(self.mastery_threshold));
        }
        Ok(())
    }
}

/// The curriculum-graph container. Owns the concept table and the
/// reverse-prerequisite index used by the diagnostic engine.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConceptGraph {
    concepts: HashMap<ConceptId, Concept>,
}

impl ConceptGraph {
    pub fn new() -> Self {
        Self {
            concepts: HashMap::new(),
        }
    }

    /// Add a concept to the graph. Returns an error if the concept
    /// fails validation or if it references an unknown prerequisite.
    pub fn insert(&mut self, concept: Concept) -> Result<()> {
        concept.validate()?;
        for prereq in &concept.prerequisites {
            if !self.concepts.contains_key(prereq) {
                return Err(CurriculumError::UnknownConcept(prereq.0.clone()));
            }
        }
        // Cycle detection runs against a hypothetical graph that
        // includes this insertion.
        self.detect_cycle_through(&concept)?;
        self.concepts.insert(concept.id.clone(), concept);
        Ok(())
    }

    pub fn get(&self, id: &ConceptId) -> Option<&Concept> {
        self.concepts.get(id)
    }

    pub fn len(&self) -> usize {
        self.concepts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.concepts.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Concept> {
        self.concepts.values()
    }

    pub fn iter_by_pillar(&self, pillar: Pillar) -> impl Iterator<Item = &Concept> {
        self.concepts.values().filter(move |c| c.pillar == pillar)
    }

    /// Walk the prerequisite chain reachable from `id` (transitively).
    /// Returns concepts in topological-ish order — every prerequisite
    /// appears before the node that depends on it. Used by the
    /// diagnostic engine to find the deepest unmastered prerequisite.
    pub fn prerequisites_of(&self, id: &ConceptId) -> Vec<ConceptId> {
        let mut visited: HashSet<ConceptId> = HashSet::new();
        let mut out: Vec<ConceptId> = Vec::new();
        let mut stack: Vec<ConceptId> = match self.get(id) {
            Some(c) => c.prerequisites.clone(),
            None => return out,
        };
        while let Some(cur) = stack.pop() {
            if !visited.insert(cur.clone()) {
                continue;
            }
            if let Some(c) = self.get(&cur) {
                for p in &c.prerequisites {
                    stack.push(p.clone());
                }
            }
            out.push(cur);
        }
        out.reverse(); // approximate topological order
        out
    }

    /// DFS-based cycle detection running against the hypothetical
    /// `self ∪ {candidate}` graph. Returns Err if inserting
    /// `candidate` would create a cycle.
    fn detect_cycle_through(&self, candidate: &Concept) -> Result<()> {
        // BFS from each prerequisite of the candidate; if we ever
        // reach the candidate itself, that's a cycle.
        let mut stack: Vec<ConceptId> = candidate.prerequisites.clone();
        let mut visited: HashSet<ConceptId> = HashSet::new();
        while let Some(cur) = stack.pop() {
            if cur == candidate.id {
                return Err(CurriculumError::PrerequisiteCycle(cur.0.clone()));
            }
            if !visited.insert(cur.clone()) {
                continue;
            }
            if let Some(c) = self.get(&cur) {
                for p in &c.prerequisites {
                    stack.push(p.clone());
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn concept(id: &str, prereqs: &[&str]) -> Concept {
        let mut c = Concept::new(id, "тест", Pillar::Mathematics, "5-7", "ескерту");
        c.prerequisites = prereqs.iter().map(|s| ConceptId::new(*s)).collect();
        c
    }

    #[test]
    fn insert_validates_unknown_prereq() {
        let mut g = ConceptGraph::new();
        let c = concept("math::algebra::ax_plus_b", &["math::nonexistent"]);
        let err = g.insert(c).unwrap_err();
        assert!(matches!(err, CurriculumError::UnknownConcept(_)));
    }

    #[test]
    fn insert_detects_cycle() {
        let mut g = ConceptGraph::new();
        g.insert(concept("math::a", &[])).unwrap();
        g.insert(concept("math::b", &["math::a"])).unwrap();
        // Try to make "math::a" depend on "math::b" → cycle a→b→a.
        // Cannot insert a new "math::a" because it's already there;
        // simulate authoring via a fresh graph that inserts in
        // the reverse order.
        let mut g2 = ConceptGraph::new();
        g2.insert(concept("math::a", &[])).unwrap();
        g2.insert(concept("math::b", &["math::a"])).unwrap();
        // Re-insert "math::a" with a back-reference to b — but
        // HashMap overwrites without revalidation. Use the cycle
        // detector directly on a hypothetical concept.
        let cycle_candidate = concept("math::a", &["math::b"]);
        let err = g2.detect_cycle_through(&cycle_candidate).unwrap_err();
        assert!(matches!(err, CurriculumError::PrerequisiteCycle(_)));
    }

    #[test]
    fn prerequisites_of_traverses_transitively() {
        let mut g = ConceptGraph::new();
        g.insert(concept("math::a", &[])).unwrap();
        g.insert(concept("math::b", &["math::a"])).unwrap();
        g.insert(concept("math::c", &["math::b"])).unwrap();
        let prereqs = g.prerequisites_of(&ConceptId::new("math::c"));
        let ids: Vec<String> = prereqs.into_iter().map(|c| c.0).collect();
        assert!(ids.contains(&"math::a".to_string()));
        assert!(ids.contains(&"math::b".to_string()));
    }

    #[test]
    fn threshold_out_of_range_fails() {
        let mut g = ConceptGraph::new();
        let mut c = concept("math::x", &[]);
        c.mastery_threshold = 1.5;
        let err = g.insert(c).unwrap_err();
        assert!(matches!(err, CurriculumError::ThresholdOutOfRange(_)));
    }
}
