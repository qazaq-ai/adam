//! adam-reasoning — Intelligent Lexical-Morphemic Retrieval & Reasoning (ILMRR).
//!
//! Stage: **v3.9.5** — fact extraction + lexical graph + forward-chaining
//! reasoner + human-authored World Core knowledge packs.
//!
//! This crate is the reasoning layer of the adam architecture. It takes
//! FST-parsed corpus samples, extracts **typed facts** with full
//! provenance, projects them into a node-edge graph, and runs a
//! deterministic forward-chaining reasoner over the result. In v3.9.0
//! it also started merging a second, orthogonal source of facts:
//! human-authored [`world_core`] entries.
//!
//! Capabilities shipped:
//!
//! - **Fact extraction** (v2.1 → v3.8.5): **11 pattern matchers** across
//!   [`patterns`] producing all 11 declared [`Predicate`] variants.
//!   Precision hardened in v3.8.5 / v3.9.0 with location / time-noun /
//!   demonstrative / fragment-root filters.
//! - **Lexical Graph** (v2.3+): pure projection of facts into a directed
//!   typed graph with per-edge provenance. See [`graph`].
//! - **Forward-chaining reasoner** (v2.4 → v3.9.5): 5 active rules (R1
//!   IsA-transitivity, R2 Has-inheritance, R3 Has-via-PartOf, R5
//!   shared-IsA → RelatedTo, R6 LivesIn-via-PartOf, R7 GoesTo-via-PartOf).
//!   See [`reasoner`].
//! - **World Core** (v3.9.0+): human-authored Kazakh knowledge packs
//!   merged into the committed fact set with `ConfidenceKind::HumanApproved`
//!   as the exclusive tier marker. See [`world_core`].
//! - **Iteration harness** (v3.1.0+): `--time-budget`, progress monitor,
//!   SIGINT → graceful commit. See [`harness`].
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
//! probability score. `Grammar` marks corpus-extracted facts,
//! `HumanApproved` marks World Core entries, `RuleInferred` marks
//! reasoner output. Consumers filter by kind, never by magnitude.

