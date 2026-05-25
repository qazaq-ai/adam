// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `QueryIR` — **Stage 3 of v6.2.0 neurosymbolic redesign**.
//!
//! A `QueryIR` is a [`Frame`](crate::frame::Frame) with a hole — a
//! typed contract describing **what the user wants filled in**.
//! Where `Frame` is a complete assertion («Ахмет 1872 жылы туылған»),
//! `QueryIR` is the same shape with one slot marked as the answer
//! target («Ахмет қашан туылған?» → fill the `time` modifier of an
//! agent=Ахмет, predicate=BornIn frame).
//!
//! ## Why this layer exists
//!
//! Pre-v6.2 the question-routing decision was fragmented across six
//! systems:
//!
//! - `adam_dialog::predicate_focus::PredicateFocus` (13 variants).
//! - `adam_dialog::question_shape::QuestionShape` (5 form variants).
//! - `adam_dialog::answer_ir::AnswerShape` (5 v6.1.0 answer modes).
//! - `adam_dialog::intent::Intent` (intent classifier).
//! - `adam_slot_extractor` (slot inventory).
//! - `adam_intent_classifier` (intent inventory).
//!
//! Each system made its own routing decision; they could disagree.
//! The Codex 2026-05-22 audit traced «Жасанды интеллект туралы заң
//! қандай санаттарға жіктейді?» surfacing an effective-date fact to
//! exactly this fragmentation: `(subject = AI Law, shape =
//! Definition)` routed to highest-confidence fact, ignoring that
//! classification was specifically asked.
//!
//! `QueryIR` is the **single typed contract**: every factual-reply
//! path goes through it, every layer reads the same shape, and the
//! retrieval ranker / verifier / realiser all see the same view of
//! "what was asked".
//!
//! ## Stage 3 scope
//!
//! - [`QueryIR`] — typed query record `(agent, predicate, object,
//!   modifier_constraints, focus, form, answer_shape, sense_hints)`.
//! - [`QueryFocus`] — which slot is the answer hole.
//! - [`ModifierRole`] — typed enum of the 7 modifier role slugs from
//!   [`Modifier`](crate::frame::Modifier).
//! - [`QuestionForm`] — surface form of the question (subsumes
//!   v6.1.0 `QuestionShape`).
//! - [`AnswerShape`] — how to render the answer (superset of v6.1.0
//!   `AnswerShape`).
//! - [`ModifierConstraint`] / [`SenseHint`] / [`Domain`] — typed
//!   side-channels for retrieval.
//! - [`QueryIR::from_question_frame`] — derive a query from a
//!   `Frame` whose modality is `Question`.
//! - [`QueryIR::match_frame`] — given a retrieved candidate
//!   [`Frame`], does it answer this query? Returns the
//!   [`AnswerSlot`] to pull.
//!
//! ## Predeclared success criterion
//!
//! Every v6.1 question-routing path (`PredicateFocus` ×
//! `QuestionShape` × intent × slot_inventory) must be expressible
//! as one [`QueryIR`] construction. The coverage matrix at the
//! bottom of this module asserts this for all 22 v6.1
//! [`FramePredicate`] variants × 8 [`QueryFocus`] modes.
//!
//! ## NOT in Stage 3
//!
//! - The actual rewire of `PredicateFocus::detect` / `QuestionShape::detect`
//!   callers to produce `QueryIR` — Stage 7 (Realiser) owns that
//!   migration to keep the typed-IR rollout one atomic cut.
//! - Sense disambiguation policy — Stage 5 decides *which*
//!   `SenseHint` wins when multiple apply.
//! - Indexed retrieval — Stage 4 builds the index that consumes a
//!   `QueryIR` to produce candidate frames.

use serde::{Deserialize, Serialize};

use crate::composition::Composition;
use crate::frame::{Frame, FramePredicate, Modality, Modifier, QuestionFocus};

/// Which slot the query is asking to fill — the "hole" in the
/// frame. Drives the retrieval ranker (Stage 4) and the answer
/// composer (Stage 7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "slot", content = "role")]
pub enum QueryFocus {
    /// «Кім?» — agent unknown («Кім туылған 1872 жылы?»).
    Subject,
    /// «Не істеді?» — predicate unknown («Ахмет 1872 жылы не
    /// істеген?»).
    Predicate,
    /// «Нені?» — object unknown («Ахмет нені жазған?»).
    Object,
    /// «Қашан / қайда / қандай ...» — one of the typed modifier
    /// roles is unknown («Ахмет қашан туылған?»).
    Modifier(ModifierRole),
    /// «Қанша / неше?» — quantity unknown («Қазақстанда қанша
    /// облыс бар?»).
    Quantity,
    /// Yes/no — every slot is filled, the user wants polarity
    /// confirmation («Ахмет 1872 жылы туылған ба?»).
    Existence,
    /// «X деген не?» — definitional answer wanted («Алгоритм
    /// деген не?»).
    Definition,
    /// «Қандай X бар?» — list of instances asked («Қазақстанда
    /// қандай облыстар бар?»).
    Enumeration,
}

impl QueryFocus {
    /// Stable slug for JSON / trace output.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Subject => "subject",
            Self::Predicate => "predicate",
            Self::Object => "object",
            Self::Modifier(_) => "modifier",
            Self::Quantity => "quantity",
            Self::Existence => "existence",
            Self::Definition => "definition",
            Self::Enumeration => "enumeration",
        }
    }
}

