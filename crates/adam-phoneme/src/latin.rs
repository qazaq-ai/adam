// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Bidirectional Latin ↔ Phoneme renderer (Layer 0d, second half).
//!
//! Symmetric to [`crate::cyrillic`] but for the Latin
//! orthography. The Latin transliteration is anchored in the
//! 2021 official Kazakh Latin alphabet (acute / cedilla variant)
//! with the v6.3-internal disambiguators documented on
//! [`crate::Phoneme::latin_glyph`].
//!
//! ## Multi-character tokens
//!
//! Two Latin tokens are multi-character:
//!
//! - `"ts"` → `Ts` (Cyrillic «ц»).
//! - `"şç"` → `Shch` (Cyrillic «щ»).
//!
//! The parser does **1-token lookahead** to disambiguate. When
//! `t` appears followed by `s`, it consumes both as `Ts`. When
//! `ş` appears followed by `ç`, it consumes both as `Shch`.
//! Otherwise each grapheme is single-character.
//!
//! ## Epenthetic rule
//!
//! Same rule as [`crate::cyrillic`]: when `is_native_root` is
//! `true`, the Latin epenthetic vowels (`y` = ы, `i` = і) are
//! dropped iff they sit in a non-initial syllable between two
//! consonant phonemes.

use crate::Phoneme;

/// Project a phoneme stream to Latin.
pub fn phonemes_to_latin(phonemes: &[Phoneme]) -> String {
    let mut out = String::with_capacity(phonemes.len());
    for p in phonemes {
        if let Some(g) = p.latin_glyph() {
            out.push_str(g);
        }
    }
    out
}

/// Project a Latin word to a phoneme stream.
///
/// The epenthetic rule (drop a non-initial-syllable `y` / `i`
/// between consonants in native roots) is applied symmetrically
/// to [`crate::cyrillic::cyrillic_to_phonemes`].
pub fn latin_to_phonemes(text: &str, is_native_root: bool) -> Vec<Phoneme> {
    // Normalise case. `to_lowercase` correctly handles `Ş` → `ş`
    // and the other extended letters used in the Kazakh Latin
    // alphabet (these are Unicode lowercase-mapped).
    let lowered = text.to_lowercase();
    let chars: Vec<char> = lowered.chars().collect();

    let mut out: Vec<Phoneme> = Vec::with_capacity(chars.len());
    let mut i = 0;

    while i < chars.len() {
        // Try 2-character tokens first.
        let (p_opt, advance) = match (chars[i], chars.get(i + 1).copied()) {
            ('t', Some('s')) => (Some(Phoneme::Ts), 2),
            ('ş', Some('ç')) => (Some(Phoneme::Shch), 2),
            (c, _) => (latin_char_to_phoneme(c), 1),
        };

        let Some(p) = p_opt else {
            i += advance;
            continue;
        };

        // Strict orthographic rule, symmetric to cyrillic.rs:
        // for a native root, every Latin `y` / `i` (= Cyrillic
        // «ы» / «і») drops unconditionally — they are pure
        // orthographic markers, not phonemes.
        if is_native_root && matches!(p, Phoneme::Y | Phoneme::Yi) {
            i += advance;
            continue;
        }

        out.push(p);
        i += advance;
    }

    out
}

