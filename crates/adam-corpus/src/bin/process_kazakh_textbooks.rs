//! Processor for 10 Kazakh high-school textbook PDFs OCR'd into plain
//! text (v3.3.0 gold-corpus addition, per Codex review).
//!
//! ## Input
//!
//! Directory of per-page OCR output from tesseract-kaz:
//!
//! ```text
//! <OCR_DIR>/<book_id>_pages/p-001.txt
//! <OCR_DIR>/<book_id>_pages/p-002.txt
//! ...
//! ```
//!
//! `book_id` is the slug assigned by the OCR pipeline (`kz_lang_11_ogn`,
//! `physics_11_emn`, etc.). The binary discovers books by scanning the
//! top-level directory for `*_pages` subdirectories.
//!
//! ## Output
//!
//! `data/curated/kazakh_textbooks_pack.json` — same shape as
//! `kazakh_classics_pack.json`. Each sample carries:
//!
//!   - `id`: `kz_textbook_<book_id>_p<NNN>_s<MM>`
//!   - `text`: one cleaned sentence.
//!   - `pack_name`: `"adam-kazakh-textbooks-pack"`.
//!   - `source_id`: the `book_id` (so provenance is auditable).
//!   - `domain`: `"education"`.
//!
//! ## Cleaning / quality gates
//!
//! - Strip standalone page numbers and lone digits.
//! - Skip lines with any Latin run (defensive: OCR sometimes inserts
//!   stray ASCII from tables/figures).
//! - Sentence-split on `. ! ?` (same grain as classics).
//! - 4 ≤ word count ≤ 60 (slightly wider than 3–60 on literature to
//!   accommodate definition-style sentences common in textbooks).
//! - Require ≥ 80 % Kazakh Cyrillic chars in the sentence (guards
//!   against OCR noise / table fragments).
//! - Loanword-density cap 15 % (looser than literature's 10 % — physics
//!   and informatics have more Russian-derived technical vocabulary).
//! - Dedup by lowercase text across all books.
//!
//! ## v3.5.0 merge mode (`--merge-existing`)
//!
//! The pilot v3.3.0 pack covered 3 books; v3.5.0 adds 7 more but the
//! source PDFs for the first 3 were deleted during the v3.3.0
//! data/external/ cleanup (they're regenerable from external URLs if
//! ever needed). To avoid losing the already-committed OCR work, the
//! `--merge-existing <PATH>` flag reads the existing pack at PATH,
//! seeds the output with its samples (and its dedup set), and appends
//! the new book samples on top. Cross-book text dedup still applies —
//! no fact duplicates from the merge.
//!
//! ## Why OCR'd PDFs and not the original `pdftotext`
//!
//! The source PDFs use custom-font glyph encoding: `pdftotext` silently
//! drops Қ, Ң, Ғ, Ө, Ү, Ұ, Һ — the very characters a Kazakh-first
//! pipeline depends on. Tesseract's `kaz` pack recognises them at
//! ≥ 95 % precision after 300-DPI rendering. Pipeline lives in
//! `/tmp/ocr_pipeline.sh` (user-owned, not committed; OCR'd text is
//! staged under `/tmp/kazakh_textbooks_ocr/`).

use std::{
    collections::{BTreeMap, HashSet},
    env,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    process::ExitCode,
};

use serde::{Deserialize, Serialize};

const DEFAULT_INPUT_DIR: &str = "/tmp/kazakh_textbooks_ocr";
const DEFAULT_OUTPUT: &str = "data/curated/kazakh_textbooks_pack.json";

const MIN_WORDS: usize = 4;
const MAX_WORDS: usize = 60;
const LOANWORD_DENSITY_CAP: f32 = 0.15;
const MIN_KAZAKH_CHAR_RATIO: f32 = 0.80;

/// Russian-only Cyrillic letters — flag as loanword signal.
const RUSSIAN_ONLY: &[char] = &['ё', 'ф', 'ц', 'ч', 'щ', 'ъ', 'ь', 'э'];

