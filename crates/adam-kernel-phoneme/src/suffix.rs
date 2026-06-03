// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Phoneme-native suffix templates.
//!
//! Each suffix declares its **variants** explicitly — one per
//! (harmony, coda) combination. Variant selection is a typed
//! lookup against the root's last vowel ([`Phoneme::
//! harmony_class`]) and last phoneme ([`classify_coda`]).
//!
//! ## Why no archiphonemes
//!
//! The old graphemic FST writes a plural suffix as the
//! abstract `{L}{A}{R}` and applies three rules:
//!   1. `{L}` → `л` after vowel / sonorant; `т` after
//!      voiceless; `д` after nasal.
//!   2. `{A}` → `а` (back harmony) / `е` (front).
//!   3. `{R}` → `р` invariant.
//!
//! Here we list all six surface variants directly. The
//! "rule" lives in `classify_coda()` and a lookup, both of
//! which are direct consequences of `Phoneme` attributes.
//! Saving the rule machinery is the whole point of the
//! phoneme-first thesis.

use adam_phoneme::{HarmonyClass, Phoneme};

/// Coda condition that drives the L / D / T choice at the
/// start of a suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CodaClass {
    /// Last phoneme is a vowel.
    Vowel,
    /// Last phoneme is a voiced sonorant (L, R, J, M, N, Ng,
    /// W) or voiced obstruent (B, D, G, Z, Zh, V, Gh).
    /// **For the 4 suffixes shipped today**, this triggers
    /// the D-initial variant (плюрал «-дар», локатив «-да»,
    /// датив «-ға»? no, dative is voiced → -ға, voiceless →
    /// -қа, so dative has its own path; see below).
    VoicedOrSonorant,
    /// Last phoneme is a voiceless consonant. Triggers the
    /// T-initial variant (плюрал «-тар», локатив «-та»,
    /// датив «-қа/-ке»).
    Voiceless,
}

/// Classify a phoneme as a coda condition.
///
/// Vowels are clear; consonant voicing comes straight from
/// [`Phoneme::voicing`]. Boundary markers are treated as
/// `VoicedOrSonorant` (the most common case) so the kernel
/// degrades safely on stray markers.
pub fn classify_coda(p: Phoneme) -> CodaClass {
    use adam_phoneme::Voicing;
    if p.is_vowel() {
        return CodaClass::Vowel;
    }
    match p.voicing() {
        Some(Voicing::Voiceless) => CodaClass::Voiceless,
        _ => CodaClass::VoicedOrSonorant,
    }
}

/// One concrete surface variant of a suffix.
#[derive(Debug, Clone, Copy)]
pub struct SuffixVariant {
    pub harmony: HarmonyClass,
    pub coda: CodaClass,
    pub phonemes: &'static [Phoneme],
}

/// Suffix template: a set of variants, the runtime picks the
/// match.
#[derive(Debug, Clone, Copy)]
pub struct PhonemeSuffix {
    pub name: &'static str,
    pub variants: &'static [SuffixVariant],
}

impl PhonemeSuffix {
    /// Pick the variant matching the given (harmony, coda).
    /// Returns `None` if no variant matches — this means the
    /// suffix table is incomplete for that combination.
    pub fn pick(&self, harmony: HarmonyClass, coda: CodaClass) -> Option<&SuffixVariant> {
        self.variants
            .iter()
            .find(|v| v.harmony == harmony && v.coda == coda)
    }
}

/// Stable identifier for the suffixes shipped with the
/// Phase 9 POC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SuffixId {
    Plural,
    Locative,
    Dative,
    Genitive,
}

/// Lookup table — `SuffixId` → `PhonemeSuffix`.
pub fn lookup(id: SuffixId) -> &'static PhonemeSuffix {
    match id {
        SuffixId::Plural => &PLURAL,
        SuffixId::Locative => &LOCATIVE,
        SuffixId::Dative => &DATIVE,
        SuffixId::Genitive => &GENITIVE,
    }
}

