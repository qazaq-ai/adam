// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Syllabification of a phoneme stream.
//!
//! Kazakh syllables fit the canonical `(C)V(C)(C)` shape: at most
//! one consonant in the onset (no initial clusters), one vowel as
//! nucleus, up to two consonants in the coda. This module
//! produces a deterministic split via the **Maximum Onset
//! Principle (MOP)** adapted to Kazakh: when consonants stand
//! between two vowels, **the last one goes to the next
//! syllable's onset** and the rest accrue as the previous
//! syllable's coda.
//!
//! The split is purely structural — it makes no judgment about
//! whether a particular cluster is "permitted" in Kazakh. Cluster
//! permission lives in the validator and is intentionally
//! permissive on the first pass.

use adam_phoneme::Phoneme;
use std::fmt;

/// A single syllable: optional onset consonants, a vowel
/// nucleus, optional coda consonants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Syllable {
    pub onset: Vec<Phoneme>,
    pub nucleus: Phoneme,
    pub coda: Vec<Phoneme>,
}

impl Syllable {
    /// Concatenate `onset ++ [nucleus] ++ coda` into a phoneme
    /// vector.
    pub fn to_phonemes(&self) -> Vec<Phoneme> {
        let mut out = Vec::with_capacity(self.onset.len() + 1 + self.coda.len());
        out.extend(&self.onset);
        out.push(self.nucleus);
        out.extend(&self.coda);
        out
    }

    /// IPA shape: `/CCV.CC/` style for compact display.
    pub fn ipa(&self) -> String {
        let mut s = String::new();
        for c in &self.onset {
            s.push_str(c.ipa());
        }
        s.push_str(self.nucleus.ipa());
        for c in &self.coda {
            s.push_str(c.ipa());
        }
        s
    }
}

impl fmt::Display for Syllable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.ipa())
    }
}

/// Return the indices in `stream` that should serve as
/// syllable nuclei, following the sonority-tier fallback
/// described on [`syllabify`].
pub fn nucleus_indices(stream: &[Phoneme]) -> Vec<usize> {
    let vowels: Vec<usize> = stream
        .iter()
        .enumerate()
        .filter(|(_, p)| p.is_vowel())
        .map(|(i, _)| i)
        .collect();
    if !vowels.is_empty() {
        return vowels;
    }
    let sonorants: Vec<usize> = stream
        .iter()
        .enumerate()
        .filter(|(_, p)| is_sonorant(**p))
        .map(|(i, _)| i)
        .collect();
    if !sonorants.is_empty() {
        return sonorants;
    }
    if stream.is_empty() {
        Vec::new()
    } else {
        vec![stream.len() - 1]
    }
}

/// A sonorant phoneme — one that can serve as a syllable
/// nucleus in the absence of a vowel. Covers nasals, liquids,
/// trills, glides, and approximants.
fn is_sonorant(p: Phoneme) -> bool {
    use adam_phoneme::Manner::*;
    matches!(
        p.manner(),
        Some(Nasal | Lateral | Trill | Glide | Approximant)
    )
}

/// Split a phoneme stream into syllables.
///
/// [`Phoneme::Glottal`] markers are transparent — the
/// syllabifier strips them before splitting (compound-word
/// boundaries are noted but do not force a syllable boundary at
/// this layer).
///
/// Nucleus selection follows a sonority-tier fallback (added
/// 2026-05-29 to accept the strict-orthographic rule's output —
/// «бір» = [B, R], «қызыл» = [Q, Z, L], «біз» = [B, Z]):
///
///   1. If the stream contains **vowels**, every vowel is a
///      nucleus (the historical rule).
///   2. Otherwise, if it contains **sonorants** (nasals, liquids,
///      trills, glides, approximants), every sonorant is a
///      nucleus — Kazakh tolerates syllabic /m̩/ /n̩/ /l̩/ /r̩/
///      the way English does in «button», «rhythm», «battle».
///   3. Otherwise, the **last consonant** of the stream is the
///      single nucleus — handles obstruent-only clusters like
///      /qz/ («қыз») or /bz/ («біз»).
///
/// Empty streams return an empty vector.
pub fn syllabify(phonemes: &[Phoneme]) -> Vec<Syllable> {
    let stream: Vec<Phoneme> = phonemes
        .iter()
        .copied()
        .filter(|p| !matches!(p, Phoneme::Glottal))
        .collect();

    let vowel_indices = nucleus_indices(&stream);

    if vowel_indices.is_empty() {
        return Vec::new();
    }

    let mut syllables: Vec<Syllable> = Vec::with_capacity(vowel_indices.len());

    for (i_nuc, &v_pos) in vowel_indices.iter().enumerate() {
        let prev_v_pos = i_nuc
            .checked_sub(1)
            .and_then(|i| vowel_indices.get(i).copied());

        // Region of consonants between prev vowel (exclusive) and
        // this vowel (exclusive).
        let cs_before_start = prev_v_pos.map(|p| p + 1).unwrap_or(0);
        let cs_before_end = v_pos;
        let n_cs = cs_before_end - cs_before_start;

        // Determine where the onset of THIS syllable begins:
        //   - First syllable: all preceding consonants are onset.
        //   - Otherwise (MOP): last consonant is onset; rest go
        //     to the PREVIOUS syllable's coda.
        let onset_start = if prev_v_pos.is_none() {
            cs_before_start
        } else if n_cs == 0 {
            v_pos
        } else {
            v_pos - 1
        };
        let prev_coda_extension: &[Phoneme] = &stream[cs_before_start..onset_start];
        let onset: Vec<Phoneme> = stream[onset_start..v_pos].to_vec();

        // Extend the previous syllable's coda before pushing
        // this one.
        if let Some(prev) = syllables.last_mut() {
            prev.coda.extend_from_slice(prev_coda_extension);
        }

        // This syllable's coda: empty until its consonants get
        // distributed by the next iteration. For the LAST
        // syllable specifically, the remaining tail belongs to
        // the coda.
        let is_last = i_nuc + 1 == vowel_indices.len();
        let coda = if is_last {
            stream[v_pos + 1..].to_vec()
        } else {
            Vec::new()
        };

        syllables.push(Syllable {
            onset,
            nucleus: stream[v_pos],
            coda,
        });
    }

    syllables
}

