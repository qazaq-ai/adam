// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! # `intent_dataset_stt_augment` — Movement B of v6.5
//!
//! Generates **STT-noise** variants for each labelled intent
//! example.  Different axis from `synth.rs`: that one paraphrases
//! semantically (politeness swap, possessive drop, question
//! particle); this one corrupts surface forms in the SAME way
//! Whisper does on Kazakh production audio.
//!
//! The retrained classifier (Movement B) sees production-shaped
//! input distribution during training, so confidence stays high
//! on noisy STT instead of dropping to 0.3-0.5 (which is what
//! drives the v6.4.x audits' wrong routing).
//!
//! ## Transformations applied
//!
//! Each based on Whisper-noise patterns observed in v6.4.x audits.
//! Each is randomised (the binary takes a `--seed` for
//! reproducibility) and applied with the probabilities below to
//! the cloned source example.  Multiple may stack.
//!
//! 1. **Kazakh-specific character drop** (prob 0.45) — replace one
//!    occurrence of ә→а, қ→к, ғ→г, ң→н, ө→о, ұ→у, ү→у, і→и, һ→х.
//! 2. **Final-consonant drop** (prob 0.15) — drop the last
//!    consonant of a non-monosyllabic token: «көбейт» → «көбей»,
//!    «бөл» stays bare (≤1 syllable).
//! 3. **Soft-sign insertion** (prob 0.10) — append «ь» to a token
//!    ending in a consonant: «бөл» → «бөль».
//! 4. **Word-final vowel duplication** (prob 0.10) — «алты» →
//!    «алтыа» (Whisper occasionally drags the vowel).
//! 5. **Space insertion at word boundary** (prob 0.10) — split a
//!    multi-syllable token: «қостанайда» → «қостанай да».
//! 6. **Punctuation injection** (prob 0.10) — insert «,» between
//!    two adjacent tokens.
//!
//! ## Target distribution
//!
//! We scale augmentation per-intent: rare classes get more copies.
//!
//!   - intents with N < 50 examples: 10× augmentation
//!   - intents with 50 ≤ N < 200: 5× augmentation
//!   - intents with N ≥ 200: 2× augmentation
//!
//! This drags low-data intents from N=4 (AskWeather) up to ~40 and
//! caps the dominant intents around their original mass × 2.
//!
//! ## Output
//!
//! Writes the augmented pack to
//! `data/curated/adam_intent_training_pack_stt_augmented.json`
//! in the same schema as the input.  Original samples are
//! preserved; augmented ones get `source: "stt_augment_v1"`.

use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

const INPUT_PACK: &str = "data/curated/adam_intent_training_pack.json";
const OUTPUT_PACK: &str = "data/curated/adam_intent_training_pack_stt_augmented.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Sample {
    text: String,
    intent: String,
    source: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Pack {
    version: String,
    name: String,
    target_language: String,
    script: String,
    intents: Vec<String>,
    sample_count: usize,
    samples: Vec<Sample>,
}

fn main() {
    let raw = fs::read_to_string(INPUT_PACK).unwrap_or_else(|e| panic!("read {INPUT_PACK}: {e}"));
    let pack: Pack =
        serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse {INPUT_PACK}: {e}"));

    let mut by_intent: HashMap<String, Vec<Sample>> = HashMap::new();
    for s in &pack.samples {
        by_intent
            .entry(s.intent.clone())
            .or_default()
            .push(s.clone());
    }

    let mut rng = StdRng::seed_from_u64(0x_a1da_60a5_50e0u64);
    let mut all: Vec<Sample> = pack.samples.clone();

    for (intent, examples) in &by_intent {
        let n = examples.len();
        // **v6.5.0-rc3 (2026-06-09) — rebalanced augmentation.**
        //
        // rc1/rc2 used 10×/5×/2× factors keyed on class size.  Live
        // audit caught a regression: chemistry queries like «Темірдің
        // формулысы» got mis-labelled as Greeting at 0.96 confidence
        // because rare classes (N=40-50) flooded the training mix
        // with 400+ synthetic copies, smearing decision boundaries
        // across short-token patterns.
        //
        // rc3 caps the rare-class factor at 5× and the mid-class
        // factor at 3×.  The classifier loses some long-tail
        // robustness but gains correctness on the head intents.  The
        // proper fix is more genuine examples; this is the safer
        // synthetic-only knob.
        let factor = if n < 50 {
            5
        } else if n < 200 {
            3
        } else {
            2
        };
        for src in examples {
            for _ in 0..factor {
                let noised = noise_pass(&src.text, &mut rng);
                if noised != src.text && !noised.trim().is_empty() {
                    all.push(Sample {
                        text: noised,
                        intent: intent.clone(),
                        source: "stt_augment_v1".to_string(),
                    });
                }
            }
        }
    }

    let out = Pack {
        version: format!("{}+stt_augment_v1", pack.version),
        name: format!("{}-stt-augmented", pack.name),
        target_language: pack.target_language,
        script: pack.script,
        intents: pack.intents,
        sample_count: all.len(),
        samples: all,
    };

    let serialised = serde_json::to_string_pretty(&out).expect("serialise");
    fs::write(OUTPUT_PACK, serialised).expect("write output");

    eprintln!(
        "[stt_augment] input: {} samples → output: {} samples (×{:.1})",
        pack.sample_count,
        out.sample_count,
        out.sample_count as f64 / pack.sample_count as f64,
    );

    // Per-intent distribution.
    let mut by_out: HashMap<String, usize> = HashMap::new();
    for s in &out.samples {
        *by_out.entry(s.intent.clone()).or_insert(0) += 1;
    }
    let mut rows: Vec<(String, usize, usize)> = by_intent
        .keys()
        .map(|k| (k.clone(), by_intent[k].len(), *by_out.get(k).unwrap_or(&0)))
        .collect();
    rows.sort_by(|a, b| b.2.cmp(&a.2));
    for (intent, before, after) in rows.iter().take(20) {
        eprintln!(
            "  {intent:32} {before:>5} → {after:>5}  (×{:.1})",
            *after as f64 / *before as f64
        );
    }
    if rows.len() > 20 {
        eprintln!("  ... {} intents total", rows.len());
    }
}

