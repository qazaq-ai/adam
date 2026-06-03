// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Fundamental-frequency (F0) detection — the YIN algorithm.
//!
//! Reference: de Cheveigné, A. & Kawahara, H. (2002), *YIN, a
//! fundamental frequency estimator for speech and music*,
//! JASA 111(4).
//!
//! YIN is a time-domain pitch estimator built on a normalised
//! difference function. It needs no FFT, fits in ~60 LoC of
//! pure Rust, and gives sub-percent F0 accuracy on clean,
//! voiced speech in the 70–500 Hz range — which spans every
//! voice the v6.3 stack cares about (low adult male →
//! high-pitched child).
//!
//! ## What this module does NOT do
//!
//! - Voicing decision: it does not tell you whether a given
//!   frame contains a periodic signal (vs. unvoiced noise /
//!   silence). The caller pairs F0 with [`crate::vad`] or
//!   inspects the returned `Option<f32>` (`None` = no clear
//!   periodicity).
//! - Jitter / shimmer: the
//!   [`crate::speaker_profile`] layer computes these on top.
//! - Sub-sample interpolation beyond parabolic: that's enough
//!   for 1-Hz precision at 16 kHz, which is well below
//!   downstream classifier thresholds.

/// Detect the fundamental frequency of a voiced audio frame.
///
/// Returns `Some(f0_hz)` when a periodic signal is found in the
/// `[70, 500]` Hz range with normalised-difference value below
/// the threshold (0.15 by default); `None` otherwise.
///
/// `samples` must be mono. For mixed-channel buffers, downmix
/// first via [`crate::PcmSamples::to_mono`].
pub fn detect_f0(samples: &[f32], sample_rate: u32) -> Option<f32> {
    detect_f0_with_range(samples, sample_rate, 70.0, 500.0, 0.15)
}

/// Like [`detect_f0`] but with explicit search-range and
/// threshold parameters for callers that know the speaker's
/// approximate range.
pub fn detect_f0_with_range(
    samples: &[f32],
    sample_rate: u32,
    min_f0_hz: f32,
    max_f0_hz: f32,
    threshold: f32,
) -> Option<f32> {
    let max_tau = (sample_rate as f32 / min_f0_hz) as usize;
    let min_tau = (sample_rate as f32 / max_f0_hz) as usize;
    if samples.len() < max_tau * 2 || min_tau == 0 {
        return None;
    }

    // Step 1: difference function d_t(τ) = Σ (x_i - x_{i+τ})².
    let n = samples.len() - max_tau;
    let mut d = vec![0.0_f32; max_tau + 1];
    for tau in 1..=max_tau {
        let mut sum = 0.0_f32;
        for i in 0..n {
            let diff = samples[i] - samples[i + tau];
            sum += diff * diff;
        }
        d[tau] = sum;
    }

    // Step 2: cumulative mean normalised difference d'(τ).
    let mut dprime = vec![1.0_f32; max_tau + 1];
    let mut running = 0.0_f32;
    for tau in 1..=max_tau {
        running += d[tau];
        if running > 0.0 {
            dprime[tau] = d[tau] * tau as f32 / running;
        }
    }

    // Step 3: first dip below threshold (absolute threshold rule).
    let mut tau_candidate: Option<usize> = None;
    let mut t = min_tau;
    while t < max_tau {
        if dprime[t] < threshold {
            // Walk to the local minimum within the dip.
            let mut local_min = t;
            while local_min + 1 < max_tau && dprime[local_min + 1] < dprime[local_min] {
                local_min += 1;
            }
            tau_candidate = Some(local_min);
            break;
        }
        t += 1;
    }

    let tau = tau_candidate?;

    // Step 4: parabolic interpolation for sub-sample precision.
    let tau_refined = if tau > 0 && tau < max_tau {
        let s0 = dprime[tau - 1];
        let s1 = dprime[tau];
        let s2 = dprime[tau + 1];
        let denom = 2.0 * (s0 - 2.0 * s1 + s2);
        if denom.abs() > 1e-10 {
            tau as f32 + (s0 - s2) / denom
        } else {
            tau as f32
        }
    } else {
        tau as f32
    };

    Some(sample_rate as f32 / tau_refined)
}

