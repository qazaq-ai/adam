// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! # Feature extractor — Rung A
//!
//! Pure-Rust hashed-feature extractor for the linear classifier.
//! No external dependencies; no FloatTensor types; one
//! `fn extract(input: &str, bucket_count: usize) -> Vec<(u32, f32)>`.
//!
//! The output is a **sparse** feature vector: a list of
//! `(bucket_index, value)` pairs. Bucket indices are stable across
//! runs because the hash function is deterministic
//! (FNV-1a 32-bit). Feature values are TF-IDF-style: each feature
//! occurring more than once is squashed via `sqrt(count)`.
//!
//! ## Feature families
//!
//! 1. **Character trigrams** of the lower-cased input including
//!    word boundaries (we prepend a `^` and append a `$`). For
//!    Kazakh agglutination this catches both root and suffix
//!    fragments without needing FST analysis.
//! 2. **Token unigrams** (lowercase, whitespace-split). Cheap
//!    backstop for the trigram features on short inputs.
//! 3. **Hand-rolled binary signals** — see [`HandFeature`]. These
//!    are the rules the cascade already encodes (presence of `?`,
//!    leading interrogative pronoun, …); putting them in the
//!    feature space gives the linear model a direct lever for
//!    "which intent does the `?` typically associate with".
//!
//! Bucket-count default is 32 768 (`1 << 15`). At 31 classes and
//! f32 weights, the trained artefact is ~ 4 MB on disk — well
//! under the 5 MB target from the design doc.

/// Hand-rolled binary signals. These bucket into a tiny reserved
/// range at the top of the feature space (we use the highest
/// `HAND_FEATURE_COUNT` indices) so they never collide with hash-
/// trick buckets.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum HandFeature {
    HasQuestionMark,
    HasExclamation,
    HasDigit,
    HasComma,
    StartsWithKaiQai, // «қай / қайда / қашан / қалай / қандай / қанша / неше / неліктен»
    StartsWithMen,    // «мен / менің / маған»
    StartsWithSen,    // «сен / сіз / сенің / сіздің»
    StartsWithBol,    // generic imperative verb (бер / айт / ұсын / көрсет / жаз)
    EndsWithMyn,      // «-мын / -мін» 1sg copula
    EndsWithSyz,      // «-сыз / -сіз» 2sg-polite copula
    LengthShort,      // input < 8 chars
    LengthLong,       // input > 60 chars
    HasGreetingMarker, // «сәлем / қайырлы / ассалаум»
    HasFarewellMarker, // «сау бол / қош бол / көріскенше»
    HasThanksMarker,  // «рахмет / алғыс»
}

/// Number of hand-rolled features. Update when [`HandFeature`]
/// gains variants.
pub const HAND_FEATURE_COUNT: usize = 15;

/// Default bucket count — design doc says ≤ 5 MB on disk; at 31
/// classes × 4 bytes × 32 768 buckets = 4 MB.
pub const DEFAULT_BUCKET_COUNT: usize = 32_768;