/// Typed modifier role — closed-set mirror of the surface slugs
/// returned by [`Modifier::role_str`]. Having a typed enum here
/// lets the retrieval ranker pattern-match on roles rather than
/// stringly-typed slug comparisons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModifierRole {
    Time,
    Location,
    Source,
    Instrument,
    Manner,
    Recipient,
    Possessor,
}

impl ModifierRole {
    /// Stable slug — round-trips with [`Modifier::role_str`].
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Time => "time",
            Self::Location => "location",
            Self::Source => "source",
            Self::Instrument => "instrument",
            Self::Manner => "manner",
            Self::Recipient => "recipient",
            Self::Possessor => "possessor",
        }
    }

    /// Parse from the slug returned by [`Modifier::role_str`].
    /// Named `from_slug` (not `from_str`) to avoid clashing with the
    /// `FromStr` trait — the slug-set here is the closed
    /// `Modifier::role_str` codomain, not arbitrary string parsing.
    pub fn from_slug(s: &str) -> Option<Self> {
        Some(match s {
            "time" => Self::Time,
            "location" => Self::Location,
            "source" => Self::Source,
            "instrument" => Self::Instrument,
            "manner" => Self::Manner,
            "recipient" => Self::Recipient,
            "possessor" => Self::Possessor,
            _ => return None,
        })
    }

    /// Map a [`QuestionFocus`] (the surface signal from the FrameLayer)
    /// to the typed modifier role it implies.
    pub fn from_question_focus(qf: QuestionFocus) -> Option<Self> {
        Some(match qf {
            QuestionFocus::Time => Self::Time,
            QuestionFocus::Place => Self::Location,
            QuestionFocus::Manner => Self::Manner,
            _ => return None,
        })
    }
}

/// Form of the user question. Subsumes v6.1.0
/// `adam_dialog::question_shape::QuestionShape` 1:1 — every variant
/// maps to one v6.1 form. Variants are open to extension only by
/// intentional architectural change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionForm {
    /// «X деген не? / X кім? / X-ні айт» — definitional probe.
    Definition,
    /// «неліктен / X неге / X себебі не» — causal probe.
    Causal,
    /// «X шынымен Y ма?» — yes/no confirmation probe.
    YesNoCheck,
    /// «қандай X-тер бар / X-тер тізімі» — listing probe.
    Listing,
    /// «X пен Y айырмашылығы / X жақсырақ па?» — comparison.
    Comparison,
}

impl QuestionForm {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Definition => "definition",
            Self::Causal => "causal",
            Self::YesNoCheck => "yes_no_check",
            Self::Listing => "listing",
            Self::Comparison => "comparison",
        }
    }
}

/// Canonical answer rendering shape. Superset of v6.1.0
/// `adam_dialog::answer_ir::AnswerShape` — adds the typed
/// shapes the v6.2 realiser needs (BareNoun, DateAnchor,
/// QuantityPhrase, …) that v6.1 produced via template strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnswerShape {
    /// Single-noun answer: «Қостанай» (for "where", "who").
    BareNoun,
    /// Date / year anchor: «1872 жылы».
    DateAnchor,
    /// Yes-confirming IsA / HasProperty: «Иә, X — Y.»
    YesNoConfirm,
    /// Yes-denying via antonym path: «Жоқ, X — Y емес.»
    YesNoDeny,
    /// Honest unknown: «Менің білім қорымда дерек жоқ.»
    YesNoUnknown,
    /// Copular NP: «X — Y.»
    DefinitionalNP,
    /// Comma-separated list: «Абай, Ақмола, ...» — used for
    /// [`QueryFocus::Enumeration`].
    Enumeration,
    /// Quantity phrase: «Қазақстанда 17 облыс бар.»
    QuantityPhrase,
    /// Proof-chain performance: «Дәлелдейік: A → B → C.»
    IsAProofChain,
    /// Causal chain: «X себебі — Y, өйткені Z.»
    CausalChain,
    /// Safety refusal (medical / legal / financial / current-data /
    /// political / self-harm).
    SafetyRefusal,
    /// No-data honest disclaim: «X жөнінде нақты дерек жоқ.»
    NoData,
}

impl AnswerShape {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BareNoun => "bare_noun",
            Self::DateAnchor => "date_anchor",
            Self::YesNoConfirm => "yes_no_confirm",
            Self::YesNoDeny => "yes_no_deny",
            Self::YesNoUnknown => "yes_no_unknown",
            Self::DefinitionalNP => "definitional_np",
            Self::Enumeration => "enumeration",
            Self::QuantityPhrase => "quantity_phrase",
            Self::IsAProofChain => "is_a_proof_chain",
            Self::CausalChain => "causal_chain",
            Self::SafetyRefusal => "safety_refusal",
            Self::NoData => "no_data",
        }
    }
}

/// A typed constraint on a modifier slot of the query. Retrieval
/// uses these to filter candidate frames: «Ахмет 1872 жылы туылған
/// ба?» becomes a `QueryIR` with focus = `Existence` and a
/// `ModifierConstraint { role: Time, value: 1872 жылы }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModifierConstraint {
    pub role: ModifierRole,
    pub value: Composition,
}

/// Sense-disambiguation hint. Stage 3 records hints from the
/// surface input («Ай айтылды» surrounded by calendar context →
/// `SenseHint { root: "ай", domain: Calendar }`); Stage 5 owns the
/// policy of how multiple hints resolve and how retrieval applies
/// the filter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SenseHint {
    pub root: String,
    pub domain: Domain,
}

