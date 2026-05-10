//! **v5.13.0 — Codex follow-up review (B5.4) unit tests for the
//! longest-path proof-chain helper.**
//!
//! Constructs a synthetic IsA hierarchy with both a direct edge AND
//! a richer multi-hop derivation between the same endpoints, then
//! confirms `find_longest_isa_chain` picks the long path while
//! `find_isa_chain` (BFS-shortest) still returns the direct edge.
//! Also covers the no-path case and the singleton self-loop.

use adam_dialog::{find_isa_chain, find_longest_isa_chain};
use adam_reasoning::{ConfidenceKind, Fact, FactSource, Predicate, SlotRef};

fn isa(subject: &str, object: &str, sample_id: &str) -> Fact {
    Fact {
        subject: SlotRef {
            surface: subject.to_string(),
            root: subject.to_string(),
            pos: "Noun".to_string(),
        },
        predicate: Predicate::IsA,
        object: SlotRef {
            surface: object.to_string(),
            root: object.to_string(),
            pos: "Noun".to_string(),
        },
        pattern: "synthetic".to_string(),
        source: FactSource {
            pack: "v5130_test".to_string(),
            sample_id: sample_id.to_string(),
        },
        confidence: ConfidenceKind::CuratedQuote,
        raw_text: format!("{subject} — {object}"),
    }
}

#[test]
fn longest_picks_richer_derivation_over_direct_edge_v5130() {
    // Synthetic: «қасқыр → тірі» exists as a direct edge (the
    // BFS-shortest answer) AND a 4-hop richer chain through
    // жыртқыш / жануар / тіршілік иесі. Longest path must pick the
    // 4-hop variant.
    let facts = vec![
        // Direct edge — what BFS-shortest would surface.
        isa("қасқыр", "тірі", "direct_001"),
        // Multi-hop derivation.
        isa("қасқыр", "жыртқыш", "hop_001"),
        isa("жыртқыш", "жануар", "hop_002"),
        isa("жануар", "тіршілік иесі", "hop_003"),
        isa("тіршілік иесі", "тірі", "hop_004"),
    ];
    let derived = vec![];

    let shortest = find_isa_chain(&facts, &derived, "қасқыр", "тірі");
    assert_eq!(
        shortest,
        Some(vec!["қасқыр".to_string(), "тірі".to_string()]),
        "BFS-shortest must pick the direct edge"
    );

    let longest = find_longest_isa_chain(&facts, &derived, "қасқыр", "тірі");
    assert_eq!(
        longest,
        Some(vec![
            "қасқыр".to_string(),
            "жыртқыш".to_string(),
            "жануар".to_string(),
            "тіршілік иесі".to_string(),
            "тірі".to_string(),
        ]),
        "longest variant must walk the full 5-element chain"
    );
}

#[test]
fn longest_returns_none_when_no_path_exists_v5130() {
    let facts = vec![isa("қасқыр", "жыртқыш", "iso_001")];
    let derived = vec![];
    let result = find_longest_isa_chain(&facts, &derived, "қасқыр", "тас");
    assert_eq!(result, None, "no path from қасқыр to тас must return None");
}

#[test]
fn longest_handles_self_loop_v5130() {
    // Same contract as find_isa_chain: subject == target → singleton.
    let facts: Vec<Fact> = vec![];
    let derived = vec![];
    let result = find_longest_isa_chain(&facts, &derived, "қасқыр", "қасқыр");
    assert_eq!(
        result,
        Some(vec!["қасқыр".to_string()]),
        "subject == target must return a singleton chain"
    );
}

#[test]
fn longest_avoids_cycles_v5130() {
    // Synthetic cycle: A → B → A. Longest must not infinite-loop;
    // it should find the longest simple (visited-once) path. Here
    // the only simple path A → C is via B if it exists.
    let facts = vec![
        isa("a", "b", "cyc_001"),
        isa("b", "a", "cyc_002"),
        isa("b", "c", "cyc_003"),
    ];
    let derived = vec![];
    let result = find_longest_isa_chain(&facts, &derived, "a", "c");
    assert_eq!(
        result,
        Some(vec!["a".to_string(), "b".to_string(), "c".to_string()]),
        "must traverse a→b→c without revisiting a"
    );
}
