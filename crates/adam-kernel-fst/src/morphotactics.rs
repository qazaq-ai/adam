//! Morphotactics — state machines that orchestrate suffix chains.
//!
//! Given a root and a feature bundle, this module emits a sequence of
//! archiphoneme strings that the [`phonology`] module will realise into
//! surface characters.
//!
//! [`phonology`]: crate::phonology

use serde::{Deserialize, Serialize};

use crate::phonology::{
    Archiphoneme, ConsonantClass, PhonologicalContext, apply_intervocalic_voicing, classify_char,
    is_vowel, realise_archiphoneme, stem_vowel_harmony,
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

// -------------------------------------------------------------------------
// Possessive suffix templates (week 1 day 1 extension).
// The common pattern is `-{Y}PERSON` where `{Y}` is a buffer ы/і that
// appears only on consonant-final stems.  3rd person also prefixes `{S}`
// (buffer с) that appears only on vowel-final stems.
// -------------------------------------------------------------------------

/// 1sg possessive: `-{Y}м` (мектеп → мектебім, бала → балам).
const POSS_1SG: SuffixTemplate = &[SuffixAtom::Arch(Archiphoneme::Y), SuffixAtom::Literal('м')];

/// 2sg informal possessive: `-{Y}ң`.
const POSS_2SG: SuffixTemplate = &[SuffixAtom::Arch(Archiphoneme::Y), SuffixAtom::Literal('ң')];

/// 3rd-person possessive (sg/pl syncretic): `-{S}{I}`.
/// After vowel-final stem: бала → баласы. After consonant: мектеп → мектебі.
const POSS_3: SuffixTemplate = &[
    SuffixAtom::Arch(Archiphoneme::S),
    SuffixAtom::Arch(Archiphoneme::I),
];

/// 1pl possessive: `-{Y}м{I}з`.
/// Back: `-ымыз`; front: `-іміз`. After vowel drops the buffer ы/і.
const POSS_1PL: SuffixTemplate = &[
    SuffixAtom::Arch(Archiphoneme::Y),
    SuffixAtom::Literal('м'),
    SuffixAtom::Arch(Archiphoneme::I),
    SuffixAtom::Literal('з'),
];

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
            let realised: Option<char> = match atom {
                SuffixAtom::Literal(c) => Some(*c),
                SuffixAtom::Arch(arch) => realise_archiphoneme(*arch, self.ctx),
            };
            if let Some(c) = realised {
                self.out.push(c);
                if let Some(class) = classify_char(c) {
                    self.ctx.preceding = class;
                    if matches!(class, ConsonantClass::Nasal) {
                        self.ctx.stem_has_nasal = true;
                    }
                    self.ctx.preceded_by_y_or_i = matches!(c, 'й' | 'и');
                }
                // After appending a vowel we may have just created an
                // intervocalic V+voiceless-obstruent+V pattern. Rules 10-12:
                // п→б, к→г, қ→ғ. Apply in place and update `preceding` if the
                // stem's final char changed as a side-effect.
                if is_vowel(c) {
                    let before = self.out.chars().nth(self.out.chars().count() - 2);
                    apply_intervocalic_voicing(&mut self.out);
                    let after = self.out.chars().nth(self.out.chars().count() - 2);
                    // The voicing does NOT shift `self.ctx.preceding` because
                    // that field tracks the character we just appended (the
                    // vowel), not the one voiced two positions back.
                    let _ = (before, after);
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
    // Possessive. Only P1SG / P2SG / P3 / P1PL implemented in day 1.
    if let Some(poss) = features.possessive {
        match poss {
            Possessive::P1Sg => acc.apply(POSS_1SG),
            Possessive::P2SgInformal => acc.apply(POSS_2SG),
            Possessive::P3 => acc.apply(POSS_3),
            Possessive::P1Pl => acc.apply(POSS_1PL),
            // P2 polite + plural forms land in week 2.
            _ => {}
        }
    }
    // Pronominal-н buffer: in classical Kazakh, a 3rd-person possessive
    // noun takes a buffer `н` before accusative / dative / ablative /
    // locative / instrumental (catalogue rules 39-42, 44). With only P3
    // implemented, we inject `н` once here if the feature combination is
    // P3 + non-nominative case.
    let needs_pronominal_n = matches!(features.possessive, Some(Possessive::P3))
        && matches!(
            features.case,
            Some(Case::Accusative)
                | Some(Case::Dative)
                | Some(Case::Ablative)
                | Some(Case::Locative)
                | Some(Case::Instrumental)
        );
    if needs_pronominal_n {
        acc.out.push('н');
        acc.ctx.preceding = ConsonantClass::Nasal;
        acc.ctx.stem_has_nasal = true;
    }
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

    // -----------------------------------------------------------------
    // Possessive + intervocalic-voicing tests.
    // -----------------------------------------------------------------

    #[test]
    fn poss_1sg_бала_vowel_final() {
        // бала + POSS.1SG = балам ({Y} buffer ы/і drops after vowel)
        let out = synthesise_noun(
            "бала",
            NounFeatures {
                possessive: Some(Possessive::P1Sg),
                ..Default::default()
            },
        );
        assert_eq!(out, "балам");
    }

    #[test]
    fn poss_1sg_мектеп_intervocalic_voicing() {
        // мектеп + POSS.1SG = мектебім
        //   - {Y} → і (front, buffer inserted after consonant)
        //   - intervocalic voicing п → б (е-п-і)
        let out = synthesise_noun(
            "мектеп",
            NounFeatures {
                possessive: Some(Possessive::P1Sg),
                ..Default::default()
            },
        );
        assert_eq!(out, "мектебім");
    }

    #[test]
    fn poss_3_бала_vowel_final_с_buffer() {
        // бала + POSS.3 = баласы ({S} buffer inserted after vowel)
        let out = synthesise_noun(
            "бала",
            NounFeatures {
                possessive: Some(Possessive::P3),
                ..Default::default()
            },
        );
        assert_eq!(out, "баласы");
    }

    #[test]
    fn poss_3_мектеп_consonant_final() {
        // мектеп + POSS.3 = мектебі
        //   - {S} drops after consonant
        //   - {I} → і front
        //   - intervocalic voicing п → б
        let out = synthesise_noun(
            "мектеп",
            NounFeatures {
                possessive: Some(Possessive::P3),
                ..Default::default()
            },
        );
        assert_eq!(out, "мектебі");
    }

    #[test]
    fn poss_1pl_мектеп_chain() {
        // мектеп + POSS.1PL = мектебіміз
        let out = synthesise_noun(
            "мектеп",
            NounFeatures {
                possessive: Some(Possessive::P1Pl),
                ..Default::default()
            },
        );
        assert_eq!(out, "мектебіміз");
    }

    #[test]
    fn poss_3_case_loc_бала_pronominal_n() {
        // бала + POSS.3 + LOC = баласында (pronominal н before locative)
        let out = synthesise_noun(
            "бала",
            NounFeatures {
                possessive: Some(Possessive::P3),
                case: Some(Case::Locative),
                ..Default::default()
            },
        );
        assert_eq!(out, "баласында");
    }
}