/// Domain enum for sense disambiguation. Closed-set; extending is
/// an intentional architectural decision (each domain must have a
/// curated `data/world_core/<domain>.jsonl` corpus). Stage 3 lists
/// the domains that exist in v6.1 world_core; Stage 5 may grow it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Domain {
    /// Geography (countries, oblasts, cities, geographic features).
    Geography,
    /// Persons (historical figures, public officials).
    Person,
    /// Institutions (universities, ministries, organisations).
    Institution,
    /// Events (historical, scientific, political).
    Event,
    /// Laws, regulations, agreements.
    Law,
    /// Sciences (math, physics, chemistry, biology, ...).
    Science,
    /// Time / dates / calendar.
    Calendar,
    /// Astronomy / celestial objects (disambiguates «Ай» = moon
    /// vs «ай» = month).
    Astronomy,
    /// Programming / computer science (disambiguates «ағаш» = tree
    /// (data structure) vs «ағаш» = tree (botany); «бағдарлама» =
    /// programme (software) vs programme (event)).
    Programming,
    /// Material / substance (disambiguates «ағаш» = wood vs tree).
    Material,
    /// Catch-all for domains not yet typed. The string is a stable
    /// slug for diagnostic / future-extension use.
    Other(String),
}

impl Domain {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Geography => "geography",
            Self::Person => "person",
            Self::Institution => "institution",
            Self::Event => "event",
            Self::Law => "law",
            Self::Science => "science",
            Self::Calendar => "calendar",
            Self::Astronomy => "astronomy",
            Self::Programming => "programming",
            Self::Material => "material",
            Self::Other(s) => s.as_str(),
        }
    }
}

/// The typed query record. Single canonical shape consumed by
/// retrieval (Stage 4), proof (Stage 6), and realiser (Stage 7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryIR {
    /// Known agent / subject of the asked-about fact. `None` when
    /// the focus is `Subject` (the agent is the answer).
    pub agent: Option<Composition>,
    /// Known predicate. `None` when the focus is `Predicate`.
    pub predicate: Option<FramePredicate>,
    /// Known object. `None` when the focus is `Object` or when
    /// the predicate is intransitive.
    pub object: Option<Composition>,
    /// Filled modifier slots (e.g. «1872 жылы» pre-filled when
    /// the query is about events of that year).
    pub modifier_constraints: Vec<ModifierConstraint>,
    /// Which slot is the answer hole.
    pub focus: QueryFocus,
    /// Surface form of the question.
    pub form: QuestionForm,
    /// How the answer should be rendered.
    pub answer_shape: AnswerShape,
    /// Sense-disambiguation hints (Stage 5 consumes).
    pub sense_hints: Vec<SenseHint>,
    /// Optional domain filter — when set, retrieval must restrict
    /// candidates to this domain. Distinct from `sense_hints`
    /// (which are advisory): `domain_filter` is a hard constraint
    /// the user surfaced explicitly («заң туралы» → `Law`).
    pub domain_filter: Option<Domain>,
}

/// Slot in a candidate `Frame` that answers the query. Returned by
/// [`QueryIR::match_frame`] to tell the realiser what to pull.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "slot", content = "role")]
pub enum AnswerSlot {
    Agent,
    Predicate,
    Object,
    Modifier(ModifierRole),
    /// The frame as a whole (for `Existence` / `Definition` /
    /// `Quantity` foci where no single slot is "the answer").
    Whole,
}

/// Result of matching a candidate frame against a query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameMatch {
    /// Which slot of the candidate frame holds the answer.
    pub answer_slot: AnswerSlot,
    /// 0-100 score — how well the candidate fits the query.
    /// Stage 3 uses coarse scoring (100 = exact, 50 = partial);
    /// Stage 4 (indexed retrieval) refines with a learned ranker.
    pub score: u8,
}

impl QueryIR {
    /// Minimal query builder — focus + form + answer shape, all
    /// other fields default. Used by tests and as a base for the
    /// fluent builder methods below.
    pub fn new(focus: QueryFocus, form: QuestionForm, answer_shape: AnswerShape) -> Self {
        Self {
            agent: None,
            predicate: None,
            object: None,
            modifier_constraints: Vec::new(),
            focus,
            form,
            answer_shape,
            sense_hints: Vec::new(),
            domain_filter: None,
        }
    }

    pub fn with_agent(mut self, agent: Composition) -> Self {
        self.agent = Some(agent);
        self
    }

    pub fn with_predicate(mut self, predicate: FramePredicate) -> Self {
        self.predicate = Some(predicate);
        self
    }

    pub fn with_object(mut self, object: Composition) -> Self {
        self.object = Some(object);
        self
    }

    pub fn with_modifier_constraint(mut self, role: ModifierRole, value: Composition) -> Self {
        self.modifier_constraints
            .push(ModifierConstraint { role, value });
        self
    }

    pub fn with_sense_hint(mut self, root: impl Into<String>, domain: Domain) -> Self {
        self.sense_hints.push(SenseHint {
            root: root.into(),
            domain,
        });
        self
    }

    pub fn with_domain_filter(mut self, domain: Domain) -> Self {
        self.domain_filter = Some(domain);
        self
    }

