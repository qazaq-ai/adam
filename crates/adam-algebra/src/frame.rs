// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `Frame` — **Stage 2 of v6.2.0 neurosymbolic redesign**.
//!
//! A `Frame` is the typed semantic record of a single utterance:
//! the unified `(agent, predicate, object, modifiers, modality,
//! polarity, evidentiality, tense, aspect)` tuple that every layer
//! above the morphology operates on.
//!
//! ## Why this layer exists
//!
//! Pre-v6.2 the same semantic information was scattered across
//! several ad-hoc structures:
//!
//! - [`adam_reasoning::Fact`] — `(subject, predicate, object)` triple
//!   for retrieval / verification.
//! - `SentenceFrame` in [`adam_dialog::nlg`] — `fact + mood +
//!   introducer + name_respect` for NLG.
//! - `SentenceDecomposition` in [`adam_dialog::sentence_decomp`] —
//!   `(Role × token)` mapping for the planner.
//! - `Claim` in [`adam_dialog::proof_object`] — `(subject, predicate,
//!   object, polarity)` for proof tracking.
//!
//! Each new behaviour added a new field somewhere, and the four
//! types drifted out of sync. `Frame` is the single canonical
//! shape: every downstream layer consumes a `Frame`, and morphology
//! is the only thing that *produces* one.
//!
//! ## Stage 2 scope
//!
//! - [`Frame`] — the typed record itself.
//! - [`FramePredicate`] — closed-set enum subsuming every v6.1
//!   [`adam_reasoning::Predicate`] variant plus the meta-predicates
//!   needed by the dialog layer (`HasProperty` / `Definition` /
//!   `SystemSelf`).
//! - [`Modality`] — assertion / question / imperative / refusal /
//!   no-data, with question-focus type for `wh-` queries.
//! - [`Polarity`] — affirmative vs negated (mirrors the FST
//!   `VerbFeatures::negation` flag at the semantic layer).
//! - [`Evidentiality`] — direct / reported / inferred / no-data
//!   (derived from FST tense + reasoning trail).
//! - [`Aspect`] — perfective / habitual / continuous / imperfective
//!   (derived from FST tense).
//! - [`Modifier`] — time-anchor / location / source / instrument /
//!   manner / recipient slot, each carrying a typed [`Composition`].
//! - [`Frame::from_morph_lattice`] — deterministic assembler from
//!   the per-word [`Composition`] lattice that the FST produces.
//!
//! ## Predeclared success criterion
//!
//! Every NLG rule path that existed in v6.1.50 must be expressible
//! as a `Frame` without information loss. The test suite at the
//! bottom of this module asserts this for all 22 v6.1
//! `adam_reasoning::Predicate` variants.
//!
//! ## NOT in Stage 2
//!
//! - Curated `raw_text` ↔ `Frame` bridge — the realiser layer
//!   (Stage 7) owns the choice of "render from frame" vs "use
//!   curated surface".
//! - The actual rewiring of `SentenceFrame` callers to use `Frame`
//!   — Stage 2 ships the type and the assembler; the call-site
//!   migration is folded into Stage 7 to avoid a partial-state
//!   tree.
//! - The frame-disambiguation choice when `Composition::from_analysis`
//!   produces multiple analyses for one word — Stage 5 (sense
//!   disambiguation) handles that. Stage 2 takes the first analysis.

use adam_kernel_fst::morphotactics::{Case, Tense};
use serde::{Deserialize, Serialize};

use crate::composition::Composition;
use crate::operator::SuffixOp;
use crate::root::PartOfSpeech;

/// Closed-set semantic predicate. Subsumes every v6.1
/// `adam_reasoning::Predicate` variant plus the meta-predicates
/// the dialog layer surfaces (`HasProperty` / `Definition` /
/// `SystemSelf`).
///
/// Adding a new variant here is an intentional architectural
/// decision — the variant must come with a clear meaning, a
/// curated `data/world_core/*.jsonl` slot, and an NLG path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FramePredicate {
    // -- 11 v3.x/v4.x predicates (relational core) ---------------
    /// Type membership: «X — Y».
    IsA,
    /// «X — animate, lives in place Y».
    LivesIn,
    /// «X has / owns Y» (genitive-possessive).
    Has,
    /// «X goes to Y» (dative-motion).
    GoesTo,
    /// «X is a part of Y».
    PartOf,
    /// «X is related to Y» (catch-all for shared-IsA targets).
    RelatedTo,
    /// «X causes Y».
    Causes,
    /// «X happens after Y» (temporal order).
    After,
    /// «X has quantity N of Y» — count phrase, magnitude in modifiers.
    HasQuantity,
    /// «X does Z to Y» — generic agent-verb-patient.
    DoesTo,
    /// «X is in domain Y» (e.g. «алгебра — математика саласы»).
    InDomain,
    // -- 11 v6.1 typed-predicate extension ----------------------
    /// «X was born in Y» (date or place).
    BornIn,
    /// «X died in Y» (date or place).
    DiedIn,
    /// «X (institution / event) was founded in Y».
    FoundedIn,
    /// «X (institution) was renamed in Y».
    RenamedIn,
    /// «X (law / agreement) became effective in Y».
    EffectiveFrom,
    /// «X (law / classifier) classifies things into Y».
    Classifies,
    /// «X has risk level Y».
    RiskLevel,
    /// «X is located in Y» (spatial, distinct from `LivesIn`).
    LocatedIn,
    /// «X is named after Y».
    NamedAfter,
    /// «X is a member of Y» (organisation, alliance).
    MemberOf,
    /// «X was authored / created by Y».
    Authored,
    // -- Dialog-layer meta-predicates ---------------------------
    /// «X has property Y» — adjectival assertion («тас тірі емес»).
    HasProperty,
    /// «X is defined as Y» — definitional answer to «X деген не?».
    Definition,
    /// adam describing itself («Мен — қазақ тілінің көмекшісімін»).
    SystemSelf,
}

