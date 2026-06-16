// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! Production-equivalent eval binary.
//!
//! Mirrors the v6.5 `voice_repl_v6_3` main-binary startup (Phase 16
//! dialog engine + Phase 15f reasoning chains + Phase 17 world_core
//! domain index) and runs each accepted case from
//! `data/eval/v6_7_real_audit_eval.json` through `Conversation::turn`
//! — the full deterministic cascade (router → retrieval → reasoning
//! → realiser → verifier), exactly as production voice REPL does.
//!
//! Contrast with `respond.rs`, which exercises the LM-only generative
//! path and bypasses world_core / reasoning / domain inference. The
//! audit eval has been measured against `respond.rs` for the v6.7 /
//! v6.8 LM iterations; that number reports the LM ceiling alone, not
//! the production ceiling. This binary reports the production ceiling.

use std::path::Path;

use adam_dialog::Conversation;
use adam_dialog::DomainIndex;
use adam_dialog::templates::TemplateRepository;
use adam_kernel_fst::lexicon::LexiconV1;
use adam_retrieval::MorphemeIndex;
use serde::Deserialize;

const DEFAULT_EVAL_PATH: &str = "data/eval/v6_7_real_audit_eval.json";
const RETRIEVAL_INDEX_PATH: &str = "data/retrieval/morpheme_index.json";
const FACTS_PATH: &str = "data/retrieval/facts.json";
const DERIVED_FACTS_PATH: &str = "data/retrieval/derived_facts.json";
const WORLD_CORE_DIR: &str = "data/world_core";

#[derive(Debug, Deserialize)]
struct EvalCase {
    input: String,
    expected_response: Option<String>,
    was_accepted: bool,
    #[allow(dead_code)]
    notes: String,
    // School-program eval also carries subject/topic columns; they
    // are optional so the v6.7 audit pack still deserialises.
    #[serde(default)]
    subject: Option<String>,
    #[serde(default)]
    topic: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EvalPack {
    #[allow(dead_code)]
    version: String,
    cases: Vec<EvalCase>,
}

fn load_retrieval_index() -> Option<MorphemeIndex> {
    let file = std::fs::File::open(RETRIEVAL_INDEX_PATH).ok()?;
    let reader = std::io::BufReader::new(file);
    let mut idx: MorphemeIndex = serde_json::from_reader(reader).ok()?;
    idx.refresh_stats();
    Some(idx)
}

fn load_reasoning_chains() -> (
    Vec<adam_reasoning::Fact>,
    Vec<adam_reasoning::reasoner::DerivedFact>,
) {
    #[derive(Deserialize)]
    struct FactsFile {
        facts: Vec<adam_reasoning::Fact>,
    }
    #[derive(Deserialize)]
    struct DerivedFile {
        derived: Vec<adam_reasoning::reasoner::DerivedFact>,
    }
    let extracted = std::fs::File::open(FACTS_PATH)
        .ok()
        .and_then(|f| serde_json::from_reader::<_, FactsFile>(std::io::BufReader::new(f)).ok())
        .map(|f| f.facts)
        .unwrap_or_default();
    let derived = std::fs::File::open(DERIVED_FACTS_PATH)
        .ok()
        .and_then(|f| serde_json::from_reader::<_, DerivedFile>(std::io::BufReader::new(f)).ok())
        .map(|f| f.derived)
        .unwrap_or_default();
    (extracted, derived)
}

/// Match the `respond` eval normalisation: lowercase + flatten
/// sub/super-script digits to plain ASCII so a deterministic answer
/// "h2o" matches an expected "H₂O".
fn normalize_for_eval(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        let mapped = match ch {
            '₀' | '⁰' => '0',
            '₁' | '¹' => '1',
            '₂' | '²' => '2',
            '₃' | '³' => '3',
            '₄' | '⁴' => '4',
            '₅' | '⁵' => '5',
            '₆' | '⁶' => '6',
            '₇' | '⁷' => '7',
            '₈' | '⁸' => '8',
            '₉' | '⁹' => '9',
            '⁻' => '-',
            '⁺' => '+',
            other => other,
        };
        out.extend(mapped.to_lowercase());
    }
    out
}

