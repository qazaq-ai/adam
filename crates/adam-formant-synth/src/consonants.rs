// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Kazakh consonant synthesis.
//!
//! Consonants don't have stable formant patterns the way vowels
//! do; instead each manner class needs its own excitation +
//! filter recipe. The four big classes:
//!
//! - **Stops** (P/B/T/D/K/G/Q): silence → brief burst → optional
//!   voicing tail. The burst is a single-sample impulse passed
//!   through a band-pass at the stop's characteristic burst
//!   frequency.
//! - **Fricatives** (S/Sh/Z/Zh/F/V/X/Gh/H): white noise passed
//!   through a band-pass at the fricative's characteristic
//!   spectral peak (the "centre of gravity" for each frication).
//!   Voiced fricatives (Z/Zh/V/Gh) get the noise mixed with a
//!   low-amplitude glottal source.
//! - **Nasals** (M/N/Ng): voiced source through a single low
//!   resonance (~250-300 Hz) with a high-frequency anti-resonance
//!   approximated as low-pass roll-off.
//! - **Liquids/glides/affricates**: special-cased (L = lateral,
//!   R = trill burst, J/W = glide stub, Ts/Ch = stop + fricative
//!   concatenation).

use crate::filter::FormantFilter;
use crate::source::{XorShift, glottal_pulse_train_v, white_noise};
use crate::voice::VoiceProfile;
use adam_phoneme::Phoneme;

/// Synthesise a consonant phoneme. Returns f32 PCM at the given
/// `sample_rate`, ~`duration_s` long. The voice profile scales
/// every spectral peak (vocal-tract-length normalisation) and
/// supplies F0 / jitter / breath for voiced segments.
pub fn synth_consonant(
    phoneme: Phoneme,
    duration_s: f32,
    sample_rate: u32,
    seed: u64,
    voice: &VoiceProfile,
) -> Vec<f32> {
    use Phoneme::*;
    let s = voice.formant_scale;
    match phoneme {
        // ─── Stops ─────────────────────────────────────────────
        P => stop(
            2000.0 * s,
            200.0,
            false,
            duration_s,
            sample_rate,
            seed,
            voice,
        ),
        B => stop(700.0 * s, 100.0, true, duration_s, sample_rate, seed, voice),
        T => stop(
            4000.0 * s,
            600.0,
            false,
            duration_s,
            sample_rate,
            seed,
            voice,
        ),
        D => stop(
            1700.0 * s,
            200.0,
            true,
            duration_s,
            sample_rate,
            seed,
            voice,
        ),
        K => stop(
            2500.0 * s,
            400.0,
            false,
            duration_s,
            sample_rate,
            seed,
            voice,
        ),
        G => stop(
            1500.0 * s,
            200.0,
            true,
            duration_s,
            sample_rate,
            seed,
            voice,
        ),
        Q => stop(
            1100.0 * s,
            250.0,
            false,
            duration_s,
            sample_rate,
            seed,
            voice,
        ),
        // ─── Fricatives ────────────────────────────────────────
        S => fricative(
            6500.0 * s,
            1500.0,
            false,
            duration_s,
            sample_rate,
            seed,
            voice,
        ),
        Sh => fricative(
            3500.0 * s,
            1200.0,
            false,
            duration_s,
            sample_rate,
            seed,
            voice,
        ),
        Z => fricative(
            6500.0 * s,
            1500.0,
            true,
            duration_s,
            sample_rate,
            seed,
            voice,
        ),
        Zh => fricative(
            3500.0 * s,
            1200.0,
            true,
            duration_s,
            sample_rate,
            seed,
            voice,
        ),
        F => fricative(
            4500.0 * s,
            1500.0,
            false,
            duration_s,
            sample_rate,
            seed,
            voice,
        ),
        V => fricative(
            3500.0 * s,
            1500.0,
            true,
            duration_s,
            sample_rate,
            seed,
            voice,
        ),
        X => fricative(
            1700.0 * s,
            800.0,
            false,
            duration_s,
            sample_rate,
            seed,
            voice,
        ),
        Gh => fricative(
            1500.0 * s,
            800.0,
            true,
            duration_s,
            sample_rate,
            seed,
            voice,
        ),
        H => fricative(
            800.0 * s,
            600.0,
            false,
            duration_s,
            sample_rate,
            seed,
            voice,
        ),
        Shch => fricative(
            3000.0 * s,
            1200.0,
            false,
            duration_s,
            sample_rate,
            seed,
            voice,
        ),
        // ─── Nasals ────────────────────────────────────────────
        M => nasal(280.0 * s, duration_s, sample_rate, voice, seed),
        N => nasal(280.0 * s, duration_s, sample_rate, voice, seed),
        Ng => nasal(250.0 * s, duration_s, sample_rate, voice, seed),
        // ─── Liquids / glides ──────────────────────────────────
        L => voiced_resonance(
            &[(360.0 * s, 80.0), (1300.0 * s, 100.0)],
            duration_s,
            sample_rate,
            voice,
            seed,
        ),
        R => trill(duration_s, sample_rate, voice, seed),
        J => voiced_resonance(
            &[(300.0 * s, 80.0), (2200.0 * s, 100.0)],
            duration_s,
            sample_rate,
            voice,
            seed,
        ),
        W => voiced_resonance(
            &[(350.0 * s, 80.0), (800.0 * s, 100.0)],
            duration_s,
            sample_rate,
            voice,
            seed,
        ),
        // ─── Affricates ────────────────────────────────────────
        Ts => {
            let mut out = stop(
                4000.0 * s,
                600.0,
                false,
                duration_s * 0.4,
                sample_rate,
                seed,
                voice,
            );
            out.extend(fricative(
                6500.0 * s,
                1500.0,
                false,
                duration_s * 0.6,
                sample_rate,
                seed.wrapping_add(1),
                voice,
            ));
            out
        }
        Ch => {
            let mut out = stop(
                4000.0 * s,
                600.0,
                false,
                duration_s * 0.4,
                sample_rate,
                seed,
                voice,
            );
            out.extend(fricative(
                3500.0 * s,
                1200.0,
                false,
                duration_s * 0.6,
                sample_rate,
                seed.wrapping_add(1),
                voice,
            ));
            out
        }
        _ => vec![0.0; (duration_s * sample_rate as f32) as usize],
    }
}

