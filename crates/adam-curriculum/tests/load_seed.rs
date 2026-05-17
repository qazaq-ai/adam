// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/agriaq-ai/adam
//! Integration test: the committed seed content under
//! `data/curriculum/` loads cleanly, validates against the v1.0
//! rubric floor, and exposes the topology the production runtime
//! expects.
//!
//! If a curriculum author commits broken JSONL or a structurally
//! invalid graph (cycle, missing prereq, threshold out of range),
//! this test fires.

use adam_curriculum::{Pillar, audit_coverage, load_concepts_jsonl, load_test_items_jsonl};

const PILLAR_DIR: &str = "../../data/curriculum/kazakh_morphology";

#[test]
fn kazakh_morphology_concepts_load_cleanly() {
    let graph = load_concepts_jsonl(format!("{PILLAR_DIR}/concepts.jsonl"))
        .expect("concepts.jsonl must load");
    // v1.0 seed ships 10 concepts; assertion is ≥ 10 to allow
    // additions without breaking the test.
    assert!(
        graph.len() >= 10,
        "expected ≥ 10 concepts in seed, got {}",
        graph.len()
    );
    // Every concept must declare KazakhMorphology pillar.
    for c in graph.iter() {
        assert_eq!(
            c.pillar,
            Pillar::KazakhMorphology,
            "concept {} mis-tagged as {:?}",
            c.id,
            c.pillar
        );
    }
}

#[test]
fn kazakh_morphology_items_load_cleanly() {
    let items = load_test_items_jsonl(format!("{PILLAR_DIR}/test_items.jsonl"))
        .expect("test_items.jsonl must load");
    // v1.0 floor is 5 items per concept × 10 concepts = ≥ 50.
    let total: usize = items.values().map(|v| v.len()).sum();
    assert!(total >= 50, "expected ≥ 50 test items in seed, got {total}");
}

#[test]
fn kazakh_morphology_seed_meets_v1_floor() {
    let graph = load_concepts_jsonl(format!("{PILLAR_DIR}/concepts.jsonl")).unwrap();
    let items = load_test_items_jsonl(format!("{PILLAR_DIR}/test_items.jsonl")).unwrap();
    // v1.0 floor: ≥ 5 items/concept, ≥ 3 mistakes/item.
    let todos = audit_coverage(&graph, &items, 5, 3);
    if !todos.is_empty() {
        panic!(
            "v1.0 floor violations in kazakh_morphology seed: {:?}",
            todos
        );
    }
}
