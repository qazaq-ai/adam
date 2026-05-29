// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Common Kazakh verb stems and infinitive forms.

use crate::{LexEntry, Pos};
use adam_phoneme::Phoneme::*;

pub const VERBS: &[LexEntry] = &[
    // Bare stems (imperative form).
    LexEntry::new("v_kel", "кел", &[K, E, L], Pos::Verb, "verb_stem"), // come!
    LexEntry::new("v_bar", "бар", &[B, A, R], Pos::Verb, "verb_stem"), // go!
    LexEntry::new("v_aitp", "айт", &[A, J, T], Pos::Verb, "verb_stem"), // say!
    LexEntry::new("v_oqy", "оқы", &[O, Q, Y], Pos::Verb, "verb_stem"), // read!
    LexEntry::new("v_kor", "көр", &[K, Oe, R], Pos::Verb, "verb_stem"), // see!
    LexEntry::new("v_bil", "біл", &[B, Yi, L], Pos::Verb, "verb_stem"), // know!
    LexEntry::new("v_jaz", "жаз", &[Zh, A, Z], Pos::Verb, "verb_stem"), // write!
    LexEntry::new("v_jur", "жүр", &[Zh, Ue, R], Pos::Verb, "verb_stem"), // walk / go!
    LexEntry::new("v_oj", "ой", &[O, J], Pos::Verb, "verb_stem"), // think (also noun «thought»)
    LexEntry::new("v_qara", "қара", &[Q, A, R, A], Pos::Verb, "verb_stem"), // look!
    LexEntry::new("v_otyr", "отыр", &[O, T, R], Pos::Verb, "verb_stem"), // sit! — epenthetic «ы»
    LexEntry::new("v_tur", "тұр", &[T, U, R], Pos::Verb, "verb_stem"), // stand!
    LexEntry::new("v_aly", "алы", &[A, L, Y], Pos::Verb, "verb_stem"), // take (informal)
    LexEntry::new("v_ber", "бер", &[B, E, R], Pos::Verb, "verb_stem"), // give!
    LexEntry::new("v_jasa", "жаса", &[Zh, A, S, A], Pos::Verb, "verb_stem"), // make / do!
    LexEntry::new("v_uqy", "ұқы", &[U, Q, Y], Pos::Verb, "verb_stem"), // understand
    LexEntry::new("v_oyna", "ойна", &[O, J, N, A], Pos::Verb, "verb_stem"), // play!
    LexEntry::new("v_oqy_p", "оқып", &[O, Q, P], Pos::Verb, "verb_form"), // reading — epenthetic «ы»
    // Infinitive (-у suffix).
    LexEntry::new("v_keluu", "келу", &[K, E, L, W], Pos::Verb, "verb_inf"), // to come
    LexEntry::new("v_baruu", "бару", &[B, A, R, W], Pos::Verb, "verb_inf"), // to go
    LexEntry::new("v_oquu", "оқу", &[O, Q, W], Pos::Verb, "verb_inf"),      // to read
    LexEntry::new("v_koruu", "көру", &[K, Oe, R, W], Pos::Verb, "verb_inf"), // to see
    LexEntry::new("v_biluu", "білу", &[B, Yi, L, W], Pos::Verb, "verb_inf"), // to know
    LexEntry::new(
        "v_jasauu",
        "жасау",
        &[Zh, A, S, A, W],
        Pos::Verb,
        "verb_inf",
    ), // to make
    LexEntry::new("v_jazuu", "жазу", &[Zh, A, Z, W], Pos::Verb, "verb_inf"), // to write
    LexEntry::new("v_juruu", "жүру", &[Zh, Ue, R, W], Pos::Verb, "verb_inf"), // to walk
    LexEntry::new("v_aytuu", "айту", &[A, J, T, W], Pos::Verb, "verb_inf"), // to say
    LexEntry::new("v_oyna_u", "ойнау", &[O, J, N, A, W], Pos::Verb, "verb_inf"), // to play
];
