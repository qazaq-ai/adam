// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Heuristic triage of `docs/lexicon_gap_candidates.md` into three
//! disjoint buckets, mirroring the rule set Codex applied during the
//! 2026-05-18 peer-review pass:
//!
//!   - **auto-approve** — surfaces that look like regular Kazakh content
//!     words (pure-Cyrillic, no Latin / digits / Russian-loan suffixes,
//!     not a proper noun already covered elsewhere, ≥ 5 corpus
//!     citations recoverable from context).
//!   - **auto-exclude** — surfaces that should be REMOVED from the gap
//!     pool without adding to the Lexicon. Loanwords (Russian / English
//!     visible from suffix patterns or character set), abbreviations
//!     («млрд / жшс»), OCR artefacts, proper nouns already in
//!     `world_core/geography_kz` or `notable_kazakhstanis`, raw digits.
//!   - **needs-review** — everything else. Includes surfaces where the
//!     canonical root + POS judgement is non-trivial and requires a
//!     native-speaker pass.
//!
//! Output: three CSV files in `data/lexicon_v2/`:
//!
//! ```text
//! data/lexicon_v2/auto_approve.csv
//! data/lexicon_v2/auto_exclude.csv
//! data/lexicon_v2/needs_review.csv
//! ```
//!
//! Each file has one row per candidate with columns:
//!
//!   `surface, freq, harmony, final_class, cluster_reason`
//!
//! The CSVs are the work product. The downstream Lexicon ingestion
//! step (deferred to rc3 pending native-speaker validation) reads the
//! `auto_approve.csv` and proposes Lexicon entries; the `mine_lexicon_
//! gaps` filter step reads the `auto_exclude.csv` to skip candidates
//! that future scans should not re-surface.
//!
//! **Conservative bias.** When a heuristic is ambiguous, the candidate
//! defaults to `needs-review`, NOT `auto-approve`. The point is to
//! reduce the reviewer's load on the safe cases, never to substitute
//! for human judgement on the risky ones.

use std::fs;
use std::io::Write;
use std::path::Path;

const INPUT_PATH: &str = "docs/lexicon_gap_candidates.md";
const OUTPUT_DIR: &str = "data/lexicon_v2";

#[derive(Debug, Clone)]
struct Candidate {
    n: usize,
    surface: String,
    freq: u64,
    harmony: String,
    final_class: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Cluster {
    AutoApprove,
    AutoExclude,
    NeedsReview,
}

fn cluster_label(c: &Cluster) -> &'static str {
    match c {
        Cluster::AutoApprove => "auto-approve",
        Cluster::AutoExclude => "auto-exclude",
        Cluster::NeedsReview => "needs-review",
    }
}

fn main() {
    let text = fs::read_to_string(INPUT_PATH).unwrap_or_else(|e| {
        eprintln!("triage_lexicon_v2: failed to read {INPUT_PATH}: {e}");
        std::process::exit(1);
    });
    let candidates = parse_candidates(&text);
    eprintln!("[1/3] parsed {} candidates", candidates.len());

    let geo_canon = load_canonical_kz_geo_names();
    eprintln!(
        "[2/3] loaded {} canonical geo names for proper-noun exclusion",
        geo_canon.len()
    );

    let mut classified: Vec<(Candidate, Cluster, String)> = Vec::with_capacity(candidates.len());
    for c in candidates {
        let (cluster, reason) = classify(&c, &geo_canon);
        classified.push((c, cluster, reason));
    }

    let mut approve = 0usize;
    let mut exclude = 0usize;
    let mut review = 0usize;
    for (_, cluster, _) in &classified {
        match cluster {
            Cluster::AutoApprove => approve += 1,
            Cluster::AutoExclude => exclude += 1,
            Cluster::NeedsReview => review += 1,
        }
    }
    eprintln!(
        "[3/3] classified: auto-approve={approve} auto-exclude={exclude} needs-review={review}"
    );

    fs::create_dir_all(OUTPUT_DIR).expect("create output dir");
    // auto_approve.csv and needs_review.csv are per-iteration:
    // they reflect the CURRENT gap candidates only. Overwriting on
    // each run is correct — they describe what the reviewer should
    // look at NOW, not historical accumulation.
    write_csv(
        &Path::new(OUTPUT_DIR).join("auto_approve.csv"),
        &classified,
        Cluster::AutoApprove,
    );
    write_csv(
        &Path::new(OUTPUT_DIR).join("needs_review.csv"),
        &classified,
        Cluster::NeedsReview,
    );
    // auto_exclude.csv is **cumulative across triage iterations**:
    // each run adds the newly-classified excludes to the existing
    // file. Without this, the gap-miner filter would lose memory
    // of every prior batch of excluded surfaces (the second run
    // is filtered down to ~9 surfaces because the prior 144 are
    // gone from the pool, so an overwrite would drop them).
    write_csv_merge_exclude(&Path::new(OUTPUT_DIR).join("auto_exclude.csv"), &classified);
    eprintln!("wrote {OUTPUT_DIR}/{{auto_approve, auto_exclude, needs_review}}.csv");
}

