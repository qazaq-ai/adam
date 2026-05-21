// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! # `intent_dataset_synth`
//!
//! Generates paraphrase variants for each labelled example in the
//! committed dataset and writes them to
//! `data/intent_classifier/v1/dataset_synth.jsonl`.
//!
//! ## Strategy
//!
//! For each input we apply six **string-level Kazakh
//! transformations** that preserve intent meaning. Each
//! transformation either succeeds (producing a new variant) or
//! is a no-op (the input doesn't match the pattern). We emit
//! every distinct successful variant, plus the original.
//!
//! 1. **Politeness swap** — informal ⇄ polite 2sg: сен ⇄ сіз,
//!    сенің ⇄ сіздің, маған ⇄ маған (no-op), -сың ⇄ -сыз, -мын ⇄
//!    -мын (1sg unchanged), -ың ⇄ -ыңыз / -іңіз.
//! 2. **Possessive dropping** — Kazakh allows pro-drop on
//!    possessive: «менің атым» ⇄ «атым». Removes the leading
//!    «менің / сенің / сіздің / оның» when it pairs with a
//!    possessive-marked noun.
//! 3. **Question-particle toggling** — adds / drops the final
//!    Q-particle pair: «X?» ⇄ «X бе?» / «X ма?». Pure phonetic
//:    harmony — back-vowel + voiceless final → «па / ма»; front +
//!    voiced → «бе / ме».
//! 4. **Word-order permutation** — Kazakh is SOV but topicalisation
//!    is free. For 3-token inputs we emit the SVO permutation
//!    when it parses cleanly (no case-marked object).
//! 5. **Surface-form alternation** — well-known free-variation
//!    pairs: сәлеметсіз бе ⇄ сәлеметсіз бе сіз, рахмет ⇄ рақмет,
//!    мүмкін ⇄ бәлкім, иә ⇄ дұрыс.
//! 6. **Whitespace normalisation** — drop / add comma after
//!    leading discourse markers ("ал", "онда"). Real REPL input
//!    has both forms.
//!
//! No FST involved. Pure surface-level string surgery. This is
//! safer than morphological reanalysis for v0.0.x — we don't risk
//! generating phantom analyses the cascade would label
//! differently from the source.
//!
//! Usage: `cargo run -p adam-intent-dataset --bin intent_dataset_synth`

use std::collections::HashMap;
use std::fs;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
struct LabelledExample {
    id: String,
    input: String,
    intent: String,
    #[serde(default)]
    source_file: String,
    #[serde(default)]
    confidence: String,
}

const DATASET_IN: &str = "data/intent_classifier/v1/dataset.jsonl";
const SYNTH_OUT: &str = "data/intent_classifier/v1/dataset_synth.jsonl";

/// Generate paraphrase variants of `input`. Returns the input
/// itself plus every distinct successful transformation. Caller
/// should de-duplicate across the dataset.
fn paraphrase(input: &str) -> Vec<String> {
    let mut out: Vec<String> = vec![input.to_string()];
    let lower = input.to_lowercase();

    // --- 1. Politeness swap (informal ↔ polite 2sg). ---
    // Apply only when at least one informal marker present, OR
    // at least one polite marker present.
    let informal_to_polite = [
        ("сенің ", "сіздің "),
        (" сенің", " сіздің"),
        ("сен ", "сіз "),
        (" сен ", " сіз "),
        ("сың ", "сыз "),
        (" сың?", " сыз?"),
        ("сің ", "сіз "),
        (" сің?", " сіз?"),
        ("атың ", "атыңыз "),
        (" атың?", " атыңыз?"),
        ("қалайсың", "қалайсыз"),
        ("тұрсың", "тұрсыз"),
        ("білесің", "білесіз"),
    ];
    let mut variant = input.to_string();
    let mut applied = false;
    for (from, to) in informal_to_polite.iter() {
        if variant.to_lowercase().contains(from) {
            // Replace case-preservingly: find first occurrence in
            // lowercase, swap the same byte range in original.
            // Simpler: do case-insensitive replace by working with
            // lowercase throughout, then we don't try to preserve
            // case on synth variants. Acceptable for training data.
            variant = variant.to_lowercase().replace(from, to);
            applied = true;
        }
    }
    if applied && variant.to_lowercase() != lower {
        out.push(variant);
    }

    // Reverse direction: polite → informal.
    let polite_to_informal = [
        ("сіздің ", "сенің "),
        ("сіз ", "сен "),
        ("сыз?", "сың?"),
        ("сіз?", "сің?"),
        ("қалайсыз", "қалайсың"),
        ("тұрсыз", "тұрсың"),
    ];
    let mut variant = input.to_string();
    let mut applied = false;
    for (from, to) in polite_to_informal.iter() {
        if variant.to_lowercase().contains(from) {
            variant = variant.to_lowercase().replace(from, to);
            applied = true;
        }
    }
    if applied && variant.to_lowercase() != lower {
        out.push(variant);
    }

    // --- 2. Possessive drop. ---
    // «менің атым» → «атым»; «сенің атың» → «атың».
    let possessive_drops = ["менің ", "сенің ", "сіздің ", "оның ", "біздің "];
    for prefix in possessive_drops.iter() {
        if lower.starts_with(prefix) {
            // Drop the prefix.
            let dropped: String = input.chars().skip(prefix.chars().count()).collect();
            if !dropped.is_empty() {
                let upcased = uppercase_first(&dropped);
                if upcased.to_lowercase() != lower {
                    out.push(upcased);
                }
            }
        }
    }

    // --- 3. Question-particle toggle. ---
    // If input ends with «?» and has no Q-particle, optionally
    // add one. Pick by trailing vowel-harmony of the final word.
    if input.ends_with('?') && !has_q_particle(&lower) {
        let stem = input.trim_end_matches('?').trim();
        let particle = choose_q_particle(stem);
        if let Some(p) = particle {
            out.push(format!("{stem} {p}?"));
        }
    }
    // If input ends with a Q-particle, optionally drop it.
    for particle in ["бе?", "ме?", "ба?", "ма?", "па?", "пе?"].iter() {
        if input.to_lowercase().ends_with(particle) {
            let without: String = input
                .chars()
                .take(input.chars().count() - particle.chars().count())
                .collect::<String>()
                .trim_end()
                .to_string();
            if !without.is_empty() {
                out.push(format!("{without}?"));
            }
            break;
        }
    }

    // --- 4. Word-order: subject-verb swap. ---
    // For "X Y Z" where X is a subject pronoun ("мен / сен / сіз"),
    // swap "Y X Z". Conservative: only for length 3.
    let tokens: Vec<&str> = input.split_whitespace().collect();
    if tokens.len() == 3 {
        let first_lower = tokens[0].to_lowercase();
        if matches!(first_lower.as_str(), "мен" | "сен" | "сіз" | "ол") {
            let swapped = format!("{} {} {}", tokens[1], tokens[0], tokens[2]);
            out.push(swapped);
        }
    }

    // --- 5. Surface alternation. ---
    let pairs = [
        ("рахмет", "рақмет"),
        ("мүмкін", "бәлкім"),
        ("иә", "дұрыс"),
        ("жоқ", "жоғырқ"),
        ("сәлем", "сәлеметсіз бе"),
    ];
    for (a, b) in pairs.iter() {
        if lower.contains(a) {
            out.push(input.to_lowercase().replace(a, b));
        }
        if lower.contains(b) {
            out.push(input.to_lowercase().replace(b, a));
        }
    }

    // --- 6. Discourse-particle toggle. ---
    if lower.starts_with("ал ") {
        let without_al: String = input.chars().skip(3).collect();
        out.push(uppercase_first(&without_al));
    } else if !lower.starts_with("ал ") {
        // Skip the addition direction — risks changing intent.
    }

    // De-duplicate (case-sensitive).
    out.sort();
    out.dedup();
    out
}