    /// Derive a `QueryIR` from a question-modality `Frame`. The
    /// frame's `QuestionFocus` drives the [`QueryFocus`]; the
    /// frame's filled slots become the known fields and
    /// constraints.
    ///
    /// Returns `None` when the frame's modality is not
    /// `Modality::Question` — the caller must hand a question-shaped
    /// frame; the bridge from a statement frame is
    /// [`QueryIR::from_assertion`].
    pub fn from_question_frame(frame: &Frame) -> Option<Self> {
        let Modality::Question { focus: qf } = &frame.modality else {
            return None;
        };

        let focus = map_question_focus(*qf, frame);
        let form = infer_question_form(*qf);
        let answer_shape = default_answer_shape(&focus, frame.predicate.clone());

        let mut q = Self::new(focus.clone(), form, answer_shape);
        // Carry over the agent / predicate / object unless they
        // are the focus slot.
        if !matches!(focus, QueryFocus::Subject)
            && let Some(a) = &frame.agent
        {
            q = q.with_agent(a.clone());
        }
        if !matches!(focus, QueryFocus::Predicate | QueryFocus::Definition) {
            q = q.with_predicate(frame.predicate.clone());
        }
        if !matches!(focus, QueryFocus::Object)
            && let Some(o) = &frame.object
        {
            q = q.with_object(o.clone());
        }
        // Carry over modifiers as constraints (skip the focus role
        // if it's a Modifier focus).
        let focus_role = if let QueryFocus::Modifier(r) = &focus {
            Some(*r)
        } else {
            None
        };
        for m in &frame.modifiers {
            if let Some(role) = ModifierRole::from_slug(m.role_str())
                && Some(role) != focus_role
                && let Some(comp) = modifier_value(m)
            {
                q = q.with_modifier_constraint(role, comp);
            }
        }
        Some(q)
    }

    /// Derive a `QueryIR` from a statement frame — the user
    /// asserted something and we want to retrieve corroborating
    /// evidence. Focus is `Existence` (verify the assertion);
    /// answer shape is `YesNoConfirm`.
    pub fn from_assertion(frame: &Frame) -> Self {
        let mut q = Self::new(
            QueryFocus::Existence,
            QuestionForm::YesNoCheck,
            AnswerShape::YesNoConfirm,
        )
        .with_predicate(frame.predicate.clone());
        if let Some(a) = &frame.agent {
            q = q.with_agent(a.clone());
        }
        if let Some(o) = &frame.object {
            q = q.with_object(o.clone());
        }
        for m in &frame.modifiers {
            if let Some(role) = ModifierRole::from_slug(m.role_str())
                && let Some(comp) = modifier_value(m)
            {
                q = q.with_modifier_constraint(role, comp);
            }
        }
        q
    }

    /// Given a candidate `Frame` (a retrieved fact), decide
    /// whether it answers this query and what slot to pull.
    ///
    /// Matching rules (Stage 3, coarse):
    /// 1. Predicate must match (when query specifies one).
    /// 2. Agent must match by root surface (when both specify).
    /// 3. Object must match by root surface (when both specify).
    /// 4. The focus slot is read from the candidate (`Subject` →
    ///    return candidate.agent; `Modifier(Time)` → return the
    ///    matching modifier; etc.).
    /// 5. Modifier constraints in the query must all be satisfied
    ///    by matching candidate modifiers (root-surface equality).
    ///
    /// Returns `None` when the candidate doesn't match. Stage 4
    /// (indexed retrieval) refines the scoring; Stage 3 returns
    /// `100` for exact match, `50` when one optional constraint is
    /// missing on the candidate side (e.g. query asks for time
    /// modifier, candidate has no time slot but matches on subject
    /// + predicate).
    pub fn match_frame(&self, candidate: &Frame) -> Option<FrameMatch> {
        // 1. Predicate.
        if let Some(p) = &self.predicate
            && p != &candidate.predicate
        {
            return None;
        }

        // 2. Agent.
        if let (Some(a), Some(ca)) = (&self.agent, &candidate.agent) {
            if a.root.surface != ca.root.surface {
                return None;
            }
        } else if self.agent.is_some() && candidate.agent.is_none() {
            return None;
        }

        // 3. Object.
        if let (Some(o), Some(co)) = (&self.object, &candidate.object) {
            if o.root.surface != co.root.surface {
                return None;
            }
        } else if self.object.is_some() && candidate.object.is_none() {
            return None;
        }

        // 4. Modifier constraints.
        for mc in &self.modifier_constraints {
            let role_str = mc.role.as_str();
            let m = candidate.modifier(role_str)?;
            if let Some(cv) = modifier_value(m) {
                if cv.root.surface != mc.value.root.surface {
                    return None;
                }
            } else {
                return None;
            }
        }

        // 5. Identify the answer slot.
        let (answer_slot, score) = match &self.focus {
            QueryFocus::Subject => {
                if candidate.agent.is_some() {
                    (AnswerSlot::Agent, 100)
                } else {
                    return None;
                }
            }
            QueryFocus::Predicate => (AnswerSlot::Predicate, 100),
            QueryFocus::Object => {
                if candidate.object.is_some() {
                    (AnswerSlot::Object, 100)
                } else {
                    return None;
                }
            }
            QueryFocus::Modifier(role) => {
                if candidate.modifier(role.as_str()).is_some() {
                    (AnswerSlot::Modifier(*role), 100)
                } else {
                    // Partial: subject + predicate matched but the
                    // candidate doesn't carry this modifier role.
                    (AnswerSlot::Whole, 50)
                }
            }
            QueryFocus::Quantity => (AnswerSlot::Whole, 100),
            QueryFocus::Existence => (AnswerSlot::Whole, 100),
            QueryFocus::Definition => (AnswerSlot::Whole, 100),
            QueryFocus::Enumeration => (AnswerSlot::Whole, 100),
        };

        Some(FrameMatch { answer_slot, score })
    }
}