/// Voiced glottal source carrying the speaker's F0 / jitter /
/// breath. Used wherever a consonant has a voiced component.
fn voiced_source(duration_s: f32, sample_rate: u32, voice: &VoiceProfile, seed: u64) -> Vec<f32> {
    glottal_pulse_train_v(
        voice.f0_hz,
        duration_s,
        sample_rate,
        voice.jitter,
        voice.breath,
        seed,
    )
}

/// Stop: short silence + burst + (optional) voiced tail.
fn stop(
    burst_freq: f32,
    burst_bw: f32,
    voiced: bool,
    duration_s: f32,
    sample_rate: u32,
    seed: u64,
    voice: &VoiceProfile,
) -> Vec<f32> {
    let total_n = (duration_s * sample_rate as f32) as usize;
    let silence_n = (0.04 * sample_rate as f32) as usize; // 40 ms hold
    let mut excitation = vec![0.0_f32; total_n];
    if silence_n + 1 < total_n {
        excitation[silence_n] = 1.0; // impulse burst
        if voiced {
            // Low-amplitude voiced tail after the burst (≈80 ms
            // glottal flow), to give voicing during release.
            let tail_dur = ((total_n - silence_n) as f32 / sample_rate as f32).max(0.0);
            let tail = voiced_source(tail_dur, sample_rate, voice, seed);
            for (i, &v) in tail.iter().enumerate() {
                let idx = silence_n + i;
                if idx >= total_n {
                    break;
                }
                excitation[idx] += v * 0.3;
            }
        }
    }
    let mut filt = FormantFilter::new(&[(burst_freq, burst_bw)], sample_rate);
    let mut out = filt.process(&excitation);
    // Tiny noise floor so CMVN doesn't blow up on near-silence.
    add_noise_floor(&mut out, seed);
    out
}

