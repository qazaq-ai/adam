// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Phase 12 (2026-05-31): per-**morpheme** PCM template bank.
//!
//! User architecture directive (2026-05-31):
//!
//! > «Сначала фундамент алфавита, потом морфемы, потом
//! >  слова, потом предложения.»
//!
//! [`crate::PcmBank`] holds one PCM clip per **phoneme** (the
//! alphabet layer). This module adds the **next layer up**:
//! one PCM clip per **morpheme** — a syllable, a suffix
//! particle, or any short multi-phoneme token that the
//! corpus has recorded as a coherent unit.
//!
//! Source: the kaz-tili.kz drill recordings (323 short audio
//! clips, each a deliberate, isolated articulation of a
//! morpheme pair like «ша/ше» or «тайын/дейін»). The drill
//! format is structurally a morpheme inventory — it is exactly
//! the layer the user identified as missing between alphabet
//! and word.
//!
//! ## Why morphemes matter for TTS
//!
//! Concatenating phoneme-level PCM clips makes audible
//! stitching artifacts at every consonant-vowel boundary
//! (~37 boundaries in a 10-phoneme word). Concatenating
//! morpheme-level clips reduces those boundaries to the
//! number of morphemes (typically 1..3 per word — agglutinative
//! Kazakh decomposes cleanly into root + 1..3 suffixes).
//!
//! ## Lookup strategy
//!
//! At synthesis time the caller does **greedy longest-prefix
//! match** from the left of the input word: try the whole word,
//! drop one character at a time, until a prefix hits the bank;
//! emit that PCM, recurse on the remainder. When no prefix
//! matches at all the synthesiser falls back to the phoneme
//! bank for that character span.
//!
//! ## Binary format (version 1)
//!
//! ```text
//! magic       b"MORM"    4 bytes
//! version     u8 = 1     1 byte
//! count       u32 LE     4 bytes
//! for each entry:
//!   cyr_bytes  u32 LE    4 bytes  (UTF-8 length)
//!   cyr        n bytes            (lowercase Kazakh Cyrillic)
//!   sample_rate u32 LE   4 bytes
//!   n_samples  u32 LE    4 bytes
//!   samples    f32 LE × n_samples
//! ```
//!
//! Entries are written in lexicographic order on `cyrillic` so
//! the on-disk artifact is deterministic.

use std::collections::HashMap;
use std::path::Path;

/// One per-morpheme PCM template.
#[derive(Debug, Clone)]
pub struct MorphemeTemplate {
    /// The morpheme's lowercase Cyrillic form.
    pub cyrillic: Box<str>,
    pub sample_rate: u32,
    pub samples: Vec<f32>,
}

#[derive(Debug, Clone, Default)]
pub struct MorphemeBank {
    templates: HashMap<Box<str>, MorphemeTemplate>,
}

impl MorphemeBank {
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    /// Insert a template. If `cyrillic` is already present the
    /// existing entry is REPLACED — callers that want multi-
    /// exemplar coverage should aggregate before insertion.
    pub fn insert(&mut self, template: MorphemeTemplate) {
        self.templates.insert(template.cyrillic.clone(), template);
    }

    pub fn get(&self, cyrillic: &str) -> Option<&MorphemeTemplate> {
        self.templates.get(cyrillic)
    }

    pub fn contains(&self, cyrillic: &str) -> bool {
        self.templates.contains_key(cyrillic)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &MorphemeTemplate)> {
        self.templates.iter().map(|(k, v)| (k.as_ref(), v))
    }

    pub fn len(&self) -> usize {
        self.templates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }

    /// Greedy longest-prefix match. Returns the **shortest
    /// remaining suffix** of `word` after consuming a contiguous
    /// run of known morphemes from the left, alongside the list
    /// of consumed-morpheme templates in order.
    ///
    /// Empty word → empty result. No match on the first char →
    /// empty match, returns the whole word as remainder.
    pub fn match_greedy<'a, 'w>(&'a self, word: &'w str) -> (Vec<&'a MorphemeTemplate>, &'w str) {
        let mut matched: Vec<&MorphemeTemplate> = Vec::new();
        let mut cursor = 0_usize;
        let bytes = word.as_bytes();

        while cursor < bytes.len() {
            // Search for the longest prefix of word[cursor..]
            // that is in the bank. Walk byte-by-byte from the
            // full-suffix length down, only stopping at valid
            // char boundaries.
            let suffix = &word[cursor..];
            let mut best: Option<&MorphemeTemplate> = None;
            let mut best_byte_end = 0_usize;
            // Iterate every char boundary inside suffix (end-
            // exclusive). Longest first.
            let mut candidate_ends: Vec<usize> = suffix
                .char_indices()
                .map(|(i, c)| i + c.len_utf8())
                .collect();
            candidate_ends.sort_unstable_by(|a, b| b.cmp(a));
            for end in candidate_ends {
                let cand = &suffix[..end];
                if let Some(t) = self.templates.get(cand) {
                    best = Some(t);
                    best_byte_end = end;
                    break;
                }
            }
            match best {
                Some(t) => {
                    matched.push(t);
                    cursor += best_byte_end;
                }
                None => break,
            }
        }
        (matched, &word[cursor..])
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"MORM");
        out.push(0x01);
        out.extend_from_slice(&(self.templates.len() as u32).to_le_bytes());

        let mut entries: Vec<_> = self.templates.values().collect();
        entries.sort_by_key(|t| t.cyrillic.clone());

