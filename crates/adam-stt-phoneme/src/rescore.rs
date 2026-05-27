// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Phonotactic rescoring of recognised phoneme streams — the
//! **Layer C** of v6.3 STT.
//!
//! [`crate::recognise_word`] outputs a phoneme stream based on
//! per-window DTW only. With the small bootstrap bank this
//! produces sequences that violate Kazakh phonotactics — too
//! many adjacent consonants, broken vowel harmony, etc.
//! Layer C **rescores** the output against the phonotactic
//! constraints from [`adam_phonotactics`]:
//!
//! - **Syllable shape**: enforce `(C)V(CC(C))` — at most one
//!   onset consonant before each vowel, at most three after.
//! - **Harmony** check (informational — vowels in a native
//!   word should share a harmony class; mixed → loan or
//!   recognition error).
//!
//! ## Current pass: [`enforce_syllable_shape`]
//!
//! Walks the stream, finds vowel positions, drops consonants
//! that exceed the `(C)V(CC(C))` budget for each syllable.
//! For Phase 6 first iteration this is the highest-value
//! filter — it cleans up the noisy [`crate::recognise_word`]
//! output without requiring per-window top-K candidates.
//!
//! ## Future passes (Layer C+)
//!
//! - **Viterbi rescoring**: take per-window top-K candidates
//!   from [`crate::RecognitionResult`] and run a phonotactic-
//!   aware DP that minimises (DTW cost + phonotactic penalty).
//! - **Harmony fixing**: swap a vowel for its top-2 candidate
//!   if doing so restores pure harmony.
//! - **Voicing assimilation**: at morpheme boundaries the
//!   suffix consonants change voicing — useful when wiring to
//!   the morphological FST.

use adam_phoneme::Phoneme;
use adam_phonotactics::validate::{MAX_CODA, MAX_ONSET};

/// Enforce the Kazakh syllable shape `(C)V(CC(C))` over a
/// recognised phoneme stream.
///
/// Rules:
/// - At most [`MAX_ONSET`] consonants before each vowel
///   (extras dropped from the start of each consonant run).
/// - At most [`MAX_CODA`] consonants after the last vowel
///   (extras truncated at the end).
/// - Between two vowels, the consonant run is split: the
///   first up-to-[`MAX_CODA`] go to the previous syllable's
///   coda; the last consonant (if room) becomes the next
///   syllable's onset.
///
/// Returns an empty vector if the input has no vowel — a
/// well-formed Kazakh word must have at least one.
pub fn enforce_syllable_shape(stream: &[Phoneme]) -> Vec<Phoneme> {
    let vowels: Vec<usize> = stream
        .iter()
        .enumerate()
        .filter(|(_, p)| p.is_vowel())
        .map(|(i, _)| i)
        .collect();
    if vowels.is_empty() {
        return Vec::new();
    }

    let mut out: Vec<Phoneme> = Vec::new();

    // Initial onset: at most MAX_ONSET consonants before the
    // first vowel. Keep the LAST MAX_ONSET (closest to the
    // vowel), drop the rest.
    let first_v = vowels[0];
    if first_v > 0 {
        let onset_start = first_v.saturating_sub(MAX_ONSET);
        out.extend_from_slice(&stream[onset_start..first_v]);
    }

    for (i_v, &v_pos) in vowels.iter().enumerate() {
        out.push(stream[v_pos]);

        let cons_start = v_pos + 1;
        let cons_end = vowels.get(i_v + 1).copied().unwrap_or(stream.len());
        let cons: Vec<Phoneme> = stream[cons_start..cons_end].to_vec();

        if i_v + 1 == vowels.len() {
            // Last vowel: tail consonants are coda only;
            // truncate at MAX_CODA.
            for p in cons.iter().take(MAX_CODA) {
                out.push(*p);
            }
        } else {
            // Mid vowel: split between this syllable's coda
            // (up to MAX_CODA) and the next syllable's onset
            // (up to MAX_ONSET).
            let total = cons.len();
            if total <= MAX_CODA + MAX_ONSET {
                out.extend_from_slice(&cons);
            } else {
                // Keep first MAX_CODA as coda…
                for p in cons.iter().take(MAX_CODA) {
                    out.push(*p);
                }
                // …and the last MAX_ONSET as next-syllable onset.
                let onset_start_in_cons = total - MAX_ONSET;
                for p in &cons[onset_start_in_cons..] {
                    out.push(*p);
                }
            }
        }
    }

    out
}

