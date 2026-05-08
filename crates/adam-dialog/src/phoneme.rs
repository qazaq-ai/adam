//! **v5.2.0** — Kazakh grapheme-to-phoneme (G2P) — rule-based.
//!
//! Maps Kazakh Cyrillic text to a stream of [`Phoneme`] units. Future
//! concatenative TTS (a `PhonemeBankTtsBackend` planned for v5.2.5+)
//! will splice pre-recorded audio per phoneme to produce kernel-pure
//! deterministic Kazakh pronunciation that doesn't depend on neural
//! models or external TTS.
//!
//! ## Why bother with G2P?
//!
//! Kazakh is heavily agglutinative with mostly-regular orthography —
//! the grapheme-to-phoneme mapping is overwhelmingly 1:1 for the
//! native 33-letter alphabet, with predictable handling of the few
//! Russian-loan letters (ё / ю / я / э / ц / щ / ъ / ь). A
//! rule-based G2P covers this without statistical training, fits the
//! `project_retrieval_not_neural_v2` philosophy, and runs in
//! microseconds.
//!
//! v5.2.0 ships **only** the G2P module; the phoneme bank itself
//! (audio WAV per phoneme) is a separate content-engineering
//! investment deferred to v5.2.5+. Even without the bank, the G2P
//! module is useful in its own right:
//!
//! - Powers a future `PhonemeBankTtsBackend`
//! - Surfaces Kazakh phonemic transcription for educational tools
//! - Substrate for phoneme-aware spell-checking / fuzzy match
//! - Debugging aid for adam's existing morphological pipeline
//!
//! ## Phoneme inventory
//!
//! 9 native Kazakh vowels + ~21 native consonants; foreign letters
//! map to native phoneme pairs (ю → /j u/, я → /j a/, ё → /j o/).
//! See [`Phoneme`] for the full enum.
//!
//! ## Stress
//!
//! Kazakh stress is overwhelmingly word-final, predictable from
//! position alone. v5.2.0 doesn't model stress explicitly — the
//! phoneme stream is unaccented. v5.2.5+ may add stress markers if
//! a phoneme bank requires them.

use std::fmt;

/// One phonetic unit in the Kazakh phoneme inventory.
///
/// **Vowels** follow IPA-ish convention but use single-byte Latin
/// letters where possible for ergonomic matching: `a æ e i ɪ o œ u y`.
/// **Consonants** likewise: `b p m w f t d n s z l r k q g ɣ h x ʃ ʒ
/// tʃ j ŋ`.
///
/// Russian-borrowed letters that exist in modern Kazakh orthography
/// (mostly in loanwords) are normalised to native phoneme pairs:
/// `ё → j + o`, `ю → j + u`, `я → j + a`, `э → e`, `ц → t + s`,
/// `щ → ʃ + tʃ`. The hard sign `ъ` and soft sign `ь` are silent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phoneme {
    // ─── Vowels (9 native + 1 foreign-only `i` for и) ────────────
    /// «а» — open central
    A,
    /// «ә» — open front
    AE,
    /// «е» / «э» — mid front
    E,
    /// «и» — close front (also part of «ю», «я» diphthongs)
    I,
    /// «ы» — close central / back-unrounded
    YBack,
    /// «о» — mid back rounded
    O,
    /// «ө» — mid front rounded
    OE,
    /// «ұ» — close back rounded
    U,
    /// «ү» — close front rounded
    UE,
    /// «і» — close front near-close
    IFront,

    // ─── Consonants (native Kazakh inventory) ────────────────────
    /// «б»
    B,
    /// «п»
    P,
    /// «м»
    M,
    /// «у» — bilabial approximant in vowel-adjacent positions
    W,
    /// «ф» — only in loanwords
    F,
    /// «т»
    T,
    /// «д»
    D,
    /// «н»
    N,
    /// «с»
    S,
    /// «з»
    Z,
    /// «л»
    L,
    /// «р»
    R,
    /// «к» — palatovelar in front-vowel words
    K,
    /// «қ» — uvular voiceless
    Q,
    /// «г» — palatovelar voiced
    G,
    /// «ғ» — uvular voiced fricative
    GH,
    /// «х» — voiceless velar fricative
    X,
    /// «һ» — voiceless glottal fricative (rare; loanwords)
    H,
    /// «ш»
    SH,
    /// «ж»
    ZH,
    /// «ч» — only in loanwords (often realised as «ш»)
    CH,
    /// «й» — palatal approximant
    J,
    /// «ң» — velar nasal
    NG,
}

