//! **v4.31.0 — Morphemic-logical IR (SemFrame).**
//!
//! Unified, typed semantic frame that combines a word's morphological
//! features (root, POS, case, number, tense, person, possessive)
//! with its logical features (polarity, modality, evidential, relation
//! kind) in one structured object. Designed to flow through the entire
//! pipeline downstream of FST analysis as the only structured
//! representation of word-level semantics.
//!
//! ## Why
//!
//! Pre-v4.31.0 the pipeline carried *two* parallel representations:
//! `parser::Analysis` (rich morphology — root + NounFeatures /
//! VerbFeatures with case/tense/person/etc) and `adam_reasoning::SlotRef`
//! (lemma-only — `{ surface, root, pos }`). FST's morphological detail
//! died at the `SlotRef` boundary: case, tense, negation, evidential
//! status — all of it was reduced to a string. Downstream consumers
//! that needed any of those features had to re-parse or guess.
//!
//! `SemFrame` closes that gap: a single typed object that preserves
//! every morphological feature AND extends with logical features
//! (polarity, modality, evidence) that the planner / reasoner /
//! verbalizer need but `Analysis` doesn't directly carry.
//!
//! ## Pipeline shape
//!
//! ```text
//!   FST::analyse(word) -> Vec<Analysis>
//!                          |
//!                          v
//!   SemFrame::from_analysis(&Analysis) -> SemFrame
//!                          |
//!                          v   (consumers added in v4.31.5+)
//!     - SearchGraph query (case/polarity-aware)
//!     - Reasoner (negation-aware derivations, evidence-tracked chains)
//!     - Template verbalizer (richer slot-fill via feature tokens)
//!     - Trace emission (--trace shows full SemFrame per token)
//! ```
//!
//! ## v4.31.0 scope
//!
//! Substrate only — type + lossless conversion from `Analysis` +
//! comprehensive unit tests. No consumers wired yet; that's
//! v4.31.5 work. Deliberately follows the v4.0.37 → v4.0.38
//! "substrate first, behaviour second" pattern: ship the
//! architectural primitive in one reviewable patch, then migrate
//! consumers one at a time so each migration is independently
//! testable.
//!
//! ## What's lossless from Analysis
//!
//! Every existing morphological feature on `Analysis` flows through
//! to `SemFrame` without loss:
//!
//! | Analysis field | SemFrame field |
//! |---|---|
//! | `root.root` | `root` (String) |
//! | `root.part_of_speech` | `pos` (PosTag) |
//! | `NounFeatures.case` | `case: Option<Case>` |
//! | `NounFeatures.number` | `number: Option<Number>` |
//! | `NounFeatures.possessive` | `possessive: Option<Possessive>` |
//! | `NounFeatures.predicate` | `predicate: Option<Predicate>` |
//! | `NounFeatures.derivation` | `derivation: Option<Derivation>` |
//! | `VerbFeatures.tense` | `tense: Option<Tense>` |
//! | `VerbFeatures.person` | `person: Option<Person>` |
//! | `VerbFeatures.number` | `number: Option<Number>` |
//! | `VerbFeatures.voice` | `voice: Option<Voice>` |
//! | `VerbFeatures.negation` | `polarity` (Affirmative / Negated) |
//! | `VerbFeatures.polite` | `polite: bool` |
//!
//! ## What's NEW in SemFrame
//!
//! - `polarity`: unified across nouns and verbs. For verbs, derived
//!   from `VerbFeatures.negation`. For nouns, defaults to
//!   `Affirmative` (multi-word negation like «мұғалім емес» needs
//!   sentence-level analysis to detect; v4.32+ work).
//! - `modality: Option<Modality>`: ability / necessity / possibility,
//!   typically realised in Kazakh through periphrastic constructions
//!   («-а ал-», «-у керек», «-у мүмкін»). v4.31.0 leaves this `None`
//!   — auto-extraction needs sentence-level analysis (v4.32+).
//! - `evidence: Option<EvidenceKind>`: direct / hearsay / inferred.
//!   Auto-derived from `Tense::PastEvidential` (the `-{Y}п(ты)`
//!   reportative form) → `Hearsay`. Other tenses leave it `None`
//!   meaning "no evidential marker present".
//! - `relation: Option<RelationKind>`: when the frame represents a
//!   triple-arc rather than a noun-phrase head, this names the
//!   relation. v4.31.0 leaves it `None`; populated by future
//!   pattern matchers (v4.32+).
//!
//! ## Determinism
//!
//! `From<&Analysis>` is a pure function — no randomness, no side
//! effects, no allocation beyond cloning the root String. Two calls
//! with the same input return bit-identical output.

use crate::morphotactics::{Case, Derivation, Number, Person, Possessive, Predicate, Tense, Voice};
use crate::parser::Analysis;
use serde::{Deserialize, Serialize};

/// Part-of-speech tag, projected from `RootEntry::part_of_speech`.
/// Closed set — extending requires migrating downstream consumers
/// that match exhaustively on this enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PosTag {
    Noun,
    Adjective,
    Pronoun,
    Numeral,
    Verb,
    /// Adverb, postposition, conjunction, particle — minor classes
    /// the FST analyser groups together under "bare-only" (no
    /// inflection). Distinguished from `Noun` so consumers can
    /// reason about "this is a content word" vs "this is a function
    /// word" without re-checking the original string.
    Function,
}

impl PosTag {
    /// Project the lexicon's freeform `part_of_speech` string into
    /// the closed `PosTag` enum. Matches the same dispatch logic
    /// used in `parser::analyse`. Unknown strings fall back to
    /// `Function` — preserves "bare-only" semantics.
    fn from_lexicon_str(pos: &str) -> Self {
        match pos {
            "noun" => Self::Noun,
            "adjective" => Self::Adjective,
            "pronoun" => Self::Pronoun,
            "numeral" => Self::Numeral,
            "verb" => Self::Verb,
            _ => Self::Function,
        }
    }
}

