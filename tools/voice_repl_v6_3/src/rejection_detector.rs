// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! # Rejection detector — v6.5 self-learning loop, signal layer
//!
//! Reads the [`SessionJournal`](crate::session_journal::SessionJournal)
//! and decides, on each new user turn, whether the user is rejecting
//! the previous reply.  rc5 ships detection + observability only;
//! rc6 wires the detection into a persisted
//! `mistake_corrections.jsonl` so adam can learn from the rejection.
//!
//! ## Signals
//!
//! 1. **Explicit rejection** — the current input is a small,
//!    closed-set negation phrase that follows adam's reply:
//!    «жоқ», «олай емес», «дұрыс емес», «нет», «не то», «ты не
//!    понял», «қате», «дұрыс жауап емес», etc.
//!
//! 2. **Rephrase** — the current input has high token overlap with
//!    the previous user input (similarity ≥ 0.55) AND is not a
//!    word-for-word repeat (similarity < 1.0).  Indicates the user
//!    asked the same thing differently because adam missed it.
//!    The threshold is conservative — we want to under-flag
//!    rather than mis-flag a legitimately different question.
//!
//! 3. **Correction prefix** — the current input begins with one of
//!    a small set of clarifying phrases: «Мен X-ні айттым»,
//!    «Я имел в виду X», «Я хотел сказать X», «Мен айтайын дегенім
//!    X».  These are unambiguous corrections.
//!
//! When ANY signal fires, the detector returns a
//! [`RejectionSignal`] with the matched signal kind and the
//! [`JournalTurn`] being rejected.  Callers (the voice REPL main
//! loop in rc5; the persist layer in rc6) decide what to do with
//! the signal.

use crate::session_journal::{JournalTurn, SessionJournal};

/// Which signal flagged the rejection.  Kept distinct so the audit
/// log can show what evidence the detector used, and so rc6's
/// persistence layer can apply different weights / confidences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectionKind {
    /// Closed-set explicit phrase («жоқ», «нет», «олай емес», …).
    Explicit,
    /// Token overlap ≥ threshold with previous user input.
    Rephrase,
    /// Input starts with a "I meant X" prefix.
    Correction,
}

/// One rejection event — the kind, plus a cloned snapshot of the
/// turn being rejected.
#[derive(Debug, Clone)]
pub struct RejectionSignal {
    pub kind: RejectionKind,
    pub rejected_turn: JournalTurn,
}

/// Question-marker tokens excluded from the rephrase overlap.
///
/// **v6.5.0-rc7 (2026-06-09 audit-T13/T14 fix).**  rc6 live audit
/// surfaced two false-positive rephrase signals:
///
///   T12: «Бүгін — қай — күн.»        → AskDate ✓ «сәрсенбі»
///   T13: «Кеше қай күн болды.»        → REPHRASE detected (wrong)
///   T14: «Ертең қай күн болады.»      → REPHRASE detected (wrong)
///
/// All three are *different* date queries (today / yesterday /
/// tomorrow), but they share the question-skeleton «қай күн».  Plain
/// containment overlap fires because those tokens dominate the small
/// set.  The fix: strip question-markers BEFORE computing overlap,
/// so the comparison runs on the content tokens that actually
/// distinguish the queries.
///
/// The list is closed and small — only words that carry no content
/// information on their own.  Adding more risks under-flagging real
/// rephrases.
const STOPWORDS: &[&str] = &[
    // Kazakh question-markers / interrogative particles
    "қай",
    "не",
    "кім",
    "қашан",
    "қандай",
    "қанша",
    "неше",
    "қайда",
    "қалай",
    "ма",
    "ме",
    "ба",
    "бе",
    // Kazakh copula / auxiliary that surfaces in most questions
    "болды",
    "болады",
    "болсын",
    "болса",
    // Kazakh topic-skeleton nouns that appear in most queries of
    // their family — sharing one of these is not enough signal to
    // call a rephrase.  «бүгін / кеше / ертең» are the distinguishing
    // tokens, «күн» is the shared skeleton.
    "күн",
    "күні",
    "сағат",
    "уақыт",
    "жыл",
    "жылы",
    "ай",
    // Russian temporal skeleton
    "день",
    "час",
    "год",
    "месяц",
    // Russian — same families
    "что",
    "какой",
    "какая",
    "какое",
    "какие",
    "когда",
    "где",
    "куда",
    "откуда",
    "кто",
    "как",
    "ли",
    "был",
    "была",
    "было",
    "будет",
];

