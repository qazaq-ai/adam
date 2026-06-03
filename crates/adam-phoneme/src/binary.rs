// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Binary format for phoneme streams.
//!
//! The v6.3 thesis stores the canonical form of every Kazakh word
//! / utterance as a typed phoneme sequence (not a Cyrillic / Latin
//! string). This module defines the **on-disk** representation of
//! such a sequence — a compact, versioned, pure-Rust binary
//! format with no external serialisation dependency.
//!
//! ## Format (version `0x01`)
//!
//! ```text
//! Offset  Bytes  Field
//! ──────  ─────  ──────────────────────────────────────────
//! 0       4      Magic: ASCII "PHST"  (b"PHST")
//! 4       1      Format version (currently 0x01)
//! 5       4      Phoneme count: u32 little-endian
//! 9       N      Phoneme bytes: each = phoneme discriminant
//!                from [`Phoneme::to_byte`] / [`Phoneme::from_byte`]
//! ```
//!
//! Header is fixed at 9 bytes; body is `count` bytes (one per
//! phoneme). A 5-phoneme word like «қазақ» = 9 + 5 = 14 bytes
//! on disk. A typical sentence (~30 phonemes) = 9 + 30 = 39 bytes.
//!
//! ## Why this format
//!
//! - **Compact:** 1 byte per phoneme, no padding, no string
//!   encoding overhead.
//! - **Versioned:** the leading byte after the magic lets future
//!   format extensions coexist with old files (the decoder can
//!   inspect and dispatch).
//! - **Self-describing length:** the count is in the header, so
//!   a stream can be embedded in larger files and the decoder
//!   knows exactly when to stop.
//! - **Pure Rust, zero deps:** no `serde`, no `bincode`, no
//!   `rkyv`. Just `u8` arithmetic. Means this crate stays
//!   dependency-free, which matters for the eventual ARM/watch
//!   deployment target.
//!
//! ## What this is NOT (yet)
//!
//! - A persistent lexicon file (Phase 8+ will define one that
//!   wraps phoneme streams with metadata: gloss, morphology,
//!   domain tags).
//! - The MFCC / waveform bank format (Layer 0b — that's a
//!   separate file format defined in a different module).
//! - A streaming codec for arbitrary-length audio transcripts
//!   (the current format reads the whole stream into memory).
//!   For Layer 0c+ we may add a chunked variant.

use crate::Phoneme;

/// Magic bytes for the phoneme-stream container format.
pub const MAGIC: [u8; 4] = *b"PHST";

/// Current format version.
pub const FORMAT_VERSION: u8 = 0x01;

/// Header length in bytes (magic + version + count).
pub const HEADER_LEN: usize = 4 + 1 + 4;

/// Error from [`decode_stream`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// Input is shorter than the fixed header.
    TruncatedHeader { got: usize, want: usize },
    /// First 4 bytes do not match [`MAGIC`].
    BadMagic { got: [u8; 4] },
    /// Format version byte is not [`FORMAT_VERSION`]. The byte is
    /// returned for diagnostics.
    UnsupportedVersion { got: u8 },
    /// Body length does not match the count declared in the
    /// header.
    LengthMismatch { declared: u32, actual: usize },
    /// A body byte is not a valid [`Phoneme`] discriminant. The
    /// offset (relative to the start of the body) and the
    /// offending byte are returned.
    UnknownPhonemeByte { offset: usize, byte: u8 },
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TruncatedHeader { got, want } => {
                write!(f, "truncated header: got {got} bytes, need {want}")
            }
            Self::BadMagic { got } => write!(f, "bad magic: {got:?} (expected b\"PHST\")"),
            Self::UnsupportedVersion { got } => {
                write!(f, "unsupported format version 0x{got:02x}")
            }
            Self::LengthMismatch { declared, actual } => write!(
                f,
                "length mismatch: header declares {declared} phonemes, body has {actual} bytes"
            ),
            Self::UnknownPhonemeByte { offset, byte } => {
                write!(
                    f,
                    "unknown phoneme byte 0x{byte:02x} at body offset {offset}"
                )
            }
        }
    }
}

impl std::error::Error for DecodeError {}

/// Encode a phoneme stream to a binary blob. The output is
/// `HEADER_LEN + phonemes.len()` bytes.
///
/// Panics if `phonemes.len()` exceeds `u32::MAX` (which would not
/// fit in the header's count field). For all practical use this
/// is unreachable — a 4 GB phoneme stream would correspond to
/// approximately a billion words of contiguous text.
pub fn encode_stream(phonemes: &[Phoneme]) -> Vec<u8> {
    let count: u32 = phonemes
        .len()
        .try_into()
        .expect("phoneme stream exceeds u32::MAX");
    let mut out = Vec::with_capacity(HEADER_LEN + phonemes.len());
    out.extend_from_slice(&MAGIC);
    out.push(FORMAT_VERSION);
    out.extend_from_slice(&count.to_le_bytes());
    for p in phonemes {
        out.push(p.to_byte());
    }
    out
}