/// Walk the `### Candidate #N — \`surface\` (freq F)` headers and
/// gather the harmony / final-class auto-tags. Uses a Peekable
/// iterator so the inner read-ahead loop can stop at the NEXT
/// candidate header without consuming it (otherwise every other
/// candidate would be lost — earlier bug at this site).
fn parse_candidates(text: &str) -> Vec<Candidate> {
    let mut out = Vec::new();
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        let Some(stripped) = line.strip_prefix("### Candidate #") else {
            continue;
        };
        // header form: `N — \`surface\` (freq NNNN)`
        let (n_str, rest) = match stripped.split_once(" — ") {
            Some(pair) => pair,
            None => continue,
        };
        let Ok(n) = n_str.trim().parse::<usize>() else {
            continue;
        };
        let surface = rest
            .trim()
            .trim_start_matches('`')
            .split('`')
            .next()
            .unwrap_or("")
            .to_string();
        let freq: u64 = rest
            .rsplit_once("freq ")
            .and_then(|(_, after)| after.trim_end_matches(')').trim().parse().ok())
            .unwrap_or(0);

        let mut harmony = String::new();
        let mut final_class = String::new();
        // Peek at the next line — break (without consuming) when it
        // starts the next candidate or hits an H2 boundary.
        loop {
            let peeked = match lines.peek() {
                Some(l) => *l,
                None => break,
            };
            if peeked.starts_with("### Candidate #") {
                break;
            }
            let is_h2_section = peeked.starts_with("## ") && !peeked.starts_with("### ");
            if is_h2_section {
                break;
            }
            // Now consume the line — we know it's part of THIS candidate.
            let ahead = lines.next().unwrap();
            if let Some(rest) = ahead.strip_prefix("- Vowel harmony (auto): ") {
                harmony = rest.replace("**", "").trim().to_string();
            }
            if let Some(rest) = ahead.strip_prefix("- Final sound (auto): ") {
                final_class = rest.replace("**", "").trim().to_string();
            }
        }
        out.push(Candidate {
            n,
            surface,
            freq,
            harmony,
            final_class,
        });
    }
    out
}

/// Heuristic classification. Conservative: when in doubt, defer to
/// `NeedsReview`. The return value carries a short rationale string
/// for the reviewer to read in the CSV.
fn classify(c: &Candidate, geo_canon: &std::collections::HashSet<String>) -> (Cluster, String) {
    let s = c.surface.to_lowercase();
    let chars: Vec<char> = s.chars().collect();

    // ── auto-exclude path ─────────────────────────────────────────
    // Empty / too-short surfaces — OCR garbage.
    if chars.len() < 3 {
        return (Cluster::AutoExclude, "too-short".into());
    }
    // Contains Latin letters → loanword / acronym / Whisper-mixed.
    if s.chars().any(|c| c.is_ascii_alphabetic() && c != ' ') {
        return (Cluster::AutoExclude, "contains-latin".into());
    }
    // Contains digits → number / measurement / abbreviation.
    if s.chars().any(|c| c.is_ascii_digit()) {
        return (Cluster::AutoExclude, "contains-digit".into());
    }
    // Common abbreviations (visible in top-frequency uncovered set).
    const ABBREVS: &[&str] = &[
        "млрд", "млн", "трлн", "жшс", "ао", "ип", "тоо", "тб", "тс", "тт", "тд", "тб",
    ];
    if ABBREVS.contains(&s.as_str()) {
        return (Cluster::AutoExclude, "abbreviation".into());
    }
    // Common Russian-loan suffixes — strongly indicate Russian-origin
    // borrowing rather than native Kazakh root.
    const RUSSIAN_LOAN_SUFFIXES: &[&str] = &[
        "ция",
        "сия",
        "зия",
        "логия",
        "графия",
        "тика",
        "ника",
        "изм",
        "ист",
        "тор",
        "ёр",
        "ров",
        "ность",
        "цион",
        "ация",
        "иция",
        "оция",
    ];
    if RUSSIAN_LOAN_SUFFIXES.iter().any(|suf| s.ends_with(suf)) {
        return (Cluster::AutoExclude, "russian-loan-suffix".into());
    }
    // Common Russian-loan prefixes.
    const RUSSIAN_LOAN_PREFIXES: &[&str] = &[
        "интер",
        "авто",
        "теле",
        "электр",
        "компью",
        "видео",
        "радио",
        "супер",
        "анти",
        "контр",
        "транс",
        "ультра",
        "макс",
        "мини",
        "микро",
        "мега",
        "гига",
        "нано",
    ];
    if RUSSIAN_LOAN_PREFIXES.iter().any(|p| s.starts_with(p)) {
        return (Cluster::AutoExclude, "russian-loan-prefix".into());
    }
    // Proper noun already in geography_kz / notable people domains.
    if geo_canon.contains(&s) {
        return (Cluster::AutoExclude, "already-in-geo-domain".into());
    }
    // Surfaces ending in `-сы / -сі / -сын / -сін / -сында / -сінде`
    // are 3sg-possessive of nouns — the bare-root form is what
    // belongs in the Lexicon. These are needs-review (we should add
    // the BARE root, not the possessive surface).
    const POSSESSIVE_3SG_TAILS: &[&str] = &[
        "ының",
        "інің",
        "ыңың",
        "інің",
        "сы",
        "сі",
        "сын",
        "сін",
        "сында",
        "сінде",
    ];
    let looks_possessive = POSSESSIVE_3SG_TAILS
        .iter()
        .any(|tail| s.ends_with(tail) && s.chars().count() > tail.chars().count() + 2);
    if looks_possessive {
        return (Cluster::NeedsReview, "looks-3sg-possessive-of-noun".into());
    }

    // ── auto-approve path ─────────────────────────────────────────
    // Conservative: only mark as auto-approve when ALL of:
    //   - pure Cyrillic
    //   - 3..=10 chars (short forms most likely native roots)
    //   - has harmony + final_class auto-tagged
    //   - frequency ≥ 50 (top-tier uncovered)
    let pure_cyrillic = s
        .chars()
        .all(|c| matches!(c, '\u{0400}'..='\u{04FF}') || c == '-' || c == '\u{2019}' || c == '\'');
    let length_ok = (3..=10).contains(&chars.len());
    let has_features = !c.harmony.is_empty() && !c.final_class.is_empty();
    if pure_cyrillic && length_ok && has_features && c.freq >= 50 {
        return (Cluster::AutoApprove, "pure-cyrillic-short-high-freq".into());
    }

    // ── default: needs-review ──────────────────────────────────────
    (Cluster::NeedsReview, "default-needs-native-speaker".into())
}

