// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `broad_topic` — Stage 4 of the v6.1.0 AnswerIR research arc.
//!
//! Pre-v6.1.0 a broad-topic query like «Қазақстан туралы айт» surfaced
//! exactly one fact — whichever the v6.0.13 retrieval picked first
//! (typically an IsA row). The Codex 2026-05-22 audit identified this
//! as a relevance/completeness gap: the user asked for a SUMMARY,
//! the kernel gave one sentence.
//!
//! Stage 4 introduces a multi-claim composer for broad-topic queries
//! and their continuations:
//!
//!   - **Broad-topic query** — «X туралы айтыңыз / айтшы / айт /
//!     айтып бер / айтып беріңізші / біл» — pulls up to N=3 facts
//!     with subject == noun_hint, ranked by predicate priority.
//!   - **Continuation request** — «ал тағы / тағы айт / әрі қарай /
//!     басқа не / тағы не» — pulls up to N=3 facts for the
//!     `broad_topic_subject` carried over from the previous turn,
//!     skipping anything in `broad_topic_seen`.
//!
//! Behind `ADAM_ANSWER_IR=1`, like Stage 3. Off by default; v6.0.0
//! cascade is bit-identical when the flag is clear.
//!
//! The composer doesn't generate text — it concatenates the
//! `raw_text` of selected curated facts. Every claim in the output
//! therefore traces to exactly one curated `source_fact_id`, matching
//! the AnswerIR design-doc contract that the realiser must not have
//! a free-text channel.

use adam_reasoning::{Fact, FactSource, Predicate};

/// Maximum number of claims surfaced in a single broad-topic reply.
///
/// 3 was chosen from the design doc: enough to feel like a summary,
/// few enough that the user can ask «ал тағы» two more times before
/// exhausting most KRU-style subjects (which have ~6-8 facts each).
pub const MAX_CLAIMS_PER_TURN: usize = 3;

/// Is the input a broad-topic enumeration request?
///
/// True iff the input contains «X туралы» followed by an enumerative
/// verb («айт» / «біл»). The detector is surface-level and
/// intentionally conservative — when in doubt, return false so the
/// existing single-fact retrieval stays authoritative.
pub fn is_broad_topic_query(input: &str) -> bool {
    let lower = input.to_lowercase();
    if !lower.contains("туралы") {
        return false;
    }
    const ENUMERATIVE_VERBS: &[&str] = &[
        "айтыңыз",
        "айтшы",
        "айтып",
        "айтсаңыз",
        "айт",
        "білемін",
        "білесіз",
        "білем",
        "білесің",
        "білемін бе",
        "білесіз бе",
    ];
    ENUMERATIVE_VERBS.iter().any(|v| lower.contains(v))
}

/// Is the input a continuation request for the active broad-topic
/// subject?
///
/// True iff the input is a bare follow-up like «тағы айт» / «ал
/// тағы» / «әрі қарай» / «басқа не» — no topic noun, just an
/// invitation to keep enumerating. The detector returns false on
/// inputs that look like fresh topic queries even if they contain
/// «тағы» (e.g. «тағы бір сұрақ»).
pub fn is_continuation_request(input: &str) -> bool {
    let lower = input.to_lowercase().trim().to_string();
    const PATTERNS: &[&str] = &[
        "тағы айт",
        "ал тағы",
        "әрі қарай",
        "басқа не",
        "тағы не",
        "тағы не білесіз",
        "жалғастыр",
        "тағы бір нәрсе айт",
    ];
    PATTERNS.iter().any(|p| lower.contains(p))
}

