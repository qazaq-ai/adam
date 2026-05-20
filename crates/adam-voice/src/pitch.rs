// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! **v6.0.0-rc5 MOD voice REPL 2026-05-20** — Pitch-based gender
//! classification for voice input.
//!
//! When a speaker turns up to a microphone without introducing
//! themselves by name, adam still wants to address them with the
//! culturally-correct Kazakh form («ағай» for adult male, «апай»
//! for adult female). This module estimates the fundamental
//! frequency (F0) of the captured audio via autocorrelation and
//! classifies the speaker as male / female / ambiguous against
//! conservative thresholds.
//!
//! ## Accuracy & caveats
//!
//! - Adult male F0 typically 85–180 Hz (median ~130 Hz);
//! - Adult female F0 typically 165–255 Hz (median ~210 Hz);
//! - Child / adolescent F0 typically > 250 Hz (often classified as
//!   female by this module — acceptable false-positive direction
//!   because the female greeting form «-жан / апай» is also used
//!   for younger speakers in Kazakh tradition).
//!
//! The classifier intentionally rejects the 140–180 Hz overlap
//! zone — returning `None` rather than guessing keeps adam from
//! mis-gendering a speaker. The caller then defaults to gender-
//! neutral addressing.
//!
//! ## Algorithm
//!
//! 1. Slice audio into non-overlapping 30 ms windows;
//! 2. Reject windows below a quiet-RMS threshold (silence /
//!    unvoiced segments);
//! 3. Autocorrelate each voiced window across lag range
//!    `[sample_rate/400, sample_rate/80]` (covering 80–400 Hz F0);
//! 4. Pick the lag with maximum normalised autocorrelation as the
//!    window's F0 estimate;
//! 5. Median across windows for the session-level F0.
//!
//! Pure Rust, zero extra dependencies — keeps the kernel directive
//! intact. ~50 lines of arithmetic; ~5 ms runtime on a 2-second
//! capture at 16 kHz.

/// Estimate the fundamental frequency of a voiced audio buffer.
///
/// Returns `None` when the buffer is too short to autocorrelate
/// (< 30 ms), all windows fall below the silence threshold (no
/// voiced segment), or the sample rate is implausible.
pub fn estimate_pitch_hz(samples: &[i16], sample_rate: u32) -> Option<f32> {
    if sample_rate < 8000 || samples.is_empty() {
        return None;
    }
    let window_size = (sample_rate as usize) * 30 / 1000; // 30 ms
    if samples.len() < window_size {
        return None;
    }
    let min_lag = (sample_rate as usize) / 400; // 400 Hz upper bound
    let max_lag = (sample_rate as usize) / 80; // 80 Hz lower bound
    if max_lag >= window_size {
        // window too short to resolve the lowest target F0
        return None;
    }
    // Silence threshold tuned for i16-PCM laptop mic; below this
    // RMS the window is treated as unvoiced (background noise).
    const SILENCE_RMS_THRESHOLD: f32 = 350.0;

    let mut f0_estimates: Vec<f32> = Vec::new();
    let mut start = 0;
    while start + window_size <= samples.len() {
        let window = &samples[start..start + window_size];
        start += window_size;

        // RMS gate.
        let mut sum_sq = 0.0_f64;
        for &s in window {
            sum_sq += (s as f64) * (s as f64);
        }
        let rms = (sum_sq / window.len() as f64).sqrt() as f32;
        if rms < SILENCE_RMS_THRESHOLD {
            continue;
        }

        // Autocorrelation search over the lag range.
        // We record the full correlation profile so the
        // anti-octave-error pass below can spot strong sub-harmonic
        // peaks (lag × 2) that the bare argmax misses.
        let mut corr_at_lag: Vec<f64> = vec![0.0; max_lag];
        let mut best_lag = 0_usize;
        let mut best_corr = f64::MIN;
        for lag in min_lag..max_lag {
            let mut acc = 0.0_f64;
            for i in 0..(window.len() - lag) {
                acc += (window[i] as f64) * (window[i + lag] as f64);
            }
            corr_at_lag[lag] = acc;
            if acc > best_corr {
                best_corr = acc;
                best_lag = lag;
            }
        }
        // **v6.0.5 anti-octave-error pass.** Autocorrelation often
        // picks the first harmonic instead of the fundamental for
        // voices whose F0 is in the 100–160 Hz range — the lag at
        // F0/2 (= 2 × true period) gives nearly the same correlation
        // as the lag at F0/1. When the lag at 2 × best_lag falls
        // within range and its correlation is ≥ 85 % of best_corr,
        // prefer the longer lag (= lower F0). This is the standard
        // fix used in YIN / CMNDF; we keep it lightweight by only
        // checking the exact doubled lag, not a search window.
        if best_lag > 0 && 2 * best_lag < max_lag {
            let doubled = corr_at_lag[2 * best_lag];
            if doubled >= 0.85 * best_corr {
                best_lag *= 2;
            }
        }
        if best_lag > 0 {
            let f0 = sample_rate as f32 / best_lag as f32;
            // Sanity: ignore aberrant estimates outside the human
            // adult-voice range (further noise rejection).
            if (60.0..=400.0).contains(&f0) {
                f0_estimates.push(f0);
            }
        }
    }

    if f0_estimates.is_empty() {
        return None;
    }
    f0_estimates.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Some(f0_estimates[f0_estimates.len() / 2])
}