impl Phoneme {
    /// IPA-ish single-symbol form, suitable for transcription
    /// debugging or future phoneme-bank file naming.
    pub fn ipa(&self) -> &'static str {
        match self {
            Self::A => "a",
            Self::AE => "æ",
            Self::E => "e",
            Self::I => "i",
            Self::YBack => "ɯ",
            Self::O => "o",
            Self::OE => "œ",
            Self::U => "u",
            Self::UE => "y",
            Self::IFront => "ɪ",
            Self::B => "b",
            Self::P => "p",
            Self::M => "m",
            Self::W => "w",
            Self::F => "f",
            Self::T => "t",
            Self::D => "d",
            Self::N => "n",
            Self::S => "s",
            Self::Z => "z",
            Self::L => "l",
            Self::R => "r",
            Self::K => "k",
            Self::Q => "q",
            Self::G => "g",
            Self::GH => "ɣ",
            Self::X => "x",
            Self::H => "h",
            Self::SH => "ʃ",
            Self::ZH => "ʒ",
            Self::CH => "tʃ",
            Self::J => "j",
            Self::NG => "ŋ",
        }
    }

    /// `true` when this phoneme is a vowel (any of the 10 vowel
    /// variants). Useful for stress placement, phoneme-bank
    /// concatenation smoothing, or Lexicon-aware analysis.
    pub fn is_vowel(&self) -> bool {
        matches!(
            self,
            Self::A
                | Self::AE
                | Self::E
                | Self::I
                | Self::YBack
                | Self::O
                | Self::OE
                | Self::U
                | Self::UE
                | Self::IFront
        )
    }

    /// Stable identifier suitable for phoneme-bank filenames (no
    /// non-ASCII characters). e.g. «ң» → `"ng"`, «ш» → `"sh"`. The
    /// future `PhonemeBankTtsBackend` will load
    /// `data/dialog/phoneme_bank/<bank_id>.wav` per phoneme.
    pub fn bank_id(&self) -> &'static str {
        match self {
            Self::A => "a",
            Self::AE => "ae",
            Self::E => "e",
            Self::I => "i",
            Self::YBack => "y",
            Self::O => "o",
            Self::OE => "oe",
            Self::U => "u",
            Self::UE => "ue",
            Self::IFront => "if",
            Self::B => "b",
            Self::P => "p",
            Self::M => "m",
            Self::W => "w",
            Self::F => "f",
            Self::T => "t",
            Self::D => "d",
            Self::N => "n",
            Self::S => "s",
            Self::Z => "z",
            Self::L => "l",
            Self::R => "r",
            Self::K => "k",
            Self::Q => "q",
            Self::G => "g",
            Self::GH => "gh",
            Self::X => "x",
            Self::H => "h",
            Self::SH => "sh",
            Self::ZH => "zh",
            Self::CH => "ch",
            Self::J => "j",
            Self::NG => "ng",
        }
    }
}

impl fmt::Display for Phoneme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.ipa())
    }
}

/// Transcribe Kazakh Cyrillic text into a phoneme stream.
///
/// Punctuation, whitespace, and unrecognised characters are dropped
/// silently — the caller is expected to chunk the input on word
/// boundaries before passing it in (or feed whole sentences and
/// rely on the silent-drop semantics for spaces / commas).
///
/// Russian-borrowed compound letters are decomposed:
/// - «ё» → `[J, O]`
/// - «ю» → `[J, U]`
/// - «я» → `[J, A]`
/// - «э» → `[E]`
/// - «ц» → `[T, S]`
/// - «щ» → `[SH, CH]`
/// - «ъ», «ь» → silent (dropped)
pub fn text_to_phonemes(text: &str) -> Vec<Phoneme> {
    let mut out = Vec::with_capacity(text.len());
    for ch in text.chars() {
        let lower_chars: Vec<char> = ch.to_lowercase().collect();
        for lc in lower_chars {
            push_phoneme_for_char(lc, &mut out);
        }
    }
    out
}

