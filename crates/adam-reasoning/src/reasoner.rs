//! v2.4 rule reasoner v0 — forward-chaining over the Lexical Graph.
//!
//! Takes a set of starting facts + a small, hand-coded rule set, and
//! derives **new facts** by pattern-matching graph edges. Every derived
//! fact carries:
//!
//!   - the rule id that fired (reproducible, not an opaque score);
//!   - the source facts it chains (as `FactSource` entries in the
//!     derived fact's `source_chain`);
//!   - `ConfidenceKind::RuleInferred` — never `Grammar`, so downstream
//!     consumers can always tell extracted-from-corpus facts apart
//!     from rule-derived conclusions.
//!
//! ## Scope of v2.4 (explicit)
//!
//! Rules are **hand-coded Rust**, not data. Five inference rules
//! ship initially, each deliberately conservative:
//!
//! | id | pattern | conclusion |
//! |---|---|---|
//! | `R1_is_a_transitivity` | A IsA B ∧ B IsA C | A IsA C |
//! | `R2_has_inheritance` | A IsA B ∧ B Has X | A HasKinded X (new predicate) |
//! | `R3_lives_in_transitivity` | A LivesIn B ∧ B PartOf C | A LivesIn C (not active yet — no PartOf facts) |
//! | `R4_is_a_symmetry_filter` | A IsA B ∧ B IsA A | flag both for curator review (returns diagnostic, not a fact) |
//! | `R5_shared_is_a_target` | A IsA X ∧ B IsA X (A ≠ B) | RelatedTo(A, B) |
//!
//! ## Determinism
//!
//! Forward-chaining reaches fixpoint in a bounded number of iterations
//! (capped at `MAX_ITER = 8`). Same starting facts → same derived facts,
//! byte-identical. No RNG, no heuristics beyond the explicit rules.
//!
//! ## Trust invariants (test-enforced)
//!
//! - A rule fires ⇒ the derived fact's `confidence` is
//!   `RuleInferred` (never `Grammar`).
//! - A derived fact's `source_chain` is non-empty (empty → unsound).
//! - Fixpoint reached ⇒ running the reasoner again adds nothing.
//! - Tautology rejection: `R1` never derives `A IsA A`.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{ConfidenceKind, Fact, FactSource, Predicate, SlotRef, graph::LexicalGraph};

const MAX_ITER: usize = 8;

/// A rule-derived fact. Parallel to [`Fact`] but distinguished by:
///
///   - `rule_id` — which rule fired;
///   - `source_chain` — every underlying fact that contributed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DerivedFact {
    pub subject: SlotRef,
    pub predicate: Predicate,
    pub object: SlotRef,
    /// Stable identifier of the rule that derived this fact
    /// (e.g. `"R1_is_a_transitivity"`). Used for audit + grepping.
    pub rule_id: String,
    /// Every source that contributed a chained-over fact, in the
    /// order the rule consumed them. Non-empty by invariant.
    pub source_chain: Vec<FactSource>,
    /// Always `ConfidenceKind::RuleInferred` for rule-derived facts.
    pub confidence: ConfidenceKind,
}

impl DerivedFact {
    /// Promote a [`DerivedFact`] to a [`Fact`] for downstream
    /// consumers that treat extracted + derived uniformly. The
    /// rule id becomes the `pattern` tag; the first source is
    /// kept as the canonical `source` (the full chain is available
    /// on the original `DerivedFact`).
    pub fn into_fact(self) -> Fact {
        // Pick the first chain entry as the canonical provenance for
        // Fact compatibility. Callers that need the full chain should
        // consume the DerivedFact directly.
        let source = self
            .source_chain
            .first()
            .cloned()
            .expect("derived fact must have non-empty source_chain");
        Fact {
            subject: self.subject,
            predicate: self.predicate,
            object: self.object,
            pattern: self.rule_id,
            source,
            confidence: self.confidence,
            raw_text: "<rule-inferred>".to_string(),
        }
    }
}

