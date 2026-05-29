// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Excitation sources for the source-filter synthesiser.
//!
//! Two source types cover every Kazakh phoneme:
//!
//! - **Voiced** (vowels, nasals, liquids, glides, voiced obstruents):
//!   a quasi-periodic glottal pulse train at fundamental
//!   frequency `f0` Hz. We approximate the glottal flow
//!   derivative with a single-cycle impulse (band-limited via
//!   the formant filter that follows), which is the standard
//!   minimum-viable glottal source — enough to give the spectrum
//!   the right harmonic comb.
//! - **Unvoiced** (voiceless fricatives, stop bursts, aspiration):
//!   white noise from a deterministic XorShift PRNG so the
//!   output is reproducible. Reproducibility matters because we
//!   want byte-identical WAVs across runs for the corpus.

/// Deterministic XorShift PRNG. Seeded so synthesis is
/// byte-stable across runs.
pub struct XorShift {
    state: u64,
}

impl XorShift {
    pub fn new(seed: u64) -> Self {
        // Avoid the all-zero fixed point.
        let state = if seed == 0 { 0xC0FFEE_DEADBEEF } else { seed };
        Self { state }
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
    /// Uniform sample in `[-1, 1)`.
    pub fn next_f32(&mut self) -> f32 {
        // Take the top 24 bits → f32 in [0, 1), then re-centre.
        let bits = (self.next_u64() >> 40) as u32;
        let u = bits as f32 / (1u32 << 24) as f32;
        u * 2.0 - 1.0
    }
}

/// Voiced glottal excitation: a buffer with a single-sample
/// impulse at the start of every glottal period (`sample_rate /
/// f0` samples). Subsequent formant filtering smears each
/// impulse into a realistic pulse shape via the resonator
/// transient response.
pub fn glottal_pulse_train(f0: f32, duration_s: f32, sample_rate: u32) -> Vec<f32> {
    glottal_pulse_train_v(f0, duration_s, sample_rate, 0.0, 0.0, 0)
}

/// Voiced glottal excitation with voice-quality knobs.
///
/// - `jitter` ∈ [0, 1) — per-cycle fractional perturbation of
///   the glottal period (`0.02` = ±2 % ≈ healthy adult;
///   `0.05+` = elderly / rough). Pseudo-random, seeded.
/// - `breath` ∈ [0, 1] — additive white-noise mixed with the
///   pulse train (`0.0` = pure harmonic, `0.3` ≈ breathy
///   soprano, `0.5+` = whispered). Same seed.
/// - `seed` — XorShift seed; same seed ⇒ byte-identical output.
pub fn glottal_pulse_train_v(
    f0: f32,
    duration_s: f32,
    sample_rate: u32,
    jitter: f32,
    breath: f32,
    seed: u64,
) -> Vec<f32> {
    let n = (duration_s * sample_rate as f32) as usize;
    let mut out = vec![0.0_f32; n];
    if f0 <= 0.0 {
        return out;
    }
    let mut rng = XorShift::new(seed.wrapping_add(0x0006_C0CE_5A1A));
    let period_samples = (sample_rate as f32 / f0).max(1.0);
    let mut next_pulse: f32 = 0.0;
    loop {
        let idx = next_pulse as usize;
        if idx >= n {
            break;
        }
        out[idx] = 1.0;
        // Perturb the next period by ±jitter (uniform).
        let jitter_factor = if jitter > 0.0 {
            1.0 + jitter * rng.next_f32()
        } else {
            1.0
        };
        next_pulse += period_samples * jitter_factor.max(0.1);
    }
    if breath > 0.0 {
        let mut breath_rng = XorShift::new(seed.wrapping_add(0xB5EA7));
        for s in out.iter_mut() {
            *s += breath * breath_rng.next_f32() * 0.5;
        }
    }
    out
}

/// Unvoiced excitation: white noise normalised to roughly
/// ±0.5 RMS. Deterministic for a given seed.
pub fn white_noise(duration_s: f32, sample_rate: u32, seed: u64) -> Vec<f32> {
    let n = (duration_s * sample_rate as f32) as usize;
    let mut rng = XorShift::new(seed);
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        out.push(rng.next_f32() * 0.5);
    }
    out
}

/// Linear envelope: ramp up over `attack_s` seconds, hold, then
/// ramp down over `release_s`. Used to fade phoneme boundaries
/// so concatenation doesn't click.
pub fn apply_envelope(samples: &mut [f32], attack_s: f32, release_s: f32, sample_rate: u32) {
    let n = samples.len();
    let attack_n = ((attack_s * sample_rate as f32) as usize).min(n / 2);
    let release_n = ((release_s * sample_rate as f32) as usize).min(n / 2);
    for (i, sample_ref) in samples.iter_mut().enumerate().take(attack_n) {
        *sample_ref *= i as f32 / attack_n as f32;
    }
    for i in 0..release_n {
        let idx = n - 1 - i;
        samples[idx] *= i as f32 / release_n as f32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xorshift_is_deterministic() {
        let mut a = XorShift::new(42);
        let mut b = XorShift::new(42);
        for _ in 0..1000 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn xorshift_f32_in_range() {
        let mut r = XorShift::new(7);
        for _ in 0..10_000 {
            let x = r.next_f32();
            assert!((-1.0..1.0).contains(&x), "out of range: {x}");
        }
    }

    #[test]
    fn pulse_train_has_correct_period() {
        // 100 Hz at 16 kHz → period = 160 samples.
        let pcm = glottal_pulse_train(100.0, 0.05, 16_000);
        let pulses: Vec<usize> = pcm
            .iter()
            .enumerate()
            .filter(|&(_, &x)| x > 0.5)
            .map(|(i, _)| i)
            .collect();
        assert!(pulses.len() >= 4, "got {} pulses", pulses.len());
        // Average spacing ≈ 160.
        let avg_spacing =
            (pulses.last().unwrap() - pulses.first().unwrap()) as f32 / (pulses.len() - 1) as f32;
        assert!(
            (avg_spacing - 160.0).abs() < 2.0,
            "avg spacing {avg_spacing}"
        );
    }

    #[test]
    fn white_noise_is_zero_mean() {
        let pcm = white_noise(1.0, 16_000, 42);
        let mean: f32 = pcm.iter().sum::<f32>() / pcm.len() as f32;
        assert!(mean.abs() < 0.01, "noise mean = {mean}");
    }

    #[test]
    fn envelope_zeroes_endpoints() {
        let mut pcm = vec![1.0_f32; 1000];
        apply_envelope(&mut pcm, 0.01, 0.01, 10_000); // 100 sample attack/release
        assert!(pcm[0].abs() < 1e-6);
        assert!(pcm[999].abs() < 1e-6);
        assert!((pcm[500] - 1.0).abs() < 1e-6); // middle untouched
    }
}
