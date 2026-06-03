// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `adam-forced-aligner` — **Phase 10 of the v6.3 arc**: pure-
//! Rust forced aligner that replaces the Montreal Forced
//! Aligner (MFA) bootstrap.
//!
//! ## Problem
//!
//! `adam-corpus-acquire build-bank` ran a 2-pass extraction:
//!
//! 1. **Iteration 0** — equipartition each source's MFCC across
//!    its phoneme sequence (every phoneme gets `T/N` frames).
//! 2. **Iteration 1+** — DTW the source against a *concatenation
//!    of templates* (one chunk per phoneme in the source's
//!    sequence). The DTW path then dictated which source frames
//!    map to which phoneme.
//!
//! The iteration-1+ trick worked on tiny single-word recordings
//! but breaks on multi-word FLEURS sentences: the *expected*
//! length (sum of template frame-counts) is much shorter than
//! the source, so the DTW path bunches long phones into one
//! template chunk and starves short ones. "Longest chunk per
//! phoneme" then snaps the bank towards dominant sibilants and
//! the «қазақ» test regresses.
//!
//! ## Approach
//!
//! Classic left-to-right HMM forced alignment via Viterbi DP:
//!
//! ```text
//!   cost[t][n] = min(cost[t-1][n],          // stay in phoneme n
//!                    cost[t-1][n-1])         // advance to phoneme n
//!              + emission_cost(t, n)
//!
//!   emission_cost(t, n) = ‖audio[t] − template_mean[phoneme_seq[n]]‖₂
//! ```
//!
//! - DP table is `T × N` where `T = audio frames`,
//!   `N = phoneme count`. Self-loops let one phoneme span many
//!   frames — the root fix for the equipartition / DTW
//!   concatenation pathology.
//! - Trace-back yields a strictly monotonic frame-level
//!   alignment: every frame is assigned to exactly one phoneme,
//!   and every phoneme owns at least one contiguous frame range.
//! - Pure deterministic — same input ⇒ same output, byte-for-byte.
//!
//! ## Use
//!
//! ```rust,ignore
//! use adam_forced_aligner::align;
//! use adam_phoneme::Phoneme;
//!
//! let alignment = align(&audio_mfcc, &phoneme_sequence, &bank)?;
//! for seg in &alignment.segments {
//!     println!("{:?}: frames {}..{}", seg.phoneme, seg.start, seg.end);
//! }
//! ```

#![forbid(unsafe_code)]

use adam_audio::mfcc::MfccSequence;
use adam_phoneme::Phoneme;
use adam_stt_phoneme::PhonemeBank;

/// One aligned phoneme region within an audio MFCC sequence.
///
/// `start`/`end` are frame indices (half-open: `[start, end)`).
/// `end > start` is guaranteed — every phoneme owns at least
/// one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhoneSegment {
    pub phoneme: Phoneme,
    pub start: usize,
    pub end: usize,
}

/// Full forced-alignment result.
#[derive(Debug, Clone)]
pub struct Alignment {
    pub segments: Vec<PhoneSegment>,
    /// Total Viterbi cost of the best path. Lower = better fit
    /// of audio to the expected phoneme sequence under the
    /// given bank. Useful for diagnostics / filtering bad
    /// alignments (mistranscribed sources).
    pub total_cost: f32,
}

/// Forced-alignment error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlignError {
    /// Phoneme sequence is empty.
    EmptyPhonemes,
    /// Audio has no MFCC frames.
    EmptyAudio,
    /// More phonemes than frames — left-to-right HMM cannot
    /// satisfy the "each phoneme owns ≥1 frame" invariant.
    PhonemesExceedFrames { n_phonemes: usize, n_frames: usize },
    /// One or more phonemes have no template in the bank.
    UncoveredPhoneme(Phoneme),
}

