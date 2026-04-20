//! Build the morpheme inverted index from committed corpus packs.
//!
//! For every sample in every committed pack, this walks whitespace
//! tokens, runs each token through the FST parser, and indexes the
//! sample under each resulting root surface form.
//!
//! A per-word analysis cache keeps this tractable: the committed
//! corpus has ~4 M total words but only ~270 k unique forms. Parsing
//! each unique form once (not each occurrence) takes ~5 minutes on an
//! M2 at ~1.2 ms/parse, versus ~75 minutes without the cache.
//!
//! Two modes (the v1.3.5 Wikipedia / v1.5.0 CC-100 pattern):
//!
//!   default  — indexes the first 500 samples per pack, writes to the
//!              committed path `data/retrieval/morpheme_index.json`
//!              (~2 MB — a reference snapshot for CI + integration tests)
//!
//!   --full   — indexes every sample in every committed pack, writes to
//!              `data/retrieval/morpheme_index_full.json` which is
//!              gitignored. Local-only fuel for the v1.7.0+ retrieval
//!              engine experiments. Full build on committed corpus takes
//!              ~10 minutes on an M2 at ~1.2 ms/parse after cache warmup.
//!
//! Keys are the canonical root surface string (entry.root) from the
//! FST analysis. If a word has multiple analyses (e.g. noun vs verb),
//! the sample is indexed under every candidate root — this is a
//! recall-bias choice; disambiguation belongs in downstream ranking,
//! not in the index build step.
//!
//! Usage:
//!   cargo run --release -p adam-retrieval --bin build_morpheme_index
//!   cargo run --release -p adam-retrieval --bin build_morpheme_index -- --full
//!   cargo run --release -p adam-retrieval --bin build_morpheme_index -- --limit 100
//!     (override the per-pack sample limit; ignored in --full mode)

use std::{
    collections::{HashMap, HashSet},
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
    time::Instant,
};

use adam_kernel_fst::{
    lexicon::LexiconV1,
    parser::{Analysis, analyse},
};
use adam_retrieval::{MorphemeIndex, SampleRef};
use serde::Deserialize;

const CURATED_DIR: &str = "data/curated";
const COMMITTED_OUTPUT: &str = "data/retrieval/morpheme_index.json";
const FULL_OUTPUT: &str = "data/retrieval/morpheme_index_full.json";
const COMMITTED_DEFAULT_LIMIT: usize = 500;

const SOURCE_PACKS: &[&str] = &[
    "tatoeba_kazakh_pack.json",
    "wikipedia_kz_pack.json",
    "common_voice_kk_pack.json",
    "cc100_kk_pack.json",
    "abai_wikisource_pack.json",
    "kazakh_proverbs_pack.json",
    "synthetic_sentences_pack.json",
    "kazakh_classics_pack.json",
];

const PROGRESS_EVERY: usize = 10_000;

#[derive(Debug, Deserialize)]
struct PackFile {
    samples: Vec<Sample>,
}

#[derive(Debug, Deserialize)]
struct Sample {
    id: String,
    text: String,
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let full_mode = args.iter().any(|a| a == "--full");
    let limit: Option<usize> = if full_mode {
        None
    } else {
        Some(parse_flag(&args, "--limit").unwrap_or(COMMITTED_DEFAULT_LIMIT))
    };
    let output_path = if full_mode {
        FULL_OUTPUT
    } else {
        COMMITTED_OUTPUT
    };