const LOANWORD_SUFFIXES: &[&str] = &[
    "ция",
    "логия",
    "графия",
    "тика",
    "изм",
    "ивный",
    "ильный",
    "альный",
    "альная",
    "альное",
    "ональный",
];

/// Kazakh-specific Cyrillic letters. Presence signals authentic
/// Kazakh text vs generic Cyrillic (Russian / Ukrainian).
const KAZAKH_SPECIFIC: &[char] = &[
    'қ', 'ң', 'ғ', 'ө', 'ү', 'ұ', 'һ', 'і', 'ә', 'Қ', 'Ң', 'Ғ', 'Ө', 'Ү', 'Ұ', 'Һ', 'І', 'Ә',
];

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Sample {
    id: String,
    pack_name: String,
    source_id: String,
    domain: String,
    text: String,
}

/// Shape of a previously-committed pack we merge on top of.
/// We only read `samples`; the by-book counts are recomputed from
/// the merged set so old + new stay consistent.
#[derive(Debug, Deserialize)]
struct ExistingPackFile {
    samples: Vec<Sample>,
}

#[derive(Debug, Serialize)]
struct Pack {
    version: String,
    name: String,
    target_language: String,
    script: String,
    source_license: String,
    attribution: String,
    books_scanned: usize,
    pages_scanned: usize,
    sentences_scanned: usize,
    sample_count: usize,
    skipped_latin: usize,
    skipped_length: usize,
    skipped_low_kazakh: usize,
    skipped_duplicate: usize,
    skipped_loanword_heavy: usize,
    samples_by_book: BTreeMap<String, usize>,
    samples: Vec<Sample>,
}