impl std::fmt::Display for AlignError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyPhonemes => write!(f, "forced-aligner: empty phoneme sequence"),
            Self::EmptyAudio => write!(f, "forced-aligner: empty audio MFCC"),
            Self::PhonemesExceedFrames {
                n_phonemes,
                n_frames,
            } => write!(
                f,
                "forced-aligner: {n_phonemes} phonemes > {n_frames} frames"
            ),
            Self::UncoveredPhoneme(p) => {
                write!(f, "forced-aligner: bank has no template for {p:?}")
            }
        }
    }
}

impl std::error::Error for AlignError {}

/// Run Viterbi forced alignment.
///
/// `audio` — MFCC of the source recording (`T` frames).
/// `phoneme_sequence` — expected phonemes in order (`N` items).
/// `bank` — provides per-phoneme template centroids.
///
/// Returns a [`PhoneSegment`] per input phoneme with the
/// inferred frame range and the best-path Viterbi cost.
pub fn align(
    audio: &MfccSequence,
    phoneme_sequence: &[Phoneme],
    bank: &PhonemeBank,
) -> Result<Alignment, AlignError> {
    let t = audio.num_frames();
    let n = phoneme_sequence.len();
    if n == 0 {
        return Err(AlignError::EmptyPhonemes);
    }
    if t == 0 {
        return Err(AlignError::EmptyAudio);
    }
    if n > t {
        return Err(AlignError::PhonemesExceedFrames {
            n_phonemes: n,
            n_frames: t,
        });
    }

    // Pre-compute per-phoneme template centroids (mean frame).
    // The bank's `mfcc` field stores one MfccSequence per
    // phoneme — collapse to one vector per phoneme. For one-
    // frame templates the centroid is the template itself.
    let mut centroids: Vec<Vec<f32>> = Vec::with_capacity(n);
    for &p in phoneme_sequence {
        let tmpl = bank.get(p).ok_or(AlignError::UncoveredPhoneme(p))?;
        centroids.push(centroid_of(&tmpl.mfcc));
    }

    // ─── Viterbi DP ──────────────────────────────────────────
    //
    // cost[t][n]    = best total cost to be at phoneme n after
    //                 emitting frame t
    // parent[t][n]  = whether we got here by SELF (cost[t-1][n])
    //                 or ADVANCE (cost[t-1][n-1])
    //
    // Stored as flat Vec for cache locality. `idx(ti, ni) = ti*n + ni`.
    let inf = f32::INFINITY;
    let mut cost: Vec<f32> = vec![inf; t * n];
    let mut from_self: Vec<bool> = vec![false; t * n];
    let idx = |ti: usize, ni: usize| ti * n + ni;

    // t = 0: only phoneme 0 reachable.
    cost[idx(0, 0)] = euclid(&audio.frames[0], &centroids[0]);

    for ti in 1..t {
        // To leave room for the remaining phonemes (each needs
        // ≥1 frame), at frame ti we can be in any phoneme
        // index ni in [max(0, ti+n-t), min(ti, n-1)].
        let min_n = (ti + n).saturating_sub(t);
        let max_n = ti.min(n - 1);
        for ni in min_n..=max_n {
            let emit = euclid(&audio.frames[ti], &centroids[ni]);
            let stay = cost[idx(ti - 1, ni)];
            let advance = if ni > 0 {
                cost[idx(ti - 1, ni - 1)]
            } else {
                inf
            };
            if stay <= advance {
                cost[idx(ti, ni)] = stay + emit;
                from_self[idx(ti, ni)] = true;
            } else {
                cost[idx(ti, ni)] = advance + emit;
                from_self[idx(ti, ni)] = false;
            }
        }
    }

    // ─── Trace-back ───────────────────────────────────────────
    //
    // We force the path to land in phoneme n-1 at frame t-1
    // (the only valid endpoint for a complete alignment).
    let total_cost = cost[idx(t - 1, n - 1)];
    if !total_cost.is_finite() {
        // This shouldn't happen given n ≤ t and a complete bank,
        // but signal cleanly if it does.
        return Err(AlignError::PhonemesExceedFrames {
            n_phonemes: n,
            n_frames: t,
        });
    }

    let mut path = vec![0_usize; t]; // path[ti] = phoneme index at frame ti
    let mut ni = n - 1;
    path[t - 1] = ni;
    for ti in (1..t).rev() {
        if !from_self[idx(ti, ni)] && ni > 0 {
            ni -= 1;
        }
        path[ti - 1] = ni;
    }

    // Convert frame-level path → phoneme segments.
    let mut segments: Vec<PhoneSegment> = Vec::with_capacity(n);
    let mut cur_phon = path[0];
    let mut seg_start = 0_usize;
    for ti in 1..t {
        if path[ti] != cur_phon {
            segments.push(PhoneSegment {
                phoneme: phoneme_sequence[cur_phon],
                start: seg_start,
                end: ti,
            });
            cur_phon = path[ti];
            seg_start = ti;
        }
    }
    segments.push(PhoneSegment {
        phoneme: phoneme_sequence[cur_phon],
        start: seg_start,
        end: t,
    });

    debug_assert_eq!(
        segments.len(),
        n,
        "forced-aligner: produced {} segments for {} phonemes",
        segments.len(),
        n
    );

    Ok(Alignment {
        segments,
        total_cost,
    })
}

