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
    /// Sync the curated `sources.toml` transcripts back into
    /// `MANIFEST.jsonl`. Used after manually curating
    /// discover-output transcripts (filename stubs → Cyrillic).
    FixManifestTranscripts(FixArgs),
    /// Build a real-data phoneme bank from the manifest.
    /// Equipartitions each word's MFCC across its phoneme
    /// sequence; per phoneme, keeps the longest collected
    /// chunk as the template. Writes to `templates.bin`.
    BuildBank(BuildBankArgs),
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

#[derive(Debug, clap::Args)]
struct FixArgs {
    /// Curated sources.toml (source of truth for transcripts).
    #[arg(long, default_value = "data/v6_3_phoneme_bank/sources.toml")]
    sources: PathBuf,
    /// Manifest file to update in place.
    #[arg(long, default_value = "data/v6_3_phoneme_bank/MANIFEST.jsonl")]
    manifest: PathBuf,
}

#[derive(Debug, clap::Args)]
struct BuildBankArgs {
    /// Manifest with (label, transcript, mfcc_path) entries.
    #[arg(long, default_value = "data/v6_3_phoneme_bank/MANIFEST.jsonl")]
    manifest: PathBuf,
    /// Root directory for the manifest's mfcc_path entries.
    #[arg(long, default_value = "data/v6_3_phoneme_bank")]
    bank_dir: PathBuf,
    /// Output template-bank file.
    #[arg(long, default_value = "data/v6_3_phoneme_bank/templates.bin")]
    output: PathBuf,
    /// Number of bootstrap iterations. Iteration 0 = naive
    /// equipartition; subsequent iterations DTW-realign each
    /// word against the current bank to refine per-phoneme
    /// chunk boundaries. Default 2 = one equipartition pass +
    /// one DTW pass.
    #[arg(long, default_value = "2")]
    iterations: usize,
}