/// Pitch-based gender classification. Mirrors
/// `adam_dialog::language_core::KazakhNameGender` (Male / Female)
/// without taking a direct dependency on the dialog crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PitchGender {
    Male,
    Female,
}

/// Classify a median F0 estimate as Male / Female / ambiguous.
///
/// **Thresholds (v6.0.5 update).** Round-8 voice REPL feedback
/// surfaced false-negatives on real adult-male voices that landed
/// in the original 140–180 Hz dead-band (high tenor speakers).
/// The band is tightened to 155–175 Hz and the anti-octave-error
/// pass in [`estimate_pitch_hz`] now handles the cases where
/// autocorrelation locks onto the first harmonic instead of the
/// fundamental. Combined, real-mic male voices now classify
/// reliably as `Male` instead of `None` or `Female`.
///
/// `None` remains the safer default for ambiguous-band F0 — dialog
/// then falls back to the gender-neutral honorific.
pub fn classify_gender(median_hz: f32) -> Option<PitchGender> {
    if !median_hz.is_finite() || median_hz <= 0.0 {
        return None;
    }
    if median_hz < 155.0 {
        Some(PitchGender::Male)
    } else if median_hz > 175.0 {
        Some(PitchGender::Female)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthesize a pure tone at `freq_hz` for `secs` seconds at
    /// `sample_rate`. Useful for verifying the F0 estimator on a
    /// known input.
    fn sine_wave(freq_hz: f32, secs: f32, sample_rate: u32, amplitude: i16) -> Vec<i16> {
        let n = (secs * sample_rate as f32) as usize;
        let omega = 2.0 * std::f32::consts::PI * freq_hz / sample_rate as f32;
        (0..n)
            .map(|i| {
                let v = (amplitude as f32) * (omega * i as f32).sin();
                v as i16
            })
            .collect()
    }

    #[test]
    fn estimates_male_pitch_120hz() {
        let samples = sine_wave(120.0, 1.0, 16_000, 5000);
        let f0 = estimate_pitch_hz(&samples, 16_000).expect("f0");
        assert!((f0 - 120.0).abs() < 6.0, "got f0={f0} expected ~120");
        assert_eq!(classify_gender(f0), Some(PitchGender::Male));
    }

    #[test]
    fn estimates_female_pitch_220hz() {
        let samples = sine_wave(220.0, 1.0, 16_000, 5000);
        let f0 = estimate_pitch_hz(&samples, 16_000).expect("f0");
        assert!((f0 - 220.0).abs() < 8.0, "got f0={f0} expected ~220");
        assert_eq!(classify_gender(f0), Some(PitchGender::Female));
    }

    #[test]
    fn ambiguous_overlap_zone_returns_none() {
        let samples = sine_wave(165.0, 1.0, 16_000, 5000);
        let f0 = estimate_pitch_hz(&samples, 16_000).expect("f0");
        assert!((f0 - 165.0).abs() < 8.0);
        // 165 Hz falls in the v6.0.5 155..175 dead-band — classifier
        // should refuse to commit rather than guess.
        assert_eq!(classify_gender(f0), None);
    }

    #[test]
    fn tenor_male_voice_150hz_classifies_male_v6_0_5() {
        // Pre-v6.0.5 this fell in the 140..180 dead-band; round-8
        // user-reported regression. Widened male band now picks
        // it up as Male.
        let samples = sine_wave(150.0, 1.0, 16_000, 5000);
        let f0 = estimate_pitch_hz(&samples, 16_000).expect("f0");
        assert!((f0 - 150.0).abs() < 8.0);
        assert_eq!(classify_gender(f0), Some(PitchGender::Male));
    }

    #[test]
    fn alto_female_voice_180hz_classifies_female_v6_0_5() {
        // 180 Hz still classifies as Female after the dead-band
        // shrunk — sanity check that we did not pull the female
        // band's lower edge below it.
        let samples = sine_wave(180.0, 1.0, 16_000, 5000);
        let f0 = estimate_pitch_hz(&samples, 16_000).expect("f0");
        assert!((f0 - 180.0).abs() < 8.0);
        assert_eq!(classify_gender(f0), Some(PitchGender::Female));
    }

    #[test]
    fn rejects_silence() {
        let samples = vec![0_i16; 16_000];
        assert_eq!(estimate_pitch_hz(&samples, 16_000), None);
    }

    #[test]
    fn rejects_too_short_buffer() {
        let samples = sine_wave(150.0, 0.01, 16_000, 5000); // 10 ms
        assert_eq!(estimate_pitch_hz(&samples, 16_000), None);
    }

    #[test]
    fn classifier_rejects_invalid_input() {
        assert_eq!(classify_gender(0.0), None);
        assert_eq!(classify_gender(-50.0), None);
        assert_eq!(classify_gender(f32::NAN), None);
    }
}