/// Decode a phoneme stream from a binary blob. Validates the
/// magic, version, length and every body byte.
pub fn decode_stream(bytes: &[u8]) -> Result<Vec<Phoneme>, DecodeError> {
    if bytes.len() < HEADER_LEN {
        return Err(DecodeError::TruncatedHeader {
            got: bytes.len(),
            want: HEADER_LEN,
        });
    }
    let magic: [u8; 4] = bytes[0..4].try_into().unwrap();
    if magic != MAGIC {
        return Err(DecodeError::BadMagic { got: magic });
    }
    let version = bytes[4];
    if version != FORMAT_VERSION {
        return Err(DecodeError::UnsupportedVersion { got: version });
    }
    let count = u32::from_le_bytes(bytes[5..9].try_into().unwrap());
    let body = &bytes[HEADER_LEN..];
    if body.len() != count as usize {
        return Err(DecodeError::LengthMismatch {
            declared: count,
            actual: body.len(),
        });
    }
    let mut out = Vec::with_capacity(count as usize);
    for (offset, &b) in body.iter().enumerate() {
        match Phoneme::from_byte(b) {
            Some(p) => out.push(p),
            None => return Err(DecodeError::UnknownPhonemeByte { offset, byte: b }),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use Phoneme::*;

    /// Empty stream encodes to just the header (9 bytes).
    #[test]
    fn empty_stream_is_header_only() {
        let bytes = encode_stream(&[]);
        assert_eq!(bytes.len(), HEADER_LEN);
        assert_eq!(&bytes[0..4], &MAGIC);
        assert_eq!(bytes[4], FORMAT_VERSION);
        assert_eq!(&bytes[5..9], &[0, 0, 0, 0]);
        assert_eq!(decode_stream(&bytes).unwrap(), Vec::<Phoneme>::new());
    }

    /// One-phoneme stream round-trips and is HEADER_LEN+1 bytes.
    #[test]
    fn single_phoneme_round_trip() {
        let bytes = encode_stream(&[Q]);
        assert_eq!(bytes.len(), HEADER_LEN + 1);
        assert_eq!(bytes[HEADER_LEN], Q.to_byte());
        assert_eq!(decode_stream(&bytes).unwrap(), vec![Q]);
    }

    /// «қазақ» — 5 phonemes — round-trips losslessly.
    #[test]
    fn qazaq_round_trip() {
        let phonemes = vec![Q, A, Z, A, Q];
        let bytes = encode_stream(&phonemes);
        assert_eq!(bytes.len(), HEADER_LEN + 5);
        assert_eq!(decode_stream(&bytes).unwrap(), phonemes);
    }

    /// Every phoneme in `ALL` round-trips through encode/decode.
    #[test]
    fn all_phonemes_round_trip() {
        let phonemes: Vec<Phoneme> = Phoneme::ALL.to_vec();
        let bytes = encode_stream(&phonemes);
        let decoded = decode_stream(&bytes).unwrap();
        assert_eq!(decoded, phonemes);
    }

    /// to_byte / from_byte are exact inverses on the full alphabet.
    #[test]
    fn to_byte_from_byte_round_trip() {
        for p in Phoneme::ALL {
            let b = p.to_byte();
            assert_eq!(Phoneme::from_byte(b), Some(*p));
        }
    }

    /// to_byte produces values < 37 (one per phoneme).
    #[test]
    fn to_byte_within_inventory_bound() {
        for p in Phoneme::ALL {
            assert!(p.to_byte() < 37, "byte for {p:?} out of bound");
        }
    }

    /// Decoder rejects empty input as truncated header.
    #[test]
    fn empty_input_rejected_as_truncated() {
        assert!(matches!(
            decode_stream(&[]),
            Err(DecodeError::TruncatedHeader { .. })
        ));
    }

    /// Decoder rejects bad magic.
    #[test]
    fn bad_magic_rejected() {
        let mut bytes = encode_stream(&[A]);
        bytes[0] = b'X';
        assert!(matches!(
            decode_stream(&bytes),
            Err(DecodeError::BadMagic { .. })
        ));
    }

    /// Decoder rejects unsupported version.
    #[test]
    fn unsupported_version_rejected() {
        let mut bytes = encode_stream(&[A]);
        bytes[4] = 0xFF;
        assert!(matches!(
            decode_stream(&bytes),
            Err(DecodeError::UnsupportedVersion { got: 0xFF })
        ));
    }

    /// Decoder rejects body shorter than header-declared count.
    #[test]
    fn length_mismatch_rejected() {
        let mut bytes = encode_stream(&[A, B, D]);
        bytes.truncate(HEADER_LEN + 2);
        let r = decode_stream(&bytes);
        assert!(
            matches!(r, Err(DecodeError::LengthMismatch { .. })),
            "got {r:?}"
        );
    }

    /// Decoder rejects unknown phoneme byte in body.
    #[test]
    fn unknown_phoneme_byte_rejected() {
        let mut bytes = encode_stream(&[A, B]);
        // Corrupt body[1] (offset 1 inside body, absolute offset
        // HEADER_LEN + 1).
        bytes[HEADER_LEN + 1] = 99;
        let r = decode_stream(&bytes);
        assert!(
            matches!(
                r,
                Err(DecodeError::UnknownPhonemeByte {
                    offset: 1,
                    byte: 99
                })
            ),
            "got {r:?}"
        );
    }
}
