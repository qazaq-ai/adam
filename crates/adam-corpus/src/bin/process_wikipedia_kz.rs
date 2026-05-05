// Process the full Kazakh Wikipedia dump into a curated pack.
//
// Input:  data/external/wikipedia_kz_plain.txt (~638 MB as of v1.3.0)
//         Articles separated by ASCII RS (0x1e).
//
// Default mode (committed):
//   data/curated/wikipedia_kz_pack.json   — first 150 k samples (~49 MB)
//
// --full mode (local-only, v1.3.5+):
//   data/curated/wikipedia_kz_pack.json   — first 150 k samples
//   data/curated/shards/wikipedia_kz_shard_02_pack.json  — next 150 k
//   data/curated/shards/wikipedia_kz_shard_03_pack.json  — next 150 k
//   ... up to ~10 shards covering the full ~1.4 M samples.
//
//   `data/curated/shards/` is gitignored — shards are local-only for the
//   v2.0 retrieval engine to consume. Committing everything would add
//   ~500 MB of pack JSON to the repo for marginal validation value.
//
// v1.3.0 rewrite (streaming) + v1.3.5 optional sharding:
//   - 64 KB chunked reads (was 1 byte/read, ~hours → 26 s)
//   - loanword-density filter (drop samples > 10 % loanword tokens)
//   - honours the v0.4.0 lesson: 2-word fragments rejected (min 4)
//   - v1.3.5: --full flag to continue past the first shard
//
// Filters (per v1.x corpus purity directive):
//   - reject if any Latin letter (stray English / wiki markup)
//   - reject if word count outside [4, 40]
//   - reject if sample's loanword density > 10 %
//   - dedup cross-article by lowercased full text

use std::{
    collections::HashSet,
    env,
    fs::File,
    io::{BufReader, Read},
    process::ExitCode,
};

use serde::Serialize;

const DEFAULT_INPUT: &str = "data/external/wikipedia_kz_plain.txt";
const DEFAULT_OUTPUT: &str = "data/curated/wikipedia_kz_pack.json";
const SHARDS_DIR: &str = "data/curated/shards";

const ARTICLE_SEP: u8 = 0x1e;
const READ_CHUNK: usize = 64 * 1024;

const MIN_WORDS: usize = 4;
const MAX_WORDS: usize = 40;
const LOANWORD_DENSITY_CAP: f32 = 0.10;

/// Samples per shard. 150 k keeps each pretty-printed JSON file under
/// ~50 MB — the project convention. Shard 1 lives at the canonical
/// committed path; shards 2+ land in the gitignored `shards/` directory.
const SHARD_TARGET_SAMPLES: usize = 150_000;

/// Russian-only letters that never appear in native pre-modern Kazakh.
const RUSSIAN_ONLY: &[char] = &['ё', 'ф', 'ц', 'ч', 'щ', 'ъ', 'ь', 'э'];

/// Loanword suffix patterns shared across processors.
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

#[derive(Debug, Serialize)]
struct Sample {
    id: String,
    pack_name: String,
    source_id: String,
    domain: String,
    text: String,
}

#[derive(Debug, Serialize)]
struct Pack {
    version: String,
    name: String,
    target_language: String,
    script: String,
    source_license: String,
    source_url: String,
    attribution: String,
    articles_scanned: usize,
    sentences_scanned: usize,
    sample_count: usize,
    skipped_latin: usize,
    skipped_length: usize,
    skipped_duplicate: usize,
    skipped_loanword_heavy: usize,
    samples: Vec<Sample>,
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let full_mode = args.iter().any(|a| a == "--full");

    // Positional args (after filtering flags).
    let positional: Vec<&String> = args
        .iter()
        .skip(1)
        .filter(|a| !a.starts_with("--"))
        .collect();
    let input_path = positional
        .first()
        .map(|s| s.to_string())
        .unwrap_or_else(|| DEFAULT_INPUT.to_string());
    let shard_one_path = positional
        .get(1)
        .map(|s| s.to_string())
        .unwrap_or_else(|| DEFAULT_OUTPUT.to_string());
    let single_shard_cap: Option<usize> = positional.get(2).and_then(|s| s.parse().ok());