/// Drop adjacent duplicate phonemes (run-length deduplication).
/// Useful when the recogniser's RLE-smoothed output still
/// produces e.g. `[T, T]` from two separate windows that both
/// landed on T.
pub fn dedupe_adjacent(stream: &[Phoneme]) -> Vec<Phoneme> {
    let mut out: Vec<Phoneme> = Vec::with_capacity(stream.len());
    for &p in stream {
        if out.last().copied() != Some(p) {
            out.push(p);
        }
    }
    out
}

/// Apply the full Layer C rescoring pipeline: dedupe adjacent
/// duplicates, then enforce the syllable shape.
pub fn rescore(stream: &[Phoneme]) -> Vec<Phoneme> {
    enforce_syllable_shape(&dedupe_adjacent(stream))
}

#[cfg(test)]
mod tests {
    use super::*;
    use Phoneme::*;

    /// `[T, T, Q, T, A, Q, T]` — the qazaq misrecognition from
    /// Phase 2d++. Should rescore to `[T, A, Q, T]` (one onset
    /// T, vowel A, coda Q+T).
    #[test]
    fn qazaq_misrecognition_cleaned() {
        let messy = vec![T, T, Q, T, A, Q, T];
        let cleaned = rescore(&messy);
        // Initial T-T-Q-T → 1 onset (last T). Q-T stays as coda.
        assert_eq!(cleaned, vec![T, A, Q, T]);
    }

    /// `[Zh, Yi, B, E, K]` — the жібек correct output. Should
    /// pass through unchanged.
    #[test]
    fn jibek_unchanged() {
        let stream = vec![Zh, Yi, B, E, K];
        assert_eq!(rescore(&stream), stream);
    }

    /// Empty stream → empty.
    #[test]
    fn empty_input_empty_output() {
        let stream: Vec<Phoneme> = vec![];
        assert!(rescore(&stream).is_empty());
    }

    /// All-consonant stream → empty (no vowel = no valid word).
    #[test]
    fn all_consonant_yields_empty() {
        let stream = vec![T, T, T, Q, S];
        assert!(rescore(&stream).is_empty());
    }

    /// Single vowel → kept.
    #[test]
    fn single_vowel_kept() {
        assert_eq!(rescore(&[A]), vec![A]);
    }

    /// CV → kept.
    #[test]
    fn cv_kept() {
        assert_eq!(rescore(&[Q, A]), vec![Q, A]);
    }

    /// CVC → kept.
    #[test]
    fn cvc_kept() {
        assert_eq!(rescore(&[Q, A, Z]), vec![Q, A, Z]);
    }

    /// Initial CC drops to just the last consonant.
    #[test]
    fn initial_cc_drops_to_one() {
        assert_eq!(rescore(&[T, K, A]), vec![K, A]);
    }

    /// Coda longer than MAX_CODA truncated.
    #[test]
    fn excessive_coda_truncated() {
        // CVCCCCC → CV + first MAX_CODA=3 consonants
        let r = rescore(&[Q, A, R, S, T, N, D]);
        assert_eq!(r, vec![Q, A, R, S, T]);
    }

    /// Adjacent duplicates collapsed before shape enforcement.
    #[test]
    fn duplicates_collapsed_first() {
        // T-T at start → dedupe to T, then onset rule keeps it.
        assert_eq!(dedupe_adjacent(&[T, T, Q]), vec![T, Q]);
    }

    /// Two-syllable word with intervening cluster — clean.
    #[test]
    fn two_syllable_intervening_cluster() {
        // V-CCC-V: 3 mid consonants → max 3 (coda) + 1 (onset)
        // = 4 allowed. 3 ≤ 4 so all kept.
        assert_eq!(rescore(&[A, R, S, T, A]), vec![A, R, S, T, A]);
    }

    /// Two-syllable word with too many mid consonants —
    /// truncated.
    #[test]
    fn two_syllable_excessive_cluster_split() {
        // V-CCCCCC-V: 6 mids → keep first 3 (coda) + last 1
        // (onset) = 4 kept.
        let r = rescore(&[A, R, S, T, N, D, L, A]);
        assert_eq!(r, vec![A, R, S, T, L, A]);
    }

    /// Real-world stream from real-audio test, full rescoring.
    #[test]
    fn rescore_qazaq_full_pipeline() {
        let raw = vec![T, T, Q, T, A, Q, T];
        let after_dedupe = dedupe_adjacent(&raw);
        // T-T collapses to T → [T, Q, T, A, Q, T]
        assert_eq!(after_dedupe, vec![T, Q, T, A, Q, T]);
        let after_shape = enforce_syllable_shape(&after_dedupe);
        // Initial T-Q-T → 1 onset (T). Coda Q-T after A. → [T, A, Q, T]
        assert_eq!(after_shape, vec![T, A, Q, T]);
    }
}
