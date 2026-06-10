// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! # `synth_corrections_pack` — v6.5.0-rc9 audit-to-training synthesis
//!
//! Closes the rc5–rc8 self-learning loop's synthesis stage.
//!
//! ## Pipeline
//!
//! ```text
//!   data/mistake_corrections.jsonl          rc5–rc7 — produced by the
//!     ┌──────────────────────────────┐      voice REPL after the
//!     │ wrong_input                  │      rejection detector fires
//!     │ wrong_intent                 │
//!     │ wrong_output                 │      (raw, not training-clean)
//!     │ rejection_kind, hint, ts     │
//!     └──────────────────────────────┘
//!                  │
//!                  ▼
//!   data/mistake_corrections_labels.jsonl   curated by hand
//!     ┌──────────────────────────────┐
//!     │ wrong_input (match key)      │
//!     │ verdict                      │      detector_false_positive |
//!     │ correct_intent  (if known)   │      cascade_issue | intent_mistake
//!     │ rationale                    │
//!     └──────────────────────────────┘
//!                  │
//!                  ▼
//!     ┌──────────────────────────────┐
//!     │ this tool                    │      augments each training-worthy
//!     │  - paraphrase                │      record into N variants
//!     │  - STT noise                 │
//!     │  - inflection drift          │
//!     │  - punctuation jitter        │
//!     └──────────────────────────────┘
//!                  │
//!                  ▼
//!   data/curated/adam_intent_training_pack_augmented.json
//!     ┌──────────────────────────────┐
//!     │ original 2 914 samples       │
//!     │ + N synthesised samples      │      tagged source="corrections_synth"
//!     └──────────────────────────────┘
//!                  │
//!                  ▼
//!   train_intent_classifier_gpu   (separate run)
//! ```
//!
//! ## Verdict policy
//!
//! - `detector_false_positive` — skip entirely.  The detector misfired;
//!   there is no labelling signal here.  Augmenting would teach the
//!   model nothing or worse.
//! - `cascade_issue` — augment under `correct_intent` to REINFORCE the
//!   existing correct label.  The intent was right; the bug was
//!   downstream in cascade routing.  Augmentation tightens the
//!   classifier's confidence on similar inputs so the downstream router
//!   has a stronger signal to work with.
//! - `intent_mistake` — augment aggressively under `correct_intent`.
//!   This is the only case where the classifier is genuinely wrong;
//!   priority training material.
//!
//! ## Augmentation strategies
//!
//! Each is a closed, deterministic function from input string to a
//! `Vec<String>` of variants.  Applied independently then dedup'd.  See
//! the individual fns for rationale on each Kazakh-specific rule.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

const CORRECTIONS_PATH: &str = "data/mistake_corrections.jsonl";
const LABELS_PATH: &str = "data/mistake_corrections_labels.jsonl";
const SOURCE_PACK_PATH: &str = "data/curated/adam_intent_training_pack.json";
const OUTPUT_PACK_PATH: &str = "data/curated/adam_intent_training_pack_augmented.json";

/// Aggressive vs reinforcing — controls how many variants to emit per
/// curated record.  Real intent mistakes need more weight in the
/// training distribution; cascade issues just need their existing
/// label tightened.
const VARIANTS_INTENT_MISTAKE: usize = 50;
const VARIANTS_CASCADE_ISSUE: usize = 20;

#[derive(Debug, Deserialize)]
struct Correction {
    wrong_input: String,
    #[allow(dead_code)]
    wrong_intent: Option<String>,
    #[allow(dead_code)]
    rejection_kind: String,
    #[allow(dead_code)]
    rejection_hint: String,
}