/// Apply 1-3 random STT-noise transformations to a clean text.
fn noise_pass(text: &str, rng: &mut StdRng) -> String {
    let mut s = text.to_string();
    let steps = rng.random_range(1..=3);
    for _ in 0..steps {
        let r: f64 = rng.random();
        s = if r < 0.45 {
            kazakh_char_drop(&s, rng)
        } else if r < 0.60 {
            final_consonant_drop(&s, rng)
        } else if r < 0.70 {
            soft_sign_insert(&s, rng)
        } else if r < 0.80 {
            vowel_duplication(&s, rng)
        } else if r < 0.90 {
            split_token(&s, rng)
        } else {
            punctuation_inject(&s, rng)
        };
    }
    s
}

/// Replace one occurrence of a Kazakh-specific character with its
/// Russian-keyboard neighbour.
fn kazakh_char_drop(s: &str, rng: &mut StdRng) -> String {
    const SUBS: &[(char, char)] = &[
        ('ә', 'а'),
        ('қ', 'к'),
        ('ғ', 'г'),
        ('ң', 'н'),
        ('ө', 'о'),
        ('ұ', 'у'),
        ('ү', 'у'),
        ('і', 'и'),
        ('һ', 'х'),
        // Uppercase too.
        ('Ә', 'А'),
        ('Қ', 'К'),
        ('Ғ', 'Г'),
        ('Ң', 'Н'),
        ('Ө', 'О'),
        ('Ұ', 'У'),
        ('Ү', 'У'),
        ('І', 'И'),
        ('Һ', 'Х'),
    ];
    let positions: Vec<(usize, char)> = s
        .char_indices()
        .filter(|(_, c)| SUBS.iter().any(|(from, _)| from == c))
        .collect();
    if positions.is_empty() {
        return s.to_string();
    }
    let pick = positions[rng.random_range(0..positions.len())];
    let target = SUBS.iter().find(|(from, _)| *from == pick.1).unwrap().1;
    let mut out = String::with_capacity(s.len());
    out.push_str(&s[..pick.0]);
    out.push(target);
    out.push_str(&s[pick.0 + pick.1.len_utf8()..]);
    out
}

/// Drop the last consonant of a random multi-syllable token.
fn final_consonant_drop(s: &str, rng: &mut StdRng) -> String {
    let tokens: Vec<&str> = s.split_whitespace().collect();
    if tokens.is_empty() {
        return s.to_string();
    }
    let idx = rng.random_range(0..tokens.len());
    let tok = tokens[idx];
    let chars: Vec<char> = tok.chars().collect();
    if chars.len() < 4 {
        return s.to_string();
    }
    let last = chars[chars.len() - 1];
    if is_vowel(last) {
        return s.to_string();
    }
    let truncated: String = chars[..chars.len() - 1].iter().collect();
    let mut new_tokens = tokens.iter().map(|t| t.to_string()).collect::<Vec<_>>();
    new_tokens[idx] = truncated;
    new_tokens.join(" ")
}

