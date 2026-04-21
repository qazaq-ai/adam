//! Phonology — resolution of abstract archiphonemes to concrete surface letters.
//!
//! Apertium's 54 two-level rules collapse into ~12 `realise_*` functions
//! here, each consuming a `(Archiphoneme, PhonologicalContext)` and returning
//! exactly one surface `char` (or a deletion, represented by `None`).
//!
//! Rule catalogue with Apertium cross-references:
//! `docs/kazakh_grammar/06_apertium_twol_catalogue.md`.
//!
//! This file is **scaffold** — types and function signatures present;
//! implementations are stubs (`todo!()` or hardcoded defaults). Tests are
//! marked `#[ignore]` where not yet satisfiable and will be un-ignored as
//! rules are ported (Tue–Fri of week 1).

use serde::{Deserialize, Serialize};

/// Abstract underlying phoneme used in suffix representations. When a suffix
/// is written in the lexicon as `{D}{I}{K}` (the 1pl past tense ending in
/// abstract form), each of these resolves to one surface letter depending on
/// the phonological context of the preceding stem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Archiphoneme {
    /// Abstract `A` — realises as `а` (back harmony) or `е` (front harmony).
    /// Used in: plural `-{l}{A}r`, dative `-{G}{A}`, ablative `-{D}{A}n`, etc.
    A,
    /// Abstract `I` — realises as `ы` (back) or `і` (front). Used in buffer
    /// vowels and in possessive `-{I}m`, etc.
    I,
    /// Abstract `D` — realises as `д` (voiced), `т` (voiceless), or `н` (nasal
    /// harmony context).
    D,
    /// Abstract `L` — realises as `л` (default), `д` (after voiced/nasal),
    /// `т` (after voiceless).
    L,
    /// Abstract `M` — realises as `м` (default), `б` (after voiced/nasal),
    /// `п` (after voiceless).
    M,
    /// Abstract `N` — realises as `н` (default), `д` (after voiced non-nasal),
    /// `т` (after voiceless).
    N,
    /// Abstract `G` — realises as `ғ` (back, voiced), `г` (front, voiced),
    /// `қ` (back, voiceless), `к` (front, voiceless).
    G,
    /// Abstract `K` — realises as `қ` (back, voiceless), `к` (front,
    /// voiceless), `ғ`/`г` (voiced neighbours).
    K,
    /// Abstract `S` — buffer `с` that deletes after a consonant.
    S,
    /// Abstract `Y` — buffer `ы/і` that deletes after a vowel.
    Y,
    /// Buffer `{n}` — pronominal-н that appears between 3rd-person possessive
    /// and some cases; deletes in other contexts per rules 39–45.
    NBuf,
}

/// Vowel harmony class — the single most important phonological feature of a
/// Kazakh root. Every suffix vowel has to agree with this class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VowelClass {
    /// Last vowel in the stem is back-harmonic: а, о, ы, ұ, (у as back), я, ё.
    Back,
    /// Last vowel in the stem is front-harmonic: ә, е, і, ө, ү, э.
    Front,
}

/// Consonant class of the preceding segment. Drives the voicing / nasal
/// assimilation rules A–C from the catalogue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConsonantClass {
    /// Not a consonant — the preceding segment is a vowel.
    VowelPreceding,
    /// Voiceless obstruent: к, қ, п, с, т, ф, х, ш, щ, ц, ч, һ.
    Voiceless,
    /// Voiced non-sonorant: б, в, г, ғ, д, ж, з.
    VoicedObstruent,
    /// Nasal: м, н, ң.
    Nasal,
    /// Liquid: л.
    Liquid,
    /// High sonorant: й, у, р, и, ю.
    HighSonorant,
}

/// A snapshot of the phonological environment around the realisation point.
/// This is what each `realise_*` function reads from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhonologicalContext {
    /// Vowel harmony class of the stem (computed once per stem).
    pub harmony: VowelClass,
    /// Class of the immediately preceding segment (what comes right before
    /// the archiphoneme being realised).
    pub preceding: ConsonantClass,
    /// Whether the stem contains a nasal anywhere (affects `{D}` nasal
    /// harmony rule — catalogue rule 4).
    pub stem_has_nasal: bool,
    /// Whether the last segment is specifically `й` or `и` (catalogue rules
    /// 21–24 override vowel harmony here).
    pub preceded_by_y_or_i: bool,
}

