// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Per-phoneme PCM template bank — the TTS counterpart of
//! `adam_stt_phoneme::PhonemeBank`.
//!
//! While the MFCC bank powers recognition (DTW matching),
//! the PCM bank powers synthesis. The two are produced
//! together by `corpus_acquire build-bank` from the same
//! source audio + alignment, so the per-phoneme regions are
//! consistent across STT and TTS.
//!
//! ## Binary format
//!
//! ```text
//! magic     b"PCMB"   4 bytes
//! version   u8 = 1    1 byte
//! count     u32 LE    4 bytes
//! for each entry:
//!   phoneme_id   u8                 (Phoneme::to_byte())
//!   sample_rate  u32 LE
//!   n_samples    u32 LE
//!   samples      f32 LE × n_samples
//! ```
//!
//! Entries are written in stable phoneme-byte order for
//! deterministic output.

use adam_phoneme::Phoneme;
use std::collections::HashMap;
use std::path::Path;

/// One per-phoneme PCM template: a short f32 mono PCM segment
/// that represents the phoneme's average acoustic form.
#[derive(Debug, Clone)]
pub struct PcmTemplate {
    pub phoneme: Phoneme,
    pub sample_rate: u32,
    pub samples: Vec<f32>,
}

/// A bank of per-phoneme PCM templates.
#[derive(Debug, Clone, Default)]
pub struct PcmBank {
    templates: HashMap<Phoneme, PcmTemplate>,
}

impl PcmBank {
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    pub fn insert(&mut self, template: PcmTemplate) {
        self.templates.insert(template.phoneme, template);
    }

    pub fn get(&self, phoneme: Phoneme) -> Option<&PcmTemplate> {
        self.templates.get(&phoneme)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Phoneme, &PcmTemplate)> {
        self.templates.iter()
    }

    pub fn len(&self) -> usize {
        self.templates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }

    /// Serialise to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"PCMB");
        out.push(0x01);
        out.extend_from_slice(&(self.templates.len() as u32).to_le_bytes());
        let mut entries: Vec<_> = self.templates.iter().collect();
        entries.sort_by_key(|(p, _)| p.to_byte());
        for (phoneme, template) in entries {
            out.push(phoneme.to_byte());
            out.extend_from_slice(&template.sample_rate.to_le_bytes());
            out.extend_from_slice(&(template.samples.len() as u32).to_le_bytes());
            for &s in &template.samples {
                out.extend_from_slice(&s.to_le_bytes());
            }
        }
        out
    }

    /// Deserialise from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PcmBankError> {
        if bytes.len() < 9 {
            return Err(PcmBankError::TruncatedHeader);
        }
        if &bytes[0..4] != b"PCMB" {
            return Err(PcmBankError::BadMagic);
        }
        if bytes[4] != 0x01 {
            return Err(PcmBankError::UnsupportedVersion(bytes[4]));
        }
        let count = u32::from_le_bytes(bytes[5..9].try_into().unwrap()) as usize;
        let mut cursor = 9;
        let mut bank = Self::new();
        for _ in 0..count {
            if cursor + 9 > bytes.len() {
                return Err(PcmBankError::TruncatedEntry);
            }
            let phoneme_byte = bytes[cursor];
            cursor += 1;
            let phoneme = Phoneme::from_byte(phoneme_byte)
                .ok_or(PcmBankError::UnknownPhonemeByte(phoneme_byte))?;
            let sample_rate = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
            cursor += 4;
            let n_samples =
                u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
            cursor += 4;
            if cursor + n_samples * 4 > bytes.len() {
                return Err(PcmBankError::TruncatedEntry);
            }
            let mut samples = Vec::with_capacity(n_samples);
            for _ in 0..n_samples {
                let v = f32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
                samples.push(v);
                cursor += 4;
            }
            bank.insert(PcmTemplate {
                phoneme,
                sample_rate,
                samples,
            });
        }
        Ok(bank)
    }

    pub fn save_to_file(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        std::fs::write(path, self.to_bytes())
    }

    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, PcmBankError> {
        let bytes = std::fs::read(path).map_err(PcmBankError::Io)?;
        Self::from_bytes(&bytes)
    }
}

/// Errors from PCM bank serialisation / loading.
#[derive(Debug)]
pub enum PcmBankError {
    Io(std::io::Error),
    TruncatedHeader,
    TruncatedEntry,
    BadMagic,
    UnsupportedVersion(u8),
    UnknownPhonemeByte(u8),
}

impl std::fmt::Display for PcmBankError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "PcmBank I/O: {e}"),
            Self::TruncatedHeader => write!(f, "PcmBank: truncated header"),
            Self::TruncatedEntry => write!(f, "PcmBank: truncated entry"),
            Self::BadMagic => write!(f, "PcmBank: bad magic (want b\"PCMB\")"),
            Self::UnsupportedVersion(v) => write!(f, "PcmBank: unsupported version 0x{v:02x}"),
            Self::UnknownPhonemeByte(b) => write!(f, "PcmBank: unknown phoneme byte 0x{b:02x}"),
        }
    }
}

impl std::error::Error for PcmBankError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_bank_round_trip() {
        let b = PcmBank::new();
        let bytes = b.to_bytes();
        let back = PcmBank::from_bytes(&bytes).unwrap();
        assert_eq!(back.len(), 0);
    }

    #[test]
    fn populated_bank_round_trip() {
        let mut b = PcmBank::new();
        b.insert(PcmTemplate {
            phoneme: Phoneme::A,
            sample_rate: 16_000,
            samples: vec![0.1, 0.2, 0.3, -0.5, 0.0],
        });
        b.insert(PcmTemplate {
            phoneme: Phoneme::Q,
            sample_rate: 16_000,
            samples: vec![0.0; 100],
        });
        let bytes = b.to_bytes();
        let back = PcmBank::from_bytes(&bytes).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(
            back.get(Phoneme::A).unwrap().samples,
            vec![0.1, 0.2, 0.3, -0.5, 0.0]
        );
        assert_eq!(back.get(Phoneme::Q).unwrap().samples.len(), 100);
    }

    #[test]
    fn loader_rejects_malformed() {
        assert!(matches!(
            PcmBank::from_bytes(&[]),
            Err(PcmBankError::TruncatedHeader)
        ));
        let mut bad = b"XCMB\x01\x00\x00\x00\x00".to_vec();
        assert!(matches!(
            PcmBank::from_bytes(&bad),
            Err(PcmBankError::BadMagic)
        ));
        bad[0] = b'P';
        bad[4] = 0xFF;
        assert!(matches!(
            PcmBank::from_bytes(&bad),
            Err(PcmBankError::UnsupportedVersion(0xFF))
        ));
    }
}
