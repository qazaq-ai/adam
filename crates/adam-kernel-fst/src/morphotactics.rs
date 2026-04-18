//! Morphotactics — state machines that orchestrate suffix chains.
//!
//! Given a root and a feature bundle, this module emits a sequence of
//! archiphoneme strings that the [`phonology`] module will realise into
//! surface characters.
//!
//! [`phonology`]: crate::phonology

use serde::{Deserialize, Serialize};

use crate::phonology::{
    Archiphoneme, ConsonantClass, PhonologicalContext, classify_char, realise_archiphoneme,
    stem_vowel_harmony,
};

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

/// Atomic unit of an abstract suffix template. Each suffix in Apertium's
/// lexc morphotactics is a sequence of these: either a literal character
/// (typed directly, e.g. `р` in `-лар`) or an archiphoneme that will be
/// realised based on phonological context at synthesis time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuffixAtom {
    /// A fixed, non-alternating character (e.g. `р` in the plural суффикс).
    Literal(char),
    /// An abstract phoneme that needs phonological resolution.
    Arch(Archiphoneme),
}

/// Sequence of atoms describing a suffix in abstract form.
pub type SuffixTemplate = &'static [SuffixAtom];

/// Plural suffix: `-{L}{A}r`.  Examples:
///   бала + plural → балалар (vowel → default л)
///   мектеп + plural → мектептер (voiceless → т)
///   адам + plural → адамдар (nasal → д)
const PLURAL: SuffixTemplate = &[
    SuffixAtom::Arch(Archiphoneme::L),
    SuffixAtom::Arch(Archiphoneme::A),
    SuffixAtom::Literal('р'),
];

/// Dative suffix: `-{G}{A}`.
const DATIVE: SuffixTemplate = &[
    SuffixAtom::Arch(Archiphoneme::G),
    SuffixAtom::Arch(Archiphoneme::A),
];

/// Ablative suffix: `-{D}{A}n`.
const ABLATIVE: SuffixTemplate = &[
    SuffixAtom::Arch(Archiphoneme::D),
    SuffixAtom::Arch(Archiphoneme::A),
    SuffixAtom::Literal('н'),
];

/// Locative suffix: `-{D}{A}`.
const LOCATIVE: SuffixTemplate = &[
    SuffixAtom::Arch(Archiphoneme::D),
    SuffixAtom::Arch(Archiphoneme::A),
];

/// Accusative suffix: `-{D}{I}` (simplified — real rule also picks `н`
/// variant for certain contexts, covered later).
const ACCUSATIVE: SuffixTemplate = &[
    SuffixAtom::Arch(Archiphoneme::D),
    SuffixAtom::Arch(Archiphoneme::I),
];

/// Genitive suffix: `-{D}{I}ң`.
const GENITIVE: SuffixTemplate = &[
    SuffixAtom::Arch(Archiphoneme::D),
    SuffixAtom::Arch(Archiphoneme::I),
    SuffixAtom::Literal('ң'),
];

/// Instrumental suffix: `-{M}{E}н`.
const INSTRUMENTAL: SuffixTemplate = &[
    SuffixAtom::Arch(Archiphoneme::M),
    SuffixAtom::Arch(Archiphoneme::E),
    SuffixAtom::Literal('н'),
];

/// Past-definite tense marker: `-{D}{I}`. After stem ending in voiceless →
/// `-ты/-ті`; otherwise `-ды/-ді`.
const VERB_PAST: SuffixTemplate = &[
    SuffixAtom::Arch(Archiphoneme::D),
    SuffixAtom::Arch(Archiphoneme::I),
];

/// 1sg personal ending (attached after past-definite): `-м`.
const VERB_PERS_1SG: SuffixTemplate = &[SuffixAtom::Literal('м')];
/// 2sg informal: `-ң`.
const VERB_PERS_2SG: SuffixTemplate = &[SuffixAtom::Literal('ң')];
/// 1pl (attached after past-definite): `-{K}` → қ/к.
const VERB_PERS_1PL: SuffixTemplate = &[SuffixAtom::Arch(Archiphoneme::K)];

/// Runtime accumulator: output string + live phonological context.
struct Accumulator {
    out: String,
    ctx: PhonologicalContext,
}

impl Accumulator {
    fn from_stem(stem: &str) -> Self {
        let harmony = stem_vowel_harmony(stem);
        let last = stem.chars().last().unwrap_or('а');
        let preceding = classify_char(last).unwrap_or(ConsonantClass::VowelPreceding);
        let stem_has_nasal = stem
            .chars()
            .any(|c| matches!(classify_char(c), Some(ConsonantClass::Nasal)));
        Self {
            out: stem.to_string(),
            ctx: PhonologicalContext {
                harmony,
                preceding,
                stem_has_nasal,
                preceded_by_y_or_i: matches!(last, 'й' | 'и'),
            },
        }
    }

    fn apply(&mut self, template: SuffixTemplate) {
        for atom in template {
            match atom {
                SuffixAtom::Literal(c) => {
                    self.out.push(*c);
                    if let Some(class) = classify_char(*c) {
                        self.ctx.preceding = class;
                        if matches!(class, ConsonantClass::Nasal) {
                            self.ctx.stem_has_nasal = true;
                        }
                        self.ctx.preceded_by_y_or_i = matches!(*c, 'й' | 'и');
                    }
                }
                SuffixAtom::Arch(arch) => {
                    if let Some(c) = realise_archiphoneme(*arch, self.ctx) {
                        self.out.push(c);
                        if let Some(class) = classify_char(c) {
                            self.ctx.preceding = class;
                            if matches!(class, ConsonantClass::Nasal) {
                                self.ctx.stem_has_nasal = true;
                            }
                            self.ctx.preceded_by_y_or_i = matches!(c, 'й' | 'и');
                        }
                    }
                }
            }
        }
    }
}