/// Single-character Latin→Phoneme lookup. Returns `None` for
/// characters outside the Kazakh Latin alphabet (punctuation,
/// whitespace, digits, ASCII letters not in the scheme).
///
/// **Does not handle** the two multi-character tokens (`ts`,
/// `şç`) — those are dispatched by [`latin_to_phonemes`].
pub fn latin_char_to_phoneme(c: char) -> Option<Phoneme> {
    use Phoneme::*;
    Some(match c {
        // Full vowels.
        'a' => A,
        'ä' => Ae,
        'o' => O,
        'ö' => Oe,
        'u' => U,
        'ü' => Ue,
        'e' => E,
        'ı' => I,
        // Epenthetic vowels.
        'y' => Y,
        'i' => Yi,
        // Native consonants.
        'p' => P,
        'b' => B,
        'm' => M,
        't' => T,
        'd' => D,
        's' => S,
        'z' => Z,
        'n' => N,
        'l' => L,
        'r' => R,
        'ş' => Sh,
        'j' => Zh,
        'ý' => J,
        'k' => K,
        'g' => G,
        'ñ' => Ng,
        'x' => X,
        'q' => Q,
        'ğ' => Gh,
        'h' => H,
        'w' => W,
        // Loan-only consonants (single-char part of multi-char
        // tokens handled in the parser).
        'f' => F,
        'v' => V,
        'ç' => Ch,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use Phoneme::*;

    /// Forward: phoneme stream → Latin string for canonical
    /// vocabulary.
    #[test]
    fn forward_render_qazaqstan() {
        assert_eq!(phonemes_to_latin(&[Q, A, Z, A, Q, S, T, A, N]), "qazaqstan");
    }

    #[test]
    fn forward_render_almaty() {
        assert_eq!(phonemes_to_latin(&[A, L, M, A, T, Y]), "almaty");
    }

    #[test]
    fn forward_render_with_special_glyphs() {
        // «көкшетау» (compound but renderable) — uses ö, ş, w.
        assert_eq!(phonemes_to_latin(&[K, Oe, K, Sh, E, T, A, W]), "kökşetaw");
    }

    #[test]
    fn forward_render_multichar_tokens() {
        // Ts → "ts", Shch → "şç". Confirm length and content.
        assert_eq!(phonemes_to_latin(&[Ts]), "ts");
        assert_eq!(phonemes_to_latin(&[Shch]), "şç");
        assert_eq!(phonemes_to_latin(&[A, Ts, A]), "atsa");
        assert_eq!(phonemes_to_latin(&[A, Shch, A]), "aşça");
    }

    /// Reverse: Latin → phonemes, simple cases without epenthetic.
    #[test]
    fn reverse_qazaq() {
        assert_eq!(latin_to_phonemes("qazaq", true), vec![Q, A, Z, A, Q]);
    }

    /// `Ts` digraph parsed as a single phoneme.
    #[test]
    fn reverse_parses_ts_digraph() {
        assert_eq!(latin_to_phonemes("atsa", true), vec![A, Ts, A]);
    }

    /// `Shch` digraph (`şç`) parsed as a single phoneme.
    #[test]
    fn reverse_parses_shch_digraph() {
        assert_eq!(latin_to_phonemes("aşça", true), vec![A, Shch, A]);
    }

    /// Round-trip: phoneme → Latin → phoneme is the identity
    /// when the rule is disabled (loanword mode).
    #[test]
    fn round_trip_loan_mode() {
        let words: [Vec<Phoneme>; 4] = [
            vec![Q, A, Z, A, Q],
            vec![A, L, M, A, T, Y],
            vec![Ng, S, A, P, A],
            vec![A, Ts, Shch, A],
        ];
        for ph in words {
            let lat = phonemes_to_latin(&ph);
            let back = latin_to_phonemes(&lat, false);
            assert_eq!(ph, back, "round-trip differs at {lat}");
        }
    }

    /// «qyz» — Latin `y` drops under the v6.3 strict
    /// orthographic rule, leaving /qz/. Symmetric to the
    /// Cyrillic test `qyz_drops_orthographic_ы`.
    #[test]
    fn qyz_drops_orthographic_y() {
        assert_eq!(latin_to_phonemes("qyz", true), vec![Q, Z]);
    }

    /// «jumys» — non-initial «y» between consonants → epenthetic
    /// → dropped.
    #[test]
    fn jumys_drops_epenthetic_y() {
        assert_eq!(latin_to_phonemes("jumys", true), vec![Zh, U, M, S]);
    }

    /// `J` (й) round-trip. The disambiguator `ý` makes this
    /// reversible.
    #[test]
    fn j_phoneme_round_trips() {
        let stream = vec![B, A, J, T, U, R, S, N, U, L, Y];
        let lat = phonemes_to_latin(&stream);
        // With loan-mode (no dropping), round-trip is exact.
        assert_eq!(latin_to_phonemes(&lat, false), stream);
    }

    /// Case is normalised (uppercase Latin renders to the same
    /// phonemes as lowercase).
    #[test]
    fn case_insensitive_input() {
        let upper = latin_to_phonemes("QAZAQ", false);
        let lower = latin_to_phonemes("qazaq", false);
        assert_eq!(upper, lower);
        assert_eq!(upper, vec![Q, A, Z, A, Q]);
    }

    /// Whitespace / punctuation / unknown chars silently dropped
    /// (and the orthographic `y` drops as well under the strict
    /// rule).
    #[test]
    fn unknown_chars_silent() {
        assert_eq!(latin_to_phonemes("qyz! 123", true), vec![Q, Z]);
    }

    /// Loan-mode round-trip on a stream containing all loan
    /// consonants — proves the loan branch carries them.
    #[test]
    fn loan_consonants_round_trip() {
        let stream = vec![F, V, Ts, Ch, Shch];
        let lat = phonemes_to_latin(&stream);
        assert_eq!(lat, "fvtsçşç");
        assert_eq!(latin_to_phonemes(&lat, false), stream);
    }

    /// Native-mode epenthetic-drop with multi-char neighbour:
    /// if the consonant on the OTHER side of `y` is a multi-char
    /// token like `ts`, the 2-char lookahead must still see it
    /// as a consonant.
    #[test]
    fn epenthetic_rule_handles_multichar_neighbour() {
        // Synthetic word: "atyts" — a t y ts. The `y` at
        // position 2 is between `t` (consonant) and `ts`
        // (consonant). Non-initial syllable (A is the vowel at
        // position 0). Should drop.
        let r = latin_to_phonemes("atyts", true);
        assert_eq!(r, vec![A, T, Ts]);
    }
}