/// Polarity — affirmative or negated. Unified across nouns and
/// verbs. For verbs, derived from `VerbFeatures.negation`. For
/// nouns, currently always `Affirmative` — sentence-level negation
/// detection («мұғалім емес» / «X емес») is v4.32+ work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Polarity {
    #[default]
    Affirmative,
    Negated,
}

/// Modality — kind of grammatical modal frame the word participates
/// in. In Kazakh these are typically periphrastic (multi-word):
/// `-а ал-` (ability), `-у керек` (necessity), `-у мүмкін`
/// (possibility). v4.31.0 leaves this `None` on every frame —
/// auto-extraction requires sentence-level analysis. Future
/// releases (v4.32+) will populate it from periphrastic-construction
/// matchers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Modality {
    /// «-а ал-» — "can / be able to"
    Ability,
    /// «-у керек» / «-у тиіс» — "must / should"
    Necessity,
    /// «-у мүмкін» — "may / might / possible"
    Possibility,
}

/// Evidential — what kind of source backs the proposition.
///
/// Kazakh marks evidentiality grammatically via `Tense::PastEvidential`
/// (the `-{Y}п(ты)` reportative form: "they say X V-ed", "apparently
/// X V-ed"). Other tenses don't carry an explicit evidential marker
/// — those frames leave `evidence` as `None` (meaning "no evidential
/// distinction expressed").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceKind {
    /// Direct knowledge — speaker witnessed the event.
    Direct,
    /// Hearsay — speaker heard about it (reportative). In Kazakh,
    /// `-{Y}п(ты)` past-evidential and `екен` particle.
    Hearsay,
    /// Inferred from indirect evidence (logical inference, derived
    /// from a rule chain, etc.).
    Inferred,
}

/// Relation — when the frame represents a typed semantic arc rather
/// than a noun-phrase head. Mirrors the `adam_reasoning::Predicate`
/// closed set, but kept here so `adam-kernel-fst` doesn't need to
/// depend on `adam-reasoning` (one-way dep direction preserved).
///
/// v4.31.0 leaves `relation` `None` on every frame — populated by
/// future pattern matchers (v4.32+) when SemFrame becomes the input
/// to extract_facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationKind {
    IsA,
    LivesIn,
    GoesTo,
    Has,
    HasQuantity,
    PartOf,
    RelatedTo,
    Causes,
    DoesTo,
    InDomain,
    After,
}

/// **The morphemic-logical intermediate representation.**
///
/// One structured frame per word. Lossless wrapper of `Analysis`
/// with three logical-feature extensions (polarity, modality,
/// evidence) and one structural extension (relation, populated by
/// future pattern matchers).
///
/// All fields are `Option<…>` where the morphology may or may not
/// mark the feature, except:
/// - `root`, `pos`, `polarity`: always present.
/// - `polite: bool`: defaults to `false`.
///
/// Constructed via `SemFrame::from_analysis(&analysis)` for
/// FST-derived frames. Sentence-level constructors (with
/// modality/relation populated) come in v4.32+.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemFrame {
    /// Lemma — the lexicon root the surface form was analysed
    /// against. Always lowercase per the lexicon's storage
    /// convention.
    pub root: String,

    /// Part-of-speech tag, projected from the lexicon's freeform
    /// `part_of_speech` field into a closed enum.
    pub pos: PosTag,

    /// Grammatical case (Nominative / Genitive / Dative / Accusative
    /// / Locative / Ablative / Instrumental / LocativeAttributive).
    /// `None` for verbs and bare nouns (Nominative is also
    /// represented as `None` in the source `NounFeatures.case`).
    pub case: Option<Case>,

    /// Number (Singular / Plural). `None` when the source
    /// `NounFeatures.number` / `VerbFeatures.number` is unmarked.
    pub number: Option<Number>,

    /// Possessive marker on nouns (P1Sg / P2SgInformal / P2SgPolite
    /// / P3 / P1Pl / P2PlInformal / P2PlPolite). `None` for verbs
    /// and bare nouns.
    pub possessive: Option<Possessive>,

    /// Predicate-person copula on noun forms (мұғаліммін / мұғалімсіз
    /// etc.). `None` for verbs and non-predicative nouns. Mutually
    /// exclusive with `possessive` in standard Kazakh grammar.
    pub predicate: Option<Predicate>,

    /// Derivational suffix that built the lemma (e.g. agent
    /// nominaliser, profession suffix). `None` when the lemma is
    /// non-derived.
    pub derivation: Option<Derivation>,

    /// Tense (PastDefinite / PastEvidential / Present / FutureIntentional
    /// / etc., plus participle and converb forms). `None` for
    /// nouns; non-`None` only for verbs.
    pub tense: Option<Tense>,

    /// Person (First / Second / Third). `None` for nouns and
    /// non-finite verb forms.
    pub person: Option<Person>,

    /// Voice (Active / Passive / Reflexive / Reciprocal /
    /// Causative). `None` for nouns; non-`None` only for verbs.
    pub voice: Option<Voice>,

    /// **NEW in v4.31.0** — unified polarity. Affirmative or
    /// Negated. Derived from `VerbFeatures.negation` for verbs;
    /// always `Affirmative` for noun frames in v4.31.0 (sentence-
    /// level «X емес» negation detection is v4.32+).
    pub polarity: Polarity,

    /// `true` selects the polite (V-form) personal ending instead
    /// of the informal (T-form). Only meaningful for 2nd-person
    /// verb forms; `false` for everything else.
    pub polite: bool,

    /// **NEW in v4.31.0** — modality. Always `None` in v4.31.0;
    /// auto-extraction from periphrastic constructions is v4.32+.
    pub modality: Option<Modality>,

    /// **NEW in v4.31.0** — evidential marker. Auto-derived from
    /// `Tense::PastEvidential` → `Hearsay`. Other tenses leave
    /// this `None` (no explicit evidential distinction).
    pub evidence: Option<EvidenceKind>,

    /// **NEW in v4.31.0** — relation, when the frame represents a
    /// typed semantic arc rather than a noun-phrase head. Always
    /// `None` in v4.31.0; populated by future pattern matchers
    /// (v4.32+).
    pub relation: Option<RelationKind>,
}

