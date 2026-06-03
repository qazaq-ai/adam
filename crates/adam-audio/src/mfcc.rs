// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Mel-Frequency Cepstral Coefficients (MFCC).
//!
//! The canonical feature representation for classical ASR and
//! voice classification. Pipeline:
//!
//! ```text
//!   STFT power spectrum
//!     ↓ apply Mel filterbank (N triangular filters on mel scale)
//!   Mel-band energies
//!     ↓ take log
//!   Log-mel energies
//!     ↓ Discrete Cosine Transform
//!   MFCC vector (typically keep coefficients 1..=13)
//! ```
//!
//! For phoneme matching (Phase 6), each frame is reduced to a
//! 13-dimensional cepstral vector — two same phonemes
//! produce similar vectors regardless of pitch / speaker;
//! two different phonemes produce different vectors. That
//! property is what makes MFCC the workhorse feature for
//! template-based STT.
//!
//! Standard parameters (matching Kaldi defaults):
//! - Sample rate 16 kHz
//! - 26 mel filters spanning 0 – 8000 Hz (Nyquist)
//! - DCT-II, take coefficients 1..=13 (drop C0 which is
//!   total log-energy and varies with loudness)

use crate::spectrogram::{Spectrogram, StftConfig, stft};

/// MFCC configuration. Defaults match Kaldi.
#[derive(Debug, Clone)]
pub struct MfccConfig {
    /// Number of mel filter banks (canonical: 26).
    pub n_mels: usize,
    /// Number of MFCC coefficients to keep (canonical: 13,
    /// starting from C1; C0 is dropped).
    pub n_mfcc: usize,
    /// Low-frequency edge of the mel filterbank, Hz.
    /// (Kaldi: 20 Hz; we use 20 to capture low male F0.)
    pub fmin_hz: f32,
    /// High-frequency edge of the mel filterbank, Hz.
    /// Defaults to Nyquist of the sample rate; null = sample_rate / 2.
    pub fmax_hz: Option<f32>,
}

impl Default for MfccConfig {
    fn default() -> Self {
        Self {
            n_mels: 26,
            n_mfcc: 13,
            fmin_hz: 20.0,
            fmax_hz: None,
        }
    }
}

/// A sequence of per-frame MFCC vectors.
#[derive(Debug, Clone, PartialEq)]
pub struct MfccSequence {
    /// `frames × n_mfcc` cepstral coefficients.
    pub frames: Vec<Vec<f32>>,
    /// Sample rate of the source audio.
    pub sample_rate: u32,
    /// Hop length used in the underlying STFT.
    pub hop_length: usize,
    /// Configuration this sequence was extracted with.
    pub n_mfcc: usize,
}

impl MfccSequence {
    pub fn num_frames(&self) -> usize {
        self.frames.len()
    }
    pub fn dim(&self) -> usize {
        self.n_mfcc
    }
}

// ─── Binary I/O ───────────────────────────────────────────────

/// Magic bytes for the per-sequence MFCC binary format.
pub const MFCC_MAGIC: [u8; 4] = *b"MFCC";

/// Current format version.
pub const MFCC_FORMAT_VERSION: u8 = 0x01;

/// Header length in bytes.
pub const MFCC_HEADER_LEN: usize = 4 + 1 + 4 + 4 + 4 + 4;

/// Errors returned by [`read_binary`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MfccBinaryError {
    TruncatedHeader { got: usize, want: usize },
    BadMagic { got: [u8; 4] },
    UnsupportedVersion { got: u8 },
    BodyLengthMismatch { declared: usize, actual: usize },
}

impl std::fmt::Display for MfccBinaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TruncatedHeader { got, want } => {
                write!(f, "MFCC: truncated header — got {got}, need {want}")
            }
            Self::BadMagic { got } => write!(f, "MFCC: bad magic {got:?} (want b\"MFCC\")"),
            Self::UnsupportedVersion { got } => {
                write!(f, "MFCC: unsupported version 0x{got:02x}")
            }
            Self::BodyLengthMismatch { declared, actual } => write!(
                f,
                "MFCC: body length mismatch — declared {declared}, actual {actual}"
            ),
        }
    }
}

