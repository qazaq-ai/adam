//! `DialogContext` — multi-turn topic / domain / focus memory.
//!
//! **v4.13.0 — humanness foundation, second half.** Pre-v4.13.0 the
//! conversation tracked only one back-reference: `session
//! ["last_query_topic"]`, the topic of the immediately-prior turn.
//! That was enough for "Ал онда қанша аймақ бар?" anaphora
//! resolution, but it gave adam the memory span of a goldfish: any
//! reference older than one turn was lost. The 2026-05-01 live REPL
//! transcript surfaced the cost — adam couldn't connect "Сіз оны
//! бағдарламалай аласыз ба?" to the Rust topic established four
//! turns earlier, because `last_query_topic` had been overwritten by
//! intervening system-aspect questions.
//!
//! `DialogContext` widens the memory window to the full conversation
//! while staying cheap and predictable: a flat `Vec` of topic
//! mentions plus three derived signals (current focus, current
//! domain, subject under discussion). All updates are O(n) over the
//! history length, no graph traversal at update time. The history is
//! capped at `MAX_HISTORY` entries with FIFO eviction so memory is
//! bounded.
//!
//! **What the user sees:** when the user says «оны / соны / мұны»
//! across multi-turn dialog, adam can resolve the anaphor against
//! the entire conversation, not just the last turn. When the user
//! changes topic abruptly («Алматы туралы айт» after a long
//! programming-domain thread), `current_domain` updates and adam
//! adjusts retrieval scope accordingly.
//!
//! **Zero ML.** All signals are derived deterministically from the
//! topic stream; no model weights, no embeddings, no probabilistic
//! ranking. Future versions may layer a domain-classifier prior
//! (v4.15+) but the v4.13.0 foundation is purely structural.

use serde::{Deserialize, Serialize};

/// One topic mention recorded during a turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicMention {
    /// Sequential turn number when this mention was recorded
    /// (matches `Conversation::turn_counter` after the increment).
    pub turn_id: usize,
    /// Lower-cased root of the topic — e.g. "rust", "алматы".
    pub topic: String,
    /// World-Core domain this topic belongs to, when discoverable
    /// via the runtime fact graph. `None` when no curated entry
    /// matches.
    pub domain: Option<String>,
    /// `true` when this mention came in via anaphora resolution
    /// rather than direct mention. Useful for diagnostics — these
    /// mentions don't strengthen the topic-history signal as much
    /// as direct mentions.
    pub from_anaphora: bool,
}

/// Cap on `topic_history` length. Older entries are FIFO-evicted.
/// 64 entries covers any plausible single-session conversation
/// (~3-turn average per topic) with comfortable headroom.
const MAX_HISTORY: usize = 64;

/// Multi-turn topic / domain / focus memory.
///
/// Lives on `Conversation`. Updated by `Conversation::turn_with_trace`
/// after each turn resolves an `Intent`; consulted on the NEXT turn
/// for anaphora resolution and domain-aware routing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogContext {
    /// Append-only (capped) sequence of every topic mention this
    /// session.
    pub topic_history: Vec<TopicMention>,
    /// **Most-recent** topic — overwritten every turn that has a
    /// concrete topic. Equivalent to the v4.6.0
    /// `session["last_query_topic"]` semantics; preserved here for
    /// callers that want the "what's the immediate referent?" signal
    /// without scanning history.
    pub last_topic: Option<String>,
    /// **Subject under discussion** — the topic that has appeared
    /// most often in the last `STICKY_WINDOW` turns. Stickier than
    /// `last_topic`: a one-off mention does not displace it. This
    /// is what anaphora SHOULD resolve to when the user says «оны»
    /// after several turns of clarification on the same subject.
    pub subject_under_discussion: Option<String>,
    /// Domain inferred from the recent topic stream — majority vote
    /// over the last `DOMAIN_WINDOW` turns. Used by the planner to
    /// scope retrieval / honest-fallback templates («programming
    /// domain» vs «history domain»).
    pub current_domain: Option<String>,
}

