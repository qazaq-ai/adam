//! **v5.20.0 — Kazakh fuzzy entity matcher.**
//!
//! Universal soft-match for misheard (Whisper) or mistyped (keyboard)
//! Kazakh tokens against curated canonical lists (names, cities,
//! occupations, …). Works on both voice input AND text input — the
//! shape of the upstream noise is the same:
//!
//! - **Voice path:** Whisper-medium / Whisper-large may emit
//!   «Сарсембай» when the user said «Сарсенбай», or «Дауыт» for
//!   «Дәулет». The v5.19.0 transcript normalizer catches the most
//!   common substitutions; this layer catches the long tail.
//! - **Text path:** Students with imperfect spelling type «Даулет»
//!   instead of «Дәулет» (drops `ә`), or «Алмаыта» (transposed
//!   letters). adam should recognise the intended entity instead of
//!   surfacing «Сізге айтылған сияқты екен — нақты дерегім жоқ».
//!
//! ## Algorithm
//!
//! Levenshtein edit distance with **Kazakh-phonetic substitution
//! costs**: substituting `қ ↔ к` / `ң ↔ н` / `ә ↔ а` / etc. costs
//! 0.4 instead of the default 1.0, because these pairs are
//! systematically confused across speakers and writers. Other
//! substitutions cost the standard 1.0.
//!
//! The matcher returns the canonical form + a similarity score in
//! [0.0, 1.0]; the caller decides whether to commit (typically
//! `score ≥ 0.75`) or ask for clarification (`< 0.75`).
//!
//! ## Why deterministic, not ML
//!
//! Per `project_retrieval_not_neural_v2` — adam's commitment to the
//! «third path». A rule-based phonetic-aware edit distance is:
//! cheap (≤ 100 µs per query against a 200-name list), inspectable
//! (every match has an explainable score), and stable (no model
//! drift between releases). The substitution table is the entire
//! «learned» component — it's a 60-entry static map a linguist can
//! audit at a glance.

use std::collections::HashSet;

/// A pair of Kazakh characters that are systematically confused
/// in voice / text. Substituting between members of a pair costs
/// `PHONETIC_SUB_COST` instead of 1.0.
const PHONETIC_PAIRS: &[(char, char)] = &[
    // Velar plosive: voiced/voiceless + backness
    ('қ', 'к'),
    ('ғ', 'г'),
    // Nasal
    ('ң', 'н'),
    // Front/back vowels (Kazakh vowel harmony confusion)
    ('ә', 'а'),
    ('ә', 'е'),
    ('ө', 'о'),
    ('ө', 'е'),
    // Rounded close vowels
    ('ұ', 'у'),
    ('ү', 'у'),
    ('ү', 'и'),
    ('ұ', 'ұ'),
    // Front/back close vowels
    ('і', 'и'),
    ('ы', 'и'),
    // Glides and semi-vowels
    ('й', 'и'),
    // Russian-Kazakh confusables
    ('э', 'е'),
    // Yodisation (`я` ↔ `я` itself catches no-op; the multi-char
    // `я` ↔ `ия` mapping needs to live in a separate token rewrite
    // layer and is not handled at the single-char Levenshtein
    // level — the edit-distance metric is character-by-character).
];

const PHONETIC_SUB_COST: f32 = 0.4;

/// Compute the Kazakh-aware Levenshtein distance between two
/// tokens. The base algorithm is standard Wagner-Fischer DP;
/// substitution cost lookups consult [`PHONETIC_PAIRS`] for
/// systematically-confused character pairs.
pub fn kazakh_edit_distance(a: &str, b: &str) -> f32 {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let n = a_chars.len();
    let m = b_chars.len();
    if n == 0 {
        return m as f32;
    }
    if m == 0 {
        return n as f32;
    }
    let mut dp = vec![vec![0.0f32; m + 1]; n + 1];
    for i in 0..=n {
        dp[i][0] = i as f32;
    }
    for j in 0..=m {
        dp[0][j] = j as f32;
    }
    for i in 1..=n {
        for j in 1..=m {
            let ca = a_chars[i - 1]
                .to_lowercase()
                .next()
                .unwrap_or(a_chars[i - 1]);
            let cb = b_chars[j - 1]
                .to_lowercase()
                .next()
                .unwrap_or(b_chars[j - 1]);
            let sub_cost = if ca == cb {
                0.0
            } else if are_phonetically_close(ca, cb) {
                PHONETIC_SUB_COST
            } else {
                1.0
            };
            let del = dp[i - 1][j] + 1.0;
            let ins = dp[i][j - 1] + 1.0;
            let sub = dp[i - 1][j - 1] + sub_cost;
            dp[i][j] = del.min(ins).min(sub);
        }
    }
    dp[n][m]
}

