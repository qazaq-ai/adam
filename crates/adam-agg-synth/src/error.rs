// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Error types for the synthetic data generator.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SynthError {
    #[error("lexicon load failed: {0}")]
    LexiconLoad(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
}