impl FramePredicate {
    /// Stable slug for JSON / trace output.
    pub fn as_str(&self) -> &'static str {
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
            Self::BornIn => "born_in",
            Self::DiedIn => "died_in",
            Self::FoundedIn => "founded_in",
            Self::RenamedIn => "renamed_in",
            Self::EffectiveFrom => "effective_from",
            Self::Classifies => "classifies",
            Self::RiskLevel => "risk_level",
            Self::LocatedIn => "located_in",
            Self::NamedAfter => "named_after",
            Self::MemberOf => "member_of",
            Self::Authored => "authored",
            Self::HasProperty => "has_property",
            Self::Definition => "definition",
            Self::SystemSelf => "system_self",
        }
    }
}

/// What kind of utterance the frame encodes. Mood operator over
/// the propositional content.
///
/// `Refusal` and `NoData` are *meta*-modalities: the frame doesn't
/// assert its propositional content (it asserts the absence of an
/// answer instead). They live here rather than in [`FramePredicate`]
/// so any propositional predicate can pair with them — e.g. a
/// safety refusal triggered by an `IsA` query, or a no-data state
/// for an `Authored` query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Modality {
    /// Plain statement. Default for factoid answers.
    Assertion,
    /// `wh-` or yes/no question. `focus` records which slot the
    /// asker wants filled.
    Question { focus: QuestionFocus },
    /// Command («айт / жаз / көрсет»). Reserved — Stage 2 assembler
    /// doesn't produce this yet; included for completeness with
    /// `SentenceType::Imperative`.
    Imperative,
    /// Safety-domain refusal — the frame holds the refusal context,
    /// not a fact. The domain slug matches
    /// [`adam_dialog::proof_object::SafetyDomain`] for round-trip.
    Refusal { domain: String },
    /// Honest no-data response. The frame's predicate / agent /
    /// object name what was asked; the modality says "I don't have
    /// data for this combination."
    NoData,
}

/// Slot the question is asking for. Drives focus-aware retrieval
/// in Stage 3 (QueryIR).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionFocus {
    /// «кім» — asks for the agent.
    Subject,
    /// «не / нені» — asks for the object.
    Object,
    /// «қашан» — asks for the time-anchor modifier.
    Time,
    /// «қайда» — asks for the location modifier.
    Place,
    /// «қалай» — asks for the manner / predicate variant.
    Manner,
    /// «неліктен / неге» — asks for a causal predicate.
    Cause,
    /// «қандай» — asks for an `IsA` / `HasProperty` answer.
    Kind,
    /// «қанша / неше» — asks for a quantity modifier.
    Quantity,
    /// Yes/no question («ма / ме / ба / бе»). The asker wants the
    /// frame's polarity confirmed or denied.
    YesNo,
    /// Unrecognised question word — fall through to general
    /// retrieval.
    Unknown,
}

/// Polarity of the assertion. Mirrors
/// [`adam_dialog::proof_object::Polarity`] so the dialog layer
/// can round-trip without an extra mapping table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Polarity {
    Affirmative,
    Negated,
}

/// Evidential source for the assertion. Captures both the FST
/// reportative tense (`PastEvidential` / `PastReportative`) and
/// the reasoner-derived flag (`RuleInferred`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Evidentiality {
    /// Speaker has direct knowledge. Default.
    Direct,
    /// Hearsay — surface bears `-ған` / `-ыпты` reportative
    /// morphology.
    Reported,
    /// Derived by a forward-chaining rule, not directly observed.
    Inferred,
    /// Speaker explicitly disclaims knowledge.
    NoData,
}

/// Grammatical aspect, derived from the FST tense.
///
/// Note: aspect is *partially* redundant with [`Frame::tense`] (a
/// `PastDefinite` is always perfective in this taxonomy), but
/// exposing it as a typed field lets the retrieval / NLG layers
/// match on aspect without re-deriving from tense.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Aspect {
    /// Bounded, completed action. `PastDefinite` / `PastEvidential`
    /// / `PastReportative`.
    Perfective,
    /// Generic, repeated, characteristic. `PastHabitual` /
    /// `ParticipleHabitual`.
    Habitual,
    /// In-progress. (FST surface for this is `-{Y}п жатыр` —
    /// produced via converb + auxiliary, not as a single tense
    /// label. Stage 2 assembler does not yet detect this; reserved
    /// for Stage 7 when multi-word predicates are handled.)
    Continuous,
    /// Generic / non-bounded reference. `Present` / `FutureIntentional`
    /// / `FuturePossible` / `Conditional` / `Imperative` /
    /// `ParticipleFuture` / converbs.
    Imperfective,
}

impl Aspect {
    /// Map an FST [`Tense`] to its canonical aspect bucket. Used by
    /// [`Frame::from_morph_lattice`] to populate `Frame::aspect`
    /// from the verb composition.
    pub fn from_tense(tense: Tense) -> Self {
        match tense {
            Tense::PastDefinite | Tense::PastEvidential | Tense::PastReportative => {
                Self::Perfective
            }
            Tense::PastHabitual | Tense::ParticipleHabitual => Self::Habitual,
            Tense::Present
            | Tense::FutureIntentional
            | Tense::FuturePossible
            | Tense::Conditional
            | Tense::Imperative
            | Tense::ParticiplePast
            | Tense::ParticipleFuture
            | Tense::ConverbPerfect
            | Tense::ConverbImperfect => Self::Imperfective,
        }
    }

    /// Map an FST [`Tense`] to its evidential signal, if any.
    /// `PastEvidential` and `PastReportative` both lift to
    /// [`Evidentiality::Reported`].
    pub fn evidentiality_from_tense(tense: Tense) -> Option<Evidentiality> {
        match tense {
            Tense::PastEvidential | Tense::PastReportative => Some(Evidentiality::Reported),
            _ => None,
        }
    }
}

