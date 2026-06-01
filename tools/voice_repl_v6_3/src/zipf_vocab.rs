// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! **Phase 15g.B (2026-06-01)** — Zipf-ranked hot vocabulary for
//! the voice REPL's fuzzy rescorer.
//!
//! ## Why
//!
//! Pre-15g.B, `INTENT_VOCAB` was a hand-curated 151-entry list that
//! grew patch-by-patch from each live REPL session. Whenever fuzzy
//! pulled an STT mishear to the wrong canonical form (e.g.
//! «даулет» → «сәулет», «тауылар» → «тауарлар»), the fix was to
//! add the right canonical to the list. That strategy never
//! converges on an agglutinative language with 243 000 distinct
//! surface forms in the corpus.
//!
//! Linguistic prior (Zipf):
//!   * top 100 words cover ~50 % of everyday speech;
//!   * top 1 000 cover ~80 %;
//!   * top 3 000 cover ~90–95 %.
//!
//! Our measured numbers on the v6.3 corpus
//! (`cc100_kk` + Wikipedia + Abai, 3.48 M tokens):
//!   * top 1 000 cover **43.74 %**
//!     (lower than the 50 % textbook figure because Kazakh's
//!     agglutinative morphology distributes the same root across
//!     many surface forms — but it is the correct floor for an
//!     STT post-processor working on raw surface tokens).
//!
//! ## What this module does
//!
//! 1. Loads `data/voice_repl/zipf_hot_1000.json` (written by
//!    `tools/build_zipf_hot_vocab/extract.py`).
//! 2. Merges it with a small explicit OVERRIDE list — words we
//!    must keep in the hot path even if they're rank > 1 000
//!    (greetings, honorifics, math operators).
//! 3. Returns a `ZipfVocab` whose `best_match` weighs phonetic
//!    similarity AND Zipf rank:
//!       final_score = similarity * (1.0 + zipf_bonus)
//!    so that ties between «даулет / сәулет» break toward the
//!    more frequent canonical instead of toward whichever happens
//!    to be in the curated lexicon.

use adam_dialog::kazakh_fuzzy::kazakh_similarity;
use serde::Deserialize;
use std::path::Path;

/// Default committed path to the JSON the Python builder writes.
pub const ZIPF_HOT_JSON: &str = "data/voice_repl/zipf_hot_1000.json";

/// **Overrides** — words that must be in the hot path regardless
/// of their corpus rank. Greetings, honorifics, math operators,
/// gender words, and explicit identity-markers. Each gets a synth
/// count of `OVERRIDE_COUNT` so they tie against rank-1000 entries
/// at most, never beating the genuine top-N.
const OVERRIDE_COUNT: u32 = 500;
const OVERRIDES: &[&str] = &[
    // Honorifics + greetings that may be rare in written corpora
    // but are critical on a voice path.
    "ассаламу",
    "алейкум",
    "уағалайкум",
    "ас-салам",
    "ағай",
    "апай",
    "балам",
    "сәлеметсіз",
    "қалыңыз",
    "жайыңыз",
    "жағдайыңыз",
    "танысайық",
    "танысалық",
    "алдымен",
    // Identity-question triggers (the dialog engine's intent
    // classifier matches these explicitly).
    "кімсің",
    "кімсіз",
    "кімсін",
    "боласың",
    "боласыз",
    "боласын",
    "екенсің",
    "екенсіз",
    "өзің",
    "өзіңіз",
    "есімім",
    "есімің",
    "атым",
    "атың",
    "менің",
    // Math operators / numerals — these *are* in the top-1000
    // for some but listing them anchors the math path.
    "қосу",
    "қос",
    "көбейту",
    "көбейт",
    "азайту",
    "азайт",
    "бөлу",
    "бөл",
    "тең",
    "нәтиже",
    "есепте",
    "қанша",
    "плюс",
    "минус",
    "бір",
    "екі",
    "үш",
    "төрт",
    "бес",
    "алты",
    "жеті",
    "сегіз",
    "тоғыз",
    "он",
    "жиырма",
    "отыз",
    "қырық",
    "елу",
    "алпыс",
    "жетпіс",
    "сексен",
    "тоқсан",
    "жүз",
    "мың",
    // Gender / identity correction
    "еркек",
    "әйел",
    "емес",
    "емеспін",
    "емессіз",
    "емессің",
    "жоқпын",
    // Geographic plurals — recurring as a topic of factual
    // queries, and the curated lexicon already shipped the
    // wrong canonicals («тауарлар» / товары) by accident.
    "тау",
    "таулар",
    "өзен",
    "өзендер",
    "көл",
    "көлдер",
    "теңіз",
    "теңіздер",
    "қала",
    "қалалар",
    "облыс",
    "облыстар",
];

