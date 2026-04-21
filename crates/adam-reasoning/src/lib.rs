//! adam-reasoning — Intelligent Lexical-Morphemic Retrieval & Reasoning (ILMRR).
//!
//! Stage: v2.3+ — **fact extraction + lexical graph projection**.
//!
//! This crate is the reasoning layer of the adam architecture. It takes
//! FST-parsed corpus samples and extracts **typed facts** with full
//! provenance, then projects those facts into a node-edge graph that
//! a future rule-reasoner (v2.4+) will traverse.
//!
//! Capabilities shipped:
//!
//! - **Fact extraction** (v2.1): three pattern matchers producing
//!   `IsA`, `LivesIn`, `Has` relations. See [`patterns`].
//! - **Lexical Graph v0** (v2.3): pure projection of facts into a
//!   directed typed graph with per-edge provenance. See [`graph`].
//!
//! The canonical example:
//!
//! ```text
//!   "Абай — ақын"  (Abai Wikisource, abai_00042)
//!       ↓
//!   Fact {
//!     subject:   SlotRef { root: "абай",  pos: "noun", surface: "абай" },
//!     predicate: Predicate::IsA,
//!     object:    SlotRef { root: "ақын",  pos: "noun", surface: "ақын" },
//!     pattern:   "X — Y",
//!     source:    { pack: "abai_wikisource_pack.json", sample_id: "abai_00042" },
//!     confidence: Grammar,
//!     raw_text:  "Абай — ақын",
//!   }
//! ```
//!
//! Facts are structured knowledge: they can be indexed, chained, and
//! reasoned over (v2.3+). Unlike retrieval hits, which are opaque
//! sentences, facts expose **subject → relation → object** shape that
//! a rule engine can match against.
//!
//! ## Determinism contract
//!
//! Every pattern matcher is a **pure function** of (parses, raw text).
//! No RNG, no learned weights, no probabilistic similarity. Same input
//! → same fact list, byte-identical across runs.
//!
//! ## Confidence is a TYPE, not a probability
//!
//! Per the `project_retrieval_not_neural_v2` commitment: `confidence`
//! is the **kind of evidence** backing a fact — never an LLM-style
//! probability score. A human or downstream consumer can filter by
//! confidence kind without trusting any learned magnitude.

pub mod graph;
pub mod patterns;
pub mod reasoner;

use adam_kernel_fst::lexicon::LexiconV1;
use adam_kernel_fst::parser::Analysis;
use serde::{Deserialize, Serialize};

/// The canonical set of relations v2.1 extracts. Small and explicit —
/// every addition here is an intentional architectural decision, not a
/// grab-bag. v2.2+ will extend this enum; keep each variant tied to a
/// matcher in [`patterns`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Predicate {
    /// Subject IS-A object (copula "X — Y" / "X Y болады" / "X Y-дан").
    /// The canonical ontological relation: "Abai is a poet".
    IsA,
    /// Subject lives / is located in object (locative + тұру).
    /// "Ол Алматыда тұрады" → (ол, LivesIn, Алматы).
    LivesIn,
    /// Subject has / owns object (genitive-possessive existence "X-тың
    /// Y-сы бар"). "Баланың кітабы бар" → (бала, Has, кітап). v2.2.
    Has,
    /// Subject goes / travels to object (dative-motion "X Y-ке барады").
    /// "Бала мектепке барады" → (бала, GoesTo, мектеп). v2.5.
    GoesTo,
    /// Subject is a physical / administrative part of object
    /// ("X Y-нің құрамында" / "X Y-нің бір бөлігі"). v2.6.
    /// "Алматы Қазақстанның құрамында" → (алматы, PartOf, қазақстан).
    PartOf,
    /// Subject is semantically related to object — shared type,
    /// co-occurrence in a sibling structure. v2.6. This predicate is
    /// typically **derived** by rule `R5_shared_is_a_target`
    /// (A IsA X ∧ B IsA X ⟹ RelatedTo(A, B)); extraction patterns for
    /// it will be added incrementally as lexical data supports them.
    RelatedTo,
}

impl Predicate {
    /// Stable string form for JSON round-trip + grepping.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::IsA => "is_a",
            Self::LivesIn => "lives_in",
            Self::Has => "has",
            Self::GoesTo => "goes_to",
            Self::PartOf => "part_of",
            Self::RelatedTo => "related_to",
        }
    }
}

