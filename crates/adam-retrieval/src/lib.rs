//! adam-retrieval — morpheme-indexed retrieval over a committed Kazakh corpus.
//!
//! Stage: v1.6.0 bootstrap.
//!
//! This crate is the first rung of the v2.0 retrieval engine. Unlike a
//! probabilistic LM, retrieval is:
//!
//!   - **deterministic** — given a morpheme bag and an index file, the top-k
//!     result set is fully determined;
//!   - **traceable** — every hit names the pack + sample id it came from,
//!     so we can always show "this response is the sentence at
//!     `wikipedia_kz_pack.json[42]`";
//!   - **cheap** — a hash lookup plus a sorted-list intersection, not a
//!     matmul.
//!
//! The v1.6.0 scope is the inverted index itself. Keys are the **root
//! surface strings** emitted by the FST parser — e.g. `балаларды`
//! indexes under `бала`. Future versions (v1.7.0+) may add suffix
//! features, KNN re-ranking, and eventually a compositional synthesiser
//! wired to the `Intent::Unknown` fallback.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

/// Separator used to flatten `(pack, sample_id)` into the string key
/// used by [`MorphemeIndex::sample_texts`]. Neither pack file names
/// nor sample ids contain `::` in the current data, so this is safe
/// and keeps the JSON form flat and diffable.
const TEXT_KEY_SEP: &str = "::";

/// A pointer to one sample in one committed pack.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SampleRef {
    /// File name of the pack (e.g. `"wikipedia_kz_pack.json"`).
    pub pack: String,
    /// Stable id from the pack's `samples[].id` field.
    pub sample_id: String,
}

impl SampleRef {
    /// Composite key for [`MorphemeIndex::sample_texts`].
    pub fn text_key(&self) -> String {
        format!("{}{}{}", self.pack, TEXT_KEY_SEP, self.sample_id)
    }
}

/// Morpheme → sorted postings-list mapping.
///
/// `BTreeMap` (instead of `HashMap`) so the on-disk JSON form is
/// deterministic — the same input always serialises to byte-identical
/// output, which makes `git diff` of committed index files meaningful
/// and lets CI verify index regeneration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MorphemeIndex {
    /// Pack file names the index was built from, in the order encountered.
    pub built_from: Vec<String>,
    /// Total number of sample references ingested across all morphemes.
    pub total_postings: usize,
    /// Number of distinct morpheme keys.
    pub unique_morphemes: usize,
    /// Number of samples that contributed at least one morpheme.
    pub samples_indexed: usize,
    /// morpheme → sorted unique list of sample refs containing a word
    /// whose FST analysis yielded this morpheme as its root.
    pub postings: BTreeMap<String, Vec<SampleRef>>,
    /// `SampleRef::text_key()` → original sample text. Populated by the
    /// index builder so downstream consumers (v1.6.5 dialog integration)
    /// can cite the actual sentence without round-tripping through the
    /// source packs. Default: empty for older indices that pre-date this
    /// field; [`sample_text`](Self::sample_text) returns `None`.
    #[serde(default)]
    pub sample_texts: BTreeMap<String, String>,
}

impl MorphemeIndex {
    /// Empty index — nothing ingested yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that `sref`'s sample contains a word whose FST root is
    /// `morpheme`. Idempotent: inserting the same (morpheme, sref) twice
    /// leaves the index unchanged.
    pub fn insert(&mut self, morpheme: impl Into<String>, sref: SampleRef) {
        let morpheme = morpheme.into();
        let entry = self.postings.entry(morpheme).or_default();
        if let Err(pos) = entry.binary_search(&sref) {
            entry.insert(pos, sref);
            self.total_postings += 1;
        }
    }

