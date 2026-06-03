// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! The single inflection entry point.
//!
//! `inflect(root, suffix_id) -> Vec<Phoneme>` picks the
//! matching suffix variant by inspecting the root's last
//! vowel (harmony class) and last phoneme (coda class), then
//! concatenates.

use adam_phoneme::{HarmonyClass, Phoneme};

use crate::lexicon::PhonemeRoot;
use crate::suffix::{CodaClass, SuffixId, classify_coda, lookup};

/// Result of [`inflect`] — the inflected phoneme sequence
/// plus the picked variant's metadata for diagnostics.
#[derive(Debug, Clone)]
pub struct InflectedForm {
    pub phonemes: Vec<Phoneme>,
    pub root_cyrillic: &'static str,
    pub suffix_id: SuffixId,
    pub harmony: HarmonyClass,
    pub coda: CodaClass,
}

/// Inflection error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InflectError {
    /// The root contains no vowel — cannot determine harmony.
    NoVowel,
    /// The suffix table has no variant for the picked
    /// (harmony, coda). Shouldn't happen for the canonical
    /// 4 suffixes (we ship all 6 variants each).
    NoMatchingVariant {
        suffix: &'static str,
        harmony: HarmonyClass,
        coda: CodaClass,
    },
}

impl std::fmt::Display for InflectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoVowel => write!(f, "inflect: root has no vowel"),
            Self::NoMatchingVariant {
                suffix,
                harmony,
                coda,
            } => write!(
                f,
                "inflect: suffix «{suffix}» has no variant for ({harmony:?}, {coda:?})"
            ),
        }
    }
}

impl std::error::Error for InflectError {}

