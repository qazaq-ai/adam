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

// **Phase 15g.B.2 (2026-06-01)** — Zipf vocabulary is removed.
//
// Two live tests showed the corpus-derived 1 000-word vocabulary
// caused fuzzy to substitute semantically-different short words
// («құн» → «қан», «бұғын» → «бұрын», «әтім» → «әкімі»). The patch-
// loop the user flagged was rooted in trying to derive corrections
// from a static list of canonicals with no contextual signal.
//
// This module now keeps **only** the explicit `OVERRIDES` — the
// short list of greetings / honorifics / math operators / gender
// words / geographic plurals that fuzzy can safely commit on
// without contextual signal. The rest of the recovery happens
// elsewhere: via the Kazakh name DB (this module), and — when
// implemented — via a neural contextual rescorer (Phase 15g.C).

use adam_dialog::kazakh_fuzzy::{KazakhNameIndex, kazakh_similarity};
use std::path::Path;

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

/// **Hot vocabulary** for STT post-processing.
///
/// Phase 15g.B.2: holds only the explicit `OVERRIDES` list (no
/// corpus-derived Zipf top-N anymore — that caused short-word
/// regression). The Kazakh name DB is held separately so the
/// caller can do a context-gated name pass.
pub struct ZipfVocab {
    pub entries: Vec<String>,
    pub names: KazakhNameIndex,
    /// Combined male+female name list, lowercased, for fast
    /// case-insensitive comparison; canonical-cased originals live
    /// in `names.{male,female}`.
    pub names_lower: Vec<(String, String)>,
}

impl ZipfVocab {
    /// Build a minimal vocab from the OVERRIDES list, plus load
    /// the Kazakh name DB (`data/lexicon/kazakh_names_*.json`,
    /// ~352 entries combined). Missing name files → empty index;
    /// the REPL still runs but loses the proper-name pass.
    ///
    /// The `_path` argument is kept on the signature for one
    /// release of source-compatibility with Phase 15g.B call
    /// sites; it is no longer read.
    pub fn load_or_overrides_only<P: AsRef<Path>>(_path: P) -> Self {
        let mut seen = std::collections::HashSet::<String>::new();
        let mut entries: Vec<String> = Vec::with_capacity(OVERRIDES.len());
        for w in OVERRIDES {
            if seen.insert((*w).to_string()) {
                entries.push((*w).to_string());
            }
        }

        let names = KazakhNameIndex::load_default();
        let mut names_lower: Vec<(String, String)> =
            Vec::with_capacity(names.male.len() + names.female.len());
        for n in names.male.iter().chain(names.female.iter()) {
            names_lower.push((n.to_lowercase(), n.clone()));
        }

        println!(
            "[voice-repl] hot vocab: {} overrides; name DB: {} male + {} female",
            entries.len(),
            names.male.len(),
            names.female.len(),
        );

        Self {
            entries,
            names,
            names_lower,
        }
    }

    /// Exact-match check (case-sensitive on lowercase tokens).
    pub fn contains(&self, lower: &str) -> bool {
        self.entries.iter().any(|w| w == lower)
    }

    /// Look up the best matching Kazakh proper name for `lower`
    /// (typically a Whisper-transcribed name in lowercase). Returns
    /// the **canonical-cased** form when similarity ≥ threshold.
    /// Used after a name trigger («атым», «есімім») to recover
    /// proper-name capitalisation that Whisper drops.
    pub fn best_name_match(&self, lower: &str, threshold: f32) -> Option<String> {
        let mut best: Option<(&str, f32)> = None;
        for (lc, canonical) in &self.names_lower {
            let sim = kazakh_similarity(lower, lc);
            match best {
                None => best = Some((canonical.as_str(), sim)),
                Some((_, prev)) if sim > prev => {
                    best = Some((canonical.as_str(), sim));
                }
                _ => {}
            }
        }
        best.filter(|(_, s)| *s >= threshold)
            .map(|(name, _)| name.to_string())
    }

    /// Phonetic best-match against the small OVERRIDES list ONLY.
    ///
    /// **Phase 15g.B.2 (2026-06-01)** — vocabulary now contains
    /// ~82 explicit overrides, not 1 061 corpus-derived canonicals.
    /// This shrinks the false-positive surface to almost zero —
    /// the OVERRIDES list is curated by hand to NOT contain
    /// confusable short-word neighbours («қан» / «құн» / «күн»
    /// are deliberately absent). Long-tail recovery moves to the
    /// neural rescorer planned for Phase 15g.C.
    ///
    /// Defences retained from 15g.B.1:
    ///   * Length floor 5 — short OOVs pass through untouched.
    ///   * Absolute edit ≥ 1.5 — single-char drifts don't flip
    ///     (covers the K→Қ / Г→Ғ phonetic-pair zone where we
    ///     intentionally trust Whisper).
    ///   * Final similarity ≥ caller-supplied threshold (0.80).
    pub fn best_match(&self, token: &str, threshold: f32) -> Option<(&str, f32)> {
        let token_len_chars = token.chars().count();
        if token_len_chars < 5 {
            return None;
        }
        let mut best: Option<(&str, f32)> = None;
        for cand in &self.entries {
            let sim = kazakh_similarity(token, cand);
            if sim < threshold {
                continue;
            }
            let cand_len_chars = cand.chars().count();
            let denom = token_len_chars.max(cand_len_chars) as f32;
            let edit = (1.0 - sim) * denom;
            if edit < 1.5 {
                continue;
            }
            match best {
                None => best = Some((cand.as_str(), sim)),
                Some((_, prev)) if sim > prev => {
                    best = Some((cand.as_str(), sim));
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
