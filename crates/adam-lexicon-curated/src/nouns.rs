// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Kazakh noun lexicon, grouped by phoneme count.
//!
//! Words were chosen to (a) be common and unambiguous, (b)
//! round-trip cleanly through `phonemes_to_cyrillic` (so no
//! Russian-loan я/ю/ё/э letters that the v6.3 mapping
//! approximates), and (c) cover the phoneme inventory thoroughly
//! across the four length buckets.

use crate::{LexEntry, Pos};
use adam_phoneme::Phoneme::*;

pub const NOUNS_LEN2: &[LexEntry] = &[
    LexEntry::new("n2_aj", "ай", &[A, J], Pos::Noun, "len2"), // moon / month
    LexEntry::new("n2_as", "ас", &[A, S], Pos::Noun, "len2"), // food / meal
    LexEntry::new("n2_at", "ат", &[A, T], Pos::Noun, "len2"), // horse / name
    LexEntry::new("n2_ar", "ар", &[A, R], Pos::Noun, "len2"), // conscience
    LexEntry::new("n2_el", "ел", &[E, L], Pos::Noun, "len2"), // country / people
    LexEntry::new("n2_er", "ер", &[E, R], Pos::Noun, "len2"), // saddle / brave man
    LexEntry::new("n2_oz", "өз", &[Oe, Z], Pos::Noun, "len2"), // self / own
    LexEntry::new("n2_ot", "от", &[O, T], Pos::Noun, "len2"), // fire
    LexEntry::new("n2_ul", "ұл", &[U, L], Pos::Noun, "len2"), // son
    LexEntry::new("n2_uj", "үй", &[Ue, J], Pos::Noun, "len2"), // house
    LexEntry::new("n2_it", "ит", &[I, T], Pos::Noun, "len2"), // dog
    LexEntry::new("n2_un", "ұн", &[U, N], Pos::Noun, "len2"), // flour
    LexEntry::new("n2_su", "су", &[S, W], Pos::Noun, "len2"), // water
];

pub const NOUNS_LEN3: &[LexEntry] = &[
    LexEntry::new("n3_ata", "ата", &[A, T, A], Pos::Noun, "len3"), // grandfather
    LexEntry::new("n3_ana", "ана", &[A, N, A], Pos::Noun, "len3"), // mother
    LexEntry::new("n3_apa", "апа", &[A, P, A], Pos::Noun, "len3"), // elder sister
    LexEntry::new("n3_kun", "күн", &[K, Ue, N], Pos::Noun, "len3"), // sun / day
    LexEntry::new("n3_zher", "жер", &[Zh, E, R], Pos::Noun, "len3"), // earth
    LexEntry::new("n3_bal", "бал", &[B, A, L], Pos::Noun, "len3"), // honey
    LexEntry::new("n3_tas", "тас", &[T, A, S], Pos::Noun, "len3"), // stone
    LexEntry::new("n3_bas", "бас", &[B, A, S], Pos::Noun, "len3"), // head
    LexEntry::new("n3_koz", "көз", &[K, Oe, Z], Pos::Noun, "len3"), // eye
    LexEntry::new("n3_qol", "қол", &[Q, O, L], Pos::Noun, "len3"), // hand
    LexEntry::new("n3_kol", "көл", &[K, Oe, L], Pos::Noun, "len3"), // lake
    LexEntry::new("n3_til", "тіл", &[T, L], Pos::Noun, "len3"), // language — orthographic «і» drops
    LexEntry::new("n3_dos", "дос", &[D, O, S], Pos::Noun, "len3"), // friend
    LexEntry::new("n3_tau", "тау", &[T, A, W], Pos::Noun, "len3"), // mountain
    LexEntry::new("n3_nan", "нан", &[N, A, N], Pos::Noun, "len3"), // bread
    LexEntry::new("n3_qan", "қан", &[Q, A, N], Pos::Noun, "len3"), // blood
    LexEntry::new("n3_san", "сан", &[S, A, N], Pos::Noun, "len3"), // number
    LexEntry::new("n3_qos", "көш", &[K, Oe, Sh], Pos::Noun, "len3"), // nomadic move
    LexEntry::new("n3_jaz", "жаз", &[Zh, A, Z], Pos::Noun, "len3"), // summer
    LexEntry::new("n3_qis", "қыс", &[Q, S], Pos::Noun, "len3"), // winter — orthographic «ы» drops
    LexEntry::new("n3_kuz", "күз", &[K, Ue, Z], Pos::Noun, "len3"), // autumn
    LexEntry::new("n3_aru", "ару", &[A, R, W], Pos::Noun, "len3"), // beauty
    LexEntry::new("n3_kop", "көп", &[K, Oe, P], Pos::Noun, "len3"), // many
    LexEntry::new("n3_aga", "аға", &[A, Gh, A], Pos::Noun, "len3"), // elder brother
];

