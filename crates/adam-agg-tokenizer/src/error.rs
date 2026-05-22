// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Error types for `adam-agg-tokenizer`.

use thiserror::Error;

/// Detokenize-time error: a malformed token slice that doesn't translate
/// to a valid morpheme bundle for the FST synthesiser.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DetokError {
    #[error("empty token slice")]
    Empty,
    #[error("first token is not a Root or Unk")]
    NotARoot,
    #[error("token following root is not a Suffix")]
    NotASuffix,
    #[error("Unk token cannot carry a suffix")]
    UnkWithSuffix,
    #[error("verb-only suffix attached to noun root")]
    VerbSuffixOnNoun,
    #[error("noun-only suffix attached to verb root")]
    NounSuffixOnVerb,
}

/// Round-trip error: tokenize + detokenize chain failed at one of the
/// steps. Currently the only failure path is detokenize.
#[derive(Debug, Error)]
pub enum RoundTripError {
    #[error("detokenize failed: {0}")]
    Detok(#[from] DetokError),
}