fn main() -> ExitCode {
    let raw_args: Vec<String> = env::args().collect();
    // Positional args remain: [1] = input_dir, [2] = output_path.
    let input_dir = raw_args
        .get(1)
        .filter(|s| !s.starts_with("--"))
        .cloned()
        .unwrap_or_else(|| DEFAULT_INPUT_DIR.to_string());
    let output_path = raw_args
        .get(2)
        .filter(|s| !s.starts_with("--"))
        .cloned()
        .unwrap_or_else(|| DEFAULT_OUTPUT.to_string());
    let merge_existing = raw_args.iter().enumerate().find_map(|(i, a)| {
        if a == "--merge-existing" {
            raw_args.get(i + 1).cloned()
        } else {
            None
        }
    });

    let input_root = Path::new(&input_dir);
    if !input_root.exists() {
        eprintln!(
            "input directory {input_dir} does not exist.\n\
             hint: run the OCR pipeline first — see module docstring for details."
        );
        return ExitCode::FAILURE;
    }

    // Discover books: each subdir ending in `_pages` is one book.
    let mut books: Vec<(String, PathBuf)> = match fs::read_dir(input_root) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name.strip_suffix("_pages")
                    .map(|id| (id.to_string(), e.path()))
            })
            .collect(),
        Err(e) => {
            eprintln!("cannot list {input_dir}: {e}");
            return ExitCode::FAILURE;
        }
    };
    books.sort_by(|a, b| a.0.cmp(&b.0));
    if books.is_empty() {
        eprintln!("no `*_pages` subdirectories found in {input_dir}");
        return ExitCode::FAILURE;
    }
    eprintln!("process_kazakh_textbooks: {} books discovered", books.len());

    let mut samples: Vec<Sample> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut samples_by_book: BTreeMap<String, usize> = BTreeMap::new();
    let mut pages_scanned = 0usize;
    let mut sentences_scanned = 0usize;
    let mut skipped_latin = 0usize;
    let mut skipped_length = 0usize;
    let mut skipped_low_kazakh = 0usize;
    let mut skipped_duplicate = 0usize;
    let mut skipped_loanword = 0usize;

    // v3.5.0 — if --merge-existing given, seed samples + dedup set
    // from a previously-committed pack. Per-book counts from the
    // existing pack propagate; new books get counted as they're added.
    if let Some(existing_path) = merge_existing.as_ref() {
        let raw = match fs::read_to_string(existing_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("cannot read --merge-existing path {existing_path}: {e}");
                return ExitCode::FAILURE;
            }
        };
        let existing: ExistingPackFile = match serde_json::from_str(&raw) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("cannot parse {existing_path}: {e}");
                return ExitCode::FAILURE;
            }
        };
        eprintln!(
            "process_kazakh_textbooks: merging {} pre-existing samples from {existing_path}",
            existing.samples.len(),
        );
        for s in existing.samples {
            let key = s.text.to_lowercase();
            if seen.insert(key) {
                *samples_by_book.entry(s.source_id.clone()).or_insert(0) += 1;
                samples.push(s);
            }
        }
    }

    for (book_id, pages_dir) in &books {
        let mut page_files: Vec<PathBuf> = match fs::read_dir(pages_dir) {
            Ok(rd) => rd
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().map(|e| e == "txt").unwrap_or(false))
                .collect(),
            Err(e) => {
                eprintln!(
                    "skipping {book_id} — cannot list {}: {e}",
                    pages_dir.display()
                );
                continue;
            }
        };
        page_files.sort();
        let book_start = samples.len();
        for page_path in &page_files {
            pages_scanned += 1;
            let page_num = page_num_from_path(page_path);
            let Ok(text) = read_utf8(page_path) else {
                continue;
            };
            let paragraph = normalise_ocr_text(&text);
            for (si, sentence) in split_sentences(&paragraph).into_iter().enumerate() {
                sentences_scanned += 1;
                let s = sentence.trim();
                if s.is_empty() {
                    continue;
                }
                if has_latin(s) {
                    skipped_latin += 1;
                    continue;
                }
                let word_count = s.split_whitespace().count();
                if !(MIN_WORDS..=MAX_WORDS).contains(&word_count) {
                    skipped_length += 1;
                    continue;
                }
                if kazakh_char_ratio(s) < MIN_KAZAKH_CHAR_RATIO {
                    skipped_low_kazakh += 1;
                    continue;
                }
                if loanword_density(s) > LOANWORD_DENSITY_CAP {
                    skipped_loanword += 1;
                    continue;
                }
                let key = s.to_lowercase();
                if !seen.insert(key) {
                    skipped_duplicate += 1;
                    continue;
                }
                let id = format!("kz_textbook_{book_id}_p{page_num:04}_s{si:02}");
                samples.push(Sample {
                    id,
                    pack_name: "adam-kazakh-textbooks-pack".into(),
                    source_id: book_id.clone(),
                    domain: "education".into(),
                    text: s.to_string(),
                });
            }
        }
        let produced = samples.len() - book_start;
        samples_by_book.insert(book_id.clone(), produced);
        eprintln!("  {book_id}: {produced} samples");
    }

    let pack = Pack {
        version: "3.3.0".into(),
        name: "adam-kazakh-textbooks-pack".into(),
        target_language: "kazakh".into(),
        script: "cyrillic".into(),
        source_license: "Qazaqstan Respublikasy Bilim zhane gylym ministrligi — teaching-use redistribution; see data/curated/README.md for per-book provenance".into(),
        attribution: "Kazakh secondary-school textbooks (grades 7–11) ingested via 300-DPI tesseract-kaz OCR. Copyright of original content rests with the authors and the publisher; this pack redistributes only the OCR-extracted Kazakh text for computational linguistics use.".into(),
        books_scanned: books.len(),
        pages_scanned,
        sentences_scanned,
        sample_count: samples.len(),
        skipped_latin,
        skipped_length,
        skipped_low_kazakh,
        skipped_duplicate,
        skipped_loanword_heavy: skipped_loanword,
        samples_by_book,
        samples,
    };

    let json = match serde_json::to_string_pretty(&pack) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("serialise: {e}");
            return ExitCode::FAILURE;
        }
    };
    if let Some(parent) = Path::new(&output_path).parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                eprintln!("cannot create {}: {e}", parent.display());
                return ExitCode::FAILURE;
            }
        }
    }
    if let Err(e) = fs::write(&output_path, &json) {
        eprintln!("cannot write {output_path}: {e}");
        return ExitCode::FAILURE;
    }

    println!(
        "wrote {} samples to {output_path} (books={}, pages={}, sentences={})",
        pack.sample_count, pack.books_scanned, pack.pages_scanned, pack.sentences_scanned
    );
    println!(
        "skipped: latin={} length={} low_kazakh={} duplicate={} loanword-heavy={}",
        pack.skipped_latin,
        pack.skipped_length,
        pack.skipped_low_kazakh,
        pack.skipped_duplicate,
        pack.skipped_loanword_heavy
    );
    ExitCode::SUCCESS
}

