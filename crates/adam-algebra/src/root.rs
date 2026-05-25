// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `Root` — typed root wrapper.
//!
//! A root in Kazakh is a part-of-speech-tagged stem. Algebraically,
//! it's the **identity element** of the agglutinative algebra: any
//! valid composition begins from a `Root` and produces a surface
//! form by applying a sequence of [`SuffixOp`](crate::SuffixOp).

use serde::{Deserialize, Serialize};

/// Part-of-speech tag carried by every root. Mirrors the v6.1
/// `LexiconV1` POS taxonomy (noun / adjective / pronoun / numeral
/// / verb / particle). The taxonomy is closed at the lexicon level
/// — adding a new POS requires a lexicon migration, NOT an algebra
/// extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartOfSpeech {
    Noun,
    Adjective,
    Pronoun,
    Numeral,
    Verb,
    /// Particles inflect like nouns at the FST level (see
    /// `morphotactics.rs::try_noun_analyses` for «емес»). v6.2
    /// follows the kernel: particles are typed as their own POS
    /// but route through the noun composition slots.
    Particle,
}

impl PartOfSpeech {
    /// Stable slug for JSON round-trip + grepping.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Noun => "noun",
            Self::Adjective => "adjective",
            Self::Pronoun => "pronoun",
            Self::Numeral => "numeral",
            Self::Verb => "verb",
            Self::Particle => "particle",
        }
    }

    /// Parse a `LexiconV1` POS string into a typed `PartOfSpeech`.
    /// Returns `None` for unknown values; callers should treat
    /// unknown as "leave the lexicon entry inert" rather than
    /// panicking — adding a new POS is a lexicon-migration issue,
    /// not an algebra-runtime issue.
    pub fn from_lexicon_str(s: &str) -> Option<Self> {
        Some(match s {
            "noun" => Self::Noun,
            "adjective" => Self::Adjective,
            "pronoun" => Self::Pronoun,
            "numeral" => Self::Numeral,
            "verb" => Self::Verb,
            "particle" => Self::Particle,
            _ => return None,
        })
    }

    /// True iff this POS routes through the **noun composition**
    /// slot stack (Derivation → Number → Possessive → Case →
    /// Predicate). Per `adam_kernel_fst::parser::analyse` this
    /// covers Noun / Adjective / Pronoun / Numeral / Particle.
    pub fn uses_noun_composition(self) -> bool {
        matches!(
            self,
            Self::Noun | Self::Adjective | Self::Pronoun | Self::Numeral | Self::Particle
        )
    }

    /// True iff this POS routes through the **verb composition**
    /// slot stack (Voice → Negation → Tense → Person / Number).
    pub fn uses_verb_composition(self) -> bool {
        matches!(self, Self::Verb)
    }
}

/// Typed root carrying the surface stem + POS tag. The identity
/// element of the agglutinative algebra: `Composition::identity(root)`
/// produces a `Composition` with zero applied operators whose
/// surface form equals `root.surface`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Root {
    /// Canonical surface form of the stem (no inflection applied).
    /// Lowercased per the lexicon convention.
    pub surface: String,
    /// Part-of-speech tag.
    pub pos: PartOfSpeech,
}

impl Root {
    /// Construct a typed root from a stem + POS pair. The surface
    /// is lowercased for canonical comparison.
    pub fn new(surface: impl Into<String>, pos: PartOfSpeech) -> Self {
        Self {
            surface: surface.into().to_lowercase(),
            pos,
        }
    }

    /// Convenience: noun root.
    pub fn noun(surface: impl Into<String>) -> Self {
        Self::new(surface, PartOfSpeech::Noun)
    }

    /// Convenience: verb root.
    pub fn verb(surface: impl Into<String>) -> Self {
        Self::new(surface, PartOfSpeech::Verb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pos_round_trip_via_str() {
        for pos in [
            PartOfSpeech::Noun,
            PartOfSpeech::Adjective,
            PartOfSpeech::Pronoun,
            PartOfSpeech::Numeral,
            PartOfSpeech::Verb,
            PartOfSpeech::Particle,
        ] {
            let s = pos.as_str();
            assert_eq!(PartOfSpeech::from_lexicon_str(s), Some(pos));
        }
    }

    #[test]
    fn unknown_pos_returns_none() {
        assert_eq!(PartOfSpeech::from_lexicon_str("unicorn"), None);
    }

    #[test]
    fn noun_composition_branch() {
        for pos in [
            PartOfSpeech::Noun,
            PartOfSpeech::Adjective,
            PartOfSpeech::Pronoun,
            PartOfSpeech::Numeral,
            PartOfSpeech::Particle,
        ] {
            assert!(pos.uses_noun_composition());
            assert!(!pos.uses_verb_composition());
        }
        assert!(PartOfSpeech::Verb.uses_verb_composition());
        assert!(!PartOfSpeech::Verb.uses_noun_composition());
    }

    #[test]
    fn root_lowercases_surface() {
        let r = Root::new("Адам", PartOfSpeech::Noun);
        assert_eq!(r.surface, "адам");
    }
}