impl SemFrame {
    /// Build a SemFrame from a single FST analysis. Lossless on
    /// every existing morphological feature; auto-derives polarity
    /// and evidence from existing fields.
    ///
    /// ```text
    ///   Analysis::Verb { negation: true, tense: PastEvidential, ... }
    ///   -> SemFrame { polarity: Negated, evidence: Some(Hearsay), ... }
    /// ```
    pub fn from_analysis(analysis: &Analysis) -> Self {
        match analysis {
            Analysis::Noun { root, features } => Self {
                root: root.root.clone(),
                pos: PosTag::from_lexicon_str(&root.part_of_speech),
                case: features.case,
                number: features.number,
                possessive: features.possessive,
                predicate: features.predicate,
                derivation: features.derivation,
                tense: None,
                person: None,
                voice: None,
                // Noun polarity is always Affirmative in v4.31.0 —
                // sentence-level «X емес» needs cross-token analysis.
                polarity: Polarity::Affirmative,
                polite: false,
                modality: None,
                // Nouns don't carry tense, so no evidential marker
                // is reachable via this path.
                evidence: None,
                relation: None,
            },
            Analysis::Verb { root, features } => {
                let polarity = if features.negation {
                    Polarity::Negated
                } else {
                    Polarity::Affirmative
                };
                let evidence = match features.tense {
                    Some(Tense::PastEvidential) => Some(EvidenceKind::Hearsay),
                    // **v4.36.5** — «-{Y}п(ты)» reportative past
                    // also marks Hearsay. Both surface forms encode
                    // reported speech / hearsay; downstream consumers
                    // (planner Hearsay routing, hedge templates)
                    // treat them identically.
                    Some(Tense::PastReportative) => Some(EvidenceKind::Hearsay),
                    _ => None,
                };
                Self {
                    root: root.root.clone(),
                    pos: PosTag::from_lexicon_str(&root.part_of_speech),
                    case: None,
                    number: features.number,
                    possessive: None,
                    predicate: None,
                    derivation: None,
                    tense: features.tense,
                    person: features.person,
                    voice: features.voice,
                    polarity,
                    polite: features.polite,
                    modality: None,
                    evidence,
                    relation: None,
                }
            }
        }
    }
}

impl From<&Analysis> for SemFrame {
    fn from(a: &Analysis) -> Self {
        Self::from_analysis(a)
    }
}

/// **v4.33.0** — sentence-level negation detection via «емес» particle.
///
/// In Kazakh, a noun, adjective, or pronoun is negated by following
/// it with the particle «емес» («not / is not»):
///
/// ```text
///   Бұл шындық емес       — "This is not the truth"
///   Ол ұзын емес          — "He/she is not tall"
///   Бұл мен емес          — "This is not me"
/// ```
///
/// When v4.31.0 introduced `Polarity` it was unified across nouns
/// and verbs but only auto-derived from `VerbFeatures.negation` —
/// noun frames defaulted to `Affirmative`. v4.33.0 closes the gap
/// for the canonical sentence-level pattern: a noun-class frame
/// followed by «емес» as its own SemFrame.
///
/// Detection — single linear scan:
///   for i in 1..frames.len():
///     if frames[i].root == "емес" and frames[i-1].pos is noun-like:
///       set frames[i-1].polarity = Negated
///
/// Noun-like POS includes `Noun`, `Adjective`, `Pronoun`, `Numeral`
/// — the four classes that take predicate-style negation.
///
/// **Known FST gap (v4.34+ work):** the predicate-copula inflected
/// forms «емеспін / емессің / емеспіз / емессіз» are NOT in the
/// SemFrame stream because `parser::analyse` dispatches `particle`
/// POS through the catch-all branch, which only emits an analysis
/// when `entry.root == surface`. The inflected forms simply don't
/// parse and get dropped by `parse_input_inner`. Detection here
/// works for bare «емес» («Бұл X емес»); inflected forms («Мен X
/// емеспін») are blocked until parser's particle dispatch is
/// extended to enumerate predicate copulas (mirror of how nouns
/// are handled). Documented as carry-forward.
///
/// First-detection-wins. Idempotent.
pub fn populate_sentential_negation(frames: &mut [SemFrame]) {
    if frames.len() < 2 {
        return;
    }
    for i in 1..frames.len() {
        if frames[i].root != "емес" {
            continue;
        }
        // Only mark noun-class frames. Skip verb / function /
        // preceding modal — negation by «емес» is a predicate
        // negator that attaches to noun-headed predicates.
        let prev_is_noun_like = matches!(
            frames[i - 1].pos,
            PosTag::Noun | PosTag::Adjective | PosTag::Pronoun | PosTag::Numeral
        );
        if !prev_is_noun_like {
            continue;
        }
        // First-detection-wins.
        if frames[i - 1].polarity == Polarity::Affirmative {
            frames[i - 1].polarity = Polarity::Negated;
        }
    }
}