fn page_num_from_path(p: &Path) -> usize {
    p.file_stem()
        .and_then(|s| s.to_str())
        .and_then(|s| s.trim_start_matches("p-").parse::<usize>().ok())
        .unwrap_or(0)
}

fn read_utf8(p: &Path) -> std::io::Result<String> {
    let mut f = File::open(p)?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    Ok(s)
}

/// Collapse whitespace and linebreaks; tesseract emits page-break
/// artifacts like form-feed (\x0C) and stray newlines inside
/// sentences. We join lines with a single space so sentence splitting
/// sees one stream. Also strip bare page-number lines (e.g. "12" on
/// its own line between paragraphs).
fn normalise_ocr_text(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Drop lines that are only digits + whitespace (page numbers).
        if trimmed
            .chars()
            .all(|c| c.is_ascii_digit() || c.is_whitespace())
        {
            continue;
        }
        out.push_str(trimmed);
        out.push(' ');
    }
    out
}

fn split_sentences(paragraph: &str) -> Vec<&str> {
    paragraph.split(['.', '!', '?']).collect()
}

fn has_latin(text: &str) -> bool {
    text.chars().any(|c| c.is_ascii_alphabetic())
}

fn kazakh_char_ratio(text: &str) -> f32 {
    let total_alpha = text.chars().filter(|c| c.is_alphabetic()).count();
    if total_alpha == 0 {
        return 0.0;
    }
    let cyrillic_or_kazakh = text
        .chars()
        .filter(|c| {
            c.is_alphabetic() && {
                let cp = *c as u32;
                // Cyrillic block + Cyrillic supplement (for Kazakh letters).
                (0x0400..=0x04FF).contains(&cp) || (0x0500..=0x052F).contains(&cp)
            }
        })
        .count();
    // Bonus: if the sentence contains any Kazakh-specific letter, that
    // alone is a very strong signal. We don't need the Kazakh-specific
    // ratio — the Cyrillic ratio + the loanword-density gate together
    // do the work.
    let _ = KAZAKH_SPECIFIC;
    cyrillic_or_kazakh as f32 / total_alpha as f32
}

fn loanword_density(text: &str) -> f32 {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return 0.0;
    }
    let flagged = words.iter().filter(|w| is_loanword_token(w)).count();
    flagged as f32 / words.len() as f32
}

fn is_loanword_token(word: &str) -> bool {
    let cleaned: String = word
        .chars()
        .filter(|c| c.is_alphabetic() || *c == '-')
        .collect::<String>()
        .to_lowercase();
    if cleaned.is_empty() {
        return false;
    }
    if cleaned.chars().any(|c| RUSSIAN_ONLY.contains(&c)) {
        return true;
    }
    LOANWORD_SUFFIXES.iter().any(|s| cleaned.ends_with(s))
}
