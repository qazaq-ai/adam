// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Resample mono audio to 16 kHz via `rubato`.

use adam_audio::PcmSamples;
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};

/// Resample mono `samples` to 16 kHz. The input must already
/// be mono (single channel).
pub fn to_16khz(samples: &PcmSamples) -> Result<PcmSamples, Box<dyn std::error::Error>> {
    assert_eq!(samples.channels, 1, "to_16khz expects mono input");
    let target_rate = 16_000_u32;
    if samples.sample_rate == target_rate {
        return Ok(samples.clone());
    }

    let ratio = target_rate as f64 / samples.sample_rate as f64;
    let params = SincInterpolationParameters {
        sinc_len: 128,
        f_cutoff: 0.95,
        oversampling_factor: 128,
        interpolation: SincInterpolationType::Cubic,
        window: WindowFunction::BlackmanHarris2,
    };
    // Process whole input as one chunk for simplicity (Wikimedia
    // pronunciations are typically < 5 s, well under typical
    // memory limits).
    let mut resampler = SincFixedIn::<f32>::new(ratio, 1.0, params, samples.data.len(), 1)?;

    let input = vec![samples.data.clone()];
    let output = resampler.process(&input, None)?;
    let data = output.into_iter().next().unwrap_or_default();

    Ok(PcmSamples {
        sample_rate: target_rate,
        channels: 1,
        data,
    })
}
