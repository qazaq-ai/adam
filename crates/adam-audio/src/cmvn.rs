// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Cepstral Mean and Variance Normalisation (CMVN) — **Phase 11**
//! cornerstone for speaker-invariant features.
//!
//! ## Why
//!
//! After Phase 10's forced aligner landed, the residual recognition
//! failure on the Wikimedia «қазақ» probe was diagnosed as speaker
//! mismatch: the FLEURS-trained per-phoneme templates are means over
//! 2773 F + 2653 M voices, so the probe's Q frames land closer to
//! the bank's *Z* centroid than to its *Q* centroid in raw cepstral
//! space. CMVN is the cheapest and most established fix — it
//! removes the speaker-dependent and channel-dependent linear
//! offset / scale from MFCC vectors before any downstream
//! comparison.
//!
//! ## What
//!
//! For each MFCC sequence (one utterance):
//!
//! ```text
//!   μ[c] = mean over frames of MFCC[t][c]
//!   σ[c] = std  over frames of MFCC[t][c]    (with ε floor)
//!   normalised[t][c] = (MFCC[t][c] − μ[c]) / σ[c]
//! ```
//!
//! Per-utterance CMVN treats every recording as its own "speaker
//! channel", which is the standard practice when speaker labels
//! aren't tracked at recognition time. The transform is invertible
//! and deterministic. Variance-floor `ε = 1e-6` prevents divide-by-
//! zero for degenerate (constant) coefficients.
//!
//! Apply CMVN identically to:
//!
//! - Template extraction (training side): each source MFCC is
//!   normalised *before* forced alignment, so the bank stores
//!   speaker-invariant centroids.
//! - Recognition (inference side): the query MFCC is normalised
//!   with its own per-utterance statistics before DTW / Viterbi.

use crate::mfcc::MfccSequence;

/// Variance-floor — prevents divide-by-zero for degenerate
/// (near-constant) coefficients. 1e-6 is the standard floor used
/// in Kaldi / HTK CMVN implementations.
const VAR_FLOOR: f32 = 1e-6;

/// Apply per-utterance CMVN in-place: subtract the per-coefficient
/// mean and divide by the per-coefficient standard deviation. A
/// 0-frame or 1-frame sequence is left untouched (no statistics to
/// compute).
pub fn normalise_in_place(seq: &mut MfccSequence) {
    let n_frames = seq.num_frames();
    let dim = seq.dim();
    if n_frames < 2 || dim == 0 {
        return;
    }

    // Mean per coefficient.
    let mut mean = vec![0.0_f32; dim];
    for frame in &seq.frames {
        for (m, x) in mean.iter_mut().zip(frame.iter()) {
            *m += *x;
        }
    }
    let inv_n = 1.0 / n_frames as f32;
    for m in &mut mean {
        *m *= inv_n;
    }

    // Variance per coefficient (population, not Bessel-corrected).
    let mut var = vec![0.0_f32; dim];
    for frame in &seq.frames {
        for (v, (x, m)) in var.iter_mut().zip(frame.iter().zip(mean.iter())) {
            let d = *x - *m;
            *v += d * d;
        }
    }
    for v in &mut var {
        *v = (*v * inv_n).max(VAR_FLOOR);
    }

    // Apply transform.
    let std: Vec<f32> = var.iter().map(|v| v.sqrt()).collect();
    for frame in &mut seq.frames {
        for (x, (m, s)) in frame.iter_mut().zip(mean.iter().zip(std.iter())) {
            *x = (*x - *m) / *s;
        }
    }
}