    let file = match File::open(&input_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("cannot open {input_path}: {e}");
            eprintln!("hint: run scripts/fetch_wikipedia_kz.sh first");
            return ExitCode::FAILURE;
        }
    };
    let mut reader = BufReader::with_capacity(READ_CHUNK, file);

    let mut shard_state = ShardState::new(shard_one_path, full_mode, single_shard_cap);
    let mut seen: HashSet<String> = HashSet::new();

    // Buffered streaming by article.
    let mut article: Vec<u8> = Vec::with_capacity(16 * 1024);
    let mut chunk = [0u8; READ_CHUNK];

    'outer: loop {
        let n = match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                eprintln!("read error: {e}");
                return ExitCode::FAILURE;
            }
        };
        for &b in &chunk[..n] {
            if b == ARTICLE_SEP {
                shard_state.begin_article();
                process_article(&article, &mut shard_state, &mut seen);
                article.clear();
                if shard_state.is_finished() {
                    break 'outer;
                }
            } else {
                article.push(b);
            }
        }
    }
    // Flush any trailing article without a terminator.
    if !article.is_empty() {
        shard_state.begin_article();
        process_article(&article, &mut shard_state, &mut seen);
    }

    if let Err(e) = shard_state.flush_final() {
        eprintln!("flush error: {e}");
        return ExitCode::FAILURE;
    }

    shard_state.print_summary();
    ExitCode::SUCCESS
}

/// Streaming shard writer. Accumulates samples up to SHARD_TARGET_SAMPLES,
/// flushes each shard to disk, resets per-shard counters, continues.
struct ShardState {
    full_mode: bool,
    single_shard_cap: Option<usize>,
    first_shard_path: String,

    // Current in-progress shard.
    shard_index: usize, // 1-based
    samples: Vec<Sample>,

    // Aggregate counters (across all shards).
    total_articles_scanned: usize,
    total_sentences_scanned: usize,
    total_skipped_latin: usize,
    total_skipped_length: usize,
    total_skipped_duplicate: usize,
    total_skipped_loanword: usize,
    total_accepted: usize,

    // Per-current-shard counters (reset after each flush so per-shard
    // metadata reflects only that shard's input subset).
    shard_articles: usize,
    shard_sentences: usize,
    shard_skipped_latin: usize,
    shard_skipped_length: usize,
    shard_skipped_duplicate: usize,
    shard_skipped_loanword: usize,

    finished: bool,
}

impl ShardState {
    fn new(first_shard_path: String, full_mode: bool, single_shard_cap: Option<usize>) -> Self {
        Self {
            full_mode,
            single_shard_cap,
            first_shard_path,
            shard_index: 1,
            samples: Vec::new(),
            total_articles_scanned: 0,
            total_sentences_scanned: 0,
            total_skipped_latin: 0,
            total_skipped_length: 0,
            total_skipped_duplicate: 0,
            total_skipped_loanword: 0,
            total_accepted: 0,
            shard_articles: 0,
            shard_sentences: 0,
            shard_skipped_latin: 0,
            shard_skipped_length: 0,
            shard_skipped_duplicate: 0,
            shard_skipped_loanword: 0,
            finished: false,
        }
    }

    fn begin_article(&mut self) {
        self.total_articles_scanned += 1;
        self.shard_articles += 1;
    }

    fn add_sentence_scan(&mut self) {
        self.total_sentences_scanned += 1;
        self.shard_sentences += 1;
    }

    fn add_skipped(&mut self, reason: SkipReason) {
        match reason {
            SkipReason::Latin => {
                self.total_skipped_latin += 1;
                self.shard_skipped_latin += 1;
            }
            SkipReason::Length => {
                self.total_skipped_length += 1;
                self.shard_skipped_length += 1;
            }
            SkipReason::Duplicate => {
                self.total_skipped_duplicate += 1;
                self.shard_skipped_duplicate += 1;
            }
            SkipReason::Loanword => {
                self.total_skipped_loanword += 1;
                self.shard_skipped_loanword += 1;
            }
        }
    }

    fn accept(&mut self, text: String) {
        self.total_accepted += 1;
        let idx = self.total_accepted;
        self.samples.push(Sample {
            id: format!("wiki_kz_{idx:07}"),
            pack_name: "adam-wikipedia-kz-pack".to_string(),
            source_id: "wikipedia_kz".to_string(),
            domain: "wikipedia".to_string(),
            text,
        });

        // Shard rollover.
        if self.samples.len() >= SHARD_TARGET_SAMPLES {
            if let Err(e) = self.flush_current_shard() {
                eprintln!("flush error on shard {}: {e}", self.shard_index);
                std::process::exit(1);
            }
            if !self.full_mode {
                // In default (committed) mode only the first shard is
                // written; the rest would bloat git.
                self.finished = true;
            }
        }

        // Honour the single-shard `cap` positional for dev runs.
        if let Some(cap) = self.single_shard_cap {
            if self.samples.len() >= cap && self.shard_index == 1 {
                self.finished = true;
            }
        }
    }

    fn is_finished(&self) -> bool {
        self.finished
    }

    fn flush_final(&mut self) -> std::io::Result<()> {
        if !self.samples.is_empty() {
            self.flush_current_shard()?;
        }
        Ok(())
    }