/// Generate a synthetic sine wave for testing pitch detection.
/// Used in unit tests; exposed pub for downstream test fixtures.
pub fn sine_wave(freq_hz: f32, duration_s: f32, sample_rate: u32, amplitude: f32) -> Vec<f32> {
    let n = (duration_s * sample_rate as f32) as usize;
    (0..n)
        .map(|i| {
            amplitude * (2.0 * std::f32::consts::PI * freq_hz * i as f32 / sample_rate as f32).sin()
        })
        .collect()
}

/// Generate a **harmonic stack** simulating a voice: F0 plus a
/// few overtones at relative amplitudes that approximate the
/// spectral envelope of a voiced segment (`-3 dB` per harmonic).
/// This is much closer to a real voice than a pure sine.
///
/// `n_harmonics = 1` collapses to a pure sine; `4–5` is a
/// reasonable voice approximation.
pub fn harmonic_voice(
    f0_hz: f32,
    duration_s: f32,
    sample_rate: u32,
    amplitude: f32,
    n_harmonics: u32,
) -> Vec<f32> {
    let n = (duration_s * sample_rate as f32) as usize;
    (0..n)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            let mut s = 0.0_f32;
            for h in 1..=n_harmonics {
                let amp = amplitude / (h as f32).sqrt();
                s += amp * (2.0 * std::f32::consts::PI * f0_hz * h as f32 * t).sin();
            }
            // Normalise to avoid clipping after stacking.
            s / (n_harmonics as f32).sqrt()
        })
        .collect()
}

/// Add deterministic pseudo-Gaussian noise at the requested
/// signal-to-noise ratio (dB). Noise is generated by a small
/// LCG so tests are reproducible without an external RNG dep.
pub fn add_noise(signal: &[f32], snr_db: f32) -> Vec<f32> {
    let signal_power: f32 = signal.iter().map(|s| s * s).sum::<f32>() / signal.len() as f32;
    let noise_power = signal_power / 10.0_f32.powf(snr_db / 10.0);
    let noise_amp = noise_power.sqrt();
    // Marsaglia-polar form using two LCG draws → approx Gaussian.
    let mut state: u64 = 0x12345678_9abcdef0;
    signal
        .iter()
        .map(|&s| {
            let u1 = lcg_next(&mut state);
            let u2 = lcg_next(&mut state);
            let r = (-2.0 * u1.ln()).sqrt();
            let theta = 2.0 * std::f32::consts::PI * u2;
            let gauss = r * theta.cos();
            s + noise_amp * gauss
        })
        .collect()
}

/// Generate a **jittered** voice: each successive period
/// (cycle) has length perturbed by `±jitter` fraction.
///
/// **This is real cycle-to-cycle jitter**, the kind a pitch
/// detector should see as elevated F0 coefficient-of-variation
/// at the frame level. (An earlier implementation perturbed F0
/// every sample, which produced smooth FM that washed out at
/// the frame averaging step — a real-data test surfaced that
/// bug; this version is the fix.)
///
/// `jitter = 0.05` ≈ ±5 % period variation per cycle, which is
/// at the lower end of pathological-voice range. `0.15` ≈ ±15 %
/// is clearly senior / hoarse. Healthy adults typically run
/// 0.01–0.02.
pub fn jittered_voice(
    f0_hz: f32,
    duration_s: f32,
    sample_rate: u32,
    amplitude: f32,
    jitter: f32,
) -> Vec<f32> {
    let mut state: u64 = 0xc0ffee_deadbeef;
    let nominal_period_samples = sample_rate as f32 / f0_hz;
    let n_total = (duration_s * sample_rate as f32) as usize;
    let mut out = Vec::with_capacity(n_total);
    while out.len() < n_total {
        // Perturb this cycle's period by ±jitter.
        let wobble = (lcg_next(&mut state) - 0.5) * 2.0 * jitter;
        let period_samples = (nominal_period_samples * (1.0 + wobble)).round().max(2.0) as usize;
        let cap = (n_total - out.len()).min(period_samples);
        for i in 0..cap {
            let t = i as f32 / period_samples as f32;
            out.push(amplitude * (2.0 * std::f32::consts::PI * t).sin());
        }
    }
    out
}