/// Window over which `subject_under_discussion` is computed.
const STICKY_WINDOW: usize = 6;
/// Window over which `current_domain` is computed.
const DOMAIN_WINDOW: usize = 4;

impl DialogContext {
    /// Empty context for a fresh conversation.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a turn's resolved topic. Updates `topic_history`,
    /// `last_topic`, `subject_under_discussion`, and `current_domain`
    /// in one pass.
    ///
    /// `domain` is the World-Core domain the topic belongs to, when
    /// discoverable; `None` is fine.
    pub fn record_turn(
        &mut self,
        turn_id: usize,
        topic: &str,
        domain: Option<&str>,
        from_anaphora: bool,
    ) {
        let lower = topic.to_lowercase();
        self.topic_history.push(TopicMention {
            turn_id,
            topic: lower.clone(),
            domain: domain.map(|s| s.to_lowercase()),
            from_anaphora,
        });
        if self.topic_history.len() > MAX_HISTORY {
            // Drop the oldest entries, keep the newest MAX_HISTORY.
            let excess = self.topic_history.len() - MAX_HISTORY;
            self.topic_history.drain(0..excess);
        }
        self.last_topic = Some(lower);
        self.recompute_subject();
        self.recompute_domain();
    }

    /// Resolve a 3rd-person pronoun to the subject most likely
    /// being referred to.
    ///
    /// Strategy (deterministic, no scoring):
    /// 1. Prefer `subject_under_discussion` — the multi-turn-stable
    ///    topic.
    /// 2. Fall back to `last_topic` — the immediate referent.
    /// 3. Return `None` when no context exists.
    pub fn resolve_anaphor(&self) -> Option<&str> {
        self.subject_under_discussion
            .as_deref()
            .or(self.last_topic.as_deref())
    }

    /// `true` when the dialog has accumulated enough context to
    /// confidently route anaphora.
    pub fn has_context(&self) -> bool {
        !self.topic_history.is_empty()
    }