    fn flush_current_shard(&mut self) -> std::io::Result<()> {
        let path = self.current_shard_path();
        let pack = Pack {
            version: env!("CARGO_PKG_VERSION").to_string(),
            name: format!("adam-wikipedia-kz-pack-shard-{:02}", self.shard_index),
            target_language: "kazakh".to_string(),
            script: "cyrillic".to_string(),
            source_license: "CC-BY-SA 4.0".to_string(),
            source_url: "https://kk.wikipedia.org".to_string(),
            attribution: "Wikipedia contributors, Kazakh Wikipedia (CC-BY-SA 4.0)".to_string(),
            articles_scanned: self.shard_articles,
            sentences_scanned: self.shard_sentences,
            sample_count: self.samples.len(),
            skipped_latin: self.shard_skipped_latin,
            skipped_length: self.shard_skipped_length,
            skipped_duplicate: self.shard_skipped_duplicate,
            skipped_loanword_heavy: self.shard_skipped_loanword,
            samples: std::mem::take(&mut self.samples),
        };

        if let Some(parent) = std::path::Path::new(&path).parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let json = serde_json::to_string_pretty(&pack).map_err(std::io::Error::other)?;
        std::fs::write(&path, json)?;
        eprintln!(
            "shard {:02}: wrote {} samples to {path}",
            self.shard_index, pack.sample_count
        );

        self.shard_index += 1;
        // Reset per-shard counters; totals keep accumulating.
        self.shard_articles = 0;
        self.shard_sentences = 0;
        self.shard_skipped_latin = 0;
        self.shard_skipped_length = 0;
        self.shard_skipped_duplicate = 0;
        self.shard_skipped_loanword = 0;
        Ok(())
    }

    fn current_shard_path(&self) -> String {
        if self.shard_index == 1 {
            self.first_shard_path.clone()
        } else {
            format!(
                "{SHARDS_DIR}/wikipedia_kz_shard_{:02}_pack.json",
                self.shard_index
            )
        }
    }

    fn print_summary(&self) {
        eprintln!(
            "TOTAL articles={} sentences_scanned={} accepted={} skipped_latin={} skipped_length={} skipped_dup={} skipped_loanword={} shards_written={}",
            self.total_articles_scanned,
            self.total_sentences_scanned,
            self.total_accepted,
            self.total_skipped_latin,
            self.total_skipped_length,
            self.total_skipped_duplicate,
            self.total_skipped_loanword,
            self.shard_index - if self.samples.is_empty() { 1 } else { 0 },
        );
    }
}

#[derive(Debug, Clone, Copy)]
enum SkipReason {
    Latin,
    Length,
    Duplicate,
    Loanword,
}

fn process_article(bytes: &[u8], state: &mut ShardState, seen: &mut HashSet<String>) {
    let text = match std::str::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => return, // malformed UTF-8 slice, skip
    };
    for sent in split_sentences(text) {
        state.add_sentence_scan();
        match accept(sent, seen) {
            Ok(clean) => state.accept(clean),
            Err(reason) => state.add_skipped(reason),
        }
        if state.is_finished() {
            return;
        }
    }
}

/// Split article text into candidate sentences on `.?!`.
fn split_sentences(text: &str) -> Vec<String> {
    let mut sents = Vec::new();
    let mut cur = String::new();
    for ch in text.chars() {
        cur.push(ch);
        if matches!(ch, '.' | '!' | '?') {
            let trimmed = cur.trim().to_string();
            if !trimmed.is_empty() {
                sents.push(trimmed);
            }
            cur.clear();
        }
    }
    if !cur.trim().is_empty() {
        sents.push(cur.trim().to_string());
    }
    sents
}

fn accept(sentence: String, seen: &mut HashSet<String>) -> Result<String, SkipReason> {
    if sentence.chars().any(|c| c.is_ascii_alphabetic()) {
        return Err(SkipReason::Latin);
    }
    let word_count = sentence.split_whitespace().count();
    if !(MIN_WORDS..=MAX_WORDS).contains(&word_count) {
        return Err(SkipReason::Length);
    }
    if loanword_density(&sentence) > LOANWORD_DENSITY_CAP {
        return Err(SkipReason::Loanword);
    }
    let key = sentence.to_lowercase();
    if !seen.insert(key) {
        return Err(SkipReason::Duplicate);
    }
    Ok(sentence)
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
    if cleaned.chars().any(|c| RUSSIAN_ONLY.contains(&c)) {
        return true;
    }
    LOANWORD_SUFFIXES
        .iter()
        .any(|s| cleaned.ends_with(s) && cleaned.chars().count() > s.chars().count() + 1)
}