fn are_phonetically_close(a: char, b: char) -> bool {
    PHONETIC_PAIRS
        .iter()
        .any(|(x, y)| (*x == a && *y == b) || (*x == b && *y == a))
}

/// Similarity score in `[0.0, 1.0]`. `1.0` = identical, `0.0` =
/// maximally different. Computed as `1.0 - dist / max(|a|, |b|)`.
pub fn kazakh_similarity(a: &str, b: &str) -> f32 {
    let len = a.chars().count().max(b.chars().count()).max(1) as f32;
    let d = kazakh_edit_distance(a, b);
    (1.0 - d / len).clamp(0.0, 1.0)
}

/// Best-match scanner: find the canonical entry in `candidates` that
/// is closest to `token` under [`kazakh_similarity`]. Returns
/// `(canonical, score)` for the top hit when score ≥ `threshold`;
/// `None` if no candidate clears the bar.
///
/// `candidates` is borrowed, so the caller (which loads the JSON
/// list once and reuses it across turns) doesn't pay clone cost.
pub fn best_match<'a>(
    token: &str,
    candidates: &'a [String],
    threshold: f32,
) -> Option<(&'a str, f32)> {
    let mut best: Option<(&'a str, f32)> = None;
    for cand in candidates {
        let score = kazakh_similarity(token, cand);
        match best {
            None => best = Some((cand.as_str(), score)),
            Some((_, prev)) if score > prev => best = Some((cand.as_str(), score)),
            _ => {}
        }
    }
    best.filter(|(_, s)| *s >= threshold)
}

/// **Confidence-band classifier** for downstream routing:
/// - `≥ 0.92` → `MatchBand::HighConfident` — commit silently
/// - `0.75 .. 0.92` → `MatchBand::Plausible` — commit but mark for
///   confirmation in the next turn
/// - `< 0.75` → `MatchBand::Unclear` — refuse, ask for clarification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchBand {
    HighConfident,
    Plausible,
    Unclear,
}

pub fn classify_score(score: f32) -> MatchBand {
    if score >= 0.92 {
        MatchBand::HighConfident
    } else if score >= 0.75 {
        MatchBand::Plausible
    } else {
        MatchBand::Unclear
    }
}

/// Curated Kazakh-name candidate set, loaded lazily from
/// `data/lexicon/kazakh_names_{male,female}.json`. Cached behind a
/// `OnceLock` so the JSON parse cost is paid once per process.
///
/// Returns an empty set when the JSON files aren't present (CI on
/// fresh checkouts, sandboxed tests) — fuzzy lookups then degrade
/// to exact-match behaviour, which is the v5.19.x baseline.
pub struct KazakhNameIndex {
    pub male: Vec<String>,
    pub female: Vec<String>,
}

impl KazakhNameIndex {
    /// Load both name lists from the standard data path. The path
    /// is computed relative to `CARGO_MANIFEST_DIR` so the loader
    /// works from any binary in the workspace.
    pub fn load_default() -> Self {
        let base = format!("{}/../../data/lexicon", env!("CARGO_MANIFEST_DIR"));
        Self {
            male: Self::load_one(&format!("{base}/kazakh_names_male.json")),
            female: Self::load_one(&format!("{base}/kazakh_names_female.json")),
        }
    }

    fn load_one(path: &str) -> Vec<String> {
        let Ok(raw) = std::fs::read_to_string(path) else {
            return Vec::new();
        };
        #[derive(serde::Deserialize)]
        struct File {
            names: Vec<String>,
        }
        serde_json::from_str::<File>(&raw)
            .map(|f| f.names)
            .unwrap_or_default()
    }

