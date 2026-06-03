// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! Lexicon-constrained phoneme decoder (Phase 14 step 1,
//! 2026-05-29).
//!
//! The Phase 13 `recognise_stream` decoder runs a frame-
//! synchronous Viterbi over a **fully-connected** phoneme
//! graph — every phoneme can follow every other phoneme,
//! penalised only by a flat-LM switch cost. On real speech
//! this over-segments and emits acoustically-plausible but
//! linguistically-impossible sequences.
//!
//! The lexicon-constrained decoder restricts the path to
//! sequences that are concatenations of **known Kazakh words**.
//! A phoneme trie built from the curated lexicon defines the
//! legal grammar; Viterbi walks it.
//!
//! ## State machine
//!
//! ```text
//!   state := root | node ∈ trie
//!   transitions (per frame):
//!     stay     :  node → node                            (self-loop)
//!     advance  :  parent(node) → node                    (next phoneme of same word)
//!     boundary :  any terminal node → root  → root.child (word boundary, costs `switch_penalty`)
//! ```
//!
//! Output: the phoneme sequence collected by the best path,
//! run-length-collapsed.
//!
//! ## Cost-scale parity with `recognise_stream`
//!
//! Same Euclidean min-over-K-exemplars emission, same per-
//! utterance CMVN, same `switch_penalty` semantics. The only
//! difference is **what transitions are legal** — every other
//! moving part is identical so cross-decoder PER comparisons
//! are apples-to-apples.

use adam_audio::mfcc::MfccSequence;
use adam_phoneme::Phoneme;
use std::collections::HashMap;

use crate::bank::PhonemeBank;

/// Tunable parameters for [`recognise_lexicon_constrained`].
#[derive(Debug, Clone, Copy)]
pub struct LexiconDecoderConfig {
    /// Cost added at each word boundary (terminal node → root
    /// → root.child). Same scale as `StreamConfig::switch_penalty`.
    pub switch_penalty: f32,
}

impl Default for LexiconDecoderConfig {
    fn default() -> Self {
        // Inherit the StreamConfig tuning from `recognise_stream`.
        Self {
            switch_penalty: 3.0,
        }
    }
}

