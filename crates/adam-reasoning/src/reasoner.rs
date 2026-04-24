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
//! | `R3_has_inheritance_via_part_of` | A Has B ∧ B PartOf C | A Has C (v3.5.5 — mereological inheritance; activated with the structural_part_of matcher) |
//! | `R4_is_a_symmetry_filter` | A IsA B ∧ B IsA A | flag both for curator review (returns diagnostic, not a fact) |
//! | `R5_shared_is_a_target` | A IsA X ∧ B IsA X (A ≠ B) | RelatedTo(A, B) |
//! | `R6_lives_in_via_part_of` | A LivesIn B ∧ B PartOf C | A LivesIn C (v3.9.5 — spatial inheritance; activated once v3.8.5 verb-root fix gave LivesIn real data) |
//! | `R7_goes_to_via_part_of` | A GoesTo B ∧ B PartOf C | A GoesTo C (v3.9.5 — directional inheritance; same motivation as R6) |
//! | `R8_after_transitivity` | A After B ∧ B After C | A After C (v4.0.4 — pure temporal order transitivity; math-clean, overreach-free) |
//! | `R9_part_of_transitivity` | A PartOf B ∧ B PartOf C | A PartOf C (v4.0.13 — pure mereological transitivity; partial order; `is_astronomical_object` cross-scale guard inherited from R6/R7) |
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

use crate::{
    ConfidenceKind, Fact, FactSource, Predicate, SlotRef, graph::LexicalGraph,
    harness::IterationBudget,
};

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

/// v4.0.3 — classify a [`DerivedFact`] by whether every source in its
/// `source_chain` points to a `data/world_core/*.jsonl` pack (i.e. every
/// supporting fact is human-reviewed). The predicate is shared by
/// `adam_demo` Part 4 (investor-safe default since v4.0.2) and
/// `adam_chat --safe` (investor-safe REPL since v4.0.3).
///
/// Empty `source_chain` is treated as NOT curated. The reasoner
/// invariant already forbids emitting derivations with empty chains,
/// but failing closed here guards any future regression.
///
/// Prefix match `"world_core/"` requires the **trailing slash** so a
/// hypothetical `world_core_mirror/...` pack never satisfies the
/// predicate by accident.
pub fn derivation_is_fully_curated(d: &DerivedFact) -> bool {
    !d.source_chain.is_empty()
        && d.source_chain
            .iter()
            .all(|s| s.pack.starts_with("world_core/"))
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
    let budget = IterationBudget::unbounded_for_tests();
    let (derived, _) = run_with_budget(initial_facts, &budget);
    derived
}

