// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! # E2 — feature extractor (Rung A)
//!
//! Per-token hashed-feature extractor for the linear slot
//! classifier. Mirrors the trainer's feature space byte-for-byte
//! so the loaded model produces identical scores at inference
//! time. See `tools/intent_dataset/src/slot_train.rs` for the
//! authoritative reference.

/// FNV-1a 32-bit hash. Deterministic, dependency-free.
fn fnv1a_32(s: &str) -> u32 {
    const OFFSET: u32 = 0x811c_9dc5;
    const PRIME: u32 = 16_777_619;
    let mut h = OFFSET;
    for b in s.as_bytes() {
        h ^= u32::from(*b);
        h = h.wrapping_mul(PRIME);
    }
    h
}

/// Extract per-token features for sentence `tokens` at position
/// `i`. Returns sparse `(bucket_index < bucket_count, value)`
/// pairs with `sqrt(count)` squashing. Mirrors the training-
/// time feature set exactly.
pub fn extract(tokens: &[String], i: usize, bucket_count: usize) -> Vec<(u32, f32)> {
    use std::collections::HashMap;
    let mut counts: HashMap<u32, f32> = HashMap::new();
    let mut fire = |key: String| {
        let b = fnv1a_32(&key) % bucket_count as u32;
        *counts.entry(b).or_insert(0.0) += 1.0;
    };
    let tok = &tokens[i];
    let lower = tok.to_lowercase();
    fire(format!("tok:{lower}"));
    let chars: Vec<char> = lower.chars().collect();
    if chars.len() >= 2 {
        fire(format!("pre2:{}", chars.iter().take(2).collect::<String>()));
        fire(format!(
            "suf2:{}",
            chars
                .iter()
                .rev()
                .take(2)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<String>()
        ));
    }
    if chars.len() >= 3 {
        fire(format!("pre3:{}", chars.iter().take(3).collect::<String>()));
        fire(format!(
            "suf3:{}",
            chars
                .iter()
                .rev()
                .take(3)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<String>()
        ));
    }
    if chars.len() >= 4 {
        fire(format!("pre4:{}", chars.iter().take(4).collect::<String>()));
        fire(format!(
            "suf4:{}",
            chars
                .iter()
                .rev()
                .take(4)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<String>()
        ));
    }
    let mut bound: Vec<char> = Vec::with_capacity(chars.len() + 2);
    bound.push('^');
    bound.extend(chars.iter());
    bound.push('$');
    for w in bound.windows(3) {
        fire(format!("3g:{}", w.iter().collect::<String>()));
    }
    if i > 0 {
        fire(format!("prev:{}", tokens[i - 1].to_lowercase()));
    } else {
        fire("prev:<BOS>".to_string());
    }
    if i + 1 < tokens.len() {
        fire(format!("next:{}", tokens[i + 1].to_lowercase()));
    } else {
        fire("next:<EOS>".to_string());
    }
    if tok.chars().next().is_some_and(|c| c.is_uppercase()) {
        fire("is:capitalised".to_string());
    }
    if tok.chars().all(|c| c.is_ascii_digit()) {
        fire("is:all-digit".to_string());
    }
    if tok.chars().any(|c| c.is_ascii_digit()) {
        fire("has:digit".to_string());
    }
    if i == 0 {
        fire("pos:first".to_string());
    }
    if i + 1 == tokens.len() {
        fire("pos:last".to_string());
    }
    let mut out: Vec<(u32, f32)> = counts.into_iter().map(|(k, v)| (k, v.sqrt())).collect();
    out.sort_by_key(|&(k, _)| k);
    out
}
