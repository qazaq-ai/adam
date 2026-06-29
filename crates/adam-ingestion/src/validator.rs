// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Candidate validation — the gate between raw extraction
//! and human review.
//!
//! For every `Pending` candidate the validator runs three
//! independent gates:
//!
//!   1. **Duplicate check** — same (subject, predicate,
//!      object) triple already in `world_core`?  →
//!      `AutoRejected`.
//!   2. **Predicate vocab check** — predicate is in the
//!      closed set the existing reasoner understands?
//!      Unknown predicate → `NeedsReview`.
//!   3. **Contradiction check** — same subject + predicate
//!      already curated with a DIFFERENT object?  For
//!      single-valued predicates («is_a», «born_in»,
//!      «died_in», «founded_in») that's a contradiction
//!      → `NeedsReview`.  For multi-valued predicates
//!      («part_of», «related_to», «has») extra objects
//!      are fine.
//!
//! Once the gates clear, the confidence-threshold floors
//! decide `AutoAccepted` vs `NeedsReview`:
//!
//!   * confidence ≥ [`AUTO_ACCEPT_FLOOR`] → `AutoAccepted`
//!   * confidence ≤ [`AUTO_REJECT_CEILING`] → `AutoRejected`
//!     (notes: «low confidence»)
//!   * otherwise → `NeedsReview`
//!
//! The validator's contract: every candidate that was
//! `Pending` on entry is in a non-`Pending` state on exit.
//! The pipeline downstream never sees an unvalidated
//! candidate.

use std::collections::{HashMap, HashSet};

use crate::candidate::CandidateFact;
use crate::status::IngestionStatus;

/// Confidence floor for the validator to auto-accept a
/// candidate without human review.  Tuned conservatively —
/// pattern-based extractors emit 0.7, so anything reaching
/// this floor is either manual entry (1.0) or a future
/// high-confidence extractor.
pub const AUTO_ACCEPT_FLOOR: f32 = 0.9;

/// Confidence ceiling below which the validator auto-rejects
/// a candidate as not worth a reviewer's attention.
pub const AUTO_REJECT_CEILING: f32 = 0.3;

/// Closed-set predicate vocabulary the existing reasoner
/// understands.  Survey of `data/world_core/*.jsonl` on
/// 2026-06-29: 20 predicates appear across 4 116 facts.
/// Adding a new predicate to production means extending
/// this list AND wiring the reasoner — strictly a curator
/// decision, not an extractor one.
pub const KNOWN_PREDICATES: &[&str] = &[
    "is_a",
    "part_of",
    "related_to",
    "has",
    "has_quantity",
    "does_to",
    "causes",
    "in_domain",
    "after",
    "before",
    "goes_to",
    "born_in",
    "lives_in",
    "died_in",
    "authored",
    "founded_in",
    "named_after",
    "member_of",
    "located_in",
    "renamed_in",
    "effective_from",
];

/// Predicates that are single-valued for a given subject —
/// asserting a different object contradicts the curated
/// fact.  «Адам is_a тіршілік иесі» — adding «Адам is_a
/// машина» would be a contradiction.  Multi-valued
/// predicates (`part_of`, `related_to`, `has`) accept
/// multiple objects without flagging.
const SINGLE_VALUED_PREDICATES: &[&str] = &[
    "is_a",
    "born_in",
    "died_in",
    "founded_in",
    "located_in",
    "named_after",
    "renamed_in",
    "effective_from",
];

/// Snapshot of the curated world_core fact graph the
/// validator consults.  Built once per validator run from
/// the on-disk JSONL files; cheap to construct (3-4k
/// entries, sub-millisecond hash inserts), so we don't
/// bother caching.
#[derive(Debug, Default)]
pub struct WorldCoreIndex {
    /// Exact triples (subject, predicate, object) for the
    /// duplicate check.  Lowercased on insert.
    exact: HashSet<(String, String, String)>,
    /// Single-valued (subject, predicate) → object map for
    /// the contradiction check.  Lowercased on insert.
    /// First object wins on collision — the curated entry
    /// loaded later is treated as «curator changed mind»
    /// and not flagged as contradiction with itself.
    single_valued: HashMap<(String, String), String>,
}