pub const NOUNS_LEN4: &[LexEntry] = &[
    LexEntry::new("n4_bala", "бала", &[B, A, L, A], Pos::Noun, "len4"), // child
    LexEntry::new("n4_qala", "қала", &[Q, A, L, A], Pos::Noun, "len4"), // city
    LexEntry::new("n4_adam", "адам", &[A, D, A, M], Pos::Noun, "len4"), // person
    LexEntry::new("n4_dala", "дала", &[D, A, L, A], Pos::Noun, "len4"), // steppe
    LexEntry::new("n4_aqsha", "ақша", &[A, Q, Sh, A], Pos::Noun, "len4"), // money
    LexEntry::new("n4_alga", "алға", &[A, L, Gh, A], Pos::Noun, "len4"), // forward
    // Note: words below with «ы» / «і» in non-initial
    // syllables between consonants drop them per the v6.3
    // epenthetic rule — the orthographic glyph is preserved in
    // the Cyrillic field, but the phonemic transcript matches
    // what `cyrillic_to_phonemes(..., true)` produces.
    LexEntry::new("n4_oryn", "орын", &[O, R, N], Pos::Noun, "len4"), // place — «ы» epenthetic
    LexEntry::new("n4_qara", "қара", &[Q, A, R, A], Pos::Noun, "len4"), // black / look
    LexEntry::new("n4_asyl", "асыл", &[A, S, L], Pos::Noun, "len4"), // precious — epenthetic
    LexEntry::new("n4_oryk", "өрік", &[Oe, R, K], Pos::Noun, "len4"), // apricot — epenthetic
    LexEntry::new("n4_egiz", "егіз", &[E, G, Z], Pos::Noun, "len4"), // twin — epenthetic
    LexEntry::new("n4_omir", "өмір", &[Oe, M, R], Pos::Noun, "len4"), // life — epenthetic
    LexEntry::new("n4_otan", "отан", &[O, T, A, N], Pos::Noun, "len4"), // motherland
    LexEntry::new("n4_aqyl", "ақыл", &[A, Q, L], Pos::Noun, "len4"), // mind — epenthetic
    LexEntry::new("n4_eski", "ескі", &[E, S, K], Pos::Noun, "len4"), // old — orthographic «і» drops
    LexEntry::new("n4_uyim", "ұйым", &[U, J, M], Pos::Noun, "len4"), // organisation
    LexEntry::new("n4_tary", "тары", &[T, A, R], Pos::Noun, "len4"), // millet — orthographic «ы» drops
    LexEntry::new("n4_anyq", "анық", &[A, N, Q], Pos::Noun, "len4"), // clear — epenthetic
    LexEntry::new("n4_jaqsi", "жақсы", &[Zh, A, Q, S], Pos::Noun, "len5"), // good — orthographic «ы» drops
];

pub const NOUNS_LEN5PLUS: &[LexEntry] = &[
    LexEntry::new("n5_qazaq", "қазақ", &[Q, A, Z, A, Q], Pos::Noun, "len5"), // Kazakh
    LexEntry::new("n5_arman", "арман", &[A, R, M, A, N], Pos::Noun, "len5"), // dream
    LexEntry::new("n5_aspan", "аспан", &[A, S, P, A, N], Pos::Noun, "len5"), // sky
    LexEntry::new("n5_jurek", "жүрек", &[Zh, Ue, R, E, K], Pos::Noun, "len5"), // heart
    LexEntry::new("n5_taraz", "тараз", &[T, A, R, A, Z], Pos::Noun, "len5"), // Taraz
    LexEntry::new("n5_uaqyt", "уақыт", &[W, A, Q, T], Pos::Noun, "len5"), // time — epenthetic «ы»
    LexEntry::new("n5_baqyt", "бақыт", &[B, A, Q, T], Pos::Noun, "len5"), // happiness — epenthetic «ы»
    LexEntry::new("n5_qogham", "қоғам", &[Q, O, Gh, A, M], Pos::Noun, "len5"), // society
    LexEntry::new("n5_dunie", "дүние", &[D, Ue, N, I, E], Pos::Noun, "len5"), // world
    LexEntry::new("n5_atasy", "атасы", &[A, T, A, S], Pos::Noun, "len5"), // his grandfather — final «ы» drops
    LexEntry::new("n5_anasy", "анасы", &[A, N, A, S], Pos::Noun, "len5"), // his mother — final «ы» drops
    LexEntry::new("n5_jibek", "жібек", &[Zh, B, E, K], Pos::Noun, "len5"), // silk — orthographic «і» drops
    LexEntry::new(
        "n5_mektep",
        "мектеп",
        &[M, E, K, T, E, P],
        Pos::Noun,
        "len6",
    ), // school
    LexEntry::new("n5_dostyq", "достық", &[D, O, S, T, Q], Pos::Noun, "len6"), // friendship — epenthetic «ы»
    LexEntry::new("n5_almaty", "алматы", &[A, L, M, A, T], Pos::Noun, "len6"), // Almaty — final «ы» drops
    LexEntry::new(
        "n5_astana",
        "астана",
        &[A, S, T, A, N, A],
        Pos::Noun,
        "len6",
    ), // Astana
    LexEntry::new("n5_balasy", "баласы", &[B, A, L, A, S], Pos::Noun, "len6"), // his child — final «ы» drops
    LexEntry::new("n5_juldyz", "жұлдыз", &[Zh, U, L, D, Z], Pos::Noun, "len6"), // star — epenthetic «ы»
    LexEntry::new(
        "n5_balalar",
        "балалар",
        &[B, A, L, A, L, A, R],
        Pos::Noun,
        "len7",
    ), // children
    LexEntry::new(
        "n5_jasandy",
        "жасанды",
        &[Zh, A, S, A, N, D],
        Pos::Noun,
        "len7",
    ), // artificial — final «ы» drops
];
