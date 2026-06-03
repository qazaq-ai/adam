// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Dynamic Time Warping (DTW) for MFCC sequence alignment.
//!
//! DTW finds the best non-linear alignment between two
//! sequences of variable length. For phoneme matching: a
//! query MFCC sequence (e.g. ~10–30 frames for one
//! pronounced phoneme) is aligned against each phoneme
//! template; the template with the minimum normalised
//! alignment cost wins.
//!
//! ## Algorithm
//!
//! Given a query Q of length M and template T of length N:
//!
//! 1. Build the M × N **local-distance matrix** D where
//!    `D[i][j] = dist(Q[i], T[j])`.
//! 2. Build the M × N **cumulative cost matrix** dp where
//!    `dp[0][0] = D[0][0]` and
//!    `dp[i][j] = D[i][j] + min(dp[i-1][j], dp[i][j-1], dp[i-1][j-1])`.
//!    The min over three predecessors captures the
//!    "step / stretch / compress" move set.
//! 3. **DTW distance** = `dp[M-1][N-1]`. Normalised by
//!    `(M + N)` so longer sequences don't accumulate more
//!    cost.
//! 4. **Path recovery** (optional): backtrack from
//!    `dp[M-1][N-1]` to `dp[0][0]` following the min-cost
//!    predecessor at each step.
//!
//! Time and space complexity: O(M·N). For typical phoneme-
//! match windows (M ≈ 20, N ≈ 30) this is trivial.

use crate::distance::euclidean_distance;

/// Result of a DTW alignment.
#[derive(Debug, Clone, PartialEq)]
pub struct DtwResult {
    /// Total alignment cost normalised by `(query_len + template_len)`.
    /// Lower is better; identical sequences score 0.
    pub cost: f32,
    /// The alignment path: a vector of `(query_index,
    /// template_index)` pairs from start (0,0) to end
    /// (M-1, N-1).
    pub path: Vec<(usize, usize)>,
}

/// Compute the DTW alignment between two MFCC sequences with
/// the default Euclidean distance.
///
/// Returns `None` if either sequence is empty or if their
/// frame dimensions don't match.
pub fn dtw(query: &[Vec<f32>], template: &[Vec<f32>]) -> Option<DtwResult> {
    dtw_with_distance(query, template, euclidean_distance)
}