    fn recompute_subject(&mut self) {
        // Count topic occurrences over the last STICKY_WINDOW turns
        // (only direct mentions — anaphora-derived mentions don't
        // strengthen the count). The most-frequent topic wins; ties
        // broken by recency.
        let cutoff = self
            .topic_history
            .last()
            .map(|m| m.turn_id.saturating_sub(STICKY_WINDOW))
            .unwrap_or(0);
        let recent: Vec<&TopicMention> = self
            .topic_history
            .iter()
            .filter(|m| m.turn_id >= cutoff && !m.from_anaphora)
            .collect();
        if recent.is_empty() {
            return;
        }
        // Tally by topic.
        let mut tally: Vec<(String, usize, usize)> = Vec::new(); // (topic, count, last_turn)
        for m in &recent {
            if let Some(entry) = tally.iter_mut().find(|(t, _, _)| t == &m.topic) {
                entry.1 += 1;
                entry.2 = entry.2.max(m.turn_id);
            } else {
                tally.push((m.topic.clone(), 1, m.turn_id));
            }
        }
        tally.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)));
        self.subject_under_discussion = tally.into_iter().next().map(|(t, _, _)| t);
    }

    fn recompute_domain(&mut self) {
        let cutoff = self
            .topic_history
            .last()
            .map(|m| m.turn_id.saturating_sub(DOMAIN_WINDOW))
            .unwrap_or(0);
        let recent_domains: Vec<&str> = self
            .topic_history
            .iter()
            .filter(|m| m.turn_id >= cutoff)
            .filter_map(|m| m.domain.as_deref())
            .collect();
        if recent_domains.is_empty() {
            self.current_domain = None;
            return;
        }
        // Majority vote.
        let mut tally: Vec<(String, usize)> = Vec::new();
        for d in &recent_domains {
            if let Some(entry) = tally.iter_mut().find(|(name, _)| name == d) {
                entry.1 += 1;
            } else {
                tally.push((d.to_string(), 1));
            }
        }
        tally.sort_by(|a, b| b.1.cmp(&a.1));
        self.current_domain = tally.into_iter().next().map(|(d, _)| d);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_context_resolves_to_none() {
        let ctx = DialogContext::new();
        assert!(ctx.resolve_anaphor().is_none());
        assert!(!ctx.has_context());
    }

    #[test]
    fn single_turn_sets_last_topic_and_subject() {
        let mut ctx = DialogContext::new();
        ctx.record_turn(1, "Rust", Some("programming"), false);
        assert_eq!(ctx.last_topic.as_deref(), Some("rust"));
        assert_eq!(ctx.subject_under_discussion.as_deref(), Some("rust"));
        assert_eq!(ctx.current_domain.as_deref(), Some("programming"));
        assert_eq!(ctx.resolve_anaphor(), Some("rust"));
    }

    #[test]
    fn subject_persists_across_one_off_topic_changes() {
        // Establish Rust as the subject across 3 turns.
        let mut ctx = DialogContext::new();
        ctx.record_turn(1, "Rust", Some("programming"), false);
        ctx.record_turn(2, "Rust", Some("programming"), false);
        ctx.record_turn(3, "Rust", Some("programming"), false);
        // One-off mention of cargo.
        ctx.record_turn(4, "cargo", Some("programming"), false);
        // Subject should still be Rust (3 mentions vs 1).
        assert_eq!(ctx.subject_under_discussion.as_deref(), Some("rust"));
        // last_topic flips to cargo, though.
        assert_eq!(ctx.last_topic.as_deref(), Some("cargo"));
    }

    #[test]
    fn domain_majority_vote() {
        let mut ctx = DialogContext::new();
        ctx.record_turn(1, "rust", Some("programming"), false);
        ctx.record_turn(2, "cargo", Some("programming"), false);
        ctx.record_turn(3, "алматы", Some("geography"), false);
        // 2 programming + 1 geography → programming wins.
        assert_eq!(ctx.current_domain.as_deref(), Some("programming"));
    }

    #[test]
    fn domain_shifts_when_subject_changes() {
        let mut ctx = DialogContext::new();
        ctx.record_turn(1, "rust", Some("programming"), false);
        ctx.record_turn(2, "алматы", Some("geography"), false);
        ctx.record_turn(3, "астана", Some("geography"), false);
        ctx.record_turn(4, "шымкент", Some("geography"), false);
        // Last 4 turns: 1 programming + 3 geography → geography wins.
        assert_eq!(ctx.current_domain.as_deref(), Some("geography"));
    }

    #[test]
    fn anaphora_only_mentions_dont_dominate_subject() {
        let mut ctx = DialogContext::new();
        ctx.record_turn(1, "rust", Some("programming"), false);
        // 3 anaphora-derived mentions of "x" — these should NOT
        // make "x" the subject.
        ctx.record_turn(2, "x", Some("misc"), true);
        ctx.record_turn(3, "x", Some("misc"), true);
        ctx.record_turn(4, "x", Some("misc"), true);
        assert_eq!(ctx.subject_under_discussion.as_deref(), Some("rust"));
    }

    #[test]
    fn history_caps_at_max() {
        let mut ctx = DialogContext::new();
        for i in 0..(MAX_HISTORY + 10) {
            ctx.record_turn(i, &format!("t{i}"), None, false);
        }
        assert_eq!(ctx.topic_history.len(), MAX_HISTORY);
        // Oldest entries should be evicted.
        assert!(ctx.topic_history.first().unwrap().turn_id >= 10);
    }

    #[test]
    fn resolve_anaphor_falls_back_to_last_topic_when_no_subject() {
        let mut ctx = DialogContext::new();
        // Single one-off mention.
        ctx.record_turn(1, "z", None, false);
        assert_eq!(ctx.resolve_anaphor(), Some("z"));
    }
}