/// Run the forward-chaining reasoner over `initial_facts`.
///
/// Returns every new fact derived from the rule set, plus a bounded
/// iteration count. Idempotent after convergence — calling again on
/// the union of inputs + outputs yields an empty additional set.
///
/// Deterministic: rules fire in declared order; new facts are checked
/// against the accumulating set using `(subject.root, predicate,
/// object.root)` as the identity key.
pub fn run(initial_facts: &[Fact]) -> Vec<DerivedFact> {
    let mut all_facts: Vec<Fact> = initial_facts.to_vec();
    let mut derived: Vec<DerivedFact> = Vec::new();
    let mut seen_triples: BTreeSet<(String, String, String)> =
        initial_facts.iter().map(|f| fact_triple_key(f)).collect();

    for _iter in 0..MAX_ITER {
        let graph = LexicalGraph::from_facts(&all_facts);
        let new_derived = run_one_pass(&all_facts, &graph, &seen_triples);
        if new_derived.is_empty() {
            break;
        }
        // Merge: append to all_facts (for the next graph build) + to
        // derived (as the return value) + to seen_triples (for
        // deduplication).
        for d in &new_derived {
            let key = (
                d.subject.root.clone(),
                d.predicate.as_str().to_string(),
                d.object.root.clone(),
            );
            if seen_triples.insert(key) {
                all_facts.push(d.clone().into_fact());
                derived.push(d.clone());
            }
        }
    }

    derived
}

/// Single pass of all active rules. Returns every new derivation from
/// this pass in declared order. Callers de-dup.
fn run_one_pass(
    facts: &[Fact],
    graph: &LexicalGraph,
    seen: &BTreeSet<(String, String, String)>,
) -> Vec<DerivedFact> {
    let mut out = Vec::new();
    rule_r1_is_a_transitivity(facts, graph, seen, &mut out);
    rule_r2_has_inheritance(facts, graph, seen, &mut out);
    rule_r5_shared_is_a_target(facts, graph, seen, &mut out);
    out
}

/// Key used to decide if we've already emitted a symmetric pair.
/// `RelatedTo` is symmetric — `(A, B)` and `(B, A)` are the same
/// logical fact; we canonicalise to the lexicographically smaller
/// first element so the pair dedup is stable.
fn canonical_relation_pair(a: &str, b: &str) -> (String, String) {
    if a <= b {
        (a.to_string(), b.to_string())
    } else {
        (b.to_string(), a.to_string())
    }
}

/// `R1`: IS-A transitivity. If `A IsA B` and `B IsA C`, derive `A IsA C`
/// (skipping A = C tautologies). Chain length limited to 1 hop per
/// pass; longer chains surface across iterations.
fn rule_r1_is_a_transitivity(
    facts: &[Fact],
    graph: &LexicalGraph,
    seen: &BTreeSet<(String, String, String)>,
    out: &mut Vec<DerivedFact>,
) {
    for first in facts {
        if first.predicate != Predicate::IsA {
            continue;
        }
        // Find every edge (first.object --IsA--> C) in the graph.
        for second in graph.outgoing(&first.object.root) {
            if second.predicate != Predicate::IsA {
                continue;
            }
            if first.subject.root == second.to {
                continue; // tautology
            }
            let key = (
                first.subject.root.clone(),
                "is_a".to_string(),
                second.to.clone(),
            );
            if seen.contains(&key) {
                continue;
            }
            // Find the supporting fact(s) for the second edge. The
            // graph merges identical triples, so pull the first source
            // as evidence.
            let second_source = second
                .sources
                .first()
                .cloned()
                .expect("graph edge must carry at least one source");
            out.push(DerivedFact {
                subject: first.subject.clone(),
                predicate: Predicate::IsA,
                object: SlotRef {
                    surface: second.to.clone(),
                    root: second.to.clone(),
                    pos: "noun".into(),
                },
                rule_id: "R1_is_a_transitivity".into(),
                source_chain: vec![first.source.clone(), second_source],
                confidence: ConfidenceKind::RuleInferred,
            });
        }
    }
}

