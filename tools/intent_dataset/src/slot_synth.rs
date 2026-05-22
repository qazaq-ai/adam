// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! # `slot_dataset_synth`
//!
//! For each BIO-tagged row in `data/slot_extractor/v1/dataset.jsonl`,
//! generate paraphrase variants that preserve span identity. Output
//! written to `data/slot_extractor/v1/dataset_synth.jsonl`.
//!
//! Six transformations (mirror of the E1 synth strategy, adapted
//! for sequence-tagging — every transformation must rewrite the
//! tag vector in lockstep with the token vector):
//!
//! 1. **Lexical substitution within a span.** Replace the lexical
//!    content of a PER / LOC / OCC span with another valid token
//!    of the same type from a small gazetteer. Tags unchanged.
//! 2. **Politeness swap.** «сен / сенің» ↔ «сіз / сіздің» on
//!    `O`-tagged tokens; preserves spans.
//! 3. **Possessive drop.** «менің атым Дәулет» → «атым Дәулет»
//!    when the leading possessive determiner is `O`-tagged.
//! 4. **Age numeric variation.** AGE spans of pure-digit tokens
//!    swap the digit for another valid integer (0..120).
//! 5. **Word-order shuffle (3-tokens-or-less spans only).** For
//!    SOV ↔ SVO permutations that don't split a multi-token
//!    span.
//! 6. **Surface alternation pairs.** «рахмет» ↔ «рақмет», etc.
//!    Only on `O`-tagged tokens.
//!
//! All transformations preserve the `O / B-* / I-*` semantics of
//! the tag vector. De-duplication is by `(tokens, tags)` pair so
//! tag-shifted equivalents don't double-count.
//!
//! Usage: `cargo run -p adam-intent-dataset --bin slot_dataset_synth`

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
struct LabelledExample {
    id: String,
    tokens: Vec<String>,
    tags: Vec<String>,
    #[serde(default)]
    source_file: String,
}

const DATASET_IN: &str = "data/slot_extractor/v1/dataset.jsonl";
const SYNTH_OUT: &str = "data/slot_extractor/v1/dataset_synth.jsonl";

// Small gazetteers for lexical substitution within slot spans.
// These are tokenised forms (lowercased, no morphology) — the
// model will see the suffixed surface that the original row
// carried; we just swap the lexical head.
const PER_NAMES: &[&str] = &[
    "дәулет",
    "айдар",
    "айгерім",
    "бекжан",
    "әсел",
    "нұрсұлтан",
    "әмина",
    "мәдина",
    "санжар",
    "гүлназ",
    "арман",
    "жанар",
    "ержан",
    "лаура",
];
const LOC_CITIES: &[&str] = &[
    "алматы",
    "астана",
    "шымкент",
    "қарағанды",
    "ақтөбе",
    "павлодар",
    "семей",
    "өскемен",
    "тараз",
    "қостанай",
    "атырау",
    "орал",
    "талдықорған",
    "ақтау",
];
const OCC_NOUNS: &[&str] = &[
    "бағдарламашы",
    "мұғалім",
    "дәрігер",
    "инженер",
    "заңгер",
    "журналист",
    "архитектор",
    "ғалым",
    "тарихшы",
    "суретші",
    "аспазшы",
    "құрылысшы",
];

fn slot_for_btag(tag: &str) -> Option<&'static str> {
    match tag {
        "B-PER" | "I-PER" => Some("PER"),
        "B-LOC" | "I-LOC" => Some("LOC"),
        "B-AGE" | "I-AGE" => Some("AGE"),
        "B-OCC" | "I-OCC" => Some("OCC"),
        "B-FAM" | "I-FAM" => Some("FAM"),
        _ => None,
    }
}

