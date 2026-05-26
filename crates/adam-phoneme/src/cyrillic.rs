// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Bidirectional Cyrillic ↔ Phoneme renderer (Layer 0d, partial).
//!
//! Implements the two projection functions from the v6.3 design:
//!
//! - [`phonemes_to_cyrillic`] — phoneme stream → Cyrillic glyph
//!   stream. Lossy at compound graphemes; default 1-to-1 mapping
//!   per [`crate::Phoneme::cyrillic_glyph`].
//! - [`cyrillic_to_phonemes`] — Cyrillic text → phoneme stream.
//!   Applies the «ы» / «і» epenthetic rule from
//!   [`docs/v6_3_phonemic_foundation.md`](../../../docs/v6_3_phonemic_foundation.md)
//!   §9 OQ4 when the caller marks the input as a native Kazakh
//!   root.
//!
//! ## What this layer does NOT do (yet)
//!
//! - Compound-grapheme handling for «и» (= `[Yi, J]`) and the
//!   «у»-as-glide / «у»-as-vowel ambiguity. Phase 1 keeps the
//!   default mapping `и → I`, `у → W`; round-trip lossiness here
//!   is documented and addressed in Phase 4.
//! - Compound-word boundary detection (would insert
//!   [`crate::Phoneme::Glottal`] at junctions). Deferred to
//!   Layer 0c (phonotactic FST).
//! - Loan-word detection. The caller passes `is_native_root`
//!   explicitly; lexicon-driven detection is Phase 8+.

use crate::Phoneme;

/// Project a phoneme stream to Cyrillic. Boundary markers
/// ([`Phoneme::Glottal`]) are silently elided.
///
/// **Lossy by design** at the «и» / «у» ambiguity points; not
/// guaranteed to be the inverse of [`cyrillic_to_phonemes`].
pub fn phonemes_to_cyrillic(phonemes: &[Phoneme]) -> String {
    let mut out = String::with_capacity(phonemes.len());
    for p in phonemes {
        if let Some(g) = p.cyrillic_glyph() {
            out.push(g);
        }
    }
    out
}

/// Project a Cyrillic word to a phoneme stream.
///
/// When `is_native_root` is `true`, the epenthetic rule from
/// design doc §9 OQ4 applies: a «ы» or «і» letter is **dropped
/// from the output stream** (treated as orthographic-only) iff
///   1. it is in a **non-initial syllable** (i.e. some vowel has
///      already been emitted into the output stream before it), AND
///   2. its immediate Cyrillic neighbours both map to consonants.
///
/// The first condition is **«non-initial syllable»**, not
/// «non-initial position»: «мысал» («example») has «ы» at
/// position 1 but in the first syllable (no vowel precedes it),
/// so it stays full. «жұмыс» («work») has «ы» at position 3 in
/// the second syllable (the `U` from «ұ» precedes it), so it
/// drops.
///
/// When `is_native_root` is `false` (Russian / European loans),
/// every «ы» becomes [`Phoneme::Y`] and every «і» becomes
/// [`Phoneme::Yi`] regardless of position.
///
/// Non-Kazakh, non-letter input characters (punctuation,
/// whitespace, digits, unknown Cyrillic) are silently skipped.
pub fn cyrillic_to_phonemes(text: &str, is_native_root: bool) -> Vec<Phoneme> {
    let lowered = text.to_lowercase();
    let chars: Vec<char> = lowered.chars().collect();
    let mut out: Vec<Phoneme> = Vec::with_capacity(chars.len());

    for (i, c) in chars.iter().enumerate() {
        let p = match cyrillic_char_to_phoneme(*c) {
            Some(p) => p,
            None => continue,
        };

        // Epenthetic rule (§9 OQ4). The first criterion is
        // «non-initial syllable» — equivalent to «some vowel has
        // already been emitted to `out`». The second criterion
        // is «both neighbours map to consonants».
        if is_native_root && matches!(p, Phoneme::Y | Phoneme::Yi) {
            let in_non_initial_syllable = out.iter().any(|p| p.is_vowel());
            let prev_is_consonant = out.last().is_some_and(|p| p.is_consonant());
            let next_is_consonant = chars
                .get(i + 1)
                .and_then(|&c| cyrillic_char_to_phoneme(c))
                .is_some_and(|p| p.is_consonant());

            if in_non_initial_syllable && prev_is_consonant && next_is_consonant {
                // Drop — orthographic epenthetic position.
                continue;
            }
        }

        out.push(p);
    }

    out
}

