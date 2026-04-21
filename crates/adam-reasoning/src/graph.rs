//! v2.3 lexical graph — the first projection of `Fact`s into a node-edge
//! knowledge graph.
//!
//! Every fact `(subject, predicate, object)` is an edge from subject to
//! object typed by the predicate. Nodes are distinct roots appearing as
//! subject or object in ≥ 1 fact. This graph is a **pure projection**
//! of `data/retrieval/facts.json`; no new extraction logic, no learned
//! weights, no heuristics beyond the ones already used in fact
//! extraction. v2.4+ will enrich it with edges derived from the
//! Lexicon itself (POS, morphological co-occurrence, domain tags).
//!
//! Why start minimal? Because every graph edge has to be defensible:
//! a reasoner (v2.4+) will traverse this graph to answer questions, and
//! every wrong edge propagates through every chain. Starting with only
//! fact-derived edges means every edge has an auditable source.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{Fact, FactSource, Predicate};

/// A directed edge between two roots, typed by a predicate and
/// carrying provenance back to the fact that produced it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: String,
    pub predicate: Predicate,
    pub to: String,
    /// Sources of the underlying facts — one graph edge may be
    /// supported by ≥ 1 fact from distinct corpus samples. More
    /// sources = more robust edge; v2.4+ will use this for
    /// `ConfidenceKind::RepeatedPattern`.
    pub sources: Vec<FactSource>,
}

/// Per-root summary: degree counts and predicate breakdown. Lets a
/// reasoner (or human) ask "what do we know about X?" in O(1) after
/// the graph is built.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeStats {
    /// Number of edges leaving this node.
    pub out_degree: usize,
    /// Number of edges arriving at this node.
    pub in_degree: usize,
    /// Per-predicate outgoing edge counts.
    pub out_by_predicate: BTreeMap<String, usize>,
    /// Per-predicate incoming edge counts.
    pub in_by_predicate: BTreeMap<String, usize>,
}

/// The lexical graph itself — BTreeMap / BTreeSet / sorted Vec so the
/// on-disk JSON is deterministic. Identical input → byte-identical
/// output, by design.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LexicalGraph {
    /// Sorted root → stats.
    pub nodes: BTreeMap<String, NodeStats>,
    /// Sorted (from, predicate, to) → one merged edge per triple.
    pub edges: Vec<GraphEdge>,
    /// Total facts ingested (may be > edges when the same triple
    /// occurs in multiple samples — those merge into one edge with
    /// multiple sources).
    pub facts_ingested: usize,
}

impl LexicalGraph {
    /// Empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Project a list of facts into a graph. Triples that repeat
    /// across facts (same from/predicate/to) merge into one edge whose
    /// `sources` is the sorted union of supporting `FactSource`s.
    pub fn from_facts(facts: &[Fact]) -> Self {
        // Accumulator keyed by (from, predicate_str, to). Predicate
        // represented as string so the Ord is stable across releases
        // even if the enum order is reordered.
        let mut merged: BTreeMap<(String, String, String), Vec<FactSource>> = BTreeMap::new();
        for f in facts {
            let key = (
                f.subject.root.clone(),
                f.predicate.as_str().to_string(),
                f.object.root.clone(),
            );
            merged.entry(key).or_default().push(f.source.clone());
        }

        let mut edges: Vec<GraphEdge> = merged
            .into_iter()
            .map(|((from, pred_str, to), mut sources)| {
                // Deterministic source ordering + dedup (same sample
                // could produce two facts with the same triple if a
                // pattern fires twice — defensive).
                sources.sort();
                sources.dedup();
                GraphEdge {
                    from,
                    predicate: match pred_str.as_str() {
                        "is_a" => Predicate::IsA,
                        "lives_in" => Predicate::LivesIn,
                        "has" => Predicate::Has,
                        // Unknown string — silently drop (forward-compat
                        // for future predicates serialised from older
                        // code). Construction is pure so this is safe.
                        _ => unreachable!("unknown predicate string in graph build"),
                    },
                    to,
                    sources,
                }
            })
            .collect();
        // Edges are already sorted by BTreeMap key order, but the
        // `as_str` strings may not match enum order. Resort on the
        // canonical (from, predicate_str, to) triple for deterministic
        // serialisation.
        edges.sort_by(|a, b| {
            (a.from.as_str(), a.predicate.as_str(), a.to.as_str()).cmp(&(
                b.from.as_str(),
                b.predicate.as_str(),
                b.to.as_str(),
            ))
        });

        // Build per-node stats in one pass over edges.
        let mut nodes: BTreeMap<String, NodeStats> = BTreeMap::new();
        let mut all_roots: BTreeSet<&str> = BTreeSet::new();
        for e in &edges {
            all_roots.insert(e.from.as_str());
            all_roots.insert(e.to.as_str());
        }
        for root in all_roots {
            nodes.insert(
                root.to_string(),
                NodeStats {
                    out_degree: 0,
                    in_degree: 0,
                    out_by_predicate: BTreeMap::new(),
                    in_by_predicate: BTreeMap::new(),
                },
            );
        }
        for e in &edges {
            if let Some(n) = nodes.get_mut(&e.from) {
                n.out_degree += 1;
                *n.out_by_predicate
                    .entry(e.predicate.as_str().to_string())
                    .or_insert(0) += 1;
            }
            if let Some(n) = nodes.get_mut(&e.to) {
                n.in_degree += 1;
                *n.in_by_predicate
                    .entry(e.predicate.as_str().to_string())
                    .or_insert(0) += 1;
            }
        }

        Self {
            nodes,
            edges,
            facts_ingested: facts.len(),
        }
    }