/// Resolution of an archiphoneme to a concrete surface character.
/// Returns `None` when the rule produces a zero (deletion).
///
/// This is the primary entry point for phonology. Internally it dispatches
/// to one of the `realise_*` helpers per archiphoneme.
pub fn realise_archiphoneme(arch: Archiphoneme, ctx: PhonologicalContext) -> Option<char> {
    match arch {
        Archiphoneme::A => Some(realise_a(ctx)),
        Archiphoneme::I => Some(realise_i(ctx)),
        Archiphoneme::D => Some(realise_d(ctx)),
        Archiphoneme::L => Some(realise_l(ctx)),
        Archiphoneme::M => Some(realise_m(ctx)),
        Archiphoneme::N => Some(realise_n(ctx)),
        Archiphoneme::G => Some(realise_g(ctx)),
        Archiphoneme::K => Some(realise_k(ctx)),
        Archiphoneme::S => realise_s_buffer(ctx),
        Archiphoneme::Y => realise_y_buffer(ctx),
        Archiphoneme::NBuf => realise_n_buffer(ctx),
    }
}

// ---------------------------------------------------------------------------
// Vowel harmony rules (catalogue group D — rules 13–24).
// ---------------------------------------------------------------------------

/// Archiphoneme `{A}` — catalogue rules 14, 21, 23.
pub fn realise_a(ctx: PhonologicalContext) -> char {
    match ctx.harmony {
        VowelClass::Back => 'а',
        VowelClass::Front => 'е',
    }
    // TODO: rule 21 (back harmony after и) overrides the above in specific
    //       contexts. Will be added when test cases are added.
}

/// Archiphoneme `{I}` — catalogue rule 13.
pub fn realise_i(ctx: PhonologicalContext) -> char {
    match ctx.harmony {
        VowelClass::Back => 'ы',
        VowelClass::Front => 'і',
    }
}

// ---------------------------------------------------------------------------
// Consonant assimilation rules (catalogue groups A, B, C — rules 1–12).
// ---------------------------------------------------------------------------

/// Archiphoneme `{D}` — catalogue rules 4, 5, 7.
pub fn realise_d(ctx: PhonologicalContext) -> char {
    // Rule 4 (D nasal harmony) full condition is `:Nasals _ :Vow :Nasals` —
    // the abstract {D} becomes н only when flanked on the LEFT by a nasal
    // AND on the RIGHT by Vow+Nasal. Since we walk atoms left-to-right we
    // don't have forward lookahead yet; in the vast majority of real cases
    // the left-only "nasal→нn" heuristic produces the wrong output (e.g.
    // адам+LOC → *адамна, should be адамда). Disable it for now and revisit
    // when we implement two-pass FST with lookahead (week 2 target).
    // Rule 5 (forward voicing): after voiceless → т.
    if matches!(ctx.preceding, ConsonantClass::Voiceless) {
        return 'т';
    }
    'д'
}

/// Archiphoneme `{L}` — catalogue rules 2, 5.
pub fn realise_l(ctx: PhonologicalContext) -> char {
    if matches!(ctx.preceding, ConsonantClass::Voiceless) {
        return 'т';
    }
    if matches!(
        ctx.preceding,
        ConsonantClass::Nasal | ConsonantClass::Liquid | ConsonantClass::VoicedObstruent
    ) {
        return 'д';
    }
    'л'
}

/// Archiphoneme `{M}` — catalogue rules 3, 5.
pub fn realise_m(ctx: PhonologicalContext) -> char {
    if matches!(ctx.preceding, ConsonantClass::Voiceless) {
        return 'п';
    }
    // After voiced obstruent the stop becomes `б` (мектеб+мен → doesn't
    // happen — voiced obstruent finals are rare). After a nasal (м/н/ң)
    // the suffix stays `м`: мұғалім+инстр = мұғаліммен (NOT
    // мұғалімбен, fixed in v0.9.9). Vowels, sonorants → also `м`.
    if matches!(ctx.preceding, ConsonantClass::VoicedObstruent) {
        return 'б';
    }
    'м'
}