/// Like [`dtw`] but with a caller-supplied distance function.
///
/// Use this with `cosine_distance` when speaker amplitude
/// variation makes Euclidean unstable.
pub fn dtw_with_distance(
    query: &[Vec<f32>],
    template: &[Vec<f32>],
    dist: fn(&[f32], &[f32]) -> f32,
) -> Option<DtwResult> {
    if query.is_empty() || template.is_empty() {
        return None;
    }
    if query[0].len() != template[0].len() {
        return None;
    }

    let m = query.len();
    let n = template.len();
    let mut dp = vec![vec![f32::INFINITY; n]; m];

    // Init.
    dp[0][0] = dist(&query[0], &template[0]);

    // First row: only horizontal moves allowed.
    for j in 1..n {
        dp[0][j] = dp[0][j - 1] + dist(&query[0], &template[j]);
    }
    // First column: only vertical moves allowed.
    for i in 1..m {
        dp[i][0] = dp[i - 1][0] + dist(&query[i], &template[0]);
    }
    // Inner cells: min of three predecessors.
    for i in 1..m {
        for j in 1..n {
            let d = dist(&query[i], &template[j]);
            let p = dp[i - 1][j].min(dp[i][j - 1]).min(dp[i - 1][j - 1]);
            dp[i][j] = d + p;
        }
    }

    let total = dp[m - 1][n - 1];
    let cost = total / (m + n) as f32;

    // Backtrack to recover the path.
    let mut path: Vec<(usize, usize)> = Vec::new();
    let mut i = m - 1;
    let mut j = n - 1;
    path.push((i, j));
    while i > 0 || j > 0 {
        let cand = match (i, j) {
            (0, _) => (0, j - 1),
            (_, 0) => (i - 1, 0),
            _ => {
                let diag = dp[i - 1][j - 1];
                let up = dp[i - 1][j];
                let left = dp[i][j - 1];
                if diag <= up && diag <= left {
                    (i - 1, j - 1)
                } else if up <= left {
                    (i - 1, j)
                } else {
                    (i, j - 1)
                }
            }
        };
        i = cand.0;
        j = cand.1;
        path.push((i, j));
    }
    path.reverse();

    Some(DtwResult { cost, path })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Identical sequences → zero cost, diagonal path.
    #[test]
    fn identical_sequences_zero_cost() {
        let seq: Vec<Vec<f32>> = (0..10).map(|i| vec![i as f32, 0.0, 0.0]).collect();
        let r = dtw(&seq, &seq).unwrap();
        assert!(r.cost < 1e-6, "cost {}", r.cost);
        // Path is the diagonal: (0,0), (1,1), …, (9,9).
        assert_eq!(r.path.len(), 10);
        for (k, &(qi, ti)) in r.path.iter().enumerate() {
            assert_eq!(qi, k);
            assert_eq!(ti, k);
        }
    }

    /// **Time-stretched copy** (insert duplicates) still
    /// aligns with low cost — the whole point of DTW.
    #[test]
    fn stretched_sequence_low_cost() {
        let base: Vec<Vec<f32>> = (0..5).map(|i| vec![i as f32, 0.0]).collect();
        // Stretch: duplicate every frame.
        let stretched: Vec<Vec<f32>> = base
            .iter()
            .flat_map(|f| std::iter::repeat_n(f.clone(), 2))
            .collect();
        let r = dtw(&base, &stretched).unwrap();
        assert!(r.cost < 1e-6, "stretched cost {}", r.cost);
    }

    /// **Time-compressed copy** (drop every other frame)
    /// still aligns with low cost.
    #[test]
    fn compressed_sequence_low_cost() {
        let base: Vec<Vec<f32>> = (0..10).map(|i| vec![i as f32, 0.0]).collect();
        let compressed: Vec<Vec<f32>> = base.iter().step_by(2).cloned().collect();
        let r = dtw(&base, &compressed).unwrap();
        // Some cost because intermediate frames don't have an
        // exact match, but small.
        assert!(r.cost < 0.5, "compressed cost too high: {}", r.cost);
    }

    /// **Different sequences** → high cost.
    #[test]
    fn different_sequences_high_cost() {
        let a: Vec<Vec<f32>> = (0..10).map(|i| vec![i as f32, 0.0]).collect();
        let b: Vec<Vec<f32>> = (0..10).map(|i| vec![-(i as f32), 100.0]).collect();
        let r = dtw(&a, &b).unwrap();
        assert!(r.cost > 5.0, "different-seq cost too low: {}", r.cost);
    }

    /// Empty / dimension-mismatched sequences → None.
    #[test]
    fn empty_or_mismatched_returns_none() {
        let a: Vec<Vec<f32>> = vec![vec![1.0]];
        let empty: Vec<Vec<f32>> = vec![];
        let mismatched: Vec<Vec<f32>> = vec![vec![1.0, 2.0]];
        assert!(dtw(&a, &empty).is_none());
        assert!(dtw(&empty, &a).is_none());
        assert!(dtw(&a, &mismatched).is_none());
    }

    /// Asymmetry sanity: dtw(a, b) and dtw(b, a) have equal
    /// cost (DTW is a metric on the alignment level).
    #[test]
    fn dtw_is_symmetric() {
        let a: Vec<Vec<f32>> = (0..8).map(|i| vec![i as f32, 1.0, 2.0]).collect();
        let b: Vec<Vec<f32>> = (0..12).map(|i| vec![(i as f32) * 0.7, 1.5, 1.5]).collect();
        let ab = dtw(&a, &b).unwrap().cost;
        let ba = dtw(&b, &a).unwrap().cost;
        assert!((ab - ba).abs() < 1e-4, "{} vs {}", ab, ba);
    }

    /// The recovered path always starts at (0,0), ends at
    /// (M-1, N-1), and only moves up / right / diagonally.
    #[test]
    fn path_invariants() {
        let a: Vec<Vec<f32>> = (0..7).map(|i| vec![i as f32]).collect();
        let b: Vec<Vec<f32>> = (0..11).map(|i| vec![(i as f32) * 1.3]).collect();
        let r = dtw(&a, &b).unwrap();
        assert_eq!(r.path[0], (0, 0));
        assert_eq!(r.path[r.path.len() - 1], (6, 10));
        for w in r.path.windows(2) {
            let (i0, j0) = w[0];
            let (i1, j1) = w[1];
            // Each step advances at least one index, by at
            // most 1.
            assert!(i1 >= i0 && j1 >= j0);
            assert!((i1 - i0) + (j1 - j0) >= 1);
            assert!(i1 - i0 <= 1 && j1 - j0 <= 1);
        }
    }

    /// Custom distance: cosine version also works.
    #[test]
    fn cosine_distance_path_works() {
        use crate::distance::cosine_distance;
        let a: Vec<Vec<f32>> = (0..5).map(|i| vec![1.0, i as f32]).collect();
        let b: Vec<Vec<f32>> = (0..5).map(|i| vec![10.0, (i as f32) * 10.0]).collect();
        // a and b are scaled copies of each other; cosine
        // distance should give ~0 between corresponding pairs.
        let r = dtw_with_distance(&a, &b, cosine_distance).unwrap();
        assert!(r.cost < 0.05, "cosine DTW cost {}", r.cost);
    }
}
