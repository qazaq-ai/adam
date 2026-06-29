// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Pattern-based candidate fact extractor.
//!
//! Walks a Kazakh text source line by line, matches sentences
//! against curated declarative shapes, and emits
//! [`CandidateFact`]s.  Conservative by design — every pattern
//! is narrow enough that its false-positive rate is
//! reviewer-managable, NOT so broad that the queue fills
//! with noise.
//!
//! The first shape this module handles is the canonical
//! «X — Y.» em-dash declaration that world_core entries
//! already use ubiquitously («Қазақстан — мемлекет.»,
//! «Алматы — қала.»).  Future commits add genitive
//! property shapes, location relations, etc. — each as a
//! separate matcher with its own confidence floor.
//!
//! ## What this module is NOT
//!
//! - Not an open-domain semantic parser.  No deep
//!   linguistic analysis; the patterns are surface regex
//!   matches against the typed substring structure.
//! - Not a validator.  Extracted candidates are
//!   syntactically valid by construction but may be
//!   semantically wrong (duplicate of curated truth,
//!   contradiction).  The validator phase handles those.
//! - Not deduplicating internally.  If a source mentions
//!   «Қазақстан — мемлекет» three times, three candidates
//!   surface.  The validator de-dups against world_core +
//!   intra-batch.

use crate::candidate::CandidateFact;
use crate::source::{SourceKind, SourceRef};
use crate::status::IngestionStatus;

/// Extract every `CandidateFact` the curated patterns can
/// find in `text`.  `source_path` becomes the
/// [`SourceRef::identifier`]; line numbers are tracked
/// 1-indexed.  `created_at` is provided by the caller (ISO
/// `YYYY-MM-DD`) so re-runs of the extractor on the same
/// source produce identical output — determinism that the
/// pipeline downstream depends on.
pub fn extract_facts_from_text(
    text: &str,
    source_path: &str,
    source_kind: SourceKind,
    created_at: &str,
) -> Vec<CandidateFact> {
    let mut out = Vec::new();
    for (line_idx, raw_line) in text.lines().enumerate() {
        let line_number = (line_idx + 1) as u32;
        // Skip empty / comment lines — extractor is line-
        // oriented; multi-line declarations are out of scope
        // for this first pass.
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        // Split on sentence terminators so a single line
        // with two declarations yields two candidates.
        for sentence in split_sentences(trimmed) {
            if let Some(fact) = extract_em_dash_declaration(
                sentence.trim(),
                source_path,
                source_kind,
                line_number,
                created_at,
                out.len(),
            ) {
                out.push(fact);
            }
        }
    }
    out
}

/// Split a line into sentence-shaped substrings.  Honours
/// `.` / `!` / `?` terminators; preserves the trailing
/// punctuation for downstream pattern matchers that care
/// about question-vs-statement shape.
fn split_sentences(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for ch in line.chars() {
        current.push(ch);
        if matches!(ch, '.' | '!' | '?' | '…') {
            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                out.push(trimmed);
            }
            current.clear();
        }
    }
    let tail = current.trim().to_string();
    if !tail.is_empty() {
        out.push(tail);
    }
    out
}

/// Match the canonical «X — Y.» em-dash declaration.
/// Returns `Some(CandidateFact { subject: X, predicate:
/// is_a, object: Y })` when the sentence parses; `None`
/// otherwise.  Conservative — requires the em-dash («—»,
/// U+2014) specifically, NOT a hyphen, NOT spaces-around-
/// nothing.  Kazakh world_core entries use the em-dash
/// uniformly.
fn extract_em_dash_declaration(
    sentence: &str,
    source_path: &str,
    source_kind: SourceKind,
    line_number: u32,
    created_at: &str,
    sequence: usize,
) -> Option<CandidateFact> {
    // Trim the final terminator so we don't carry it into
    // the object surface.  Only act on `.` declarations —
    // questions and exclamations aren't `is_a` claims, so
    // a sentence without a trailing period returns empty
    // here and the body-empty guard below rejects it.
    let s = sentence.trim();
    let body = s.strip_suffix('.').unwrap_or("");
    if body.is_empty() {
        return None;
    }
    // Find em-dash surrounded by single spaces.  The pattern
    // is exactly « — » (space + U+2014 + space).  Anything
    // else falls through.
    let (left, right) = body.split_once(" — ")?;
    let subject = left.trim();
    let object = right.trim();
    // Both sides must be non-empty and substantive
    // (multi-character).  Single-letter «X» / «Y» tokens
    // are too noisy to keep at this stage.
    if subject.chars().count() < 2 || object.chars().count() < 2 {
        return None;
    }
    // Reject if subject or object contains its own em-dash
    // — that's a multi-clause declaration we don't parse
    // yet («Қазақстан — мемлекет — Орталық Азияда»).
    if subject.contains(" — ") || object.contains(" — ") {
        return None;
    }
    // Reject if either side starts with a question or
    // refusal marker — those are sentence types this
    // matcher isn't meant to catch.
    let lower_subject = subject.to_lowercase();
    if lower_subject.starts_with("қандай")
        || lower_subject.starts_with("қалай")
        || lower_subject.starts_with("неге")
        || lower_subject.starts_with("кім")
        || lower_subject.starts_with("не ")
    {
        return None;
    }
    let id = format!(
        "ingest_{stem}_{line}_{seq}",
        stem = path_stem(source_path),
        line = line_number,
        seq = sequence,
    );
    Some(CandidateFact {
        id,
        subject: subject.to_lowercase(),
        predicate: "is_a".into(),
        object: object.to_lowercase(),
        source_sentence: s.to_string(),
        source: SourceRef {
            kind: source_kind,
            identifier: source_path.to_string(),
            line: Some(line_number),
            notes: String::new(),
        },
        status: IngestionStatus::Pending,
        // Confidence floor for the em-dash pattern.  Tuned
        // empirically against curated world_core sentences:
        // the pattern catches ~95 % of `X — Y.` lines with
        // ~5 % false positives (multi-clause / metaphorical
        // / Russian-language interleavings).  The 0.7
        // figure puts these candidates in NeedsReview at
        // the validator's default thresholds, not
        // AutoAccepted — appropriate for an unsupervised
        // first pass.
        confidence: 0.7,
        created_at: created_at.to_string(),
        notes: String::new(),
    })
}