/// Archiphoneme `{N}` — catalogue rules 1, 5.
pub fn realise_n(ctx: PhonologicalContext) -> char {
    if matches!(ctx.preceding, ConsonantClass::Voiceless) {
        return 'т';
    }
    if matches!(
        ctx.preceding,
        ConsonantClass::HighSonorant
            | ConsonantClass::Liquid
            | ConsonantClass::Nasal
            | ConsonantClass::VoicedObstruent
    ) {
        return 'д';
    }
    'н'
}

/// Archiphoneme `{G}` — catalogue rules 6, 7, 26, 27.
pub fn realise_g(ctx: PhonologicalContext) -> char {
    let voiced = !matches!(ctx.preceding, ConsonantClass::Voiceless);
    match (ctx.harmony, voiced) {
        (VowelClass::Back, true) => 'ғ',
        (VowelClass::Back, false) => 'қ',
        (VowelClass::Front, true) => 'г',
        (VowelClass::Front, false) => 'к',
    }
}

/// Archiphoneme `{K}` — catalogue rules 27, 28, 29.
///
/// Simplification for week 1: {K} stays voiceless by default (`қ`/`к`). The
/// intervocalic-voicing rule (29 "Voicing of K") applies only when the {K}
/// is followed by a vowel (`_ V`), which we cannot see yet with single-pass
/// left-to-right realisation. The single-sided heuristic (voice after any
/// preceding vowel) breaks word-final `{K}` (e.g., 1pl past `жаздық`). We
/// prefer the under-voicing default; the intervocalic case will be handled
/// by a post-pass in week 2.
pub fn realise_k(ctx: PhonologicalContext) -> char {
    match ctx.harmony {
        VowelClass::Back => 'қ',
        VowelClass::Front => 'к',
    }
}

// ---------------------------------------------------------------------------
// Buffer realisations (catalogue group G + rules 44–45).
// ---------------------------------------------------------------------------

/// Buffer `с` inserted in 3rd-person possessive on vowel-final stems.
/// Catalogue rule 34 (deletion after a consonant).
pub fn realise_s_buffer(ctx: PhonologicalContext) -> Option<char> {
    if matches!(ctx.preceding, ConsonantClass::VowelPreceding) {
        Some('с')
    } else {
        None
    }
}

/// Buffer `ы/і` before certain suffixes on consonant-final stems.
/// Catalogue rule 35 (deletion after a vowel).
pub fn realise_y_buffer(ctx: PhonologicalContext) -> Option<char> {
    if matches!(ctx.preceding, ConsonantClass::VowelPreceding) {
        None
    } else {
        Some(realise_i(ctx))
    }
}

/// Pronominal-н buffer between 3rd-person possessive and certain cases.
/// Catalogue rules 39–45 all concern this buffer. Partial implementation.
pub fn realise_n_buffer(_ctx: PhonologicalContext) -> Option<char> {
    Some('н')
}

// ---------------------------------------------------------------------------
// Helper: classify a single Cyrillic character into its phonological class.
// Used by the lexicon loader and by the morphotactics module to build
// `PhonologicalContext` instances.
// ---------------------------------------------------------------------------