#[cfg(test)]
mod tests {
    use super::*;
    use Phoneme::*;

    fn syl(onset: &[Phoneme], nucleus: Phoneme, coda: &[Phoneme]) -> Syllable {
        Syllable {
            onset: onset.to_vec(),
            nucleus,
            coda: coda.to_vec(),
        }
    }

    /// «қазақ» = Q A Z A Q → «қа.зақ»
    #[test]
    fn qazaq_two_syllables() {
        let s = syllabify(&[Q, A, Z, A, Q]);
        assert_eq!(s, vec![syl(&[Q], A, &[]), syl(&[Z], A, &[Q])]);
    }

    /// «алты» = A L T Y → «ал.ты»
    #[test]
    fn alty_keeps_l_as_first_coda() {
        let s = syllabify(&[A, L, T, Y]);
        assert_eq!(s, vec![syl(&[], A, &[L]), syl(&[T], Y, &[])]);
    }

    /// «жұмыс» (post-epenthetic) = Zh U M S → «жұмс» as one
    /// syllable (single vowel) with onset Zh, nucleus U,
    /// coda M S.
    #[test]
    fn jumys_single_syllable_after_dropping() {
        let s = syllabify(&[Zh, U, M, S]);
        assert_eq!(s, vec![syl(&[Zh], U, &[M, S])]);
    }

    /// «мемлекет» = M E M L E K E T → «мем.ле.кет»
    #[test]
    fn memleket_three_syllables() {
        let s = syllabify(&[M, E, M, L, E, K, E, T]);
        assert_eq!(
            s,
            vec![syl(&[M], E, &[M]), syl(&[L], E, &[]), syl(&[K], E, &[T]),]
        );
    }

    /// «байтұрсынұлы» post-epenthetic = B A J T U R S N U L Y →
    /// expected: «бай.тұрс.ну.лы». Verifies MOP distributes a
    /// triple-consonant cluster R-S-N as R,S → coda, N → onset.
    #[test]
    fn baitursunuly_triple_consonant_distribution() {
        let s = syllabify(&[B, A, J, T, U, R, S, N, U, L, Y]);
        assert_eq!(
            s,
            vec![
                syl(&[B], A, &[J]),
                syl(&[T], U, &[R, S]),
                syl(&[N], U, &[]),
                syl(&[L], Y, &[]),
            ]
        );
    }

    /// Vowel hiatus (no consonant between): «ауа» (air)
    /// with «у» = W (consonant by current Layer 0d defaults)
    /// becomes A W A — testable. But if we feed pure hiatus
    /// A A: → «а.а» (two syllables, both with no onset, no
    /// coda).
    #[test]
    fn vowel_hiatus_splits() {
        let s = syllabify(&[A, A]);
        assert_eq!(s, vec![syl(&[], A, &[]), syl(&[], A, &[])]);
    }

    /// «ауа» = A W A → «а.уа» (W is consonant; max onset gives
    /// it to second syllable).
    #[test]
    fn aua_with_w_glide() {
        let s = syllabify(&[A, W, A]);
        assert_eq!(s, vec![syl(&[], A, &[]), syl(&[W], A, &[])]);
    }

    /// Empty input syllabifies to nothing. Consonant-only input
    /// uses the sonority-tier fallback (sonorant → last consonant)
    /// to find a nucleus, per the 2026-05-29 strict-orthographic
    /// rule support.
    #[test]
    fn empty_input_no_syllables_consonant_only_picks_sonorant() {
        // Empty → empty.
        assert!(syllabify(&[]).is_empty());
        // [Q, R, S]: R is a sonorant (trill) → it's the nucleus.
        let qrs = syllabify(&[Q, R, S]);
        assert_eq!(qrs.len(), 1);
        assert_eq!(qrs[0].nucleus, R);
        // [Q, Z]: no vowels, no sonorants → fallback to last
        // consonant (Z) as nucleus.
        let qz = syllabify(&[Q, Z]);
        assert_eq!(qz.len(), 1);
        assert_eq!(qz[0].nucleus, Z);
    }

    /// Glottal markers stripped before syllabification (do NOT
    /// force syllable boundaries at this layer).
    #[test]
    fn glottal_marker_transparent() {
        let with_g = syllabify(&[Q, A, Glottal, Z, A, Q]);
        let without_g = syllabify(&[Q, A, Z, A, Q]);
        assert_eq!(with_g, without_g);
    }

    /// to_phonemes recovers the full segment list of a syllable.
    #[test]
    fn syllable_to_phonemes_recovers_segments() {
        let s = syl(&[T], U, &[R, S]);
        assert_eq!(s.to_phonemes(), vec![T, U, R, S]);
    }

    /// IPA rendering of a syllable concatenates onset + nucleus
    /// + coda.
    #[test]
    fn syllable_ipa_render() {
        let s = syl(&[Q], A, &[Z]);
        assert_eq!(s.ipa(), "qaz");
    }
}
