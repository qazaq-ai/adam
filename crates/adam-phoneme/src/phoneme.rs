// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! The 37-unit phoneme alphabet of Kazakh.
//!
//! Variant naming uses ASCII transliteration of the IPA / Cyrillic
//! form: `A` (а), `Ae` (ә), `O` (о), `Oe` (ө), `U` (ұ), `Ue` (ү),
//! `E` (е), `I` (digraph «и» = /ij/, kept as a separate phoneme
//! because it surfaces as one orthographic glyph in Cyrillic),
//! `Y` (ы — epenthetic), `Yi` (і — epenthetic), and so on for
//! consonants.

use std::fmt;

/// The 37 phonemes of Kazakh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Phoneme {
    // === Vowels — 8 full ===
    /// /a/ — back open unrounded. Cyrillic «а».
    A,
    /// /æ/ — front open unrounded. Cyrillic «ә».
    Ae,
    /// /o/ — back open rounded. Cyrillic «о».
    O,
    /// /ø/ — front open rounded. Cyrillic «ө».
    Oe,
    /// /ʊ/ — back close rounded. Cyrillic «ұ».
    U,
    /// /y/ — front close rounded. Cyrillic «ү».
    Ue,
    /// /e/ — front mid unrounded. Cyrillic «е».
    E,
    /// /i/ — front close unrounded, long. Cyrillic «и» (digraph;
    /// resolves to `[Yi, J]` in finer-grained analysis but kept as
    /// one phoneme at this layer for orthographic round-trip).
    I,

    // === Vowels — 2 epenthetic ===
    /// /ɯ/ — back close unrounded. Cyrillic «ы». **Epenthetic**:
    /// in most non-initial inter-consonantal positions of native
    /// roots, this segment has no acoustic realisation.
    Y,
    /// /ɪ/ — front close unrounded. Cyrillic «і». **Epenthetic**,
    /// symmetric counterpart to [`Phoneme::Y`].
    Yi,

    // === Consonants — 21 native ===
    /// /p/ — bilabial voiceless stop. Cyrillic «п».
    P,
    /// /b/ — bilabial voiced stop. Cyrillic «б».
    B,
    /// /m/ — bilabial nasal. Cyrillic «м».
    M,
    /// /t/ — dental voiceless stop. Cyrillic «т».
    T,
    /// /d/ — dental voiced stop. Cyrillic «д».
    D,
    /// /s/ — alveolar voiceless fricative. Cyrillic «с».
    S,
    /// /z/ — alveolar voiced fricative. Cyrillic «з».
    Z,
    /// /n/ — alveolar nasal. Cyrillic «н».
    N,
    /// /l/ — alveolar lateral. Cyrillic «л».
    L,
    /// /r/ — alveolar trill. Cyrillic «р».
    R,
    /// /ʃ/ — postalveolar voiceless fricative. Cyrillic «ш».
    Sh,
    /// /ʒ/ — postalveolar voiced fricative. Cyrillic «ж».
    Zh,
    /// /j/ — palatal glide. Cyrillic «й».
    J,
    /// /k/ — velar voiceless stop. Cyrillic «к».
    K,
    /// /g/ — velar voiced stop. Cyrillic «г».
    G,
    /// /ŋ/ — velar nasal. Cyrillic «ң».
    Ng,
    /// /x/ — velar voiceless fricative. Cyrillic «х».
    X,
    /// /q/ — uvular voiceless stop. Cyrillic «қ».
    Q,
    /// /ʁ/ — uvular voiced fricative. Cyrillic «ғ».
    Gh,
    /// /h/ — glottal voiceless fricative. Cyrillic «һ».
    H,
    /// /w/ — labiovelar approximant. Cyrillic «у» (consonantal use).
    W,

    // === Consonants — 5 loan-only ===
    /// /f/ — labiodental voiceless fricative. Cyrillic «ф». Loan.
    F,
    /// /v/ — labiodental voiced fricative. Cyrillic «в». Loan.
    V,
    /// /ts/ — alveolar voiceless affricate. Cyrillic «ц». Loan.
    Ts,
    /// /tʃ/ — postalveolar voiceless affricate. Cyrillic «ч». Loan.
    Ch,
    /// /ɕɕ/ — alveolopalatal voiceless geminate fricative.
    /// Cyrillic «щ». Loan.
    Shch,

    // === Boundary marker — 1 ===
    /// /ʔ/ — glottal stop, used as an internal boundary marker
    /// at compound-word junctions or hiatus avoidance. Has no
    /// Cyrillic glyph; inserted / elided by the phonotactic FST
    /// (Layer 0c).
    Glottal,
}

/// Top-level classification of a phoneme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhonemeClass {
    Vowel,
    Consonant,
    /// Boundary marker, not a sounded segment.
    Boundary,
}