/// All shipped suffixes, in stable order. Useful for
/// exhaustive paradigm tests.
pub const SUFFIXES: &[SuffixId] = &[
    SuffixId::Plural,
    SuffixId::Locative,
    SuffixId::Dative,
    SuffixId::Genitive,
];

// ─── Suffix declarations ──────────────────────────────────

use HarmonyClass::*;
use Phoneme::*;

/// Plural: -лар/-лер (vowel/voiced root) / -тар/-тер
/// (voiceless root). (We collapse the L/D distinction into a
/// single "voiced-or-sonorant" category in this POC; proper
/// Kazakh has -дар/-дер after specific consonants — refinement
/// in v2 of the kernel.)
pub const PLURAL: PhonemeSuffix = PhonemeSuffix {
    name: "plural",
    variants: &[
        SuffixVariant {
            harmony: Back,
            coda: CodaClass::Vowel,
            phonemes: &[L, A, R],
        },
        SuffixVariant {
            harmony: Front,
            coda: CodaClass::Vowel,
            phonemes: &[L, E, R],
        },
        SuffixVariant {
            harmony: Back,
            coda: CodaClass::VoicedOrSonorant,
            phonemes: &[D, A, R],
        },
        SuffixVariant {
            harmony: Front,
            coda: CodaClass::VoicedOrSonorant,
            phonemes: &[D, E, R],
        },
        SuffixVariant {
            harmony: Back,
            coda: CodaClass::Voiceless,
            phonemes: &[T, A, R],
        },
        SuffixVariant {
            harmony: Front,
            coda: CodaClass::Voiceless,
            phonemes: &[T, E, R],
        },
    ],
};

/// Locative: -да/-де after voiced/vowel, -та/-те after
/// voiceless.
pub const LOCATIVE: PhonemeSuffix = PhonemeSuffix {
    name: "locative",
    variants: &[
        SuffixVariant {
            harmony: Back,
            coda: CodaClass::Vowel,
            phonemes: &[D, A],
        },
        SuffixVariant {
            harmony: Front,
            coda: CodaClass::Vowel,
            phonemes: &[D, E],
        },
        SuffixVariant {
            harmony: Back,
            coda: CodaClass::VoicedOrSonorant,
            phonemes: &[D, A],
        },
        SuffixVariant {
            harmony: Front,
            coda: CodaClass::VoicedOrSonorant,
            phonemes: &[D, E],
        },
        SuffixVariant {
            harmony: Back,
            coda: CodaClass::Voiceless,
            phonemes: &[T, A],
        },
        SuffixVariant {
            harmony: Front,
            coda: CodaClass::Voiceless,
            phonemes: &[T, E],
        },
    ],
};

/// Dative: -ға/-ге after voiced/vowel, -қа/-ке after
/// voiceless. (Kazakh-specific: the uvular Q/Gh-initial
/// variants — а sharp departure from Russian/Turkish where
/// the equivalent suffix would be -ka/-ke or -ga/-ge.)
pub const DATIVE: PhonemeSuffix = PhonemeSuffix {
    name: "dative",
    variants: &[
        SuffixVariant {
            harmony: Back,
            coda: CodaClass::Vowel,
            phonemes: &[Gh, A],
        },
        SuffixVariant {
            harmony: Front,
            coda: CodaClass::Vowel,
            phonemes: &[G, E],
        },
        SuffixVariant {
            harmony: Back,
            coda: CodaClass::VoicedOrSonorant,
            phonemes: &[Gh, A],
        },
        SuffixVariant {
            harmony: Front,
            coda: CodaClass::VoicedOrSonorant,
            phonemes: &[G, E],
        },
        SuffixVariant {
            harmony: Back,
            coda: CodaClass::Voiceless,
            phonemes: &[Q, A],
        },
        SuffixVariant {
            harmony: Front,
            coda: CodaClass::Voiceless,
            phonemes: &[K, E],
        },
    ],
};

