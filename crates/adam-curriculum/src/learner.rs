// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Learner state — per-concept mastery records.
//!
//! The mastery record is the public-facing artefact: a learner can
//! show it to a parent, teacher, employer, or admissions committee
//! as evidence of what they actually know.
//!
//! Records are append-only by design — a concept can be re-tested
//! and the mastery field updated, but the history of attempts is
//! preserved for audit. The L10-edu outcome verifier is the only
//! component that may declare a concept "mastered" or revoke that
//! status.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::concept::ConceptId;

/// A single test attempt on a concept.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttemptRecord {
    /// Local timestamp in ISO 8601 (UTC). The system clock is the
    /// source; for offline-first deployments without a synced clock
    /// the device's monotonic counter is acceptable.
    pub timestamp: String,
    /// Number of test items drawn from the concept's bank.
    pub items_attempted: u32,
    /// Number of items answered correctly.
    pub items_correct: u32,
    /// Wall-clock duration in seconds.
    pub time_taken_s: u32,
}

impl AttemptRecord {
    pub fn score(&self) -> f32 {
        if self.items_attempted == 0 {
            return 0.0;
        }
        self.items_correct as f32 / self.items_attempted as f32
    }
}

/// Per-concept mastery state. Aggregates the attempt history.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConceptMastery {
    /// All attempts on this concept, chronological order.
    #[serde(default)]
    pub attempts: Vec<AttemptRecord>,
    /// Set to `true` by `outcome::verify_concept` when the rubric is
    /// satisfied. The L10-edu outcome verifier is the only writer.
    #[serde(default)]
    pub mastered: bool,
}

impl ConceptMastery {
    /// Best-attempt score across the attempt history.
    pub fn best_score(&self) -> f32 {
        self.attempts
            .iter()
            .map(|a| a.score())
            .fold(0.0f32, f32::max)
    }

    /// Most-recent-attempt score (or 0.0 if no attempts).
    pub fn latest_score(&self) -> f32 {
        self.attempts.last().map(|a| a.score()).unwrap_or(0.0)
    }
}

/// The full learner record. Indexed by concept id; concepts the
/// learner has not yet attempted are absent from the map (no
/// `ConceptMastery::default` placeholders) so the file stays compact.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LearnerRecord {
    /// Opaque identifier — the device generates a UUID on first
    /// install; the learner can export and re-import it to switch
    /// devices.
    pub learner_id: String,
    /// Display name as configured by the learner. Optional; the
    /// product never requires real-name disclosure.
    #[serde(default)]
    pub display_name: String,
    /// Mastery state per concept.
    #[serde(default)]
    pub mastery: HashMap<ConceptId, ConceptMastery>,
}

impl LearnerRecord {
    pub fn new(learner_id: impl Into<String>) -> Self {
        Self {
            learner_id: learner_id.into(),
            display_name: String::new(),
            mastery: HashMap::new(),
        }
    }

    /// True iff the concept has been declared mastered by
    /// `outcome::verify_concept`.
    pub fn is_mastered(&self, id: &ConceptId) -> bool {
        self.mastery.get(id).map(|m| m.mastered).unwrap_or(false)
    }

    /// Append an attempt record for `concept`. Mastery flag is
    /// NOT updated here — that is the outcome verifier's job.
    pub fn record_attempt(&mut self, concept: ConceptId, attempt: AttemptRecord) {
        self.mastery
            .entry(concept)
            .or_default()
            .attempts
            .push(attempt);
    }

    /// Mark a concept as mastered. Only the outcome verifier should
    /// call this; downstream code goes through
    /// `outcome::verify_concept`.
    pub(crate) fn mark_mastered(&mut self, id: &ConceptId) {
        self.mastery.entry(id.clone()).or_default().mastered = true;
    }

    /// Number of concepts the learner has mastered.
    pub fn mastered_count(&self) -> usize {
        self.mastery.values().filter(|m| m.mastered).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attempt_score_handles_zero_items() {
        let a = AttemptRecord {
            timestamp: "2026-05-17T00:00:00Z".into(),
            items_attempted: 0,
            items_correct: 0,
            time_taken_s: 0,
        };
        assert_eq!(a.score(), 0.0);
    }

    #[test]
    fn best_score_picks_max() {
        let mut m = ConceptMastery::default();
        m.attempts.push(AttemptRecord {
            timestamp: "t1".into(),
            items_attempted: 10,
            items_correct: 5,
            time_taken_s: 60,
        });
        m.attempts.push(AttemptRecord {
            timestamp: "t2".into(),
            items_attempted: 10,
            items_correct: 9,
            time_taken_s: 60,
        });
        assert!((m.best_score() - 0.9).abs() < 1e-6);
    }

    #[test]
    fn learner_records_attempt_without_mastering() {
        let mut l = LearnerRecord::new("test-uuid");
        let cid = ConceptId::new("math::a");
        l.record_attempt(
            cid.clone(),
            AttemptRecord {
                timestamp: "t".into(),
                items_attempted: 5,
                items_correct: 4,
                time_taken_s: 30,
            },
        );
        assert!(!l.is_mastered(&cid)); // attempt alone doesn't grant mastery
        assert_eq!(l.mastery[&cid].attempts.len(), 1);
    }
}