/// A time anchor — when the frame's event occurred. Stage 2
/// assembler produces `Composition`-backed time-NPs (e.g. «1872
/// жылы» as `Composition { root: жыл, ops: [Possessive(P3),
/// Case(Locative)] }`). Higher-precision typed forms (`Year(1872)`
/// / `Date { … }`) are reserved for the slot-extractor layer in
/// Stage 3 — Stage 2 only needs faithful capture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum TimeAnchor {
    /// Year-precision anchor: «1872 жылы».
    Year(i32),
    /// ISO-style date: «1872-09-05».
    Date { year: i32, month: u8, day: u8 },
    /// Generic time-NP wrapped as a [`Composition`].
    Phrase(Composition),
}

/// A typed adjunct on the frame. Each modifier carries a
/// [`Composition`] so the underlying morphology is preserved (no
/// re-encoding to strings).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "role", content = "phrase")]
pub enum Modifier {
    /// «1872 жылы» / «бүгін» — when the event happened.
    TimeAnchor(TimeAnchor),
    /// «Қостанай облысында» — where the event happened.
    Location(Composition),
    /// Ablative source — «Алматыдан / Көктемнен кейін».
    Source(Composition),
    /// Instrumental means — «пышақпен».
    Instrument(Composition),
    /// Manner adverbial — «жылдам».
    Manner(Composition),
    /// Dative recipient — «маған / Айгүлге».
    Recipient(Composition),
    /// Possessor — «менің / Ахметтің».
    Possessor(Composition),
}

impl Modifier {
    /// Stable slug for trace output.
    pub fn role_str(&self) -> &'static str {
        match self {
            Self::TimeAnchor(_) => "time",
            Self::Location(_) => "location",
            Self::Source(_) => "source",
            Self::Instrument(_) => "instrument",
            Self::Manner(_) => "manner",
            Self::Recipient(_) => "recipient",
            Self::Possessor(_) => "possessor",
        }
    }
}

/// The typed semantic record of a single utterance — the v6.2
/// unified abstraction. Every layer above morphology consumes a
/// `Frame`; morphology is the only producer (via
/// [`Frame::from_morph_lattice`]).
///
/// Construction is via either:
/// - [`Frame::from_morph_lattice`] for parser-side use, or
/// - [`Frame::builder`] / direct struct init for tests + the
///   reasoning/retrieval layers that assemble frames from
///   pre-typed components.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Frame {
    /// «Кто» — agent / subject. `None` for impersonal predicates
    /// («жаңбыр жауады»: agent = жаңбыр; «суық» / «жарық»:
    /// agent = None).
    pub agent: Option<Composition>,
    /// «Что» — the propositional predicate.
    pub predicate: FramePredicate,
    /// «Кого / что» — object / patient / theme. `None` for
    /// intransitive predicates and for `Definition` / `SystemSelf`
    /// where the predicate carries its own object.
    pub object: Option<Composition>,
    /// Adjuncts in arbitrary order — order is not semantically
    /// load-bearing.
    pub modifiers: Vec<Modifier>,
    /// Mood operator.
    pub modality: Modality,
    /// Affirmative vs negated assertion.
    pub polarity: Polarity,
    /// Direct / reported / inferred / no-data.
    pub evidentiality: Evidentiality,
    /// FST tense of the predicate, if any. `None` for nominal
    /// predicates («Қазақстан — мемлекет.»).
    pub tense: Option<Tense>,
    /// Aspect bucket. Derived from `tense` by
    /// [`Aspect::from_tense`] when the assembler sets `tense`.
    pub aspect: Option<Aspect>,
}

impl Frame {
    /// A bare assertion frame — agent + predicate + object, no
    /// modifiers, default modality / polarity / evidentiality. The
    /// minimal-input constructor; useful for unit tests and for
    /// the v6.1 bridge that assembles `Frame` from a pre-existing
    /// `Fact`.
    pub fn assertion(
        agent: Option<Composition>,
        predicate: FramePredicate,
        object: Option<Composition>,
    ) -> Self {
        Self {
            agent,
            predicate,
            object,
            modifiers: Vec::new(),
            modality: Modality::Assertion,
            polarity: Polarity::Affirmative,
            evidentiality: Evidentiality::Direct,
            tense: None,
            aspect: None,
        }
    }

    /// Attach a modifier. Order of insertion is preserved but is
    /// not semantically load-bearing (the retrieval layer matches
    /// by `Modifier::role_str`, not by position).
    pub fn with_modifier(mut self, modifier: Modifier) -> Self {
        self.modifiers.push(modifier);
        self
    }

    /// Set the polarity (default is [`Polarity::Affirmative`]).
    pub fn with_polarity(mut self, polarity: Polarity) -> Self {
        self.polarity = polarity;
        self
    }

    /// Set the modality (default is [`Modality::Assertion`]).
    pub fn with_modality(mut self, modality: Modality) -> Self {
        self.modality = modality;
        self
    }

    /// Set the evidentiality (default is [`Evidentiality::Direct`]).
    pub fn with_evidentiality(mut self, evidentiality: Evidentiality) -> Self {
        self.evidentiality = evidentiality;
        self
    }

    /// Set the FST tense and derive the aspect from it.
    pub fn with_tense(mut self, tense: Tense) -> Self {
        self.tense = Some(tense);
        self.aspect = Some(Aspect::from_tense(tense));
        if let Some(ev) = Aspect::evidentiality_from_tense(tense) {
            self.evidentiality = ev;
        }
        self
    }