/// Kazakh-language stop tokens that carry no semantic content for
/// production eval. These get dropped before content-overlap
/// matching. Includes copulas, particles, common pronouns,
/// quantifiers and discourse markers.
///
/// NOT a linguistic stoplist for general NLP — this is an eval-
/// time list tuned to the real-audit pack. Add tokens as new
/// false-negatives appear.
const STOP_TOKENS: &[&str] = &[
    // Discourse / fillers
    "иә",
    "жоқ",
    "иэ",
    "сонымен",
    "айтсам",
    "айтыңыз",
    "айтыңызшы",
    "бәлкім",
    "немесе",
    "болса",
    "болатын",
    "болғандықтан",
    "қысқаша",
    "оған",
    // Copulas / aux
    "екен",
    "емес",
    "болады",
    "болады",
    "болсын",
    // Pronouns and particles
    "мен",
    "сен",
    "сіз",
    "ол",
    "бұл",
    "сол",
    "оның",
    "сіздің",
    "менің",
    "ма",
    "ме",
    "ба",
    "бе",
    "па",
    "пе",
    "ғой",
    "да",
    "де",
    "та",
    "те",
    // Common short connectors and metawords
    "және",
    "немесе",
    "сондай",
    "сондай-ақ",
    "мысалы",
    "арналған",
    // Punctuation tokens we'll see standalone after split
    "—",
    "–",
    "-",
    ".",
    ",",
    "!",
    "?",
    ":",
    ";",
    // Honorifics / address forms (vary by user but same role)
    "ағай",
    "апай",
    "дәке",
    // Politeness wrappers — semantically equivalent to bare answer
    "рахмет",
    "рақмет",
];

/// Strip case suffixes and possessives from a content word so
/// «судың» and «суда» both reduce toward «су». Heuristic —
/// matches Kazakh morphotactics roughly. Drops trailing suffix
/// only when ≥3 chars remain.
fn root_prefix(w: &str) -> String {
    let chars: Vec<char> = w.chars().collect();
    let n = chars.len();
    if n < 4 {
        return w.to_string();
    }
    // Iteratively peel off common suffixes (longest first).
    let suffixes: &[&str] = &[
        // Possessives + cases (long → short)
        "ыңыздың",
        "іңіздің",
        "ыңызды",
        "іңізді",
        "ыңызда",
        "іңізде",
        "ыңыздан",
        "іңізден",
        "ыңыз",
        "іңіз",
        "ңыз",
        "ңіз",
        "ымыз",
        "іміз",
        "мыз",
        "міз",
        // Genitive (case used heavily for "X-тың формуласы")
        "ның",
        "нің",
        "дың",
        "дің",
        "тың",
        "тің",
        // Other cases
        "дан",
        "ден",
        "тан",
        "тен",
        "нан",
        "нен",
        "ға",
        "ге",
        "қа",
        "ке",
        "на",
        "не",
        "да",
        "де",
        "та",
        "те",
        "ды",
        "ді",
        "ты",
        "ті",
        "ны",
        "ні",
        // Possessive 1sg / 3sg / plural
        "ым",
        "ім",
        "ың",
        "ің",
        "сы",
        "сі",
        "ы",
        "і",
        // Plural
        "лар",
        "лер",
        "дар",
        "дер",
        "тар",
        "тер",
        // Verb endings (very rough)
        "майды",
        "мейді",
        "мын",
        "мін",
        "сың",
        "сің",
        "сыз",
        "сіз",
        "ады",
        "еді",
    ];
    let mut surface: String = chars.iter().collect();
    let mut shrunk = true;
    while shrunk {
        shrunk = false;
        for suf in suffixes {
            if surface.ends_with(suf) {
                let new_len_chars = surface.chars().count() - suf.chars().count();
                if new_len_chars >= 3 {
                    surface = surface.chars().take(new_len_chars).collect();
                    shrunk = true;
                    break;
                }
            }
        }
    }
    surface
}

