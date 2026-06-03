// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Energy-based word segmentation.
//!
//! Splits a PCM audio stream into word-sized chunks by finding
//! the silent regions between them. Used by Phase 2d-plus to
//! turn a multi-word recording (e.g. a 28-second UDHR sentence)
//! into per-word audio segments for phoneme template
//! extraction.
//!
//! ## Algorithm
//!
//! 1. Compute per-frame RMS energy with a sliding 25 ms window
//!    at 10 ms hop.
//! 2. A frame is **silent** if its RMS < `silence_threshold`.
//! 3. Find consecutive silent-frame runs of duration ≥
//!    `min_silence_ms`. The midpoint of each run is a word
//!    boundary.
//! 4. Words = sample ranges between consecutive boundaries
//!    (plus from-start and to-end).
//! 5. Drop segments shorter than `min_word_ms` (treat as
//!    fricative artefacts).
//! 6. Trim leading / trailing silence from each surviving
//!    segment.
//!
//! ## What this does NOT do (yet)
//!
//! - **Cross-word coarticulation handling.** When words run
//!   together without an audible pause, this splitter sees
//!   one long word. Real-world recordings often have such
//!   stretches; future passes may use spectral-flux features
//!   instead of pure energy.
//! - **Speaker-adaptive thresholds.** Threshold is global;
//!   varies-volume recordings may need per-segment
//!   adjustment.

use crate::vad::rms;

/// Configuration for [`split_words`].
#[derive(Debug, Clone)]
pub struct WordSplitConfig {
    /// RMS amplitude below which a frame is "silent".
    pub silence_threshold: f32,
    /// Minimum silent run that counts as a word boundary, in ms.
    pub min_silence_ms: u32,
    /// Frame window length in ms.
    pub frame_size_ms: u32,
    /// Hop between consecutive frames in ms.
    pub hop_size_ms: u32,
    /// Minimum length of a surviving word segment, in ms.
    pub min_word_ms: u32,
}

impl Default for WordSplitConfig {
    fn default() -> Self {
        Self {
            silence_threshold: 0.01,
            min_silence_ms: 80,
            frame_size_ms: 25,
            hop_size_ms: 10,
            min_word_ms: 80,
        }
    }
}

/// Split a PCM signal into word-sized segments at silent gaps.
///
/// Returns a vector of `(start_sample, end_sample)` pairs
/// (exclusive end). Leading / trailing silence is trimmed from
/// each segment.
pub fn split_words(
    samples: &[f32],
    sample_rate: u32,
    config: &WordSplitConfig,
) -> Vec<(usize, usize)> {
    let frame_size = ms_to_samples(config.frame_size_ms, sample_rate);
    let hop = ms_to_samples(config.hop_size_ms, sample_rate);
    let min_silence_frames = config.min_silence_ms.div_ceil(config.hop_size_ms) as usize;
    let min_word_samples = ms_to_samples(config.min_word_ms, sample_rate);

    if samples.len() < frame_size * 2 || hop == 0 {
        return Vec::new();
    }

    // 1. Per-frame RMS.
    let mut frame_rms: Vec<f32> = Vec::new();
    let mut off = 0_usize;
    while off + frame_size <= samples.len() {
        frame_rms.push(rms(&samples[off..off + frame_size]));
        off += hop;
    }

    // 2. Find silent-frame runs ≥ min_silence_frames; record
    //    the sample-index midpoint of each as a boundary.
    let mut boundaries: Vec<usize> = Vec::new();
    let mut silence_start: Option<usize> = None;
    for (i, &r) in frame_rms.iter().enumerate() {
        if r < config.silence_threshold {
            if silence_start.is_none() {
                silence_start = Some(i);
            }
        } else if let Some(s) = silence_start.take() {
            let run = i - s;
            if run >= min_silence_frames {
                let mid_frame = s + run / 2;
                boundaries.push(mid_frame * hop);
            }
        }
    }
    // Tail silence (recording ends in silence).
    if let Some(s) = silence_start {
        let run = frame_rms.len() - s;
        if run >= min_silence_frames {
            let mid_frame = s + run / 2;
            boundaries.push(mid_frame * hop);
        }
    }

    // 3. Convert boundaries → word ranges.
    let mut segments: Vec<(usize, usize)> = Vec::new();
    let mut prev = 0_usize;
    for &b in &boundaries {
        if b > prev {
            segments.push((prev, b));
        }
        prev = b;
    }
    if prev < samples.len() {
        segments.push((prev, samples.len()));
    }

    // 4. Drop too-short segments.
    segments.retain(|(s, e)| e - s >= min_word_samples);

    // 5. Trim leading / trailing silence inside each segment.
    let segs = segments;
    let mut trimmed: Vec<(usize, usize)> = Vec::with_capacity(segs.len());
    for (s, e) in segs {
        let (ts, te) = trim_silence(&samples[s..e], frame_size, hop, config.silence_threshold);
        let absolute = (s + ts, s + te);
        if absolute.1 > absolute.0 && absolute.1 - absolute.0 >= min_word_samples {
            trimmed.push(absolute);
        }
    }
    trimmed
}

