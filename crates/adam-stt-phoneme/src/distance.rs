// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Frame-level distance metrics for MFCC vectors.
//!
//! DTW operates over a 2-D cost grid; each cell is the
//! distance between one query frame and one template frame.
//! For MFCC vectors:
//!
//! - **Euclidean** is the standard ASR default.
//! - **Cosine** is more robust to amplitude differences (e.g.
//!   speaker-to-speaker volume variation) — preferred when
//!   the template bank averages across speakers.
//!
//! Both functions are defined for vectors of any equal length;
//! `debug_assert` checks dimensions.

/// Euclidean distance between two same-length f32 vectors:
/// `sqrt(Σ (a[i] - b[i])²)`.
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len(), "euclidean: dimension mismatch");
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let d = x - y;
            d * d
        })
        .sum::<f32>()
        .sqrt()
}

/// Cosine distance: `1 - (a · b) / (|a| * |b|)`. Ranges in
/// `[0, 2]`; identical vectors → 0, orthogonal → 1, opposite
/// → 2.
///
/// Returns `1.0` (max neutral) when either vector has near-
/// zero magnitude — avoids divide-by-zero on silence frames.
pub fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len(), "cosine: dimension mismatch");
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na < 1e-9 || nb < 1e-9 {
        return 1.0;
    }
    1.0 - dot / (na * nb)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn euclidean_identical_is_zero() {
        let a = vec![1.0, 2.0, 3.0];
        assert_eq!(euclidean_distance(&a, &a), 0.0);
    }

    #[test]
    fn euclidean_orthogonal_unit_vectors() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        // |a - b| = sqrt(2).
        assert!((euclidean_distance(&a, &b) - 2.0_f32.sqrt()).abs() < 1e-6);
    }

    #[test]
    fn cosine_identical_is_zero() {
        let a = vec![1.0, 2.0, 3.0];
        assert!(cosine_distance(&a, &a).abs() < 1e-6);
    }

    #[test]
    fn cosine_orthogonal_is_one() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!((cosine_distance(&a, &b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_opposite_is_two() {
        let a = vec![1.0_f32, 2.0];
        let b = vec![-1.0_f32, -2.0];
        assert!((cosine_distance(&a, &b) - 2.0).abs() < 1e-6);
    }

    /// Cosine is invariant to magnitude — same direction,
    /// different scales → distance 0.
    #[test]
    fn cosine_invariant_to_scale() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![10.0, 20.0, 30.0];
        assert!(cosine_distance(&a, &b) < 1e-5);
    }

    /// Euclidean is NOT invariant to scale — same direction,
    /// different scales → large distance.
    #[test]
    fn euclidean_sensitive_to_scale() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![10.0, 20.0, 30.0];
        assert!(euclidean_distance(&a, &b) > 5.0);
    }

    /// Both distances handle zero vectors safely.
    #[test]
    fn zero_vector_handling() {
        let z = vec![0.0_f32; 13];
        let a = vec![1.0_f32; 13];
        assert!(euclidean_distance(&z, &a).is_finite());
        // Cosine returns the neutral 1.0 for zero-magnitude.
        assert_eq!(cosine_distance(&z, &a), 1.0);
    }
}
