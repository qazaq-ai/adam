// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `adam-stt-phoneme` — **Phase 6 of the v6.3 phonemic-
//! foundation arc**: pure-Rust phoneme-level speech recognition
//! via DTW (Dynamic Time Warping) against a per-phoneme MFCC
//! template bank.
//!
//! ## Why this exists
//!
//! v6.2 used Whisper for STT. Whisper is excellent but: (a)
//! C dependency, (b) trained on graphemes so it produces
//! orthographically-noisy output («толған» where the speaker
//! said «туылған»), (c) heavyweight (~1.6 GB model, 2–3 s
//! latency per query). v6.3 replaces it with a deterministic
//! template-matching recogniser that operates on **phonemes**,
//! avoiding the orthographic-noise class of bugs entirely.
//!
//! ## Pipeline
//!
//! ```text
//!   PCM audio (mono, 16 kHz)
//!     ↓ adam_audio::mfcc::mfcc()
//!   MfccSequence (T frames × 13 coefficients)
//!     ↓ DTW against each Phoneme's reference template
//!   Best-matching Phoneme per query segment
//!     ↓ Optional phonotactic-FST rescoring
//!   Vec<Phoneme> — the recognised phoneme stream
//! ```
//!
//! ## What this crate ships now
//!
//! - [`distance`] — frame-level distance metrics (Euclidean,
//!   cosine) used inside DTW.
//! - [`dtw`] — the DTW algorithm + alignment-path recovery.
//! - [`bank`] — [`PhonemeBank`] storing per-phoneme reference
//!   MFCC templates. **Phase 6 ships with synthetic templates
//!   for testing**; Phase 2d will replace them with corpus-
//!   extracted real templates.
//! - [`recogniser`] — the top-level `recognise(&MfccSequence,
//!   &PhonemeBank)` API.
//!
//! ## Honest limitations (first pass)
//!
//! - **Templates are synthetic.** Until Phase 2d delivers
//!   forced-alignment-extracted per-phoneme MFCC averages,
//!   the bank's templates come from `harmonic_voice` + the
//!   phoneme's nominal formant centres — useful for
//!   algorithm validation but **not yet production-quality
//!   ASR**.
//! - **No phonotactic rescoring yet.** The recogniser picks
//!   the closest template per query window independently;
//!   a future pass will rescore the per-frame top-K via the
//!   `adam-phonotactics` FST so morphologically illegal
//!   sequences get downweighted.
//! - **No speaker adaptation.** Real ASR adapts to the
//!   speaker's vocal-tract length / accent; we don't.
//!
//! These limitations are scoped: each becomes a Phase 6b /
//! Phase 6c milestone once the foundation is in.

#![forbid(unsafe_code)]

pub mod bank;
pub mod distance;
pub mod dtw;
pub mod recogniser;
pub mod word;

pub use bank::{PhonemeBank, PhonemeTemplate};
pub use distance::{cosine_distance, euclidean_distance};
pub use dtw::{DtwResult, dtw};
pub use recogniser::{RecognitionResult, recognise};
pub use word::{WordConfig, recognise_word};
