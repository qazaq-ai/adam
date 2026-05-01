//! `DomainIndex` — topic → World-Core domain inverted index.
//!
//! **v4.14.0 — domain-awareness foundation.** Pre-v4.14.0
//! `DialogContext.current_domain` was always `None`: there was no
//! way to map a topic noun (e.g. `жасуша`) to its World-Core domain
//! (`biology_school`). This module builds the inverted index once at
//! conversation startup and exposes O(1) lookup. Combined with
//! `DialogContext.subject_under_discussion` (v4.13.0), it lets the
//! planner reason about *what subject area* the conversation is in,
//! not just *which noun* was last mentioned.
//!
//! **Algorithm.** Walk every World-Core fact, collect every subject
//! and object root, and tally `(root, domain)` pairs. For each root,
//! the **primary domain** is the domain with the most fact-mentions
//! (ties broken alphabetically for determinism). Lookup returns the
//! primary domain or `None` if the root never appears in the curated
//! graph.
//!
//! **Why not TF-IDF?** Pure count-based ranking is enough at the
//! v4.14.0 scale (38 domains, ~1 800 facts). TF-IDF would weight
//! domain-uniqueness, but with curated facts almost every root maps
//! to a single dominant domain anyway. If the index ever needs to
//! disambiguate cross-domain roots (e.g. `тіл` = biology body-part
//! AND linguistics subject), TF-IDF can be layered later.
//!
//! **Cost.** Build is O(n) over fact count; lookup is O(1). Memory:
//! ~50 KB for the current world_core (1 791 facts × ~30 byte avg
//! root). Negligible.
//!
//! **Zero ML.** Pure deterministic count over curated facts.

use std::collections::HashMap;

use adam_reasoning::world_core::WorldCoreEntry;

/// Topic → primary domain index.
///
/// The map is `topic_root.to_lowercase() → primary_domain`. Built
/// once at conversation init and frozen for the lifetime of the
/// Conversation; no runtime mutation.
#[derive(Debug, Clone, Default)]
pub struct DomainIndex {
    by_topic: HashMap<String, String>,
}

impl DomainIndex {
    /// Empty index — `lookup_domain` always returns `None`. Used
    /// when no curated facts are attached (e.g. tests that don't
    /// load world_core).
    pub fn empty() -> Self {
        Self::default()
    }

    /// Build the index from a slice of curated facts. Each `Fact`
    /// is expected to carry a non-empty `domain` field; facts with
    /// empty domain are skipped (they wouldn't contribute a useful
    /// signal anyway).
    ///
    /// Deterministic: same input slice produces the same map.
    /// Tie-break (when a topic has equal counts in 2+ domains) is
    /// alphabetical on the domain name.
    pub fn build(entries: &[WorldCoreEntry]) -> Self {
        // First pass: per-topic per-domain counts. Each
        // `WorldCoreEntry` carries one `domain` string and a list of
        // `WorldCoreFact { subject, predicate, object }`. Both the
        // subject and object roots contribute to the index for the
        // entry's domain.
        let mut tally: HashMap<String, HashMap<String, usize>> = HashMap::new();
        for entry in entries {
            let domain = entry.domain.trim();
            if domain.is_empty() {
                continue;
            }
            for fact in &entry.facts {
                for root in [&fact.subject, &fact.object] {
                    let key = root.to_lowercase();
                    if key.is_empty() {
                        continue;
                    }
                    *tally
                        .entry(key)
                        .or_default()
                        .entry(domain.to_string())
                        .or_insert(0) += 1;
                }
            }
        }

        // Second pass: pick the primary domain per topic.
        let mut by_topic = HashMap::with_capacity(tally.len());
        for (topic, domain_counts) in tally {
            // Sort by (descending count, ascending domain name) for
            // a fully-deterministic primary pick.
            let mut entries: Vec<(String, usize)> = domain_counts.into_iter().collect();
            entries.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            if let Some((primary, _)) = entries.into_iter().next() {
                by_topic.insert(topic, primary);
            }
        }
        Self { by_topic }
    }

    /// Number of indexed topics. Mostly for diagnostic / test use.
    pub fn len(&self) -> usize {
        self.by_topic.len()
    }

    /// `true` if no topics are indexed.
    pub fn is_empty(&self) -> bool {
        self.by_topic.is_empty()
    }