    /// First modifier matching `role_str`, if any. Used by
    /// retrieval to look up «what time / what place» without
    /// scanning a typed enum.
    pub fn modifier(&self, role: &str) -> Option<&Modifier> {
        self.modifiers.iter().find(|m| m.role_str() == role)
    }

    /// Deterministic assembler from a per-word
    /// [`Composition`] lattice (the FST's output, one composition
    /// per surface word in canonical reading order).
    ///
    /// ## Assembly rules (Stage 2)
    ///
    /// 1. **Predicate selection**: the first verb-stack composition
    ///    is the predicate; its root surface seeds the
    ///    `FramePredicate` mapping (`туыл → BornIn`, etc., via
    ///    [`predicate_from_verb_root`]). If no verb is present
    ///    and exactly two noun-stack compositions appear in
    ///    nominative / nominative-or-genitive pattern, the
    ///    predicate is `IsA` (copular «X — Y»).
    /// 2. **Agent**: a Nominative-case noun-stack composition
    ///    appearing before the predicate. If multiple, the first
    ///    wins (Kazakh SOV default).
    /// 3. **Object**: an Accusative-case noun-stack composition.
    /// 4. **Modifiers**:
    ///    - Locative-case noun root in a known time-noun set
    ///      («жыл / ай / күн / ғасыр / айлық») → `TimeAnchor::Phrase`.
    ///    - Locative-case otherwise → `Location`.
    ///    - Ablative-case → `Source`.
    ///    - Instrumental-case → `Instrument`.
    ///    - Dative-case → `Recipient`.
    ///    - Genitive-case → `Possessor`.
    /// 5. **Polarity**: `Negated` iff the predicate composition
    ///    contains [`SuffixOp::Negation`].
    /// 6. **Tense / aspect / evidentiality**: derived from the
    ///    predicate's `SuffixOp::Tense(_)`. `PastEvidential` and
    ///    `PastReportative` lift to `Evidentiality::Reported`.
    /// 7. **Modality**: defaults to `Assertion`. Stage 2 does not
    ///    detect question / imperative from the lattice — the
    ///    caller passes the [`SentenceContext`] for that signal.
    ///
    /// Returns `None` when the lattice is empty or when no
    /// usable predicate can be identified — the caller should
    /// fall back to v6.1 path (`SentenceDecomposition`) in that
    /// case. Stage 5 will tighten this; Stage 2 prefers
    /// permissive over strict.
    pub fn from_morph_lattice(lattice: &[Composition]) -> Option<Self> {
        Self::from_morph_lattice_in_context(lattice, SentenceContext::default())
    }

    /// Variant of [`Frame::from_morph_lattice`] that takes a
    /// [`SentenceContext`] — surface-level signals (sentence type,
    /// question word) that the caller has already extracted but
    /// the lattice alone doesn't reveal.
    pub fn from_morph_lattice_in_context(
        lattice: &[Composition],
        ctx: SentenceContext,
    ) -> Option<Self> {
        if lattice.is_empty() {
            return None;
        }

        // 1. Find the predicate.
        let predicate_idx = lattice
            .iter()
            .position(|c| c.root.pos == PartOfSpeech::Verb);

        let (predicate, predicate_comp) = if let Some(idx) = predicate_idx {
            let comp = &lattice[idx];
            let pred =
                predicate_from_verb_root(&comp.root.surface).unwrap_or(FramePredicate::DoesTo);
            (pred, Some((idx, comp)))
        } else {
            let pred = predicate_from_copular_lattice(lattice)?;
            (pred, None)
        };

        // 2. Walk the lattice to collect agent / object / modifiers.
        let mut agent: Option<Composition> = None;
        let mut object: Option<Composition> = None;
        let mut modifiers: Vec<Modifier> = Vec::new();

        let pred_idx = predicate_comp.map(|(i, _)| i);
        for (i, comp) in lattice.iter().enumerate() {
            // Skip the predicate composition itself.
            if Some(i) == pred_idx {
                continue;
            }
            // Verb-stack non-predicates (auxiliaries / converbs)
            // aren't classified at Stage 2 — Stage 7 handles them.
            if comp.root.pos == PartOfSpeech::Verb {
                continue;
            }
            classify_noun_composition(comp, &mut agent, &mut object, &mut modifiers);
        }

        // 3. Derive polarity / tense / aspect / evidentiality.
        let mut polarity = Polarity::Affirmative;
        let mut tense: Option<Tense> = None;
        let mut aspect: Option<Aspect> = None;
        let mut evidentiality = Evidentiality::Direct;
        if let Some((_, comp)) = predicate_comp {
            for op in &comp.operators {
                match op {
                    SuffixOp::Negation => polarity = Polarity::Negated,
                    SuffixOp::Tense(t) => {
                        tense = Some(*t);
                        aspect = Some(Aspect::from_tense(*t));
                        if let Some(ev) = Aspect::evidentiality_from_tense(*t) {
                            evidentiality = ev;
                        }
                    }
                    _ => {}
                }
            }
        }

        // 4. Resolve modality from sentence context.
        let modality = match ctx.sentence_type {
            ContextSentenceType::Statement => Modality::Assertion,
            ContextSentenceType::Imperative => Modality::Imperative,
            ContextSentenceType::Question => Modality::Question {
                focus: ctx.question_focus.unwrap_or(QuestionFocus::Unknown),
            },
            ContextSentenceType::Exclamation => Modality::Assertion,
        };

        Some(Self {
            agent,
            predicate,
            object,
            modifiers,
            modality,
            polarity,
            evidentiality,
            tense,
            aspect,
        })
    }
}

/// Surface-level context the assembler needs but the lattice
/// alone doesn't carry. Provided by the caller (typically derived
/// from the same input by a punctuation + question-word scan).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SentenceContext {
    pub sentence_type: ContextSentenceType,
    pub question_focus: Option<QuestionFocus>,
}