/// Classify a character. Returns `None` for punctuation / non-letter input.
///
/// v2.3: **glide-vowels у, и, ю** moved from `VowelPreceding` to
/// `HighSonorant`, matching the enum docstring ("High sonorant: й, у,
/// р, и, ю"). Rationale: these letters spell the consonantal glides
/// [w] and [j] in Kazakh; they pattern with consonants for several
/// morphophonological rules, not with true vowels.
///
/// Observable effects of the fix:
///
///   - `realise_s_buffer` no longer inserts с after у/и/ю → `оқу+P3` =
///     `оқуы` (not `оқусы`), `бастау+P3` = `бастауы` (not `бастаусы`).
///   - `realise_y_buffer` now inserts ы/і after у/и/ю → `оқу+P1SG`
///     correctly produces `оқуым` instead of `оқум`.
///   - `realise_n` now returns `д` after у/и/ю (HighSonorant path).
///     Dative/genitive are synthesised by `{G}` / `{N}` archiphonemes
///     whose `HighSonorant` branch already matched existing corpus
///     forms, so this is a pure correction.
pub fn classify_char(c: char) -> Option<ConsonantClass> {
    let c = c.to_lowercase().next()?;
    Some(match c {
        'а' | 'ә' | 'е' | 'ё' | 'і' | 'о' | 'ө' | 'ұ' | 'ү' | 'ы' | 'э' | 'я' => {
            ConsonantClass::VowelPreceding
        }
        'к' | 'қ' | 'п' | 'с' | 'т' | 'ф' | 'х' | 'ш' | 'щ' | 'ц' | 'ч' | 'һ' => {
            ConsonantClass::Voiceless
        }
        'б' | 'в' | 'г' | 'ғ' | 'д' | 'ж' | 'з' => ConsonantClass::VoicedObstruent,
        'м' | 'н' | 'ң' => ConsonantClass::Nasal,
        'л' => ConsonantClass::Liquid,
        // у / и / ю are glide vowels — spelt as letters, pattern as
        // consonants for P3 с-buffer and Y-buffer alternation. Moved
        // here from VowelPreceding in v2.3.
        'й' | 'р' | 'у' | 'и' | 'ю' => ConsonantClass::HighSonorant,
        _ => return None,
    })
}

/// Test whether a character is a vowel.
pub fn is_vowel(c: char) -> bool {
    matches!(
        c.to_lowercase().next().unwrap_or(c),
        'а' | 'ә' | 'е' | 'ё' | 'и' | 'і' | 'о' | 'ө' | 'у' | 'ұ' | 'ү' | 'ы' | 'э' | 'ю' | 'я'
    )
}

/// Catalogue rules 10–12 — intervocalic voicing of `п`, `к`, `қ`.
///
/// When we just appended a vowel to `out` and the character *before* the
/// last consonant is also a vowel, the now-intervocalic voiceless obstruent
/// at position `out[-2]` voices:
///   - п → б  (мектеп + ім → мектебім)
///   - к → г  (білек + ім → білегім)
///   - қ → ғ  (тарақ + ым → тарағым)
///
/// Apply *in place* to an existing string. Idempotent for any already-voiced
/// forms. Called from `Accumulator` whenever a vowel-initial atom lands.
pub fn apply_intervocalic_voicing(out: &mut String) {
    let chars: Vec<char> = out.chars().collect();
    let n = chars.len();
    if n < 3 {
        return;
    }
    let last = chars[n - 1];
    let mid = chars[n - 2];
    let before_mid = chars[n - 3];
    if !is_vowel(last) || !is_vowel(before_mid) {
        return;
    }
    let voiced = match mid {
        'п' => Some('б'),
        'к' => Some('г'),
        'қ' => Some('ғ'),
        _ => None,
    };
    if let Some(v) = voiced {
        // Rebuild string with the swap.
        let mut rebuilt = String::with_capacity(out.len());
        for (i, c) in chars.iter().enumerate() {
            if i == n - 2 {
                rebuilt.push(v);
            } else {
                rebuilt.push(*c);
            }
        }
        *out = rebuilt;
    }
}

/// Determine vowel harmony from a stem by looking at its last vowel.
pub fn stem_vowel_harmony(stem: &str) -> VowelClass {
    for c in stem.chars().rev() {
        match c.to_lowercase().next().unwrap_or(c) {
            'а' | 'о' | 'ұ' | 'ы' | 'я' | 'ё' => return VowelClass::Back,
            'ә' | 'е' | 'ө' | 'ү' | 'і' | 'э' => return VowelClass::Front,
            // 'у' and 'и' are ambiguous — fall through to check earlier
            // vowels. Apertium marks this explicitly as a FIXME.
            _ => continue,
        }
    }
    // Default to back when stem has no determinable vowels (proper nouns,
    // digit sequences, etc.).
    VowelClass::Back
}

#[cfg(test)]
mod tests {
    use super::*;

    fn back_after_voiceless() -> PhonologicalContext {
        PhonologicalContext {
            harmony: VowelClass::Back,
            preceding: ConsonantClass::Voiceless,
            stem_has_nasal: false,
            preceded_by_y_or_i: false,
        }
    }