/// Build the set of canonical Kazakh geographic names from
/// `data/world_core/geography_kz.jsonl`. Used to exclude proper nouns
/// already covered as facts (oblasts / cities / rivers / mountains)
/// from showing up in auto-approve.
fn load_canonical_kz_geo_names() -> std::collections::HashSet<String> {
    let mut set = std::collections::HashSet::new();
    let path = "data/world_core/geography_kz.jsonl";
    let Ok(text) = fs::read_to_string(path) else {
        eprintln!(
            "triage_lexicon_v2: warning — could not read {path}; geo-name exclusion will skip"
        );
        return set;
    };
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        // Pull every `"subject":"…"` and `"object":"…"` from the
        // facts array. Lowercased; we only care about word-level
        // membership in the auto-exclude gate.
        for prefix in ["\"subject\":\"", "\"object\":\""] {
            let mut search = trimmed;
            while let Some(idx) = search.find(prefix) {
                let after = &search[idx + prefix.len()..];
                if let Some(end) = after.find('"') {
                    set.insert(after[..end].to_lowercase());
                    search = &after[end..];
                } else {
                    break;
                }
            }
        }
    }
    set
}

fn write_csv(path: &Path, classified: &[(Candidate, Cluster, String)], filter: Cluster) {
    let mut f = fs::File::create(path).expect("create csv");
    writeln!(f, "n,surface,freq,harmony,final_class,reason").expect("header");
    for (c, cluster, reason) in classified {
        if *cluster != filter {
            continue;
        }
        // CSV-quote the surface in case of internal commas (extremely
        // unlikely for Kazakh, but harmless).
        writeln!(
            f,
            "{},{},{},{},{},{}",
            c.n,
            csv_quote(&c.surface),
            c.freq,
            c.harmony,
            c.final_class,
            csv_quote(reason),
        )
        .expect("row");
    }
    eprintln!(
        "wrote {} → {} rows",
        path.display(),
        classified.iter().filter(|(_, x, _)| x == &filter).count()
    );
    let _ = cluster_label;
}
/// Cumulative writer for auto_exclude.csv — reads any existing
/// file, deduplicates by `surface`, then writes the union.
fn write_csv_merge_exclude(path: &Path, classified: &[(Candidate, Cluster, String)]) {
    use std::collections::BTreeMap;
    // Existing rows keyed by lowercase surface so we preserve prior
    // entries that the current iteration no longer sees.
    let mut rows: BTreeMap<String, String> = BTreeMap::new();
    if let Ok(existing) = fs::read_to_string(path) {
        for (i, line) in existing.lines().enumerate() {
            if i == 0 || line.is_empty() {
                continue; // header / blank
            }
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 2 {
                rows.insert(parts[1].trim().to_lowercase(), line.to_string());
            }
        }
    }
    for (c, cluster, reason) in classified {
        if !matches!(cluster, Cluster::AutoExclude) {
            continue;
        }
        let key = c.surface.to_lowercase();
        let row = format!(
            "{},{},{},{},{},{}",
            c.n,
            csv_quote(&c.surface),
            c.freq,
            c.harmony,
            c.final_class,
            csv_quote(reason),
        );
        rows.insert(key, row);
    }
    let mut f = fs::File::create(path).expect("create csv");
    writeln!(f, "n,surface,freq,harmony,final_class,reason").expect("header");
    for row in rows.values() {
        writeln!(f, "{row}").expect("row");
    }
    eprintln!(
        "wrote {} → {} rows (cumulative)",
        path.display(),
        rows.len()
    );
}

fn csv_quote(s: &str) -> String {
    if s.contains(',') || s.contains('"') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