/// `R5`: shared IS-A target → `RelatedTo`. If `A IsA X` and `B IsA X`
/// for distinct A, B, both are **related** via the common type X.
/// Activated in v2.6 once `Predicate::RelatedTo` exists. This is the
/// first rule that produces derivations on the committed v2.5 fact
/// set: `кітап IsA бұлақ` and `ілім IsA бұлақ` share target `бұлақ`,
/// so R5 derives `кітап RelatedTo ілім` with the `бұлақ` hub as
/// evidence.
///
/// Symmetry: `RelatedTo` is symmetric. To avoid emitting both
/// `(A, B)` and `(B, A)`, we canonicalise each pair with the
/// lexicographically smaller root first. The emitted fact has
/// `subject = min(A, B)` and `object = max(A, B)`.
///
/// Tautology: A = B is impossible here because we iterate over
/// distinct subjects sharing a target.

/// `R2`: Has inheritance through IsA. If `A IsA B` and `B Has X`,
/// derive `A Has X` — a kind inherits properties from its super-type.
/// Activated in v2.8.
///
/// Soundness note: this is **conservative monotonic inheritance**. It
/// assumes that "Has" relationships documented at a higher type level
/// apply to all subtypes. In natural language this can fail (бала IsA
/// адам and адам Has автокөлік does NOT always mean бала Has автокөлік),
/// so we label every derivation `ConfidenceKind::RuleInferred` — never
/// `Grammar` — and keep the rule active only because downstream
/// consumers can filter by confidence kind.
///
/// Tautology guard: A = X is rejected (avoid `A Has A` nonsense).
fn rule_r2_has_inheritance(
    facts: &[Fact],
    graph: &LexicalGraph,
    seen: &BTreeSet<(String, String, String)>,
    out: &mut Vec<DerivedFact>,
) {
    for first in facts {
        if first.predicate != Predicate::IsA {
            continue;
        }
        // A IsA B — now find every (B Has X) edge in the graph.
        for second in graph.outgoing(&first.object.root) {
            if second.predicate != Predicate::Has {
                continue;
            }
            // A = X tautology.
            if first.subject.root == second.to {
                continue;
            }
            let key = (
                first.subject.root.clone(),
                "has".to_string(),
                second.to.clone(),
            );
            if seen.contains(&key) {
                continue;
            }
            let second_source = second
                .sources
                .first()
                .cloned()
                .expect("graph edge must carry at least one source");
            out.push(DerivedFact {
                subject: first.subject.clone(),
                predicate: Predicate::Has,
                object: SlotRef {
                    surface: second.to.clone(),
                    root: second.to.clone(),
                    pos: "noun".into(),
                },
                rule_id: "R2_has_inheritance".into(),
                source_chain: vec![first.source.clone(), second_source],
                confidence: ConfidenceKind::RuleInferred,
            });
        }
    }
}