    /// Look up the primary domain for a topic root. The lookup is
    /// case-insensitive (the input is lower-cased before query).
    /// Returns `None` when the topic is absent from the curated
    /// graph.
    pub fn lookup_domain(&self, topic: &str) -> Option<&str> {
        let key = topic.to_lowercase();
        self.by_topic.get(&key).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adam_reasoning::Predicate;
    use adam_reasoning::world_core::{ConfidenceTier, ReviewStatus, WorldCoreFact};

    fn entry(domain: &str, facts: Vec<(&str, Predicate, &str)>) -> WorldCoreEntry {
        WorldCoreEntry {
            id: format!("{domain}_001"),
            kk: String::new(),
            facts: facts
                .into_iter()
                .map(|(s, p, o)| WorldCoreFact {
                    subject: s.to_string(),
                    predicate: p,
                    object: o.to_string(),
                })
                .collect(),
            domain: domain.to_string(),
            source: "curated".to_string(),
            confidence: ConfidenceTier::High,
            review_status: ReviewStatus::Approved,
            reviewer: "test".to_string(),
            reviewed_at: "2026-05-01".to_string(),
        }
    }

    #[test]
    fn empty_index_returns_none() {
        let idx = DomainIndex::empty();
        assert!(idx.lookup_domain("rust").is_none());
        assert!(idx.is_empty());
    }

    #[test]
    fn single_topic_maps_to_its_domain() {
        let entries = vec![entry(
            "biology_school",
            vec![("жасуша", Predicate::IsA, "тіршілік")],
        )];
        let idx = DomainIndex::build(&entries);
        assert_eq!(idx.lookup_domain("жасуша"), Some("biology_school"));
    }

    #[test]
    fn case_insensitive_lookup() {
        let entries = vec![entry(
            "programming_rust",
            vec![("Rust", Predicate::IsA, "тіл")],
        )];
        let idx = DomainIndex::build(&entries);
        assert_eq!(idx.lookup_domain("rust"), Some("programming_rust"));
        assert_eq!(idx.lookup_domain("RUST"), Some("programming_rust"));
        assert_eq!(idx.lookup_domain("Rust"), Some("programming_rust"));
    }

    #[test]
    fn primary_domain_is_majority_winner() {
        let entries = vec![
            entry(
                "biology_school",
                vec![
                    ("жасуша", Predicate::IsA, "тіршілік"),
                    ("жасуша", Predicate::Has, "ядро"),
                    ("жасуша", Predicate::Has, "цитоплазма"),
                ],
            ),
            entry(
                "chemistry_school",
                vec![("жасуша", Predicate::Has, "ақуыз")],
            ),
        ];
        let idx = DomainIndex::build(&entries);
        assert_eq!(idx.lookup_domain("жасуша"), Some("biology_school"));
    }

    #[test]
    fn ties_broken_alphabetically_for_determinism() {
        // Tie: `тіл` appears 1x in each domain.
        // Alphabetical: biology_school < linguistics.
        let entries = vec![
            entry("biology_school", vec![("тіл", Predicate::PartOf, "адам")]),
            entry("linguistics", vec![("тіл", Predicate::IsA, "коммуникация")]),
        ];
        let idx = DomainIndex::build(&entries);
        assert_eq!(idx.lookup_domain("тіл"), Some("biology_school"));
    }

    #[test]
    fn empty_domain_facts_skipped() {
        let entries = vec![
            entry("", vec![("rust", Predicate::IsA, "тіл")]),
            entry("programming_rust", vec![("rust", Predicate::IsA, "тіл")]),
        ];
        let idx = DomainIndex::build(&entries);
        assert_eq!(idx.lookup_domain("rust"), Some("programming_rust"));
    }

    #[test]
    fn unknown_topic_returns_none() {
        let entries = vec![entry(
            "programming_rust",
            vec![("rust", Predicate::IsA, "тіл")],
        )];
        let idx = DomainIndex::build(&entries);
        assert!(idx.lookup_domain("неизвестно").is_none());
    }

    #[test]
    fn objects_also_contribute_to_index() {
        let entries = vec![entry(
            "biology_school",
            vec![("жасуша", Predicate::Has, "ядро")],
        )];
        let idx = DomainIndex::build(&entries);
        assert_eq!(idx.lookup_domain("ядро"), Some("biology_school"));
    }
}
