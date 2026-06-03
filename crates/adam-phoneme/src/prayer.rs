// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Phase 11 (2026-05-31): Arabic-prayer code-switching detection.
//!
//! Classical Kazakh literature — Abai's «Қара сөздер», Шакарим,
//! traditional бата/дұға — regularly embeds Arabic prayer
//! phrases into otherwise-Kazakh text. The user identified this
//! by ear on the Abai Word #36 audiobook (2026-05-31):
//!
//! > «Там в начале и далее иногда звучит арабская слова из
//! >  молитв. Необходимо, чтобы модель понимала, что это
//! >  чтение молитвы.»
//!
//! Without prayer-mode awareness:
//! - STT phonotactic gate rejects Arabic patterns as «illegal
//!   Kazakh» (вокальная гармония violated, кластеры like
//!   «-стағфир-» trigger consonant-cluster prune rules).
//! - TTS misapplies the strict «ы»/«і»-drop native-root rule
//!   to Arabic words, mangling the pronunciation.
//! - The semantic layer can't attribute citations to a source
//!   (Qur'an verse, Hadith, traditional бата formula).
//!
//! ## Design
//!
//! Two-tier detection:
//!
//! 1. **Lexicon match** — exact case-insensitive substring
//!    hits against the canonical phrase list. Cheap, zero
//!    false-positives on hand-curated entries, used for the
//!    most common ~30 phrases that account for the vast
//!    majority of corpus occurrences.
//! 2. **Arabic Unicode block** — text written in original
//!    Arabic script (U+0600..U+06FF, U+FB50..U+FDFF,
//!    U+FE70..U+FEFF). Some KZ publications preserve the
//!    Arabic source verbatim before the transliteration.
//!
//! A third tier — **phonotactic-likelihood detector** — is
//! defined in this module's interface but not yet wired: a
//! span whose Kazakh-vowel-harmony score falls below a
//! threshold AND lacks lexicon match could be flagged for
//! human curation. Deferred until the lexicon's coverage
//! plateaus.
//!
//! ## Output
//!
//! [`tag_prayer_spans`] returns a `Vec<PraySpan>` with
//! byte-offset boundaries (so callers can splice the text
//! and re-render the prayer span under different
//! phonotactic rules / TTS prosody / KB-citation type).

use std::ops::Range;

/// One detected prayer-citation span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PraySpan {
    /// Byte range in the source `&str`.
    pub range: Range<usize>,
    /// The matched canonical phrase entry that triggered the
    /// detection. `None` when the span was flagged by Arabic-
    /// script Unicode rather than the lexicon.
    pub canonical: Option<&'static str>,
    /// Detector that produced this span.
    pub source: PraySource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PraySource {
    /// Exact lexicon match (one of [`CANONICAL_PHRASES`]).
    Lexicon,
    /// Run of Arabic-script Unicode characters.
    ArabicScript,
}

/// Hand-curated lexicon of Arabic prayer phrases as they
/// appear in published Kazakh literature (Cyrillic
/// transliteration). All entries are **lower-case**; matcher
/// case-insensitively scans the input. Sorted longest-first
/// to prefer the most specific match.
///
/// Sources (user 2026-05-31): Abai «Қара сөздер», Шакарим,
/// classical бата formulae, common dua taught in Kazakh
/// Islamic instruction.
pub const CANONICAL_PHRASES: &[&str] = &[
    // Longest first → matcher prefers the most specific phrase
    // when several candidates overlap on a prefix.
    "салләллаһу ғәләйһи уә сәлләм",
    "бісмілләһир-рахманир-рахим",
    "бісмілләһир рахманир рахим",
    "лә иләһе илла аллаһ",
    "ла иләһе илла аллаһ",
    "лә иләһа илла аллаһ",
    "әл-хәмду лілләһ",
    "әл хәмду лілләһ",
    "ал-хамду лиллах",
    "астағфируллаһ",
    "астагфируллах",
    "субхана аллаһ",
    "сұбхана аллаһ",
    "аллаһу акбар",
    "аллах акбар",
    "иншааллаһ",
    "иншаллаһ",
    "иншалла",
    "машааллаһ",
    "машаллаһ",
    "ассалаумағалейкум",
    "ассалаумағалейкүм",
    "уағалейкумәссалам",
    "бісмілләһ",
    "бисмиллах",
    "альхамдулиллах",
    "рахматуллаһ",
    "рахматулла",
    "тәубе",
    "омен",
    "ауминь",
    "аумин",
];