/// **v4.32.5** — ability `-а ал-` detection.
///
/// Kazakh expresses ability/possibility periphrastically: a lexical
/// verb in **converb-imperfect** form (`-а / -е / -й` ending,
/// `Tense::ConverbImperfect` in the FST analysis) followed by the
/// auxiliary `ал` ("can / be able to") in some person/tense:
///
/// ```text
///   жаза алам         — "I can write"
///   жаза аламын       — "I can write" (longer 1sg form)
///   жаза алмаймын     — "I cannot write" (negated auxiliary)
///   оқи аласыз ба?    — "Can you read?"
///   сөйлей алады      — "(s)he can speak"
/// ```
///
/// **Why a dual-signal check.** The auxiliary `ал` is also a real
/// transitive verb meaning "to take/get". Naive surface detection
/// — "if `ал` follows another word, mark Ability" — would mis-mark
/// genuine sentences like «Кітапты алдым» ("I took the book") as
/// ability-modal. We require BOTH:
///   (a) the auxiliary's root is `ал` AND its POS is Verb, AND
///   (b) the immediately-preceding frame is a verb in
///       `ConverbImperfect` tense (the `-а / -е / -й` form).
/// Both signals together unambiguously identify the periphrastic
/// ability construction.
///
/// **Pre-v4.32.5 the dual-signal check was impossible.** The FST
/// parser's `tenses` enumeration didn't include `ConverbImperfect`
/// (only None / PastDefinite / PastEvidential / Present), so converb
/// forms like «жаза» didn't parse at all. v4.32.5 extends the
/// parser enumeration AND ships this detector together — the
/// matched architectural pair.
///
/// **Polarity note.** When the auxiliary is negated («алмаймын» —
/// "cannot"), the COMBINED meaning is "cannot V". In the SemFrame
/// stream this is recorded as:
///   - lexical predicate: modality=Ability, polarity=Affirmative
///   - auxiliary `ал`:    polarity=Negated (own negation flag)
/// Sentence-level negation aggregation across modal constructions
/// (collapsing the auxiliary's polarity onto the lexical predicate's
/// effective meaning) is v4.33+ work.
///
/// **First-detection-wins.** Doesn't overwrite modality already set
/// by `populate_periphrastic_modality` (which runs first in the
/// pipeline). Keeps the two detectors independent and safely
/// composable.
pub fn populate_ability_modality(frames: &mut [SemFrame]) {
    if frames.len() < 2 {
        return;
    }
    for i in 1..frames.len() {
        // Signal (a): current frame is the `ал` auxiliary as Verb.
        if frames[i].root != "ал" || frames[i].pos != PosTag::Verb {
            continue;
        }
        // Signal (b): preceding frame is a verb in ConverbImperfect tense.
        if frames[i - 1].tense != Some(Tense::ConverbImperfect) {
            continue;
        }
        // Both signals fired — this is the periphrastic ability
        // construction. Mark the lexical predicate (frame i-1).
        if frames[i - 1].modality.is_none() {
            frames[i - 1].modality = Some(Modality::Ability);
        }
    }
}