    /// All samples indexed under `morpheme`. Returns an empty slice if
    /// the morpheme is not in the index.
    pub fn search(&self, morpheme: &str) -> &[SampleRef] {
        self.postings
            .get(morpheme)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// AND-search: samples that appear in the postings lists of ALL the
    /// given morphemes. Empty input returns empty. Unknown morphemes
    /// collapse the result to empty. Output is sorted (pack, sample_id).
    pub fn search_conjunction(&self, morphemes: &[&str]) -> Vec<SampleRef> {
        if morphemes.is_empty() {
            return Vec::new();
        }
        // Start from the shortest postings list for a cheap intersection.
        let mut sorted_keys: Vec<&&str> = morphemes.iter().collect();
        sorted_keys.sort_by_key(|m| self.postings.get(**m).map(|v| v.len()).unwrap_or(0));
        let first = match self.postings.get(*sorted_keys[0]) {
            Some(v) => v,
            None => return Vec::new(),
        };
        let rest: Vec<&Vec<SampleRef>> = sorted_keys[1..]
            .iter()
            .map(|m| self.postings.get(**m))
            .collect::<Option<Vec<_>>>()
            .unwrap_or_default();
        if rest.len() != sorted_keys.len() - 1 {
            return Vec::new();
        }
        let rest_sets: Vec<HashSet<&SampleRef>> = rest.iter().map(|v| v.iter().collect()).collect();
        first
            .iter()
            .filter(|s| rest_sets.iter().all(|set| set.contains(s)))
            .cloned()
            .collect()
    }

    /// Record the original text of `sref`'s sample. Idempotent —
    /// re-inserting the same text is a no-op. Last write wins if two
    /// calls disagree on the text, which shouldn't happen in practice
    /// (sample ids are stable per pack).
    pub fn remember_text(&mut self, sref: &SampleRef, text: impl Into<String>) {
        self.sample_texts.insert(sref.text_key(), text.into());
    }

    /// Original text of the sample, if the index was built with texts.
    /// Returns `None` for indices predating v1.6.5 or for refs whose
    /// text was not included.
    pub fn sample_text(&self, sref: &SampleRef) -> Option<&str> {
        self.sample_texts.get(&sref.text_key()).map(|s| s.as_str())
    }

    /// Refresh the derived counts after direct mutation of `postings`
    /// (e.g. after bulk-loading from JSON).
    pub fn refresh_stats(&mut self) {
        self.total_postings = self.postings.values().map(|v| v.len()).sum();
        self.unique_morphemes = self.postings.len();
        // sample count: unique SampleRefs across all postings lists
        let mut seen: HashMap<(String, String), ()> = HashMap::new();
        for refs in self.postings.values() {
            for s in refs {
                seen.insert((s.pack.clone(), s.sample_id.clone()), ());
            }
        }
        self.samples_indexed = seen.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sref(pack: &str, id: &str) -> SampleRef {
        SampleRef {
            pack: pack.into(),
            sample_id: id.into(),
        }
    }

    #[test]
    fn insert_is_idempotent() {
        let mut idx = MorphemeIndex::new();
        idx.insert("бала", sref("pack_a", "id_1"));
        idx.insert("бала", sref("pack_a", "id_1"));
        assert_eq!(idx.search("бала").len(), 1);
        assert_eq!(idx.total_postings, 1);
    }

    #[test]
    fn insert_keeps_postings_sorted() {
        let mut idx = MorphemeIndex::new();
        idx.insert("бала", sref("pack_b", "id_1"));
        idx.insert("бала", sref("pack_a", "id_9"));
        idx.insert("бала", sref("pack_a", "id_2"));
        let found = idx.search("бала");
        assert_eq!(found.len(), 3);
        for w in found.windows(2) {
            assert!(w[0] < w[1], "postings must be sorted: {w:?}");
        }
    }

    #[test]
    fn search_unknown_morpheme_returns_empty() {
        let idx = MorphemeIndex::new();
        assert!(idx.search("нет-такого").is_empty());
    }

    #[test]
    fn conjunction_finds_common_sample() {
        let mut idx = MorphemeIndex::new();
        idx.insert("бала", sref("pack_a", "id_1"));
        idx.insert("бала", sref("pack_a", "id_2"));
        idx.insert("үй", sref("pack_a", "id_2"));
        idx.insert("үй", sref("pack_a", "id_3"));
        let hits = idx.search_conjunction(&["бала", "үй"]);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].sample_id, "id_2");
    }

    #[test]
    fn conjunction_with_unknown_morpheme_is_empty() {
        let mut idx = MorphemeIndex::new();
        idx.insert("бала", sref("pack_a", "id_1"));
        assert!(idx.search_conjunction(&["бала", "нет-такого"]).is_empty());
    }

    #[test]
    fn conjunction_empty_input_is_empty() {
        let mut idx = MorphemeIndex::new();
        idx.insert("бала", sref("pack_a", "id_1"));
        assert!(idx.search_conjunction(&[]).is_empty());
    }

    #[test]
    fn remember_and_retrieve_text() {
        let mut idx = MorphemeIndex::new();
        let s = sref("pack_a", "id_1");
        idx.insert("бала", s.clone());
        idx.remember_text(&s, "бала кітап оқиды");
        assert_eq!(idx.sample_text(&s), Some("бала кітап оқиды"));
    }

    #[test]
    fn sample_text_returns_none_when_absent() {
        let mut idx = MorphemeIndex::new();
        let s = sref("pack_a", "id_1");
        idx.insert("бала", s.clone());
        assert!(idx.sample_text(&s).is_none());
    }

    #[test]
    fn text_key_is_pack_and_id_joined() {
        let s = sref("wikipedia_kz_pack.json", "wiki_kz_0000001");
        assert_eq!(s.text_key(), "wikipedia_kz_pack.json::wiki_kz_0000001");
    }

    #[test]
    fn refresh_stats_after_bulk_load() {
        let mut idx = MorphemeIndex::new();
        idx.postings.insert(
            "бала".into(),
            vec![sref("pack_a", "id_1"), sref("pack_a", "id_2")],
        );
        idx.postings
            .insert("үй".into(), vec![sref("pack_a", "id_2")]);
        idx.refresh_stats();
        assert_eq!(idx.unique_morphemes, 2);
        assert_eq!(idx.total_postings, 3);
        assert_eq!(idx.samples_indexed, 2); // id_1 + id_2, id_2 only counted once
    }
}
