//! Lexicon — v1.0.0 loader.
//!
//! The v1.0.0 lexicon is the union of:
//! - `data/tokenizer/segmentation_roots.json` (211 curated by hand + 4243
//!   added via LCP-extractor in commit 366d7a1 = 4454 total)
//! - `data/lexicon_v1/apertium_imported_roots.json` (11,919 entries from
//!   Apertium-kaz; see `data/lexicon_v1/README.md`)
//! - future `data/lexicon_v1/corpus_derived_roots.json` (week 3)
//! - future `data/lexicon_v1/proper_nouns.json` (week 4)
//!
//! This module only needs to load and merge these files into a single
//! [`LexiconV1`] struct. Actual segmentation of words uses the
//! [`morphotactics`] FST, not a flat lookup.
//!
//! [`morphotactics`]: crate::morphotactics

use std::{collections::HashMap, fs, path::Path};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct RootEntry {
    pub id: String,
    pub root: String,
    pub part_of_speech: String,
    pub vowel_harmony: String,
    pub final_sound_class: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RootsFile {
    pub version: String,
    pub name: String,
    pub target_language: String,
    pub script: String,
    pub roots: Vec<RootEntry>,
}

/// Runtime lexicon — keyed lookup by surface form, with source tag.
///
/// **Dual storage (v3.2.0):**
///
/// - `by_surface: HashMap<String, RootEntry>` — O(1) lookup for
///   `get(word)` in the hot path.
/// - `entries_ordered: Vec<RootEntry>` — the same entries, kept in
///   deterministic root-alphabetical order. `parser::analyse` iterates
///   **this** vector so that ambiguous surfaces always yield the same
///   first analysis across runs.
///
/// The v2.1+ reasoning pipeline depends on `parser::analyse` returning
/// stable ordering — otherwise `extract_facts` emits a different fact
/// set every run, invalidating the "deterministic by design" thesis.
/// The dual storage costs a single extra `Vec<RootEntry>` (a few
/// hundred KB on a 16 k-entry Lexicon) but keeps lookup throughput at
/// HashMap level.
///
/// See v3.2.0 CHANGELOG for the latent non-determinism story.
#[derive(Debug, Clone)]
pub struct LexiconV1 {
    /// Surface → entry. First winner on merge is curated > apertium > corpus.
    pub by_surface: HashMap<String, RootEntry>,
    /// Same entries as `by_surface.values()`, sorted by `root`. Iterate
    /// this (not `by_surface.values()`) when determinism matters.
    pub entries_ordered: Vec<RootEntry>,
    pub curated_count: usize,
    pub apertium_count: usize,
}

impl LexiconV1 {
    /// Load the v1.0.0 lexicon from the canonical repository paths.
    pub fn load_default() -> Result<Self, LexiconLoadError> {
        Self::load(
            "data/tokenizer/segmentation_roots.json",
            "data/lexicon_v1/apertium_imported_roots.json",
        )
    }

    pub fn load<P: AsRef<Path>>(
        curated_path: P,
        apertium_path: P,
    ) -> Result<Self, LexiconLoadError> {
        let curated: RootsFile = read_json(curated_path)?;
        let apertium: RootsFile = read_json(apertium_path)?;

        // **v4.4.13** — `by_surface` is intentionally lossy single-POS
        // lookup; downstream consumers that need fast root → entry
        // resolution (and don't care about POS — e.g. spelling /
        // morphology lookups) get one entry per surface. `entries_ordered`
        // below is the **complete** set, no dedup — that's the source
        // of truth the FST analyser iterates over. Pre-v4.4.13 both
        // were lossy because `entries_ordered` was rebuilt from
        // `by_surface.values()`, so multi-POS homonyms like `тау`
        // (verb + noun) silently lost one reading and the FST returned
        // only the wrong POS. Closes the v4.4.12 carry-forward where
        // `тау` parsed only as a verb root despite `noun_apt_tau`
        // existing in the lexicon.
        let mut by_surface: HashMap<String, RootEntry> = HashMap::new();
        for e in &curated.roots {
            by_surface.insert(e.root.clone(), e.clone());
        }
        for e in &apertium.roots {
            by_surface
                .entry(e.root.clone())
                .or_insert_with(|| e.clone());
        }

        // v3.2.0 + v4.4.13 — build a root-alphabetical Vec for
        // deterministic `parser::analyse` iteration. Source is the
        // **full union** of curated + apertium entries, deduplicated
        // only by `id` (catches exact-duplicate copies of the same
        // root entry across both files, e.g. `noun_apt_tau` appearing
        // in both pure_kazakh and apertium). Multiple entries with the
        // same `root` but different `id` (and hence different POS or
        // semantic class) are preserved — the analyser tries each in
        // turn, so multi-POS homonyms produce multi-POS analyses as
        // expected.
        let mut entries_ordered: Vec<RootEntry> = curated
            .roots
            .iter()
            .chain(apertium.roots.iter())
            .cloned()
            .collect();
        entries_ordered.sort_by(|a, b| {
            a.root
                .cmp(&b.root)
                .then_with(|| a.id.cmp(&b.id))
                .then_with(|| a.part_of_speech.cmp(&b.part_of_speech))
        });
        entries_ordered.dedup_by(|a, b| a.id == b.id && a.part_of_speech == b.part_of_speech);

        Ok(Self {
            by_surface,
            entries_ordered,
            curated_count: curated.roots.len(),
            apertium_count: apertium.roots.len(),
        })
    }

    pub fn total(&self) -> usize {
        self.by_surface.len()
    }

    pub fn get(&self, surface: &str) -> Option<&RootEntry> {
        self.by_surface.get(surface)
    }
}

fn read_json<P: AsRef<Path>>(path: P) -> Result<RootsFile, LexiconLoadError> {
    let raw = fs::read_to_string(&path).map_err(|e| LexiconLoadError::Io {
        path: path.as_ref().display().to_string(),
        source: e,
    })?;
    serde_json::from_str(&raw).map_err(|e| LexiconLoadError::Parse {
        path: path.as_ref().display().to_string(),
        source: e,
    })
}

#[derive(Debug, thiserror::Error)]
pub enum LexiconLoadError {
    #[error("io error reading {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("parse error for {path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: serde_json::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_default_succeeds() {
        // This test needs the repository to be checked out. Skip if the
        // lexicon files aren't present (e.g. in an isolated test harness).
        let curated_exists = Path::new("../../data/tokenizer/segmentation_roots.json").exists();
        let apertium_exists =
            Path::new("../../data/lexicon_v1/apertium_imported_roots.json").exists();
        if !curated_exists || !apertium_exists {
            eprintln!("lexicon files not present at expected paths; skipping");
            return;
        }
        let lex = LexiconV1::load(
            "../../data/tokenizer/segmentation_roots.json",
            "../../data/lexicon_v1/apertium_imported_roots.json",
        )
        .expect("load lexicon");
        assert!(
            lex.total() >= 12_000,
            "expected 12k+ entries, got {}",
            lex.total()
        );
        assert!(
            lex.get("бала").is_some(),
            "curated lexicon should contain 'бала'"
        );
    }
}