/// Run lexicon-constrained Viterbi decoding.
///
/// `vocabulary` is a list of phoneme sequences — one per word
/// the decoder is allowed to emit. The caller supplies it so
/// `adam-stt-phoneme` doesn't pull a vocabulary dependency.
///
/// Returns the phoneme stream of the best path, run-length-
/// collapsed (consecutive identical phonemes merged).
pub fn recognise_lexicon_constrained(
    query: &MfccSequence,
    bank: &PhonemeBank,
    vocabulary: &[&[Phoneme]],
    cfg: &LexiconDecoderConfig,
) -> Vec<Phoneme> {
    let t = query.num_frames();
    if t == 0 || bank.is_empty() || vocabulary.is_empty() {
        return Vec::new();
    }
    let query = adam_audio::cmvn::normalise(query);

    // Build phoneme trie from vocabulary. Words sharing a
    // prefix share trie nodes.
    let trie = Trie::build(vocabulary);
    if trie.nodes.is_empty() {
        return Vec::new();
    }

    // Pre-compute centroid sets per Phoneme (one set covers
    // all trie nodes that emit that phoneme).
    let mut centroid_sets: HashMap<Phoneme, Vec<Vec<f32>>> = HashMap::new();
    for (p, exemplars) in bank.iter_all() {
        let mut set: Vec<Vec<f32>> = Vec::new();
        for tmpl in exemplars {
            if tmpl.mfcc.dim() == query.dim() && tmpl.mfcc.num_frames() > 0 {
                set.push(centroid(&tmpl.mfcc));
            }
        }
        if !set.is_empty() {
            centroid_sets.insert(*p, set);
        }
    }

    // Per-frame emission cache: emit_cache[frame][phoneme] =
    // min over phoneme's exemplar centroids of euclid(frame,c).
    // Many trie nodes share the same phoneme, so per-frame
    // we look up each phoneme's cost once.
    let emit_for = |frame: &[f32], p: Phoneme| -> f32 {
        match centroid_sets.get(&p) {
            Some(set) => set
                .iter()
                .map(|c| euclid(frame, c))
                .fold(f32::INFINITY, f32::min),
            None => f32::INFINITY,
        }
    };

    // States: 0..n_nodes — one per trie node. There is NO
    // root state in the DP. Root is a virtual junction:
    // entering it from a terminal node and immediately leaving
    // to a first-phoneme node happens **between** frames as a
    // word-boundary transition, paying `switch_penalty`. Every
    // frame consumes exactly one phoneme emission — there is
    // no "stay at root for free" escape hatch.
    let n_nodes = trie.nodes.len();
    let inf = f32::INFINITY;

    // Init at frame 0: only first-phoneme nodes are reachable
    // (no preceding word ⇒ no switch penalty).
    let mut prev_cost = vec![inf; n_nodes];
    for &root_child in &trie.root_children {
        let p = trie.nodes[root_child].phoneme;
        prev_cost[root_child] = emit_for(&query.frames[0], p);
    }
    // Back-pointers: for each frame ti > 0 and each node s,
    // back[ti][s] = source node id at frame ti-1.
    // A sentinel `u32::MAX` means "came from a word-boundary"
    // (i.e. from the best terminal at ti-1).
    let mut back: Vec<Vec<u32>> = vec![vec![0; n_nodes]; t];
    // Track which terminal was the best at each frame (the
    // word-boundary fan-in source).
    let mut prev_best_term: Vec<u32> = vec![u32::MAX; t];

    let mut cur_cost = vec![inf; n_nodes];
    for ti in 1..t {
        let frame = &query.frames[ti];
        // Best terminal cost from previous frame — used as the
        // fan-in for word-boundary transitions at THIS frame.
        let mut best_term_cost = inf;
        let mut best_term_idx: u32 = u32::MAX;
        for &n_id in &trie.terminals {
            if prev_cost[n_id] < best_term_cost {
                best_term_cost = prev_cost[n_id];
                best_term_idx = n_id as u32;
            }
        }
        prev_best_term[ti] = best_term_idx;

        for n_id in 0..n_nodes {
            let node = &trie.nodes[n_id];
            let emit_p = emit_for(frame, node.phoneme);

            // Candidate 1: stay (self-loop).
            let stay = prev_cost[n_id];
            let mut best = stay;
            let mut best_from = n_id as u32;

            // Candidate 2: advance from parent (mid-word).
            if let Some(parent) = node.parent {
                let adv = prev_cost[parent];
                if adv < best {
                    best = adv;
                    best_from = parent as u32;
                }
            } else {
                // Candidate 3: word-boundary — coming from the
                // best terminal at ti-1, paying `switch_penalty`.
                // Only allowed for first-phoneme nodes.
                if best_term_idx != u32::MAX {
                    let boundary = best_term_cost + cfg.switch_penalty;
                    if boundary < best {
                        best = boundary;
                        best_from = u32::MAX; // sentinel: came via boundary
                    }
                }
            }

            cur_cost[n_id] = if best.is_finite() { best + emit_p } else { inf };
            back[ti][n_id] = best_from;
        }
        std::mem::swap(&mut prev_cost, &mut cur_cost);
        for c in cur_cost.iter_mut() {
            *c = inf;
        }
    }

    // Terminate at the best terminal node — the path is
    // constrained to end on a word boundary.
    let mut best_final = inf;
    let mut best_final_state: u32 = u32::MAX;
    for &n_id in &trie.terminals {
        if prev_cost[n_id] < best_final {
            best_final = prev_cost[n_id];
            best_final_state = n_id as u32;
        }
    }
    if best_final_state == u32::MAX || !best_final.is_finite() {
        return Vec::new();
    }

    // Back-trace: every frame's state emits its node's phoneme.
    // A `u32::MAX` back-pointer means "came via word-boundary",
    // and the actual previous-frame state is the best terminal
    // recorded in `prev_best_term[ti]`.
    let mut path_phonemes: Vec<Phoneme> = Vec::with_capacity(t);
    let mut state = best_final_state;
    for ti in (1..t).rev() {
        path_phonemes.push(trie.nodes[state as usize].phoneme);
        let bp = back[ti][state as usize];
        if bp == u32::MAX {
            // Word-boundary: the previous frame was at the best
            // terminal recorded for frame ti.
            state = prev_best_term[ti];
        } else {
            state = bp;
        }
    }
    // Final emission for frame 0.
    path_phonemes.push(trie.nodes[state as usize].phoneme);
    path_phonemes.reverse();

    // Run-length-collapse consecutive identical phonemes
    // (within-phoneme self-loops).
    let mut out: Vec<Phoneme> = Vec::with_capacity(path_phonemes.len());
    let mut last: Option<Phoneme> = None;
    for p in path_phonemes {
        if last != Some(p) {
            out.push(p);
            last = Some(p);
        }
    }
    out
}

