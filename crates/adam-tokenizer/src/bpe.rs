use std::{collections::HashMap, fs};

use adam_kernel::{SegmentationLexicon, SegmentationRuleSet};
use serde::Deserialize;

use crate::pretokenize;

#[derive(Debug, Deserialize)]
struct VocabEntry {
    id: u32,
    token: String,
}

#[derive(Debug, Deserialize)]
struct VocabFile {
    vocab: Vec<VocabEntry>,
    #[serde(default)]
    #[allow(dead_code)]
    special_tokens: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct MergeRecord {
    #[allow(dead_code)]
    rank: usize,
    left: String,
    right: String,
    #[allow(dead_code)]
    merged: String,
    #[allow(dead_code)]
    frequency: usize,
}

#[derive(Debug, Deserialize)]
struct MergesFile {
    merges: Vec<MergeRecord>,
}

/// BPE tokenizer that wraps the morpheme-aware pre-tokenizer and applies
/// learned merges to produce token IDs.
pub struct BpeTokenizer {
    token_to_id: HashMap<String, u32>,
    id_to_token: Vec<String>,
    /// (left, right) -> rank. Lower rank means earlier-learned (higher priority) merge.
    merge_ranks: HashMap<(String, String), usize>,
    pub pad_id: u32,
    pub bos_id: u32,
    pub eos_id: u32,
    pub unk_id: u32,
}

impl BpeTokenizer {
    pub fn load(vocab_path: &str, merges_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let vocab_raw = fs::read_to_string(vocab_path)?;
        let vocab: VocabFile = serde_json::from_str(&vocab_raw)?;
        let merges_raw = fs::read_to_string(merges_path)?;
        let merges: MergesFile = serde_json::from_str(&merges_raw)?;

        let mut token_to_id: HashMap<String, u32> = HashMap::with_capacity(vocab.vocab.len());
        let mut id_to_token: Vec<String> = Vec::with_capacity(vocab.vocab.len());
        for e in &vocab.vocab {
            token_to_id.insert(e.token.clone(), e.id);
            id_to_token.push(e.token.clone());
        }

        let mut merge_ranks: HashMap<(String, String), usize> =
            HashMap::with_capacity(merges.merges.len());
        for m in &merges.merges {
            merge_ranks.insert((m.left.clone(), m.right.clone()), m.rank);
        }

        let lookup = |name: &str| -> u32 { *token_to_id.get(name).unwrap_or(&0) };
        let pad_id = lookup("<pad>");
        let bos_id = lookup("<bos>");
        let eos_id = lookup("<eos>");
        let unk_id = lookup("<unk>");

        Ok(Self {
            token_to_id,
            id_to_token,
            merge_ranks,
            pad_id,
            bos_id,
            eos_id,
            unk_id,
        })
    }

    pub fn vocab_size(&self) -> usize {
        self.id_to_token.len()
    }

    fn apply_merges(&self, mut tokens: Vec<String>) -> Vec<String> {
        loop {
            let mut best_rank = usize::MAX;
            let mut best_pair: Option<(String, String)> = None;
            for i in 0..tokens.len().saturating_sub(1) {
                let pair = (tokens[i].clone(), tokens[i + 1].clone());
                if let Some(&rank) = self.merge_ranks.get(&pair) {
                    if rank < best_rank {
                        best_rank = rank;
                        best_pair = Some(pair);
                    }
                }
            }
            let Some((left, right)) = best_pair else {
                break;
            };
            let merged = format!("{}{}", left, right);
            let mut new_tokens: Vec<String> = Vec::with_capacity(tokens.len());
            let mut i = 0;
            while i < tokens.len() {
                if i + 1 < tokens.len() && tokens[i] == left && tokens[i + 1] == right {
                    new_tokens.push(merged.clone());
                    i += 2;
                } else {
                    new_tokens.push(tokens[i].clone());
                    i += 1;
                }
            }
            tokens = new_tokens;
        }
        tokens
    }

    /// Encode raw text into a sequence of token ids. Does NOT add <bos>/<eos>.
    pub fn encode(
        &self,
        text: &str,
        lexicon: &SegmentationLexicon,
        rules: &SegmentationRuleSet,
    ) -> Vec<u32> {
        let pretoks = pretokenize(text, lexicon, rules);
        let final_tokens = self.apply_merges(pretoks);
        final_tokens
            .into_iter()
            .map(|t| *self.token_to_id.get(&t).unwrap_or(&self.unk_id))
            .collect()
    }

    /// Decode ids back to text. Special tokens (<pad>/<bos>/<eos>) are skipped.
    /// Unknown tokens are emitted literally as "<unk>".
    pub fn decode(&self, ids: &[u32]) -> String {
        let mut result = String::new();
        for &id in ids {
            let Some(token) = self.id_to_token.get(id as usize) else {
                continue;
            };
            if token == "<pad>" || token == "<bos>" || token == "<eos>" {
                continue;
            }
            if let Some(stripped) = token.strip_prefix('\u{2581}') {
                if !result.is_empty() {
                    result.push(' ');
                }
                result.push_str(stripped);
            } else {
                result.push_str(token);
            }
        }
        result
    }
}
