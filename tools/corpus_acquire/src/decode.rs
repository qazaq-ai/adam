// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Decode any symphonia-supported audio file into a
//! [`PcmSamples`] buffer at its native sample rate / channel
//! count.

use adam_audio::PcmSamples;
use std::fs::File;
use std::path::Path;
use symphonia::core::audio::AudioBufferRef;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Decode an audio file (OGG/Vorbis, WAV, MP3, FLAC, etc.)
/// into f32 PCM. Multi-channel input is returned interleaved
/// in [`PcmSamples`]; the caller can downmix to mono via
/// [`PcmSamples::to_mono`].
pub fn decode_file(path: &Path) -> Result<PcmSamples, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        hint.with_extension(ext);
    }
    decode_mss(mss, hint)
}

/// Decode raw bytes (e.g., from a streamed tar entry) without
/// ever touching the filesystem. `extension_hint` may be "wav",
/// "ogg", "mp3", etc.
pub fn decode_bytes(
    bytes: Vec<u8>,
    extension_hint: &str,
) -> Result<PcmSamples, Box<dyn std::error::Error>> {
    use std::io::Cursor;
    let cursor = Cursor::new(bytes);
    let mss = MediaSourceStream::new(Box::new(cursor), Default::default());
    let mut hint = Hint::new();
    hint.with_extension(extension_hint);
    decode_mss(mss, hint)
}

fn decode_mss(
    mss: MediaSourceStream,
    hint: Hint,
) -> Result<PcmSamples, Box<dyn std::error::Error>> {
    let probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;
    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or("no decodable audio track")?;
    let track_id = track.id;
    let codec_params = track.codec_params.clone();

    let mut decoder =
        symphonia::default::get_codecs().make(&codec_params, &DecoderOptions::default())?;

    let mut sample_rate: u32 = codec_params.sample_rate.unwrap_or(16_000);
    let mut channels: u16 = codec_params.channels.map(|c| c.count() as u16).unwrap_or(1);
    let mut samples: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(SymError::ResetRequired) => break,
            Err(e) => return Err(Box::new(e)),
        };
        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(SymError::DecodeError(_)) => continue, // skip recoverable
            Err(e) => return Err(Box::new(e)),
        };
        // Update metadata from the decoded buffer (more
        // reliable than codec_params for some formats).
        let spec = decoded.spec();
        sample_rate = spec.rate;
        channels = spec.channels.count() as u16;

        append_f32(&decoded, &mut samples);
    }

    Ok(PcmSamples {
        sample_rate,
        channels,
        data: samples,
    })
}

/// Append a symphonia decoded buffer to a flat interleaved f32 vec.
fn append_f32(buf: &AudioBufferRef<'_>, out: &mut Vec<f32>) {
    use symphonia::core::conv::FromSample;
    match buf {
        AudioBufferRef::F32(b) => append_planar_to_interleaved(b, out, |x: f32| x),
        AudioBufferRef::F64(b) => append_planar_to_interleaved(b, out, f32::from_sample),
        AudioBufferRef::S32(b) => append_planar_to_interleaved(b, out, f32::from_sample),
        AudioBufferRef::S16(b) => append_planar_to_interleaved(b, out, f32::from_sample),
        AudioBufferRef::S8(b) => append_planar_to_interleaved(b, out, f32::from_sample),
        AudioBufferRef::U8(b) => append_planar_to_interleaved(b, out, f32::from_sample),
        AudioBufferRef::U16(b) => append_planar_to_interleaved(b, out, f32::from_sample),
        AudioBufferRef::U32(b) => append_planar_to_interleaved(b, out, f32::from_sample),
        AudioBufferRef::U24(b) => append_planar_to_interleaved(b, out, f32::from_sample),
        AudioBufferRef::S24(b) => append_planar_to_interleaved(b, out, f32::from_sample),
    }
}

fn append_planar_to_interleaved<T>(
    buf: &symphonia::core::audio::AudioBuffer<T>,
    out: &mut Vec<f32>,
    to_f32: impl Fn(T) -> f32,
) where
    T: Copy + symphonia::core::sample::Sample,
{
    use symphonia::core::audio::Signal;
    let frames = buf.frames();
    let channels = buf.spec().channels.count();
    out.reserve(frames * channels);
    for f in 0..frames {
        for ch in 0..channels {
            let s = buf.chan(ch)[f];
            out.push(to_f32(s));
        }
    }
}
