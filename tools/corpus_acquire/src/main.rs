// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `corpus_acquire` — disk-constrained Kazakh-audio acquisition.
//!
//! Per the user directive (2026-05-26):
//!
//! > «Скачай одну, переработай её, а потом из-за отсутствия
//! >  надобности удаляй этот скаченный файл. … В итоге в конце
//! >  может остаться совсем небольшой, но чистый от мусора
//! >  файл.»
//!
//! Pipeline for each source:
//! 1. **Download** the original audio (any format symphonia
//!    can decode: OGG/Vorbis, WAV, MP3, FLAC, AAC, MP4).
//! 2. **Decode** to f32 PCM at the source's native sample rate.
//! 3. **Resample** to 16 kHz mono.
//! 4. **Extract MFCC** via [`adam_audio::mfcc`].
//! 5. **Persist** in `data/v6_3_phoneme_bank/`:
//!    - `audio/<label>.wav` — 16 kHz mono curated WAV (≪ original)
//!    - `mfcc/<label>.bin` — MFCC sequence in binary form
//! 6. **Update** `MANIFEST.jsonl` with source URL, label,
//!    durations, sizes.
//! 7. **Delete** the downloaded original.
//!
//! The persistent artefacts are 10–100× smaller than the
//! source: a typical Wikimedia OGG of one Kazakh word at
//! 44 kHz weighs ~80 KB; the derived 16 kHz mono WAV is
//! ~30 KB and the MFCC ~5 KB.

use clap::Parser;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

mod decode;
mod manifest;
mod resample;

use manifest::{ManifestEntry, append_manifest};

#[derive(Debug, Parser)]
#[command(
    name = "corpus_acquire",
    about = "Download + decode + extract one Kazakh audio source, then delete the original.",
    version
)]
struct Args {
    /// Direct URL to download.
    #[arg(long)]
    url: String,
    /// Human-readable label (e.g. "kazakhstan" or "salam"). Used as
    /// the filename root for derived artefacts.
    #[arg(long)]
    label: String,
    /// Cyrillic transcript (the spoken word / phrase). Goes into
    /// the manifest entry — required for later forced alignment.
    #[arg(long)]
    transcript: String,
    /// Speaker gender hint, if known: "male" / "female" / "mixed".
    /// Defaults to "unknown".
    #[arg(long, default_value = "unknown")]
    gender: String,
    /// Provenance tag (e.g. "wikimedia", "archive-org", "kazneb",
    /// "common-voice", "self").
    #[arg(long, default_value = "wikimedia")]
    source_class: String,
    /// Output directory for derived artefacts (audio + mfcc +
    /// manifest). Defaults to `data/v6_3_phoneme_bank`.
    #[arg(long, default_value = "data/v6_3_phoneme_bank")]
    out_dir: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    fs::create_dir_all(args.out_dir.join("audio"))?;
    fs::create_dir_all(args.out_dir.join("mfcc"))?;
    fs::create_dir_all(args.out_dir.join("tmp"))?;

    let tmp_path = args.out_dir.join("tmp").join(format!("{}.dl", args.label));
    let wav_path = args
        .out_dir
        .join("audio")
        .join(format!("{}.wav", args.label));
    let mfcc_path = args
        .out_dir
        .join("mfcc")
        .join(format!("{}.bin", args.label));
    let manifest_path = args.out_dir.join("MANIFEST.jsonl");

    println!("[acquire] {} → {}", args.url, args.label);

    // 1. Download.
    let dl_start = std::time::Instant::now();
    let original_bytes = download_to(&args.url, &tmp_path)?;
    println!(
        "[acquire] downloaded {} bytes in {:.2}s",
        original_bytes,
        dl_start.elapsed().as_secs_f32(),
    );

    // 2 + 3. Decode + resample to 16 kHz mono.
    let pcm = decode::decode_file(&tmp_path)?;
    println!(
        "[acquire] decoded: {:.2} s @ {} Hz, {} channels",
        pcm.duration_s(),
        pcm.sample_rate,
        pcm.channels,
    );
    let pcm_mono = if pcm.channels > 1 { pcm.to_mono() } else { pcm };
    let pcm_16k = if pcm_mono.sample_rate != 16_000 {
        resample::to_16khz(&pcm_mono)?
    } else {
        pcm_mono
    };

