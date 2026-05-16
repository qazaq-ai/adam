//! Vocab — id assignment for morpheme tokens.
//!
//! The vocab is **derived** from the [`LexiconV1`] at tokenizer
//! construction time; it is **not** persisted yet (that's a Phase 1
//! concern when we serialise embedding tables). For Phase 0 we just
//! need stable ids so unit-tests of the future neural decoder can
//! reference tokens by integer.
//!
//! ID layout:
//!
//! ```text
//!  0      — Pad
//!  1      — Bos
//!  2      — Eos
//!  3      — Space
//!  4      — Unk
//!  5-24   — Punct (20 reserved KZ punctuation chars)
//!  25-99  — Suffix tokens (one per SuffixKind variant; sparse)
//! 100-N   — Root tokens (one per Lexicon entries_ordered slot)
//! ```

use std::collections::HashMap;

use adam_kernel_fst::lexicon::LexiconV1;
use thiserror::Error;

pub const ID_PAD: u32 = 0;
pub const ID_BOS: u32 = 1;
pub const ID_EOS: u32 = 2;
pub const ID_SPACE: u32 = 3;
pub const ID_UNK: u32 = 4;
pub const PUNCT_RANGE_START: u32 = 5;
pub const PUNCT_RANGE_END: u32 = 24;
pub const SUFFIX_RANGE_START: u32 = 25;
pub const SUFFIX_RANGE_END: u32 = 99;
pub const ROOT_RANGE_START: u32 = 100;

/// Stable id mapping from Lexicon root identifier (the `RootEntry::id`
/// string) to a contiguous integer slot starting at [`ROOT_RANGE_START`].
#[derive(Debug, Clone)]
pub struct Vocab {
    root_ids: HashMap<String, u32>,
    root_count: u32,
}

impl Vocab {
    /// Build vocab from a loaded lexicon. Assigns root ids in the order
    /// the lexicon's `entries_ordered` vector lists them — which is
    /// deterministic per the v3.2.0 determinism contract.
    pub fn from_lexicon(lex: &LexiconV1) -> Self {
        let mut root_ids = HashMap::with_capacity(lex.entries_ordered.len());
        let mut next_id = ROOT_RANGE_START;
        for entry in &lex.entries_ordered {
            root_ids.entry(entry.id.clone()).or_insert_with(|| {
                let id = next_id;
                next_id += 1;
                id
            });
        }
        Self {
            root_ids,
            root_count: next_id - ROOT_RANGE_START,
        }
    }

    /// Look up a root token id by its Lexicon `RootEntry::id`.
    pub fn root_id(&self, root_entry_id: &str) -> Option<u32> {
        self.root_ids.get(root_entry_id).copied()
    }

    /// Total number of root tokens in the vocab.
    pub fn root_count(&self) -> u32 {
        self.root_count
    }

    /// Total vocab size (service + punct + suffix slots + roots).
    pub fn size(&self) -> u32 {
        ROOT_RANGE_START + self.root_count
    }
}

#[derive(Debug, Error)]
pub enum VocabBuildError {
    #[error("lexicon has duplicate root entry ids; cannot build deterministic vocab")]
    DuplicateRootId,
}

#[cfg(test)]
mod tests {
    use super::*;
    use adam_kernel_fst::lexicon::{LexiconV1, RootEntry};

    fn mini_lex() -> LexiconV1 {
        let entries = vec![
            RootEntry {
                id: "a".into(),
                root: "а".into(),
                part_of_speech: "noun".into(),
                vowel_harmony: "back".into(),
                final_sound_class: "vowel".into(),
            },
            RootEntry {
                id: "b".into(),
                root: "б".into(),
                part_of_speech: "noun".into(),
                vowel_harmony: "back".into(),
                final_sound_class: "consonant".into(),
            },
        ];
        let mut by_surface = HashMap::new();
        for e in &entries {
            by_surface.insert(e.root.clone(), e.clone());
        }
        let curated_count = entries.len();
        LexiconV1 {
            by_surface,
            entries_ordered: entries,
            curated_count,
            apertium_count: 0,
        }
    }

    #[test]
    fn root_ids_are_contiguous_from_root_range_start() {
        let v = Vocab::from_lexicon(&mini_lex());
        assert_eq!(v.root_id("a"), Some(ROOT_RANGE_START));
        assert_eq!(v.root_id("b"), Some(ROOT_RANGE_START + 1));
        assert_eq!(v.root_count(), 2);
        assert_eq!(v.size(), ROOT_RANGE_START + 2);
    }

    #[test]
    fn unknown_root_returns_none() {
        let v = Vocab::from_lexicon(&mini_lex());
        assert!(v.root_id("nonexistent").is_none());
    }

    // Compile-time invariants about layout. If any of these breaks,
    // the crate stops compiling — stronger than runtime assertion.
    const _: () = {
        assert!(PUNCT_RANGE_START > ID_UNK);
        assert!(SUFFIX_RANGE_START > PUNCT_RANGE_END);
        assert!(ROOT_RANGE_START > SUFFIX_RANGE_END);
    };
}