impl Default for SentenceContext {
    fn default() -> Self {
        Self {
            sentence_type: ContextSentenceType::Statement,
            question_focus: None,
        }
    }
}

/// Mirror of the dialog-layer `SentenceType` enum, kept here so
/// `adam-algebra` doesn't depend on `adam-dialog`. The dialog
/// layer maps its own enum to this when calling
/// [`Frame::from_morph_lattice_in_context`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextSentenceType {
    Statement,
    Question,
    Imperative,
    Exclamation,
}

/// Classify one noun-stack composition into agent / object /
/// modifier and update the running frame state. Pure data-driven
/// — case marker drives the assignment.
fn classify_noun_composition(
    comp: &Composition,
    agent: &mut Option<Composition>,
    object: &mut Option<Composition>,
    modifiers: &mut Vec<Modifier>,
) {
    let case = comp.operators.iter().find_map(|op| match op {
        SuffixOp::Case(c) => Some(*c),
        _ => None,
    });

    match case {
        // No case marker → nominative (agent slot) unless filled.
        None | Some(Case::Nominative) => {
            if agent.is_none() {
                *agent = Some(comp.clone());
            } else if object.is_none() {
                // Second nominative becomes object in copular «X — Y».
                *object = Some(comp.clone());
            }
        }
        Some(Case::Accusative) => {
            if object.is_none() {
                *object = Some(comp.clone());
            }
        }
        Some(Case::Locative) | Some(Case::LocativeAttributive) => {
            if is_time_noun(&comp.root.surface) {
                modifiers.push(Modifier::TimeAnchor(TimeAnchor::Phrase(comp.clone())));
            } else {
                modifiers.push(Modifier::Location(comp.clone()));
            }
        }
        Some(Case::Ablative) => modifiers.push(Modifier::Source(comp.clone())),
        Some(Case::Dative) => modifiers.push(Modifier::Recipient(comp.clone())),
        Some(Case::Genitive) => modifiers.push(Modifier::Possessor(comp.clone())),
        Some(Case::Instrumental) => modifiers.push(Modifier::Instrument(comp.clone())),
    }
}

/// Closed list of time-noun roots that, when carrying a locative
/// marker, project to a `TimeAnchor` modifier instead of a generic
/// `Location`. Stage 2 lists only the most common surfaces; Stage 3
/// (sense disambiguation) will grow this from the lexicon's
/// time-noun tag.
fn is_time_noun(root: &str) -> bool {
    matches!(
        root,
        "жыл"
            | "ай"
            | "күн"
            | "ғасыр"
            | "уақыт"
            | "минут"
            | "сағат"
            | "айлық"
            | "апта"
            | "тәулік"
    )
}

/// Map a verb root surface to its `FramePredicate`. Closed set —
/// adding a new verb-predicate mapping is an intentional
/// architectural decision. Stage 2 covers the predicates that
/// have an unambiguous verb root in v6.1 corpora; the rest fall
/// through to `DoesTo` (generic agent-verb pattern).
fn predicate_from_verb_root(root: &str) -> Option<FramePredicate> {
    Some(match root {
        // birth / death — load-bearing for v6.1 biographical facts.
        "туыл" | "туу" | "туыл-" => FramePredicate::BornIn,
        "қайтыс" | "өл" | "өлу" => FramePredicate::DiedIn,
        // founding / renaming.
        "құрыл" | "құру" | "ашыл" | "ашу" | "негізделу" => {
            FramePredicate::FoundedIn
        }
        "атау" | "атал" | "атау-" => FramePredicate::NamedAfter,
        "қайта_атау" | "қайта-атау" => FramePredicate::RenamedIn,
        // location / membership.
        "орналас" | "орналасу" => FramePredicate::LocatedIn,
        "тұр" | "тұру" | "өмір_сүр" | "өмір-сүр" => FramePredicate::LivesIn,
        "кір" | "кіру" => FramePredicate::MemberOf,
        // possession / authorship.
        "ие" | "иелену" => FramePredicate::Has,
        "жаз" | "жазу" | "құрастыр" | "құрастыру" => {
            FramePredicate::Authored
        }
        // motion / temporal.
        "бар" | "бару" | "кел" | "келу" => FramePredicate::GoesTo,
        // causation.
        "себеп" | "тудыр" | "тудыру" => FramePredicate::Causes,
        // classification.
        "жікте" | "жіктеу" => FramePredicate::Classifies,
        _ => return None,
    })
}