    let lexicon = match LexiconV1::load_default() {
        Ok(l) => l,
        Err(e) => {
            eprintln!("cannot load lexicon: {e:?}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!(
        "lexicon loaded: {} curated + {} apertium entries",
        lexicon.curated_count, lexicon.apertium_count
    );

    let mut index = MorphemeIndex::new();
    // unique-word cache: a FST parse of every unique word once, reused
    // across every occurrence. Words with no analyses go into `no_hit`
    // so we avoid re-parsing.
    let mut root_cache: HashMap<String, Vec<String>> = HashMap::new();
    let mut no_hit: HashSet<String> = HashSet::new();

    let mut total_samples = 0usize;
    let mut total_words = 0usize;
    let mut unique_words_seen = 0usize;
    let mut parses = 0usize;

    let started = Instant::now();

    for pack_name in SOURCE_PACKS {
        let path = Path::new(CURATED_DIR).join(pack_name);
        if !path.exists() {
            eprintln!("skipping missing: {}", path.display());
            continue;
        }
        index.built_from.push(pack_name.to_string());
        eprintln!("indexing {} ...", path.display());
        let pack = match load_pack(&path) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("cannot load {}: {e}", path.display());
                return ExitCode::FAILURE;
            }
        };
        for (i, sample) in pack.samples.iter().enumerate() {
            if let Some(n) = limit {
                if i >= n {
                    break;
                }
            }
            total_samples += 1;
            let sref = SampleRef {
                pack: pack_name.to_string(),
                sample_id: sample.id.clone(),
            };
            let mut contributed_any_morpheme = false;
            for word in sample.text.split_whitespace() {
                let cleaned = normalise(word);
                if cleaned.is_empty() {
                    continue;
                }
                total_words += 1;
                if no_hit.contains(&cleaned) {
                    continue;
                }
                let roots = match root_cache.get(&cleaned) {
                    Some(rs) => rs.clone(),
                    None => {
                        unique_words_seen += 1;
                        parses += 1;
                        let rs = extract_roots(&cleaned, &lexicon);
                        if rs.is_empty() {
                            no_hit.insert(cleaned.clone());
                            root_cache.insert(cleaned, Vec::new());
                            continue;
                        } else {
                            root_cache.insert(cleaned.clone(), rs.clone());
                            rs
                        }
                    }
                };
                for root in roots {
                    index.insert(root, sref.clone());
                }
                contributed_any_morpheme = true;
            }
            // v1.6.5: remember sample text so downstream consumers (the
            // dialog layer's `Intent::Unknown` fallback) can cite the
            // actual sentence. Skip samples that contributed no morpheme
            // to keep the texts map in sync with the postings.
            if contributed_any_morpheme {
                index.remember_text(&sref, sample.text.clone());
            }
            if total_samples % PROGRESS_EVERY == 0 {
                eprintln!(
                    "progress: samples={} words={} unique_parsed={} no_hit={} elapsed={:.1}s",
                    total_samples,
                    total_words,
                    unique_words_seen,
                    no_hit.len(),
                    started.elapsed().as_secs_f64(),
                );
            }
        }
    }

    index.refresh_stats();

    eprintln!(
        "DONE: {} samples, {} words, {} unique words parsed, {} distinct morphemes, {} total postings, elapsed {:.1}s",
        total_samples,
        total_words,
        unique_words_seen,
        index.unique_morphemes,
        index.total_postings,
        started.elapsed().as_secs_f64()
    );
    eprintln!(
        "parses performed: {parses} (cache saved {})",
        total_words.saturating_sub(parses)
    );

    if let Some(parent) = Path::new(output_path).parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                eprintln!("cannot create {}: {e}", parent.display());
                return ExitCode::FAILURE;
            }
        }
    }
    let json = match serde_json::to_string_pretty(&index) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("serialise: {e}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = fs::write(output_path, json) {
        eprintln!("write {output_path}: {e}");
        return ExitCode::FAILURE;
    }
    eprintln!("wrote {output_path}");
    ExitCode::SUCCESS
}

fn load_pack(path: &PathBuf) -> Result<PackFile, String> {
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}

fn normalise(word: &str) -> String {
    word.chars()
        .filter(|c| c.is_alphabetic() || *c == '-')
        .collect::<String>()
        .to_lowercase()
}

fn extract_roots(word: &str, lex: &LexiconV1) -> Vec<String> {
    let analyses = analyse(word, lex);
    let mut seen: HashSet<String> = HashSet::new();
    let mut out = Vec::new();
    for a in analyses {
        let root = match a {
            Analysis::Noun { root, .. } => root.root,
            Analysis::Verb { root, .. } => root.root,
        };
        if seen.insert(root.clone()) {
            out.push(root);
        }
    }
    out
}

fn parse_flag(args: &[String], name: &str) -> Option<usize> {
    let idx = args.iter().position(|a| a == name)?;
    args.get(idx + 1).and_then(|s| s.parse().ok())
}
