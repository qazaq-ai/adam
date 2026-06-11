// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! # `adam_blind_eval` — v6.5.0-rc14 evaluation harness
//!
//! Loads [`data/eval/blind_eval_v1.json`](../../../../data/eval/blind_eval_v1.json),
//! runs each item through the same `Conversation` pipeline `adam_chat` uses,
//! scores each item by its declared `match` type, and writes a per-category +
//! overall accuracy report to stdout.  This is the external-audit
//! recommendation #4 (2026-06-10): a same-task scoreboard adam can be graded
//! on (and that LLMs / Llama / Claude / GPT can later be graded on for fair
//! comparison).
//!
//! Run from repo root:
//!
//! ```sh
//! cargo run --release --bin adam_blind_eval
//! ```
//!
//! Optional positional arg: an alternative pack path (defaults to
//! `data/eval/blind_eval_v1.json`).
//!
//! Exit code is `0` iff every item was correctly handled.  Non-zero exit
//! flags a real regression that should be reviewed before tagging a
//! release.  Use `--allow-failures` (one positional flag after the pack
//! arg) to override and force exit 0 — useful while the pack is being
//! grown but the kernel hasn't caught up yet.

use std::path::Path;

use adam_dialog::{Conversation, DomainIndex, TemplateRepository};
use adam_kernel_fst::lexicon::LexiconV1;
use adam_kernel_fst::root_affinity::RootAffinity;
use adam_kernel_fst::suffix_priors::SuffixPriors;
use adam_reasoning::Fact as ReasFact;
use adam_reasoning::reasoner::DerivedFact;
use adam_retrieval::MorphemeIndex;
use serde::Deserialize;

const DEFAULT_PACK: &str = "data/eval/blind_eval_v1.json";

#[derive(Debug, Deserialize)]
struct Pack {
    #[allow(dead_code)]
    version: String,
    #[allow(dead_code)]
    name: String,
    categories: Vec<String>,
    items: Vec<Item>,
}

#[derive(Debug, Deserialize, Clone)]
struct Item {
    id: String,
    category: String,
    query: String,
    #[serde(rename = "match")]
    match_kind: String,
    #[serde(default)]
    targets: Vec<String>,
    #[serde(default)]
    target_number: Option<i64>,
    #[allow(dead_code)]
    #[serde(default)]
    rationale: String,
}

/// Phrases adam emits when it refuses to answer.  Closed list grounded in
/// the actual templates that ship from `adam-dialog/src/templates/*` and
/// the wellness escalation copy.  A reply is considered a refusal iff its
/// lowercased form contains at least one of these substrings.
const REFUSAL_PATTERNS: &[&str] = &[
    "нақты дерегім жоқ",
    "білмеймін",
    "айта алмаймын",
    "орындай алмаймын",
    "мүмкіндігімде жоқ",
    "мүмкіндігім жоқ",
    "көмектесе алмаймын",
    "маман адаммен",
    "сенім телефоны",
    "112",
    "150",
    "сұрағыңызды дұрыс түсінбедім",
    "сұрағыңызды толық түсінбедім",
    "басқаша айтып көріңізші",
    "басқа сөздермен",
    "ондай мәлімет",
    // **v6.5.0-rc17 — additional refusal patterns from rc16 audit.**
    // adam emits these on out-of-corpus / unknown topics; the pattern
    // is honest refusal even when the exact «нақты дерегім жоқ»
    // wording isn't used.  Each is a strong signal: «дерек жоқ» =
    // "no data", «білім қорымда» = "in my knowledge base", «басқаша
    // қойсаңыз» = "if you ask differently".
    "дерек жоқ",
    "білім қорымда",
    "сұрағыңызды басқаша",
    "ағымдағы / реалды-уақыттық",
    "live-feed қосылған емес",
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let pack_path = args.next().unwrap_or_else(|| DEFAULT_PACK.to_string());
    let allow_failures = args.next().as_deref() == Some("--allow-failures");

    // **v6.5.0-rc19 — restore explicit env-var set.**  rc19 was
    // going to flip the global default to ON, but the change
    // exposed several v6.1→v6.2 regressions in
    // `tests/{cognitive_eval, adversarial_dialog_v1, curriculum_*}`
    // that need rc20+ to close.  Until then production binaries
    // (this one, voice REPL, `adam_chat`) opt in explicitly; the
    // library default stays OFF so test consumers see no
    // regression.
    //
    // SAFETY: only env-var mutation in the process, before any
    // thread spawns.
    unsafe {
        std::env::set_var("ADAM_V6_2", "1");
    }

    eprintln!("[eval] pack: {pack_path}");
    let pack: Pack = serde_json::from_str(&std::fs::read_to_string(&pack_path)?)?;
    eprintln!(
        "[eval] {} items across {} categories",
        pack.items.len(),
        pack.categories.len()
    );

    // ----- runtime load (mirrors adam_resource_bench) ---------------------

    let lex = load_lexicon()?;
    let repo = TemplateRepository::load_default()?;

    let mut index: MorphemeIndex = serde_json::from_str(&std::fs::read_to_string(
        "data/retrieval/morpheme_index.json",
    )?)?;
    index.refresh_stats();
    let extracted: Vec<ReasFact> = load_field("data/retrieval/facts.json", "facts")?;
    let derived: Vec<DerivedFact> = load_field("data/retrieval/derived_facts.json", "derived")?;
    let priors = SuffixPriors::load(Path::new("data/retrieval/suffix_chain_priors.json"))?;
    let affinity = RootAffinity::load(Path::new("data/retrieval/root_affinity.json")).ok();
    let domain_idx = build_domain_index();

    let mut conv = Conversation::new()
        .with_morpheme_index(index)
        .with_reasoning_chains(extracted, derived)
        .with_suffix_priors(priors)
        .with_priors_alpha(0.3)
        .with_domain_index(domain_idx);
    if let Some(aff) = affinity {
        conv = conv.with_root_affinity(aff);
    }

    eprintln!("[eval] runtime ready; scoring…");

    // ----- score each item ------------------------------------------------

    struct Outcome<'a> {
        item: &'a Item,
        reply: String,
        ok: bool,
        reason: &'static str,
    }

    let mut outcomes: Vec<Outcome<'_>> = Vec::with_capacity(pack.items.len());

    for item in &pack.items {
        // Each item runs in a FRESH conversation — single-turn semantics.
        // The session-bearing variant lands in a future multi-turn pack.
        let mut fresh = conv.clone();
        let reply = fresh.turn(&item.query, &lex, &repo, 0);
        let (ok, reason) = score_item(item, &reply);
        outcomes.push(Outcome {
            item,
            reply,
            ok,
            reason,
        });
    }

    // ----- aggregate ------------------------------------------------------

    println!("# adam Blind Eval — v{}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Pack:  `{pack_path}`");
    println!("Items: {}", pack.items.len());
    println!();

    let mut by_cat: std::collections::BTreeMap<&str, (usize, usize)> =
        std::collections::BTreeMap::new();
    for o in &outcomes {
        let entry = by_cat.entry(o.item.category.as_str()).or_default();
        entry.0 += 1;
        if o.ok {
            entry.1 += 1;
        }
    }

    println!("## Per-category accuracy");
    println!();
    println!("| Category | Pass | Total | Accuracy |");
    println!("|---|---|---|---|");
    for (cat, (total, pass)) in &by_cat {
        let acc = if *total > 0 {
            (*pass as f64 / *total as f64) * 100.0
        } else {
            0.0
        };
        println!("| {cat} | {pass} | {total} | {acc:.1} % |");
    }
    let total_pass: usize = outcomes.iter().filter(|o| o.ok).count();
    let acc_overall = (total_pass as f64 / outcomes.len() as f64) * 100.0;
    println!(
        "| **overall** | **{total_pass}** | **{}** | **{acc_overall:.1} %** |",
        outcomes.len()
    );
    println!();

    println!("## Failures ({})", outcomes.len() - total_pass);
    println!();
    if total_pass == outcomes.len() {
        println!("(none — every item passed)");
    } else {
        println!("| ID | Category | Query | Reason | Reply (first 80 chars) |");
        println!("|---|---|---|---|---|");
        for o in outcomes.iter().filter(|o| !o.ok) {
            let trimmed: String = o.reply.chars().take(80).collect();
            let trimmed = trimmed.replace('|', "\\|");
            println!(
                "| {} | {} | `{}` | {} | `{}` |",
                o.item.id,
                o.item.category,
                o.item.query.replace('|', "\\|"),
                o.reason,
                trimmed
            );
        }
    }

    if !allow_failures && total_pass < outcomes.len() {
        std::process::exit(1);
    }
    Ok(())
}