/// Inflect a root with the named suffix.
pub fn inflect(root: &PhonemeRoot, suffix_id: SuffixId) -> Result<InflectedForm, InflectError> {
    // Harmony from the LAST vowel in the root (Kazakh
    // suffix harmony agrees with the rightmost root vowel).
    let harmony = root
        .phonemes
        .iter()
        .rev()
        .find_map(|p| p.harmony_class())
        .ok_or(InflectError::NoVowel)?;
    // Coda from the last phoneme.
    let last_phoneme = *root.phonemes.last().expect("root checked to have vowel");
    let coda = classify_coda(last_phoneme);

    let suffix = lookup(suffix_id);
    let variant = suffix
        .pick(harmony, coda)
        .ok_or(InflectError::NoMatchingVariant {
            suffix: suffix.name,
            harmony,
            coda,
        })?;

    let mut phonemes = root.phonemes.clone();
    phonemes.extend_from_slice(variant.phonemes);

    Ok(InflectedForm {
        phonemes,
        root_cyrillic: root.cyrillic,
        suffix_id,
        harmony,
        coda,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexicon::sample_lexicon;
    use adam_phoneme::cyrillic::phonemes_to_cyrillic;

    /// Find a sample lexicon root by Cyrillic surface.
    fn root_by_cyrillic(cyr: &str) -> PhonemeRoot {
        sample_lexicon()
            .into_iter()
            .find(|r| r.cyrillic == cyr)
            .unwrap_or_else(|| panic!("no root «{cyr}»"))
    }

    #[test]
    fn no_vowel_root_rejected() {
        // Synthetic: «зщ» has no vowel.
        let r = PhonemeRoot {
            phonemes: vec![Phoneme::Z, Phoneme::Sh],
            pos: crate::PartOfSpeech::Other,
            cyrillic: "зш",
        };
        assert!(matches!(
            inflect(&r, SuffixId::Locative),
            Err(InflectError::NoVowel)
        ));
    }

    /// «бала» (vowel-final, back) + plural → «балалар».
    #[test]
    fn bala_plural() {
        let f = inflect(&root_by_cyrillic("бала"), SuffixId::Plural).unwrap();
        assert_eq!(phonemes_to_cyrillic(&f.phonemes), "балалар");
    }

    /// «тас» (voiceless-final, back) + plural → «тастар».
    #[test]
    fn tas_plural() {
        let f = inflect(&root_by_cyrillic("тас"), SuffixId::Plural).unwrap();
        assert_eq!(phonemes_to_cyrillic(&f.phonemes), "тастар");
    }

    /// «адам» (voiced-sonorant-final, back) + plural → «адамдар».
    #[test]
    fn adam_plural() {
        let f = inflect(&root_by_cyrillic("адам"), SuffixId::Plural).unwrap();
        assert_eq!(phonemes_to_cyrillic(&f.phonemes), "адамдар");
    }

    /// «көл» (voiced-sonorant-final, front) + plural → «көлдер».
    #[test]
    fn kol_plural() {
        let f = inflect(&root_by_cyrillic("көл"), SuffixId::Plural).unwrap();
        assert_eq!(phonemes_to_cyrillic(&f.phonemes), "көлдер");
    }

    /// «кітап» (voiceless-final, front) + plural → «кітаптер».
    /// (Phonemically [K, Yi, T, A, P] — last vowel A is back!
    /// So Kazakh actually says «кітаптар», not «кітаптер».
    /// The harmony of the last vowel rules — this is a
    /// good test of "right-most vowel wins" behaviour.)
    #[test]
    fn kitap_plural_back_harmony_via_last_vowel() {
        let f = inflect(&root_by_cyrillic("кітап"), SuffixId::Plural).unwrap();
        // [K, Yi, T, A, P] — last vowel A is back-harmonic.
        assert_eq!(f.harmony, HarmonyClass::Back);
        assert_eq!(phonemes_to_cyrillic(&f.phonemes), "кітаптар");
    }

    /// «тас» + locative → «таста».
    #[test]
    fn tas_locative() {
        let f = inflect(&root_by_cyrillic("тас"), SuffixId::Locative).unwrap();
        assert_eq!(phonemes_to_cyrillic(&f.phonemes), "таста");
    }

    /// «адам» + locative → «адамда».
    #[test]
    fn adam_locative() {
        let f = inflect(&root_by_cyrillic("адам"), SuffixId::Locative).unwrap();
        assert_eq!(phonemes_to_cyrillic(&f.phonemes), "адамда");
    }

    /// «көл» + locative → «көлде».
    #[test]
    fn kol_locative() {
        let f = inflect(&root_by_cyrillic("көл"), SuffixId::Locative).unwrap();
        assert_eq!(phonemes_to_cyrillic(&f.phonemes), "көлде");
    }

    /// «бала» + dative → «балаға».
    #[test]
    fn bala_dative() {
        let f = inflect(&root_by_cyrillic("бала"), SuffixId::Dative).unwrap();
        assert_eq!(phonemes_to_cyrillic(&f.phonemes), "балаға");
    }

    /// «тас» + dative → «тасқа» (uvular-Q variant after
    /// voiceless coda).
    #[test]
    fn tas_dative() {
        let f = inflect(&root_by_cyrillic("тас"), SuffixId::Dative).unwrap();
        assert_eq!(phonemes_to_cyrillic(&f.phonemes), "тасқа");
    }

    /// «көл» + dative → «көлге» (voiced velar G after voiced
    /// coda + front harmony).
    #[test]
    fn kol_dative() {
        let f = inflect(&root_by_cyrillic("көл"), SuffixId::Dative).unwrap();
        assert_eq!(phonemes_to_cyrillic(&f.phonemes), "көлге");
    }

    /// «бала» + genitive → «баланың».
    #[test]
    fn bala_genitive() {
        let f = inflect(&root_by_cyrillic("бала"), SuffixId::Genitive).unwrap();
        assert_eq!(phonemes_to_cyrillic(&f.phonemes), "баланың");
    }

    /// «адам» + genitive → «адамдың».
    #[test]
    fn adam_genitive() {
        let f = inflect(&root_by_cyrillic("адам"), SuffixId::Genitive).unwrap();
        assert_eq!(phonemes_to_cyrillic(&f.phonemes), "адамдың");
    }

    /// «тас» + genitive → «тастың» (voiceless coda → T-initial).
    #[test]
    fn tas_genitive() {
        let f = inflect(&root_by_cyrillic("тас"), SuffixId::Genitive).unwrap();
        assert_eq!(phonemes_to_cyrillic(&f.phonemes), "тастың");
    }

    /// «көл» + genitive → «көлдің».
    #[test]
    fn kol_genitive() {
        let f = inflect(&root_by_cyrillic("көл"), SuffixId::Genitive).unwrap();
        assert_eq!(phonemes_to_cyrillic(&f.phonemes), "көлдің");
    }
}
