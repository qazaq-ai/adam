//! Morphotactics — state machines that orchestrate suffix chains.
//!
//! Given a root and a feature bundle, this module emits a sequence of
//! archiphoneme strings that the [`phonology`] module will realise into
//! surface characters.
//!
//! Stage: week 1 day 1 scaffold. Types only; implementation follows when
//! the suffix tables are transcribed from the Apertium `.lexc` morphotactics
//! section.
//!
//! [`phonology`]: crate::phonology

use serde::{Deserialize, Serialize};

/// Partial feature bundle for a noun-like word.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NounFeatures {
    pub number: Option<Number>,
    pub possessive: Option<Possessive>,
    pub case: Option<Case>,
}

/// Partial feature bundle for a verb-like word.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerbFeatures {
    pub voice: Option<Voice>,
    pub negation: bool,
    pub tense: Option<Tense>,
    pub person: Option<Person>,
    pub number: Option<Number>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Number {
    Singular,
    Plural,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Possessive {
    P1Sg,
    P2SgInformal,
    P2SgPolite,
    P3,
    P1Pl,
    P2PlInformal,
    P2PlPolite,
}

/// Seven canonical cases of Kazakh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Case {
    Nominative,
    Genitive,
    Dative,
    Accusative,
    Locative,
    Ablative,
    Instrumental,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Voice {
    Active,
    Passive,
    Reflexive,
    Reciprocal,
    Causative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tense {
    PastDefinite,
    PastEvidential,
    PastHabitual,
    Present,
    FutureIntentional,
    FuturePossible,
    Conditional,
    Imperative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Person {
    First,
    Second,
    Third,
}

/// Placeholder for the noun-FST entry point. Not yet implemented.
///
/// Target signature (week 2):
/// ```ignore
/// pub fn synthesise_noun(root: &str, features: NounFeatures) -> String
/// ```
/// Parses the stem, builds a feature-ordered abstract string, then walks
/// each archiphoneme through `phonology::realise_archiphoneme`.
pub fn synthesise_noun(_root: &str, _features: NounFeatures) -> String {
    // Week 2: implement.
    String::new()
}

/// Placeholder for the verb-FST entry point.
pub fn synthesise_verb(_root: &str, _features: VerbFeatures) -> String {
    // Week 2: implement.
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests are written ahead of implementation so the week-2 port has
    // unambiguous targets. All marked `#[ignore]` until the implementation
    // lands.

    #[test]
    #[ignore]
    fn noun_plural_dative_бала() {
        // бала (child) + PLURAL + DATIVE = балаларға
        let out = synthesise_noun(
            "бала",
            NounFeatures {
                number: Some(Number::Plural),
                case: Some(Case::Dative),
                ..Default::default()
            },
        );
        assert_eq!(out, "балаларға");
    }

    #[test]
    #[ignore]
    fn noun_plural_мектеп() {
        // мектеп (school) + PLURAL = мектептер (voiceless final triggers
        // {L}{A}r → тер)
        let out = synthesise_noun(
            "мектеп",
            NounFeatures {
                number: Some(Number::Plural),
                ..Default::default()
            },
        );
        assert_eq!(out, "мектептер");
    }

    #[test]
    #[ignore]
    fn noun_plural_адам_nasal() {
        // адам (person) + PLURAL = адамдар (nasal final triggers
        // {L}{A}r → дар)
        let out = synthesise_noun(
            "адам",
            NounFeatures {
                number: Some(Number::Plural),
                ..Default::default()
            },
        );
        assert_eq!(out, "адамдар");
    }

    #[test]
    #[ignore]
    fn verb_past_3sg_жаз() {
        // жаз (write) + PAST + 3 = жазды
        let out = synthesise_verb(
            "жаз",
            VerbFeatures {
                tense: Some(Tense::PastDefinite),
                person: Some(Person::Third),
                ..Default::default()
            },
        );
        assert_eq!(out, "жазды");
    }

    #[test]
    #[ignore]
    fn verb_past_1sg_бер() {
        // бер (give) + PAST + 1SG = бердім (front harmony)
        let out = synthesise_verb(
            "бер",
            VerbFeatures {
                tense: Some(Tense::PastDefinite),
                person: Some(Person::First),
                number: Some(Number::Singular),
                ..Default::default()
            },
        );
        assert_eq!(out, "бердім");
    }
}