/// **Named-entity triggers** — when one of these appears in the
/// previous 1-2 tokens, the next token is treated as a proper
/// name and **NOT** rewritten by fuzzy. Prevents «менің атым
/// Даулет» → «менің атым сәулет» (an architecture term in the
/// curated lexicon).
pub const NAMED_ENTITY_TRIGGERS: &[&str] = &[
    "атым",
    "атың",
    "атыңыз",
    "есімім",
    "есімің",
    "есіміңіз",
    "аты",
    "есімі",
];

#[derive(Debug, Deserialize)]
struct ZipfEntryJson {
    word: String,
    count: u32,
}

#[derive(Debug, Deserialize)]
struct ZipfFileJson {
    vocab: Vec<ZipfEntryJson>,
    #[serde(default)]
    total_tokens: u64,
    #[serde(default)]
    top_n: u32,
}

/// In-memory hot vocabulary.
///
/// `entries` is sorted **descending by count** so iteration in
/// `best_match` visits the most frequent canonical first; on equal
/// `final_score`, the ordering breaks toward the higher rank.
pub struct ZipfVocab {
    pub entries: Vec<(String, u32)>,
    pub max_count: u32,
    pub total_tokens: u64,
}

impl ZipfVocab {
    /// Load the committed JSON and merge with `OVERRIDES`.
    /// Missing file → an `OVERRIDES`-only vocabulary; the REPL
    /// still runs but with degraded fuzzy quality.
    pub fn load_or_overrides_only<P: AsRef<Path>>(path: P) -> Self {
        let json = std::fs::read_to_string(path.as_ref()).ok();
        let parsed: Option<ZipfFileJson> = json.and_then(|s| serde_json::from_str(&s).ok());

        let (corpus_entries, total_tokens, top_n) = match parsed {
            Some(p) => (p.vocab, p.total_tokens, p.top_n),
            None => (Vec::new(), 0, 0),
        };

        let mut entries: Vec<(String, u32)> =
            Vec::with_capacity(corpus_entries.len() + OVERRIDES.len());
        let mut seen = std::collections::HashSet::<String>::new();
        for e in corpus_entries.into_iter() {
            if seen.insert(e.word.clone()) {
                entries.push((e.word, e.count));
            }
        }
        for w in OVERRIDES {
            if seen.insert((*w).to_string()) {
                entries.push((w.to_string(), OVERRIDE_COUNT));
            }
        }

        // Sort high-to-low so iteration order during `best_match`
        // naturally prefers more frequent canonicals on tie-break.
        entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let max_count = entries.iter().map(|(_, c)| *c).max().unwrap_or(1);

        println!(
            "[voice-repl] zipf vocab: {} entries (corpus top-{} + {} overrides), max_count={}",
            entries.len(),
            top_n,
            OVERRIDES.len(),
            max_count,
        );

        Self {
            entries,
            max_count,
            total_tokens,
        }
    }

