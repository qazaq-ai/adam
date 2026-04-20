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
//! v1.6.0 shipped the inverted index itself. Keys are the **root
//! surface strings** emitted by the FST parser — e.g. `балаларды`
//! indexes under `бала`.
//!
//! v1.7.0 adds **ranking**. Given a set of input morphemes, `rank`
//! returns samples scored along four deterministic axes:
//!
//!   - **overlap** (share of the input morphemes present in the
//!     sample) — the main "smart" signal. A sentence that mentions
//!     both `бала` and `мектеп` outranks one that only mentions
//!     `бала` for the input "бала мектепке барды";
//!   - **pack purity prior** — classical literature > curated corpora
//!     > Wikipedia > web crawl. The "safe" signal;
//!   - **length goodness** (Gaussian around 8 words) — rejects
//!     fragments and essays;
//!   - **loanword density** penalty — preserves the native-Kazakh
//!     thesis.
//!
//! No weights are learned; they are editorial constants that a future
//! version can tune. Same input + same index → same top-k.

pub mod compose;

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

    /// v1.7.0: rank candidate samples for the given input morphemes.
    ///
    /// Returns up to `config.top_k` hits sorted by descending score.
    /// Ties (two samples scoring identically) break by
    /// `(pack, sample_id)` order so the output is fully deterministic.
    /// Samples must have a stored text (see [`remember_text`](Self::remember_text))
    /// to be eligible — this keeps the result set aligned with what the
    /// dialog layer can actually quote.
    pub fn rank(&self, input_morphemes: &[&str], config: &RankConfig) -> Vec<Hit> {
        if input_morphemes.is_empty() {
            return Vec::new();
        }
        // candidate set = union of postings for every input morpheme.
        // Along the way, count how many distinct input morphemes each
        // sample covers (the "overlap" signal).
        let mut overlap: HashMap<SampleRef, usize> = HashMap::new();
        let mut distinct_input: HashSet<&str> = HashSet::new();
        for m in input_morphemes {
            if !distinct_input.insert(*m) {
                continue;
            }
            if let Some(refs) = self.postings.get(*m) {
                for sref in refs {
                    *overlap.entry(sref.clone()).or_insert(0) += 1;
                }
            }
        }
        let n_input = distinct_input.len().max(1) as f32;

        let mut hits: Vec<Hit> = overlap
            .into_iter()
            .filter_map(|(sref, overlap_count)| {
                let text = self.sample_text(&sref)?;
                let word_count = text.split_whitespace().count();
                let length = length_goodness(word_count);
                let loanword = sample_loanword_density(text);
                let purity = config
                    .pack_purity
                    .get(&sref.pack)
                    .copied()
                    .unwrap_or(DEFAULT_UNKNOWN_PACK_PURITY);
                let overlap_ratio = overlap_count as f32 / n_input;
                let score = config.weight_overlap * overlap_ratio
                    + config.weight_purity * purity
                    + config.weight_length * length
                    - config.weight_loanword_penalty * loanword;
                Some(Hit {
                    sref,
                    score,
                    overlap_count,
                    overlap_ratio,
                    length_goodness: length,
                    loanword_density: loanword,
                    pack_purity: purity,
                })
            })
            .collect();

        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.sref.cmp(&b.sref))
        });
        hits.truncate(config.top_k);
        hits
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

/// A single ranked retrieval result.
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    /// Pointer to the underlying sample in the corpus.
    pub sref: SampleRef,
    /// Composite score (higher = better).
    pub score: f32,
    /// Number of distinct input morphemes matched by this sample.
    pub overlap_count: usize,
    /// `overlap_count / distinct_input_morphemes` — the `overlap_ratio`
    /// component of the score, exposed for tracing.
    pub overlap_ratio: f32,
    /// Gaussian length score around 8 words, exposed for tracing.
    pub length_goodness: f32,
    /// Fraction of sample words flagged as loanwords, exposed for tracing.
    pub loanword_density: f32,
    /// Editorial purity prior for the sample's pack, exposed for tracing.
    pub pack_purity: f32,
}