fn lcg_next(state: &mut u64) -> f32 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*state >> 11) as f32) / ((1_u64 << 53) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthetic pure 220 Hz tone at 16 kHz: detector should
    /// recover ~220 Hz within 1 % tolerance.
    #[test]
    fn detects_220hz_sine_wave() {
        let samples = sine_wave(220.0, 0.25, 16_000, 0.5);
        let f0 = detect_f0(&samples, 16_000).unwrap();
        let err = (f0 - 220.0).abs() / 220.0;
        assert!(err < 0.01, "expected ~220 Hz, got {f0}");
    }

    /// 120 Hz (typical adult male) → recovered within 1 %.
    #[test]
    fn detects_male_voice_range() {
        let samples = sine_wave(120.0, 0.25, 16_000, 0.5);
        let f0 = detect_f0(&samples, 16_000).unwrap();
        let err = (f0 - 120.0).abs() / 120.0;
        assert!(err < 0.01, "expected ~120 Hz, got {f0}");
    }

    /// 250 Hz (typical adult female / child) → recovered within
    /// 1 %.
    #[test]
    fn detects_high_voice_range() {
        let samples = sine_wave(250.0, 0.25, 16_000, 0.5);
        let f0 = detect_f0(&samples, 16_000).unwrap();
        let err = (f0 - 250.0).abs() / 250.0;
        assert!(err < 0.01, "expected ~250 Hz, got {f0}");
    }

    /// Pure noise produces no F0 detection (or a value that
    /// fails the absolute threshold).
    #[test]
    fn pure_noise_yields_no_detection() {
        // White-noise-like signal: pseudo-random samples.
        let samples: Vec<f32> = (0..4_000)
            .map(|i| ((i * 12_345 + 7) % 200) as f32 / 100.0 - 1.0)
            .collect();
        let f0 = detect_f0(&samples, 16_000);
        // Either None, or far outside any expected speaker
        // range — the point is that voiced classification must
        // not return a confidently-low number.
        if let Some(hz) = f0 {
            assert!(
                !(80.0..400.0).contains(&hz),
                "noise spuriously classified as voiced at {hz} Hz"
            );
        }
    }

    /// Silence (all zeros) → no detection.
    #[test]
    fn silence_yields_no_detection() {
        let samples = vec![0.0_f32; 4_000];
        assert_eq!(detect_f0(&samples, 16_000), None);
    }

    /// Too-short buffer → no detection.
    #[test]
    fn too_short_buffer_yields_none() {
        let samples = sine_wave(220.0, 0.005, 16_000, 0.5);
        // 5 ms at 16 kHz = 80 samples; max_tau at 70 Hz min_f0
        // is ~228 samples, so 2*max_tau > buffer → None.
        assert_eq!(detect_f0(&samples, 16_000), None);
    }

    // ─── Realistic-voice tests ─────────────────────────────────
    // These cover signal characteristics a real voice has but a
    // pure sine does NOT: harmonics, noise, and period jitter.
    // Adding them is the cure for «cherry-picked pure-sine tests
    // pass while real-voice REPL audits surface bugs».

    /// Harmonic stack (F0 + 4 overtones) at 120 Hz — much closer
    /// to a real adult male voice than a pure sine. YIN is
    /// supposed to be robust to harmonic content; this test
    /// pins that.
    #[test]
    fn detects_harmonic_voice_120hz() {
        let samples = harmonic_voice(120.0, 0.25, 16_000, 0.5, 4);
        let f0 = detect_f0(&samples, 16_000).unwrap();
        let err = (f0 - 120.0).abs() / 120.0;
        assert!(err < 0.02, "harmonic 120 Hz → got {f0}, err {err}");
    }

    /// Harmonic voice at 220 Hz with 20 dB SNR Gaussian noise.
    /// Real recordings always have noise; YIN should still
    /// recover F0 within 2 %.
    #[test]
    fn detects_voice_at_20db_snr() {
        let clean = harmonic_voice(220.0, 0.30, 16_000, 0.5, 4);
        let noisy = add_noise(&clean, 20.0);
        let f0 = detect_f0(&noisy, 16_000).unwrap();
        let err = (f0 - 220.0).abs() / 220.0;
        assert!(err < 0.02, "20 dB SNR 220 Hz → got {f0}, err {err}");
    }

    /// Same at 10 dB SNR — louder noise. YIN may degrade but
    /// should still be within 5 %.
    #[test]
    fn detects_voice_at_10db_snr() {
        let clean = harmonic_voice(150.0, 0.30, 16_000, 0.5, 4);
        let noisy = add_noise(&clean, 10.0);
        let f0 = detect_f0(&noisy, 16_000).unwrap();
        let err = (f0 - 150.0).abs() / 150.0;
        assert!(err < 0.05, "10 dB SNR 150 Hz → got {f0}, err {err}");
    }

    /// Jittered voice (5 % CV) — models elderly / pathological
    /// voice. F0 estimate should still land near the centre
    /// frequency, just with reduced precision.
    #[test]
    fn detects_jittered_voice() {
        let samples = jittered_voice(140.0, 0.50, 16_000, 0.5, 0.05);
        let f0 = detect_f0(&samples, 16_000).unwrap();
        let err = (f0 - 140.0).abs() / 140.0;
        assert!(err < 0.07, "5% jitter 140 Hz → got {f0}, err {err}");
    }

    /// Octave-ambiguity stress: signal with strong second
    /// harmonic (`F0 + 0.9 * 2F0`) at F0 = 100 Hz. A naive
    /// estimator might lock onto 200 Hz. YIN's normalised-
    /// difference scoring should prefer the true F0.
    #[test]
    fn does_not_octave_jump_on_strong_second_harmonic() {
        let f0 = 100.0_f32;
        let n = (0.30 * 16_000.0) as usize;
        let samples: Vec<f32> = (0..n)
            .map(|i| {
                let t = i as f32 / 16_000.0;
                let s1 = (2.0 * std::f32::consts::PI * f0 * t).sin();
                let s2 = (2.0 * std::f32::consts::PI * f0 * 2.0 * t).sin();
                0.4 * s1 + 0.36 * s2
            })
            .collect();
        let detected = detect_f0(&samples, 16_000).unwrap();
        // Allow either the true F0 (correct) or near 200 Hz
        // (octave jump). Pin which one we got so a regression
        // surfaces:
        let near_100 = (detected - 100.0).abs() < 5.0;
        let near_200 = (detected - 200.0).abs() < 5.0;
        assert!(
            near_100 || near_200,
            "expected ~100 Hz or ~200 Hz, got {detected}",
        );
        // We want the true F0; mark octave jump as known-issue
        // if it surfaces.
        assert!(
            near_100,
            "octave jump regression: got {detected}, expected ~100 Hz",
        );
    }

    /// Sweep test: detect F0 across the full speaker range
    /// (80 Hz–350 Hz in 10 Hz steps). Average error must stay
    /// under 1 % across the range.
    #[test]
    fn accuracy_sweep_across_speaker_range() {
        let mut total_err = 0.0_f32;
        let mut count = 0_u32;
        let mut f = 80.0_f32;
        while f <= 350.0 {
            let samples = harmonic_voice(f, 0.20, 16_000, 0.4, 4);
            let detected = detect_f0(&samples, 16_000).unwrap();
            let err = (detected - f).abs() / f;
            assert!(err < 0.03, "sweep failed at {f} Hz: got {detected}");
            total_err += err;
            count += 1;
            f += 10.0;
        }
        let avg = total_err / count as f32;
        assert!(avg < 0.01, "average error across sweep too high: {avg}");
    }

    /// 48 kHz sample-rate path: same voice, recovered F0 within
    /// 1 %. Confirms the algorithm is sample-rate agnostic.
    #[test]
    fn works_at_48khz() {
        let samples = harmonic_voice(180.0, 0.25, 48_000, 0.5, 4);
        let f0 = detect_f0(&samples, 48_000).unwrap();
        let err = (f0 - 180.0).abs() / 180.0;
        assert!(err < 0.01, "48 kHz 180 Hz → got {f0}");
    }
}