fn push_phoneme_for_char(ch: char, out: &mut Vec<Phoneme>) {
    use Phoneme::*;
    match ch {
        // ── Native Kazakh vowels ─────────────────────────────────
        'а' => out.push(A),
        'ә' => out.push(AE),
        'е' => out.push(E),
        'и' => out.push(I),
        'ы' => out.push(YBack),
        'і' => out.push(IFront),
        'о' => out.push(O),
        'ө' => out.push(OE),
        'ұ' => out.push(U),
        'ү' => out.push(UE),

        // ── Russian-loan compound vowels (decomposed) ────────────
        'ё' => {
            out.push(J);
            out.push(O);
        }
        'ю' => {
            out.push(J);
            out.push(U);
        }
        'я' => {
            out.push(J);
            out.push(A);
        }
        'э' => out.push(E),

        // ── Native Kazakh consonants ─────────────────────────────
        'б' => out.push(B),
        'п' => out.push(P),
        'м' => out.push(M),
        'у' => out.push(W),
        'ф' => out.push(F),
        'т' => out.push(T),
        'д' => out.push(D),
        'н' => out.push(N),
        'с' => out.push(S),
        'з' => out.push(Z),
        'л' => out.push(L),
        'р' => out.push(R),
        'к' => out.push(K),
        'қ' => out.push(Q),
        'г' => out.push(G),
        'ғ' => out.push(GH),
        'х' => out.push(X),
        'һ' => out.push(H),
        'ш' => out.push(SH),
        'ж' => out.push(ZH),
        'ч' => out.push(CH),
        'й' => out.push(J),
        'ң' => out.push(NG),

        // ── Russian-loan compound consonants ─────────────────────
        'ц' => {
            out.push(T);
            out.push(S);
        }
        'щ' => {
            out.push(SH);
            out.push(CH);
        }

        // ── Hard / soft signs — silent ───────────────────────────
        'ъ' | 'ь' => {}

        // ── Punctuation / whitespace / unknown — silent drop ─────
        _ => {}
    }
}

