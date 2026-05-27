// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `corpus_acquire` — disk-constrained Kazakh-audio acquisition.
//!
//! Three subcommands:
//!
//! 1. **`pull`** — download + decode + extract + delete one
//!    URL. The original disk-bounded primitive.
//! 2. **`discover`** — query Wikimedia Commons category API for
//!    Kazakh pronunciation files, write a curatable
//!    `sources.toml`.
//! 3. **`batch`** — read a `sources.toml` and process every
//!    entry through the `pull` pipeline. Skips entries already
//!    present in the manifest (idempotent re-runs).
//!
//! Per the user directive (2026-05-26):
//!
//! > «Скачай одну, переработай её, удали скаченный файл,
//! >  перейди к следующей. Не засоряя диск.»
//!
//! Each acquisition cycle: download → decode → resample to
//! 16 kHz mono → extract MFCC → persist WAV+MFCC+manifest
//! line → delete original.

use clap::{Parser, Subcommand};
use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};

mod decode;
mod manifest;
mod resample;

use manifest::{ManifestEntry, append_manifest};

// ─── CLI ──────────────────────────────────────────────────────

#[derive(Debug, Parser)]
#[command(
    name = "corpus_acquire",
    about = "Disk-bounded Kazakh-audio acquisition + extraction pipeline.",
    version
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Download one URL, decode, extract MFCC, persist
    /// (audio+mfcc+manifest), delete original.
    Pull(PullArgs),
    /// Query Wikimedia Commons category for files, write a
    /// curatable `sources.toml`.
    Discover(DiscoverArgs),
    /// Read a sources.toml and process every entry through the
    /// pull pipeline. Idempotent: existing manifest labels are
    /// skipped.
    Batch(BatchArgs),
}

#[derive(Debug, clap::Args)]
struct PullArgs {
    /// Direct URL to download.
    #[arg(long)]
    url: String,
    /// Human-readable label (filename root for derived files).
    #[arg(long)]
    label: String,
    /// Cyrillic transcript of the spoken content.
    #[arg(long)]
    transcript: String,
    /// Speaker gender: "male" / "female" / "mixed" / "unknown".
    #[arg(long, default_value = "unknown")]
    gender: String,
    /// Provenance class.
    #[arg(long, default_value = "wikimedia")]
    source_class: String,
    /// Output directory.
    #[arg(long, default_value = "data/v6_3_phoneme_bank")]
    out_dir: PathBuf,
}

#[derive(Debug, clap::Args)]
struct DiscoverArgs {
    /// Wikimedia Commons category title (without "Category:"
    /// prefix; we prepend it).
    #[arg(long, default_value = "Kazakh pronunciation")]
    category: String,
    /// Output sources.toml file.
    #[arg(long, default_value = "data/v6_3_phoneme_bank/sources.toml")]
    output: PathBuf,
    /// Maximum number of files to include (the API paginates
    /// 500 at a time; this caps total).
    #[arg(long, default_value = "500")]
    limit: usize,
}

#[derive(Debug, clap::Args)]
struct BatchArgs {
    /// Sources file produced by `discover` (or hand-curated).
    #[arg(long, default_value = "data/v6_3_phoneme_bank/sources.toml")]
    sources: PathBuf,
    /// Output directory.
    #[arg(long, default_value = "data/v6_3_phoneme_bank")]
    out_dir: PathBuf,
    /// Stop after this many successful acquisitions (0 = no
    /// limit). Useful for incremental runs that respect disk
    /// pressure.
    #[arg(long, default_value = "0")]
    max: usize,
}

// ─── main ──────────────────────────────────────────────────────

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match Cli::parse().cmd {
        Cmd::Pull(args) => run_pull(args),
        Cmd::Discover(args) => run_discover(args),
        Cmd::Batch(args) => run_batch(args),
    }
}

// ─── pull ──────────────────────────────────────────────────────

fn run_pull(args: PullArgs) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(args.out_dir.join("audio"))?;
    fs::create_dir_all(args.out_dir.join("mfcc"))?;
    fs::create_dir_all(args.out_dir.join("tmp"))?;
    pull_one(
        &args.url,
        &args.label,
        &args.transcript,
        &args.gender,
        &args.source_class,
        &args.out_dir,
    )
}