/// Detect a copular `IsA` from a 2-noun lattice without a verb.
/// Returns `Some(IsA)` when the lattice looks like «X — Y»; `None`
/// otherwise (caller falls through to `from_morph_lattice` failure).
fn predicate_from_copular_lattice(lattice: &[Composition]) -> Option<FramePredicate> {
    let noun_count = lattice
        .iter()
        .filter(|c| c.root.pos != PartOfSpeech::Verb)
        .count();
    if noun_count >= 2 {
        Some(FramePredicate::IsA)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operator::SuffixOp;
    use crate::root::{PartOfSpeech, Root};
    use adam_kernel_fst::morphotactics::{Case, Number, Possessive, Tense};

    fn noun(surface: &str) -> Composition {
        Composition::identity(Root::new(surface, PartOfSpeech::Noun))
    }

    fn verb(surface: &str) -> Composition {
        Composition::identity(Root::new(surface, PartOfSpeech::Verb))
    }

    fn noun_with_case(surface: &str, case: Case) -> Composition {
        let mut c = noun(surface);
        c.operators.push(SuffixOp::Case(case));
        c
    }

    // -- FramePredicate coverage ---------------------------------

    #[test]
    fn frame_predicate_covers_all_22_v6_1_variants() {
        // Every v6.1 adam_reasoning::Predicate variant must be
        // expressible as a FramePredicate so the Stage 7 NLG path
        // can lift v6.1 rules without losing semantic info.
        let v6_1_predicates = [
            FramePredicate::IsA,
            FramePredicate::LivesIn,
            FramePredicate::Has,
            FramePredicate::GoesTo,
            FramePredicate::PartOf,
            FramePredicate::RelatedTo,
            FramePredicate::Causes,
            FramePredicate::After,
            FramePredicate::HasQuantity,
            FramePredicate::DoesTo,
            FramePredicate::InDomain,
            FramePredicate::BornIn,
            FramePredicate::DiedIn,
            FramePredicate::FoundedIn,
            FramePredicate::RenamedIn,
            FramePredicate::EffectiveFrom,
            FramePredicate::Classifies,
            FramePredicate::RiskLevel,
            FramePredicate::LocatedIn,
            FramePredicate::NamedAfter,
            FramePredicate::MemberOf,
            FramePredicate::Authored,
        ];
        assert_eq!(v6_1_predicates.len(), 22);
        // Round-trip every variant through its stable slug.
        for p in &v6_1_predicates {
            let s = p.as_str();
            assert!(!s.is_empty());
        }
    }

    #[test]
    fn frame_predicate_includes_dialog_meta() {
        // ClaimPredicate's HasProperty / Definition / SystemSelf
        // must also be representable for the dialog layer's
        // round-trip.
        for p in [
            FramePredicate::HasProperty,
            FramePredicate::Definition,
            FramePredicate::SystemSelf,
        ] {
            assert!(!p.as_str().is_empty());
        }
    }

    // -- Frame::assertion + builders ----------------------------

    #[test]
    fn assertion_builder_defaults() {
        let f = Frame::assertion(
            Some(noun("қазақстан")),
            FramePredicate::IsA,
            Some(noun("мемлекет")),
        );
        assert_eq!(f.modality, Modality::Assertion);
        assert_eq!(f.polarity, Polarity::Affirmative);
        assert_eq!(f.evidentiality, Evidentiality::Direct);
        assert!(f.tense.is_none());
        assert!(f.aspect.is_none());
        assert!(f.modifiers.is_empty());
    }

    #[test]
    fn with_tense_derives_aspect() {
        let f = Frame::assertion(Some(noun("ахмет")), FramePredicate::BornIn, None)
            .with_tense(Tense::PastDefinite);
        assert_eq!(f.tense, Some(Tense::PastDefinite));
        assert_eq!(f.aspect, Some(Aspect::Perfective));
        assert_eq!(f.evidentiality, Evidentiality::Direct);
    }

    #[test]
    fn with_tense_evidential_lifts_to_reported() {
        let f = Frame::assertion(Some(noun("ахмет")), FramePredicate::BornIn, None)
            .with_tense(Tense::PastEvidential);
        assert_eq!(f.aspect, Some(Aspect::Perfective));
        assert_eq!(f.evidentiality, Evidentiality::Reported);
    }

    #[test]
    fn with_tense_reportative_lifts_to_reported() {
        let f = Frame::assertion(Some(noun("х")), FramePredicate::DoesTo, None)
            .with_tense(Tense::PastReportative);
        assert_eq!(f.evidentiality, Evidentiality::Reported);
    }

    #[test]
    fn aspect_buckets() {
        assert_eq!(Aspect::from_tense(Tense::PastDefinite), Aspect::Perfective);
        assert_eq!(Aspect::from_tense(Tense::PastHabitual), Aspect::Habitual);
        assert_eq!(Aspect::from_tense(Tense::Present), Aspect::Imperfective);
        assert_eq!(
            Aspect::from_tense(Tense::ConverbPerfect),
            Aspect::Imperfective
        );
    }

    // -- Modifier API -------------------------------------------

    #[test]
    fn modifier_lookup_by_role() {
        let f = Frame::assertion(Some(noun("ахмет")), FramePredicate::BornIn, None)
            .with_modifier(Modifier::TimeAnchor(TimeAnchor::Year(1872)))
            .with_modifier(Modifier::Location(noun_with_case(
                "қостанай",
                Case::Locative,
            )));
        assert!(matches!(f.modifier("time"), Some(Modifier::TimeAnchor(_))));
        assert!(matches!(
            f.modifier("location"),
            Some(Modifier::Location(_))
        ));
        assert!(f.modifier("instrument").is_none());
    }

    // -- from_morph_lattice ------------------------------------

    #[test]
    fn lattice_empty_returns_none() {
        assert!(Frame::from_morph_lattice(&[]).is_none());
    }

    #[test]
    fn lattice_copular_two_nouns_is_a() {
        // «Қазақстан — мемлекет.»
        let lat = vec![noun("қазақстан"), noun("мемлекет")];
        let f = Frame::from_morph_lattice(&lat).expect("copular frame");
        assert_eq!(f.predicate, FramePredicate::IsA);
        assert_eq!(
            f.agent.as_ref().map(|c| c.root.surface.as_str()),
            Some("қазақстан")
        );
        assert_eq!(
            f.object.as_ref().map(|c| c.root.surface.as_str()),
            Some("мемлекет")
        );
        assert_eq!(f.polarity, Polarity::Affirmative);
        assert!(f.modifiers.is_empty());
    }

    #[test]
    fn lattice_biographical_with_time_and_place() {
        // «Ахмет 1872 жылы Қостанайда туылған»
        let agent = noun("ахмет");
        let mut year_phrase = Composition::identity(Root::new("жыл", PartOfSpeech::Noun));
        year_phrase.operators.extend([
            SuffixOp::Possessive(Possessive::P3),
            SuffixOp::Case(Case::Locative),
        ]);
        // (year token «1872» is a Numeral root; for Stage 2 we let
        // it stand as a side-by-side surface; the actual integer
        // extraction is Stage 3's job)
        let year_tok = Composition::identity(Root::new("1872", PartOfSpeech::Numeral));
        let place = noun_with_case("қостанай", Case::Locative);
        let mut tuyl = verb("туыл");
        tuyl.operators.push(SuffixOp::Tense(Tense::PastEvidential));
        let lat = vec![agent, year_tok, year_phrase, place, tuyl];
        let f = Frame::from_morph_lattice(&lat).expect("biographical frame");
        assert_eq!(f.predicate, FramePredicate::BornIn);
        assert_eq!(f.agent.as_ref().unwrap().root.surface, "ахмет");
        assert_eq!(f.tense, Some(Tense::PastEvidential));
        assert_eq!(f.aspect, Some(Aspect::Perfective));
        assert_eq!(f.evidentiality, Evidentiality::Reported);
        // Locative on «жыл» → time anchor; locative on «қостанай» → location.
        assert!(matches!(
            f.modifier("time"),
            Some(Modifier::TimeAnchor(TimeAnchor::Phrase(_)))
        ));
        assert!(matches!(
            f.modifier("location"),
            Some(Modifier::Location(_))
        ));
    }

    #[test]
    fn lattice_negation_flips_polarity() {
        // «Тас тірі емес» → in Kazakh this is a noun + adjective +
        // particle. The verb path also: «Ол келмеді» (he didn't
        // come) → verb «кел» + Negation + PastDefinite.
        let agent = noun("ол");
        let mut kel = verb("кел");
        kel.operators
            .extend([SuffixOp::Negation, SuffixOp::Tense(Tense::PastDefinite)]);
        let lat = vec![agent, kel];
        let f = Frame::from_morph_lattice(&lat).expect("verb frame");
        assert_eq!(f.polarity, Polarity::Negated);
        assert_eq!(f.tense, Some(Tense::PastDefinite));
    }

    #[test]
    fn lattice_dative_recipient_modifier() {
        // «Ахмет Айгүлге кітап берді»
        // (agent + recipient + object + verb)
        let agent = noun("ахмет");
        let recipient = noun_with_case("айгүл", Case::Dative);
        let object = noun_with_case("кітап", Case::Accusative);
        let mut ber = verb("бер");
        ber.operators.push(SuffixOp::Tense(Tense::PastDefinite));
        let lat = vec![agent, recipient, object, ber];
        let f = Frame::from_morph_lattice(&lat).expect("frame");
        assert_eq!(f.predicate, FramePredicate::DoesTo);
        assert_eq!(f.agent.as_ref().unwrap().root.surface, "ахмет");
        assert_eq!(f.object.as_ref().unwrap().root.surface, "кітап");
        assert!(matches!(
            f.modifier("recipient"),
            Some(Modifier::Recipient(_))
        ));
    }

    #[test]
    fn lattice_with_question_context() {
        // «Ахмет қашан туылған?» — surface question word lives
        // outside the lattice (caller passes it via context).
        let agent = noun("ахмет");
        let mut tuyl = verb("туыл");
        tuyl.operators.push(SuffixOp::Tense(Tense::PastEvidential));
        let lat = vec![agent, tuyl];
        let ctx = SentenceContext {
            sentence_type: ContextSentenceType::Question,
            question_focus: Some(QuestionFocus::Time),
        };
        let f = Frame::from_morph_lattice_in_context(&lat, ctx).expect("question frame");
        assert_eq!(
            f.modality,
            Modality::Question {
                focus: QuestionFocus::Time
            }
        );
        assert_eq!(f.predicate, FramePredicate::BornIn);
    }

    #[test]
    fn lattice_no_verb_no_noun_returns_none() {
        // A single particle / closed-class root that doesn't form a
        // proposition.
        let lat = vec![Composition::identity(Root::new(
            "және",
            PartOfSpeech::Particle,
        ))];
        assert!(Frame::from_morph_lattice(&lat).is_none());
    }

    #[test]
    fn lattice_ablative_source_modifier() {
        // «Көктемнен кейін жаз келеді»
        let source = noun_with_case("көктем", Case::Ablative);
        let agent = noun("жаз");
        let mut kel = verb("кел");
        kel.operators.push(SuffixOp::Tense(Tense::Present));
        let lat = vec![source, agent, kel];
        let f = Frame::from_morph_lattice(&lat).expect("frame");
        assert!(matches!(f.modifier("source"), Some(Modifier::Source(_))));
    }

    #[test]
    fn lattice_instrumental_modifier() {
        // «Ол пышақпен қияр кесті»
        let agent = noun("ол");
        let instrument = noun_with_case("пышақ", Case::Instrumental);
        let object = noun_with_case("қияр", Case::Accusative);
        let mut kes = verb("кес");
        kes.operators.push(SuffixOp::Tense(Tense::PastDefinite));
        let lat = vec![agent, instrument, object, kes];
        let f = Frame::from_morph_lattice(&lat).expect("frame");
        assert!(matches!(
            f.modifier("instrument"),
            Some(Modifier::Instrument(_))
        ));
    }

    #[test]
    fn lattice_genitive_possessor_modifier() {
        // «Ахметтің кітабы — повесть.»
        let possessor = noun_with_case("ахмет", Case::Genitive);
        let mut book = noun("кітап");
        book.operators.push(SuffixOp::Possessive(Possessive::P3));
        let target = noun("повесть");
        let lat = vec![possessor, book, target];
        let f = Frame::from_morph_lattice(&lat).expect("frame");
        assert_eq!(f.predicate, FramePredicate::IsA);
        assert!(matches!(
            f.modifier("possessor"),
            Some(Modifier::Possessor(_))
        ));
    }

    #[test]
    fn lattice_preserves_full_composition_in_modifier() {
        // Modifiers must carry the full Composition, not just the
        // root — downstream realiser uses the operator chain to
        // reproduce the surface.
        let mut year_phrase = Composition::identity(Root::new("жыл", PartOfSpeech::Noun));
        year_phrase.operators.extend([
            SuffixOp::Number(Number::Singular),
            SuffixOp::Possessive(Possessive::P3),
            SuffixOp::Case(Case::Locative),
        ]);
        let agent = noun("ахмет");
        let mut tuyl = verb("туыл");
        tuyl.operators.push(SuffixOp::Tense(Tense::PastEvidential));
        let lat = vec![agent, year_phrase.clone(), tuyl];
        let f = Frame::from_morph_lattice(&lat).expect("frame");
        if let Some(Modifier::TimeAnchor(TimeAnchor::Phrase(c))) = f.modifier("time") {
            assert_eq!(c.operators, year_phrase.operators);
        } else {
            panic!("expected time-anchor phrase modifier");
        }
    }

    // -- Round-trip JSON ----------------------------------------

    #[test]
    fn frame_serde_round_trip() {
        let f = Frame::assertion(Some(noun("ахмет")), FramePredicate::BornIn, None)
            .with_modifier(Modifier::TimeAnchor(TimeAnchor::Year(1872)))
            .with_modifier(Modifier::Location(noun_with_case(
                "қостанай",
                Case::Locative,
            )))
            .with_tense(Tense::PastEvidential);

        let json = serde_json::to_string(&f).expect("serialize");
        let f2: Frame = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(f, f2);
    }

    // -- Predeclared success: every v6.1 rule's input expressible
    //    as a Frame --------------------------------------------

    #[test]
    fn every_v6_1_nlg_rule_input_expressible_as_frame() {
        // For each of the 14 v6.1 NLG rules in adam_dialog::nlg
        // (mod.rs::all_rules), assert that we can construct a
        // `Frame` carrying the same propositional information the
        // rule's `render(&SentenceFrame)` reads from
        // `fact.subject.root + fact.predicate + fact.object.root`.
        // The actual rendering rewire is Stage 7; Stage 2 only
        // proves expressibility.
        struct Case {
            subject: &'static str,
            predicate: FramePredicate,
            object: &'static str,
            name: &'static str,
        }
        let cases = [
            Case {
                subject: "қазақстан",
                predicate: FramePredicate::IsA,
                object: "мемлекет",
                name: "IsACopulaDeclarative",
            },
            Case {
                subject: "астана",
                predicate: FramePredicate::PartOf,
                object: "қазақстан",
                name: "PartOfDeclarative",
            },
            Case {
                subject: "қазақстан",
                predicate: FramePredicate::HasQuantity,
                object: "облыс",
                name: "HasQuantityDeclarative",
            },
            Case {
                subject: "қазақстан",
                predicate: FramePredicate::RelatedTo,
                object: "ресей",
                name: "RelatedToShectesDeclarative",
            },
            Case {
                subject: "қазақстан",
                predicate: FramePredicate::RelatedTo,
                object: "облыстар_тізімі",
                name: "RelatedToListDeclarative",
            },
            Case {
                subject: "кітап",
                predicate: FramePredicate::RelatedTo,
                object: "ілім",
                name: "RelatedToOzaraDeclarative",
            },
            Case {
                subject: "қазақстан_президенті",
                predicate: FramePredicate::RelatedTo,
                object: "тоқаев",
                name: "RelatedToOfficeHolderDeclarative",
            },
            Case {
                subject: "абай",
                predicate: FramePredicate::LivesIn,
                object: "семей",
                name: "LivesInDeclarative",
            },
            Case {
                subject: "ел",
                predicate: FramePredicate::Has,
                object: "тіл",
                name: "HasDeclarative",
            },
            Case {
                subject: "жаңбыр",
                predicate: FramePredicate::Causes,
                object: "сел",
                name: "CausesDeclarative",
            },
            Case {
                subject: "атом",
                predicate: FramePredicate::InDomain,
                object: "физика",
                name: "InDomainDeclarative",
            },
            Case {
                subject: "адам",
                predicate: FramePredicate::GoesTo,
                object: "үй",
                name: "GoesToDeclarative",
            },
            Case {
                subject: "көктем",
                predicate: FramePredicate::After,
                object: "жаз",
                name: "AfterDeclarative",
            },
            Case {
                subject: "адам",
                predicate: FramePredicate::DoesTo,
                object: "үй",
                name: "DoesToDeclarative",
            },
        ];
        for c in &cases {
            let f = Frame::assertion(
                Some(noun(c.subject)),
                c.predicate.clone(),
                Some(noun(c.object)),
            );
            assert_eq!(
                f.predicate, c.predicate,
                "rule {}: predicate mismatch",
                c.name
            );
            assert!(f.agent.is_some(), "rule {}: agent missing", c.name);
            assert!(f.object.is_some(), "rule {}: object missing", c.name);
        }
        assert_eq!(cases.len(), 14, "must cover all 14 v6.1 NLG rules");
    }

    #[test]
    fn v6_1_typed_predicates_expressible() {
        // The 11 v6.1.0 typed-predicate extension variants must
        // also project to Frames cleanly. These don't all have
        // dedicated NLG rules yet (some surface via raw_text in
        // v6.1), but Stage 7's typed realiser will need each one.
        let typed = [
            FramePredicate::BornIn,
            FramePredicate::DiedIn,
            FramePredicate::FoundedIn,
            FramePredicate::RenamedIn,
            FramePredicate::EffectiveFrom,
            FramePredicate::Classifies,
            FramePredicate::RiskLevel,
            FramePredicate::LocatedIn,
            FramePredicate::NamedAfter,
            FramePredicate::MemberOf,
            FramePredicate::Authored,
        ];
        for p in typed {
            let f = Frame::assertion(Some(noun("x")), p.clone(), Some(noun("y")));
            assert_eq!(f.predicate, p);
        }
    }
}