/// Extract the file stem from a path-like identifier so
/// candidate ids carry the source-name root («geography_kz»
/// for `data/world_core/geography_kz.jsonl`).  Pure-string
/// implementation — doesn't depend on `std::path` so this
/// also works for URL identifiers later.
fn path_stem(identifier: &str) -> String {
    let last_slash = identifier.rfind('/').map(|i| i + 1).unwrap_or(0);
    let after_slash = &identifier[last_slash..];
    let dot = after_slash.find('.').unwrap_or(after_slash.len());
    let stem: String = after_slash[..dot]
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    if stem.is_empty() { "src".into() } else { stem }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn em_dash_declaration_extracts_single_fact() {
        let text = "Қазақстан — мемлекет.\n";
        let facts = extract_facts_from_text(
            text,
            "data/raw/sample.txt",
            SourceKind::TextFile,
            "2026-06-28",
        );
        assert_eq!(facts.len(), 1);
        let f = &facts[0];
        assert_eq!(f.subject, "қазақстан");
        assert_eq!(f.predicate, "is_a");
        assert_eq!(f.object, "мемлекет");
        assert_eq!(f.source.line, Some(1));
        assert_eq!(f.confidence, 0.7);
        assert_eq!(f.status, IngestionStatus::Pending);
        f.check_invariants().expect("invariants hold");
    }

    #[test]
    fn multiple_sentences_on_one_line_yield_multiple_facts() {
        let text = "Алматы — қала. Астана — қала.";
        let facts = extract_facts_from_text(text, "src.txt", SourceKind::TextFile, "2026-06-28");
        assert_eq!(facts.len(), 2);
        assert_eq!(facts[0].subject, "алматы");
        assert_eq!(facts[1].subject, "астана");
        // Both on the same source line.
        assert_eq!(facts[0].source.line, Some(1));
        assert_eq!(facts[1].source.line, Some(1));
        // Distinct ids by sequence index.
        assert_ne!(facts[0].id, facts[1].id);
    }

    #[test]
    fn comments_and_empty_lines_skipped() {
        let text = "\n# header comment\nАлматы — қала.\n\n";
        let facts = extract_facts_from_text(text, "src.txt", SourceKind::TextFile, "2026-06-28");
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].source.line, Some(3));
    }

    #[test]
    fn questions_not_extracted() {
        let text = "Қандай қала Алматы? Алматы — қала.";
        let facts = extract_facts_from_text(text, "src.txt", SourceKind::TextFile, "2026-06-28");
        // Only the declaration matches — the question
        // doesn't have a terminator-«.» AND starts with a
        // question word.
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].subject, "алматы");
    }

    #[test]
    fn multi_clause_em_dash_rejected() {
        // Three-segment construction («X — Y — Z») is
        // ambiguous between (X is-a Y) and (X is-a Y, Z is
        // attribute) — skip until a future commit adds a
        // proper matcher.
        let text = "Қазақстан — мемлекет — Орталық Азияда.";
        let facts = extract_facts_from_text(text, "src.txt", SourceKind::TextFile, "2026-06-28");
        assert!(facts.is_empty(), "got: {facts:?}");
    }

    #[test]
    fn hyphen_does_not_count_as_em_dash() {
        // ASCII hyphen «-» / non-spaced em-dash «—» / Cyrillic
        // dash-like punctuation should NOT match.  Only the
        // « — » (space + U+2014 + space) shape fires.
        let text = "Алматы-қала.";
        let facts = extract_facts_from_text(text, "src.txt", SourceKind::TextFile, "2026-06-28");
        assert!(facts.is_empty(), "got: {facts:?}");
    }

    #[test]
    fn determinism_across_reruns() {
        // Same input → identical CandidateFact set (ids,
        // sequence, line numbers).  Pipeline downstream
        // depends on this.
        let text = "Алматы — қала.\nАстана — қала.";
        let a = extract_facts_from_text(text, "src.txt", SourceKind::TextFile, "2026-06-28");
        let b = extract_facts_from_text(text, "src.txt", SourceKind::TextFile, "2026-06-28");
        assert_eq!(a, b);
    }

    #[test]
    fn short_subject_or_object_rejected() {
        // Single-letter «X» / «Y» tokens are too noisy —
        // reject.  Two-character minimum on each side.
        let text = "X — қала.\nАлматы — Y.";
        let facts = extract_facts_from_text(text, "src.txt", SourceKind::TextFile, "2026-06-28");
        assert!(facts.is_empty(), "got: {facts:?}");
    }

    #[test]
    fn ids_carry_source_stem() {
        let text = "Алматы — қала.";
        let facts = extract_facts_from_text(
            text,
            "data/raw/geography_kz.txt",
            SourceKind::TextFile,
            "2026-06-28",
        );
        assert!(
            facts[0].id.starts_with("ingest_geography_kz_"),
            "got: {}",
            facts[0].id
        );
    }
}