/// Place of articulation (consonants only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Place {
    Bilabial,
    Labiodental,
    Dental,
    Alveolar,
    Postalveolar,
    Alveolopalatal,
    Palatal,
    Velar,
    Labiovelar,
    Uvular,
    Glottal,
}

/// Manner of articulation (consonants only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Manner {
    Stop,
    Fricative,
    Affricate,
    Nasal,
    Lateral,
    Trill,
    Glide,
    Approximant,
}

/// Voicing (consonants only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Voicing {
    Voiceless,
    Voiced,
}

/// Vowel harmony class — the foundational determinism of Kazakh
/// agglutination. Within a native word, vowels do not mix classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HarmonyClass {
    Front,
    Back,
}

/// Vowel height.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Height {
    Open,
    Mid,
    Close,
}

/// Vowel lip rounding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rounding {
    Rounded,
    Unrounded,
}

/// Vowel length (short vs. long; relevant for `I`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Length {
    Short,
    Long,
}

impl Phoneme {
    /// All 37 phonemes, in declaration order. Useful for table
    /// generation and exhaustive tests.
    pub const ALL: &'static [Phoneme] = &[
        Phoneme::A,
        Phoneme::Ae,
        Phoneme::O,
        Phoneme::Oe,
        Phoneme::U,
        Phoneme::Ue,
        Phoneme::E,
        Phoneme::I,
        Phoneme::Y,
        Phoneme::Yi,
        Phoneme::P,
        Phoneme::B,
        Phoneme::M,
        Phoneme::T,
        Phoneme::D,
        Phoneme::S,
        Phoneme::Z,
        Phoneme::N,
        Phoneme::L,
        Phoneme::R,
        Phoneme::Sh,
        Phoneme::Zh,
        Phoneme::J,
        Phoneme::K,
        Phoneme::G,
        Phoneme::Ng,
        Phoneme::X,
        Phoneme::Q,
        Phoneme::Gh,
        Phoneme::H,
        Phoneme::W,
        Phoneme::F,
        Phoneme::V,
        Phoneme::Ts,
        Phoneme::Ch,
        Phoneme::Shch,
        Phoneme::Glottal,
    ];

    /// IPA transcription as a `&'static str`.
    pub fn ipa(self) -> &'static str {
        use Phoneme::*;
        match self {
            A => "a",
            Ae => "æ",
            O => "o",
            Oe => "ø",
            U => "ʊ",
            Ue => "y",
            E => "e",
            I => "i",
            Y => "ɯ",
            Yi => "ɪ",
            P => "p",
            B => "b",
            M => "m",
            T => "t",
            D => "d",
            S => "s",
            Z => "z",
            N => "n",
            L => "l",
            R => "r",
            Sh => "ʃ",
            Zh => "ʒ",
            J => "j",
            K => "k",
            G => "g",
            Ng => "ŋ",
            X => "x",
            Q => "q",
            Gh => "ʁ",
            H => "h",
            W => "w",
            F => "f",
            V => "v",
            Ts => "ts",
            Ch => "tʃ",
            Shch => "ɕɕ",
            Glottal => "ʔ",
        }
    }

    /// Top-level class.
    pub fn class(self) -> PhonemeClass {
        use Phoneme::*;
        match self {
            A | Ae | O | Oe | U | Ue | E | I | Y | Yi => PhonemeClass::Vowel,
            Glottal => PhonemeClass::Boundary,
            _ => PhonemeClass::Consonant,
        }
    }

    /// `true` for vowel variants (full or epenthetic).
    pub fn is_vowel(self) -> bool {
        matches!(self.class(), PhonemeClass::Vowel)
    }

    /// `true` for consonant variants (native or loan).
    pub fn is_consonant(self) -> bool {
        matches!(self.class(), PhonemeClass::Consonant)
    }

    /// `true` for the two epenthetic vowels (`Y` = ы, `Yi` = і).
    /// **Epenthetic** means the segment is acoustically null /
    /// minimal in most positions and is treated as an
    /// orthographic marker, not a sounded vowel. See
    /// [`docs/v6_3_phonemic_foundation.md`](../../../docs/v6_3_phonemic_foundation.md)
    /// §9 OQ4 for the position-dependent realisation rule.
    pub fn is_epenthetic(self) -> bool {
        matches!(self, Phoneme::Y | Phoneme::Yi)
    }

    /// `true` for loan-only consonants (`F V Ts Ch Shch`). Their
    /// appearance in a claimed-native lexicon entry should warn.
    pub fn is_loan(self) -> bool {
        matches!(
            self,
            Phoneme::F | Phoneme::V | Phoneme::Ts | Phoneme::Ch | Phoneme::Shch
        )
    }

    /// Vowel harmony class (`Front` / `Back`). `None` for
    /// non-vowels.
    pub fn harmony_class(self) -> Option<HarmonyClass> {
        use Phoneme::*;
        match self {
            A | O | U | Y => Some(HarmonyClass::Back),
            Ae | Oe | Ue | E | I | Yi => Some(HarmonyClass::Front),
            _ => None,
        }
    }

    /// Vowel height. `None` for non-vowels.
    pub fn height(self) -> Option<Height> {
        use Phoneme::*;
        match self {
            A | Ae | O | Oe => Some(Height::Open),
            E => Some(Height::Mid),
            U | Ue | I | Y | Yi => Some(Height::Close),
            _ => None,
        }
    }

    /// Vowel rounding. `None` for non-vowels.
    pub fn rounding(self) -> Option<Rounding> {
        use Phoneme::*;
        match self {
            O | Oe | U | Ue => Some(Rounding::Rounded),
            A | Ae | E | I | Y | Yi => Some(Rounding::Unrounded),
            _ => None,
        }
    }

    /// Vowel length. `None` for non-vowels. Only `I` is long at
    /// this layer; refinement may follow in Phase 4.
    pub fn length(self) -> Option<Length> {
        use Phoneme::*;
        match self {
            I => Some(Length::Long),
            A | Ae | O | Oe | U | Ue | E | Y | Yi => Some(Length::Short),
            _ => None,
        }
    }

    /// Place of articulation. `None` for vowels and the boundary
    /// marker.
    pub fn place(self) -> Option<Place> {
        use Phoneme::*;
        match self {
            P | B | M => Some(Place::Bilabial),
            F | V => Some(Place::Labiodental),
            T | D => Some(Place::Dental),
            S | Z | N | L | R | Ts => Some(Place::Alveolar),
            Sh | Zh | Ch => Some(Place::Postalveolar),
            Shch => Some(Place::Alveolopalatal),
            J => Some(Place::Palatal),
            K | G | Ng | X => Some(Place::Velar),
            W => Some(Place::Labiovelar),
            Q | Gh => Some(Place::Uvular),
            H => Some(Place::Glottal),
            _ => None,
        }
    }

    /// Manner of articulation. `None` for vowels and the boundary
    /// marker.
    pub fn manner(self) -> Option<Manner> {
        use Phoneme::*;
        match self {
            P | B | T | D | K | G | Q => Some(Manner::Stop),
            F | V | S | Z | Sh | Zh | X | Gh | H | Shch => Some(Manner::Fricative),
            Ts | Ch => Some(Manner::Affricate),
            M | N | Ng => Some(Manner::Nasal),
            L => Some(Manner::Lateral),
            R => Some(Manner::Trill),
            J => Some(Manner::Glide),
            W => Some(Manner::Approximant),
            _ => None,
        }
    }

    /// Voicing. `None` for vowels and the boundary marker.
    pub fn voicing(self) -> Option<Voicing> {
        use Phoneme::*;
        match self {
            P | T | K | Q | S | Sh | X | H | F | Ts | Ch | Shch => Some(Voicing::Voiceless),
            B | D | G | Gh | Z | Zh | M | N | Ng | L | R | J | W | V => Some(Voicing::Voiced),
            _ => None,
        }
    }

    /// Default Cyrillic glyph for this phoneme (single-character
    /// projection, lossy at compound-grapheme positions). `None`
    /// for the boundary marker.
    pub fn cyrillic_glyph(self) -> Option<char> {
        use Phoneme::*;
        Some(match self {
            A => 'а',
            Ae => 'ә',
            O => 'о',
            Oe => 'ө',
            U => 'ұ',
            Ue => 'ү',
            E => 'е',
            I => 'и',
            Y => 'ы',
            Yi => 'і',
            P => 'п',
            B => 'б',
            M => 'м',
            T => 'т',
            D => 'д',
            S => 'с',
            Z => 'з',
            N => 'н',
            L => 'л',
            R => 'р',
            Sh => 'ш',
            Zh => 'ж',
            J => 'й',
            K => 'к',
            G => 'г',
            Ng => 'ң',
            X => 'х',
            Q => 'қ',
            Gh => 'ғ',
            H => 'һ',
            W => 'у',
            F => 'ф',
            V => 'в',
            Ts => 'ц',
            Ch => 'ч',
            Shch => 'щ',
            Glottal => return None,
        })
    }
}