fn rule_r5_shared_is_a_target(
    _facts: &[Fact],
    graph: &LexicalGraph,
    seen: &BTreeSet<(String, String, String)>,
    out: &mut Vec<DerivedFact>,
) {
    // Track pairs we've already emitted in THIS pass (guarding against
    // a hub node with many incoming IS-A edges producing one pair twice
    // from different orderings). Seen-set dedup across passes is the
    // caller's responsibility.
    let mut pass_pairs: BTreeSet<(String, String)> = BTreeSet::new();

    // Scan every node; if it has ≥ 2 incoming IsA edges, every pair
    // of those incoming subjects is RelatedTo each other via it.
    for (hub, _stats) in graph.nodes.iter() {
        let incoming_is_a: Vec<&crate::graph::GraphEdge> = graph
            .incoming(hub)
            .into_iter()
            .filter(|e| e.predicate == Predicate::IsA)
            .collect();
        if incoming_is_a.len() < 2 {
            continue;
        }
        // Every pair (a, b) with a < b lexicographically.
        for (i, first) in incoming_is_a.iter().enumerate() {
            for second in &incoming_is_a[i + 1..] {
                let (a, b) = canonical_relation_pair(&first.from, &second.from);
                if a == b {
                    continue; // defensive
                }
                if !pass_pairs.insert((a.clone(), b.clone())) {
                    continue;
                }
                let key = (a.clone(), "related_to".to_string(), b.clone());
                if seen.contains(&key) {
                    continue;
                }
                // Source chain = one source per side of the hub.
                let first_source = first
                    .sources
                    .first()
                    .cloned()
                    .expect("graph edge must carry at least one source");
                let second_source = second
                    .sources
                    .first()
                    .cloned()
                    .expect("graph edge must carry at least one source");
                out.push(DerivedFact {
                    subject: SlotRef {
                        surface: a.clone(),
                        root: a,
                        pos: "noun".into(),
                    },
                    predicate: Predicate::RelatedTo,
                    object: SlotRef {
                        surface: b.clone(),
                        root: b,
                        pos: "noun".into(),
                    },
                    rule_id: "R5_shared_is_a_target".into(),
                    source_chain: vec![first_source, second_source],
                    confidence: ConfidenceKind::RuleInferred,
                });
            }
        }
    }
}