    fn back_after_vowel() -> PhonologicalContext {
        PhonologicalContext {
            harmony: VowelClass::Back,
            preceding: ConsonantClass::VowelPreceding,
            stem_has_nasal: false,
            preceded_by_y_or_i: false,
        }
    }

    fn front_after_voiced() -> PhonologicalContext {
        PhonologicalContext {
            harmony: VowelClass::Front,
            preceding: ConsonantClass::VoicedObstruent,
            stem_has_nasal: false,
            preceded_by_y_or_i: false,
        }
    }

    #[test]
    fn a_harmony_basic() {
        assert_eq!(realise_a(back_after_voiceless()), 'а');
        assert_eq!(realise_a(front_after_voiced()), 'е');
    }

    #[test]
    fn i_harmony_basic() {
        assert_eq!(realise_i(back_after_voiceless()), 'ы');
        assert_eq!(realise_i(front_after_voiced()), 'і');
    }

    #[test]
    fn d_after_voiceless_is_t() {
        // мектеп + {D}{A}n → мектеп + тен (ablative)
        assert_eq!(realise_d(back_after_voiceless()), 'т');
    }

    #[test]
    fn d_elsewhere_is_d() {
        // бала + {D}{A}n → бала + дан (ablative)
        assert_eq!(realise_d(back_after_vowel()), 'д');
    }

    #[test]
    fn l_after_voiceless_is_t() {
        // мектеп + {L}{A}r → мектеп + тер (plural)
        assert_eq!(realise_l(back_after_voiceless()), 'т');
    }

    #[test]
    fn l_after_nasal_is_d() {
        let ctx = PhonologicalContext {
            harmony: VowelClass::Back,
            preceding: ConsonantClass::Nasal,
            stem_has_nasal: true,
            preceded_by_y_or_i: false,
        };
        // адам + {L}{A}r → адам + дар
        assert_eq!(realise_l(ctx), 'д');
    }

    #[test]
    fn g_back_voiced_is_gh() {
        // бала + {G}{A} → бала + ға (dative)
        assert_eq!(realise_g(back_after_vowel()), 'ғ');
    }

    #[test]
    fn g_back_voiceless_is_q() {
        // мектеп + {G}{A} → мектеп + қа (dative)
        assert_eq!(realise_g(back_after_voiceless()), 'қ');
    }

    #[test]
    fn g_front_voiceless_is_k() {
        let ctx = PhonologicalContext {
            harmony: VowelClass::Front,
            preceding: ConsonantClass::Voiceless,
            stem_has_nasal: false,
            preceded_by_y_or_i: false,
        };
        // іс + {G}{A} → іс + ке (dative)
        assert_eq!(realise_g(ctx), 'к');
    }

    #[test]
    fn s_buffer_only_after_vowel() {
        // бала + {S}{I} → бала + сы (3sg possessive)
        assert_eq!(realise_s_buffer(back_after_vowel()), Some('с'));
        // мектеп + {S}{I} → мектеп + і (no buffer, just vowel)
        assert_eq!(realise_s_buffer(back_after_voiceless()), None);
    }

    #[test]
    fn stem_harmony_by_last_vowel() {
        assert_eq!(stem_vowel_harmony("бала"), VowelClass::Back);
        assert_eq!(stem_vowel_harmony("мектеп"), VowelClass::Front);
        assert_eq!(stem_vowel_harmony("ел"), VowelClass::Front);
        assert_eq!(stem_vowel_harmony("адам"), VowelClass::Back);
    }

    #[test]
    fn classify_basic_letters() {
        assert_eq!(classify_char('а'), Some(ConsonantClass::VowelPreceding));
        assert_eq!(classify_char('к'), Some(ConsonantClass::Voiceless));
        assert_eq!(classify_char('б'), Some(ConsonantClass::VoicedObstruent));
        assert_eq!(classify_char('н'), Some(ConsonantClass::Nasal));
        assert_eq!(classify_char('л'), Some(ConsonantClass::Liquid));
        assert_eq!(classify_char('й'), Some(ConsonantClass::HighSonorant));
        assert_eq!(classify_char(','), None);
    }
}