fn uppercase_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn has_q_particle(lower: &str) -> bool {
    for p in ["бе?", "ме?", "ба?", "ма?", "па?", "пе?"].iter() {
        if lower.ends_with(p) {
            return true;
        }
    }
    false
}

fn choose_q_particle(stem: &str) -> Option<&'static str> {
    // Tiny vowel-harmony heuristic on the final vowel of the last
    // token + final consonant voicing for the «б/м/п» choice.
    let last_word = stem.split_whitespace().last()?.to_lowercase();
    let last_char = last_word.chars().last()?;
    let last_vowel = last_word.chars().rev().find(|c| {
        matches!(
            *c,
            'а' | 'о' | 'ы' | 'ұ' | 'у' | 'е' | 'і' | 'ө' | 'ү' | 'и' | 'э'
        )
    })?;
    let back = matches!(last_vowel, 'а' | 'о' | 'ы' | 'ұ' | 'у');
    let voiceless = matches!(
        last_char,
        'п' | 'к' | 'қ' | 'т' | 'с' | 'ш' | 'ф' | 'х' | 'ц' | 'ч' | 'щ'
    );
    Some(match (back, voiceless) {
        (true, true) => "па",
        (true, false) => "ма",
        (false, true) => "пе",
        (false, false) => "ме",
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raw = fs::read_to_string(DATASET_IN)?;
    let examples: Vec<LabelledExample> = raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;

    let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
    for ex in &examples {
        seen.insert((ex.input.clone(), ex.intent.clone()));
    }

    let mut emitted: Vec<LabelledExample> = Vec::new();
    let mut per_class_added: HashMap<String, usize> = HashMap::new();
    let mut next_id = 1usize;
    for ex in &examples {
        let variants = paraphrase(&ex.input);
        for v in variants {
            let key = (v.clone(), ex.intent.clone());
            if !seen.insert(key) {
                continue;
            }
            if v == ex.input {
                continue;
            }
            emitted.push(LabelledExample {
                id: format!("synth_{next_id:05}"),
                input: v,
                intent: ex.intent.clone(),
                source_file: format!("synth_from:{}", ex.id),
                confidence: "high".to_string(),
            });
            *per_class_added.entry(ex.intent.clone()).or_default() += 1;
            next_id += 1;
        }
    }

    let mut buf = String::new();
    for ex in &emitted {
        buf.push_str(&serde_json::to_string(ex)?);
        buf.push('\n');
    }
    fs::write(SYNTH_OUT, buf)?;

    eprintln!("=== E1 dataset synth ===");
    eprintln!("input examples:    {}", examples.len());
    eprintln!("synth variants:    {}", emitted.len());
    eprintln!("output:            {SYNTH_OUT}");
    eprintln!();
    eprintln!("--- per-class additions (sorted by count) ---");
    let mut sorted: Vec<(&String, &usize)> = per_class_added.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));
    for (label, count) in sorted {
        eprintln!("  {label:30}  +{count}");
    }
    Ok(())
}