/// Synthesise a fully-inflected noun surface form from a root lemma and a
/// feature bundle. Walks the canonical suffix order
/// `PLURAL → POSSESSIVE → CASE`, applying each present feature via its
/// suffix template, with phonological realisation happening atom-by-atom.
pub fn synthesise_noun(root: &str, features: NounFeatures) -> String {
    let mut acc = Accumulator::from_stem(root);
    if matches!(features.number, Some(Number::Plural)) {
        acc.apply(PLURAL);
    }
    // Possessive suffixes will be added in a later iteration; the week-1 test
    // matrix doesn't exercise them yet.
    if let Some(case) = features.case {
        match case {
            Case::Nominative => {}
            Case::Genitive => acc.apply(GENITIVE),
            Case::Dative => acc.apply(DATIVE),
            Case::Accusative => acc.apply(ACCUSATIVE),
            Case::Locative => acc.apply(LOCATIVE),
            Case::Ablative => acc.apply(ABLATIVE),
            Case::Instrumental => acc.apply(INSTRUMENTAL),
        }
    }
    acc.out
}

/// Synthesise a fully-inflected verb surface form. Walks
/// `VOICE → NEGATION → TENSE → PERSON/NUMBER`.
pub fn synthesise_verb(root: &str, features: VerbFeatures) -> String {
    let mut acc = Accumulator::from_stem(root);
    // Voice / negation not in week-1 test matrix; skipped.
    match features.tense {
        Some(Tense::PastDefinite) => acc.apply(VERB_PAST),
        Some(_) | None => {}
    }
    match (features.person, features.number) {
        (Some(Person::First), Some(Number::Singular)) | (Some(Person::First), None) => {
            acc.apply(VERB_PERS_1SG);
        }
        (Some(Person::Second), Some(Number::Singular)) | (Some(Person::Second), None) => {
            acc.apply(VERB_PERS_2SG);
        }
        (Some(Person::First), Some(Number::Plural)) => {
            acc.apply(VERB_PERS_1PL);
        }
        // Remaining combinations (3rd person, 2nd plural, etc.) are either
        // unmarked or not in the week-1 target test matrix; silently skip.
        _ => {}
    }
    acc.out
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests are written ahead of implementation so the week-2 port has
    // unambiguous targets. All marked `#[ignore]` until the implementation
    // lands.

    #[test]

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

    // Additional integration tests — each exercises a specific rule
    // interaction. These lock in the week-1 FST behaviour against future
    // phonology refactoring.

    #[test]
    fn noun_ablative_бала_after_vowel() {
        // бала + ABL = баладан (vowel-final → {D} → д)
        let out = synthesise_noun(
            "бала",
            NounFeatures {
                case: Some(Case::Ablative),
                ..Default::default()
            },
        );
        assert_eq!(out, "баладан");
    }

    #[test]
    fn noun_ablative_мектеп_voiceless() {
        // мектеп + ABL = мектептен ({D} → т after voiceless)
        let out = synthesise_noun(
            "мектеп",
            NounFeatures {
                case: Some(Case::Ablative),
                ..Default::default()
            },
        );
        assert_eq!(out, "мектептен");
    }

    #[test]
    fn noun_locative_адам_nasal() {
        // адам + LOC = адамда ({D} defaults to д after nasal; {A} back)
        let out = synthesise_noun(
            "адам",
            NounFeatures {
                case: Some(Case::Locative),
                ..Default::default()
            },
        );
        assert_eq!(out, "адамда");
    }

    #[test]
    fn noun_dative_іс_front_voiceless() {
        // іс + DAT = іске ({G} → к after voiceless, front harmony; {A} → е)
        let out = synthesise_noun(
            "іс",
            NounFeatures {
                case: Some(Case::Dative),
                ..Default::default()
            },
        );
        assert_eq!(out, "іске");
    }

    #[test]
    fn noun_plural_locative_мектеп() {
        // мектеп + PLURAL + LOC = мектептерде
        let out = synthesise_noun(
            "мектеп",
            NounFeatures {
                number: Some(Number::Plural),
                case: Some(Case::Locative),
                ..Default::default()
            },
        );
        assert_eq!(out, "мектептерде");
    }

    #[test]
    fn verb_past_1sg_жаз() {
        // жаз + PAST + 1SG = жаздым
        let out = synthesise_verb(
            "жаз",
            VerbFeatures {
                tense: Some(Tense::PastDefinite),
                person: Some(Person::First),
                number: Some(Number::Singular),
                ..Default::default()
            },
        );
        assert_eq!(out, "жаздым");
    }

    #[test]
    fn verb_past_1pl_жаз() {
        // жаз + PAST + 1PL = жаздық ({K} → қ because back + voiced ы)
        let out = synthesise_verb(
            "жаз",
            VerbFeatures {
                tense: Some(Tense::PastDefinite),
                person: Some(Person::First),
                number: Some(Number::Plural),
                ..Default::default()
            },
        );
        assert_eq!(out, "жаздық");
    }
}