/// Fricative: band-passed white noise, optionally mixed with a
/// low-amplitude glottal source for voiced variants.
fn fricative(
    centre_hz: f32,
    bw: f32,
    voiced: bool,
    duration_s: f32,
    sample_rate: u32,
    seed: u64,
    voice: &VoiceProfile,
) -> Vec<f32> {
    let noise = white_noise(duration_s, sample_rate, seed);
    let mut excitation = noise;
    if voiced {
        let pulse = voiced_source(duration_s, sample_rate, voice, seed);
        for (e, p) in excitation.iter_mut().zip(pulse.iter()) {
            *e += p * 0.3;
        }
    }
    let mut filt = FormantFilter::new(&[(centre_hz, bw)], sample_rate);
    filt.process(&excitation)
}

/// Nasal: voiced glottal source through one low resonance + a
/// gentle low-pass via the resonator's natural high-frequency
/// roll-off.
fn nasal(f1: f32, duration_s: f32, sample_rate: u32, voice: &VoiceProfile, seed: u64) -> Vec<f32> {
    let pulse = voiced_source(duration_s, sample_rate, voice, seed);
    let s = voice.formant_scale;
    let mut filt = FormantFilter::new(&[(f1, 60.0), (1100.0 * s, 200.0)], sample_rate);
    filt.process(&pulse)
}

/// Voiced segment with a fixed resonance pattern (used for
/// L / J / W). Formants already scaled by caller.
fn voiced_resonance(
    formants: &[(f32, f32)],
    duration_s: f32,
    sample_rate: u32,
    voice: &VoiceProfile,
    seed: u64,
) -> Vec<f32> {
    let pulse = voiced_source(duration_s, sample_rate, voice, seed);
    let mut filt = FormantFilter::new(formants, sample_rate);
    filt.process(&pulse)
}

/// R trill: voiced source amplitude-modulated at trill rate
/// (~25 Hz) so the recogniser sees a buzzing energy pattern.
fn trill(duration_s: f32, sample_rate: u32, voice: &VoiceProfile, seed: u64) -> Vec<f32> {
    let pulse = voiced_source(duration_s, sample_rate, voice, seed);
    let trill_hz = 25.0;
    let modulated: Vec<f32> = pulse
        .iter()
        .enumerate()
        .map(|(i, &x)| {
            let t = i as f32 / sample_rate as f32;
            let envelope = 0.5 + 0.5 * (2.0 * std::f32::consts::PI * trill_hz * t).sin();
            x * envelope
        })
        .collect();
    let s = voice.formant_scale;
    let mut filt = FormantFilter::new(&[(400.0 * s, 80.0), (1400.0 * s, 150.0)], sample_rate);
    filt.process(&modulated)
}

/// Add ~-60 dB white noise so silent regions have non-zero
/// variance — keeps CMVN well-behaved.
fn add_noise_floor(samples: &mut [f32], seed: u64) {
    let mut rng = XorShift::new(seed.wrapping_add(0xDEAD));
    for s in samples.iter_mut() {
        *s += rng.next_f32() * 0.001;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every consonant in the inventory synthesises to a
    /// non-empty, finite buffer with non-trivial energy at the
    /// raw (un-normalised) stage. The amplitude is small until
    /// the top-level `synth_phoneme` normaliser scales it up;
    /// the floor here is just "not numerical zero".
    #[test]
    fn all_consonants_synthesise_audibly() {
        let voice = crate::voice::BARITONE;
        for p in Phoneme::ALL {
            if !p.is_consonant() {
                continue;
            }
            let pcm = synth_consonant(*p, 0.15, 16_000, 42, &voice);
            assert!(!pcm.is_empty(), "{p:?} produced empty buffer");
            let rms = (pcm.iter().map(|x| x * x).sum::<f32>() / pcm.len() as f32).sqrt();
            assert!(rms > 1e-8, "{p:?} effectively zero: rms={rms}");
            assert!(
                pcm.iter().all(|x| x.is_finite()),
                "{p:?} produced non-finite samples"
            );
        }
    }
}