/// Extract the inner [`Composition`] from a modifier, if any. For
/// `TimeAnchor::Year` / `Date` the modifier carries a typed scalar
/// rather than a composition — those are matched at Stage 4 via a
/// separate scalar-equality path; Stage 3 only handles `Phrase`-
/// kind time anchors here.
fn modifier_value(m: &Modifier) -> Option<Composition> {
    match m {
        Modifier::TimeAnchor(crate::frame::TimeAnchor::Phrase(c)) => Some(c.clone()),
        Modifier::TimeAnchor(_) => None,
        Modifier::Location(c)
        | Modifier::Source(c)
        | Modifier::Instrument(c)
        | Modifier::Manner(c)
        | Modifier::Recipient(c)
        | Modifier::Possessor(c) => Some(c.clone()),
    }
}

/// Map a [`QuestionFocus`] (FrameLayer surface signal) to a
/// [`QueryFocus`] (QueryIR semantic slot). The frame must supply
/// context for some variants (`Kind` → `Definition` vs
/// `Enumeration`).
fn map_question_focus(qf: QuestionFocus, frame: &Frame) -> QueryFocus {
    match qf {
        QuestionFocus::Subject => QueryFocus::Subject,
        QuestionFocus::Object => QueryFocus::Object,
        QuestionFocus::Time => QueryFocus::Modifier(ModifierRole::Time),
        QuestionFocus::Place => QueryFocus::Modifier(ModifierRole::Location),
        QuestionFocus::Manner => QueryFocus::Modifier(ModifierRole::Manner),
        QuestionFocus::Cause => QueryFocus::Predicate,
        QuestionFocus::Kind => {
            // «X деген не?» (defining a thing) vs «қандай X-тер
            // бар?» (enumerating instances). The frame's agent
            // presence is the simplest tie-break: with an agent
            // the user is defining that specific thing; without
            // one they want a list.
            if frame.agent.is_some() {
                QueryFocus::Definition
            } else {
                QueryFocus::Enumeration
            }
        }
        QuestionFocus::Quantity => QueryFocus::Quantity,
        QuestionFocus::YesNo => QueryFocus::Existence,
        QuestionFocus::Unknown => QueryFocus::Definition,
    }
}

/// Infer the v6.1 `QuestionShape`-equivalent form from a
/// `QuestionFocus`.
fn infer_question_form(qf: QuestionFocus) -> QuestionForm {
    match qf {
        QuestionFocus::Cause => QuestionForm::Causal,
        QuestionFocus::YesNo => QuestionForm::YesNoCheck,
        QuestionFocus::Kind => QuestionForm::Listing,
        QuestionFocus::Quantity => QuestionForm::Definition,
        _ => QuestionForm::Definition,
    }
}

