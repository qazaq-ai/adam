// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Adjectives, pronouns, numerals, interjections, adverbs.

use crate::{LexEntry, Pos};
use adam_phoneme::Phoneme::*;

pub const ADJECTIVES: &[LexEntry] = &[
    // First «ы» = nucleus of «қыз» (initial syllable, kept).
    // Second «ы» = epenthetic between Z and L, dropped.
    LexEntry::new("adj_qyzyl", "қызыл", &[Q, Y, Z, L], Pos::Adjective, "color"),
    LexEntry::new("adj_aq", "ақ", &[A, Q], Pos::Adjective, "color"),
    LexEntry::new("adj_qara", "қара", &[Q, A, R, A], Pos::Adjective, "color"),
    LexEntry::new("adj_kok", "көк", &[K, Oe, K], Pos::Adjective, "color"),
    LexEntry::new("adj_sary", "сары", &[S, A, R, Y], Pos::Adjective, "color"),
    LexEntry::new(
        "adj_jasyl",
        "жасыл",
        &[Zh, A, S, L],
        Pos::Adjective,
        "color",
    ), // epenthetic «ы»
    LexEntry::new(
        "adj_ulken",
        "үлкен",
        &[Ue, L, K, E, N],
        Pos::Adjective,
        "size",
    ),
    LexEntry::new(
        "adj_kishi",
        "кіші",
        &[K, Yi, Sh, Yi],
        Pos::Adjective,
        "size",
    ),
    LexEntry::new(
        "adj_jaqsy",
        "жақсы",
        &[Zh, A, Q, S, Y],
        Pos::Adjective,
        "quality",
    ),
    LexEntry::new(
        "adj_jaman",
        "жаман",
        &[Zh, A, M, A, N],
        Pos::Adjective,
        "quality",
    ),
    LexEntry::new(
        "adj_jana",
        "жаңа",
        &[Zh, A, Ng, A],
        Pos::Adjective,
        "quality",
    ),
    LexEntry::new(
        "adj_eski",
        "ескі",
        &[E, S, K, Yi],
        Pos::Adjective,
        "quality",
    ),
    LexEntry::new("adj_alys", "алыс", &[A, L, S], Pos::Adjective, "distance"), // epenthetic «ы»
    LexEntry::new(
        "adj_jaqyn",
        "жақын",
        &[Zh, A, Q, N],
        Pos::Adjective,
        "distance",
    ), // epenthetic «ы»
    LexEntry::new(
        "adj_tatti",
        "тәтті",
        &[T, Ae, T, T, Yi],
        Pos::Adjective,
        "taste",
    ),
    LexEntry::new("adj_ashshy", "ащы", &[A, Shch, Y], Pos::Adjective, "taste"),
];

pub const PRONOUNS: &[LexEntry] = &[
    LexEntry::new("pn_men", "мен", &[M, E, N], Pos::Pronoun, "personal"),
    LexEntry::new("pn_sen", "сен", &[S, E, N], Pos::Pronoun, "personal"),
    LexEntry::new("pn_siz", "сіз", &[S, Yi, Z], Pos::Pronoun, "personal"),
    LexEntry::new("pn_ol", "ол", &[O, L], Pos::Pronoun, "personal"),
    LexEntry::new("pn_biz", "біз", &[B, Yi, Z], Pos::Pronoun, "personal"),
    LexEntry::new(
        "pn_sender",
        "сендер",
        &[S, E, N, D, E, R],
        Pos::Pronoun,
        "personal",
    ),
    LexEntry::new("pn_olar", "олар", &[O, L, A, R], Pos::Pronoun, "personal"),
    LexEntry::new("pn_bul", "бұл", &[B, U, L], Pos::Pronoun, "demonstrative"),
    LexEntry::new(
        "pn_anau",
        "анау",
        &[A, N, A, W],
        Pos::Pronoun,
        "demonstrative",
    ),
    LexEntry::new("pn_kim", "кім", &[K, Yi, M], Pos::Pronoun, "interrogative"),
    LexEntry::new("pn_ne", "не", &[N, E], Pos::Pronoun, "interrogative"),
    LexEntry::new(
        "pn_qaida",
        "қайда",
        &[Q, A, J, D, A],
        Pos::Pronoun,
        "interrogative",
    ),
    LexEntry::new(
        "pn_qalaj",
        "қалай",
        &[Q, A, L, A, J],
        Pos::Pronoun,
        "interrogative",
    ),
];