    // 4. MFCC extraction.
    let mfcc_seq = adam_audio::mfcc::mfcc(
        &pcm_16k.data,
        pcm_16k.sample_rate,
        &adam_audio::mfcc::MfccConfig::default(),
    );

    // 5. Persist curated WAV + MFCC.
    adam_audio::wav::write_wav(&wav_path, &pcm_16k)?;
    write_mfcc_binary(&mfcc_path, &mfcc_seq)?;

    let wav_size = fs::metadata(&wav_path)?.len();
    let mfcc_size = fs::metadata(&mfcc_path)?.len();

    // 6. Manifest entry.
    let entry = ManifestEntry {
        label: args.label.clone(),
        source_url: args.url.clone(),
        transcript: args.transcript.clone(),
        gender: args.gender.clone(),
        source_class: args.source_class.clone(),
        original_bytes,
        duration_s: pcm_16k.duration_s(),
        wav_path: wav_path
            .strip_prefix(&args.out_dir)
            .unwrap_or(&wav_path)
            .to_string_lossy()
            .to_string(),
        wav_bytes: wav_size,
        mfcc_path: mfcc_path
            .strip_prefix(&args.out_dir)
            .unwrap_or(&mfcc_path)
            .to_string_lossy()
            .to_string(),
        mfcc_frames: mfcc_seq.num_frames(),
        mfcc_bytes: mfcc_size,
        collected_at: chrono_date(),
        used_in_bank: false,
    };
    append_manifest(&manifest_path, &entry)?;

    // 7. Delete the downloaded original.
    fs::remove_file(&tmp_path)?;

    // Report disk footprint. We don't claim compression — for
    // lossy-encoded sources (OGG, MP3) the persisted PCM WAV is
    // typically larger than the original, and that's fine: the
    // point is the persistent footprint is **bounded** (~50 KB
    // per Kazakh word) regardless of how big the corpus grows.
    let persisted = wav_size + mfcc_size;
    println!(
        "[acquire] persisted {} ({} B WAV + {} B MFCC = {:.1} KB total; source was {:.1} KB)",
        args.label,
        wav_size,
        mfcc_size,
        persisted as f32 / 1024.0,
        original_bytes as f32 / 1024.0,
    );
    println!("[acquire] manifest: {}", manifest_path.display());
    Ok(())
}

/// Download a URL to a file. Returns the number of bytes written.
fn download_to(url: &str, path: &Path) -> Result<u64, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("adam-corpus-acquire/0.1 (https://github.com/qazaq-ai/adam)")
        .timeout(std::time::Duration::from_secs(120))
        .build()?;
    let mut resp = client.get(url).send()?.error_for_status()?;
    let mut out = fs::File::create(path)?;
    let mut buf = [0u8; 8192];
    let mut total = 0_u64;
    loop {
        let n = resp.read(&mut buf)?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n])?;
        total += n as u64;
    }
    Ok(total)
}

/// Write an MFCC sequence to a simple binary file:
/// `[4 bytes magic "MFCC"][1 byte version=1][4 bytes n_frames LE]
///  [4 bytes n_mfcc LE][4 bytes sample_rate LE][4 bytes hop LE]
///  [f32 data...]`.
fn write_mfcc_binary(
    path: &Path,
    seq: &adam_audio::mfcc::MfccSequence,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut out = fs::File::create(path)?;
    out.write_all(b"MFCC")?;
    out.write_all(&[0x01])?;
    out.write_all(&(seq.num_frames() as u32).to_le_bytes())?;
    out.write_all(&(seq.dim() as u32).to_le_bytes())?;
    out.write_all(&seq.sample_rate.to_le_bytes())?;
    out.write_all(&(seq.hop_length as u32).to_le_bytes())?;
    for frame in &seq.frames {
        for &c in frame {
            out.write_all(&c.to_le_bytes())?;
        }
    }
    Ok(())
}

/// Today's date in ISO format.
fn chrono_date() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = secs / 86_400;
    let (y, m, d) = days_to_ymd(days as i64);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Convert days-since-1970 to (year, month, day). Naive but
/// correct for years 1970-9999.
fn days_to_ymd(mut days: i64) -> (i32, u32, u32) {
    days += 719468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let doe = (days - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
}