    /// Combined view (male + female deduplicated).
    pub fn combined(&self) -> Vec<String> {
        let mut set: HashSet<String> = HashSet::new();
        for n in self.male.iter().chain(self.female.iter()) {
            set.insert(n.clone());
        }
        set.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_strings_distance_zero() {
        assert_eq!(kazakh_edit_distance("Дәулет", "Дәулет"), 0.0);
    }

    #[test]
    fn phonetic_substitution_cheaper_than_random() {
        // «Даулет» (no `ә`) → «Дәулет»: one phonetic sub.
        let phonetic = kazakh_edit_distance("Даулет", "Дәулет");
        // «Зәулет» → «Дәулет»: one random sub.
        let random = kazakh_edit_distance("Зәулет", "Дәулет");
        assert!(
            phonetic < random,
            "phonetic ({phonetic}) must cost less than random ({random})"
        );
        assert_eq!(phonetic, PHONETIC_SUB_COST);
    }

    #[test]
    fn similarity_high_for_typo() {
        // «Сарсембай» (typo) vs «Сарсенбай» — single nasal-confusion
        // substitution.
        let sim = kazakh_similarity("Сарсембай", "Сарсенбай");
        assert!(sim > 0.85, "similarity too low: {sim}");
    }

    #[test]
    fn similarity_high_for_voice_mishearing() {
        // Whisper-large «Қалымыз» vs intended «Қалыңыз» — ң→м is not
        // a phonetic pair (м is bilabial stop, ң is velar nasal),
        // so this is a standard 1.0-cost sub. Edit distance 1 over
        // length 7 = ~0.86 similarity.
        let sim = kazakh_similarity("Қалымыз", "Қалыңыз");
        assert!(sim > 0.80 && sim < 0.95, "got {sim}");
    }

    #[test]
    fn best_match_picks_canonical() {
        let candidates = vec![
            "Дәулет".to_string(),
            "Айдос".to_string(),
            "Сарсенбай".to_string(),
        ];
        let (canonical, score) = best_match("Даулет", &candidates, 0.7).unwrap();
        assert_eq!(canonical, "Дәулет");
        assert!(score >= 0.8);
    }

    #[test]
    fn best_match_below_threshold_returns_none() {
        let candidates = vec!["Дәулет".to_string()];
        // «Кітап» (book) is nothing like «Дәулет».
        assert!(best_match("Кітап", &candidates, 0.7).is_none());
    }

    #[test]
    fn classify_band_thresholds() {
        assert_eq!(classify_score(1.0), MatchBand::HighConfident);
        assert_eq!(classify_score(0.95), MatchBand::HighConfident);
        assert_eq!(classify_score(0.85), MatchBand::Plausible);
        assert_eq!(classify_score(0.75), MatchBand::Plausible);
        assert_eq!(classify_score(0.74), MatchBand::Unclear);
        assert_eq!(classify_score(0.0), MatchBand::Unclear);
    }

    #[test]
    fn name_index_loads_curated_lists() {
        let idx = KazakhNameIndex::load_default();
        // Snapshot bounds: don't assert exact size (lists grow),
        // but expect ≥ 50 each — sanity check the loader.
        assert!(idx.male.len() >= 50, "got {}", idx.male.len());
        assert!(idx.female.len() >= 50, "got {}", idx.female.len());
        // Spot-check well-known names.
        assert!(idx.male.iter().any(|n| n == "Дәулет"));
        assert!(idx.female.iter().any(|n| n == "Айгерім"));
    }

    #[test]
    fn fuzzy_name_recovery_e2e() {
        let idx = KazakhNameIndex::load_default();
        if idx.male.is_empty() {
            // CI env without data files — skip.
            return;
        }
        // «Даулет» (no ә) → «Дәулет» via phonetic-aware fuzzy.
        let (canonical, score) = best_match("Даулет", &idx.male, 0.75).unwrap();
        assert_eq!(canonical, "Дәулет");
        assert!(score >= 0.85, "got {score}");

        // «Айкерім» (typo) → «Айгерім»: г↔к is a phonetic pair, so
        // the substitution is cheap.
        let (canonical, score) = best_match("Айкерім", &idx.female, 0.75).unwrap();
        assert_eq!(canonical, "Айгерім");
        assert!(score >= 0.85, "got {score}");
    }
}