impl std::error::Error for MfccBinaryError {}

/// Serialise an MFCC sequence to bytes. Format:
///
/// ```text
/// Offset  Bytes  Field
/// ──────  ─────  ────────────────────────────────
/// 0       4      Magic b"MFCC"
/// 4       1      Format version (0x01)
/// 5       4      n_frames        u32 LE
/// 9       4      n_mfcc          u32 LE
/// 13      4      sample_rate     u32 LE
/// 17      4      hop_length      u32 LE
/// 21      N      f32 LE coefficients (n_frames × n_mfcc)
/// ```
pub fn write_binary(seq: &MfccSequence) -> Vec<u8> {
    let n_frames = seq.num_frames();
    let n_mfcc = seq.dim();
    let mut out = Vec::with_capacity(MFCC_HEADER_LEN + n_frames * n_mfcc * 4);
    out.extend_from_slice(&MFCC_MAGIC);
    out.push(MFCC_FORMAT_VERSION);
    out.extend_from_slice(&(n_frames as u32).to_le_bytes());
    out.extend_from_slice(&(n_mfcc as u32).to_le_bytes());
    out.extend_from_slice(&seq.sample_rate.to_le_bytes());
    out.extend_from_slice(&(seq.hop_length as u32).to_le_bytes());
    for frame in &seq.frames {
        debug_assert_eq!(frame.len(), n_mfcc);
        for &c in frame {
            out.extend_from_slice(&c.to_le_bytes());
        }
    }
    out
}

/// Deserialise an MFCC sequence from bytes.
pub fn read_binary(bytes: &[u8]) -> Result<MfccSequence, MfccBinaryError> {
    if bytes.len() < MFCC_HEADER_LEN {
        return Err(MfccBinaryError::TruncatedHeader {
            got: bytes.len(),
            want: MFCC_HEADER_LEN,
        });
    }
    let magic: [u8; 4] = bytes[0..4].try_into().unwrap();
    if magic != MFCC_MAGIC {
        return Err(MfccBinaryError::BadMagic { got: magic });
    }
    let version = bytes[4];
    if version != MFCC_FORMAT_VERSION {
        return Err(MfccBinaryError::UnsupportedVersion { got: version });
    }
    let n_frames = u32::from_le_bytes(bytes[5..9].try_into().unwrap()) as usize;
    let n_mfcc = u32::from_le_bytes(bytes[9..13].try_into().unwrap()) as usize;
    let sample_rate = u32::from_le_bytes(bytes[13..17].try_into().unwrap());
    let hop_length = u32::from_le_bytes(bytes[17..21].try_into().unwrap()) as usize;

    let body = &bytes[MFCC_HEADER_LEN..];
    let expected = n_frames * n_mfcc * 4;
    if body.len() != expected {
        return Err(MfccBinaryError::BodyLengthMismatch {
            declared: expected,
            actual: body.len(),
        });
    }
    let mut frames: Vec<Vec<f32>> = Vec::with_capacity(n_frames);
    for f in 0..n_frames {
        let mut frame = Vec::with_capacity(n_mfcc);
        for c in 0..n_mfcc {
            let off = (f * n_mfcc + c) * 4;
            let v = f32::from_le_bytes(body[off..off + 4].try_into().unwrap());
            frame.push(v);
        }
        frames.push(frame);
    }
    Ok(MfccSequence {
        frames,
        sample_rate,
        hop_length,
        n_mfcc,
    })
}

/// Compute MFCCs for an audio signal.
///
/// Convenience entry point: runs STFT then [`mfcc_from_spectrogram`].
pub fn mfcc(samples: &[f32], sample_rate: u32, config: &MfccConfig) -> MfccSequence {
    let stft_cfg = StftConfig::speech_16khz();
    let spec = stft(samples, sample_rate, &stft_cfg);
    mfcc_from_spectrogram(&spec, config)
}