impl fmt::Display for Phoneme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "/{}/", self.ipa())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `ALL` table contains every variant exactly once.
    #[test]
    fn all_table_has_37_unique_entries() {
        assert_eq!(Phoneme::ALL.len(), 37);
        let mut seen = std::collections::HashSet::new();
        for p in Phoneme::ALL {
            assert!(seen.insert(*p), "duplicate in ALL: {p:?}");
        }
    }

    /// Every vowel has the four vowel-specific attributes; every
    /// consonant has the three consonant-specific attributes; the
    /// boundary has none.
    #[test]
    fn attributes_partition_correctly() {
        for &p in Phoneme::ALL {
            match p.class() {
                PhonemeClass::Vowel => {
                    assert!(p.harmony_class().is_some(), "{p:?} vowel missing harmony");
                    assert!(p.height().is_some(), "{p:?} vowel missing height");
                    assert!(p.rounding().is_some(), "{p:?} vowel missing rounding");
                    assert!(p.length().is_some(), "{p:?} vowel missing length");
                    assert!(p.place().is_none(), "{p:?} vowel has place");
                    assert!(p.manner().is_none(), "{p:?} vowel has manner");
                    assert!(p.voicing().is_none(), "{p:?} vowel has voicing");
                }
                PhonemeClass::Consonant => {
                    assert!(p.place().is_some(), "{p:?} consonant missing place");
                    assert!(p.manner().is_some(), "{p:?} consonant missing manner");
                    assert!(p.voicing().is_some(), "{p:?} consonant missing voicing");
                    assert!(p.harmony_class().is_none(), "{p:?} consonant has harmony");
                }
                PhonemeClass::Boundary => {
                    assert!(p.place().is_none());
                    assert!(p.manner().is_none());
                    assert!(p.voicing().is_none());
                    assert!(p.harmony_class().is_none());
                    assert!(p.cyrillic_glyph().is_none());
                }
            }
        }
    }

    /// Exactly two phonemes are epenthetic: `Y` and `Yi`.
    #[test]
    fn exactly_two_epenthetic_phonemes() {
        let epenthetic: Vec<_> = Phoneme::ALL.iter().filter(|p| p.is_epenthetic()).collect();
        assert_eq!(epenthetic, vec![&Phoneme::Y, &Phoneme::Yi]);
    }

    /// Exactly five phonemes are loan-only.
    #[test]
    fn exactly_five_loan_consonants() {
        let loan: Vec<_> = Phoneme::ALL.iter().filter(|p| p.is_loan()).collect();
        assert_eq!(loan.len(), 5);
        for p in loan {
            assert!(p.is_consonant(), "{p:?} loan flag on non-consonant");
        }
    }

    /// 10 vowels + 27 consonants = 37. (The boundary marker counts
    /// as a consonant-class phoneme for inventory purposes? No —
    /// it is its own class.) Re-check the documented split:
    /// 10 vowel + 26 sounded consonant + 1 boundary = 37.
    #[test]
    fn inventory_split_matches_design_doc() {
        let n_vowel = Phoneme::ALL.iter().filter(|p| p.is_vowel()).count();
        let n_consonant = Phoneme::ALL.iter().filter(|p| p.is_consonant()).count();
        let n_boundary = Phoneme::ALL
            .iter()
            .filter(|p| matches!(p.class(), PhonemeClass::Boundary))
            .count();
        assert_eq!(n_vowel, 10, "expected 10 vowels (8 full + 2 epenthetic)");
        assert_eq!(
            n_consonant, 26,
            "expected 26 sounded consonants (21 native + 5 loan)"
        );
        assert_eq!(n_boundary, 1, "expected 1 boundary marker");
        assert_eq!(n_vowel + n_consonant + n_boundary, 37);
    }

    /// Harmony classes split into 4 back vowels + 6 front vowels.
    #[test]
    fn harmony_class_distribution() {
        let back = Phoneme::ALL
            .iter()
            .filter(|p| p.harmony_class() == Some(HarmonyClass::Back))
            .count();
        let front = Phoneme::ALL
            .iter()
            .filter(|p| p.harmony_class() == Some(HarmonyClass::Front))
            .count();
        // Back: A, O, U, Y
        // Front: Ae, Oe, Ue, E, I, Yi
        assert_eq!(back, 4);
        assert_eq!(front, 6);
        assert_eq!(back + front, 10);
    }

    /// IPA strings are unique across phonemes (no two phonemes
    /// share the same IPA transcription).
    #[test]
    fn ipa_strings_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for p in Phoneme::ALL {
            assert!(seen.insert(p.ipa()), "duplicate IPA: {} on {p:?}", p.ipa());
        }
    }

    /// Cyrillic glyphs are unique across phonemes that have them.
    #[test]
    fn cyrillic_glyphs_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for p in Phoneme::ALL {
            if let Some(g) = p.cyrillic_glyph() {
                assert!(seen.insert(g), "duplicate glyph {g} on {p:?}");
            }
        }
    }
}