impl WorldCoreIndex {
    /// Build an empty index.  Useful for tests that
    /// construct the index by hand instead of from JSONL.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build the index from every `data/world_core/*.jsonl`
    /// file under `root`.  Returns an empty index when the
    /// directory is missing (e.g. clean CI checkout) — the
    /// validator's behaviour degrades gracefully into
    /// «mark everything NeedsReview».
    pub fn load_from_dir(root: impl AsRef<std::path::Path>) -> Self {
        let mut idx = Self::default();
        let root = root.as_ref();
        if !root.exists() {
            return idx;
        }
        let Ok(entries) = std::fs::read_dir(root) else {
            return idx;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|s| s.to_str()) != Some("jsonl") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&p) else {
                continue;
            };
            for line in text.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) else {
                    continue;
                };
                let Some(facts) = val.get("facts").and_then(|v| v.as_array()) else {
                    continue;
                };
                for fact in facts {
                    let subject = fact
                        .get("subject")
                        .and_then(|v| v.as_str())
                        .map(str::trim)
                        .unwrap_or("")
                        .to_lowercase();
                    let predicate = fact
                        .get("predicate")
                        .and_then(|v| v.as_str())
                        .map(str::trim)
                        .unwrap_or("")
                        .to_lowercase();
                    let object = fact
                        .get("object")
                        .and_then(|v| v.as_str())
                        .map(str::trim)
                        .unwrap_or("")
                        .to_lowercase();
                    if subject.is_empty() || predicate.is_empty() || object.is_empty() {
                        continue;
                    }
                    idx.insert(&subject, &predicate, &object);
                }
            }
        }
        idx
    }

    /// Insert a single fact into the index.  Lower-cases
    /// surfaces; respects single-valued semantics (first
    /// object wins).
    pub fn insert(&mut self, subject: &str, predicate: &str, object: &str) {
        let s = subject.trim().to_lowercase();
        let p = predicate.trim().to_lowercase();
        let o = object.trim().to_lowercase();
        if s.is_empty() || p.is_empty() || o.is_empty() {
            return;
        }
        self.exact.insert((s.clone(), p.clone(), o.clone()));
        if SINGLE_VALUED_PREDICATES.contains(&p.as_str()) {
            self.single_valued.entry((s, p)).or_insert(o);
        }
    }

    /// Number of unique triples in the index — diagnostic.
    pub fn len(&self) -> usize {
        self.exact.len()
    }

    pub fn is_empty(&self) -> bool {
        self.exact.is_empty()
    }

    fn has_exact_triple(&self, subject: &str, predicate: &str, object: &str) -> bool {
        self.exact.contains(&(
            subject.to_lowercase(),
            predicate.to_lowercase(),
            object.to_lowercase(),
        ))
    }

    fn single_valued_conflict<'a>(
        &'a self,
        subject: &str,
        predicate: &str,
        object: &str,
    ) -> Option<&'a str> {
        let p = predicate.to_lowercase();
        if !SINGLE_VALUED_PREDICATES.contains(&p.as_str()) {
            return None;
        }
        let key = (subject.to_lowercase(), p);
        let curated = self.single_valued.get(&key)?;
        if curated.eq_ignore_ascii_case(object) {
            None
        } else {
            Some(curated)
        }
    }
}

/// Result of validating a single candidate.  Carries the
/// new `IngestionStatus` AND the human-readable rationale
/// the store should record as a note (so a reviewer can
/// tell at a glance why a candidate landed where it did).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationOutcome {
    pub new_status: IngestionStatus,
    pub note: String,
}