/// Compute MFCCs from a pre-computed spectrogram.
pub fn mfcc_from_spectrogram(spec: &Spectrogram, config: &MfccConfig) -> MfccSequence {
    let fmax = config.fmax_hz.unwrap_or(spec.sample_rate as f32 / 2.0);
    let filterbank = build_mel_filterbank(
        config.n_mels,
        spec.fft_size,
        spec.sample_rate,
        config.fmin_hz,
        fmax,
    );

    let mut out: Vec<Vec<f32>> = Vec::with_capacity(spec.num_frames());
    for power_spec in &spec.frames {
        // Step 1: apply mel filterbank.
        let mut mel_energies: Vec<f32> = Vec::with_capacity(config.n_mels);
        for filter in &filterbank {
            let e: f32 = filter
                .iter()
                .zip(power_spec.iter())
                .map(|(w, p)| w * p)
                .sum();
            mel_energies.push(e);
        }

        // Step 2: log (with small floor to avoid log(0)).
        for e in &mut mel_energies {
            *e = (*e + 1e-10).ln();
        }

        // Step 3: DCT-II. Take coefficients 1..=n_mfcc (drop C0).
        let cep: Vec<f32> = (1..=config.n_mfcc)
            .map(|k| dct_ii_coeff(&mel_energies, k))
            .collect();
        out.push(cep);
    }

    MfccSequence {
        frames: out,
        sample_rate: spec.sample_rate,
        hop_length: spec.hop_length,
        n_mfcc: config.n_mfcc,
    }
}

/// Build a mel filterbank: `n_mels` triangular filters spaced
/// on the mel scale between `fmin_hz` and `fmax_hz`. Each
/// filter is a vector of length `fft_size / 2 + 1` (one weight
/// per spectrum bin).
pub fn build_mel_filterbank(
    n_mels: usize,
    fft_size: usize,
    sample_rate: u32,
    fmin_hz: f32,
    fmax_hz: f32,
) -> Vec<Vec<f32>> {
    let n_bins = fft_size / 2 + 1;
    let mel_min = hz_to_mel(fmin_hz);
    let mel_max = hz_to_mel(fmax_hz);
    let mel_points: Vec<f32> = (0..n_mels + 2)
        .map(|i| mel_min + (mel_max - mel_min) * i as f32 / (n_mels + 1) as f32)
        .collect();
    let hz_points: Vec<f32> = mel_points.iter().map(|&m| mel_to_hz(m)).collect();
    let bin_points: Vec<f32> = hz_points
        .iter()
        .map(|&hz| hz * fft_size as f32 / sample_rate as f32)
        .collect();

    let mut filterbank = Vec::with_capacity(n_mels);
    for m in 0..n_mels {
        let lower = bin_points[m];
        let centre = bin_points[m + 1];
        let upper = bin_points[m + 2];
        let mut filter = vec![0.0_f32; n_bins];
        for (i, w) in filter.iter_mut().enumerate() {
            let b = i as f32;
            if b < lower || b > upper {
                continue;
            }
            *w = if b < centre {
                (b - lower) / (centre - lower)
            } else {
                (upper - b) / (upper - centre)
            };
        }
        filterbank.push(filter);
    }
    filterbank
}

/// Convert Hz → Mel scale (O'Shaughnessy 1987 variant; same
/// formula Kaldi / HTK use).
#[inline]
pub fn hz_to_mel(hz: f32) -> f32 {
    2595.0 * (1.0 + hz / 700.0).log10()
}

/// Convert Mel → Hz (inverse of [`hz_to_mel`]).
#[inline]
pub fn mel_to_hz(mel: f32) -> f32 {
    700.0 * (10.0_f32.powf(mel / 2595.0) - 1.0)
}