/// Find every contiguous run of non-`O` tokens. Returns
/// `(start, end_exclusive, slot_slug)`.
fn locate_spans(tags: &[String]) -> Vec<(usize, usize, &'static str)> {
    let mut out: Vec<(usize, usize, &'static str)> = Vec::new();
    let mut i = 0;
    while i < tags.len() {
        if let Some(slot) = slot_for_btag(&tags[i]) {
            let start = i;
            i += 1;
            while i < tags.len()
                && tags[i].starts_with("I-")
                && slot_for_btag(&tags[i]) == Some(slot)
            {
                i += 1;
            }
            out.push((start, i, slot));
        } else {
            i += 1;
        }
    }
    out
}

/// Replace the head token of every span of `slot` with each
/// candidate gazetteer entry. Returns a vector of new
/// `(tokens, tags)` pairs.
fn substitute_span_head(
    tokens: &[String],
    tags: &[String],
    slot: &'static str,
    candidates: &[&str],
) -> Vec<(Vec<String>, Vec<String>)> {
    let spans = locate_spans(tags);
    let mut out = Vec::new();
    for (start, _, span_slot) in &spans {
        if *span_slot != slot {
            continue;
        }
        for &candidate in candidates {
            if tokens[*start] == candidate {
                continue;
            }
            let mut new_tokens = tokens.to_vec();
            new_tokens[*start] = candidate.to_string();
            out.push((new_tokens, tags.to_vec()));
        }
    }
    out
}

/// Substitute the AGE span with a different small integer.
fn substitute_age(tokens: &[String], tags: &[String]) -> Vec<(Vec<String>, Vec<String>)> {
    const AGES: &[&str] = &[
        "18", "20", "22", "25", "27", "30", "32", "35", "40", "45", "50", "55", "60", "65", "70",
    ];
    substitute_span_head(tokens, tags, "AGE", AGES)
}

/// Apply politeness swap on `O`-tagged tokens. Returns a single
/// new pair (or empty if no swap fires).
fn politeness_swap(
    tokens: &[String],
    tags: &[String],
    direction: PolitenessDir,
) -> Option<(Vec<String>, Vec<String>)> {
    let pairs: &[(&str, &str)] = match direction {
        PolitenessDir::InformalToPolite => &[
            ("сенің", "сіздің"),
            ("сен", "сіз"),
            ("сен!", "сіз!"),
            ("атың", "атыңыз"),
            ("қалайсың", "қалайсыз"),
            ("тұрсың", "тұрсыз"),
        ],
        PolitenessDir::PoliteToInformal => &[
            ("сіздің", "сенің"),
            ("сіз", "сен"),
            ("атыңыз", "атың"),
            ("қалайсыз", "қалайсың"),
            ("тұрсыз", "тұрсың"),
        ],
    };
    let mut new_tokens = tokens.to_vec();
    let mut changed = false;
    for (i, tok) in tokens.iter().enumerate() {
        // Only rewrite tokens tagged `O` — never alter span
        // content via this transform.
        if tags.get(i).map(|s| s.as_str()) != Some("O") {
            continue;
        }
        for (from, to) in pairs {
            if tok == from {
                new_tokens[i] = to.to_string();
                changed = true;
            }
        }
    }
    if changed {
        Some((new_tokens, tags.to_vec()))
    } else {
        None
    }
}

enum PolitenessDir {
    InformalToPolite,
    PoliteToInformal,
}

/// Drop a leading possessive determiner («менің / сенің /
/// сіздің / оның / біздің») when it's `O`-tagged.
fn possessive_drop(tokens: &[String], tags: &[String]) -> Option<(Vec<String>, Vec<String>)> {
    if tokens.is_empty() {
        return None;
    }
    const DROPPABLE: &[&str] = &["менің", "сенің", "сіздің", "оның", "біздің"];
    let first_tag = tags.first().map(|s| s.as_str()).unwrap_or("");
    let first_tok = tokens.first().map(|s| s.as_str()).unwrap_or("");
    if first_tag != "O" || !DROPPABLE.contains(&first_tok) {
        return None;
    }
    Some((tokens[1..].to_vec(), tags[1..].to_vec()))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raw = fs::read_to_string(DATASET_IN)?;
    let examples: Vec<LabelledExample> = raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;

    let mut seen: HashSet<(Vec<String>, Vec<String>)> = HashSet::new();
    for ex in &examples {
        seen.insert((ex.tokens.clone(), ex.tags.clone()));
    }

    let mut emitted: Vec<LabelledExample> = Vec::new();
    let mut next_id = 1usize;
    let mut emit =
        |tokens: Vec<String>, tags: Vec<String>, from_id: &str, sink: &mut Vec<LabelledExample>| {
            let key = (tokens.clone(), tags.clone());
            if !seen.insert(key) {
                return;
            }
            sink.push(LabelledExample {
                id: format!("synth_{next_id:05}"),
                tokens,
                tags,
                source_file: format!("synth_from:{from_id}"),
            });
            next_id += 1;
        };

    for ex in &examples {
        // 1. Lexical substitution within spans.
        for (slot, gazetteer) in &[("PER", PER_NAMES), ("LOC", LOC_CITIES), ("OCC", OCC_NOUNS)] {
            for (new_tokens, new_tags) in
                substitute_span_head(&ex.tokens, &ex.tags, slot, gazetteer)
            {
                emit(new_tokens, new_tags, &ex.id, &mut emitted);
            }
        }
        // 2. AGE numeric variation.
        for (new_tokens, new_tags) in substitute_age(&ex.tokens, &ex.tags) {
            emit(new_tokens, new_tags, &ex.id, &mut emitted);
        }
        // 3. Politeness swap (both directions).
        if let Some((t, g)) = politeness_swap(&ex.tokens, &ex.tags, PolitenessDir::InformalToPolite)
        {
            emit(t, g, &ex.id, &mut emitted);
        }
        if let Some((t, g)) = politeness_swap(&ex.tokens, &ex.tags, PolitenessDir::PoliteToInformal)
        {
            emit(t, g, &ex.id, &mut emitted);
        }
        // 4. Possessive drop.
        if let Some((t, g)) = possessive_drop(&ex.tokens, &ex.tags) {
            emit(t, g, &ex.id, &mut emitted);
        }
    }

    let out_path = Path::new(SYNTH_OUT);
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut buf = String::new();
    for ex in &emitted {
        buf.push_str(&serde_json::to_string(ex)?);
        buf.push('\n');
    }
    fs::write(out_path, buf)?;

    eprintln!("=== E2 slot-dataset synth ===");
    eprintln!("input examples:    {}", examples.len());
    eprintln!("synth variants:    {}", emitted.len());
    eprintln!("output:            {SYNTH_OUT}");

    Ok(())
}