/// Detect every prayer-citation span in `text`. Returns the
/// spans in left-to-right order, with **no overlaps** —
/// adjacent matches that abut are merged into a single span.
pub fn tag_prayer_spans(text: &str) -> Vec<PraySpan> {
    let mut spans: Vec<PraySpan> = Vec::new();
    let lower = text.to_lowercase();

    // 1. Lexicon scan. Sweep each phrase in `CANONICAL_PHRASES`
    //    over the lowercase string; record every hit. The list
    //    is pre-sorted longest-first so when a shorter phrase
    //    is a prefix of a longer one (e.g. «бісмілләһ» inside
    //    «бісмілләһир-рахманир-рахим») the longer one wins.
    for &phrase in CANONICAL_PHRASES {
        let mut search_from = 0_usize;
        while let Some(off) = lower[search_from..].find(phrase) {
            let start = search_from + off;
            let end = start + phrase.len();
            // Skip if covered by an already-found longer span.
            if spans
                .iter()
                .any(|s| s.range.start <= start && end <= s.range.end)
            {
                search_from = end;
                continue;
            }
            spans.push(PraySpan {
                range: start..end,
                canonical: Some(phrase),
                source: PraySource::Lexicon,
            });
            search_from = end;
        }
    }

    // 2. Arabic-script run detection: any contiguous run of
    //    Arabic-block code points becomes one span.
    {
        let bytes = text.as_bytes();
        let mut idx = 0_usize;
        while idx < bytes.len() {
            let c = match text[idx..].chars().next() {
                Some(c) => c,
                None => break,
            };
            let clen = c.len_utf8();
            if is_arabic_script(c) {
                let start = idx;
                let mut end = idx + clen;
                let mut cursor = end;
                while cursor < bytes.len() {
                    let nc = match text[cursor..].chars().next() {
                        Some(c) => c,
                        None => break,
                    };
                    let nlen = nc.len_utf8();
                    // Allow ASCII space inside an Arabic run so
                    // multi-word prayers stay one span.
                    if is_arabic_script(nc) || nc == ' ' {
                        end = cursor + nlen;
                        cursor = end;
                    } else {
                        break;
                    }
                }
                // Trim trailing whitespace so the range hugs
                // the actual Arabic content.
                while end > start && text.as_bytes()[end - 1] == b' ' {
                    end -= 1;
                }
                if end > start {
                    spans.push(PraySpan {
                        range: start..end,
                        canonical: None,
                        source: PraySource::ArabicScript,
                    });
                }
                idx = cursor;
            } else {
                idx += clen;
            }
        }
    }

    spans.sort_by_key(|s| s.range.start);
    merge_adjacent(&mut spans);
    spans
}

fn merge_adjacent(spans: &mut Vec<PraySpan>) {
    if spans.len() <= 1 {
        return;
    }
    let mut out: Vec<PraySpan> = Vec::with_capacity(spans.len());
    let mut iter = spans.drain(..);
    let mut cur = iter.next().unwrap();
    for nxt in iter {
        // Adjacent (touching) or overlapping — merge.
        if nxt.range.start <= cur.range.end {
            cur.range.end = cur.range.end.max(nxt.range.end);
            // Keep `cur`'s canonical/source; lose the second.
        } else {
            out.push(cur);
            cur = nxt;
        }
    }
    out.push(cur);
    *spans = out;
}

fn is_arabic_script(c: char) -> bool {
    let u = c as u32;
    // Arabic, Arabic Supplement, Arabic Extended-A, Arabic
    // Presentation Forms-A, Arabic Presentation Forms-B.
    (0x0600..=0x06FF).contains(&u)
        || (0x0750..=0x077F).contains(&u)
        || (0x08A0..=0x08FF).contains(&u)
        || (0xFB50..=0xFDFF).contains(&u)
        || (0xFE70..=0xFEFF).contains(&u)
}

/// True when *any* part of `text` is a prayer span. Convenience
/// shortcut for callers that just need the boolean.
pub fn contains_prayer(text: &str) -> bool {
    !tag_prayer_spans(text).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_bismillah() {
        let t = "Бісмілләһ деп бастайды.";
        let spans = tag_prayer_spans(t);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].source, PraySource::Lexicon);
        assert!(spans[0].canonical.unwrap().starts_with("бісмілләһ"));
    }

    #[test]
    fn detects_longest_match_wins() {
        let t = "Бісмілләһир-рахманир-рахим, тағала.";
        let spans = tag_prayer_spans(t);
        assert_eq!(spans.len(), 1, "longest phrase should swallow the shorter");
        assert_eq!(
            spans[0].canonical,
            Some("бісмілләһир-рахманир-рахим"),
            "got: {:?}",
            spans[0]
        );
    }

    #[test]
    fn detects_two_separate_phrases() {
        let t = "Бісмілләһ. Кейін: Аллаһу акбар.";
        let spans = tag_prayer_spans(t);
        assert_eq!(spans.len(), 2);
    }

    #[test]
    fn detects_arabic_script_run() {
        let t = "Сура: بسم الله الرحمن الرحيم — кейін аударма.";
        let spans = tag_prayer_spans(t);
        assert!(
            spans.iter().any(|s| s.source == PraySource::ArabicScript),
            "Arabic-script run not detected; got {:?}",
            spans
        );
    }

    #[test]
    fn no_false_positive_on_pure_kazakh() {
        let t = "Қазақстан Республикасы — біздің Отанымыз.";
        assert!(tag_prayer_spans(t).is_empty());
    }

    #[test]
    fn contains_prayer_shortcut() {
        assert!(contains_prayer("ИншаАллаһ келеміз."));
        assert!(!contains_prayer("Жай хабар сөйлем."));
    }
}