// ─── Phoneme trie ─────────────────────────────────────────────

struct TrieNode {
    phoneme: Phoneme,
    /// `None` for first-phoneme nodes (children of root).
    parent: Option<usize>,
    children: Vec<(Phoneme, usize)>,
    is_terminal: bool,
}

struct Trie {
    nodes: Vec<TrieNode>,
    root_children: Vec<usize>,
    terminals: Vec<usize>,
}

impl Trie {
    fn build(vocabulary: &[&[Phoneme]]) -> Self {
        let mut nodes: Vec<TrieNode> = Vec::new();
        // root: track its children separately (no node entry
        // — we don't emit at root).
        let mut root_children: Vec<usize> = Vec::new();

        for word in vocabulary {
            if word.is_empty() {
                continue;
            }
            // Find / create first-phoneme node.
            let mut current: Option<usize> = None;
            for (i, &p) in word.iter().enumerate() {
                let next_id = if let Some(cur) = current {
                    // Look up or create child of `cur`.
                    let existing = nodes[cur]
                        .children
                        .iter()
                        .find(|(pp, _)| *pp == p)
                        .map(|(_, id)| *id);
                    match existing {
                        Some(id) => id,
                        None => {
                            let id = nodes.len();
                            nodes.push(TrieNode {
                                phoneme: p,
                                parent: Some(cur),
                                children: Vec::new(),
                                is_terminal: false,
                            });
                            nodes[cur].children.push((p, id));
                            id
                        }
                    }
                } else {
                    // First phoneme: look up or create root_child.
                    let existing = root_children
                        .iter()
                        .find(|&&id| nodes[id].phoneme == p)
                        .copied();
                    match existing {
                        Some(id) => id,
                        None => {
                            let id = nodes.len();
                            nodes.push(TrieNode {
                                phoneme: p,
                                parent: None,
                                children: Vec::new(),
                                is_terminal: false,
                            });
                            root_children.push(id);
                            id
                        }
                    }
                };
                if i + 1 == word.len() {
                    nodes[next_id].is_terminal = true;
                }
                current = Some(next_id);
            }
        }

        let terminals: Vec<usize> = nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.is_terminal)
            .map(|(i, _)| i)
            .collect();
        Self {
            nodes,
            root_children,
            terminals,
        }
    }
}

// ─── Helpers — duplicated from `recogniser` to keep the
// module standalone. ──────────────────────────────────────────