// ─── main ──────────────────────────────────────────────────────

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match Cli::parse().cmd {
        Cmd::Pull(args) => run_pull(args),
        Cmd::Discover(args) => run_discover(args),
        Cmd::Batch(args) => run_batch(args),
        Cmd::FixManifestTranscripts(args) => run_fix_transcripts(args),
        Cmd::BuildBank(args) => run_build_bank(args),
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

// ─── fix-manifest-transcripts ─────────────────────────────────

fn run_fix_transcripts(args: FixArgs) -> Result<(), Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(&args.sources)?;
    let parsed: SourcesFile = toml::from_str(&contents)?;
    let by_label: std::collections::HashMap<String, String> = parsed
        .source
        .into_iter()
        .map(|s| (s.label, s.transcript))
        .collect();

    if !args.manifest.exists() {
        return Err(format!("manifest not found: {}", args.manifest.display()).into());
    }

    let file = fs::File::open(&args.manifest)?;
    let mut updated: Vec<ManifestEntry> = Vec::new();
    let mut changed = 0_usize;
    let mut unchanged = 0_usize;
    let mut unmatched = 0_usize;
    for line in BufReader::new(file).lines() {
        let line = line?;
        let mut entry: ManifestEntry = match serde_json::from_str(&line) {
            Ok(e) => e,
            Err(_) => {
                eprintln!("[fix] skipping malformed line");
                continue;
            }
        };
        if let Some(new_transcript) = by_label.get(&entry.label) {
            if &entry.transcript == new_transcript {
                unchanged += 1;
            } else {
                println!(
                    "[fix] {}: «{}» → «{}»",
                    entry.label, entry.transcript, new_transcript
                );
                entry.transcript = new_transcript.clone();
                changed += 1;
            }
        } else {
            eprintln!(
                "[fix] {} not found in sources.toml — leaving transcript unchanged",
                entry.label
            );
            unmatched += 1;
        }
        updated.push(entry);
    }

    // Rewrite manifest atomically.
    let tmp_path = args.manifest.with_extension("jsonl.tmp");
    {
        let mut tmp = fs::File::create(&tmp_path)?;
        for e in &updated {
            writeln!(tmp, "{}", serde_json::to_string(e)?)?;
        }
    }
    fs::rename(&tmp_path, &args.manifest)?;
    println!(
        "[fix] done: {changed} updated, {unchanged} already current, {unmatched} no source entry"
    );
    Ok(())
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

// ─── build-bank ───────────────────────────────────────────────

/// One usable (audio MFCC, phoneme sequence) pair preloaded for
/// reuse across bootstrap iterations.
#[allow(dead_code)] // label is for diagnostics
struct BankSource {
    label: String,
    phonemes: Vec<adam_phoneme::Phoneme>,
    mfcc: adam_audio::mfcc::MfccSequence,
}

fn run_build_bank(args: BuildBankArgs) -> Result<(), Box<dyn std::error::Error>> {
    use adam_phoneme::Phoneme;
    use adam_phoneme::cyrillic::cyrillic_to_phonemes;

    if !args.manifest.exists() {
        return Err(format!("manifest not found: {}", args.manifest.display()).into());
    }

    // Pre-load all usable entries so each iteration can re-use
    // them without re-reading + re-parsing. Multi-word entries
    // are split via energy-based word segmentation; each word
    // becomes its own source.
    let mut sources: Vec<BankSource> = Vec::new();
    let mut skipped = 0_usize;
    let file = fs::File::open(&args.manifest)?;
    for line in BufReader::new(file).lines() {
        let entry: ManifestEntry = match serde_json::from_str(&line?) {
            Ok(e) => e,
            Err(_) => continue,
        };

        let words: Vec<&str> = entry.transcript.split_whitespace().collect();
        if words.is_empty() {
            skipped += 1;
            continue;
        }

        if words.len() == 1 {
            // Single-word path: use the pre-computed MFCC.
            let phonemes = cyrillic_to_phonemes(&entry.transcript, true);
            if phonemes.is_empty() {
                skipped += 1;
                continue;
            }
            let mfcc_path = args.bank_dir.join(&entry.mfcc_path);
            let mfcc_bytes = match fs::read(&mfcc_path) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("[build-bank] cannot read {}: {}", mfcc_path.display(), e);
                    skipped += 1;
                    continue;
                }
            };
            let mfcc_seq = adam_audio::mfcc::read_binary(&mfcc_bytes)?;
            if mfcc_seq.num_frames() < phonemes.len() {
                skipped += 1;
                continue;
            }
            sources.push(BankSource {
                label: entry.label,
                phonemes,
                mfcc: mfcc_seq,
            });
        } else {
            // Multi-word path: load the WAV, split at silent
            // gaps, compute per-word MFCC, generate one source
            // per word. Skip the whole entry if word count
            // doesn't match the splitter's output.
            let wav_path = args.bank_dir.join(&entry.wav_path);
            let pcm = match adam_audio::wav::read_wav(&wav_path) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("[build-bank] cannot read {}: {}", wav_path.display(), e);
                    skipped += 1;
                    continue;
                }
            };
            let pcm_mono = if pcm.channels > 1 { pcm.to_mono() } else { pcm };
            let segs = adam_audio::word_split::split_words(
                &pcm_mono.data,
                pcm_mono.sample_rate,
                &adam_audio::word_split::WordSplitConfig::default(),
            );
            if segs.len() != words.len() {
                println!(
                    "[build-bank] skipping «{}» — splitter found {} segments, transcript has {} words",
                    entry.label,
                    segs.len(),
                    words.len(),
                );
                skipped += 1;
                continue;
            }
            for (w_idx, (word_txt, (start, end))) in words.iter().zip(segs.iter()).enumerate() {
                let phonemes = cyrillic_to_phonemes(word_txt, true);
                if phonemes.is_empty() {
                    continue;
                }
                let word_samples = &pcm_mono.data[*start..*end];
                let word_mfcc = adam_audio::mfcc::mfcc(
                    word_samples,
                    pcm_mono.sample_rate,
                    &adam_audio::mfcc::MfccConfig::default(),
                );
                if word_mfcc.num_frames() < phonemes.len() {
                    continue;
                }
                sources.push(BankSource {
                    label: format!("{}_w{w_idx}", entry.label),
                    phonemes,
                    mfcc: word_mfcc,
                });
            }
        }
    }

    println!(
        "[build-bank] loaded {} usable sources, {skipped} skipped",
        sources.len()
    );
    println!(
        "[build-bank] running {} bootstrap iterations",
        args.iterations
    );

    // Iteration 0: equipartition.
    let mut bank = build_pass_equipartition(&sources);
    println!(
        "[build-bank] iter 0 (equipartition): {} phonemes",
        bank.len()
    );

    // Iterations 1+: DTW re-alignment against the current bank.
    for iter in 1..args.iterations {
        bank = build_pass_dtw_realign(&sources, &bank);
        println!(
            "[build-bank] iter {iter} (DTW re-align): {} phonemes",
            bank.len()
        );
    }

    // Final coverage report.
    let mut report: Vec<(Phoneme, usize)> = bank
        .iter()
        .map(|(p, t)| (*p, t.mfcc.num_frames()))
        .collect();
    report.sort_by_key(|(p, _)| p.to_byte());
    println!("\n[build-bank] final per-phoneme template lengths:");
    for (p, n_frames) in &report {
        println!("  {p:?} → {n_frames} frames");
    }

    bank.save_to_file(&args.output)?;
    println!("\n[build-bank] wrote {}", args.output.display());
    Ok(())
}

