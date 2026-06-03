// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Biquad resonator + formant-cascade filter.
//!
//! Each formant is a single second-order IIR resonator
//! (band-pass biquad) tuned to a centre frequency `f_c` Hz with
//! a 3-dB bandwidth `bw` Hz. The vocal tract is approximated by
//! cascading three of them — F1, F2, F3 — which is the standard
//! Klatt-style minimum-viable formant synthesiser.
//!
//! ## Resonator coefficients
//!
//! Pole at `(r · cos θ, ±r · sin θ)` where
//!
//! ```text
//!   θ = 2π · f_c / sample_rate
//!   r = exp(-π · bw / sample_rate)
//! ```
//!
//! Direct-form-I difference equation:
//!
//! ```text
//!   y[n] = b0·x[n] + b1·x[n-1] + b2·x[n-2] − a1·y[n-1] − a2·y[n-2]
//! ```
//!
//! With `b0 = (1 - r²) · sinθ`, `b1 = 0`, `b2 = 0`,
//! `a1 = -2r·cosθ`, `a2 = r²` we get a unit-gain resonant
//! band-pass centred on `f_c`. (Klatt 1980; Rabiner & Schafer.)
//!
//! Lip radiation is approximated by a `1 - α·z⁻¹` high-pass
//! pre-emphasis applied to the final output.

/// One second-order IIR resonator. Maintains two-sample state
/// across calls so the filter can be invoked frame-by-frame.
#[derive(Debug, Clone)]
pub struct Resonator {
    a1: f32,
    a2: f32,
    b0: f32,
    z1: f32,
    z2: f32,
}

impl Resonator {
    /// Build a band-pass resonator centred at `f_c` Hz with a
    /// 3-dB bandwidth `bw` Hz, sampled at `sample_rate`.
    pub fn new(f_c: f32, bw: f32, sample_rate: u32) -> Self {
        let sr = sample_rate as f32;
        let theta = 2.0 * std::f32::consts::PI * f_c / sr;
        let r = (-std::f32::consts::PI * bw / sr).exp();
        let a1 = -2.0 * r * theta.cos();
        let a2 = r * r;
        // Unit-magnitude gain at f_c.
        let b0 = (1.0 - r * r) * theta.sin();
        Self {
            a1,
            a2,
            b0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    /// Process one sample.
    pub fn step(&mut self, x: f32) -> f32 {
        // Direct-form-I difference eqn (b1 = b2 = 0).
        let y = self.b0 * x - self.a1 * self.z1 - self.a2 * self.z2;
        self.z2 = self.z1;
        self.z1 = y;
        y
    }
}

/// A vocal-tract filter: cascade of formant resonators
/// (typically F1, F2, F3) plus a final lip-radiation
/// pre-emphasis (`y[n] = x[n] - 0.97·x[n-1]`).
#[derive(Debug, Clone)]
pub struct FormantFilter {
    resonators: Vec<Resonator>,
    radiation_prev: f32,
}

impl FormantFilter {
    pub fn new(formants: &[(f32, f32)], sample_rate: u32) -> Self {
        let resonators = formants
            .iter()
            .map(|&(f, bw)| Resonator::new(f, bw, sample_rate))
            .collect();
        Self {
            resonators,
            radiation_prev: 0.0,
        }
    }

    /// Filter one input sample through every resonator in
    /// series, then apply lip-radiation pre-emphasis.
    pub fn step(&mut self, x: f32) -> f32 {
        let mut y = x;
        for r in self.resonators.iter_mut() {
            y = r.step(y);
        }
        let out = y - 0.97 * self.radiation_prev;
        self.radiation_prev = y;
        out
    }

    /// Apply to a whole buffer.
    pub fn process(&mut self, input: &[f32]) -> Vec<f32> {
        input.iter().map(|&x| self.step(x)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stability: feeding a unit impulse to a resonator must
    /// produce a bounded decaying oscillation, never blow up.
    #[test]
    fn resonator_is_stable() {
        let mut r = Resonator::new(500.0, 80.0, 16_000);
        let mut max = 0.0_f32;
        let mut y = r.step(1.0);
        for _ in 0..16_000 {
            y = r.step(0.0);
            let a = y.abs();
            if a > max {
                max = a;
            }
            assert!(a.is_finite(), "resonator blew up: {a}");
        }
        // Decay verified: the response should be well under 1 after a second.
        assert!(y.abs() < 0.001, "should have decayed");
        assert!(max < 5.0, "transient too large: {max}");
    }

    /// Frequency selectivity: a 500 Hz resonator should pass a
    /// 500 Hz tone much more strongly than a 2000 Hz tone.
    #[test]
    fn resonator_is_frequency_selective() {
        let sr = 16_000;
        let dur_s = 0.1;
        let n = (dur_s * sr as f32) as usize;
        let on_freq: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * 500.0 * i as f32 / sr as f32).sin())
            .collect();
        let off_freq: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * 2000.0 * i as f32 / sr as f32).sin())
            .collect();
        let mut r_on = Resonator::new(500.0, 80.0, sr);
        let mut r_off = Resonator::new(500.0, 80.0, sr);
        let on_out: Vec<f32> = on_freq.iter().map(|&x| r_on.step(x)).collect();
        let off_out: Vec<f32> = off_freq.iter().map(|&x| r_off.step(x)).collect();
        // Last quarter (steady state).
        let on_rms: f32 =
            (on_out[n * 3 / 4..].iter().map(|x| x * x).sum::<f32>() / (n / 4) as f32).sqrt();
        let off_rms: f32 =
            (off_out[n * 3 / 4..].iter().map(|x| x * x).sum::<f32>() / (n / 4) as f32).sqrt();
        assert!(
            on_rms > off_rms * 3.0,
            "resonator not selective: on={on_rms} off={off_rms}"
        );
    }

    /// Pre-emphasis: a DC signal should be killed by the
    /// 1 - 0.97 z^-1 stage after one sample of settling.
    #[test]
    fn radiation_kills_dc() {
        let mut f = FormantFilter::new(&[], 16_000);
        let mut out_last = 0.0;
        for _ in 0..100 {
            out_last = f.step(1.0);
        }
        // 1 - 0.97 = 0.03 steady-state for unit DC.
        assert!(
            (out_last - 0.03).abs() < 1e-3,
            "DC response should be ~0.03, got {out_last}"
        );
    }
}