/// Returns `(ok, reason)`.  The reason is a short tag that explains WHICH
/// rule fired — useful in the failures table so the user can tell a
/// targets-miss from a refusal-miss at a glance.
fn score_item(item: &Item, reply: &str) -> (bool, &'static str) {
    let lower = reply.to_lowercase();
    match item.match_kind.as_str() {
        "contains_any" => {
            if item.targets.is_empty() {
                return (false, "no targets");
            }
            let hit = item
                .targets
                .iter()
                .any(|t| lower.contains(&t.to_lowercase()));
            if hit {
                (true, "ok")
            } else {
                (false, "no_target_match")
            }
        }
        "contains_numeric" => {
            let Some(n) = item.target_number else {
                return (false, "no target_number");
            };
            let needle = n.to_string();
            // A reply like «5 жас» / «25» / «√4 = 2» all contain the digit;
            // we accept any substring digit match.  False-positive risk is
            // low for numeric tutor items.
            if reply.contains(&needle) {
                (true, "ok")
            } else {
                (false, "no_numeric_match")
            }
        }
        "refuse" => {
            if REFUSAL_PATTERNS.iter().any(|p| lower.contains(p)) {
                (true, "ok")
            } else {
                (false, "no_refusal")
            }
        }
        other => {
            eprintln!("[eval] unknown match kind «{other}» on item {}", item.id);
            (false, "unknown_match")
        }
    }
}

fn load_lexicon() -> Result<LexiconV1, Box<dyn std::error::Error>> {
    let curated = Path::new("data/tokenizer/segmentation_roots.json");
    let apertium = Path::new("data/lexicon_v1/apertium_imported_roots.json");
    Ok(LexiconV1::load(curated, apertium)?)
}

fn load_field<T: serde::de::DeserializeOwned>(
    path: &str,
    field: &str,
) -> Result<Vec<T>, Box<dyn std::error::Error>> {
    let raw = std::fs::read_to_string(path)?;
    let v: serde_json::Value = serde_json::from_str(&raw)?;
    let arr = v
        .get(field)
        .and_then(|x| x.as_array())
        .ok_or_else(|| format!("{path}: missing array field {field}"))?;
    Ok(arr
        .iter()
        .filter_map(|item| serde_json::from_value::<T>(item.clone()).ok())
        .collect())
}

fn build_domain_index() -> DomainIndex {
    let world_core_dir = Path::new("data/world_core");
    if !world_core_dir.exists() {
        return DomainIndex::default();
    }
    match adam_reasoning::world_core::load_world_core_dir(world_core_dir) {
        Ok(report) => {
            let entries: Vec<_> = report.entries.into_iter().map(|(e, _)| e).collect();
            DomainIndex::build(&entries)
        }
        Err(_) => DomainIndex::default(),
    }
}
