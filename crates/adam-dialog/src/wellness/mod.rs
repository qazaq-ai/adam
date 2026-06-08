// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! # wellness — IFS / red-flag layer integrated into adam-dialog
//!
//! ## v6.4.0-rc6 architecture pivot (2026-06-08)
//!
//! Earlier `adam-wellness` shipped as a standalone crate and
//! re-implemented things adam-dialog already had — name capture,
//! Kazakh numeral parsing with case suffixes, multi-turn session
//! state, STT-noise tolerance.  Live audit user feedback (rc5):
//!
//! > «У нас уже все это было отработано и работало хорошо, почему
//! >  ты не продолжил новую функцию там, где все хорошо работает.
//! >  Получается дублирование того, что уже хорошо работает.»
//!
//! rc6 moves the wellness logic INTO adam-dialog as a submodule.
//! Name extraction now uses `semantics::interpret_text` →
//! `Intent::StatementOfName`.  Kazakh numerals route through
//! `adam_algebra::math_solver`.  No more parallel cascade.
//!
//! Submodules:
//! - [`red_flags`] — defence-in-depth crisis detector (suicidal,
//!   acute medical, child abuse, IPV, psychosis).  Token-bounded
//!   + STT-vowel-tolerant phonetic anchors.
//! - [`ifs`] — Internal Family Systems state machine that opens
//!   with conversational intake (greet → name → age → problem)
//!   then runs the 6-stage parts-work cycle.

pub mod ifs;
pub mod red_flags;