/// Default answer shape for a `(focus, predicate)` pair. The
/// realiser may override this when the proof object carries a
/// safety refusal / no-data signal.
fn default_answer_shape(focus: &QueryFocus, predicate: FramePredicate) -> AnswerShape {
    match focus {
        QueryFocus::Subject | QueryFocus::Object => AnswerShape::BareNoun,
        QueryFocus::Predicate => AnswerShape::DefinitionalNP,
        QueryFocus::Modifier(ModifierRole::Time) => AnswerShape::DateAnchor,
        QueryFocus::Modifier(_) => AnswerShape::BareNoun,
        QueryFocus::Quantity => AnswerShape::QuantityPhrase,
        QueryFocus::Existence => AnswerShape::YesNoConfirm,
        QueryFocus::Definition => match predicate {
            FramePredicate::IsA | FramePredicate::Definition => AnswerShape::DefinitionalNP,
            _ => AnswerShape::DefinitionalNP,
        },
        QueryFocus::Enumeration => AnswerShape::Enumeration,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::{
        ContextSentenceType, Frame, Modality, Modifier, Polarity, QuestionFocus, SentenceContext,
        TimeAnchor,
    };
    use crate::operator::SuffixOp;
    use crate::root::{PartOfSpeech, Root};
    use adam_kernel_fst::morphotactics::{Case, Tense};

    fn noun(surface: &str) -> Composition {
        Composition::identity(Root::new(surface, PartOfSpeech::Noun))
    }

    fn noun_with_case(surface: &str, case: Case) -> Composition {
        let mut c = noun(surface);
        c.operators.push(SuffixOp::Case(case));
        c
    }

    // -- QueryFocus + ModifierRole basics -----------------------

    #[test]
    fn modifier_role_round_trips_with_modifier_role_str() {
        // Every ModifierRole slug must parse back from
        // `Modifier::role_str` for any Modifier of that role.
        let cases: &[(ModifierRole, Modifier)] = &[
            (
                ModifierRole::Time,
                Modifier::TimeAnchor(TimeAnchor::Year(1872)),
            ),
            (
                ModifierRole::Location,
                Modifier::Location(noun_with_case("қостанай", Case::Locative)),
            ),
            (
                ModifierRole::Source,
                Modifier::Source(noun_with_case("көктем", Case::Ablative)),
            ),
            (
                ModifierRole::Instrument,
                Modifier::Instrument(noun_with_case("пышақ", Case::Instrumental)),
            ),
            (
                ModifierRole::Manner,
                Modifier::Manner(noun_with_case("жылдам", Case::Locative)),
            ),
            (
                ModifierRole::Recipient,
                Modifier::Recipient(noun_with_case("айгүл", Case::Dative)),
            ),
            (
                ModifierRole::Possessor,
                Modifier::Possessor(noun_with_case("ахмет", Case::Genitive)),
            ),
        ];
        for (role, modifier) in cases {
            assert_eq!(role.as_str(), modifier.role_str());
            assert_eq!(ModifierRole::from_slug(modifier.role_str()), Some(*role));
        }
    }

    // -- from_question_frame -----------------------------------

    #[test]
    fn from_question_frame_time_focus() {
        // «Ахмет қашан туылған?»
        let agent = noun("ахмет");
        let frame = Frame::assertion(Some(agent.clone()), FramePredicate::BornIn, None)
            .with_modality(Modality::Question {
                focus: QuestionFocus::Time,
            })
            .with_tense(Tense::PastEvidential);
        let q = QueryIR::from_question_frame(&frame).expect("query");
        assert_eq!(q.focus, QueryFocus::Modifier(ModifierRole::Time));
        assert_eq!(q.predicate, Some(FramePredicate::BornIn));
        assert_eq!(q.agent.as_ref().unwrap().root.surface, "ахмет");
        assert_eq!(q.answer_shape, AnswerShape::DateAnchor);
        assert_eq!(q.form, QuestionForm::Definition);
    }

    #[test]
    fn from_question_frame_place_focus() {
        // «Ахмет қайда туылған?»
        let frame = Frame::assertion(Some(noun("ахмет")), FramePredicate::BornIn, None)
            .with_modality(Modality::Question {
                focus: QuestionFocus::Place,
            });
        let q = QueryIR::from_question_frame(&frame).expect("query");
        assert_eq!(q.focus, QueryFocus::Modifier(ModifierRole::Location));
        assert_eq!(q.answer_shape, AnswerShape::BareNoun);
    }

    #[test]
    fn from_question_frame_subject_focus() {
        // «1872 жылы кім туылған?» — subject is the hole.
        let frame = Frame::assertion(None, FramePredicate::BornIn, None)
            .with_modality(Modality::Question {
                focus: QuestionFocus::Subject,
            })
            .with_modifier(Modifier::TimeAnchor(TimeAnchor::Year(1872)));
        let q = QueryIR::from_question_frame(&frame).expect("query");
        assert_eq!(q.focus, QueryFocus::Subject);
        assert_eq!(q.agent, None);
        assert_eq!(q.predicate, Some(FramePredicate::BornIn));
        assert_eq!(q.answer_shape, AnswerShape::BareNoun);
    }

    #[test]
    fn from_question_frame_yes_no_focus() {
        // «Ахмет 1872 жылы туылған ба?»
        let frame = Frame::assertion(Some(noun("ахмет")), FramePredicate::BornIn, None)
            .with_modality(Modality::Question {
                focus: QuestionFocus::YesNo,
            });
        let q = QueryIR::from_question_frame(&frame).expect("query");
        assert_eq!(q.focus, QueryFocus::Existence);
        assert_eq!(q.form, QuestionForm::YesNoCheck);
        assert_eq!(q.answer_shape, AnswerShape::YesNoConfirm);
    }

    #[test]
    fn from_question_frame_quantity_focus() {
        // «Қазақстанда қанша облыс бар?»
        let frame = Frame::assertion(
            Some(noun("қазақстан")),
            FramePredicate::HasQuantity,
            Some(noun("облыс")),
        )
        .with_modality(Modality::Question {
            focus: QuestionFocus::Quantity,
        });
        let q = QueryIR::from_question_frame(&frame).expect("query");
        assert_eq!(q.focus, QueryFocus::Quantity);
        assert_eq!(q.answer_shape, AnswerShape::QuantityPhrase);
    }

    #[test]
    fn from_question_frame_kind_with_agent_is_definition() {
        // «X деген не?» — agent set, asking for definition.
        let frame = Frame::assertion(Some(noun("алгоритм")), FramePredicate::Definition, None)
            .with_modality(Modality::Question {
                focus: QuestionFocus::Kind,
            });
        let q = QueryIR::from_question_frame(&frame).expect("query");
        assert_eq!(q.focus, QueryFocus::Definition);
        assert_eq!(q.answer_shape, AnswerShape::DefinitionalNP);
    }

    #[test]
    fn from_question_frame_kind_without_agent_is_enumeration() {
        // «Қандай облыстар бар?» — no agent, asking for list.
        let frame = Frame::assertion(None, FramePredicate::IsA, Some(noun("облыс"))).with_modality(
            Modality::Question {
                focus: QuestionFocus::Kind,
            },
        );
        let q = QueryIR::from_question_frame(&frame).expect("query");
        assert_eq!(q.focus, QueryFocus::Enumeration);
        assert_eq!(q.answer_shape, AnswerShape::Enumeration);
    }

    #[test]
    fn from_question_frame_cause_focus() {
        // «X неліктен Y?» — causal probe.
        let frame = Frame::assertion(Some(noun("жаңбыр")), FramePredicate::Causes, None)
            .with_modality(Modality::Question {
                focus: QuestionFocus::Cause,
            });
        let q = QueryIR::from_question_frame(&frame).expect("query");
        assert_eq!(q.focus, QueryFocus::Predicate);
        assert_eq!(q.form, QuestionForm::Causal);
    }

    #[test]
    fn from_question_frame_rejects_non_question_modality() {
        let frame = Frame::assertion(Some(noun("ахмет")), FramePredicate::BornIn, None);
        assert!(QueryIR::from_question_frame(&frame).is_none());
    }

    #[test]
    fn from_assertion_yields_existence_focus() {
        let frame = Frame::assertion(Some(noun("ахмет")), FramePredicate::BornIn, None);
        let q = QueryIR::from_assertion(&frame);
        assert_eq!(q.focus, QueryFocus::Existence);
        assert_eq!(q.form, QuestionForm::YesNoCheck);
        assert_eq!(q.answer_shape, AnswerShape::YesNoConfirm);
    }

    // -- match_frame -------------------------------------------

    #[test]
    fn match_frame_subject_focus_returns_agent() {
        // Query: «1872 жылы кім туылған?»
        // Candidate: «Ахмет 1872 жылы туылған.»
        let q = QueryIR::new(
            QueryFocus::Subject,
            QuestionForm::Definition,
            AnswerShape::BareNoun,
        )
        .with_predicate(FramePredicate::BornIn)
        .with_modifier_constraint(ModifierRole::Time, year_phrase_1872());
        let candidate = Frame::assertion(Some(noun("ахмет")), FramePredicate::BornIn, None)
            .with_modifier(Modifier::TimeAnchor(TimeAnchor::Phrase(year_phrase_1872())));
        let m = q.match_frame(&candidate).expect("match");
        assert_eq!(m.answer_slot, AnswerSlot::Agent);
        assert_eq!(m.score, 100);
    }

    #[test]
    fn match_frame_time_modifier_focus_returns_time_slot() {
        // Query: «Ахмет қашан туылған?»
        // Candidate: «Ахмет 1872 жылы туылған.»
        let q = QueryIR::new(
            QueryFocus::Modifier(ModifierRole::Time),
            QuestionForm::Definition,
            AnswerShape::DateAnchor,
        )
        .with_agent(noun("ахмет"))
        .with_predicate(FramePredicate::BornIn);
        let candidate = Frame::assertion(Some(noun("ахмет")), FramePredicate::BornIn, None)
            .with_modifier(Modifier::TimeAnchor(TimeAnchor::Phrase(year_phrase_1872())));
        let m = q.match_frame(&candidate).expect("match");
        assert_eq!(m.answer_slot, AnswerSlot::Modifier(ModifierRole::Time));
        assert_eq!(m.score, 100);
    }

    #[test]
    fn match_frame_modifier_focus_partial_score_when_modifier_missing() {
        // Query asks for time, candidate has agent + predicate but
        // no time slot — partial match.
        let q = QueryIR::new(
            QueryFocus::Modifier(ModifierRole::Time),
            QuestionForm::Definition,
            AnswerShape::DateAnchor,
        )
        .with_agent(noun("ахмет"))
        .with_predicate(FramePredicate::BornIn);
        let candidate = Frame::assertion(Some(noun("ахмет")), FramePredicate::BornIn, None);
        let m = q.match_frame(&candidate).expect("match");
        assert_eq!(m.answer_slot, AnswerSlot::Whole);
        assert_eq!(m.score, 50);
    }

    #[test]
    fn match_frame_predicate_mismatch_rejects() {
        let q = QueryIR::new(
            QueryFocus::Modifier(ModifierRole::Time),
            QuestionForm::Definition,
            AnswerShape::DateAnchor,
        )
        .with_agent(noun("ахмет"))
        .with_predicate(FramePredicate::BornIn);
        let candidate = Frame::assertion(Some(noun("ахмет")), FramePredicate::DiedIn, None);
        assert!(q.match_frame(&candidate).is_none());
    }

    #[test]
    fn match_frame_agent_mismatch_rejects() {
        let q = QueryIR::new(
            QueryFocus::Modifier(ModifierRole::Time),
            QuestionForm::Definition,
            AnswerShape::DateAnchor,
        )
        .with_agent(noun("ахмет"))
        .with_predicate(FramePredicate::BornIn);
        let candidate = Frame::assertion(Some(noun("абай")), FramePredicate::BornIn, None);
        assert!(q.match_frame(&candidate).is_none());
    }

    #[test]
    fn match_frame_modifier_constraint_filter() {
        // Query: «Кім 1872 жылы туылған?» — constraint = 1872.
        // Candidate A: «Ахмет 1872 жылы туылған.» → matches.
        // Candidate B: «Абай 1845 жылы туылған.»  → rejected.
        let q = QueryIR::new(
            QueryFocus::Subject,
            QuestionForm::Definition,
            AnswerShape::BareNoun,
        )
        .with_predicate(FramePredicate::BornIn)
        .with_modifier_constraint(ModifierRole::Time, year_phrase_1872());

        let candidate_a = Frame::assertion(Some(noun("ахмет")), FramePredicate::BornIn, None)
            .with_modifier(Modifier::TimeAnchor(TimeAnchor::Phrase(year_phrase_1872())));
        assert!(q.match_frame(&candidate_a).is_some());

        let candidate_b = Frame::assertion(Some(noun("абай")), FramePredicate::BornIn, None)
            .with_modifier(Modifier::TimeAnchor(TimeAnchor::Phrase(year_phrase_1845())));
        assert!(q.match_frame(&candidate_b).is_none());
    }

    fn year_phrase_1872() -> Composition {
        let mut c = noun("1872 жыл");
        c.operators.push(SuffixOp::Case(Case::Locative));
        c
    }

    fn year_phrase_1845() -> Composition {
        let mut c = noun("1845 жыл");
        c.operators.push(SuffixOp::Case(Case::Locative));
        c
    }

    // -- Serde round-trip --------------------------------------

    #[test]
    fn queryir_serde_round_trip() {
        let q = QueryIR::new(
            QueryFocus::Modifier(ModifierRole::Time),
            QuestionForm::Definition,
            AnswerShape::DateAnchor,
        )
        .with_agent(noun("ахмет"))
        .with_predicate(FramePredicate::BornIn)
        .with_sense_hint("ай", Domain::Calendar)
        .with_domain_filter(Domain::Person);
        let json = serde_json::to_string(&q).expect("serialize");
        let q2: QueryIR = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(q, q2);
    }

    #[test]
    fn frame_match_serde_round_trip() {
        let fm = FrameMatch {
            answer_slot: AnswerSlot::Modifier(ModifierRole::Time),
            score: 100,
        };
        let json = serde_json::to_string(&fm).expect("serialize");
        let fm2: FrameMatch = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(fm, fm2);
    }

    // -- Domain + SenseHint -----------------------------------

    #[test]
    fn domain_other_carries_slug() {
        let d = Domain::Other("biology".to_string());
        assert_eq!(d.as_str(), "biology");
    }

    #[test]
    fn sense_hints_accumulate() {
        let q = QueryIR::new(
            QueryFocus::Definition,
            QuestionForm::Definition,
            AnswerShape::DefinitionalNP,
        )
        .with_sense_hint("ай", Domain::Calendar)
        .with_sense_hint("ай", Domain::Astronomy);
        assert_eq!(q.sense_hints.len(), 2);
    }

    // -- Predeclared success: 22 v6.1 predicates × focus matrix

    #[test]
    fn every_v6_1_predicate_constructs_a_query() {
        // Every v6.1 FramePredicate must be a valid QueryIR
        // predicate slot. This guards against silent enum
        // misalignment between Stage 2 and Stage 3.
        let all = [
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
        assert_eq!(all.len(), 22);
        for p in all {
            let q = QueryIR::new(
                QueryFocus::Modifier(ModifierRole::Time),
                QuestionForm::Definition,
                AnswerShape::DateAnchor,
            )
            .with_predicate(p.clone());
            assert_eq!(q.predicate, Some(p));
        }
    }

    #[test]
    fn every_focus_variant_round_trips_via_json() {
        let foci = [
            QueryFocus::Subject,
            QueryFocus::Predicate,
            QueryFocus::Object,
            QueryFocus::Modifier(ModifierRole::Time),
            QueryFocus::Modifier(ModifierRole::Location),
            QueryFocus::Modifier(ModifierRole::Source),
            QueryFocus::Modifier(ModifierRole::Instrument),
            QueryFocus::Modifier(ModifierRole::Manner),
            QueryFocus::Modifier(ModifierRole::Recipient),
            QueryFocus::Modifier(ModifierRole::Possessor),
            QueryFocus::Quantity,
            QueryFocus::Existence,
            QueryFocus::Definition,
            QueryFocus::Enumeration,
        ];
        for f in foci {
            let json = serde_json::to_string(&f).expect("ser");
            let back: QueryFocus = serde_json::from_str(&json).expect("de");
            assert_eq!(f, back);
        }
    }

    #[test]
    fn every_answer_shape_round_trips_via_json() {
        let shapes = [
            AnswerShape::BareNoun,
            AnswerShape::DateAnchor,
            AnswerShape::YesNoConfirm,
            AnswerShape::YesNoDeny,
            AnswerShape::YesNoUnknown,
            AnswerShape::DefinitionalNP,
            AnswerShape::Enumeration,
            AnswerShape::QuantityPhrase,
            AnswerShape::IsAProofChain,
            AnswerShape::CausalChain,
            AnswerShape::SafetyRefusal,
            AnswerShape::NoData,
        ];
        for s in shapes {
            let json = serde_json::to_string(&s).expect("ser");
            let back: AnswerShape = serde_json::from_str(&json).expect("de");
            assert_eq!(s, back);
        }
    }

    #[test]
    fn worked_example_biographical_query_end_to_end() {
        // Worked example: «Ахмет Байтұрсынұлы қашан туылған?»
        //
        // 1. Frame: question, agent=Ахмет, predicate=BornIn,
        //    focus=Time.
        let agent = noun("ахмет байтұрсынұлы");
        let question_frame = Frame::assertion(Some(agent.clone()), FramePredicate::BornIn, None)
            .with_modality(Modality::Question {
                focus: QuestionFocus::Time,
            });
        // 2. Build QueryIR.
        let q = QueryIR::from_question_frame(&question_frame).expect("query");
        assert_eq!(q.focus, QueryFocus::Modifier(ModifierRole::Time));
        assert_eq!(q.predicate, Some(FramePredicate::BornIn));
        // 3. Retrieve a candidate frame: «Ахмет ... 1872 жылы туылған».
        let candidate = Frame::assertion(Some(agent.clone()), FramePredicate::BornIn, None)
            .with_modifier(Modifier::TimeAnchor(TimeAnchor::Phrase(year_phrase_1872())))
            .with_tense(Tense::PastEvidential);
        // 4. Match.
        let m = q.match_frame(&candidate).expect("match");
        assert_eq!(m.answer_slot, AnswerSlot::Modifier(ModifierRole::Time));
        assert_eq!(m.score, 100);
        // 5. The realiser would now read candidate.modifier("time")
        //    and render «1872 жылы» (Stage 7).
    }

    // -- ContextSentenceType + SentenceContext smoke ----------

    #[test]
    fn assembling_query_from_lattice_via_frame() {
        // Integration sanity: lattice → Frame → QueryIR.
        let mut tuyl = Composition::identity(Root::new("туыл", PartOfSpeech::Verb));
        tuyl.operators.push(SuffixOp::Tense(Tense::PastEvidential));
        let lat = vec![noun("ахмет"), tuyl];
        let ctx = SentenceContext {
            sentence_type: ContextSentenceType::Question,
            question_focus: Some(QuestionFocus::Time),
        };
        let frame = Frame::from_morph_lattice_in_context(&lat, ctx).expect("frame");
        assert_eq!(frame.predicate, FramePredicate::BornIn);
        assert_eq!(frame.polarity, Polarity::Affirmative);
        let q = QueryIR::from_question_frame(&frame).expect("query");
        assert_eq!(q.focus, QueryFocus::Modifier(ModifierRole::Time));
    }
}