pub const NUMERALS: &[LexEntry] = &[
    LexEntry::new("num_bir", "бір", &[B, Yi, R], Pos::Numeral, "1-10"),
    LexEntry::new("num_eki", "екі", &[E, K, Yi], Pos::Numeral, "1-10"),
    LexEntry::new("num_ush", "үш", &[Ue, Sh], Pos::Numeral, "1-10"),
    LexEntry::new("num_tort", "төрт", &[T, Oe, R, T], Pos::Numeral, "1-10"),
    LexEntry::new("num_bes", "бес", &[B, E, S], Pos::Numeral, "1-10"),
    LexEntry::new("num_alty", "алты", &[A, L, T, Y], Pos::Numeral, "1-10"),
    LexEntry::new("num_jeti", "жеті", &[Zh, E, T, Yi], Pos::Numeral, "1-10"),
    LexEntry::new("num_segiz", "сегіз", &[S, E, G, Z], Pos::Numeral, "1-10"), // epenthetic «і»
    LexEntry::new("num_toghyz", "тоғыз", &[T, O, Gh, Z], Pos::Numeral, "1-10"), // epenthetic «ы»
    LexEntry::new("num_on", "он", &[O, N], Pos::Numeral, "10s"),
    LexEntry::new(
        "num_jiyrma",
        "жиырма",
        &[Zh, I, Y, R, M, A],
        Pos::Numeral,
        "10s",
    ),
    LexEntry::new("num_otyz", "отыз", &[O, T, Z], Pos::Numeral, "10s"), // epenthetic «ы»
    LexEntry::new("num_qyryq", "қырық", &[Q, Y, R, Q], Pos::Numeral, "10s"), // first ы = nucleus, second epenthetic
    LexEntry::new("num_elyu", "елу", &[E, L, W], Pos::Numeral, "10s"),
    LexEntry::new("num_alpys", "алпыс", &[A, L, P, S], Pos::Numeral, "10s"), // epenthetic «ы»
    LexEntry::new(
        "num_jetpis",
        "жетпіс",
        &[Zh, E, T, P, S],
        Pos::Numeral,
        "10s",
    ), // epenthetic «і»
    LexEntry::new(
        "num_seksen",
        "сексен",
        &[S, E, K, S, E, N],
        Pos::Numeral,
        "10s",
    ),
    LexEntry::new(
        "num_toqsan",
        "тоқсан",
        &[T, O, Q, S, A, N],
        Pos::Numeral,
        "10s",
    ),
    LexEntry::new("num_juz", "жүз", &[Zh, Ue, Z], Pos::Numeral, "100s"),
    LexEntry::new("num_myng", "мың", &[M, Y, Ng], Pos::Numeral, "1000s"),
];

pub const INTERJECTIONS: &[LexEntry] = &[
    LexEntry::new("intj_iya", "иә", &[I, Ae], Pos::Interjection, "answer"), // yes
    LexEntry::new("intj_joq", "жоқ", &[Zh, O, Q], Pos::Interjection, "answer"), // no
    LexEntry::new(
        "intj_rahmet",
        "рақмет",
        &[R, A, Q, M, E, T],
        Pos::Interjection,
        "thanks",
    ), // thanks
    LexEntry::new(
        "intj_salem",
        "сәлем",
        &[S, Ae, L, E, M],
        Pos::Interjection,
        "greeting",
    ),
    LexEntry::new(
        "intj_qosh",
        "қош",
        &[Q, O, Sh],
        Pos::Interjection,
        "farewell",
    ),
    LexEntry::new("intj_kesh", "кеш", &[K, E, Sh], Pos::Interjection, "time"), // evening
    LexEntry::new(
        "intj_kane",
        "қане",
        &[Q, A, N, E],
        Pos::Interjection,
        "exhort",
    ), // come on
    LexEntry::new(
        "intj_jaraidy",
        "жарайды",
        &[Zh, A, R, A, J, D, Y],
        Pos::Interjection,
        "agree",
    ), // ok
];

pub const ADVERBS: &[LexEntry] = &[
    LexEntry::new("adv_bugin", "бүгін", &[B, Ue, G, N], Pos::Adverb, "time"), // today — epenthetic «і»
    LexEntry::new(
        "adv_erteng",
        "ертең",
        &[E, R, T, E, Ng],
        Pos::Adverb,
        "time",
    ), // tomorrow
    LexEntry::new("adv_kesh", "кеше", &[K, E, Sh, E], Pos::Adverb, "time"),   // yesterday
    LexEntry::new("adv_qazir", "қазір", &[Q, A, Z, R], Pos::Adverb, "time"), // now — epenthetic «і»
    LexEntry::new("adv_kop", "көп", &[K, Oe, P], Pos::Adverb, "quantity"),   // a lot
    LexEntry::new("adv_az", "аз", &[A, Z], Pos::Adverb, "quantity"),         // a few
    LexEntry::new("adv_baryp", "барып", &[B, A, R, P], Pos::Adverb, "manner"), // having gone — epenthetic
    LexEntry::new("adv_oqyp", "оқып", &[O, Q, P], Pos::Adverb, "manner"), // having read — epenthetic
];