fn pull_one(
    url: &str,
    label: &str,
    transcript: &str,
    gender: &str,
    source_class: &str,
    out_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let tmp_path = out_dir.join("tmp").join(format!("{label}.dl"));
    let wav_path = out_dir.join("audio").join(format!("{label}.wav"));
    let mfcc_path = out_dir.join("mfcc").join(format!("{label}.bin"));
    let manifest_path = out_dir.join("MANIFEST.jsonl");

    println!("[acquire] {url} → {label}");

    // 1. Download.
    let dl_start = std::time::Instant::now();
    let original_bytes = download_to(url, &tmp_path)?;
    println!(
        "[acquire]   downloaded {} bytes in {:.2}s",
        original_bytes,
        dl_start.elapsed().as_secs_f32(),
    );

    // 2 + 3. Decode + resample.
    let pcm = decode::decode_file(&tmp_path)?;
    let pcm_mono = if pcm.channels > 1 { pcm.to_mono() } else { pcm };
    let pcm_16k = if pcm_mono.sample_rate != 16_000 {
        resample::to_16khz(&pcm_mono)?
    } else {
        pcm_mono
    };

    // 4. MFCC.
    let mfcc_seq = adam_audio::mfcc::mfcc(
        &pcm_16k.data,
        pcm_16k.sample_rate,
        &adam_audio::mfcc::MfccConfig::default(),
    );

    // 5. Persist.
    adam_audio::wav::write_wav(&wav_path, &pcm_16k)?;
    write_mfcc_binary(&mfcc_path, &mfcc_seq)?;
    let wav_size = fs::metadata(&wav_path)?.len();
    let mfcc_size = fs::metadata(&mfcc_path)?.len();

    // 6. Manifest.
    let entry = ManifestEntry {
        label: label.to_string(),
        source_url: url.to_string(),
        transcript: transcript.to_string(),
        gender: gender.to_string(),
        source_class: source_class.to_string(),
        original_bytes,
        duration_s: pcm_16k.duration_s(),
        wav_path: wav_path
            .strip_prefix(out_dir)
            .unwrap_or(&wav_path)
            .to_string_lossy()
            .to_string(),
        wav_bytes: wav_size,
        mfcc_path: mfcc_path
            .strip_prefix(out_dir)
            .unwrap_or(&mfcc_path)
            .to_string_lossy()
            .to_string(),
        mfcc_frames: mfcc_seq.num_frames(),
        mfcc_bytes: mfcc_size,
        collected_at: chrono_date(),
        used_in_bank: false,
    };
    append_manifest(&manifest_path, &entry)?;

    // 7. Delete original.
    fs::remove_file(&tmp_path)?;

    println!(
        "[acquire]   persisted {label} ({:.1} KB derived; source was {:.1} KB)",
        (wav_size + mfcc_size) as f32 / 1024.0,
        original_bytes as f32 / 1024.0,
    );
    Ok(())
}

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

// ─── discover ─────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
struct CmResponse {
    query: CmQuery,
    #[serde(default)]
    #[serde(rename = "continue")]
    cont: Option<CmContinue>,
}
#[derive(Debug, serde::Deserialize)]
struct CmQuery {
    categorymembers: Vec<CmMember>,
}
#[derive(Debug, serde::Deserialize)]
struct CmMember {
    title: String,
    ns: i32,
}
#[derive(Debug, serde::Deserialize)]
struct CmContinue {
    cmcontinue: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct IiResponse {
    query: IiQuery,
}
#[derive(Debug, serde::Deserialize)]
struct IiQuery {
    pages: std::collections::HashMap<String, IiPage>,
}
#[derive(Debug, serde::Deserialize)]
struct IiPage {
    title: String,
    #[serde(default)]
    imageinfo: Vec<IiInfo>,
}
#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)] // size/duration kept for future curation pass
struct IiInfo {
    url: String,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    duration: f32,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct SourcesFile {
    source: Vec<SourceEntry>,
}
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct SourceEntry {
    label: String,
    url: String,
    transcript: String,
    #[serde(default = "default_gender")]
    gender: String,
    #[serde(default = "default_class")]
    source_class: String,
}
fn default_gender() -> String {
    "unknown".into()
}
fn default_class() -> String {
    "wikimedia".into()
}

fn run_discover(args: DiscoverArgs) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("adam-corpus-acquire/0.1 (https://github.com/qazaq-ai/adam)")
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    let cat_title = format!("Category:{}", args.category);
    println!("[discover] enumerating files in «{cat_title}»");

    // 1. Walk category members (paginated) to collect file titles.
    let mut file_titles: Vec<String> = Vec::new();
    let mut cmcontinue: Option<String> = None;
    while file_titles.len() < args.limit {
        let mut url = format!(
            "https://commons.wikimedia.org/w/api.php?action=query&list=categorymembers&\
             cmtitle={}&cmtype=file&cmlimit=500&format=json",
            urlencode(&cat_title),
        );
        if let Some(c) = &cmcontinue {
            url.push_str(&format!("&cmcontinue={}", urlencode(c)));
        }
        let resp: CmResponse = client.get(&url).send()?.error_for_status()?.json()?;
        for m in resp.query.categorymembers {
            if m.ns == 6 {
                // File namespace.
                file_titles.push(m.title);
            }
        }
        match resp.cont.and_then(|c| c.cmcontinue) {
            Some(c) => cmcontinue = Some(c),
            None => break,
        }
    }
    file_titles.truncate(args.limit);
    println!("[discover] {} files found", file_titles.len());