/// Configuration for [`MorphemeIndex::rank`]. The weights and pack
/// priors are editorial constants (not learned) chosen to encode the
/// v1.7.0 hypothesis: overlap is the main "smart" signal, pack purity
/// is the main "safe" signal, length and loanword density are polish.
#[derive(Debug, Clone)]
pub struct RankConfig {
    pub top_k: usize,
    /// Weight on `overlap_count / n_input_morphemes` (0..1).
    pub weight_overlap: f32,
    /// Weight on the pack's editorial purity prior (0..1).
    pub weight_purity: f32,
    /// Weight on `length_goodness(word_count)` (0..1).
    pub weight_length: f32,
    /// Coefficient on the loanword-density *penalty* (subtracted).
    pub weight_loanword_penalty: f32,
    /// Editorial purity prior per pack file name. Unknown packs fall
    /// back to [`DEFAULT_UNKNOWN_PACK_PURITY`].
    pub pack_purity: BTreeMap<String, f32>,
}

/// Default purity assumed for a pack not listed in `RankConfig::pack_purity`.
/// Conservative middle value — leans toward "probably OK" without
/// endorsing the pack as literary.
pub const DEFAULT_UNKNOWN_PACK_PURITY: f32 = 0.70;

impl Default for RankConfig {
    fn default() -> Self {
        let mut pack_purity = BTreeMap::new();
        // Classical literature — curated for centuries.
        pack_purity.insert("kazakh_classics_pack.json".into(), 1.00);
        pack_purity.insert("abai_wikisource_pack.json".into(), 1.00);
        // Proverbs — distilled native Kazakh by definition.
        pack_purity.insert("kazakh_proverbs_pack.json".into(), 1.00);
        // Synthetic — uses Lexicon exclusively; zero loanword exposure.
        pack_purity.insert("synthetic_sentences_pack.json".into(), 0.95);
        // Human-translated parallel data — mostly clean.
        pack_purity.insert("tatoeba_kazakh_pack.json".into(), 0.95);
        // Read-aloud sentences — clean by selection.
        pack_purity.insert("common_voice_kk_pack.json".into(), 0.95);
        // Wikipedia — edited, but loanwords in technical domains.
        pack_purity.insert("wikipedia_kz_pack.json".into(), 0.85);
        // Web crawl — unfiltered language, the weakest source.
        pack_purity.insert("cc100_kk_pack.json".into(), 0.75);
        Self {
            top_k: 5,
            weight_overlap: 0.40,
            weight_purity: 0.30,
            weight_length: 0.15,
            weight_loanword_penalty: 0.15,
            pack_purity,
        }
    }
}

/// Gaussian centered at 8 words with σ = 6. Peaks at 1.0, falls
/// below 0.2 around 15+ words or 0-2 words.
pub fn length_goodness(word_count: usize) -> f32 {
    const TARGET: f32 = 8.0;
    const SIGMA: f32 = 6.0;
    let x = word_count as f32;
    let z = (x - TARGET) / SIGMA;
    (-z * z).exp()
}