/// Budget-aware variant — returns `(derived_facts, iterations_completed)`.
///
/// v3.1.0: the reasoner checks `budget.should_stop()` between iterations.
/// On stop, the current set of derived facts is returned — the caller
/// commits them with the appropriate `status`. Inside a single iteration
/// the reasoner runs to completion (each pass is O(|facts| × fanout)
/// and at current scale fits in well under one second).
///
/// When unbudgeted, behaviour is byte-identical to [`run`] — same
/// iteration cap, same merge order, same dedup.
pub fn run_with_budget(
    initial_facts: &[Fact],
    budget: &IterationBudget,
) -> (Vec<DerivedFact>, usize) {
    let mut all_facts: Vec<Fact> = initial_facts.to_vec();
    let mut derived: Vec<DerivedFact> = Vec::new();
    let mut seen_triples: BTreeSet<(String, String, String)> =
        initial_facts.iter().map(|f| fact_triple_key(f)).collect();
    let mut iterations_completed = 0usize;

    for _iter in 0..MAX_ITER {
        if budget.should_stop() {
            break;
        }
        let graph = LexicalGraph::from_facts(&all_facts);
        let new_derived = run_one_pass(&all_facts, &graph, &seen_triples);
        if new_derived.is_empty() {
            iterations_completed += 1;
            break;
        }
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
        iterations_completed += 1;
    }

    (derived, iterations_completed)
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
    rule_r3_has_inheritance_via_part_of(facts, graph, seen, &mut out);
    rule_r5_shared_is_a_target(facts, graph, seen, &mut out);
    rule_r6_lives_in_via_part_of(facts, graph, seen, &mut out);
    rule_r7_goes_to_via_part_of(facts, graph, seen, &mut out);
    rule_r8_after_transitivity(facts, graph, seen, &mut out);
    rule_r9_part_of_transitivity(facts, graph, seen, &mut out);
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

/// `R3`: Has-inheritance via PartOf (mereological). If
/// `A Has B` and `B PartOf C`, derive `A Has C`.
///
/// Intuition: if A owns B, and B is structurally part of C, A
/// effectively has a claim on the whole of C (or at least on the
/// presence of C via B). In natural language this is weaker than the
/// object-level claim — «адам Has денеміздің бөлігі» — so every
/// derivation is labelled `ConfidenceKind::RuleInferred` (never
/// Grammar) and downstream consumers can filter by confidence kind.
///
/// Tautology guard: A = C rejected.
fn rule_r3_has_inheritance_via_part_of(
    facts: &[Fact],
    graph: &LexicalGraph,
    seen: &BTreeSet<(String, String, String)>,
    out: &mut Vec<DerivedFact>,
) {
    for first in facts {
        if first.predicate != Predicate::Has {
            continue;
        }
        // A Has B — find every (B PartOf C) edge in the graph.
        for second in graph.outgoing(&first.object.root) {
            if second.predicate != Predicate::PartOf {
                continue;
            }
            // A = C tautology.
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
                rule_id: "R3_has_inheritance_via_part_of".into(),
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

/// `R6`: Spatial inheritance via PartOf. If `A LivesIn B` and
/// `B PartOf C`, derive `A LivesIn C`.
///
/// Example: `(Дәулет, LivesIn, Қостанай) ∧ (Қостанай, PartOf, Қазақстан)`
/// ⟹ `(Дәулет, LivesIn, Қазақстан)`. A person who lives in a city also
/// lives in the country that city is part of.
///
/// Added v3.9.5 — waited on (a) v3.8.0's verb-root bug fix that gave
/// `LivesIn` real data for the first time, and (b) v3.9.0's World Core
/// which contributed the `city PartOf country` chain via curated
/// `geography_kz.jsonl` entries. Before these, R6 would have fired zero
/// times; the rule is architecturally correct regardless.
///
/// **v4.0.0 guard** — refuse derivations where the target `C` is an
/// astronomical-scale object (`is_astronomical_object`). Codex v3.9.5
/// review flagged «бала lives_in күн жүйесі» as a canonical false
/// chain: the homonymous «жер» (both "ground" and "Earth") bridges
/// two unrelated semantic domains. Blocking astronomical targets in
/// R6 output resolves the cross-domain absurdity without needing
/// per-sense disambiguation of the intermediate node.
///
/// Tautology guard: A = C rejected.
fn rule_r6_lives_in_via_part_of(
    facts: &[Fact],
    graph: &LexicalGraph,
    seen: &BTreeSet<(String, String, String)>,
    out: &mut Vec<DerivedFact>,
) {
    for first in facts {
        if first.predicate != Predicate::LivesIn {
            continue;
        }
        // A LivesIn B — find every (B PartOf C) edge in the graph.
        for second in graph.outgoing(&first.object.root) {
            if second.predicate != Predicate::PartOf {
                continue;
            }
            // A = C tautology.
            if first.subject.root == second.to {
                continue;
            }
            // v4.0.0 — refuse astronomical-scale derived targets.
            // «(бала, LivesIn, жер)» + «(жер, PartOf, күн жүйесі)»
            // must NOT produce «(бала, LivesIn, күн жүйесі)».
            if crate::patterns::is_astronomical_object(&second.to) {
                continue;
            }
            let key = (
                first.subject.root.clone(),
                "lives_in".to_string(),
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
                predicate: Predicate::LivesIn,
                object: SlotRef {
                    surface: second.to.clone(),
                    root: second.to.clone(),
                    pos: "noun".into(),
                },
                rule_id: "R6_lives_in_via_part_of".into(),
                source_chain: vec![first.source.clone(), second_source],
                confidence: ConfidenceKind::RuleInferred,
            });
        }
    }
}

/// `R7`: Directional inheritance via PartOf. If `A GoesTo B` and
/// `B PartOf C`, derive `A GoesTo C`.
///
/// Symmetric structure to R6, applied to motion rather than residence.
/// «Ол Алматыға барды» + «Алматы Қазақстанның бөлігі» ⟹ «Ол Қазақстанға
/// барды». Added v3.9.5 for the same reason as R6 — requires v3.8.0's
/// verb-root fix + v3.9.0's curated PartOf chains.
///
/// Tautology guard: A = C rejected.
fn rule_r7_goes_to_via_part_of(
    facts: &[Fact],
    graph: &LexicalGraph,
    seen: &BTreeSet<(String, String, String)>,
    out: &mut Vec<DerivedFact>,
) {
    for first in facts {
        if first.predicate != Predicate::GoesTo {
            continue;
        }
        for second in graph.outgoing(&first.object.root) {
            if second.predicate != Predicate::PartOf {
                continue;
            }
            if first.subject.root == second.to {
                continue;
            }
            // v4.0.0 — same astronomical-target guard as R6.
            // «(жалға, GoesTo, жер)» was a Codex-flagged FST-misparse
            // chain; blocking astronomical targets prunes the noisiest
            // branch of R7 output. Legitimate country-level chains
            // (e.g. `(X, GoesTo, Алматы)` → `(X, GoesTo, Қазақстан)`)
            // still fire.
            if crate::patterns::is_astronomical_object(&second.to) {
                continue;
            }
            let key = (
                first.subject.root.clone(),
                "goes_to".to_string(),
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
                predicate: Predicate::GoesTo,
                object: SlotRef {
                    surface: second.to.clone(),
                    root: second.to.clone(),
                    pos: "noun".into(),
                },
                rule_id: "R7_goes_to_via_part_of".into(),
                source_chain: vec![first.source.clone(), second_source],
                confidence: ConfidenceKind::RuleInferred,
            });
        }
    }
}

/// `R8`: Temporal-order transitivity. If `A After B` and `B After C`,
/// derive `A After C`.
///
/// Added v4.0.4. `After` is a **strict partial order** — mathematically
/// the cleanest predicate to make transitive. If X happens after Y and
/// Y happens after Z, then X happens after Z. No semantic overreach
/// risk, unlike Has-transitivity (which mixes ownership with
/// composition) or LivesIn-transitivity (which mixes residence with
/// physical inclusion).
///
/// Example chain from curated `world_core/time.jsonl`:
///
/// - `(жаз, After, көктем)` + `(күз, After, жаз)` ⟹ `(күз, After, көктем)`
/// - `(күз, After, жаз)` + `(қыс, After, күз)` ⟹ `(қыс, After, жаз)`
///
/// After two fixpoint passes this gives full orderings like
/// `(қыс, After, көктем)` — the full seasonal round.
///
/// Invariants: `After` is anti-symmetric (`A After B` forbids
/// `B After A` — but the reasoner doesn't enforce this; it would be a
/// contradiction in the data). `MAX_ITER` bounds chain length.
///
/// Tautology guard: `A = C` rejected (defensive, though a well-formed
/// `After` chain never produces it).
fn rule_r8_after_transitivity(
    facts: &[Fact],
    graph: &LexicalGraph,
    seen: &BTreeSet<(String, String, String)>,
    out: &mut Vec<DerivedFact>,
) {
    for first in facts {
        if first.predicate != Predicate::After {
            continue;
        }
        // A After B — find every (B After C) edge in the graph.
        for second in graph.outgoing(&first.object.root) {
            if second.predicate != Predicate::After {
                continue;
            }
            // A = C tautology — defensive. Well-formed After chains
            // are anti-symmetric so this would mean A After B ∧ B After
            // A, a contradiction in the data, not a rule-safe
            // derivation.
            if first.subject.root == second.to {
                continue;
            }
            let key = (
                first.subject.root.clone(),
                "after".to_string(),
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
                predicate: Predicate::After,
                object: SlotRef {
                    surface: second.to.clone(),
                    root: second.to.clone(),
                    pos: "noun".into(),
                },
                rule_id: "R8_after_transitivity".into(),
                source_chain: vec![first.source.clone(), second_source],
                confidence: ConfidenceKind::RuleInferred,
            });
        }
    }
}

/// v4.0.13 — **R9 PartOf transitivity**. `PartOf` is a partial order
/// (reflexive, anti-symmetric, transitive); R9 closes the transitive
/// part. Semantically: if A is a part of B and B is a part of C, then
/// A is a part of C — the classic mereological axiom.
///
/// Examples this closes (on v4.0.12 data):
///
///   - `шаш part_of бас ∧ бас part_of дене ⟹ шаш part_of дене`
///   - `жер part_of күн жүйесі ∧ күн жүйесі part_of галактика ⟹ жер part_of галактика`
///   - `жапырақ part_of ағаш ∧ ... (future)` → propagates once plants-in-forest / trees-in-ecosystem entries land.
///
/// **Tautology guard**: `A = C` rejected. Mereological partial orders
/// forbid cycles, so a well-formed PartOf chain never produces
/// `A PartOf A`; this is defensive against noisy input.
///
/// **Cross-scale guard (inherited from R6/R7 pattern)**: if `C` is an
/// astronomical-scale object (`is_astronomical_object`) and `A` is
/// **not**, the derivation is rejected. Prevents «жапырақ part_of
/// ағаш part_of орман part_of ... part_of галактика» style cross-scale
/// leaks that would emerge once future data adds intermediate links.
/// Pure astronomy chains (жер / марс / шолпан part_of күн жүйесі
/// part_of галактика) are unaffected — both ends are astronomical.
///
/// R9 does NOT validate whether each input PartOf fact is semantically
/// correct; noise in the base set (e.g. `теңіз part_of өсімдік` from
/// text extraction) propagates through, same as every other rule. The
/// `derivation_is_fully_curated` helper stays the recommended filter
/// for investor-safe dialog surfaces.
fn rule_r9_part_of_transitivity(
    facts: &[Fact],
    graph: &LexicalGraph,
    seen: &BTreeSet<(String, String, String)>,
    out: &mut Vec<DerivedFact>,
) {
    for first in facts {
        if first.predicate != Predicate::PartOf {
            continue;
        }
        for second in graph.outgoing(&first.object.root) {
            if second.predicate != Predicate::PartOf {
                continue;
            }
            if first.subject.root == second.to {
                continue;
            }
            // Cross-scale guard — same pattern as R6/R7.
            if crate::patterns::is_astronomical_object(&second.to)
                && !crate::patterns::is_astronomical_object(&first.subject.root)
            {
                continue;
            }
            let key = (
                first.subject.root.clone(),
                "part_of".to_string(),
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
                predicate: Predicate::PartOf,
                object: SlotRef {
                    surface: second.to.clone(),
                    root: second.to.clone(),
                    pos: "noun".into(),
                },
                rule_id: "R9_part_of_transitivity".into(),
                source_chain: vec![first.source.clone(), second_source],
                confidence: ConfidenceKind::RuleInferred,
            });
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

    // ------------------------- R3 (v3.5.5) --------------------------------

    #[test]
    fn r3_derives_has_inheritance_via_part_of() {
        // адам Has жүрек, жүрек PartOf дене ⟹ адам Has дене
        let facts = vec![
            mk_fact("адам", Predicate::Has, "жүрек", "p1", "s1"),
            mk_fact("жүрек", Predicate::PartOf, "дене", "p2", "s2"),
        ];
        let derived = run(&facts);
        let r3: Vec<_> = derived
            .iter()
            .filter(|d| d.rule_id == "R3_has_inheritance_via_part_of")
            .collect();
        assert_eq!(r3.len(), 1, "R3 should produce exactly one derivation");
        let d = r3[0];
        assert_eq!(d.subject.root, "адам");
        assert_eq!(d.predicate, Predicate::Has);
        assert_eq!(d.object.root, "дене");
        assert_eq!(d.confidence, ConfidenceKind::RuleInferred);
        assert_eq!(
            d.source_chain.len(),
            2,
            "R3 source chain must have both fact sources"
        );
    }

    #[test]
    fn r3_respects_tautology_guard() {
        // A Has B, B PartOf A → A Has A tautology; must NOT derive.
        let facts = vec![
            mk_fact("A", Predicate::Has, "B", "p", "s1"),
            mk_fact("B", Predicate::PartOf, "A", "p", "s2"),
        ];
        let derived = run(&facts);
        for d in &derived {
            assert!(
                !(d.subject.root == d.object.root && d.rule_id == "R3_has_inheritance_via_part_of"),
                "R3 must never derive A Has A: {d:?}"
            );
        }
    }

    #[test]
    fn r3_does_not_fire_without_part_of_edge() {
        // A Has B, no B PartOf X anywhere.
        let facts = vec![
            mk_fact("A", Predicate::Has, "B", "p", "s1"),
            mk_fact("C", Predicate::PartOf, "D", "p", "s2"),
        ];
        let derived = run(&facts);
        let r3: Vec<_> = derived
            .iter()
            .filter(|d| d.rule_id == "R3_has_inheritance_via_part_of")
            .collect();
        assert!(r3.is_empty());
    }

    #[test]
    fn r3_dedupes_against_existing_facts() {
        // адам Has жүрек, жүрек PartOf дене, адам Has дене (already known)
        //   → R3 MUST NOT re-emit a duplicate.
        let facts = vec![
            mk_fact("адам", Predicate::Has, "жүрек", "p", "s1"),
            mk_fact("жүрек", Predicate::PartOf, "дене", "p", "s2"),
            mk_fact("адам", Predicate::Has, "дене", "p", "s3"),
        ];
        let derived = run(&facts);
        let r3: Vec<_> = derived
            .iter()
            .filter(|d| d.rule_id == "R3_has_inheritance_via_part_of")
            .collect();
        assert!(r3.is_empty(), "R3 must not duplicate an existing fact");
    }

    // ------------------------- R6 / R7 (v3.9.5) -------------------------

    #[test]
    fn r6_derives_lives_in_via_part_of() {
        // Дәулет LivesIn Қостанай, Қостанай PartOf Қазақстан
        //   ⟹ Дәулет LivesIn Қазақстан
        let facts = vec![
            mk_fact("дәулет", Predicate::LivesIn, "қостанай", "wiki", "s1"),
            mk_fact(
                "қостанай",
                Predicate::PartOf,
                "қазақстан",
                "world_core",
                "geo_013",
            ),
        ];
        let derived = run(&facts);
        let r6: Vec<_> = derived
            .iter()
            .filter(|d| d.rule_id == "R6_lives_in_via_part_of")
            .collect();
        assert_eq!(r6.len(), 1, "R6 should fire exactly once (got {derived:?})");
        let d = r6[0];
        assert_eq!(d.subject.root, "дәулет");
        assert_eq!(d.predicate, Predicate::LivesIn);
        assert_eq!(d.object.root, "қазақстан");
        assert_eq!(d.confidence, ConfidenceKind::RuleInferred);
        assert_eq!(d.source_chain.len(), 2);
    }

    #[test]
    fn r6_respects_tautology_guard() {
        let facts = vec![
            mk_fact("A", Predicate::LivesIn, "B", "p", "s1"),
            mk_fact("B", Predicate::PartOf, "A", "p", "s2"),
        ];
        let derived = run(&facts);
        for d in &derived {
            assert!(
                !(d.subject.root == d.object.root && d.rule_id == "R6_lives_in_via_part_of"),
                "R6 must never derive A LivesIn A: {d:?}"
            );
        }
    }

    #[test]
    fn r6_does_not_fire_without_part_of_edge() {
        let facts = vec![mk_fact(
            "дәулет",
            Predicate::LivesIn,
            "қостанай",
            "wiki",
            "s1",
        )];
        let derived = run(&facts);
        let r6: Vec<_> = derived
            .iter()
            .filter(|d| d.rule_id == "R6_lives_in_via_part_of")
            .collect();
        assert!(r6.is_empty());
    }

    #[test]
    fn r6_dedupes_against_existing_fact() {
        let facts = vec![
            mk_fact("дәулет", Predicate::LivesIn, "қостанай", "wiki", "s1"),
            mk_fact("қостанай", Predicate::PartOf, "қазақстан", "wc", "geo"),
            mk_fact("дәулет", Predicate::LivesIn, "қазақстан", "explicit", "s3"),
        ];
        let derived = run(&facts);
        let r6: Vec<_> = derived
            .iter()
            .filter(|d| d.rule_id == "R6_lives_in_via_part_of")
            .collect();
        assert!(
            r6.is_empty(),
            "R6 must not re-derive an already-asserted LivesIn fact"
        );
    }

    #[test]
    fn r7_derives_goes_to_via_part_of() {
        // ол GoesTo Алматы, Алматы PartOf Қазақстан
        //   ⟹ ол GoesTo Қазақстан
        let facts = vec![
            mk_fact("балалар", Predicate::GoesTo, "алматы", "wiki", "s1"),
            mk_fact(
                "алматы",
                Predicate::PartOf,
                "қазақстан",
                "world_core",
                "geo_004",
            ),
        ];
        let derived = run(&facts);
        let r7: Vec<_> = derived
            .iter()
            .filter(|d| d.rule_id == "R7_goes_to_via_part_of")
            .collect();
        assert_eq!(r7.len(), 1);
        let d = r7[0];
        assert_eq!(d.subject.root, "балалар");
        assert_eq!(d.predicate, Predicate::GoesTo);
        assert_eq!(d.object.root, "қазақстан");
        assert_eq!(d.confidence, ConfidenceKind::RuleInferred);
        assert_eq!(d.source_chain.len(), 2);
    }

    #[test]
    fn r7_respects_tautology_guard() {
        let facts = vec![
            mk_fact("A", Predicate::GoesTo, "B", "p", "s1"),
            mk_fact("B", Predicate::PartOf, "A", "p", "s2"),
        ];
        let derived = run(&facts);
        for d in &derived {
            assert!(
                !(d.subject.root == d.object.root && d.rule_id == "R7_goes_to_via_part_of"),
                "R7 must never derive A GoesTo A: {d:?}"
            );
        }
    }

    // ------------------- v4.0.0 astronomical-target guard ----------------

    #[test]
    fn r6_refuses_astronomical_derived_target() {
        // Codex v3.9.5-flagged chain: (бала, LivesIn, жер) + (жер, PartOf,
        // күн жүйесі) must NOT produce (бала, LivesIn, күн жүйесі).
        let facts = vec![
            mk_fact("бала", Predicate::LivesIn, "жер", "extracted", "s1"),
            mk_fact(
                "жер",
                Predicate::PartOf,
                "күн жүйесі",
                "world_core",
                "astro_001",
            ),
        ];
        let derived = run(&facts);
        let bad: Vec<_> = derived
            .iter()
            .filter(|d| {
                d.rule_id == "R6_lives_in_via_part_of"
                    && d.subject.root == "бала"
                    && d.object.root == "күн жүйесі"
            })
            .collect();
        assert!(
            bad.is_empty(),
            "R6 must not derive (бала, LivesIn, күн жүйесі) — astronomical target (got {derived:?})"
        );
    }

    #[test]
    fn r6_still_fires_for_country_target() {
        // Regression: the astronomical-target guard must NOT block the
        // legitimate `(person, LivesIn, city)` + `(city, PartOf, country)`
        // → `(person, LivesIn, country)` chain.
        let facts = vec![
            mk_fact("дәулет", Predicate::LivesIn, "қостанай", "wiki", "s1"),
            mk_fact(
                "қостанай",
                Predicate::PartOf,
                "қазақстан",
                "world_core",
                "geo_013",
            ),
        ];
        let derived = run(&facts);
        let ok: Vec<_> = derived
            .iter()
            .filter(|d| d.rule_id == "R6_lives_in_via_part_of")
            .collect();
        assert_eq!(
            ok.len(),
            1,
            "country-target R6 chain must still fire (got {derived:?})"
        );
        assert_eq!(ok[0].object.root, "қазақстан");
    }

    #[test]
    fn r7_refuses_astronomical_derived_target() {
        // «(жалға, GoesTo, жер)» + «(жер, PartOf, күн жүйесі)» must NOT
        // produce «(жалға, GoesTo, күн жүйесі)».
        let facts = vec![
            mk_fact("жалға", Predicate::GoesTo, "жер", "extracted", "s1"),
            mk_fact(
                "жер",
                Predicate::PartOf,
                "күн жүйесі",
                "world_core",
                "astro_001",
            ),
        ];
        let derived = run(&facts);
        let bad: Vec<_> = derived
            .iter()
            .filter(|d| d.rule_id == "R7_goes_to_via_part_of" && d.object.root == "күн жүйесі")
            .collect();
        assert!(
            bad.is_empty(),
            "R7 must not derive GoesTo against astronomical targets (got {derived:?})"
        );
    }

    // ------------------------- R8 (v4.0.4) ------------------------------

    #[test]
    fn r8_derives_after_transitivity() {
        // жаз After көктем, күз After жаз ⟹ күз After көктем
        let facts = vec![
            mk_fact("жаз", Predicate::After, "көктем", "world_core", "time_015"),
            mk_fact("күз", Predicate::After, "жаз", "world_core", "time_016"),
        ];
        let derived = run(&facts);
        let r8: Vec<_> = derived
            .iter()
            .filter(|d| d.rule_id == "R8_after_transitivity")
            .collect();
        assert_eq!(r8.len(), 1, "R8 should fire once (got {derived:?})");
        let d = r8[0];
        assert_eq!(d.subject.root, "күз");
        assert_eq!(d.predicate, Predicate::After);
        assert_eq!(d.object.root, "көктем");
        assert_eq!(d.confidence, ConfidenceKind::RuleInferred);
        assert_eq!(d.source_chain.len(), 2);
    }

    #[test]
    fn r8_respects_tautology_guard() {
        // A After B + B After A → A After A must be rejected.
        let facts = vec![
            mk_fact("A", Predicate::After, "B", "p", "s1"),
            mk_fact("B", Predicate::After, "A", "p", "s2"),
        ];
        let derived = run(&facts);
        for d in &derived {
            assert!(
                !(d.subject.root == d.object.root && d.rule_id == "R8_after_transitivity"),
                "R8 must never derive A After A: {d:?}"
            );
        }
    }

    #[test]
    fn r8_does_not_fire_without_chain() {
        // жаз After көктем alone — no chain to extend.
        let facts = vec![mk_fact(
            "жаз",
            Predicate::After,
            "көктем",
            "world_core",
            "time_015",
        )];
        let derived = run(&facts);
        let r8: Vec<_> = derived
            .iter()
            .filter(|d| d.rule_id == "R8_after_transitivity")
            .collect();
        assert!(r8.is_empty());
    }

    #[test]
    fn r8_dedupes_against_existing_fact() {
        // Full chain plus explicit long-arc: R8 must not re-derive.
        let facts = vec![
            mk_fact("жаз", Predicate::After, "көктем", "wc", "s1"),
            mk_fact("күз", Predicate::After, "жаз", "wc", "s2"),
            mk_fact("күз", Predicate::After, "көктем", "wc", "s3"),
        ];
        let derived = run(&facts);
        let r8: Vec<_> = derived
            .iter()
            .filter(|d| d.rule_id == "R8_after_transitivity")
            .collect();
        assert!(
            r8.is_empty(),
            "R8 must not duplicate an explicitly-asserted After fact"
        );
    }

    #[test]
    fn r8_chains_across_iterations() {
        // Four-season chain: reasoner should reach full closure in
        // bounded iterations. Start: көктем → жаз → күз → қыс.
        // Expected R8 derivations:
        //   (күз, After, көктем)  — iter 1
        //   (қыс, After, жаз)     — iter 1
        //   (қыс, After, көктем)  — iter 2 (from күз After көктем + қыс After күз)
        let facts = vec![
            mk_fact("жаз", Predicate::After, "көктем", "wc", "time_015"),
            mk_fact("күз", Predicate::After, "жаз", "wc", "time_016"),
            mk_fact("қыс", Predicate::After, "күз", "wc", "time_017"),
        ];
        let derived = run(&facts);
        let r8: Vec<_> = derived
            .iter()
            .filter(|d| d.rule_id == "R8_after_transitivity")
            .collect();
        assert!(
            r8.len() >= 3,
            "expected R8 to reach full closure (≥3 derivations), got {}: {derived:?}",
            r8.len()
        );
        let derived_pairs: std::collections::BTreeSet<(String, String)> = r8
            .iter()
            .map(|d| (d.subject.root.clone(), d.object.root.clone()))
            .collect();
        assert!(derived_pairs.contains(&("күз".into(), "көктем".into())));
        assert!(derived_pairs.contains(&("қыс".into(), "жаз".into())));
        assert!(derived_pairs.contains(&("қыс".into(), "көктем".into())));
    }

    // ------------------------- R9 (v4.0.13) -----------------------------

    #[test]
    fn r9_derives_part_of_transitivity() {
        // шаш part_of бас ∧ бас part_of дене ⟹ шаш part_of дене
        let facts = vec![
            mk_fact("шаш", Predicate::PartOf, "бас", "wc", "body_a"),
            mk_fact("бас", Predicate::PartOf, "дене", "wc", "body_b"),
        ];
        let derived = run(&facts);
        let r9: Vec<_> = derived
            .iter()
            .filter(|d| d.rule_id == "R9_part_of_transitivity")
            .collect();
        assert_eq!(r9.len(), 1, "R9 should fire once (got {derived:?})");
        let d = r9[0];
        assert_eq!(d.subject.root, "шаш");
        assert_eq!(d.predicate, Predicate::PartOf);
        assert_eq!(d.object.root, "дене");
        assert_eq!(d.confidence, ConfidenceKind::RuleInferred);
        assert_eq!(d.source_chain.len(), 2);
    }

    #[test]
    fn r9_respects_tautology_guard() {
        // Cyclic PartOf is pathological, but guard is defensive.
        let facts = vec![
            mk_fact("A", Predicate::PartOf, "B", "p", "s1"),
            mk_fact("B", Predicate::PartOf, "A", "p", "s2"),
        ];
        let derived = run(&facts);
        for d in &derived {
            assert!(
                !(d.subject.root == d.object.root && d.rule_id == "R9_part_of_transitivity"),
                "R9 must never derive A PartOf A: {d:?}"
            );
        }
    }

    #[test]
    fn r9_astronomy_same_scale_allowed() {
        // жер part_of күн жүйесі ∧ күн жүйесі part_of галактика
        // Both жер and галактика are astronomical → guard allows.
        let facts = vec![
            mk_fact("жер", Predicate::PartOf, "күн жүйесі", "wc", "astro_001"),
            mk_fact(
                "күн жүйесі",
                Predicate::PartOf,
                "галактика",
                "wc",
                "astro_013",
            ),
        ];
        let derived = run(&facts);
        let r9: Vec<_> = derived
            .iter()
            .filter(|d| d.rule_id == "R9_part_of_transitivity")
            .collect();
        assert_eq!(
            r9.len(),
            1,
            "астрономический same-scale: R9 должен сработать"
        );
        assert_eq!(r9[0].subject.root, "жер");
        assert_eq!(r9[0].object.root, "галактика");
    }

    #[test]
    fn r9_astronomy_cross_scale_rejected() {
        // Synthetic chain: бала part_of жер ∧ жер part_of күн жүйесі.
        // Subject бала is non-astronomical; target күн жүйесі is
        // astronomical → guard blocks derivation (same pattern as R6/R7
        // from v4.0.0).
        let facts = vec![
            mk_fact("бала", Predicate::PartOf, "жер", "p", "s1"),
            mk_fact("жер", Predicate::PartOf, "күн жүйесі", "wc", "astro_001"),
        ];
        let derived = run(&facts);
        let r9: Vec<_> = derived
            .iter()
            .filter(|d| d.rule_id == "R9_part_of_transitivity" && d.object.root == "күн жүйесі")
            .collect();
        assert!(
            r9.is_empty(),
            "R9 must reject cross-scale astronomical leaks: {derived:?}"
        );
    }

    #[test]
    fn r9_chains_across_iterations() {
        // Four-level chain: тіс part_of ауыз ∧ ауыз part_of бет ∧ бет
        // part_of бас ∧ бас part_of дене.
        // Expected derivations span all non-adjacent pairs.
        let facts = vec![
            mk_fact("тіс", Predicate::PartOf, "ауыз", "wc", "a"),
            mk_fact("ауыз", Predicate::PartOf, "бет", "wc", "b"),
            mk_fact("бет", Predicate::PartOf, "бас", "wc", "c"),
            mk_fact("бас", Predicate::PartOf, "дене", "wc", "d"),
        ];
        let derived = run(&facts);
        let r9: Vec<_> = derived
            .iter()
            .filter(|d| d.rule_id == "R9_part_of_transitivity")
            .collect();
        let pairs: std::collections::BTreeSet<(String, String)> = r9
            .iter()
            .map(|d| (d.subject.root.clone(), d.object.root.clone()))
            .collect();
        // Transitive closure over 4-node chain: C(4,2) - 3 adjacent = 3 derived.
        assert!(pairs.contains(&("тіс".into(), "бет".into())));
        assert!(pairs.contains(&("тіс".into(), "бас".into())));
        assert!(pairs.contains(&("тіс".into(), "дене".into())));
        assert!(pairs.contains(&("ауыз".into(), "бас".into())));
        assert!(pairs.contains(&("ауыз".into(), "дене".into())));
        assert!(pairs.contains(&("бет".into(), "дене".into())));
    }

    #[test]
    fn r9_dedupes_against_existing_fact() {
        // Full chain plus explicit long-arc: R9 must not re-derive.
        let facts = vec![
            mk_fact("шаш", Predicate::PartOf, "бас", "wc", "a"),
            mk_fact("бас", Predicate::PartOf, "дене", "wc", "b"),
            mk_fact("шаш", Predicate::PartOf, "дене", "wc", "c"),
        ];
        let derived = run(&facts);
        let r9: Vec<_> = derived
            .iter()
            .filter(|d| d.rule_id == "R9_part_of_transitivity")
            .collect();
        assert!(
            r9.is_empty(),
            "R9 must not duplicate an explicitly-asserted PartOf fact: {derived:?}"
        );
    }

    // ---------------- v4.0.3 derivation_is_fully_curated ----------------

    fn mk_derived(chain: &[&str]) -> DerivedFact {
        DerivedFact {
            subject: SlotRef {
                surface: "s".into(),
                root: "s".into(),
                pos: "noun".into(),
            },
            predicate: Predicate::IsA,
            object: SlotRef {
                surface: "o".into(),
                root: "o".into(),
                pos: "noun".into(),
            },
            rule_id: "R1".into(),
            source_chain: chain
                .iter()
                .map(|pack| FactSource {
                    pack: pack.to_string(),
                    sample_id: "id".into(),
                })
                .collect(),
            confidence: ConfidenceKind::RuleInferred,
        }
    }

    #[test]
    fn curated_predicate_accepts_world_core_only_chain() {
        let d = mk_derived(&[
            "world_core/astronomy.jsonl",
            "world_core/biology_basic.jsonl",
        ]);
        assert!(derivation_is_fully_curated(&d));
    }

    #[test]
    fn curated_predicate_rejects_mixed_chain() {
        let d = mk_derived(&["world_core/astronomy.jsonl", "wikipedia_kz_pack.json"]);
        assert!(
            !derivation_is_fully_curated(&d),
            "any non-world_core source disqualifies the chain"
        );
    }

    #[test]
    fn curated_predicate_rejects_text_only_chain() {
        let d = mk_derived(&["wikipedia_kz_pack.json", "kazakh_textbooks_pack.json"]);
        assert!(!derivation_is_fully_curated(&d));
    }

    #[test]
    fn curated_predicate_fails_closed_on_empty_chain() {
        let d = mk_derived(&[]);
        assert!(!derivation_is_fully_curated(&d));
    }

    #[test]
    fn curated_predicate_requires_trailing_slash_in_prefix() {
        // Guard against prefix collisions with hypothetical
        // `world_core_mirror` or `world_core_drafts` packs.
        let d = mk_derived(&["world_core_mirror/x.jsonl"]);
        assert!(!derivation_is_fully_curated(&d));
    }
}