fn centroid(seq: &MfccSequence) -> Vec<f32> {
    let dim = seq.dim();
    let mut mean = vec![0.0_f32; dim];
    for frame in &seq.frames {
        for (m, x) in mean.iter_mut().zip(frame.iter()) {
            *m += *x;
        }
    }
    let inv = 1.0 / seq.num_frames().max(1) as f32;
    for m in &mut mean {
        *m *= inv;
    }
    mean
}

fn euclid(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return f32::INFINITY;
    }
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f32>()
        .sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bank::{PhonemeBank, PhonemeTemplate};
    use adam_audio::mfcc::MfccSequence;
    use adam_phoneme::Phoneme::*;

    fn frame(c: f32) -> Vec<f32> {
        vec![c; 13]
    }
    fn seq(values: &[f32]) -> MfccSequence {
        MfccSequence {
            frames: values.iter().map(|&v| frame(v)).collect(),
            sample_rate: 16_000,
            hop_length: 160,
            n_mfcc: 13,
        }
    }
    fn bank_with(a_val: f32, b_val: f32) -> PhonemeBank {
        let mut b = PhonemeBank::new();
        b.insert(PhonemeTemplate {
            phoneme: A,
            mfcc: seq(&[a_val]),
        });
        b.insert(PhonemeTemplate {
            phoneme: B,
            mfcc: seq(&[b_val]),
        });
        b
    }

    /// Trie correctly shares prefixes.
    #[test]
    fn trie_shares_prefix() {
        let v: &[&[Phoneme]] = &[&[A, B], &[A, B, A]];
        let trie = Trie::build(v);
        // Expect 3 nodes (A → B → A) with [AB] AND [ABA] terminal.
        assert_eq!(trie.nodes.len(), 3);
        // The "AB" node is terminal AND has one child.
        let ab = trie
            .nodes
            .iter()
            .find(|n| n.phoneme == B && n.is_terminal)
            .unwrap();
        assert!(!ab.children.is_empty());
    }

    /// Vocabulary {"A","B"}, audio low-then-high → decoder
    /// outputs [A, B]. Same setup as the free-decoder test, in
    /// CMVN space.
    #[test]
    fn decodes_two_single_phoneme_words() {
        let bank = bank_with(-1.0, 1.0);
        let vocab: &[&[Phoneme]] = &[&[A], &[B]];
        let query = seq(&[0.0, 0.5, 0.0, 10.0, 10.5, 10.0]);
        let out =
            recognise_lexicon_constrained(&query, &bank, vocab, &LexiconDecoderConfig::default());
        assert_eq!(out, vec![A, B]);
    }

    /// Empty inputs degrade gracefully.
    #[test]
    fn degenerate_inputs() {
        let bank = bank_with(-1.0, 1.0);
        let vocab: &[&[Phoneme]] = &[&[A]];
        let empty = MfccSequence {
            frames: vec![],
            sample_rate: 16_000,
            hop_length: 160,
            n_mfcc: 13,
        };
        assert!(
            recognise_lexicon_constrained(&empty, &bank, vocab, &LexiconDecoderConfig::default())
                .is_empty()
        );
        let query = seq(&[1.0, 2.0]);
        assert!(
            recognise_lexicon_constrained(&query, &bank, &[], &LexiconDecoderConfig::default())
                .is_empty()
        );
    }

    /// Vocabulary {"AB"} (no "A" word alone) — decoder cannot
    /// emit just [A]; the only legal terminal is AB, so even
    /// when the audio looks like a single A, the decoder
    /// either reaches AB or the path is pruned.
    #[test]
    fn constrained_to_legal_words() {
        let bank = bank_with(-1.0, 1.0);
        let vocab: &[&[Phoneme]] = &[&[A, B]];
        let query = seq(&[0.0, 0.5, 10.0, 10.5]);
        let out =
            recognise_lexicon_constrained(&query, &bank, vocab, &LexiconDecoderConfig::default());
        // Output should be [A, B] (the only legal word) and
        // its RLE collapse.
        assert_eq!(out, vec![A, B]);
    }
}
