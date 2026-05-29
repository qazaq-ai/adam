// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Phoneme-as-word entries — the "alphabet" the synthesiser uses
//! as the ground truth for each pure sound.
//!
//! One entry per non-boundary phoneme in the v6.3 inventory (36
//! entries — Glottal has no surface glyph and is excluded). The
//! Cyrillic surface is the single glyph from
//! [`adam_phoneme::Phoneme::cyrillic_glyph`].

use crate::{LexEntry, Pos};
use adam_phoneme::Phoneme;

pub const ALPHABET: &[LexEntry] = &[
    // Vowels.
    LexEntry::new("alpha_a", "а", &[Phoneme::A], Pos::Phoneme, "alphabet"),
    LexEntry::new("alpha_ae", "ә", &[Phoneme::Ae], Pos::Phoneme, "alphabet"),
    LexEntry::new("alpha_o", "о", &[Phoneme::O], Pos::Phoneme, "alphabet"),
    LexEntry::new("alpha_oe", "ө", &[Phoneme::Oe], Pos::Phoneme, "alphabet"),
    LexEntry::new("alpha_u", "ұ", &[Phoneme::U], Pos::Phoneme, "alphabet"),
    LexEntry::new("alpha_ue", "ү", &[Phoneme::Ue], Pos::Phoneme, "alphabet"),
    LexEntry::new("alpha_e", "е", &[Phoneme::E], Pos::Phoneme, "alphabet"),
    LexEntry::new("alpha_i", "и", &[Phoneme::I], Pos::Phoneme, "alphabet"),
    // **Intentionally omitted**: «ы» (Phoneme::Y) and «і» (Phoneme::Yi).
    //
    // Per the v6.3 thesis (design doc §9 OQ4 and the project's
    // standing memory `project_v6_3_phonemic_foundation`), these
    // Cyrillic glyphs are **orthographic epenthetic markers**,
    // not pure Kazakh phonemes — they exist in writing as a
    // break between consonant clusters but don't carry an
    // isolated sound a native speaker would produce in
    // isolation. Putting them in the "alphabet" — as a thing the
    // formant synthesiser produces a standalone exemplar for —
    // would bake a phantom sound into the bank. We keep
    // `Phoneme::Y` / `Phoneme::Yi` in the inventory because they
    // surface as nuclei in initial syllables («қыз», «біз»), but
    // we do not enumerate them as standalone alphabet items.
    // Native consonants.
    LexEntry::new("alpha_p", "п", &[Phoneme::P], Pos::Phoneme, "alphabet"),
    LexEntry::new("alpha_b", "б", &[Phoneme::B], Pos::Phoneme, "alphabet"),
    LexEntry::new("alpha_m", "м", &[Phoneme::M], Pos::Phoneme, "alphabet"),
    LexEntry::new("alpha_t", "т", &[Phoneme::T], Pos::Phoneme, "alphabet"),
    LexEntry::new("alpha_d", "д", &[Phoneme::D], Pos::Phoneme, "alphabet"),
    LexEntry::new("alpha_s", "с", &[Phoneme::S], Pos::Phoneme, "alphabet"),
    LexEntry::new("alpha_z", "з", &[Phoneme::Z], Pos::Phoneme, "alphabet"),
    LexEntry::new("alpha_n", "н", &[Phoneme::N], Pos::Phoneme, "alphabet"),
    LexEntry::new("alpha_l", "л", &[Phoneme::L], Pos::Phoneme, "alphabet"),
    LexEntry::new("alpha_r", "р", &[Phoneme::R], Pos::Phoneme, "alphabet"),
    LexEntry::new("alpha_sh", "ш", &[Phoneme::Sh], Pos::Phoneme, "alphabet"),
    LexEntry::new("alpha_zh", "ж", &[Phoneme::Zh], Pos::Phoneme, "alphabet"),
    LexEntry::new("alpha_j", "й", &[Phoneme::J], Pos::Phoneme, "alphabet"),
    LexEntry::new("alpha_k", "к", &[Phoneme::K], Pos::Phoneme, "alphabet"),
    LexEntry::new("alpha_g", "г", &[Phoneme::G], Pos::Phoneme, "alphabet"),
    LexEntry::new("alpha_ng", "ң", &[Phoneme::Ng], Pos::Phoneme, "alphabet"),
    LexEntry::new("alpha_x", "х", &[Phoneme::X], Pos::Phoneme, "alphabet"),
    LexEntry::new("alpha_q", "қ", &[Phoneme::Q], Pos::Phoneme, "alphabet"),
    LexEntry::new("alpha_gh", "ғ", &[Phoneme::Gh], Pos::Phoneme, "alphabet"),
    LexEntry::new("alpha_h", "һ", &[Phoneme::H], Pos::Phoneme, "alphabet"),
    LexEntry::new("alpha_w", "у", &[Phoneme::W], Pos::Phoneme, "alphabet"),
    // Loanword consonants (still part of the inventory; we
    // synthesise them so the bank covers Russian loans).
    LexEntry::new("alpha_f", "ф", &[Phoneme::F], Pos::Phoneme, "alphabet"),
    LexEntry::new("alpha_v", "в", &[Phoneme::V], Pos::Phoneme, "alphabet"),
    LexEntry::new("alpha_ts", "ц", &[Phoneme::Ts], Pos::Phoneme, "alphabet"),
    LexEntry::new("alpha_ch", "ч", &[Phoneme::Ch], Pos::Phoneme, "alphabet"),
    LexEntry::new(
        "alpha_shch",
        "щ",
        &[Phoneme::Shch],
        Pos::Phoneme,
        "alphabet",
    ),
];