/// Extract semantic content tokens: lowercase, normalised, split on
/// whitespace + punctuation, drop stop tokens, reduce each to its
/// rough Kazakh root via `root_prefix`. Numbers and ASCII chemistry
/// symbols pass through unchanged.
fn content_roots(s: &str) -> Vec<String> {
    let norm = normalize_for_eval(s);
    let mut out = Vec::new();
    for raw in norm.split(|c: char| {
        c.is_whitespace()
            || matches!(
                c,
                '.' | ',' | '!' | '?' | ':' | ';' | '«' | '»' | '(' | ')' | '"' | '\'' | '—' | '–'
            )
    }) {
        let tok = raw.trim();
        if tok.is_empty() {
            continue;
        }
        if tok.len() < 2 {
            continue;
        }
        if STOP_TOKENS.contains(&tok) {
            continue;
        }
        // Keep numbers + chemistry symbols as-is (don't suffix-strip).
        let is_short_chem = tok.chars().all(|c| c.is_ascii_alphanumeric());
        let root = if is_short_chem {
            tok.to_string()
        } else {
            root_prefix(tok)
        };
        out.push(root);
    }
    out
}

/// **Semantic match**: content-word root-overlap with expected
/// answer. Passes when ≥70 % of expected content roots have a
/// matching root somewhere in predicted (or vice-versa, lenient).
/// Captures "Жақсымын" vs "Жақсы", "сізді Дәке деп атаймын" vs
/// "сізді Дәке деп атаймын — қазақша", "Қостанай екен" vs
/// "Қостанай екен, түсіндім" — surface-different, semantically same.
fn semantic_match(expected: &str, predicted: &str) -> bool {
    let e_roots = content_roots(expected);
    let p_roots = content_roots(predicted);
    if e_roots.is_empty() || p_roots.is_empty() {
        // Fall back to strict for empty content (very short answers like "сәлем").
        let p = normalize_for_eval(predicted);
        let e = normalize_for_eval(expected);
        return p == e || p.contains(&e) || e.contains(&p);
    }
    let mut matched = 0;
    for e in &e_roots {
        for p in &p_roots {
            // Root or prefix match in either direction.
            if e == p || e.starts_with(p.as_str()) || p.starts_with(e.as_str()) {
                matched += 1;
                break;
            }
        }
    }
    let smaller = e_roots.len().min(p_roots.len());
    let coverage = matched as f32 / smaller.max(1) as f32;
    coverage >= 0.70
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Mirror main.rs: enable v6.2 router before constructing the
    // Conversation engine. Without this, the math_solver / system_clock
    // / FrameIndex+realiser stack stays gated off and only the v6.1
    // cascade runs.
    if std::env::var("ADAM_V6_2").is_err() {
        // SAFETY: no threads spawned yet; single write before any
        // env reader exists. Matches the pattern in voice_repl main.rs.
        unsafe { std::env::set_var("ADAM_V6_2", "1") };
        eprintln!("[respond_full] ADAM_V6_2=1 (v6.2 router enabled)");
    }

    let lex = LexiconV1::load_default()?;
    let repo = TemplateRepository::load_default()?;
    eprintln!(
        "[respond_full] lexicon + {} template families loaded",
        repo.len()
    );

    let mut conv = Conversation::new();

    if let Some(idx) = load_retrieval_index() {
        eprintln!(
            "[respond_full] retrieval: {} morphemes / {} postings indexed",
            idx.unique_morphemes, idx.total_postings
        );
        conv = conv.with_morpheme_index(idx);
    } else {
        eprintln!("[respond_full] retrieval index not found at {RETRIEVAL_INDEX_PATH}");
    }

    let (extracted, derived) = load_reasoning_chains();
    if !extracted.is_empty() || !derived.is_empty() {
        eprintln!(
            "[respond_full] reasoning: {} facts + {} derived loaded",
            extracted.len(),
            derived.len()
        );
        conv = conv.with_reasoning_chains(extracted, derived);
    }

    match adam_reasoning::world_core::load_world_core_dir(Path::new(WORLD_CORE_DIR)) {
        Ok(report) => {
            let entries: Vec<_> = report.entries.into_iter().map(|(e, _)| e).collect();
            let idx = DomainIndex::build(&entries);
            eprintln!(
                "[respond_full] world_core: {} domains / {} entries indexed",
                idx.len(),
                entries.len()
            );
            conv = conv.with_domain_index(idx);
        }
        Err(e) => {
            eprintln!("[respond_full] world_core: load failed ({e})");
        }
    }

    // Accept an optional eval-pack path as positional arg so we can
    // run multiple suites (audit / school-program / safety / …)
    // through the same production cascade without code changes.
    let eval_path: String = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_EVAL_PATH.to_string());
    let pack: EvalPack = serde_json::from_str(&std::fs::read_to_string(&eval_path)?)?;
    println!(
        "[respond_full] eval mode — {} cases from {}",
        pack.cases.len(),
        eval_path
    );

    use std::collections::BTreeMap;
    let mut strict_correct = 0;
    let mut semantic_correct = 0;
    let mut accepted_total = 0;
    // per-subject roll-up: (strict_ok, semantic_ok, total_accepted)
    let mut by_subject: BTreeMap<String, (u32, u32, u32)> = BTreeMap::new();
    for (i, c) in pack.cases.iter().enumerate() {
        // Phase 16 production calls Conversation::turn(input, lex, repo, seed).
        // Seed=42 matches the default --seed argument in the voice REPL.
        let predicted = conv.turn(&c.input, &lex, &repo, 42);
        let expected = c
            .expected_response
            .clone()
            .unwrap_or_else(|| "<none>".into());
        let strict_pass = c.was_accepted
            && c.expected_response.as_ref().is_some_and(|e| {
                let p = normalize_for_eval(&predicted);
                let e = normalize_for_eval(e);
                p == e || p.contains(&e) || e.contains(&p)
            });
        let semantic_pass = c.was_accepted
            && c.expected_response
                .as_ref()
                .is_some_and(|e| semantic_match(e, &predicted));
        if c.was_accepted {
            accepted_total += 1;
            if strict_pass {
                strict_correct += 1;
            }
            if semantic_pass {
                semantic_correct += 1;
            }
            let subj = c.subject.clone().unwrap_or_else(|| "(unspecified)".into());
            let entry = by_subject.entry(subj).or_insert((0, 0, 0));
            entry.2 += 1;
            if strict_pass {
                entry.0 += 1;
            }
            if semantic_pass {
                entry.1 += 1;
            }
        }
        let tag = if !c.was_accepted {
            "(was-rejected — any response is a probe)"
        } else {
            match (strict_pass, semantic_pass) {
                (true, true) => "✓ strict + semantic",
                (false, true) => "✓ semantic only",
                (true, false) => "✓ strict, ✗ semantic (likely false-positive)",
                (false, false) => "✗",
            }
        };
        println!(
            "#{:<3} [{}] in: «{}»\n     expected: «{}»\n     predicted: «{}»\n     {}",
            i,
            if c.was_accepted { "ACC" } else { "REJ" },
            c.input,
            expected,
            predicted,
            tag
        );
    }
    println!(
        "\n[respond_full] strict   : {}/{} = {:.0}%",
        strict_correct,
        accepted_total,
        100.0 * strict_correct as f32 / accepted_total.max(1) as f32
    );
    println!(
        "[respond_full] semantic : {}/{} = {:.0}%",
        semantic_correct,
        accepted_total,
        100.0 * semantic_correct as f32 / accepted_total.max(1) as f32
    );
    if !by_subject.is_empty() {
        println!("\n[respond_full] by subject:");
        println!(
            "{:<22} {:>8} {:>10} {:>10}",
            "subject", "n", "strict", "semantic"
        );
        println!("{}", "-".repeat(54));
        for (subj, (s_ok, sem_ok, n)) in &by_subject {
            println!(
                "{:<22} {:>8} {:>4}/{:<3} {:>4}/{:<3}",
                subj, n, s_ok, n, sem_ok, n
            );
        }
    }

    Ok(())
}