/// Bootstrap iteration 0: equipartition each source's MFCC
/// across its phoneme sequence, collect per-phoneme chunks,
/// DBA-average per phoneme → bank.
fn build_pass_equipartition(sources: &[BankSource]) -> adam_stt_phoneme::PhonemeBank {
    use adam_phoneme::Phoneme;
    use adam_stt_phoneme::{PhonemeBank, PhonemeTemplate};
    use std::collections::HashMap;

    let mut per_phoneme: HashMap<Phoneme, Vec<adam_audio::mfcc::MfccSequence>> = HashMap::new();
    for src in sources {
        let n_frames = src.mfcc.num_frames();
        let chunk = n_frames / src.phonemes.len();
        for (i, &phoneme) in src.phonemes.iter().enumerate() {
            let start = i * chunk;
            let end = if i + 1 == src.phonemes.len() {
                n_frames
            } else {
                (i + 1) * chunk
            };
            let frames: Vec<Vec<f32>> = src.mfcc.frames[start..end].to_vec();
            if frames.is_empty() {
                continue;
            }
            per_phoneme
                .entry(phoneme)
                .or_default()
                .push(adam_audio::mfcc::MfccSequence {
                    frames,
                    sample_rate: src.mfcc.sample_rate,
                    hop_length: src.mfcc.hop_length,
                    n_mfcc: src.mfcc.n_mfcc,
                });
        }
    }
    let mut bank = PhonemeBank::new();
    for (phoneme, chunks) in per_phoneme {
        bank.insert(PhonemeTemplate {
            phoneme,
            mfcc: average_chunks(&chunks),
        });
    }
    bank
}