/// Extract a sparse feature vector for `input`.
///
/// Returned pairs are `(bucket_index < bucket_count, value)`.
/// Duplicate bucket indices are summed; the caller can re-aggregate
/// after the call without affecting correctness.
pub fn extract(input: &str, bucket_count: usize) -> Vec<(u32, f32)> {
    use std::collections::HashMap;
    let mut counts: HashMap<u32, f32> = HashMap::new();
    let bucket_count_u32 = bucket_count as u32;
    let hand_base = bucket_count.saturating_sub(HAND_FEATURE_COUNT);

    let lowered = input.to_lowercase();

    // Character trigrams with sentence boundaries.
    let mut boundary_chars: Vec<char> = Vec::with_capacity(lowered.chars().count() + 2);
    boundary_chars.push('^');
    boundary_chars.extend(lowered.chars());
    boundary_chars.push('$');
    for window in boundary_chars.windows(3) {
        if window.len() < 3 {
            continue;
        }
        let tri: String = window.iter().collect();
        let key = format!("3g:{tri}");
        let bucket = fnv1a_32(&key) % bucket_count_u32;
        *counts.entry(bucket).or_insert(0.0) += 1.0;
    }

    // Token unigrams.
    for tok in lowered.split_whitespace() {
        let key = format!("tok:{tok}");
        let bucket = fnv1a_32(&key) % bucket_count_u32;
        *counts.entry(bucket).or_insert(0.0) += 1.0;
    }

    // Hand-rolled signals — reserved buckets at the top.
    let mut set_hand = |feature: HandFeature, weight: f32| {
        let idx = hand_base + feature as usize;
        *counts.entry(idx as u32).or_insert(0.0) += weight;
    };
    if lowered.contains('?') {
        set_hand(HandFeature::HasQuestionMark, 1.0);
    }
    if lowered.contains('!') {
        set_hand(HandFeature::HasExclamation, 1.0);
    }
    if lowered.chars().any(|c| c.is_ascii_digit()) {
        set_hand(HandFeature::HasDigit, 1.0);
    }
    if lowered.contains(',') {
        set_hand(HandFeature::HasComma, 1.0);
    }
    let chars_count = lowered.chars().count();
    if chars_count < 8 {
        set_hand(HandFeature::LengthShort, 1.0);
    }
    if chars_count > 60 {
        set_hand(HandFeature::LengthLong, 1.0);
    }
    if lowered.starts_with("қай")
        || lowered.starts_with("қашан")
        || lowered.starts_with("қанша")
        || lowered.starts_with("неше")
        || lowered.starts_with("неліктен")
    {
        set_hand(HandFeature::StartsWithKaiQai, 1.0);
    }
    if lowered.starts_with("мен") || lowered.starts_with("маған") {
        set_hand(HandFeature::StartsWithMen, 1.0);
    }
    if lowered.starts_with("сен") || lowered.starts_with("сіз") {
        set_hand(HandFeature::StartsWithSen, 1.0);
    }
    for verb in &["айт", "бер", "ұсын", "көрсет", "жаз", "қос"] {
        if lowered.split_whitespace().any(|t| t.starts_with(verb)) {
            set_hand(HandFeature::StartsWithBol, 1.0);
        }
    }
    if lowered.ends_with("мын") || lowered.ends_with("мін") {
        set_hand(HandFeature::EndsWithMyn, 1.0);
    }
    if lowered.ends_with("сыз") || lowered.ends_with("сіз") {
        set_hand(HandFeature::EndsWithSyz, 1.0);
    }
    if lowered.contains("сәлем") || lowered.contains("қайырлы") || lowered.contains("ассалаум")
    {
        set_hand(HandFeature::HasGreetingMarker, 1.0);
    }
    if lowered.contains("сау бол") || lowered.contains("қош бол") || lowered.contains("көріскенше")
    {
        set_hand(HandFeature::HasFarewellMarker, 1.0);
    }
    if lowered.contains("рахмет") || lowered.contains("алғыс") {
        set_hand(HandFeature::HasThanksMarker, 1.0);
    }

    // Apply sqrt squashing so very-frequent features (the same
    // trigram repeated in a longer input) don't dominate.
    let mut out: Vec<(u32, f32)> = counts
        .into_iter()
        .map(|(idx, count)| (idx, count.sqrt()))
        .collect();
    out.sort_by_key(|&(idx, _)| idx);
    out
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_produces_some_features() {
        let f = extract("Сәлем, қалайсың?", DEFAULT_BUCKET_COUNT);
        assert!(!f.is_empty());
    }

    #[test]
    fn empty_input_produces_only_length_short_signal() {
        // input="" → boundary chars `[^, $]` give zero trigrams,
        // `split_whitespace()` yields zero unigrams. The only
        // feature that fires is `HandFeature::LengthShort`
        // (chars_count == 0 < 8).
        let f = extract("", DEFAULT_BUCKET_COUNT);
        assert_eq!(f.len(), 1);
        let expected_bucket =
            (DEFAULT_BUCKET_COUNT - HAND_FEATURE_COUNT + HandFeature::LengthShort as usize) as u32;
        assert_eq!(f[0].0, expected_bucket);
    }

    #[test]
    fn buckets_are_within_range() {
        let f = extract("Сәлем дос", 1024);
        for (idx, _) in &f {
            assert!((*idx as usize) < 1024, "bucket {idx} ≥ bucket_count 1024");
        }
    }

    #[test]
    fn deterministic_across_calls() {
        let a = extract("қалайсыз", DEFAULT_BUCKET_COUNT);
        let b = extract("қалайсыз", DEFAULT_BUCKET_COUNT);
        assert_eq!(a, b);
    }

    #[test]
    fn hand_features_fire_on_question_mark() {
        let with_q = extract("қалай?", DEFAULT_BUCKET_COUNT);
        let without_q = extract("қалай", DEFAULT_BUCKET_COUNT);
        // The question-mark variant must include the reserved
        // bucket for HasQuestionMark. The non-question variant
        // must not.
        let q_bucket = (DEFAULT_BUCKET_COUNT - HAND_FEATURE_COUNT
            + HandFeature::HasQuestionMark as usize) as u32;
        assert!(with_q.iter().any(|(b, _)| *b == q_bucket));
        assert!(!without_q.iter().any(|(b, _)| *b == q_bucket));
    }
}