/// Genitive: -ның/-нің after vowel, -дың/-дің after voiced/
/// sonorant, -тың/-тің after voiceless. The Kazakh genitive
/// is the most paradigm-rich case: 6 surface variants
/// distinguishing every coda class × harmony combination.
pub const GENITIVE: PhonemeSuffix = PhonemeSuffix {
    name: "genitive",
    variants: &[
        SuffixVariant {
            harmony: Back,
            coda: CodaClass::Vowel,
            phonemes: &[N, Y, Ng],
        },
        SuffixVariant {
            harmony: Front,
            coda: CodaClass::Vowel,
            phonemes: &[N, Yi, Ng],
        },
        SuffixVariant {
            harmony: Back,
            coda: CodaClass::VoicedOrSonorant,
            phonemes: &[D, Y, Ng],
        },
        SuffixVariant {
            harmony: Front,
            coda: CodaClass::VoicedOrSonorant,
            phonemes: &[D, Yi, Ng],
        },
        SuffixVariant {
            harmony: Back,
            coda: CodaClass::Voiceless,
            phonemes: &[T, Y, Ng],
        },
        SuffixVariant {
            harmony: Front,
            coda: CodaClass::Voiceless,
            phonemes: &[T, Yi, Ng],
        },
    ],
};

#[cfg(test)]
mod tests {
    use super::*;

    /// `classify_coda` returns the documented class for every
    /// phoneme class.
    #[test]
    fn classify_coda_matches_phoneme_class() {
        assert_eq!(classify_coda(A), CodaClass::Vowel);
        assert_eq!(classify_coda(Yi), CodaClass::Vowel);
        assert_eq!(classify_coda(T), CodaClass::Voiceless);
        assert_eq!(classify_coda(P), CodaClass::Voiceless);
        assert_eq!(classify_coda(S), CodaClass::Voiceless);
        assert_eq!(classify_coda(Q), CodaClass::Voiceless);
        assert_eq!(classify_coda(M), CodaClass::VoicedOrSonorant);
        assert_eq!(classify_coda(N), CodaClass::VoicedOrSonorant);
        assert_eq!(classify_coda(L), CodaClass::VoicedOrSonorant);
        assert_eq!(classify_coda(R), CodaClass::VoicedOrSonorant);
        assert_eq!(classify_coda(Z), CodaClass::VoicedOrSonorant);
        assert_eq!(classify_coda(B), CodaClass::VoicedOrSonorant);
        assert_eq!(classify_coda(J), CodaClass::VoicedOrSonorant);
    }

    /// Every shipped suffix has all 6 (harmony × coda) variants.
    #[test]
    fn all_suffixes_complete() {
        for &id in SUFFIXES {
            let suffix = lookup(id);
            assert_eq!(
                suffix.variants.len(),
                6,
                "suffix «{}» has {} variants, expected 6",
                suffix.name,
                suffix.variants.len()
            );
            // Each (harmony, coda) combination appears exactly once.
            for harmony in [Back, Front] {
                for coda in [
                    CodaClass::Vowel,
                    CodaClass::VoicedOrSonorant,
                    CodaClass::Voiceless,
                ] {
                    let found = suffix
                        .variants
                        .iter()
                        .filter(|v| v.harmony == harmony && v.coda == coda)
                        .count();
                    assert_eq!(
                        found, 1,
                        "suffix «{}» missing variant ({harmony:?}, {coda:?})",
                        suffix.name
                    );
                }
            }
        }
    }

    /// Locative voiceless / Back picks -та.
    #[test]
    fn locative_picks_correct_variant() {
        let v = LOCATIVE.pick(Back, CodaClass::Voiceless).unwrap();
        assert_eq!(v.phonemes, &[T, A]);
    }
}
