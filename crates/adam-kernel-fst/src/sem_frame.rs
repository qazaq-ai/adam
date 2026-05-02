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
}