/// Compose a multi-claim broad-topic reply for `subject` from
/// `facts`, skipping anything in `already_seen`.
///
/// Returns `None` when no unseen fact matches the subject — the
/// caller should fall through to the existing single-fact retrieval
/// (which typically lands a refusal / no-data template). The
/// `seen_out` vec is filled with the FactSource ids of the surfaced
/// claims so the caller can persist them on `DialogContext`.
///
/// Ranking: IsA first (canonical definition opens the summary),
/// then the typed-predicate facts in design-doc priority order
/// (BornIn / FoundedIn / EffectiveFrom first, then Classifies /
/// RiskLevel / Authored, then MemberOf / NamedAfter / RenamedIn,
/// finally LocatedIn / RelatedTo). Within the same predicate tier
/// the order is by `pack` then `sample_id` (alphabetical, the
/// `FactSource` derive order) — deterministic, no RNG.
pub fn compose_broad_topic(
    subject: &str,
    facts: &[Fact],
    already_seen: &[FactSource],
    seen_out: &mut Vec<FactSource>,
) -> Option<String> {
    let subject_lower = subject.to_lowercase();
    let mut candidates: Vec<&Fact> = facts
        .iter()
        .filter(|f| {
            f.subject.root.to_lowercase() == subject_lower && !already_seen.contains(&f.source)
        })
        .collect();
    candidates.sort_by_key(|f| (predicate_tier(f.predicate), f.source.clone()));

    let mut deduped_texts: Vec<String> = Vec::new();
    for fact in candidates.iter().take(MAX_CLAIMS_PER_TURN * 2) {
        let normalised = fact.raw_text.trim().to_string();
        if normalised.is_empty() {
            continue;
        }
        // Skip if a previously-picked claim's raw_text is identical
        // (curated entries sometimes share a raw_text across rows
        // like kru_002 born_in × 2 — date + place from the same
        // sentence; we want one of those, not both).
        if deduped_texts.contains(&normalised) {
            continue;
        }
        deduped_texts.push(normalised);
        seen_out.push(fact.source.clone());
        if deduped_texts.len() >= MAX_CLAIMS_PER_TURN {
            break;
        }
    }

    if deduped_texts.is_empty() {
        return None;
    }
    Some(deduped_texts.join(" "))
}

fn predicate_tier(p: Predicate) -> u8 {
    // IsA opens (the definitional sentence sets up the rest).
    // Date-shaped predicates come next — readers expect chronology
    // early in a profile-style summary. Then categorisation /
    // authorship / membership. RelatedTo last because it's the
    // catch-all "loosely linked" tier.
    match p {
        Predicate::IsA => 0,
        Predicate::BornIn | Predicate::FoundedIn | Predicate::EffectiveFrom => 1,
        Predicate::DiedIn | Predicate::RenamedIn => 2,
        Predicate::Classifies | Predicate::RiskLevel | Predicate::Authored => 3,
        Predicate::MemberOf | Predicate::NamedAfter => 4,
        Predicate::LocatedIn | Predicate::PartOf => 5,
        Predicate::Has | Predicate::HasQuantity | Predicate::LivesIn | Predicate::GoesTo => 6,
        Predicate::InDomain | Predicate::DoesTo | Predicate::Causes | Predicate::After => 7,
        Predicate::RelatedTo => 8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broad_topic_recognises_canonical_request() {
        assert!(is_broad_topic_query("Қазақстан туралы айтыңыз."));
        assert!(is_broad_topic_query("Ахмет Байтұрсынұлы туралы айтшы"));
        assert!(is_broad_topic_query("КРУ туралы айтып беріңізші"));
    }

    #[test]
    fn broad_topic_rejects_specific_predicate() {
        // Specific predicate queries route through Stage 3, not
        // here. The «туралы» marker is absent.
        assert!(!is_broad_topic_query("Ахмет Байтұрсынұлы қашан туылған?"));
        assert!(!is_broad_topic_query("КРУ қашан құрылған?"));
    }

    #[test]
    fn broad_topic_rejects_bare_topic_noun() {
        // No enumerative verb → not a broad-topic request.
        assert!(!is_broad_topic_query("Ахмет Байтұрсынұлы"));
        assert!(!is_broad_topic_query("Қазақстан"));
    }

    #[test]
    fn continuation_recognises_canonical_forms() {
        assert!(is_continuation_request("тағы айт"));
        assert!(is_continuation_request("Ал тағы?"));
        assert!(is_continuation_request("әрі қарай"));
        assert!(is_continuation_request("басқа не білесіз?"));
    }

    #[test]
    fn continuation_rejects_topic_queries() {
        // «тағы» appears, but the input is a fresh topic question.
        assert!(!is_continuation_request("тағы бір сұрақ — Абай кім?"));
    }

    #[test]
    fn predicate_tier_orders_definition_first() {
        assert!(predicate_tier(Predicate::IsA) < predicate_tier(Predicate::BornIn));
        assert!(predicate_tier(Predicate::BornIn) < predicate_tier(Predicate::Classifies));
        assert!(predicate_tier(Predicate::Classifies) < predicate_tier(Predicate::RelatedTo));
    }
}