/// Single-character Cyrillic→Phoneme lookup. Returns `None` for
/// non-Kazakh characters (whitespace, digits, punctuation,
/// foreign letters). Used internally; exposed for tests.
pub fn cyrillic_char_to_phoneme(c: char) -> Option<Phoneme> {
    use Phoneme::*;
    Some(match c {
        // Full vowels.
        'а' => A,
        'ә' => Ae,
        'о' => O,
        'ө' => Oe,
        'ұ' => U,
        'ү' => Ue,
        'е' => E,
        'и' => I,
        // Epenthetic vowels (post-rule filtering may drop them).
        'ы' => Y,
        'і' => Yi,
        // Native consonants.
        'п' => P,
        'б' => B,
        'м' => M,
        'т' => T,
        'д' => D,
        'с' => S,
        'з' => Z,
        'н' => N,
        'л' => L,
        'р' => R,
        'ш' => Sh,
        'ж' => Zh,
        'й' => J,
        'к' => K,
        'г' => G,
        'ң' => Ng,
        'х' => X,
        'қ' => Q,
        'ғ' => Gh,
        'һ' => H,
        // «у» — provisional default: treat as W (glide). Real
        // vowel / glide disambiguation is a Phase 4 refinement.
        'у' => W,
        // Loan-only consonants.
        'ф' => F,
        'в' => V,
        'ц' => Ts,
        'ч' => Ch,
        'щ' => Shch,
        // Russian orthographic markers — silent.
        'ъ' | 'ь' => return None,
        // Russian loan letters that surface in mixed text.
        'э' => E,
        'ё' => O, // approximation; lossy
        'ю' => U, // approximation; lossy (should expand to [J, U])
        'я' => A, // approximation; lossy (should expand to [J, A])
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use Phoneme::*;

    /// Round-trip of a fully-spelled-out phoneme list through
    /// [`phonemes_to_cyrillic`] then [`cyrillic_to_phonemes`]
    /// (with `is_native_root = false`, so the rule never drops
    /// anything) yields the input back.
    #[test]
    fn lossless_round_trip_when_rule_disabled() {
        let words = [
            vec![Q, A, Z, A, Q],    // қазақ
            vec![A, L, M, A, T, Y], // алматы (no epenthetic dropping)
            vec![Ng, S, A, P, A],   // ңсапа (synthetic, just for cover)
            vec![M, A, M, Y, R],    // мамыр (May)
        ];
        for ph in words {
            let cyr = phonemes_to_cyrillic(&ph);
            let back = cyrillic_to_phonemes(&cyr, /* is_native_root */ false);
            assert_eq!(ph, back, "round-trip differs for {cyr}");
        }
    }

    /// «қыз» — single-syllable, «ы» is initial-syllable nucleus,
    /// so even with the rule enabled it stays full.
    #[test]
    fn qyz_keeps_ы_in_initial_syllable() {
        let ph = cyrillic_to_phonemes("қыз", true);
        assert_eq!(ph, vec![Q, Y, Z]);
    }

    /// «жұмыс» — «ы» in second syllable between consonants in a
    /// native root → epenthetic → dropped.
    #[test]
    fn jumys_drops_epenthetic_ы() {
        let ph = cyrillic_to_phonemes("жұмыс", true);
        assert_eq!(ph, vec![Zh, U, M, S]);
    }

    /// «жұмыссыз» — two «ы»s, both non-initial between
    /// consonants → both dropped.
    #[test]
    fn jumyssyz_drops_both_epenthetic_ы() {
        let ph = cyrillic_to_phonemes("жұмыссыз", true);
        assert_eq!(ph, vec![Zh, U, M, S, S, Z]);
    }

    /// «бизнес» — loan root, rule does NOT apply, both «и»/«е»
    /// stay as full vowels and (had it had «ы») «ы» would too.
    #[test]
    fn loan_root_keeps_all_vowels() {
        let ph = cyrillic_to_phonemes("бизнес", false);
        // б и з н е с — note: «и» here = Phoneme::I (digraph
        // placeholder; Phase 4 may split into [Yi, J]).
        assert_eq!(ph, vec![B, I, Z, N, E, S]);
    }

    /// Symmetric: «і» in non-initial inter-consonantal native
    /// position is also epenthetic.
    #[test]
    fn epenthetic_і_drops_symmetrically() {
        // «кітап» — «і» is the initial-syllable nucleus, must
        // STAY full. (book, K Yi T A P — actually loanword from
        // Arabic, but treated as native by Kazakh lexicon.)
        let kitap = cyrillic_to_phonemes("кітап", true);
        assert_eq!(kitap, vec![K, Yi, T, A, P]);

        // «білім» — «і» appears twice; first is initial, stays
        // full; second is non-initial between L and M, both
        // consonants → epenthetic → dropped.
        let bilim = cyrillic_to_phonemes("білім", true);
        assert_eq!(bilim, vec![B, Yi, L, M]);
    }

    /// Whitespace and punctuation are silently dropped.
    #[test]
    fn whitespace_and_punctuation_silent() {
        let ph = cyrillic_to_phonemes("қыз!  ?", true);
        assert_eq!(ph, vec![Q, Y, Z]);
    }

    /// Unknown / Latin / digit characters silently dropped.
    #[test]
    fn unknown_chars_silent() {
        let ph = cyrillic_to_phonemes("қыз2 abc", true);
        assert_eq!(ph, vec![Q, Y, Z]);
    }

    /// Boundary marker projection: appears in a stream, gets
    /// elided in the Cyrillic projection.
    #[test]
    fn glottal_elided_in_cyrillic_projection() {
        let cyr = phonemes_to_cyrillic(&[Q, A, Z, Glottal, A, Q]);
        assert_eq!(cyr, "қазақ");
    }

    /// Case is normalised: «ҚАЗАҚ» renders the same as «қазақ».
    #[test]
    fn case_insensitive_input() {
        let upper = cyrillic_to_phonemes("ҚАЗАҚ", false);
        let lower = cyrillic_to_phonemes("қазақ", false);
        assert_eq!(upper, lower);
        assert_eq!(upper, vec![Q, A, Z, A, Q]);
    }
}