#[derive(Debug, Deserialize)]
struct Label {
    wrong_input: String,
    verdict: String,
    correct_intent: Option<String>,
    #[allow(dead_code)]
    rationale: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Sample {
    text: String,
    intent: String,
    source: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TrainingPack {
    version: String,
    name: String,
    target_language: String,
    script: String,
    intents: Vec<String>,
    sample_count: usize,
    samples: Vec<Sample>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let corrections = read_jsonl::<Correction>(CORRECTIONS_PATH)?;
    let labels = read_jsonl::<Label>(LABELS_PATH)?;
    let mut pack: TrainingPack = serde_json::from_str(&std::fs::read_to_string(SOURCE_PACK_PATH)?)?;

    eprintln!(
        "[synth] {} corrections, {} labels, {} base samples",
        corrections.len(),
        labels.len(),
        pack.samples.len()
    );

    let mut added: Vec<Sample> = Vec::new();
    let mut by_verdict = [0usize; 3]; // intent_mistake, cascade_issue, false_positive

    for c in &corrections {
        let Some(l) = labels.iter().find(|l| l.wrong_input == c.wrong_input) else {
            eprintln!("[synth] WARN: no label for «{}» — skipping", c.wrong_input);
            continue;
        };
        let (n_variants, intent) = match (l.verdict.as_str(), &l.correct_intent) {
            ("intent_mistake", Some(intent)) => {
                by_verdict[0] += 1;
                (VARIANTS_INTENT_MISTAKE, intent.clone())
            }
            ("cascade_issue", Some(intent)) => {
                by_verdict[1] += 1;
                (VARIANTS_CASCADE_ISSUE, intent.clone())
            }
            ("detector_false_positive", _) => {
                by_verdict[2] += 1;
                eprintln!("[synth] skip false-positive: «{}»", c.wrong_input);
                continue;
            }
            (other, _) => {
                eprintln!(
                    "[synth] WARN: unknown verdict «{}» on «{}» — skipping",
                    other, c.wrong_input
                );
                continue;
            }
        };
        if !pack.intents.contains(&intent) {
            eprintln!("[synth] WARN: intent «{}» not in pack — adding", intent);
            pack.intents.push(intent.clone());
        }
        let variants = augment(&c.wrong_input, n_variants);
        eprintln!(
            "[synth] «{}» → {} variants under intent={}",
            c.wrong_input,
            variants.len(),
            intent
        );
        for v in variants {
            added.push(Sample {
                text: v,
                intent: intent.clone(),
                source: "corrections_synth".into(),
            });
        }
    }

    eprintln!(
        "[synth] verdict summary: intent_mistake={} cascade_issue={} false_positive={}",
        by_verdict[0], by_verdict[1], by_verdict[2]
    );
    eprintln!("[synth] adding {} synthesised samples to pack", added.len());

    pack.samples.extend(added);
    pack.sample_count = pack.samples.len();
    pack.version = format!("{}-augmented-rc9", pack.version);
    pack.name = format!("{}-augmented", pack.name);

    let json = serde_json::to_string_pretty(&pack)?;
    std::fs::write(OUTPUT_PACK_PATH, json)?;
    eprintln!(
        "[synth] wrote {} → {} samples",
        OUTPUT_PACK_PATH, pack.sample_count
    );

    Ok(())
}

fn read_jsonl<T: serde::de::DeserializeOwned>(path: impl AsRef<Path>) -> std::io::Result<Vec<T>> {
    let text = std::fs::read_to_string(path)?;
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<T>(line) {
            Ok(v) => out.push(v),
            Err(e) => eprintln!("[synth] skip malformed line: {e}"),
        }
    }
    Ok(out)
}

// ---------- augmentation ----------------------------------------------

/// Top-level augmentation entry.  Applies every strategy to the input,
/// then truncates to `cap` variants.  Always includes the original.
fn augment(input: &str, cap: usize) -> Vec<String> {
    let mut all: BTreeSet<String> = BTreeSet::new();
    all.insert(input.to_string());
    all.insert(input.trim_end_matches(['.', '?', '!']).to_string());
    for v in vowel_jitter(input) {
        all.insert(v);
    }
    for v in case_suffix_swap(input) {
        all.insert(v);
    }
    for v in synonym_swap(input) {
        all.insert(v);
    }
    for v in stt_drift_kazakh(input) {
        all.insert(v);
    }
    for v in punctuation_jitter(input) {
        all.insert(v);
    }
    let mut out: Vec<String> = all.into_iter().collect();
    out.truncate(cap);
    out
}

/// Vowel substitutions Whisper-multilingual produces on Kazakh input:
/// ы ↔ і, у ↔ ұ, ө ↔ о, ә ↔ а, ң ↔ н.  Each pair generates one variant
/// per direction, applied to the FIRST occurrence to keep the variant
/// list small but realistic.
fn vowel_jitter(s: &str) -> Vec<String> {
    let pairs = [
        ('ы', 'і'),
        ('і', 'ы'),
        ('у', 'ұ'),
        ('ұ', 'у'),
        ('ө', 'о'),
        ('о', 'ө'),
        ('ә', 'а'),
        ('а', 'ә'),
        ('ң', 'н'),
    ];
    let mut out = Vec::new();
    for (a, b) in pairs {
        if let Some(idx) = s.chars().position(|c| c == a) {
            let mut chars: Vec<char> = s.chars().collect();
            chars[idx] = b;
            out.push(chars.into_iter().collect());
        }
    }
    out
}

/// Kazakh case-suffix harmony swaps that change surface form but not
/// meaning under common queries: -ның ↔ -нің, -да ↔ -де, -ды ↔ -ді,
/// -қа ↔ -ке.  Only swaps the LAST occurrence to preserve syntax.
fn case_suffix_swap(s: &str) -> Vec<String> {
    let pairs = [
        ("ның", "нің"),
        ("нің", "ның"),
        ("да", "де"),
        ("де", "да"),
        ("ды", "ді"),
        ("ді", "ды"),
        ("қа", "ке"),
        ("ке", "қа"),
    ];
    let mut out = Vec::new();
    for (a, b) in pairs {
        if let Some(idx) = s.rfind(a) {
            let mut v = s.to_string();
            v.replace_range(idx..idx + a.len(), b);
            out.push(v);
        }
    }
    out
}

/// Curated lexical swaps the rc7 audits surfaced as common rephrase
/// patterns.  Both directions for each pair; semantic-preserving in
/// the dialog domain.
const SYNONYMS: &[(&str, &str)] = &[
    ("танымал", "атақты"),
    ("атақты", "танымал"),
    ("білесің", "білесіз"),
    ("білесіз", "білесің"),
    ("білесің бе", "білесің"),
    ("білесің", "айтасың"),
    ("формуласы", "формулысы"),
    ("формулысы", "формуласы"),
    ("жазушы", "ақын"),
    ("ақын", "жазушы"),
    ("есім", "ат"),
    ("ат", "есім"),
];

fn synonym_swap(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (a, b) in SYNONYMS {
        if s.contains(a) {
            out.push(s.replace(a, b));
        }
    }
    out
}

/// STT drift patterns observed in actual rc6/rc7 audits.  These are
/// the exact mis-transcriptions Whisper-multilingual produces on
/// Kazakh audio at our sample rate; folding them in makes the
/// classifier robust to the noise the live REPL actually receives.
const STT_DRIFT: &[(&str, &str)] = &[
    ("тоқсан", "доқсан"),
    ("тоқсан", "топсан"),
    ("жазушы", "жазуысы"),
    ("жазушы", "жазуы"),
    ("жазушы", "жасушы"),
    ("Қостанай", "қосанай"),
    ("қостанай", "қосанай"),
    ("формуласын", "формулосын"),
    ("формуласы", "формулосы"),
    ("шүкір", "шұқыр"),
    ("әлей", "аууу"),
    ("ассалаумағалейкум", "ассаляма аууу"),
];

fn stt_drift_kazakh(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (clean, noisy) in STT_DRIFT {
        if s.contains(clean) {
            out.push(s.replace(clean, noisy));
        }
        // Reverse: training on the NOISY side too — if the audit
        // captured a drift, augmentation should not lock that
        // drift in.  Swap noisy → clean to teach both endpoints.
        if s.contains(noisy) {
            out.push(s.replace(noisy, clean));
        }
    }
    out
}

/// Cheap punctuation variants — terminal `.`, `?`, no punct.
fn punctuation_jitter(s: &str) -> Vec<String> {
    let stripped = s.trim_end_matches(['.', '?', '!']).to_string();
    vec![
        stripped.clone(),
        format!("{stripped}."),
        format!("{stripped}?"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vowel_jitter_produces_known_pairs() {
        let v = vowel_jitter("қыз");
        assert!(v.iter().any(|s| s.contains('і')), "ы → і missing in {v:?}");
    }

    #[test]
    fn case_suffix_swap_handles_kazakhness() {
        let v = case_suffix_swap("Қазақстанның");
        assert!(v.iter().any(|s| s == "Қазақстаннің"), "ның → нің missing");
    }

    #[test]
    fn synonym_swap_covers_writers_pair() {
        let v = synonym_swap("атақты жазушы");
        assert!(v.iter().any(|s| s.contains("танымал")));
        assert!(v.iter().any(|s| s.contains("ақын")));
    }

    #[test]
    fn stt_drift_recovers_whisper_noise() {
        // Real drift seen in rc6/rc7 audit logs.
        let v = stt_drift_kazakh("тоқсан алты");
        assert!(v.iter().any(|s| s == "доқсан алты"));
        assert!(v.iter().any(|s| s == "топсан алты"));
    }

    #[test]
    fn augment_caps_output() {
        let v = augment("Қазақстанның жазушылары", 10);
        assert!(v.len() <= 10);
        assert!(v.iter().any(|s| s.contains("Қазақстан")));
    }
}