/// Mean MFCC frame of a template (used as the phoneme centroid
/// for the emission cost). A 1-frame template is its own
/// centroid; longer templates collapse to their per-coefficient
/// arithmetic mean.
fn centroid_of(seq: &MfccSequence) -> Vec<f32> {
    let n = seq.num_frames();
    debug_assert!(n > 0, "centroid_of: empty MFCC sequence");
    let dim = seq.dim();
    let mut mean = vec![0.0_f32; dim];
    for frame in &seq.frames {
        for (m, x) in mean.iter_mut().zip(frame.iter()) {
            *m += *x;
        }
    }
    let inv = 1.0 / n as f32;
    for m in &mut mean {
        *m *= inv;
    }
    mean
}

/// Euclidean distance between two equal-length f32 vectors.
/// Returns +∞ if dimensions disagree (caller bug, but degrades
/// safely — Viterbi will simply route around the offending
/// phoneme).
fn euclid(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return f32::INFINITY;
    }
    let mut sum = 0.0_f32;
    for (x, y) in a.iter().zip(b.iter()) {
        let d = x - y;
        sum += d * d;
    }
    sum.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use adam_audio::mfcc::MfccSequence;
    use adam_stt_phoneme::{PhonemeBank, PhonemeTemplate};

    fn frame_of(c: f32) -> Vec<f32> {
        // 13-dim MFCC frame where every coefficient = c.
        vec![c; 13]
    }

    fn seq_of(values: &[f32]) -> MfccSequence {
        MfccSequence {
            frames: values.iter().map(|&v| frame_of(v)).collect(),
            sample_rate: 16_000,
            hop_length: 160,
            n_mfcc: 13,
        }
    }

    fn bank_for(phonemes: &[(Phoneme, f32)]) -> PhonemeBank {
        let mut bank = PhonemeBank::new();
        for &(p, v) in phonemes {
            bank.insert(PhonemeTemplate {
                phoneme: p,
                mfcc: seq_of(&[v]),
            });
        }
        bank
    }

    /// Trivial 3-frame audio, 3 phonemes, exact match → each
    /// phoneme owns 1 frame.
    #[test]
    fn one_frame_per_phoneme() {
        use Phoneme::*;
        let audio = seq_of(&[1.0, 2.0, 3.0]);
        let bank = bank_for(&[(A, 1.0), (B, 2.0), (T, 3.0)]);
        let r = align(&audio, &[A, B, T], &bank).unwrap();
        assert_eq!(r.segments.len(), 3);
        assert_eq!(
            r.segments[0],
            PhoneSegment {
                phoneme: A,
                start: 0,
                end: 1
            }
        );
        assert_eq!(
            r.segments[1],
            PhoneSegment {
                phoneme: B,
                start: 1,
                end: 2
            }
        );
        assert_eq!(
            r.segments[2],
            PhoneSegment {
                phoneme: T,
                start: 2,
                end: 3
            }
        );
        assert!(r.total_cost.abs() < 1e-5);
    }

    /// Long phoneme stretches across multiple frames via
    /// self-loop — the equipartition / template-concat path
    /// breaks here, the Viterbi path doesn't.
    #[test]
    fn self_loop_absorbs_extra_frames() {
        use Phoneme::*;
        // Phoneme A spans 4 frames (centroid=1.0), then one
        // frame of B (centroid=5.0). 5 frames, 2 phonemes.
        let audio = seq_of(&[1.0, 1.0, 1.0, 1.0, 5.0]);
        let bank = bank_for(&[(A, 1.0), (B, 5.0)]);
        let r = align(&audio, &[A, B], &bank).unwrap();
        assert_eq!(
            r.segments[0],
            PhoneSegment {
                phoneme: A,
                start: 0,
                end: 4
            }
        );
        assert_eq!(
            r.segments[1],
            PhoneSegment {
                phoneme: B,
                start: 4,
                end: 5
            }
        );
    }

    /// Boundary picks the lowest total cost: A=1, B=5, with
    /// frames [1, 1, 2, 5, 5] — frame 2 is closer to A (cost 1)
    /// than to B (cost 3), so phoneme A must absorb it. Without
    /// self-loops the boundary would land at the wrong place.
    #[test]
    fn boundary_chooses_lowest_cost() {
        use Phoneme::*;
        let audio = seq_of(&[1.0, 1.0, 2.0, 5.0, 5.0]);
        let bank = bank_for(&[(A, 1.0), (B, 5.0)]);
        let r = align(&audio, &[A, B], &bank).unwrap();
        assert_eq!(r.segments[0].end, 3);
        assert_eq!(r.segments[1].start, 3);
    }

    /// Every phoneme owns at least one frame, no matter how
    /// short the source.
    #[test]
    fn each_phoneme_owns_at_least_one_frame() {
        use Phoneme::*;
        let audio = seq_of(&[1.0, 2.0, 3.0]);
        let bank = bank_for(&[(A, 1.0), (B, 2.0), (T, 3.0), (E, 9.0)]);
        // 4 phonemes, 3 frames → must fail with PhonemesExceedFrames.
        let err = align(&audio, &[A, B, T, E], &bank).unwrap_err();
        assert!(matches!(err, AlignError::PhonemesExceedFrames { .. }));

        // 3 phonemes, 3 frames → each must own ≥1.
        let audio2 = seq_of(&[1.0, 2.0, 3.0]);
        let r = align(&audio2, &[A, B, T], &bank).unwrap();
        for s in &r.segments {
            assert!(s.end > s.start, "empty segment: {s:?}");
        }
    }

    /// Uncovered phoneme reports the offending one.
    #[test]
    fn uncovered_phoneme_reports_which() {
        use Phoneme::*;
        let audio = seq_of(&[1.0, 2.0]);
        let bank = bank_for(&[(A, 1.0)]); // B missing
        let err = align(&audio, &[A, B], &bank).unwrap_err();
        assert_eq!(err, AlignError::UncoveredPhoneme(B));
    }

    /// Empty inputs report cleanly.
    #[test]
    fn empty_inputs_rejected() {
        use Phoneme::*;
        let audio = seq_of(&[1.0]);
        let bank = bank_for(&[(A, 1.0)]);
        assert_eq!(
            align(&audio, &[], &bank).unwrap_err(),
            AlignError::EmptyPhonemes
        );
        let empty_audio = MfccSequence {
            frames: vec![],
            sample_rate: 16_000,
            hop_length: 160,
            n_mfcc: 13,
        };
        assert_eq!(
            align(&empty_audio, &[A], &bank).unwrap_err(),
            AlignError::EmptyAudio
        );
    }
}