/// Bootstrap iteration N>0: for each source, DTW-align its
/// MFCC against the concatenation of current templates for
/// its phoneme sequence; use the alignment path to extract
/// refined per-phoneme chunks; DBA-average → new bank.
///
/// Sources whose phoneme sequence isn't fully covered by the
/// current bank are skipped — they keep the previous-iteration
/// template if any (the new bank seeds from `current`).
fn build_pass_dtw_realign(
    sources: &[BankSource],
    current: &adam_stt_phoneme::PhonemeBank,
) -> adam_stt_phoneme::PhonemeBank {
    use adam_phoneme::Phoneme;
    use adam_stt_phoneme::{PhonemeTemplate, dtw, euclidean_distance};
    use std::collections::HashMap;

    let mut per_phoneme: HashMap<Phoneme, Vec<adam_audio::mfcc::MfccSequence>> = HashMap::new();
    let mut realigned = 0_usize;
    let mut uncovered = 0_usize;

    for src in sources {
        // Pull templates for every phoneme in this source.
        let templates: Vec<&adam_audio::mfcc::MfccSequence> = src
            .phonemes
            .iter()
            .filter_map(|p| current.get(*p).map(|t| &t.mfcc))
            .collect();
        if templates.len() != src.phonemes.len() {
            uncovered += 1;
            continue;
        }

        // Build expected = concatenation of templates, with
        // per-phoneme [start, end] ranges in expected coords.
        let mut expected_frames: Vec<Vec<f32>> = Vec::new();
        let mut phoneme_ranges: Vec<(usize, usize)> = Vec::with_capacity(templates.len());
        for t in &templates {
            let start = expected_frames.len();
            expected_frames.extend_from_slice(&t.frames);
            phoneme_ranges.push((start, expected_frames.len()));
        }

        // DTW-align source MFCC (M frames) vs expected (N frames).
        let Some(result) =
            dtw::dtw_with_distance(&src.mfcc.frames, &expected_frames, euclidean_distance)
        else {
            continue;
        };

        // Walk the path; for each phoneme range, collect the
        // source-frame indices that mapped into that range.
        for (p_idx, &(rng_start, rng_end)) in phoneme_ranges.iter().enumerate() {
            let phoneme = src.phonemes[p_idx];
            let mut src_indices: Vec<usize> = Vec::new();
            for &(qi, ti) in &result.path {
                if ti >= rng_start && ti < rng_end {
                    src_indices.push(qi);
                }
            }
            src_indices.sort_unstable();
            src_indices.dedup();
            if src_indices.is_empty() {
                continue;
            }
            let chunk_frames: Vec<Vec<f32>> = src_indices
                .iter()
                .map(|&i| src.mfcc.frames[i].clone())
                .collect();
            per_phoneme
                .entry(phoneme)
                .or_default()
                .push(adam_audio::mfcc::MfccSequence {
                    frames: chunk_frames,
                    sample_rate: src.mfcc.sample_rate,
                    hop_length: src.mfcc.hop_length,
                    n_mfcc: src.mfcc.n_mfcc,
                });
        }
        realigned += 1;
    }

    eprintln!("[build-bank]   re-aligned {realigned} sources, {uncovered} uncovered (skipped)");

    // Seed new bank from current so phonemes without new
    // chunks keep their previous template.
    let mut bank = current.clone();
    for (phoneme, chunks) in per_phoneme {
        bank.insert(PhonemeTemplate {
            phoneme,
            mfcc: average_chunks(&chunks),
        });
    }
    bank
}

/// Pseudo-DBA averaging across a bucket of MFCC chunks: pick
/// the mean chunk length, linearly resample each chunk to
/// that length, frame-wise average. Returns one averaged
/// MfccSequence representing the bucket.
fn average_chunks(chunks: &[adam_audio::mfcc::MfccSequence]) -> adam_audio::mfcc::MfccSequence {
    assert!(!chunks.is_empty(), "average_chunks needs ≥1 chunk");
    let n_mfcc = chunks[0].dim();
    let target_len = (chunks.iter().map(|c| c.num_frames()).sum::<usize>() / chunks.len()).max(1);

    let mut acc: Vec<Vec<f32>> = vec![vec![0.0_f32; n_mfcc]; target_len];
    for chunk in chunks {
        let resampled = resample_frames(&chunk.frames, target_len);
        for (a, r) in acc.iter_mut().zip(resampled.iter()) {
            for (ac, rc) in a.iter_mut().zip(r.iter()) {
                *ac += rc;
            }
        }
    }
    for frame in &mut acc {
        for c in frame.iter_mut() {
            *c /= chunks.len() as f32;
        }
    }
    adam_audio::mfcc::MfccSequence {
        frames: acc,
        sample_rate: chunks[0].sample_rate,
        hop_length: chunks[0].hop_length,
        n_mfcc,
    }
}

/// Linearly resample a frame sequence to `target_len` frames
/// by nearest-neighbour pick. (For MFCC, simpler than interp
/// — each coefficient is already smooth across frames.)
fn resample_frames(frames: &[Vec<f32>], target_len: usize) -> Vec<Vec<f32>> {
    let src_len = frames.len().max(1);
    (0..target_len)
        .map(|i| {
            let src_i = i * src_len / target_len;
            frames[src_i.min(src_len - 1)].clone()
        })
        .collect()
}

// ─── helpers ──────────────────────────────────────────────────

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