/// **v4.32.0** — periphrastic-modality detection.
///
/// Kazakh expresses modality NOT as a single inflectional suffix on
/// the main verb, but periphrastically — through dedicated modal
/// auxiliary words placed AFTER the lexical predicate:
///
/// | Auxiliary | Modality | Example |
/// |---|---|---|
/// | `керек` | Necessity ("must / need") | «жазу керек» — "must write" |
/// | `тиіс`  | Necessity ("ought to")    | «келу тиіс» — "ought to come" |
/// | `мүмкін` | Possibility ("may")      | «болу мүмкін» — "may be" |
///
/// All three modals share `part_of_speech: "modal"` in the lexicon
/// and parse as bare `Analysis::Noun` (the FST's catch-all branch
/// for non-noun-non-verb POS). They surface as `SemFrame.pos =
/// PosTag::Function`. We detect them by ROOT match (not POS) so
/// other Function-class words (postpositions like `туралы`,
/// particles) don't interfere.
///
/// Algorithm — single linear scan:
///   for i in 0..frames.len():
///     if frames[i].root in MODAL_AUXILIARIES and i > 0:
///       set frames[i-1].modality = corresponding Modality
///
/// The modality is recorded on the **lexical predicate** (frame
/// `i-1`), not on the auxiliary itself — that's the modal-bearer
/// in the canonical analysis. The auxiliary's own SemFrame is left
/// untouched (its `relation/modality/etc` stay `None`); future
/// passes may want to mark it as "consumed by the lexical
/// predicate's modal frame", but that's not required for v4.32.0.
///
/// Conservative: ability `-а ал-` is NOT detected here. The auxiliary
/// `ал` is also a transitive verb meaning "take/get", and naive
/// surface-form detection would mis-mark genuine "X took Y" sentences
/// as ability-modal. Proper ability detection needs the converb form
/// of the lexical verb (`-а / -е / -й`) AND a person-marked form of
/// `ал` — both signals checked together. Deferred to v4.32.5+.
///
/// Idempotent: running twice produces the same result. Safe to call
/// even on frames that already have modality set (won't overwrite
/// non-None modality — first detection wins).
pub fn populate_periphrastic_modality(frames: &mut [SemFrame]) {
    if frames.len() < 2 {
        return;
    }
    for i in 1..frames.len() {
        let modal = match frames[i].root.as_str() {
            "керек" | "тиіс" => Some(Modality::Necessity),
            "мүмкін" => Some(Modality::Possibility),
            _ => None,
        };
        let Some(m) = modal else { continue };
        // First detection wins — don't overwrite a previously-set
        // modality. Useful when a future pass adds ability detection
        // and we want the first match to take effect.
        if frames[i - 1].modality.is_none() {
            frames[i - 1].modality = Some(m);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexicon::RootEntry;
    use crate::morphotactics::{NounFeatures, VerbFeatures};

    fn noun_root(root: &str) -> RootEntry {
        RootEntry {
            id: format!("test_{root}"),
            root: root.to_string(),
            part_of_speech: "noun".to_string(),
            vowel_harmony: "back".to_string(),
            final_sound_class: "vowel".to_string(),
        }
    }

    fn verb_root(root: &str) -> RootEntry {
        RootEntry {
            id: format!("test_{root}"),
            root: root.to_string(),
            part_of_speech: "verb".to_string(),
            vowel_harmony: "back".to_string(),
            final_sound_class: "consonant".to_string(),
        }
    }

    /// **Lossless contract — Noun.** Every NounFeatures field
    /// must round-trip to the corresponding SemFrame field.
    #[test]
    fn noun_roundtrip_preserves_all_fields() {
        let analysis = Analysis::Noun {
            root: noun_root("қала"),
            features: NounFeatures {
                derivation: None,
                number: Some(Number::Plural),
                possessive: Some(Possessive::P1Sg),
                case: Some(Case::Locative),
                predicate: None,
            },
        };
        let frame = SemFrame::from_analysis(&analysis);
        assert_eq!(frame.root, "қала");
        assert_eq!(frame.pos, PosTag::Noun);
        assert_eq!(frame.case, Some(Case::Locative));
        assert_eq!(frame.number, Some(Number::Plural));
        assert_eq!(frame.possessive, Some(Possessive::P1Sg));
        assert_eq!(frame.predicate, None);
        assert_eq!(frame.derivation, None);
        // Verb-side fields stay None on noun frames.
        assert_eq!(frame.tense, None);
        assert_eq!(frame.person, None);
        assert_eq!(frame.voice, None);
        // v4.31.0 NEW fields — defaults for noun.
        assert_eq!(frame.polarity, Polarity::Affirmative);
        assert!(!frame.polite);
        assert_eq!(frame.modality, None);
        assert_eq!(frame.evidence, None);
        assert_eq!(frame.relation, None);
    }

    /// **Lossless contract — Verb.** Every VerbFeatures field must
    /// round-trip to the corresponding SemFrame field.
    #[test]
    fn verb_roundtrip_preserves_all_fields() {
        let analysis = Analysis::Verb {
            root: verb_root("жаз"),
            features: VerbFeatures {
                voice: Some(Voice::Active),
                negation: false,
                tense: Some(Tense::PastDefinite),
                person: Some(Person::First),
                number: Some(Number::Singular),
                polite: false,
            },
        };
        let frame = SemFrame::from_analysis(&analysis);
        assert_eq!(frame.root, "жаз");
        assert_eq!(frame.pos, PosTag::Verb);
        assert_eq!(frame.tense, Some(Tense::PastDefinite));
        assert_eq!(frame.person, Some(Person::First));
        assert_eq!(frame.number, Some(Number::Singular));
        assert_eq!(frame.voice, Some(Voice::Active));
        assert!(!frame.polite);
        // Noun-side fields stay None on verb frames.
        assert_eq!(frame.case, None);
        assert_eq!(frame.possessive, None);
        assert_eq!(frame.predicate, None);
        assert_eq!(frame.derivation, None);
        // v4.31.0 NEW fields — defaults for plain affirmative verb.
        assert_eq!(frame.polarity, Polarity::Affirmative);
        assert_eq!(frame.modality, None);
        assert_eq!(frame.evidence, None);
        assert_eq!(frame.relation, None);
    }

    /// **NEW v4.31.0 logic — polarity from VerbFeatures.negation.**
    /// `жазбадым` (I did NOT write) → polarity Negated.
    #[test]
    fn verb_negation_maps_to_polarity_negated() {
        let analysis = Analysis::Verb {
            root: verb_root("жаз"),
            features: VerbFeatures {
                voice: Some(Voice::Active),
                negation: true,
                tense: Some(Tense::PastDefinite),
                person: Some(Person::First),
                number: Some(Number::Singular),
                polite: false,
            },
        };
        let frame = SemFrame::from_analysis(&analysis);
        assert_eq!(frame.polarity, Polarity::Negated);
    }

    /// **NEW v4.31.0 logic — evidence from PastEvidential tense.**
    /// `жазыпты` (apparently/they-say wrote) → evidence Hearsay.
    /// Other tenses leave evidence None.
    #[test]
    fn past_evidential_tense_maps_to_evidence_hearsay() {
        let evidential = Analysis::Verb {
            root: verb_root("жаз"),
            features: VerbFeatures {
                voice: Some(Voice::Active),
                negation: false,
                tense: Some(Tense::PastEvidential),
                person: Some(Person::Third),
                number: Some(Number::Singular),
                polite: false,
            },
        };
        let evidential_frame = SemFrame::from_analysis(&evidential);
        assert_eq!(evidential_frame.evidence, Some(EvidenceKind::Hearsay));

        let definite = Analysis::Verb {
            root: verb_root("жаз"),
            features: VerbFeatures {
                voice: Some(Voice::Active),
                negation: false,
                tense: Some(Tense::PastDefinite),
                person: Some(Person::Third),
                number: Some(Number::Singular),
                polite: false,
            },
        };
        let definite_frame = SemFrame::from_analysis(&definite);
        assert_eq!(definite_frame.evidence, None);
    }

    /// **NEW v4.31.0 — POS projection.** The freeform
    /// `part_of_speech` string from the lexicon must project to a
    /// PosTag enum variant; unknown strings collapse to `Function`.
    #[test]
    fn pos_projection_covers_known_classes() {
        for (raw, expected) in [
            ("noun", PosTag::Noun),
            ("adjective", PosTag::Adjective),
            ("pronoun", PosTag::Pronoun),
            ("numeral", PosTag::Numeral),
            ("verb", PosTag::Verb),
            ("adverb", PosTag::Function),
            ("postposition", PosTag::Function),
            ("conjunction", PosTag::Function),
            ("particle", PosTag::Function),
            ("undocumented_class", PosTag::Function),
        ] {
            assert_eq!(PosTag::from_lexicon_str(raw), expected, "raw={raw}");
        }
    }

    /// **NEW v4.31.0 — Polite verb form preserved.** v-form (polite)
    /// 2nd-person verbs must carry `polite=true` through the frame.
    #[test]
    fn polite_verb_form_preserved() {
        let analysis = Analysis::Verb {
            root: verb_root("жаз"),
            features: VerbFeatures {
                voice: Some(Voice::Active),
                negation: false,
                tense: Some(Tense::Present),
                person: Some(Person::Second),
                number: Some(Number::Plural),
                polite: true,
            },
        };
        let frame = SemFrame::from_analysis(&analysis);
        assert!(frame.polite);
    }

    /// **NEW v4.31.0 — From<&Analysis> trait equivalence.** The
    /// trait impl must produce bit-identical frames to the
    /// inherent `from_analysis` method.
    #[test]
    fn trait_impl_matches_inherent_method() {
        let analysis = Analysis::Verb {
            root: verb_root("көр"),
            features: VerbFeatures {
                voice: Some(Voice::Active),
                negation: true,
                tense: Some(Tense::PastEvidential),
                person: Some(Person::Third),
                number: Some(Number::Singular),
                polite: false,
            },
        };
        let via_inherent = SemFrame::from_analysis(&analysis);
        let via_trait: SemFrame = (&analysis).into();
        assert_eq!(via_inherent, via_trait);
    }

    /// **NEW v4.31.0 — Determinism.** Two From conversions on the
    /// same input must produce bit-identical output. Crucial for
    /// downstream pipeline determinism (per project_v4_direction
    /// memory: every layer's decision must be traceable + bit-
    /// identical run-to-run).
    #[test]
    fn from_analysis_is_deterministic() {
        let analysis = Analysis::Noun {
            root: noun_root("ел"),
            features: NounFeatures {
                derivation: None,
                number: None,
                possessive: Some(Possessive::P3),
                case: Some(Case::Genitive),
                predicate: None,
            },
        };
        let f1 = SemFrame::from_analysis(&analysis);
        let f2 = SemFrame::from_analysis(&analysis);
        let f3 = SemFrame::from_analysis(&analysis);
        assert_eq!(f1, f2);
        assert_eq!(f2, f3);
    }

    /// **v4.32.0** — periphrastic-modality detection sets
    /// `Modality::Necessity` on the lexical predicate when followed
    /// by «керек» auxiliary.
    ///
    /// Pattern: «жазу керек» — "must write".
    /// frames[0] = жазу (lexical predicate, verbal noun form).
    /// frames[1] = керек (modal auxiliary, POS=modal → Function tag).
    /// Expectation: frames[0].modality = Some(Necessity); frames[1]
    /// untouched.
    fn modal_root_function(root: &str) -> RootEntry {
        RootEntry {
            id: format!("test_modal_{root}"),
            root: root.to_string(),
            part_of_speech: "modal".to_string(),
            vowel_harmony: "front".to_string(),
            final_sound_class: "consonant".to_string(),
        }
    }

    fn bare_noun_frame(root: &str) -> SemFrame {
        SemFrame::from_analysis(&Analysis::Noun {
            root: noun_root(root),
            features: NounFeatures::default(),
        })
    }

    fn bare_modal_frame(root: &str) -> SemFrame {
        SemFrame::from_analysis(&Analysis::Noun {
            root: modal_root_function(root),
            features: NounFeatures::default(),
        })
    }

    #[test]
    fn periphrastic_modality_detects_kerek_as_necessity() {
        let mut frames = vec![bare_noun_frame("жазу"), bare_modal_frame("керек")];
        populate_periphrastic_modality(&mut frames);
        assert_eq!(frames[0].modality, Some(Modality::Necessity));
        // Modal auxiliary itself is left untouched.
        assert_eq!(frames[1].modality, None);
    }

    #[test]
    fn periphrastic_modality_detects_tiis_as_necessity() {
        let mut frames = vec![bare_noun_frame("келу"), bare_modal_frame("тиіс")];
        populate_periphrastic_modality(&mut frames);
        assert_eq!(frames[0].modality, Some(Modality::Necessity));
    }

    #[test]
    fn periphrastic_modality_detects_mumkin_as_possibility() {
        let mut frames = vec![bare_noun_frame("болу"), bare_modal_frame("мүмкін")];
        populate_periphrastic_modality(&mut frames);
        assert_eq!(frames[0].modality, Some(Modality::Possibility));
    }

    /// Without a modal auxiliary, all frames retain `modality: None`.
    /// Closes the no-false-positive contract: only specific roots
    /// (not POS) trigger detection.
    #[test]
    fn periphrastic_modality_no_match_leaves_all_none() {
        let mut frames = vec![
            bare_noun_frame("қазақстан"),
            bare_noun_frame("туралы"),
            bare_noun_frame("айт"),
        ];
        populate_periphrastic_modality(&mut frames);
        for frame in &frames {
            assert_eq!(frame.modality, None);
        }
    }

    /// Single-frame input is a degenerate case (no preceding
    /// predicate to attach modality to). Function must early-exit
    /// without panicking and without touching the lone frame.
    #[test]
    fn periphrastic_modality_single_frame_noop() {
        let mut frames = vec![bare_modal_frame("керек")];
        populate_periphrastic_modality(&mut frames);
        assert_eq!(frames[0].modality, None);
    }

    /// Sentence-initial modal — no preceding frame to receive
    /// the modality. Function must skip gracefully (modal at index 0
    /// has no `i-1`).
    #[test]
    fn periphrastic_modality_sentence_initial_modal_skipped() {
        let mut frames = vec![bare_modal_frame("керек"), bare_noun_frame("деген")];
        populate_periphrastic_modality(&mut frames);
        assert_eq!(frames[0].modality, None);
        assert_eq!(frames[1].modality, None);
    }

    /// Idempotence: running the detector twice produces the same
    /// output. Important for trace-replay scenarios where the same
    /// frames Vec might pass through the detector multiple times.
    #[test]
    fn periphrastic_modality_is_idempotent() {
        let mut frames = vec![bare_noun_frame("жазу"), bare_modal_frame("керек")];
        populate_periphrastic_modality(&mut frames);
        let after_first = frames.clone();
        populate_periphrastic_modality(&mut frames);
        assert_eq!(frames, after_first);
    }

    /// First-detection-wins: if a frame already has `modality` set
    /// (e.g. from a future ability detector running first), the
    /// periphrastic pass must not overwrite it.
    #[test]
    fn periphrastic_modality_preserves_preset_modality() {
        let mut frames = vec![bare_noun_frame("жазу"), bare_modal_frame("керек")];
        // Simulate a future ability detector having run first.
        frames[0].modality = Some(Modality::Ability);
        populate_periphrastic_modality(&mut frames);
        // First detection wins — Ability not overwritten by Necessity.
        assert_eq!(frames[0].modality, Some(Modality::Ability));
    }

    /// **v4.32.5** — helper to build an ability-detection test:
    /// lexical verb in ConverbImperfect tense + auxiliary `ал` as Verb.
    fn converb_verb_frame(root: &str) -> SemFrame {
        SemFrame::from_analysis(&Analysis::Verb {
            root: verb_root(root),
            features: VerbFeatures {
                voice: None,
                negation: false,
                tense: Some(Tense::ConverbImperfect),
                person: None,
                number: None,
                polite: false,
            },
        })
    }

    fn finite_al_frame() -> SemFrame {
        SemFrame::from_analysis(&Analysis::Verb {
            root: verb_root("ал"),
            features: VerbFeatures {
                voice: None,
                negation: false,
                tense: Some(Tense::Present),
                person: Some(Person::First),
                number: Some(Number::Singular),
                polite: false,
            },
        })
    }

    /// **v4.32.5** — canonical ability case: «жаза алам».
    /// Lexical verb in ConverbImperfect + auxiliary `ал` (Verb) →
    /// Modality::Ability on the lexical verb's frame.
    #[test]
    fn ability_modality_detects_canonical_periphrastic_able() {
        let mut frames = vec![converb_verb_frame("жаз"), finite_al_frame()];
        populate_ability_modality(&mut frames);
        assert_eq!(frames[0].modality, Some(Modality::Ability));
        // Auxiliary itself untouched.
        assert_eq!(frames[1].modality, None);
    }

    /// Negated auxiliary («алмаймын» — "cannot") is still ability.
    /// Polarity is recorded on the auxiliary's own frame (not on the
    /// lexical predicate); sentence-level polarity aggregation is
    /// v4.33+ work.
    #[test]
    fn ability_modality_fires_when_auxiliary_negated() {
        let mut frames = vec![
            converb_verb_frame("жаз"),
            SemFrame::from_analysis(&Analysis::Verb {
                root: verb_root("ал"),
                features: VerbFeatures {
                    voice: None,
                    negation: true,
                    tense: Some(Tense::Present),
                    person: Some(Person::First),
                    number: Some(Number::Singular),
                    polite: false,
                },
            }),
        ];
        populate_ability_modality(&mut frames);
        assert_eq!(frames[0].modality, Some(Modality::Ability));
        // Auxiliary's own polarity reflects the negation.
        assert_eq!(frames[1].polarity, Polarity::Negated);
    }

    /// **No false positive on literal "X took Y".** When the
    /// preceding frame is a noun (not a converb-imperfect verb),
    /// `ал` is just the regular transitive verb meaning "take/get"
    /// and ability detection must NOT fire.
    #[test]
    fn ability_modality_no_false_positive_on_literal_take() {
        // «Кітапты алдым» — "I took the book". Frame 0 is the noun
        // кітап (in accusative), frame 1 would be «ал» as a regular
        // verb. No converb signal — detection must skip.
        let mut frames = vec![
            SemFrame::from_analysis(&Analysis::Noun {
                root: noun_root("кітап"),
                features: NounFeatures {
                    derivation: None,
                    number: None,
                    possessive: None,
                    case: Some(Case::Accusative),
                    predicate: None,
                },
            }),
            finite_al_frame(),
        ];
        populate_ability_modality(&mut frames);
        // Noun frames have no `tense` — and the contract says a verb
        // in ConverbImperfect must precede `ал`. So no Ability set.
        assert_eq!(frames[0].modality, None);
        assert_eq!(frames[1].modality, None);
    }

    /// **No false positive when preceding verb is finite.** «Келдім
    /// алдым» (hypothetical sequence) — frame 0 is `кел` in
    /// PastDefinite, frame 1 is `ал` finite. Not a converb chain.
    #[test]
    fn ability_modality_no_false_positive_on_finite_chain() {
        let mut frames = vec![
            SemFrame::from_analysis(&Analysis::Verb {
                root: verb_root("кел"),
                features: VerbFeatures {
                    voice: None,
                    negation: false,
                    tense: Some(Tense::PastDefinite),
                    person: Some(Person::First),
                    number: Some(Number::Singular),
                    polite: false,
                },
            }),
            finite_al_frame(),
        ];
        populate_ability_modality(&mut frames);
        assert_eq!(frames[0].modality, None);
        assert_eq!(frames[1].modality, None);
    }

    /// Single-frame input is a degenerate case (no preceding
    /// predicate to attach Ability to). Function must early-exit
    /// without panicking.
    #[test]
    fn ability_modality_single_frame_noop() {
        let mut frames = vec![finite_al_frame()];
        populate_ability_modality(&mut frames);
        assert_eq!(frames[0].modality, None);
    }

    /// First-detection-wins: when periphrastic detector has already
    /// set Necessity (e.g. ability auxiliary is itself part of a
    /// nested «жаза ала керек» — hypothetical edge case), the
    /// pre-existing modality is preserved.
    #[test]
    fn ability_modality_preserves_preset_modality() {
        let mut frames = vec![converb_verb_frame("жаз"), finite_al_frame()];
        // Simulate periphrastic detector having marked Necessity first.
        frames[0].modality = Some(Modality::Necessity);
        populate_ability_modality(&mut frames);
        // First detection wins — Necessity not overwritten by Ability.
        assert_eq!(frames[0].modality, Some(Modality::Necessity));
    }

    /// Idempotence: running the ability detector twice produces the
    /// same result.
    #[test]
    fn ability_modality_is_idempotent() {
        let mut frames = vec![converb_verb_frame("жаз"), finite_al_frame()];
        populate_ability_modality(&mut frames);
        let after_first = frames.clone();
        populate_ability_modality(&mut frames);
        assert_eq!(frames, after_first);
    }

    /// **v4.33.0** — sentence-level negation: noun frame followed by
    /// «емес» particle should be marked Negated.
    fn emes_particle_frame() -> SemFrame {
        SemFrame::from_analysis(&Analysis::Noun {
            // particle POS projects to PosTag::Function via from_lexicon_str.
            root: RootEntry {
                id: "test_part_emes".into(),
                root: "емес".into(),
                part_of_speech: "particle".into(),
                vowel_harmony: "front".into(),
                final_sound_class: "voiceless_consonant".into(),
            },
            features: NounFeatures::default(),
        })
    }

    #[test]
    fn sentential_negation_sets_negated_on_prior_noun() {
        let mut frames = vec![bare_noun_frame("шындық"), emes_particle_frame()];
        populate_sentential_negation(&mut frames);
        assert_eq!(frames[0].polarity, Polarity::Negated);
        // The «емес» particle's own polarity is untouched
        // (it's the negator, not the thing-being-negated).
        assert_eq!(frames[1].polarity, Polarity::Affirmative);
    }

    /// «Бұл X емес» — leading pronoun + content noun + emes.
    /// Negation must attach to the immediately-preceding noun
    /// (frame[1]=шындық), not to the pronoun (frame[0]=бұл).
    #[test]
    fn sentential_negation_attaches_to_immediate_predecessor() {
        let mut frames = vec![
            // бұл — pronoun
            SemFrame::from_analysis(&Analysis::Noun {
                root: RootEntry {
                    id: "test_pron_bul".into(),
                    root: "бұл".into(),
                    part_of_speech: "pronoun".into(),
                    vowel_harmony: "front".into(),
                    final_sound_class: "voiceless_consonant".into(),
                },
                features: NounFeatures::default(),
            }),
            bare_noun_frame("шындық"),
            emes_particle_frame(),
        ];
        populate_sentential_negation(&mut frames);
        // Pronoun «бұл» stays Affirmative.
        assert_eq!(frames[0].polarity, Polarity::Affirmative);
        // Noun «шындық» (immediate predecessor) is Negated.
        assert_eq!(frames[1].polarity, Polarity::Negated);
    }

    /// Verb frame preceding «емес» is NOT marked. Verb negation goes
    /// through `VerbFeatures.negation` → `polarity` on the verb
    /// frame itself; sentential «емес» applies only to noun-class
    /// predicates. Avoids stomping on auto-derived verb polarity.
    #[test]
    fn sentential_negation_skips_verb_frame() {
        let mut frames = vec![
            SemFrame::from_analysis(&Analysis::Verb {
                root: verb_root("жаз"),
                features: VerbFeatures {
                    voice: None,
                    negation: false,
                    tense: Some(Tense::PastDefinite),
                    person: Some(Person::Third),
                    number: Some(Number::Singular),
                    polite: false,
                },
            }),
            emes_particle_frame(),
        ];
        populate_sentential_negation(&mut frames);
        // Verb frame stays Affirmative.
        assert_eq!(frames[0].polarity, Polarity::Affirmative);
    }

    /// No false positive when there's no «емес» in the stream.
    #[test]
    fn sentential_negation_no_match_leaves_all_affirmative() {
        let mut frames = vec![
            bare_noun_frame("қазақстан"),
            bare_noun_frame("туралы"),
            bare_noun_frame("айт"),
        ];
        populate_sentential_negation(&mut frames);
        for frame in &frames {
            assert_eq!(frame.polarity, Polarity::Affirmative);
        }
    }

    /// Single «емес» frame at index 0 — no preceding noun to negate.
    /// Function must early-exit gracefully.
    #[test]
    fn sentential_negation_emes_at_position_zero_skipped() {
        let mut frames = vec![emes_particle_frame()];
        populate_sentential_negation(&mut frames);
        assert_eq!(frames[0].polarity, Polarity::Affirmative);
    }

    /// Idempotence: running twice produces the same result.
    #[test]
    fn sentential_negation_is_idempotent() {
        let mut frames = vec![bare_noun_frame("шындық"), emes_particle_frame()];
        populate_sentential_negation(&mut frames);
        let after_first = frames.clone();
        populate_sentential_negation(&mut frames);
        assert_eq!(frames, after_first);
    }
}