/// Allocating wrapper around [`normalise_in_place`].
pub fn normalise(seq: &MfccSequence) -> MfccSequence {
    let mut out = MfccSequence {
        frames: seq.frames.clone(),
        sample_rate: seq.sample_rate,
        hop_length: seq.hop_length,
        n_mfcc: seq.n_mfcc,
    };
    normalise_in_place(&mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mfcc::MfccSequence;

    fn seq(frames: Vec<Vec<f32>>) -> MfccSequence {
        let n_mfcc = frames.first().map(|f| f.len()).unwrap_or(0);
        MfccSequence {
            frames,
            sample_rate: 16_000,
            hop_length: 160,
            n_mfcc,
        }
    }

    /// After CMVN, every coefficient should have mean ≈ 0 and
    /// unit variance across frames.
    #[test]
    fn normalised_has_zero_mean_unit_variance() {
        let s = seq(vec![
            vec![1.0, 5.0, -3.0],
            vec![2.0, 6.0, -2.0],
            vec![3.0, 7.0, -4.0],
            vec![4.0, 8.0, -1.0],
            vec![5.0, 9.0, -5.0],
        ]);
        let n = normalise(&s);

        // Per-coefficient mean across frames ≈ 0.
        for c in 0..3 {
            let mean: f32 = n.frames.iter().map(|f| f[c]).sum::<f32>() / n.frames.len() as f32;
            assert!(mean.abs() < 1e-5, "coef {c} mean = {mean}");
        }

        // Per-coefficient variance across frames ≈ 1.
        for c in 0..3 {
            let mean: f32 = n.frames.iter().map(|f| f[c]).sum::<f32>() / n.frames.len() as f32;
            let var: f32 =
                n.frames.iter().map(|f| (f[c] - mean).powi(2)).sum::<f32>() / n.frames.len() as f32;
            assert!((var - 1.0).abs() < 1e-4, "coef {c} var = {var}");
        }
    }

    /// CMVN must remove a constant additive offset — the same
    /// audio shifted by a speaker-dependent channel bias should
    /// produce identical normalised features.
    #[test]
    fn channel_offset_is_invariant() {
        let base = seq(vec![
            vec![1.0, 5.0],
            vec![2.0, 6.0],
            vec![3.0, 7.0],
            vec![4.0, 8.0],
        ]);
        let shifted = seq(vec![
            vec![1.0 + 100.0, 5.0 - 50.0],
            vec![2.0 + 100.0, 6.0 - 50.0],
            vec![3.0 + 100.0, 7.0 - 50.0],
            vec![4.0 + 100.0, 8.0 - 50.0],
        ]);
        let n_base = normalise(&base);
        let n_shift = normalise(&shifted);
        for (a, b) in n_base.frames.iter().zip(n_shift.frames.iter()) {
            for (x, y) in a.iter().zip(b.iter()) {
                assert!(
                    (x - y).abs() < 1e-5,
                    "CMVN failed to cancel offset: {x} vs {y}"
                );
            }
        }
    }

    /// CMVN must remove a multiplicative scaling — the same
    /// audio scaled per-coefficient should also be invariant.
    #[test]
    fn channel_scale_is_invariant() {
        let base = seq(vec![
            vec![1.0, 5.0],
            vec![2.0, 6.0],
            vec![3.0, 7.0],
            vec![4.0, 8.0],
        ]);
        let scaled = seq(vec![
            vec![1.0 * 3.0, 5.0 * 0.5],
            vec![2.0 * 3.0, 6.0 * 0.5],
            vec![3.0 * 3.0, 7.0 * 0.5],
            vec![4.0 * 3.0, 8.0 * 0.5],
        ]);
        let n_base = normalise(&base);
        let n_scaled = normalise(&scaled);
        for (a, b) in n_base.frames.iter().zip(n_scaled.frames.iter()) {
            for (x, y) in a.iter().zip(b.iter()) {
                assert!(
                    (x - y).abs() < 1e-4,
                    "CMVN failed to cancel scale: {x} vs {y}"
                );
            }
        }
    }

    /// 0-frame and 1-frame sequences pass through untouched
    /// (no statistics to compute).
    #[test]
    fn degenerate_lengths_are_passthrough() {
        let empty = seq(vec![]);
        let one = seq(vec![vec![1.0, 2.0, 3.0]]);
        let n_empty = normalise(&empty);
        let n_one = normalise(&one);
        assert!(n_empty.frames.is_empty());
        assert_eq!(n_one.frames, vec![vec![1.0, 2.0, 3.0]]);
    }

    /// Constant-valued coefficient (zero variance) doesn't
    /// explode — the variance floor catches it and yields a
    /// finite normalised value (close to 0 because mean ≈ value).
    #[test]
    fn constant_coefficient_does_not_explode() {
        let s = seq(vec![vec![5.0, 1.0], vec![5.0, 2.0], vec![5.0, 3.0]]);
        let n = normalise(&s);
        for frame in &n.frames {
            for x in frame {
                assert!(x.is_finite(), "non-finite output: {x}");
            }
        }
    }
}