pub mod graph;
pub mod harness;
pub mod patterns;
pub mod reasoner;
pub mod world_core;

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
    /// Subject **causes** object. v3.5.0. Kazakh construction:
    /// `X — Y-нің себебі` ("X is the cause of Y") or
    /// `X Y-ға себеп болады` ("X becomes a cause for Y").
    /// Example: (су, Causes, өмір) from "су — өмірдің себебі".
    Causes,
    /// Subject happens **after** object in time. v3.5.0. Kazakh
    /// construction: `X Y-дан кейін` / `X Y-ден соң` ("X after Y").
    /// Example: (түс, After, таң) from "түс таңнан кейін келеді".
    After,
    /// Subject **has quantity** of object (numeric + counted noun).
    /// v3.5.0. Kazakh construction: `X Y-дің N Z-ы бар`
    /// ("X has N Y's"). Example: (бала, HasQuantity, кітап).
    /// The count is kept in the `raw_text` field; this predicate
    /// records the relationship, not the magnitude.
    HasQuantity,
    /// Subject **does** (verb) object. v3.5.0 agent-verb pattern.
    /// Kazakh construction: `X Y-ні Z-лайды` ("X does Z to Y"). The
    /// subject is the agent, the object is the patient; the verb is
    /// encoded in the `pattern` field of the fact.
    /// Example: (бала, DoesTo, доп) from "бала допты тебеді".
    DoesTo,
    /// Subject is a member of domain (object). v3.5.0. Kazakh
    /// construction: `X — Y саласы` ("X is a field of Y"),
    /// `X Y ғылымына жатады` ("X belongs to Y science").
    /// Example: (алгебра, InDomain, математика).
    InDomain,
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
            Self::Causes => "causes",
            Self::After => "after",
            Self::HasQuantity => "has_quantity",
            Self::DoesTo => "does_to",
            Self::InDomain => "in_domain",
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
    // v2.x baseline matchers.
    patterns::copula_is_a(text, parses, lexicon, source, &mut out);
    patterns::locative_lives_in(text, parses, lexicon, source, &mut out);
    patterns::possessive_has(text, parses, lexicon, source, &mut out);
    patterns::dative_goes_to(text, parses, lexicon, source, &mut out);
    // v3.5.0 breadth expansion — 6 new matchers bring predicate
    // coverage from 4 → 9 distinct predicates (adds Causes, After,
    // HasQuantity, DoesTo, InDomain; `nominal_conjunction` extracts
    // into RelatedTo — a second extraction path for the rule-derived
    // predicate, grounded now in explicit syntactic co-predication).
    patterns::copula_causes(text, parses, lexicon, source, &mut out);
    patterns::temporal_after(text, parses, lexicon, source, &mut out);
    patterns::quantity_count(text, parses, lexicon, source, &mut out);
    patterns::agent_verb(text, parses, lexicon, source, &mut out);
    patterns::nominal_conjunction(text, parses, lexicon, source, &mut out);
    patterns::domain_membership(text, parses, lexicon, source, &mut out);
    // v3.5.5: structural partitive — first PartOf extractor. Feeds
    // the R3_has_inheritance_via_part_of rule (Has + PartOf → Has).
    patterns::structural_part_of(text, parses, lexicon, source, &mut out);
    // v3.9.0 — central hygiene gate. The FST tokenizer sometimes
    // splits compound tokens like `2021-жылғы` into dash-prefixed
    // fragments (`-жылғы`, `-ға`, `-қа`, `-дүниежүзілік`, `-ғасыр`)
    // that leak into pattern matchers as subject / object roots.
    // Codex external review of v3.8.5 flagged 87 such facts on the
    // committed runtime (`-дүниежүзілік`=20, `-ға`=8, `-жыл`=6, etc).
    // Every root starting with `-` is guaranteed to be a suffix-
    // fragment parse and can never represent a real entity — reject
    // unconditionally at the pipeline boundary.
    out.retain(|f| !is_fragment_root(&f.subject.root) && !is_fragment_root(&f.object.root));
    out
}

/// v3.9.0 — filter for dash-prefixed fragment roots. Returns `true`
/// for any root that begins with `-` (suffix fragment, e.g. `-ға`,
/// `-жылғы`, `-дүниежүзілік`). Also filters roots that are just a
/// single dash character. Empty strings are treated as fragments.
fn is_fragment_root(root: &str) -> bool {
    root.is_empty() || root.starts_with('-')
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
        // v3.5.0 additions.
        assert_eq!(Predicate::Causes.as_str(), "causes");
        assert_eq!(Predicate::After.as_str(), "after");
        assert_eq!(Predicate::HasQuantity.as_str(), "has_quantity");
        assert_eq!(Predicate::DoesTo.as_str(), "does_to");
        assert_eq!(Predicate::InDomain.as_str(), "in_domain");
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

    // ----------------------- v3.9.0 hygiene gate -------------------------

    #[test]
    fn is_fragment_root_rejects_dash_prefixed() {
        // Codex-flagged noise from the committed v3.8.5 runtime.
        assert!(is_fragment_root("-ға"));
        assert!(is_fragment_root("-жыл"));
        assert!(is_fragment_root("-қа"));
        assert!(is_fragment_root("-ғасыр"));
        assert!(is_fragment_root("-дүниежүзілік"));
        assert!(is_fragment_root("-тармағын"));
        // Empty also refused.
        assert!(is_fragment_root(""));
        // Legitimate content nouns pass.
        assert!(!is_fragment_root("еңбек"));
        assert!(!is_fragment_root("жер"));
        assert!(!is_fragment_root("қазақстан"));
        // Internal dashes are fine (compound content nouns).
        assert!(!is_fragment_root("сондай-ақ"));
        assert!(!is_fragment_root("нұр-сұлтан"));
    }
}