    /// Iterator over canonical surface forms — used elsewhere when
    /// only the wordlist (not the counts) is needed.
    pub fn words(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|(w, _)| w.as_str())
    }

    /// Exact-match check (case-sensitive on lowercase tokens).
    pub fn contains(&self, lower: &str) -> bool {
        self.entries.iter().any(|(w, _)| w == lower)
    }

    /// Zipf-weighted best-match.
    ///
    /// `final = similarity * (1 + zipf_bonus)` where
    /// `zipf_bonus = (log(count + 1) / log(max_count + 1)) * BONUS_WEIGHT`.
    /// `BONUS_WEIGHT = 0.10` — at most a 10 % lift, so the
    /// phonetic distance still dominates; frequency only breaks
    /// ties between equally-plausible canonicals.
    ///
    /// Threshold gate applies to `final_score`, not raw similarity
    /// — a marginal phonetic match (sim = 0.65) on a very common
    /// word can still cross 0.70 via the Zipf bonus, which is the
    /// whole point: if Whisper drops two letters of «қазақстан»,
    /// we'd rather rescore against the rank-3 canonical than
    /// pass through a fragment.
    pub fn best_match(&self, token: &str, threshold: f32) -> Option<(&str, f32)> {
        let max_log = ((self.max_count + 1) as f32).ln().max(1e-6);
        let bonus_weight = 0.10_f32;
        let mut best: Option<(&str, f32)> = None;
        for (cand, count) in &self.entries {
            let sim = kazakh_similarity(token, cand);
            if sim < 0.30 {
                // Cheap prune: a phonetic distance of <0.30 cannot
                // be lifted past 0.70 by a 10 % Zipf bonus.
                continue;
            }
            let log_c = ((*count + 1) as f32).ln();
            let zipf_bonus = (log_c / max_log) * bonus_weight;
            let final_score = sim * (1.0 + zipf_bonus);
            match best {
                None => best = Some((cand.as_str(), final_score)),
                Some((_, prev)) if final_score > prev => {
                    best = Some((cand.as_str(), final_score));
                }
                _ => {}
            }
        }
        best.filter(|(_, s)| *s >= threshold)
    }
}

/// Returns `true` if the previous 1-2 tokens (when stripped of
/// punctuation and lowercased) include a named-entity trigger like
/// «атым», «есімім», «менің атым». The caller uses this to skip
/// fuzzy rewriting of proper names that the canonical lexicon would
/// otherwise mangle («Даулет» → «сәулет»).
pub fn is_after_name_trigger(previous_tokens: &[String]) -> bool {
    for prev in previous_tokens.iter().rev().take(2) {
        let core: String = prev
            .chars()
            .filter(|c| c.is_alphabetic())
            .collect::<String>()
            .to_lowercase();
        if NAMED_ENTITY_TRIGGERS.iter().any(|t| *t == core) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overrides_load_without_json() {
        let v = ZipfVocab::load_or_overrides_only("nonexistent.json");
        assert!(
            !v.entries.is_empty(),
            "overrides alone should populate vocab"
        );
        assert!(v.contains("ағай"));
        assert!(v.contains("апай"));
    }

    #[test]
    fn name_trigger_one_token_back() {
        let prev = vec!["атым".to_string()];
        assert!(is_after_name_trigger(&prev));
    }

    #[test]
    fn name_trigger_two_tokens_back() {
        // «менің атым Даулет» — when scoring «Даулет» the previous
        // tokens are ["менің", "атым"]. trigger is at offset -1.
        let prev = vec!["менің".to_string(), "атым".to_string()];
        assert!(is_after_name_trigger(&prev));
    }

    #[test]
    fn name_trigger_strips_punctuation() {
        let prev = vec!["атым,".to_string()];
        assert!(is_after_name_trigger(&prev));
    }

    #[test]
    fn name_trigger_negative_far_away() {
        // 3 tokens back doesn't trigger — only 1 or 2.
        let prev = vec!["атым".to_string(), "X".to_string(), "Y".to_string()];
        assert!(!is_after_name_trigger(&prev));
    }
}
