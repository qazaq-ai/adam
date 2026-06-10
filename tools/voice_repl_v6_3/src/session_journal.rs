// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! # Session journal — v6.5 self-learning loop, foundation layer
//!
//! Per-session ring buffer of the last N voice REPL turns.  Each
//! entry captures enough context to recognise — on the NEXT turn —
//! that the user has rejected, rephrased, or corrected the previous
//! response.
//!
//! ## Why this exists
//!
//! Live audit feedback (2026-06-09):
//!
//! > «не будем заниматься вечно исправлением багов. А надо, чтобы
//! >  он сам учился находить свои ошибки и следующий раз их не
//! >  делать.»
//!
//! Past releases closed audit bugs by adding entries to fuzzy
//! tables, blacklists, intent-classifier datasets — patches that
//! work for the captured case but don't generalise.  The
//! self-learning loop is a different shape: instead of me encoding
//! every Whisper drift by hand, adam writes down its own mistakes
//! (when the user signals one), and consults that log on future
//! turns.
//!
//! ## Layering (v6.5.0-rc5 ships only the journal)
//!
//! - **rc5** — `SessionJournal`: ring buffer + observability log.
//!   Records turns; emits a single line `[journal] turn #N ...`
//!   per loop iteration so the audit transcript shows what's being
//!   captured.  No persistence yet.  No behavioural change.
//! - **rc6** — `RejectionDetector` reads the journal to spot
//!   explicit-rejection / rephrase / correction signals.
//! - **rc7** — `MistakeIndex` persists confirmed rejections to
//!   `data/mistake_corrections.jsonl` and consults it on every
//!   new turn to override the cascade.
//!
//! Each rc is a separate ship.  rc5 is the foundation, watching
//! the conversation without changing it.
//!
//! ## Capacity
//!
//! Default capacity is 5 turns — enough to catch a rephrase across
//! adam's reply + the user's clarification + a follow-up, without
//! holding indefinite session history in RAM.

use std::collections::VecDeque;

/// One completed user-adam exchange — recorded after adam emits its
/// reply, before the next user input arrives.
///
/// Several fields are unused in rc5 (the foundation ship — observability
/// only) but read by rc6's persistence layer + rc7's MistakeIndex
/// override, so `dead_code` is allowed at the struct level.  Clippy
/// would otherwise reject the rc5 build under `-D warnings`.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct JournalTurn {
    /// Monotonically increasing turn index within this session
    /// (1-based — first turn is `#1`).
    pub turn_no: usize,
    /// Raw STT output before any normalisation.
    pub input_raw: String,
    /// Post-normalisation form (`context_corrections::apply` +
    /// `fuzzy_normalise` + neural rescore).  This is what the
    /// downstream cascade actually saw.  rc6 will use it as the
    /// canonical key for the mistake_corrections.jsonl lookup.
    pub input_normalised: String,
    /// Intent classifier's top class for this turn (`None` when
    /// classifier was unavailable).
    pub intent: Option<String>,
    /// Confidence on `intent`.  `None` mirrors `intent`.  rc6 will
    /// weight the persisted correction by this confidence.
    pub intent_confidence: Option<f32>,
    /// adam's final reply that was synthesised and played back.
    /// rc6 saves this as `wrong_output` when the user rejects.
    pub output: String,
}

/// Fixed-size ring buffer of the most recent N turns.
///
/// `len()` returns the current occupancy (not the capacity); use
/// [`prev`](Self::prev) to read the most recently appended turn
/// without consuming it.
#[derive(Debug, Clone)]
pub struct SessionJournal {
    capacity: usize,
    turns: VecDeque<JournalTurn>,
    next_no: usize,
}

impl SessionJournal {
    /// Build a journal that retains the most recent `capacity`
    /// turns.  Smaller capacity = lighter memory; the rejection
    /// detector (rc6) only needs to look back ~3 turns, so the
    /// default of 5 leaves headroom without bloat.
    pub fn with_capacity(capacity: usize) -> Self {
        assert!(capacity > 0, "SessionJournal capacity must be > 0");
        Self {
            capacity,
            turns: VecDeque::with_capacity(capacity),
            next_no: 1,
        }
    }