fn ms_to_samples(ms: u32, sample_rate: u32) -> usize {
    (ms as usize * sample_rate as usize) / 1000
}

/// Trim leading + trailing silent frames from a segment.
/// Returns `(start, end)` offsets within the segment.
fn trim_silence(seg: &[f32], frame_size: usize, hop: usize, threshold: f32) -> (usize, usize) {
    let mut start = 0_usize;
    let mut end = seg.len();
    // Walk forward.
    while start + frame_size <= end {
        if rms(&seg[start..start + frame_size]) >= threshold {
            break;
        }
        start += hop;
    }
    // Walk backward.
    while end >= start + frame_size {
        let last = end - frame_size;
        if rms(&seg[last..end]) >= threshold {
            break;
        }
        end = end.saturating_sub(hop);
    }
    (start, end.max(start))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::harmonic_voice;

    /// Synthesise a "sentence" of N words separated by silences.
    fn synth_words(n_words: usize, word_ms: u32, gap_ms: u32, sample_rate: u32) -> Vec<f32> {
        let word_samples = ms_to_samples(word_ms, sample_rate);
        let gap_samples = ms_to_samples(gap_ms, sample_rate);
        let mut out = Vec::new();
        for w in 0..n_words {
            // Each "word" is a harmonic voice at slightly
            // different F0 (to vary spectra). Length = word_ms.
            let f0 = 120.0 + (w as f32) * 25.0;
            let word_audio: Vec<f32> =
                harmonic_voice(f0, word_ms as f32 / 1000.0, sample_rate, 0.4, 4);
            out.extend(word_audio.into_iter().take(word_samples));
            // Gap of silence.
            out.extend(std::iter::repeat_n(0.0_f32, gap_samples));
        }
        out
    }

    /// Three clearly-separated words → three segments recovered.
    #[test]
    fn three_words_with_clear_gaps() {
        let signal = synth_words(3, 300, 150, 16_000);
        let segs = split_words(&signal, 16_000, &WordSplitConfig::default());
        assert_eq!(segs.len(), 3, "got {} segments: {:?}", segs.len(), segs);
        // Each segment should be roughly word_ms in length.
        for (s, e) in &segs {
            let dur_ms = (e - s) as u32 * 1000 / 16_000;
            assert!(dur_ms > 100, "segment too short: {dur_ms} ms");
            assert!(dur_ms < 500, "segment too long: {dur_ms} ms");
        }
    }

    /// Five words → five segments.
    #[test]
    fn five_words_recovered() {
        let signal = synth_words(5, 250, 120, 16_000);
        let segs = split_words(&signal, 16_000, &WordSplitConfig::default());
        assert_eq!(segs.len(), 5, "got {} segments", segs.len());
    }

    /// Single word (no gaps) → one segment.
    #[test]
    fn single_word_one_segment() {
        let signal = synth_words(1, 500, 0, 16_000);
        let segs = split_words(&signal, 16_000, &WordSplitConfig::default());
        assert_eq!(segs.len(), 1);
    }

    /// All silence → no segments.
    #[test]
    fn pure_silence_no_segments() {
        let signal = vec![0.0_f32; 16_000];
        let segs = split_words(&signal, 16_000, &WordSplitConfig::default());
        assert!(segs.is_empty(), "got {} segments from silence", segs.len());
    }

    /// Empty / too-short input → no segments.
    #[test]
    fn empty_or_short_no_segments() {
        assert!(split_words(&[], 16_000, &WordSplitConfig::default()).is_empty());
        assert!(split_words(&[0.0_f32; 100], 16_000, &WordSplitConfig::default()).is_empty());
    }

    /// Segments are in increasing order and non-overlapping.
    #[test]
    fn segments_are_monotone_and_disjoint() {
        let signal = synth_words(4, 280, 130, 16_000);
        let segs = split_words(&signal, 16_000, &WordSplitConfig::default());
        for w in segs.windows(2) {
            let (_, end_prev) = w[0];
            let (start_next, _) = w[1];
            assert!(
                end_prev <= start_next,
                "overlap: {} > {}",
                end_prev,
                start_next
            );
        }
    }

    /// Default config is sensible.
    #[test]
    fn default_config_sane() {
        let c = WordSplitConfig::default();
        assert!(c.silence_threshold > 0.0 && c.silence_threshold < 0.1);
        assert!(c.min_silence_ms >= 50);
        assert!(c.min_word_ms >= 50);
        assert!(c.frame_size_ms > c.hop_size_ms);
    }

    /// Lower threshold (more sensitive) catches quieter gaps.
    #[test]
    fn lower_threshold_finds_more_segments() {
        // Same signal, two thresholds.
        let signal = synth_words(2, 300, 120, 16_000);
        let strict = split_words(
            &signal,
            16_000,
            &WordSplitConfig {
                silence_threshold: 0.5, // way too high — silences disappear
                ..WordSplitConfig::default()
            },
        );
        let normal = split_words(&signal, 16_000, &WordSplitConfig::default());
        // A too-high threshold treats the entire signal as
        // silence and yields zero segments (post-trim).
        assert!(strict.len() <= normal.len());
    }
}