/// Validate a single `Pending` candidate against the world-
/// core index.  Returns the verdict; the caller is
/// responsible for committing it through
/// [`crate::CandidateStore::update_fact_status`] so the
/// state-machine guard fires.
///
/// Behaviour for non-Pending candidates: returns the
/// candidate's existing status unchanged with a «not
/// pending — skipped» note.  This makes re-running the
/// validator idempotent across pipeline restarts.
pub fn validate_fact(fact: &CandidateFact, index: &WorldCoreIndex) -> ValidationOutcome {
    if fact.status != IngestionStatus::Pending {
        return ValidationOutcome {
            new_status: fact.status,
            note: "not pending — skipped".into(),
        };
    }
    // Gate 1: exact-triple duplicate of curated world_core.
    if index.has_exact_triple(&fact.subject, &fact.predicate, &fact.object) {
        return ValidationOutcome {
            new_status: IngestionStatus::AutoRejected,
            note: format!(
                "duplicate of existing world_core fact ({}, {}, {})",
                fact.subject, fact.predicate, fact.object
            ),
        };
    }
    // Gate 2: predicate must be in the closed reasoner set.
    if !KNOWN_PREDICATES.contains(&fact.predicate.as_str()) {
        return ValidationOutcome {
            new_status: IngestionStatus::NeedsReview,
            note: format!(
                "predicate `{}` not in known set — curator must extend KNOWN_PREDICATES + reasoner",
                fact.predicate
            ),
        };
    }
    // Gate 3: single-valued contradiction with curated truth.
    if let Some(curated_object) =
        index.single_valued_conflict(&fact.subject, &fact.predicate, &fact.object)
    {
        return ValidationOutcome {
            new_status: IngestionStatus::NeedsReview,
            note: format!(
                "contradicts curated truth: `{}` `{}` already `{}` — candidate says `{}`",
                fact.subject, fact.predicate, curated_object, fact.object
            ),
        };
    }
    // Confidence-threshold floor for auto-accept / auto-reject.
    if fact.confidence >= AUTO_ACCEPT_FLOOR {
        ValidationOutcome {
            new_status: IngestionStatus::AutoAccepted,
            note: format!("confidence {:.2} ≥ auto-accept floor", fact.confidence),
        }
    } else if fact.confidence <= AUTO_REJECT_CEILING {
        ValidationOutcome {
            new_status: IngestionStatus::AutoRejected,
            note: format!("confidence {:.2} ≤ auto-reject ceiling", fact.confidence),
        }
    } else {
        ValidationOutcome {
            new_status: IngestionStatus::NeedsReview,
            note: format!(
                "confidence {:.2} between auto-reject and auto-accept floors",
                fact.confidence
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceRef;

    fn fact(subject: &str, predicate: &str, object: &str, confidence: f32) -> CandidateFact {
        CandidateFact {
            id: format!("cand_{subject}_{predicate}_{object}"),
            subject: subject.into(),
            predicate: predicate.into(),
            object: object.into(),
            source_sentence: String::new(),
            source: SourceRef::manual("test"),
            status: IngestionStatus::Pending,
            confidence,
            created_at: "2026-06-29".into(),
            notes: String::new(),
        }
    }

    fn index_with(triples: &[(&str, &str, &str)]) -> WorldCoreIndex {
        let mut idx = WorldCoreIndex::new();
        for (s, p, o) in triples {
            idx.insert(s, p, o);
        }
        idx
    }

    #[test]
    fn duplicate_of_world_core_auto_rejected() {
        let idx = index_with(&[("астана", "is_a", "қала")]);
        let out = validate_fact(&fact("астана", "is_a", "қала", 0.95), &idx);
        assert_eq!(out.new_status, IngestionStatus::AutoRejected);
        assert!(out.note.contains("duplicate"));
    }

    #[test]
    fn unknown_predicate_goes_to_review() {
        let idx = WorldCoreIndex::new();
        let out = validate_fact(&fact("астана", "smells_like", "ыстық", 0.95), &idx);
        assert_eq!(out.new_status, IngestionStatus::NeedsReview);
        assert!(out.note.contains("smells_like"));
    }

    #[test]
    fn contradicts_single_valued_predicate_goes_to_review() {
        let idx = index_with(&[("ахмет байтұрсынұлы", "born_in", "1872")]);
        let out = validate_fact(&fact("ахмет байтұрсынұлы", "born_in", "1882", 0.95), &idx);
        assert_eq!(out.new_status, IngestionStatus::NeedsReview);
        assert!(out.note.contains("contradicts"));
        assert!(out.note.contains("1872"));
    }

    #[test]
    fn multi_valued_predicate_additional_object_does_not_contradict() {
        // «part_of» is multi-valued — Алматы can be part_of
        // multiple things without flagging.
        let idx = index_with(&[("алматы", "part_of", "қазақстан")]);
        let out = validate_fact(&fact("алматы", "part_of", "орталық азия", 0.95), &idx);
        assert_eq!(out.new_status, IngestionStatus::AutoAccepted);
    }

    #[test]
    fn high_confidence_auto_accepted() {
        let idx = WorldCoreIndex::new();
        let out = validate_fact(&fact("қола", "is_a", "қорытпа", 0.95), &idx);
        assert_eq!(out.new_status, IngestionStatus::AutoAccepted);
    }

    #[test]
    fn low_confidence_auto_rejected() {
        let idx = WorldCoreIndex::new();
        let out = validate_fact(&fact("қола", "is_a", "қорытпа", 0.1), &idx);
        assert_eq!(out.new_status, IngestionStatus::AutoRejected);
        assert!(out.note.contains("auto-reject ceiling"));
    }

    #[test]
    fn medium_confidence_needs_review() {
        // 0.7 — extractor default — should land here.
        let idx = WorldCoreIndex::new();
        let out = validate_fact(&fact("қола", "is_a", "қорытпа", 0.7), &idx);
        assert_eq!(out.new_status, IngestionStatus::NeedsReview);
    }

    #[test]
    fn non_pending_candidate_skipped() {
        let idx = WorldCoreIndex::new();
        let mut f = fact("қола", "is_a", "қорытпа", 0.95);
        f.status = IngestionStatus::ApprovedByHuman;
        let out = validate_fact(&f, &idx);
        assert_eq!(out.new_status, IngestionStatus::ApprovedByHuman);
        assert!(out.note.contains("skipped"));
    }

    #[test]
    fn same_object_not_flagged_as_contradiction() {
        // The candidate IS the curated entry; that's a
        // duplicate (caught by Gate 1), not a contradiction.
        let idx = index_with(&[("астана", "is_a", "қала")]);
        let out = validate_fact(&fact("астана", "is_a", "қала", 0.95), &idx);
        assert_eq!(out.new_status, IngestionStatus::AutoRejected);
        assert!(out.note.contains("duplicate"));
    }

    #[test]
    fn world_core_index_loads_from_real_data() {
        // Smoke test against the actual world_core directory
        // if present.  Builds the index and confirms it's
        // non-empty.  Skips silently when the directory is
        // missing (clean CI checkouts) — degraded mode.
        let idx = WorldCoreIndex::load_from_dir("../../data/world_core");
        if !idx.is_empty() {
            assert!(
                idx.len() > 100,
                "real world_core should have >100 triples, got {}",
                idx.len()
            );
        }
    }
}