    /// All outgoing edges from a root — useful for "tell me about X" queries.
    /// Returns an empty slice if the root is not in the graph.
    pub fn outgoing(&self, root: &str) -> Vec<&GraphEdge> {
        self.edges.iter().filter(|e| e.from == root).collect()
    }

    /// All incoming edges to a root — useful for "what is an X?" queries
    /// (e.g. `incoming("бұлақ")` finds every known IS-A pointing at бұлақ).
    pub fn incoming(&self, root: &str) -> Vec<&GraphEdge> {
        self.edges.iter().filter(|e| e.to == root).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConfidenceKind, FactSource, SlotRef};

    fn fact(subj: &str, pred: Predicate, obj: &str, pack: &str, id: &str) -> Fact {
        Fact {
            subject: SlotRef {
                surface: subj.into(),
                root: subj.into(),
                pos: "noun".into(),
            },
            predicate: pred,
            object: SlotRef {
                surface: obj.into(),
                root: obj.into(),
                pos: "noun".into(),
            },
            pattern: "test".into(),
            source: FactSource {
                pack: pack.into(),
                sample_id: id.into(),
            },
            confidence: ConfidenceKind::Grammar,
            raw_text: format!("{subj} {obj}"),
        }
    }

    #[test]
    fn empty_facts_empty_graph() {
        let g = LexicalGraph::from_facts(&[]);
        assert!(g.nodes.is_empty());
        assert!(g.edges.is_empty());
        assert_eq!(g.facts_ingested, 0);
    }

    #[test]
    fn single_fact_single_edge() {
        let facts = vec![fact("кітап", Predicate::IsA, "бұлақ", "p1", "s1")];
        let g = LexicalGraph::from_facts(&facts);
        assert_eq!(g.edges.len(), 1);
        assert_eq!(g.nodes.len(), 2);
        assert_eq!(g.facts_ingested, 1);
        let e = &g.edges[0];
        assert_eq!(e.from, "кітап");
        assert_eq!(e.predicate, Predicate::IsA);
        assert_eq!(e.to, "бұлақ");
        assert_eq!(e.sources.len(), 1);
    }

    #[test]
    fn repeated_triple_merges_sources() {
        let facts = vec![
            fact("кітап", Predicate::IsA, "бұлақ", "p1", "s1"),
            fact("кітап", Predicate::IsA, "бұлақ", "p2", "s2"),
        ];
        let g = LexicalGraph::from_facts(&facts);
        assert_eq!(g.edges.len(), 1, "same triple → merged edge");
        assert_eq!(g.edges[0].sources.len(), 2);
        assert_eq!(g.facts_ingested, 2);
    }

    #[test]
    fn node_stats_track_degree_per_predicate() {
        let facts = vec![
            fact("бала", Predicate::IsA, "адам", "p", "s1"),
            fact("бала", Predicate::Has, "кітап", "p", "s2"),
            fact("бала", Predicate::IsA, "болашақ", "p", "s3"),
        ];
        let g = LexicalGraph::from_facts(&facts);
        let bala = &g.nodes["бала"];
        assert_eq!(bala.out_degree, 3);
        assert_eq!(bala.in_degree, 0);
        assert_eq!(bala.out_by_predicate["is_a"], 2);
        assert_eq!(bala.out_by_predicate["has"], 1);
    }

    #[test]
    fn outgoing_and_incoming_lookups() {
        let facts = vec![
            fact("кітап", Predicate::IsA, "бұлақ", "p", "s1"),
            fact("ілім", Predicate::IsA, "бұлақ", "p", "s2"),
        ];
        let g = LexicalGraph::from_facts(&facts);
        assert_eq!(g.outgoing("кітап").len(), 1);
        assert_eq!(g.incoming("бұлақ").len(), 2, "both sources point at бұлақ");
        assert_eq!(g.outgoing("неизвестный").len(), 0);
    }

    #[test]
    fn edges_are_deterministically_sorted() {
        let facts = vec![
            fact("б", Predicate::IsA, "я", "p", "s1"),
            fact("а", Predicate::IsA, "я", "p", "s2"),
            fact("а", Predicate::Has, "б", "p", "s3"),
        ];
        let g = LexicalGraph::from_facts(&facts);
        let order: Vec<(String, String, String)> = g
            .edges
            .iter()
            .map(|e| {
                (
                    e.from.clone(),
                    e.predicate.as_str().to_string(),
                    e.to.clone(),
                )
            })
            .collect();
        let mut expected = order.clone();
        expected.sort();
        assert_eq!(order, expected, "edges must be sorted deterministically");
    }

    #[test]
    fn graph_round_trips_through_json() {
        let facts = vec![fact("кітап", Predicate::IsA, "бұлақ", "p", "s")];
        let g = LexicalGraph::from_facts(&facts);
        let json = serde_json::to_string(&g).unwrap();
        let parsed: LexicalGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.edges.len(), 1);
        assert_eq!(parsed.facts_ingested, 1);
    }
}