/// Default containment threshold for the rephrase signal.
///
/// The metric is `|A ∩ B| / min(|A|, |B|)` — the fraction of the
/// SMALLER input's tokens that appear in the other one.  Jaccard
/// (`|A ∩ B| / |A ∪ B|`) is too strict when the rephrase adds new
/// words: "Қазақстандағы жазушыларды білесің ба" vs "Қазақстандағы
/// жазушылар туралы білесің ба" share 3 tokens out of 4 in the
/// shorter (= 0.75) but only 3 / 6 by Jaccard (= 0.50).  Containment
/// matches the rephrase intent more faithfully.
///
/// Threshold 0.50 sits above the unrelated-follow-up noise floor
/// (≈0.10-0.25) and below the natural rephrase ceiling (≈0.70-1.00).
pub const REPHRASE_OVERLAP_THRESHOLD: f32 = 0.50;

/// Closed-set explicit-rejection phrases (lowercased, sentence-
/// punctuation stripped before matching).  Kazakh + Russian + a
/// few hybrids that surface in real audit transcripts.
///
/// Matching rule (see [`is_explicit_rejection`]): the normalised
/// input is `==` to a phrase, OR `starts_with` a phrase AND has
/// ≤4 total tokens.  The 4-token cap blocks false positives on
/// long sentences that happen to begin with "Нет, ...".
const EXPLICIT_REJECTIONS: &[&str] = &[
    // Kazakh
    "жоқ",
    "жок",
    "олай емес",
    "дұрыс емес",
    "қате",
    "дұрыс жауап емес",
    "сен түсінбедің",
    "сен мені түсінбедің",
    "түсінбедің",
    "жоқ олай емес",
    "жоқ дұрыс емес",
    // Russian
    "нет",
    "не то",
    "не так",
    "неправильно",
    "ты не понял",
    "ты меня не понял",
    "это не то",
    "это неправильно",
    "не верно",
    "неверно",
    "нет не то",
    "нет не так",
];

/// Closed-set correction-prefix phrases.  Each entry is the FULL
/// prefix the detector matches against (lowercased).  Match is
/// `starts_with` after normalisation, so the user's clarification
/// content trails the prefix.
const CORRECTION_PREFIXES: &[&str] = &[
    // Kazakh
    "мен айтайын дегенім",
    "мен айтайын деген",
    "мен айттым",
    // Russian
    "я имел в виду",
    "я хотел сказать",
    "я имела в виду",
    "я говорил про",
    "я говорила про",
    "я про",
];

/// Run all signals on `current_input` against the journal and
/// return the FIRST one that fires (priority: Explicit > Correction
/// > Rephrase).  Returns `None` when the journal is empty or no
/// signal triggers.
pub fn detect(current_input: &str, journal: &SessionJournal) -> Option<RejectionSignal> {
    let prev = journal.prev()?;
    let normalised = normalise_for_match(current_input);

    if is_explicit_rejection(&normalised) {
        return Some(RejectionSignal {
            kind: RejectionKind::Explicit,
            rejected_turn: prev.clone(),
        });
    }

    if is_correction_prefix(&normalised) {
        return Some(RejectionSignal {
            kind: RejectionKind::Correction,
            rejected_turn: prev.clone(),
        });
    }

    let overlap = token_overlap(&normalised, &normalise_for_match(&prev.input_raw));
    if (REPHRASE_OVERLAP_THRESHOLD..1.0).contains(&overlap) {
        return Some(RejectionSignal {
            kind: RejectionKind::Rephrase,
            rejected_turn: prev.clone(),
        });
    }

    None
}