/// Render a phoneme stream as an IPA-ish string for human-readable
/// transcription output. Phonemes are space-separated.
pub fn phonemes_to_ipa(stream: &[Phoneme]) -> String {
    stream.iter().map(|p| p.ipa()).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::Phoneme::*;
    use super::*;

    #[test]
    fn vowels_classified_correctly() {
        for v in [A, AE, E, I, YBack, O, OE, U, UE, IFront] {
            assert!(v.is_vowel(), "{v:?} should be vowel");
        }
        for c in [B, P, M, W, T, K, Q, NG, J] {
            assert!(!c.is_vowel(), "{c:?} should NOT be vowel");
        }
    }

    #[test]
    fn ipa_symbols_unique_per_phoneme() {
        let all = [
            A, AE, E, I, YBack, O, OE, U, UE, IFront, B, P, M, W, F, T, D, N, S, Z, L, R, K, Q, G,
            GH, X, H, SH, ZH, CH, J, NG,
        ];
        let mut seen = std::collections::HashSet::new();
        for p in all {
            let ipa = p.ipa();
            assert!(seen.insert(ipa), "duplicate IPA symbol: {ipa}");
        }
    }

    #[test]
    fn bank_ids_are_ascii_unique() {
        let all = [
            A, AE, E, I, YBack, O, OE, U, UE, IFront, B, P, M, W, F, T, D, N, S, Z, L, R, K, Q, G,
            GH, X, H, SH, ZH, CH, J, NG,
        ];
        let mut seen = std::collections::HashSet::new();
        for p in all {
            let id = p.bank_id();
            assert!(id.is_ascii(), "bank_id must be ASCII for filenames: {id}");
            assert!(seen.insert(id), "duplicate bank_id: {id}");
        }
    }

    #[test]
    fn simple_kazakh_word_алма() {
        // «алма» (apple) — а л м а
        assert_eq!(text_to_phonemes("алма"), vec![A, L, M, A]);
    }

    #[test]
    fn kazakh_word_with_қ_and_ұ() {
        // «құс» (bird) — қ ұ с
        assert_eq!(text_to_phonemes("құс"), vec![Q, U, S]);
    }

    #[test]
    fn kazakh_word_with_front_vowels() {
        // «көйлек» (shirt) — к ө й л е к
        assert_eq!(text_to_phonemes("көйлек"), vec![K, OE, J, L, E, K]);
    }

    #[test]
    fn kazakh_word_with_ң() {
        // «таң» (dawn) — т а ң
        assert_eq!(text_to_phonemes("таң"), vec![T, A, NG]);
    }

    #[test]
    fn kazakh_word_with_ғ() {
        // «бағдарлама» (program) — б а ғ д а р л а м а
        assert_eq!(
            text_to_phonemes("бағдарлама"),
            vec![B, A, GH, D, A, R, L, A, M, A]
        );
    }

    #[test]
    fn russian_loan_ю_decomposes() {
        // «юбка» — ю б к а → j u b k a
        assert_eq!(text_to_phonemes("юбка"), vec![J, U, B, K, A]);
    }

    #[test]
    fn russian_loan_я_decomposes() {
        // «семья» — с е м ь я → s e m  j a (ь silent)
        assert_eq!(text_to_phonemes("семья"), vec![S, E, M, J, A]);
    }

    #[test]
    fn russian_loan_ё_decomposes() {
        // «ёлка» — ё л к а → j o l k a
        assert_eq!(text_to_phonemes("ёлка"), vec![J, O, L, K, A]);
    }

    #[test]
    fn russian_loan_ц_decomposes_to_ts() {
        // «цех» — ц е х → t s e x
        assert_eq!(text_to_phonemes("цех"), vec![T, S, E, X]);
    }

    #[test]
    fn russian_loan_щ_decomposes_to_sh_ch() {
        // «щётка» — щ ё т к а → sh ch j o t k a
        assert_eq!(text_to_phonemes("щётка"), vec![SH, CH, J, O, T, K, A]);
    }

    #[test]
    fn hard_and_soft_signs_silent() {
        // Both ъ and ь produce no phoneme. v5.2.0 takes the simpler
        // route: drop them silently. In Russian loanwords this loses
        // the /j/ glide that ь/ъ-before-vowel would mark — a v5.2.5+
        // refinement could add the rule. For Kazakh-native words ь/ъ
        // is essentially absent so the simplification has no impact.
        assert_eq!(text_to_phonemes("обьект"), vec![O, B, E, K, T]);
        assert_eq!(text_to_phonemes("съезд"), vec![S, E, Z, D]);
    }

    #[test]
    fn whitespace_and_punctuation_dropped() {
        // «Сәлем!» — с ә л е м (! dropped)
        assert_eq!(text_to_phonemes("Сәлем!"), vec![S, AE, L, E, M]);
        // Multi-word — space dropped silently.
        assert_eq!(text_to_phonemes("сен кім?"), vec![S, E, N, K, IFront, M]);
    }

    #[test]
    fn uppercase_normalises_to_lowercase_phonemes() {
        // «АДАМ» (man) should yield same as «адам»
        let upper = text_to_phonemes("АДАМ");
        let lower = text_to_phonemes("адам");
        assert_eq!(upper, lower);
        assert_eq!(lower, vec![A, D, A, M]);
    }

    #[test]
    fn empty_input_yields_empty_stream() {
        assert!(text_to_phonemes("").is_empty());
        assert!(text_to_phonemes("!?, ").is_empty());
    }

    #[test]
    fn mixed_kazakh_and_latin_drops_latin() {
        // Latin characters aren't part of Kazakh phoneme inventory —
        // dropped silently. Caller pre-filters or accepts this.
        assert_eq!(
            text_to_phonemes("Rust-та қандай?"),
            vec![T, A, Q, A, N, D, A, J]
        );
    }

    #[test]
    fn phonemes_to_ipa_renders_space_separated() {
        let stream = vec![A, L, M, A];
        assert_eq!(phonemes_to_ipa(&stream), "a l m a");
    }

    #[test]
    fn long_word_бағдарламашылық_decomposes_correctly() {
        // Real Kazakh word — «бағдарламашылық» (programming) —
        // tests vowel harmony agnosticism (G2P doesn't enforce
        // harmony, just renders letters phonemically).
        let got = text_to_phonemes("бағдарламашылық");
        let expected = vec![B, A, GH, D, A, R, L, A, M, A, SH, YBack, L, YBack, Q];
        assert_eq!(got, expected);
    }
}