/// **Kind of evidence** backing a fact. Not a probability — a
/// categorical tag describing HOW the fact was derived. Downstream
/// consumers can filter / rank by this without ever trusting a numeric
/// score. (The order below is NOT a ranking; don't sort by variant.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfidenceKind {
    /// Grammar-derived via FST features + syntactic pattern
    /// (the v2.1 baseline path — every `patterns.rs` match).
    Grammar,
    /// Exact quote from a curated high-purity source pack (Abai,
    /// classics, proverbs). Reserved for future use.
    CuratedQuote,
    /// A pattern observed ≥ N times across multiple source packs.
    /// Reserved for v2.3+.
    RepeatedPattern,
    /// A human marked this fact as correct during review. Reserved for
    /// future annotation workflows.
    HumanApproved,
    /// Derived by a rule from other facts (v2.3+ reasoner output).
    RuleInferred,
}

impl ConfidenceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Grammar => "grammar",
            Self::CuratedQuote => "curated_quote",
            Self::RepeatedPattern => "repeated_pattern",
            Self::HumanApproved => "human_approved",
            Self::RuleInferred => "rule_inferred",
        }
    }
}

/// A pointer to one slot (subject or object) inside a fact. Carries
/// the canonical root (after FST analysis), the surface form as it
/// appeared in the text, and the POS tag from the Lexicon so downstream
/// consumers can filter by noun / adjective / verb without re-running
/// the parser.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotRef {
    /// Surface form as it appeared in the sample (case-preserved).
    pub surface: String,
    /// Canonical root from the FST analysis.
    pub root: String,
    /// Part-of-speech tag from the Lexicon entry.
    pub pos: String,
}

/// Provenance — every fact traces back to exactly one sample in one
/// committed pack. Identical shape to `adam_retrieval::SampleRef` but
/// kept independent to avoid a reasoning→retrieval dep cycle.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FactSource {
    pub pack: String,
    pub sample_id: String,
}

/// One extracted fact. Serde round-trips cleanly to the committed
/// `data/retrieval/facts.json` artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fact {
    pub subject: SlotRef,
    pub predicate: Predicate,
    pub object: SlotRef,
    /// Which pattern in [`patterns`] matched. Human-readable; stable
    /// across v2.1 releases (breaking changes rename the matcher).
    pub pattern: String,
    pub source: FactSource,
    pub confidence: ConfidenceKind,
    /// Original sample text (or the sub-span the pattern matched on).
    /// Kept for audit / --trace; NOT used for matching downstream.
    pub raw_text: String,
}

/// Extract every fact that the v2.1 pattern set can find in `text`,
/// given its `parses` and the `lexicon` needed for POS tagging.
///
/// Returns facts in **pattern order** then **left-to-right match
/// order** — deterministic across runs. Empty vec if no pattern matches.
pub fn extract_facts(
    text: &str,
    parses: &[Analysis],
    lexicon: &LexiconV1,
    source: &FactSource,
) -> Vec<Fact> {
    let mut out = Vec::new();
    patterns::copula_is_a(text, parses, lexicon, source, &mut out);
    patterns::locative_lives_in(text, parses, lexicon, source, &mut out);
    patterns::possessive_has(text, parses, lexicon, source, &mut out);
    patterns::dative_goes_to(text, parses, lexicon, source, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn predicate_strings_are_stable() {
        assert_eq!(Predicate::IsA.as_str(), "is_a");
        assert_eq!(Predicate::LivesIn.as_str(), "lives_in");
        assert_eq!(Predicate::Has.as_str(), "has");
        assert_eq!(Predicate::GoesTo.as_str(), "goes_to");
        assert_eq!(Predicate::PartOf.as_str(), "part_of");
        assert_eq!(Predicate::RelatedTo.as_str(), "related_to");
    }

    #[test]
    fn confidence_strings_are_stable() {
        assert_eq!(ConfidenceKind::Grammar.as_str(), "grammar");
        assert_eq!(ConfidenceKind::RuleInferred.as_str(), "rule_inferred");
    }

    #[test]
    fn fact_round_trips_through_json() {
        let f = Fact {
            subject: SlotRef {
                surface: "Абай".into(),
                root: "абай".into(),
                pos: "noun".into(),
            },
            predicate: Predicate::IsA,
            object: SlotRef {
                surface: "ақын".into(),
                root: "ақын".into(),
                pos: "noun".into(),
            },
            pattern: "X — Y".into(),
            source: FactSource {
                pack: "abai_wikisource_pack.json".into(),
                sample_id: "abai_00042".into(),
            },
            confidence: ConfidenceKind::Grammar,
            raw_text: "Абай — ақын".into(),
        };
        let json = serde_json::to_string(&f).unwrap();
        let parsed: Fact = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, f);
    }
}