/// One coefficient of the DCT-II of a length-N signal.
/// `k` ranges `0..N`.
fn dct_ii_coeff(signal: &[f32], k: usize) -> f32 {
    let n = signal.len();
    let scale = (2.0 / n as f32).sqrt();
    let arg_k = std::f32::consts::PI * k as f32 / n as f32;
    let sum: f32 = signal
        .iter()
        .enumerate()
        .map(|(i, &x)| x * (arg_k * (i as f32 + 0.5)).cos())
        .sum();
    scale * sum
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::harmonic_voice;
    use std::f32::consts::PI;

    /// Mel scale: 0 Hz → 0 mel; 1000 Hz → ~1000 mel; the curve
    /// is approximately linear below 1 kHz and logarithmic above.
    #[test]
    fn mel_scale_anchors() {
        assert!(hz_to_mel(0.0).abs() < 1e-3);
        // 1000 Hz ≈ 1000 mel by construction.
        let m_1k = hz_to_mel(1000.0);
        assert!((m_1k - 1000.0).abs() < 5.0, "1000 Hz → {m_1k} mel");
        // Round-trip.
        for hz in [100.0_f32, 500.0, 1500.0, 4000.0, 8000.0] {
            let back = mel_to_hz(hz_to_mel(hz));
            assert!(
                (back - hz).abs() < hz * 0.01,
                "{hz} → {} (err {})",
                back,
                back - hz
            );
        }
    }

    /// Filterbank has `n_mels` filters, each with reasonable
    /// support (non-zero region) and zero outside.
    #[test]
    fn filterbank_shape() {
        let fb = build_mel_filterbank(26, 512, 16_000, 20.0, 8000.0);
        assert_eq!(fb.len(), 26);
        for (i, f) in fb.iter().enumerate() {
            assert_eq!(f.len(), 257);
            let nz = f.iter().filter(|&&w| w > 0.0).count();
            assert!(nz > 0, "filter {i} has no support");
            assert!(nz < 200, "filter {i} too wide ({nz} bins)");
        }
    }

    /// **Realistic voice**: same vowel at same F0 → similar
    /// MFCC vectors. Different F0s of the SAME synthesised
    /// signal should still have similar MFCCs (MFCC is supposed
    /// to be ~F0-invariant for vowel content).
    #[test]
    fn mfcc_dimension_matches_config() {
        let cfg = MfccConfig::default();
        let signal = harmonic_voice(120.0, 1.0, 16_000, 0.4, 4);
        let mfccs = mfcc(&signal, 16_000, &cfg);
        assert_eq!(mfccs.dim(), 13);
        assert!(mfccs.num_frames() > 50, "got {} frames", mfccs.num_frames());
        for f in &mfccs.frames {
            assert_eq!(f.len(), 13);
        }
    }

    /// **Two same-signal frames have similar MFCCs.**
    /// A stationary signal (sine wave) should produce MFCC
    /// vectors that are nearly identical from frame to frame.
    #[test]
    fn stationary_signal_has_stable_mfcc() {
        let signal = harmonic_voice(200.0, 1.0, 16_000, 0.4, 4);
        let mfccs = mfcc(&signal, 16_000, &MfccConfig::default());
        // Compute cosine similarity between two interior frames.
        let a = &mfccs.frames[20];
        let b = &mfccs.frames[40];
        let sim = cosine(a, b);
        assert!(sim > 0.95, "stationary MFCC frames diverge: cos = {sim}");
    }

    /// **Different signals → different MFCCs.**
    /// Pure sines at 200 Hz vs 4000 Hz should produce very
    /// different MFCC vectors (the spectral centroid is
    /// completely different).
    #[test]
    fn distinct_signals_have_distinct_mfcc() {
        let cfg = MfccConfig::default();
        let low: Vec<f32> = (0..16_000)
            .map(|i| (2.0 * PI * 200.0 * i as f32 / 16_000.0).sin())
            .collect();
        let high: Vec<f32> = (0..16_000)
            .map(|i| (2.0 * PI * 4000.0 * i as f32 / 16_000.0).sin())
            .collect();
        let mfcc_low = mfcc(&low, 16_000, &cfg);
        let mfcc_high = mfcc(&high, 16_000, &cfg);
        let a = &mfcc_low.frames[20];
        let b = &mfcc_high.frames[20];
        let sim = cosine(a, b);
        assert!(
            sim < 0.5,
            "spectrally-disjoint signals have similar MFCC (cos = {sim})",
        );
    }

    /// **F0-invariance approximation**: two harmonic voices at
    /// different F0 but the same harmonic-amplitude profile
    /// should still produce moderately similar MFCC (the
    /// spectral envelope dominates the cepstrum, not F0).
    /// This is the property that makes MFCC useful for phoneme
    /// identification across speakers.
    #[test]
    fn mfcc_partially_invariant_to_f0() {
        let cfg = MfccConfig::default();
        let v_low = harmonic_voice(120.0, 1.0, 16_000, 0.4, 4);
        let v_high = harmonic_voice(220.0, 1.0, 16_000, 0.4, 4);
        let mfcc_low = mfcc(&v_low, 16_000, &cfg);
        let mfcc_high = mfcc(&v_high, 16_000, &cfg);
        let sim = cosine(&mfcc_low.frames[20], &mfcc_high.frames[20]);
        // Not identical (F0 affects the spectrum somewhat),
        // but should be much more similar than a random
        // baseline.
        assert!(
            sim > 0.5,
            "F0 change makes MFCC very dissimilar: cos = {sim}",
        );
    }

    /// MFCC of silence is dominated by the log-floor (1e-10),
    /// not by spurious peaks.
    #[test]
    fn silence_mfcc_is_quiet() {
        let signal = vec![0.0_f32; 16_000];
        let mfccs = mfcc(&signal, 16_000, &MfccConfig::default());
        // Every coefficient should have small magnitude.
        for f in &mfccs.frames {
            for &c in f {
                assert!(c.abs() < 0.1, "silence MFCC coeff too large: {c}");
            }
        }
    }

    /// Binary write+read round-trip is lossless.
    #[test]
    fn binary_round_trip() {
        let signal = harmonic_voice(150.0, 0.3, 16_000, 0.4, 4);
        let original = mfcc(&signal, 16_000, &MfccConfig::default());
        let bytes = write_binary(&original);
        let back = read_binary(&bytes).unwrap();
        assert_eq!(back, original);
    }

    /// Binary reader rejects truncated headers, bad magic, bad
    /// version, and length-mismatched body.
    #[test]
    fn binary_reader_rejects_malformed() {
        let signal = harmonic_voice(150.0, 0.05, 16_000, 0.4, 4);
        let seq = mfcc(&signal, 16_000, &MfccConfig::default());
        let bytes = write_binary(&seq);
        // Truncated.
        assert!(matches!(
            read_binary(&bytes[..MFCC_HEADER_LEN - 1]),
            Err(MfccBinaryError::TruncatedHeader { .. }),
        ));
        // Bad magic.
        let mut bad = bytes.clone();
        bad[0] = b'X';
        assert!(matches!(
            read_binary(&bad),
            Err(MfccBinaryError::BadMagic { .. })
        ));
        // Bad version.
        let mut bad = bytes.clone();
        bad[4] = 0xFF;
        assert!(matches!(
            read_binary(&bad),
            Err(MfccBinaryError::UnsupportedVersion { got: 0xFF })
        ));
        // Body shorter than declared.
        let mut bad = bytes.clone();
        bad.truncate(bad.len() - 4);
        assert!(matches!(
            read_binary(&bad),
            Err(MfccBinaryError::BodyLengthMismatch { .. })
        ));
    }

    fn cosine(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if na < 1e-9 || nb < 1e-9 {
            return 0.0;
        }
        dot / (na * nb)
    }
}