    // 2. Look up imageinfo (URL + size + duration) for each file.
    // The API accepts up to 50 titles per call.
    let mut entries: Vec<SourceEntry> = Vec::new();
    for chunk in file_titles.chunks(50) {
        let titles = chunk.join("|");
        let url = format!(
            "https://commons.wikimedia.org/w/api.php?action=query&titles={}\
             &prop=imageinfo&iiprop=url|size|duration&format=json",
            urlencode(&titles),
        );
        let resp: IiResponse = client.get(&url).send()?.error_for_status()?.json()?;
        for page in resp.query.pages.values() {
            if let Some(info) = page.imageinfo.first() {
                let label = label_from_title(&page.title);
                let transcript = transcript_placeholder(&page.title);
                entries.push(SourceEntry {
                    label,
                    url: info.url.clone(),
                    transcript,
                    gender: "unknown".into(),
                    source_class: "wikimedia".into(),
                });
            }
        }
    }

    // 3. Sort by label for stable output.
    entries.sort_by(|a, b| a.label.cmp(&b.label));

    // 4. Write sources.toml.
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    let serialised = toml::to_string_pretty(&SourcesFile { source: entries })?;
    fs::write(&args.output, serialised)?;
    println!("[discover] wrote {}", args.output.display());
    Ok(())
}

fn urlencode(s: &str) -> String {
    // Minimal percent-encoding for query-string values. Avoids
    // pulling another dep just for this.
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Extract a stable filename-root label from a Wikimedia title
/// like `File:Kk-kazakh.ogg` → `kk_kazakh`.
fn label_from_title(title: &str) -> String {
    let no_ns = title.strip_prefix("File:").unwrap_or(title);
    let no_ext = no_ns
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(no_ns);
    no_ext
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

/// Best-effort transcript placeholder. Without an explicit
/// transcript field, we put the filename stem here; the user
/// curates the sources.toml before running `batch`.
fn transcript_placeholder(title: &str) -> String {
    let no_ns = title.strip_prefix("File:").unwrap_or(title);
    no_ns
        .rsplit_once('.')
        .map(|(s, _)| s)
        .unwrap_or(no_ns)
        .strip_prefix("Kk-")
        .unwrap_or(no_ns)
        .to_string()
}

// ─── batch ────────────────────────────────────────────────────

fn run_batch(args: BatchArgs) -> Result<(), Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(&args.sources)?;
    let parsed: SourcesFile = toml::from_str(&contents)?;
    println!(
        "[batch] {} sources in {}",
        parsed.source.len(),
        args.sources.display()
    );

    fs::create_dir_all(args.out_dir.join("audio"))?;
    fs::create_dir_all(args.out_dir.join("mfcc"))?;
    fs::create_dir_all(args.out_dir.join("tmp"))?;

    let manifest_path = args.out_dir.join("MANIFEST.jsonl");
    let acquired_labels = read_manifest_labels(&manifest_path)?;
    println!(
        "[batch] {} labels already in manifest, will skip",
        acquired_labels.len()
    );

    let mut ok = 0_usize;
    let mut skipped = 0_usize;
    let mut failed = 0_usize;
    for src in &parsed.source {
        if acquired_labels.contains(&src.label) {
            skipped += 1;
            continue;
        }
        match pull_one(
            &src.url,
            &src.label,
            &src.transcript,
            &src.gender,
            &src.source_class,
            &args.out_dir,
        ) {
            Ok(()) => {
                ok += 1;
                if args.max > 0 && ok >= args.max {
                    println!("[batch] reached --max {} — stopping", args.max);
                    break;
                }
            }
            Err(e) => {
                eprintln!("[batch] FAILED «{}»: {}", src.label, e);
                failed += 1;
            }
        }
    }
    println!("[batch] done: {ok} acquired, {skipped} skipped, {failed} failed");
    Ok(())
}

fn read_manifest_labels(path: &Path) -> std::io::Result<HashSet<String>> {
    let mut out = HashSet::new();
    if !path.exists() {
        return Ok(out);
    }
    let file = fs::File::open(path)?;
    for line in BufReader::new(file).lines() {
        let line = line?;
        if let Ok(entry) = serde_json::from_str::<ManifestEntry>(&line) {
            out.insert(entry.label);
        }
    }
    Ok(out)
}

// ─── date util ────────────────────────────────────────────────

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