    /// Default journal — 5-turn window.
    pub fn new() -> Self {
        Self::with_capacity(5)
    }

    /// Append one completed turn.  The `turn_no` field is auto-
    /// assigned (caller doesn't supply it).  When the buffer is
    /// full, the oldest turn is dropped.  Returns the assigned
    /// `turn_no` so callers can log it.
    pub fn append(
        &mut self,
        input_raw: impl Into<String>,
        input_normalised: impl Into<String>,
        intent: Option<String>,
        intent_confidence: Option<f32>,
        output: impl Into<String>,
    ) -> usize {
        let turn_no = self.next_no;
        let entry = JournalTurn {
            turn_no,
            input_raw: input_raw.into(),
            input_normalised: input_normalised.into(),
            intent,
            intent_confidence,
            output: output.into(),
        };
        if self.turns.len() == self.capacity {
            self.turns.pop_front();
        }
        self.turns.push_back(entry);
        self.next_no += 1;
        turn_no
    }

    /// Number of turns currently buffered (≤ capacity).
    pub fn len(&self) -> usize {
        self.turns.len()
    }

    /// Was anything ever appended?
    /// (Used by rc6 persistence + tests; allow-dead-code so rc5
    /// shipping the journal alone doesn't fail clippy `-D warnings`.)
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.turns.is_empty()
    }

    /// Most recently appended turn, if any.
    pub fn prev(&self) -> Option<&JournalTurn> {
        self.turns.back()
    }

    /// The N-th most recent turn (`offset_back = 0` = most recent).
    /// Returns `None` when `offset_back >= len()`.  Used by rc6
    /// when the rejection signal needs to look further back than
    /// one turn (e.g. correction following adam's reply following
    /// the original question).
    #[allow(dead_code)]
    pub fn nth_back(&self, offset_back: usize) -> Option<&JournalTurn> {
        if offset_back >= self.turns.len() {
            return None;
        }
        let idx = self.turns.len() - 1 - offset_back;
        self.turns.get(idx)
    }

    /// Iterate from oldest to newest.  Used by rc6 when persisting
    /// the final accumulated mistake log on session shutdown.
    #[allow(dead_code)]
    pub fn iter(&self) -> impl Iterator<Item = &JournalTurn> {
        self.turns.iter()
    }
}

impl Default for SessionJournal {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_assigns_monotonic_turn_numbers() {
        let mut j = SessionJournal::with_capacity(3);
        let n1 = j.append("a", "a", None, None, "r1");
        let n2 = j.append("b", "b", None, None, "r2");
        let n3 = j.append("c", "c", None, None, "r3");
        assert_eq!((n1, n2, n3), (1, 2, 3));
        assert_eq!(j.len(), 3);
    }

    #[test]
    fn ring_drops_oldest_when_full() {
        let mut j = SessionJournal::with_capacity(2);
        j.append("a", "a", None, None, "r1");
        j.append("b", "b", None, None, "r2");
        j.append("c", "c", None, None, "r3");
        assert_eq!(j.len(), 2);
        // Oldest survivor is now turn #2, most recent is #3.
        assert_eq!(j.nth_back(0).unwrap().turn_no, 3);
        assert_eq!(j.nth_back(1).unwrap().turn_no, 2);
        assert!(j.nth_back(2).is_none());
    }

    #[test]
    fn prev_returns_most_recent() {
        let mut j = SessionJournal::new();
        assert!(j.prev().is_none());
        j.append(
            "hello",
            "hello",
            Some("Greeting".into()),
            Some(0.95),
            "salem",
        );
        let p = j.prev().expect("prev exists");
        assert_eq!(p.input_raw, "hello");
        assert_eq!(p.intent.as_deref(), Some("Greeting"));
        assert_eq!(p.intent_confidence, Some(0.95));
    }

    #[test]
    fn iter_yields_oldest_first() {
        let mut j = SessionJournal::with_capacity(3);
        j.append("a", "a", None, None, "r1");
        j.append("b", "b", None, None, "r2");
        let inputs: Vec<_> = j.iter().map(|t| t.input_raw.clone()).collect();
        assert_eq!(inputs, vec!["a".to_string(), "b".to_string()]);
    }
}
