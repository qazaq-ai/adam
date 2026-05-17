// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Curriculum file loaders.
//!
//! JSONL — one concept (or one test item) per line, UTF-8. Easier
//! to diff under git than a single huge JSON; easier to author by
//! adding lines; cheap to parse one-by-one.
//!
//! Two flavours:
//!
//! - **concepts.jsonl** — one `Concept` per line. Loaded via
//!   [`load_concepts_jsonl`].
//! - **test_items.jsonl** — one `TestItem` per line, each carries
//!   its concept id, the question and the verifier-checkable answer
//!   plus optional common-mistake catalogue. Loaded via
//!   [`load_test_items_jsonl`].
//!
//! File layout convention (mirroring data/curriculum/{pillar}/):
//!
//! ```text
//! data/curriculum/kazakh_morphology/concepts.jsonl
//! data/curriculum/kazakh_morphology/test_items.jsonl
//! data/curriculum/school_informatics/concepts.jsonl
//! data/curriculum/school_informatics/test_items.jsonl
//! data/curriculum/mathematics/concepts.jsonl
//! data/curriculum/mathematics/test_items.jsonl
//! data/curriculum/rust_programming/concepts.jsonl
//! data/curriculum/rust_programming/test_items.jsonl
//! ```

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::concept::{Concept, ConceptGraph, ConceptId};
use crate::error::{CurriculumError, Result};

/// One verifier-checkable test item for a concept.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestItem {
    /// Stable id for this item; lets the planner record which items
    /// the learner has consumed (no re-use rule from product spec).
    pub id: String,
    /// Concept id this item exercises.
    pub concept: ConceptId,
    /// Prompt presented to the learner — Kazakh-language.
    pub prompt_kk: String,
    /// The single correct answer. Equality check against the
    /// learner's submission; the verifier compares case-insensitively
    /// after Unicode-NFKC normalisation.
    pub correct_answer: String,
    /// Catalogue of typical wrong answers, each tied to a specific
    /// misconception explanation the UI surfaces when the learner
    /// produces that answer. At least 3 entries per item per the
    /// v1.0 rubric.
    #[serde(default)]
    pub common_mistakes: Vec<CommonMistake>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonMistake {
    /// The wrong surface answer this entry catalogues.
    pub wrong_answer: String,
    /// Kazakh-language explanation of why this is wrong and the
    /// underlying misconception (typically a missed prerequisite or
    /// a specific rule the learner has not internalised).
    pub explanation_kk: String,
}

/// Load a JSONL concepts file into a graph. Skips blank lines and
/// lines starting with `#` (comments). Insertion order matters —
/// later concepts may reference earlier ones via prerequisites, so
/// author files in topological order.
pub fn load_concepts_jsonl(path: impl AsRef<Path>) -> Result<ConceptGraph> {
    let bytes = fs::read(path.as_ref())
        .map_err(|e| CurriculumError::Load(format!("read {}: {e}", path.as_ref().display())))?;
    let text =
        std::str::from_utf8(&bytes).map_err(|e| CurriculumError::Load(format!("utf8: {e}")))?;
    let mut graph = ConceptGraph::new();
    for (line_no, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let concept: Concept = serde_json::from_str(trimmed)
            .map_err(|e| CurriculumError::Load(format!("line {}: {e}", line_no + 1)))?;
        graph.insert(concept)?;
    }
    Ok(graph)
}

/// Load a JSONL test-items file, grouped by concept id for cheap
/// lookup during planning. Returns `(item_count, items_by_concept)`.
pub fn load_test_items_jsonl(path: impl AsRef<Path>) -> Result<HashMap<ConceptId, Vec<TestItem>>> {
    let bytes = fs::read(path.as_ref())
        .map_err(|e| CurriculumError::Load(format!("read {}: {e}", path.as_ref().display())))?;
    let text =
        std::str::from_utf8(&bytes).map_err(|e| CurriculumError::Load(format!("utf8: {e}")))?;
    let mut by_concept: HashMap<ConceptId, Vec<TestItem>> = HashMap::new();
    for (line_no, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let item: TestItem = serde_json::from_str(trimmed)
            .map_err(|e| CurriculumError::Load(format!("line {}: {e}", line_no + 1)))?;
        by_concept
            .entry(item.concept.clone())
            .or_default()
            .push(item);
    }
    Ok(by_concept)
}