/// Stable triple key for dedup — same shape used by LexicalGraph edge
/// sorting, so consumers can share ordering.
fn fact_triple_key(f: &Fact) -> (String, String, String) {
    (
        f.subject.root.clone(),
        f.predicate.as_str().to_string(),
        f.object.root.clone(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConfidenceKind, FactSource, SlotRef};

    fn mk_fact(subj: &str, pred: Predicate, obj: &str, pack: &str, id: &str) -> Fact {
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
            raw_text: format!("{subj} — {obj}"),
        }
    }

    #[test]
    fn r1_derives_is_a_transitivity() {
        let facts = vec![
            mk_fact("кітап", Predicate::IsA, "құрал", "p1", "s1"),
            mk_fact("құрал", Predicate::IsA, "зат", "p2", "s2"),
        ];
        let derived = run(&facts);
        // R1 derives кітап IsA зат. R5 then also fires on the
        // resulting graph because after R1, both кітап and құрал
        // share the target `зат`, yielding кітап RelatedTo құрал.
        // That's correct interleaving behaviour; the test asserts
        // R1's contribution specifically.
        let r1: Vec<_> = derived
            .iter()
            .filter(|d| d.rule_id == "R1_is_a_transitivity")
            .collect();
        assert_eq!(r1.len(), 1, "exactly one R1 derivation expected");
        let d = r1[0];
        assert_eq!(d.subject.root, "кітап");
        assert_eq!(d.object.root, "зат");
        assert_eq!(d.predicate, Predicate::IsA);
        assert_eq!(d.confidence, ConfidenceKind::RuleInferred);
        assert_eq!(d.source_chain.len(), 2);
    }

    #[test]
    fn r1_chains_three_hops() {
        // A → B → C → D should become {A IsA C, A IsA D, B IsA D}
        // after enough iterations.
        let facts = vec![
            mk_fact("A", Predicate::IsA, "B", "p", "s1"),
            mk_fact("B", Predicate::IsA, "C", "p", "s2"),
            mk_fact("C", Predicate::IsA, "D", "p", "s3"),
        ];
        let derived = run(&facts);
        let triples: BTreeSet<_> = derived
            .iter()
            .map(|d| (d.subject.root.clone(), d.object.root.clone()))
            .collect();
        assert!(triples.contains(&("A".to_string(), "C".to_string())));
        assert!(triples.contains(&("A".to_string(), "D".to_string())));
        assert!(triples.contains(&("B".to_string(), "D".to_string())));
    }

    #[test]
    fn r1_rejects_tautology() {
        // A IsA B ∧ B IsA A should NOT derive A IsA A or B IsA B.
        let facts = vec![
            mk_fact("A", Predicate::IsA, "B", "p", "s1"),
            mk_fact("B", Predicate::IsA, "A", "p", "s2"),
        ];
        let derived = run(&facts);
        for d in &derived {
            assert_ne!(
                d.subject.root, d.object.root,
                "R1 must never derive A IsA A: {d:?}"
            );
        }
    }

    #[test]
    fn reasoner_reaches_fixpoint() {
        // Running the reasoner on (initial + derived) should add nothing.
        let facts = vec![
            mk_fact("A", Predicate::IsA, "B", "p", "s1"),
            mk_fact("B", Predicate::IsA, "C", "p", "s2"),
        ];
        let first = run(&facts);
        let mut combined: Vec<Fact> = facts.clone();
        for d in &first {
            combined.push(d.clone().into_fact());
        }
        let second = run(&combined);
        assert!(
            second.is_empty(),
            "reasoner must be idempotent after fixpoint: got {second:?}"
        );
    }

    #[test]
    fn derived_fact_has_nonempty_source_chain() {
        let facts = vec![
            mk_fact("A", Predicate::IsA, "B", "p", "s1"),
            mk_fact("B", Predicate::IsA, "C", "p", "s2"),
        ];
        let derived = run(&facts);
        for d in &derived {
            assert!(
                !d.source_chain.is_empty(),
                "derived fact must have ≥ 1 source: {d:?}"
            );
        }
    }

    #[test]
    fn derived_fact_always_rule_inferred_confidence() {
        let facts = vec![
            mk_fact("A", Predicate::IsA, "B", "p", "s1"),
            mk_fact("B", Predicate::IsA, "C", "p", "s2"),
        ];
        let derived = run(&facts);
        for d in &derived {
            assert_eq!(d.confidence, ConfidenceKind::RuleInferred);
        }
    }

    #[test]
    fn into_fact_promotes_cleanly() {
        let facts = vec![
            mk_fact("A", Predicate::IsA, "B", "p", "s1"),
            mk_fact("B", Predicate::IsA, "C", "p", "s2"),
        ];
        let derived = run(&facts).into_iter().next().unwrap();
        let f: Fact = derived.clone().into_fact();
        assert_eq!(f.pattern, derived.rule_id);
        assert_eq!(f.confidence, ConfidenceKind::RuleInferred);
        assert_eq!(f.subject.root, derived.subject.root);
        assert_eq!(f.object.root, derived.object.root);
    }

    #[test]
    fn empty_input_empty_output() {
        assert!(run(&[]).is_empty());
    }

    #[test]
    fn r5_derives_related_to_from_shared_target() {
        // A IsA X, B IsA X → (A, RelatedTo, B) (canonical order)
        let facts = vec![
            mk_fact("кітап", Predicate::IsA, "бұлақ", "p", "s1"),
            mk_fact("ілім", Predicate::IsA, "бұлақ", "p", "s2"),
        ];
        let derived = run(&facts);
        // Find the RelatedTo derivation.
        let rel: Vec<_> = derived
            .iter()
            .filter(|d| d.predicate == Predicate::RelatedTo)
            .collect();
        assert_eq!(
            rel.len(),
            1,
            "exactly one RelatedTo pair expected from two shared-target facts: got {rel:?}"
        );
        let r = &rel[0];
        assert_eq!(r.rule_id, "R5_shared_is_a_target");
        assert_eq!(r.confidence, ConfidenceKind::RuleInferred);
        assert_eq!(r.source_chain.len(), 2);
        // Canonical ordering: lexicographically smaller first.
        let (a, b) = canonical_relation_pair("кітап", "ілім");
        assert_eq!(r.subject.root, a);
        assert_eq!(r.object.root, b);
    }

    #[test]
    fn r5_no_derivation_without_shared_target() {
        let facts = vec![
            mk_fact("A", Predicate::IsA, "X", "p", "s1"),
            mk_fact("B", Predicate::IsA, "Y", "p", "s2"),
        ];
        let derived = run(&facts);
        let rel: Vec<_> = derived
            .iter()
            .filter(|d| d.predicate == Predicate::RelatedTo)
            .collect();
        assert!(rel.is_empty(), "no shared target → no RelatedTo");
    }

    #[test]
    fn r5_three_way_hub_produces_three_pairs() {
        // A, B, C all IsA X → 3 RelatedTo pairs: (A,B), (A,C), (B,C).
        let facts = vec![
            mk_fact("A", Predicate::IsA, "X", "p", "s1"),
            mk_fact("B", Predicate::IsA, "X", "p", "s2"),
            mk_fact("C", Predicate::IsA, "X", "p", "s3"),
        ];
        let derived = run(&facts);
        let rel: Vec<_> = derived
            .iter()
            .filter(|d| d.predicate == Predicate::RelatedTo)
            .collect();
        assert_eq!(
            rel.len(),
            3,
            "three-way hub must yield 3 unique pairs: got {rel:?}"
        );
    }

    #[test]
    fn r5_symmetry_dedups_pairs() {
        // Even if the input order flips, only one RelatedTo per pair.
        let facts = vec![
            mk_fact("B", Predicate::IsA, "X", "p", "s1"),
            mk_fact("A", Predicate::IsA, "X", "p", "s2"),
        ];
        let derived = run(&facts);
        let rel: Vec<_> = derived
            .iter()
            .filter(|d| d.predicate == Predicate::RelatedTo)
            .collect();
        assert_eq!(rel.len(), 1);
        // Canonical order: A first.
        assert_eq!(rel[0].subject.root, "A");
        assert_eq!(rel[0].object.root, "B");
    }

    #[test]
    fn canonical_relation_pair_is_sorted() {
        assert_eq!(
            canonical_relation_pair("кітап", "ілім"),
            ("кітап".to_string(), "ілім".to_string())
        );
        assert_eq!(
            canonical_relation_pair("ілім", "кітап"),
            ("кітап".to_string(), "ілім".to_string())
        );
    }

    #[test]
    fn r2_derives_has_inheritance() {
        // бала IsA адам, адам Has жан ⟹ бала Has жан
        let facts = vec![
            mk_fact("бала", Predicate::IsA, "адам", "p1", "s1"),
            mk_fact("адам", Predicate::Has, "жан", "p2", "s2"),
        ];
        let derived = run(&facts);
        let r2: Vec<_> = derived
            .iter()
            .filter(|d| d.rule_id == "R2_has_inheritance")
            .collect();
        assert_eq!(r2.len(), 1);
        let d = r2[0];
        assert_eq!(d.subject.root, "бала");
        assert_eq!(d.predicate, Predicate::Has);
        assert_eq!(d.object.root, "жан");
        assert_eq!(d.confidence, ConfidenceKind::RuleInferred);
        assert_eq!(d.source_chain.len(), 2);
    }

    #[test]
    fn r2_respects_tautology_guard() {
        // A IsA X, X Has A → would produce A Has A (tautology).
        let facts = vec![
            mk_fact("A", Predicate::IsA, "X", "p", "s1"),
            mk_fact("X", Predicate::Has, "A", "p", "s2"),
        ];
        let derived = run(&facts);
        for d in &derived {
            assert!(
                !(d.subject.root == d.object.root && d.rule_id == "R2_has_inheritance"),
                "R2 must never derive A Has A: {d:?}"
            );
        }
    }

    #[test]
    fn r2_does_not_fire_without_has_edge() {
        // A IsA B, but no B Has X → no R2 derivations.
        let facts = vec![
            mk_fact("A", Predicate::IsA, "B", "p", "s1"),
            mk_fact("C", Predicate::Has, "D", "p", "s2"),
        ];
        let derived = run(&facts);
        let r2: Vec<_> = derived
            .iter()
            .filter(|d| d.rule_id == "R2_has_inheritance")
            .collect();
        assert!(r2.is_empty());
    }
}