/// Lower-case + strip sentence punctuation + collapse whitespace.
/// Mirrors the way the journal stores `input_raw` so the comparison
/// is stable.
fn normalise_for_match(s: &str) -> String {
    let lower = s.to_lowercase();
    let cleaned: String = lower
        .chars()
        .map(|c| {
            if matches!(c, '.' | ',' | '!' | '?' | ';' | ':' | '«' | '»' | '"') {
                ' '
            } else {
                c
            }
        })
        .collect();
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_explicit_rejection(normalised: &str) -> bool {
    let n_tokens = normalised.split_whitespace().count();
    EXPLICIT_REJECTIONS.iter().any(|p| {
        if normalised == *p {
            return true;
        }
        // Short utterance starting with a rejection — covers
        // "Нет, не то." → normalised "нет не то" which is in the
        // list directly, plus shorter casual forms.  Cap at 4
        // tokens to avoid matching legitimate long sentences that
        // begin with "Нет, ...".
        if n_tokens <= 4 && normalised.starts_with(p) {
            // Confirm word-boundary after the prefix.
            let suffix = &normalised[p.len()..];
            return suffix.is_empty() || suffix.starts_with(' ');
        }
        false
    })
}

fn is_correction_prefix(normalised: &str) -> bool {
    CORRECTION_PREFIXES
        .iter()
        .any(|p| normalised.starts_with(p))
}

/// Containment overlap on whitespace-split tokens — `|A ∩ B| /
/// min(|A|, |B|)`.  Returns 0.0 for either side empty.  Range
/// [0.0, 1.0].  See [`REPHRASE_OVERLAP_THRESHOLD`] for why we
/// prefer containment to Jaccard here.
///
/// **rc7 — stopword filter.**  [`STOPWORDS`] tokens are removed
/// from BOTH sides before counting.  If either side ends up empty
/// after stopword removal, return 0.0 (no overlap signal — the
/// utterance was entirely question-skeleton and carries no content
/// to compare).
fn token_overlap(a: &str, b: &str) -> f32 {
    use std::collections::HashSet;
    let toks_a: HashSet<&str> = a
        .split_whitespace()
        .filter(|t| !STOPWORDS.contains(t))
        .collect();
    let toks_b: HashSet<&str> = b
        .split_whitespace()
        .filter(|t| !STOPWORDS.contains(t))
        .collect();
    if toks_a.is_empty() || toks_b.is_empty() {
        return 0.0;
    }
    let inter = toks_a.intersection(&toks_b).count() as f32;
    let smaller = toks_a.len().min(toks_b.len()) as f32;
    inter / smaller
}

/// One-line audit-log summary of a signal.  Used by the voice REPL
/// main loop to print what the detector saw.
pub fn render_log(signal: &RejectionSignal) -> String {
    let kind = match signal.kind {
        RejectionKind::Explicit => "explicit",
        RejectionKind::Rephrase => "rephrase",
        RejectionKind::Correction => "correction",
    };
    let rejected_no = signal.rejected_turn.turn_no;
    let rejected_intent = signal.rejected_turn.intent.as_deref().unwrap_or("(none)");
    format!("kind={kind} rejected_turn=#{rejected_no} rejected_intent={rejected_intent}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jrn_with_prev(input: &str, intent: &str, output: &str) -> SessionJournal {
        let mut j = SessionJournal::new();
        j.append(input, input, Some(intent.into()), Some(0.9), output);
        j
    }

    #[test]
    fn empty_journal_yields_no_signal() {
        let j = SessionJournal::new();
        assert!(detect("анда қандай таулар бар", &j).is_none());
    }

    #[test]
    fn explicit_kazakh_rejection_fires() {
        let j = jrn_with_prev("Қазақстандағы жазушылар", "AskAboutTopic", "Мемлекет");
        let sig = detect("Жоқ.", &j).expect("explicit rejection");
        assert_eq!(sig.kind, RejectionKind::Explicit);
        assert_eq!(sig.rejected_turn.output, "Мемлекет");
    }

    #[test]
    fn russian_rejection_fires() {
        let j = jrn_with_prev("писатели Казахстана", "AskAboutTopic", "Государство");
        let sig = detect("Нет, не то.", &j).expect("rejection");
        assert_eq!(sig.kind, RejectionKind::Explicit);
    }

    #[test]
    fn correction_prefix_fires() {
        let j = jrn_with_prev("писатели Казахстана", "AskAboutTopic", "Государство");
        let sig = detect("Я имел в виду писателей", &j).expect("correction");
        assert_eq!(sig.kind, RejectionKind::Correction);
    }

    #[test]
    fn rephrase_fires_on_high_overlap() {
        // Original: "Қазақстандағы жазушыларды білесің ба"
        // Rephrase: "Қазақстанда жазушылар туралы білесің ба"
        // Shared tokens: жазушылар... білесің ба → high overlap
        let j = jrn_with_prev(
            "Қазақстандағы жазушыларды білесің ба",
            "AskAboutTopic",
            "Мемлекет",
        );
        let sig = detect("Қазақстандағы жазушылар туралы білесің ба", &j).expect("rephrase");
        assert_eq!(sig.kind, RejectionKind::Rephrase);
    }

    #[test]
    fn word_for_word_repeat_does_not_fire_as_rephrase() {
        // Exact repeat (overlap == 1.0) is excluded from the
        // rephrase signal — the user may just be testing STT, not
        // rejecting.  rc6 may revisit this.
        let j = jrn_with_prev("Қазақстандағы жазушылар", "AskAboutTopic", "Мемлекет");
        assert!(detect("Қазақстандағы жазушылар", &j).is_none());
    }

    #[test]
    fn unrelated_question_does_not_fire() {
        let j = jrn_with_prev("Қазақстандағы жазушылар", "AskAboutTopic", "Мемлекет");
        // Completely unrelated follow-up — should NOT fire.
        assert!(detect("Қазір сағат неше", &j).is_none());
    }

    /// **v6.5.0-rc7 audit fix.**  rc6 live audit T13/T14 falsely
    /// fired rephrase on legitimately different date queries that
    /// happened to share the «қай күн» skeleton.  rc7 stopword
    /// filter strips the question-markers before overlap.
    #[test]
    fn rc7_question_skeleton_does_not_trigger_rephrase() {
        let j = jrn_with_prev("Бүгін қай күн", "AskDate", "сәрсенбі");

        // "Кеше қай күн болды" — different day, but the previous
        // detector saw {қай, күн} ⊂ smaller → fired.  After rc7
        // stopword filter, content tokens are {бүгін} vs {кеше} → 0.
        assert!(
            detect("Кеше қай күн болды.", &j).is_none(),
            "{{бүгін}} vs {{кеше}} share zero content tokens"
        );

        // "Ертең қай күн болады" — same family, also a new question.
        assert!(detect("Ертең қай күн болады.", &j).is_none());

        // Sanity: the writers-rephrase case from rc5 still fires
        // because the CONTENT token «жазушы» is shared.
        let j = jrn_with_prev(
            "Қазақстандағы жазушыларды білесің ба",
            "AskAboutTopic",
            "Мемлекет",
        );
        let sig = detect("Қазақстандағы жазушылар туралы білесің ба", &j)
            .expect("legitimate rephrase must still fire");
        assert_eq!(sig.kind, RejectionKind::Rephrase);
    }

    #[test]
    fn token_overlap_containment_matches_known_values() {
        // Containment = |A ∩ B| / min(|A|, |B|)
        assert_eq!(token_overlap("a b c", "a b c"), 1.0);
        // {a b c} ∩ {a b} = {a b} (size 2), min(3, 2) = 2 → 1.0
        assert_eq!(token_overlap("a b c", "a b"), 1.0);
        assert_eq!(token_overlap("a b", "c d"), 0.0);
        assert_eq!(token_overlap("", "a"), 0.0);
        // Partial: {a b c d} ∩ {a b e f} = {a b}, min(4, 4) = 4 → 0.5
        assert_eq!(token_overlap("a b c d", "a b e f"), 0.5);
    }
}