/// Fraction of whitespace-split tokens in `text` that look like loanwords
/// by the v1.x purity rules (Russian-only letter OR a stock loanword
/// suffix of ≥ 3 chars longer than the suffix itself). Returns 0.0 on
/// empty input.
pub fn sample_loanword_density(text: &str) -> f32 {
    const RUSSIAN_ONLY: &[char] = &['ё', 'ф', 'ц', 'ч', 'щ', 'ъ', 'ь', 'э'];
    const LOANWORD_SUFFIXES: &[&str] = &[
        "ция",
        "логия",
        "графия",
        "тика",
        "изм",
        "ивный",
        "ильный",
        "альный",
        "альная",
        "альное",
        "ональный",
    ];
    fn is_loanword(word: &str) -> bool {
        let cleaned: String = word
            .chars()
            .filter(|c| c.is_alphabetic() || *c == '-')
            .collect::<String>()
            .to_lowercase();
        if cleaned.chars().any(|c| RUSSIAN_ONLY.contains(&c)) {
            return true;
        }
        LOANWORD_SUFFIXES
            .iter()
            .any(|s| cleaned.ends_with(s) && cleaned.chars().count() > s.chars().count() + 1)
    }
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return 0.0;
    }
    let flagged = words.iter().filter(|w| is_loanword(w)).count();
    flagged as f32 / words.len() as f32
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

    fn seed_rank_index() -> MorphemeIndex {
        let mut idx = MorphemeIndex::new();
        // Three samples, deliberately crafted so each test can control
        // exactly which score component wins.
        let a = SampleRef {
            pack: "abai_wikisource_pack.json".into(),
            sample_id: "abai_001".into(),
        };
        let b = SampleRef {
            pack: "cc100_kk_pack.json".into(),
            sample_id: "cc100_001".into(),
        };
        let c = SampleRef {
            pack: "abai_wikisource_pack.json".into(),
            sample_id: "abai_002".into(),
        };
        // Sample A — short Abai fragment, only matches "бала"
        idx.insert("бала", a.clone());
        idx.remember_text(&a, "Бала ән салды");
        // Sample B — CC-100 matches both "бала" and "мектеп" (overlap 2)
        idx.insert("бала", b.clone());
        idx.insert("мектеп", b.clone());
        idx.remember_text(
            &b,
            "Бала мектепке барды және мұғалімнің сабағын тыңдады бүгін",
        );
        // Sample C — Abai, matches "мектеп" only, length 8 words
        idx.insert("мектеп", c.clone());
        idx.remember_text(&c, "Мектеп — білім бастауы, балаға жол нұсқайды");
        idx.refresh_stats();
        idx
    }

    #[test]
    fn rank_prefers_higher_overlap() {
        let idx = seed_rank_index();
        let config = RankConfig::default();
        let hits = idx.rank(&["бала", "мектеп"], &config);
        assert_eq!(hits.len(), 3);
        // B matches both, overlap_count = 2
        assert_eq!(hits[0].overlap_count, 2);
        assert_eq!(hits[0].sref.sample_id, "cc100_001");
    }

    #[test]
    fn rank_breaks_ties_with_pack_purity() {
        // Two samples both matching "мектеп" — A is Abai, B is CC-100.
        // With equal overlap = 1, Abai should win on purity.
        let mut idx = MorphemeIndex::new();
        let abai = SampleRef {
            pack: "abai_wikisource_pack.json".into(),
            sample_id: "abai_001".into(),
        };
        let cc = SampleRef {
            pack: "cc100_kk_pack.json".into(),
            sample_id: "cc100_001".into(),
        };
        idx.insert("мектеп", abai.clone());
        idx.insert("мектеп", cc.clone());
        idx.remember_text(&abai, "Мектеп балаға бар білімнің бастауы");
        idx.remember_text(&cc, "Мектеп балаға бар білімнің бастауы");
        let config = RankConfig::default();
        let hits = idx.rank(&["мектеп"], &config);
        assert_eq!(hits[0].sref.pack, "abai_wikisource_pack.json");
    }

    #[test]
    fn rank_penalises_loanword_heavy_sample() {
        let mut idx = MorphemeIndex::new();
        let clean = SampleRef {
            pack: "abai_wikisource_pack.json".into(),
            sample_id: "abai_001".into(),
        };
        let loany = SampleRef {
            pack: "abai_wikisource_pack.json".into(),
            sample_id: "abai_002".into(),
        };
        idx.insert("бала", clean.clone());
        idx.insert("бала", loany.clone());
        idx.remember_text(&clean, "Бала туралы жақсы әңгіме айтты");
        // Three loanword-suffix tokens out of five → high density.
        idx.remember_text(&loany, "Бала конституция ұйымының комиссиясына бармады");
        let config = RankConfig::default();
        let hits = idx.rank(&["бала"], &config);
        assert_eq!(hits[0].sref.sample_id, "abai_001");
        assert!(hits[0].loanword_density < hits[1].loanword_density);
    }

    #[test]
    fn length_goodness_peaks_at_8_words() {
        let g8 = length_goodness(8);
        assert!((g8 - 1.0).abs() < 1e-5);
        assert!(length_goodness(2) < 0.7);
        assert!(length_goodness(30) < 0.01);
    }

    #[test]
    fn sample_loanword_density_flags_russian_only_letters() {
        // `ёж` has Russian-only `ё`; `биология` matches suffix `логия`.
        // `бала` and `бардык` are native → 2/4 = 0.50.
        let t = "бала ёж биология бардык";
        let d = sample_loanword_density(t);
        assert!(d > 0.4 && d < 0.6, "density should catch ё + -логия: {d}");
    }

    #[test]
    fn rank_top_k_is_respected() {
        let idx = seed_rank_index();
        let mut config = RankConfig::default();
        config.top_k = 2;
        let hits = idx.rank(&["бала", "мектеп"], &config);
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn rank_empty_input_returns_empty() {
        let idx = seed_rank_index();
        assert!(idx.rank(&[], &RankConfig::default()).is_empty());
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