/// Append «ь» to a random token ending in a consonant.
fn soft_sign_insert(s: &str, rng: &mut StdRng) -> String {
    let tokens: Vec<&str> = s.split_whitespace().collect();
    if tokens.is_empty() {
        return s.to_string();
    }
    let idx = rng.random_range(0..tokens.len());
    let tok = tokens[idx];
    let last = tok.chars().last();
    if last.is_none() || is_vowel(last.unwrap()) {
        return s.to_string();
    }
    let mut new_tokens = tokens.iter().map(|t| t.to_string()).collect::<Vec<_>>();
    new_tokens[idx] = format!("{}ь", tok);
    new_tokens.join(" ")
}

/// «алты» → «алтыа» — Whisper drags vowel.
fn vowel_duplication(s: &str, rng: &mut StdRng) -> String {
    let tokens: Vec<&str> = s.split_whitespace().collect();
    if tokens.is_empty() {
        return s.to_string();
    }
    let idx = rng.random_range(0..tokens.len());
    let tok = tokens[idx];
    let last = tok.chars().last();
    if last.is_none() || !is_vowel(last.unwrap()) {
        return s.to_string();
    }
    let mut new_tokens = tokens.iter().map(|t| t.to_string()).collect::<Vec<_>>();
    new_tokens[idx] = format!("{}{}", tok, last.unwrap());
    new_tokens.join(" ")
}

/// «қостанайда» → «қостанай да» — Whisper splits.
fn split_token(s: &str, rng: &mut StdRng) -> String {
    let tokens: Vec<&str> = s.split_whitespace().collect();
    let multi: Vec<usize> = (0..tokens.len())
        .filter(|&i| tokens[i].chars().count() >= 6)
        .collect();
    if multi.is_empty() {
        return s.to_string();
    }
    let idx = multi[rng.random_range(0..multi.len())];
    let tok = tokens[idx];
    let chars: Vec<char> = tok.chars().collect();
    let split_at = chars.len() / 2 + rng.random_range(0..2);
    if split_at == 0 || split_at >= chars.len() {
        return s.to_string();
    }
    let left: String = chars[..split_at].iter().collect();
    let right: String = chars[split_at..].iter().collect();
    let mut new_tokens = tokens.iter().map(|t| t.to_string()).collect::<Vec<_>>();
    new_tokens[idx] = left;
    new_tokens.insert(idx + 1, right);
    new_tokens.join(" ")
}

/// Insert «,» between two adjacent tokens.
fn punctuation_inject(s: &str, rng: &mut StdRng) -> String {
    let tokens: Vec<&str> = s.split_whitespace().collect();
    if tokens.len() < 2 {
        return s.to_string();
    }
    let idx = rng.random_range(0..tokens.len() - 1);
    let mut new_tokens = tokens.iter().map(|t| t.to_string()).collect::<Vec<_>>();
    new_tokens[idx] = format!("{},", tokens[idx]);
    new_tokens.join(" ")
}

fn is_vowel(c: char) -> bool {
    matches!(
        c.to_lowercase().next().unwrap_or(c),
        'а' | 'е' | 'ё' | 'и' | 'о' | 'у' | 'ы' | 'э' | 'ю' | 'я' | 'ә' | 'і' | 'ө' | 'ұ' | 'ү'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kazakh_char_drop_replaces_one_char() {
        let mut rng = StdRng::seed_from_u64(1);
        let out = kazakh_char_drop("қазақ", &mut rng);
        assert_ne!(out, "қазақ");
        // Output should still contain Kazakh letters or their substitutes.
        assert!(out.chars().count() == 5);
    }

    #[test]
    fn final_consonant_drop_shortens_long_token() {
        let mut rng = StdRng::seed_from_u64(2);
        let out = final_consonant_drop("көбейт", &mut rng);
        assert_eq!(out, "көбей");
    }

    #[test]
    fn soft_sign_appended_to_consonant_ending() {
        let mut rng = StdRng::seed_from_u64(3);
        let out = soft_sign_insert("бөл", &mut rng);
        assert_eq!(out, "бөль");
    }

    #[test]
    fn split_token_breaks_long_word() {
        let mut rng = StdRng::seed_from_u64(4);
        let out = split_token("қостанайда", &mut rng);
        assert!(out.contains(' '));
        // Total alpha chars preserved
        let orig_alpha: String = "қостанайда".chars().filter(|c| c.is_alphabetic()).collect();
        let out_alpha: String = out.chars().filter(|c| c.is_alphabetic()).collect();
        assert_eq!(orig_alpha, out_alpha);
    }

    #[test]
    fn noise_pass_changes_input() {
        let mut rng = StdRng::seed_from_u64(5);
        let s = "Менің атым Дәулет";
        let out = noise_pass(s, &mut rng);
        assert_ne!(out, s, "1-3 transformations should produce a change");
    }
}