        for t in entries {
            let cyr_bytes = t.cyrillic.as_bytes();
            out.extend_from_slice(&(cyr_bytes.len() as u32).to_le_bytes());
            out.extend_from_slice(cyr_bytes);
            out.extend_from_slice(&t.sample_rate.to_le_bytes());
            out.extend_from_slice(&(t.samples.len() as u32).to_le_bytes());
            for &s in &t.samples {
                out.extend_from_slice(&s.to_le_bytes());
            }
        }
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, MorphemeBankError> {
        if bytes.len() < 9 {
            return Err(MorphemeBankError::TruncatedHeader);
        }
        if &bytes[0..4] != b"MORM" {
            return Err(MorphemeBankError::BadMagic);
        }
        if bytes[4] != 0x01 {
            return Err(MorphemeBankError::UnsupportedVersion(bytes[4]));
        }
        let count = u32::from_le_bytes(bytes[5..9].try_into().unwrap()) as usize;
        let mut cursor = 9_usize;
        let mut bank = Self::new();
        for _ in 0..count {
            if cursor + 4 > bytes.len() {
                return Err(MorphemeBankError::TruncatedEntry);
            }
            let n = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
            cursor += 4;
            if cursor + n > bytes.len() {
                return Err(MorphemeBankError::TruncatedEntry);
            }
            let cyr = match std::str::from_utf8(&bytes[cursor..cursor + n]) {
                Ok(s) => s.to_string(),
                Err(_) => return Err(MorphemeBankError::InvalidUtf8),
            };
            cursor += n;
            if cursor + 8 > bytes.len() {
                return Err(MorphemeBankError::TruncatedEntry);
            }
            let sample_rate = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
            cursor += 4;
            let n_samples =
                u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
            cursor += 4;
            if cursor + n_samples * 4 > bytes.len() {
                return Err(MorphemeBankError::TruncatedEntry);
            }
            let mut samples = Vec::with_capacity(n_samples);
            for _ in 0..n_samples {
                let v = f32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
                samples.push(v);
                cursor += 4;
            }
            bank.insert(MorphemeTemplate {
                cyrillic: cyr.into_boxed_str(),
                sample_rate,
                samples,
            });
        }
        Ok(bank)
    }

    pub fn save_to_file(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        std::fs::write(path, self.to_bytes())
    }

    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, MorphemeBankError> {
        let bytes = std::fs::read(path).map_err(MorphemeBankError::Io)?;
        Self::from_bytes(&bytes)
    }
}

#[derive(Debug)]
pub enum MorphemeBankError {
    Io(std::io::Error),
    TruncatedHeader,
    TruncatedEntry,
    BadMagic,
    UnsupportedVersion(u8),
    InvalidUtf8,
}

impl std::fmt::Display for MorphemeBankError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "morpheme bank: I/O error: {e}"),
            Self::TruncatedHeader => write!(f, "morpheme bank: truncated header"),
            Self::TruncatedEntry => write!(f, "morpheme bank: truncated entry"),
            Self::BadMagic => write!(f, "morpheme bank: bad magic (want b\"MORM\")"),
            Self::UnsupportedVersion(v) => {
                write!(f, "morpheme bank: unsupported version {v:#x}")
            }
            Self::InvalidUtf8 => write!(f, "morpheme bank: invalid UTF-8 in cyrillic field"),
        }
    }
}

impl std::error::Error for MorphemeBankError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(cyr: &str, samples: Vec<f32>) -> MorphemeTemplate {
        MorphemeTemplate {
            cyrillic: cyr.to_string().into_boxed_str(),
            sample_rate: 16_000,
            samples,
        }
    }

    #[test]
    fn round_trip_one_entry() {
        let mut b = MorphemeBank::new();
        b.insert(t("ша", vec![0.1, 0.2, 0.3]));
        let bytes = b.to_bytes();
        let restored = MorphemeBank::from_bytes(&bytes).unwrap();
        assert_eq!(restored.len(), 1);
        let got = restored.get("ша").unwrap();
        assert_eq!(got.cyrillic.as_ref(), "ша");
        assert_eq!(got.samples, vec![0.1, 0.2, 0.3]);
    }

    #[test]
    fn greedy_longest_prefix() {
        let mut b = MorphemeBank::new();
        b.insert(t("бала", vec![0.0; 4]));
        b.insert(t("лар", vec![0.0; 3]));
        b.insert(t("ба", vec![0.0; 2])); // shorter; longest should still pick "бала"
        let (matched, rem) = b.match_greedy("балалар");
        assert_eq!(matched.len(), 2);
        assert_eq!(matched[0].cyrillic.as_ref(), "бала");
        assert_eq!(matched[1].cyrillic.as_ref(), "лар");
        assert_eq!(rem, "");
    }

    #[test]
    fn greedy_unknown_remainder_returned() {
        let mut b = MorphemeBank::new();
        b.insert(t("қ", vec![0.0; 1]));
        let (matched, rem) = b.match_greedy("қалай");
        // After consuming «қ», rest is «алай» which has no entry.
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].cyrillic.as_ref(), "қ");
        assert_eq!(rem, "алай");
    }

    #[test]
    fn greedy_no_match_returns_whole_word() {
        let b = MorphemeBank::new();
        let (matched, rem) = b.match_greedy("сәлем");
        assert!(matched.is_empty());
        assert_eq!(rem, "сәлем");
    }

    #[test]
    fn bad_magic_rejected() {
        let bytes = b"XXXX\x01\x00\x00\x00\x00";
        assert!(matches!(
            MorphemeBank::from_bytes(bytes),
            Err(MorphemeBankError::BadMagic)
        ));
    }
}