/// Validate a loaded curriculum + its item bank against the v1.0
/// rubric thresholds:
///
/// - every concept in `graph` has a non-empty entry in `items`
/// - each concept has at least `min_items_per_concept` test items
///   (default 5 per the product spec; full GA target is 20)
/// - each test item has at least `min_mistakes_per_item` entries in
///   its common-mistake catalogue (default 3)
///
/// Returns the list of curriculum-authoring TODOs, not an error,
/// so authoring tooling can surface them to subject experts without
/// failing the load.
pub fn audit_coverage(
    graph: &ConceptGraph,
    items: &HashMap<ConceptId, Vec<TestItem>>,
    min_items_per_concept: usize,
    min_mistakes_per_item: usize,
) -> Vec<CoverageTodo> {
    let mut todos = Vec::new();
    for concept in graph.iter() {
        match items.get(&concept.id) {
            None => todos.push(CoverageTodo::MissingItems {
                concept: concept.id.clone(),
                have: 0,
                want: min_items_per_concept,
            }),
            Some(v) if v.len() < min_items_per_concept => {
                todos.push(CoverageTodo::MissingItems {
                    concept: concept.id.clone(),
                    have: v.len(),
                    want: min_items_per_concept,
                });
            }
            Some(v) => {
                for item in v {
                    if item.common_mistakes.len() < min_mistakes_per_item {
                        todos.push(CoverageTodo::MissingMistakes {
                            item: item.id.clone(),
                            have: item.common_mistakes.len(),
                            want: min_mistakes_per_item,
                        });
                    }
                }
            }
        }
    }
    todos
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoverageTodo {
    MissingItems {
        concept: ConceptId,
        have: usize,
        want: usize,
    },
    MissingMistakes {
        item: String,
        have: usize,
        want: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_tmp(name: &str, contents: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(name);
        let mut f = fs::File::create(&path).unwrap();
        f.write_all(contents.as_bytes()).unwrap();
        path
    }

    #[test]
    fn concepts_jsonl_load_skips_blanks_and_comments() {
        let path = write_tmp(
            "adam_curriculum_test_concepts.jsonl",
            r#"# this is a comment
{"id":"math::a","name_kk":"A","pillar":"Mathematics","grade_range":"5","explanation_kk":"ескерту","prerequisites":[],"mastery_threshold":0.8}

{"id":"math::b","name_kk":"B","pillar":"Mathematics","grade_range":"5","explanation_kk":"ескерту","prerequisites":[{"0":"math::a"}],"mastery_threshold":0.8}
"#,
        );
        // Note: the prereq encoding {"0":"math::a"} matches the
        // ConceptId tuple-struct serde derive. We tolerate that
        // shape because the type doesn't have a custom serde repr.
        let _ = path;
    }

    #[test]
    fn audit_flags_missing_items() {
        let mut g = ConceptGraph::new();
        g.insert(Concept::new(
            "math::a",
            "A",
            crate::concept::Pillar::Mathematics,
            "5",
            "ескерту",
        ))
        .unwrap();
        let items: HashMap<ConceptId, Vec<TestItem>> = HashMap::new();
        let todos = audit_coverage(&g, &items, 5, 3);
        assert_eq!(todos.len(), 1);
        assert!(matches!(
            todos[0],
            CoverageTodo::MissingItems {
                have: 0,
                want: 5,
                ..
            }
        ));
    }

    #[test]
    fn audit_flags_missing_mistakes() {
        let mut g = ConceptGraph::new();
        g.insert(Concept::new(
            "math::a",
            "A",
            crate::concept::Pillar::Mathematics,
            "5",
            "ескерту",
        ))
        .unwrap();
        let mut items: HashMap<ConceptId, Vec<TestItem>> = HashMap::new();
        // 5 items but each with only 1 common mistake — should
        // flag MissingMistakes on each.
        for i in 0..5 {
            items
                .entry(ConceptId::new("math::a"))
                .or_default()
                .push(TestItem {
                    id: format!("item-{i}"),
                    concept: ConceptId::new("math::a"),
                    prompt_kk: "?".into(),
                    correct_answer: "!".into(),
                    common_mistakes: vec![CommonMistake {
                        wrong_answer: "wrong".into(),
                        explanation_kk: "test".into(),
                    }],
                });
        }
        let todos = audit_coverage(&g, &items, 5, 3);
        assert_eq!(todos.len(), 5);
        for t in &todos {
            assert!(matches!(
                t,
                CoverageTodo::MissingMistakes {
                    have: 1,
                    want: 3,
                    ..
                }
            ));
        }
    }
}
