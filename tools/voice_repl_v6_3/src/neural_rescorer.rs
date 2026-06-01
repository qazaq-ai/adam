// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! **Phase 15g.C.2 step 3 (2026-06-01)** — neural sentence-scoring
//! safety net for `fuzzy_normalise`.
//!
//! User directive: «А в fuzzy реализуем нейронную сеть с весами,
//! чтобы он выбирал лучший вариант из возможных согласно
//! контекста предложения.»
//!
//! ## What this module does
//!
//! Loads the small contextual LM trained by
//! `train_contextual_lm` (vocab=5188 BPE tokens, ~2 M params,
//! cross-entropy 1.12 on the v6.3 Kazakh corpus) and exposes one
//! function:
//!
//! ```ignore
//! pub fn score_text(rescorer: &NeuralRescorer, text: &str) -> Option<f32>
//! ```
//!
//! Returns the **per-token average log-likelihood** the LM
//! assigns to the BPE-tokenized form of `text`. Higher (closer
//! to 0) means the sentence looks more like Kazakh the model has
//! seen during training.
//!
//! ## How the voice REPL uses it
//!
//! After `fuzzy_normalise` has produced a rewritten sentence,
//! the REPL calls `score_text` on BOTH the raw Whisper output
//! and the rewritten version. If fuzzy's rewrite has a LOWER
//! score (= the rewrite is less plausible than the original),
//! the REPL reverts to the original — preventing regressions
//! like «даулет → сәулет» that were the failure mode in
//! Phase 15g.B.1 / B.2.
//!
//! ## Why a sentence-level safety net, not per-word rescoring
//!
//! Per-word rescoring requires generating N candidate sentences
//! per ambiguous token + N forward passes per token. Wall-clock
//! cost on CPU is prohibitive for an interactive REPL. Whole-
//! sentence scoring is two passes per turn (raw + rewritten)
//! and catches the bulk of fuzzy false positives because they
//! land the sentence in unlikely-context regions of the LM.
//!
//! ## Failure modes / fallback
//!
//! Missing checkpoint, missing BPE files, or segmentation
//! manifests → all loaders return `None` from
//! [`NeuralRescorer::load_default`]. The REPL keeps running
//! without rescoring (a printed `[voice-repl] neural rescorer
//! unavailable: …` warning at startup is the only signal).
//! This matches the "the REPL still runs" contract of every
//! other loader in the voice-REPL stack.

use adam_agg_model::checkpoint::load_checkpoint;
use adam_agg_model::score::score_sequence;
use adam_agg_model::{TinyAgt, TinyAgtConfig};
use adam_kernel::{SegmentationLexicon, SegmentationRuleSet};
use adam_tokenizer::bpe::BpeTokenizer;
use burn::backend::ndarray::{NdArray, NdArrayDevice};

type B = NdArray<f32>;

const CHECKPOINT_DIR: &str = "data/checkpoints/contextual_lm";
const BPE_VOCAB: &str = "data/tokenizer/bpe_vocab.json";
const BPE_MERGES: &str = "data/tokenizer/bpe_merges.json";
const SEG_ROOTS: &str = "data/tokenizer/segmentation_roots.json";
const SEG_RULES: &str = "data/tokenizer/segmentation_rules.json";

pub struct NeuralRescorer {
    model: TinyAgt<B>,
    device: NdArrayDevice,
    bpe: BpeTokenizer,
    lexicon: SegmentationLexicon,
    rules: SegmentationRuleSet,
    /// `vocab_size` from the loaded checkpoint config — used as a
    /// sanity check against the BPE vocab so a mis-paired checkpoint
    /// degrades cleanly instead of crashing on tensor shape mismatch.
    pub vocab_size: usize,
}

impl NeuralRescorer {
    /// Try to load every artefact the rescorer needs. Returns
    /// `None` (with an `eprintln` for each missing piece) if any
    /// loader fails — voice REPL then runs without rescoring.
    pub fn load_default() -> Option<Self> {
        let device = NdArrayDevice::Cpu;

        let ckpt = match load_checkpoint::<B>(std::path::Path::new(CHECKPOINT_DIR), &device) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[voice-repl] neural rescorer: checkpoint load failed ({e})");
                return None;
            }
        };
        let TinyAgtConfig { vocab_size, .. } = ckpt.config;

        let bpe = match BpeTokenizer::load(BPE_VOCAB, BPE_MERGES) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("[voice-repl] neural rescorer: BPE load failed ({e})");
                return None;
            }
        };
        if bpe.vocab_size() != vocab_size {
            eprintln!(
                "[voice-repl] neural rescorer: BPE vocab ({}) does not match checkpoint ({}) — \
                 the model was trained against a different tokenizer",
                bpe.vocab_size(),
                vocab_size
            );
            return None;
        }

        let lexicon: SegmentationLexicon = match load_json(SEG_ROOTS) {
            Some(l) => l,
            None => {
                eprintln!("[voice-repl] neural rescorer: segmentation roots load failed");
                return None;
            }
        };
        let rules: SegmentationRuleSet = match load_json(SEG_RULES) {
            Some(r) => r,
            None => {
                eprintln!("[voice-repl] neural rescorer: segmentation rules load failed");
                return None;
            }
        };

        println!(
            "[voice-repl] neural rescorer: ready (vocab={}, ckpt={})",
            vocab_size, CHECKPOINT_DIR
        );

        Some(Self {
            model: ckpt.model,
            device,
            bpe,
            lexicon,
            rules,
            vocab_size,
        })
    }

    /// Per-token average log-likelihood under the contextual LM.
    /// Returns `None` if the sentence tokenises to fewer than 2
    /// tokens (no conditional positions to score).
    pub fn score_text(&self, text: &str) -> Option<f32> {
        let mut ids: Vec<u32> = vec![self.bpe.bos_id];
        ids.extend(self.bpe.encode(text, &self.lexicon, &self.rules));
        ids.push(self.bpe.eos_id);
        let i64_ids: Vec<i64> = ids.into_iter().map(|i| i as i64).collect();
        score_sequence::<B>(&self.model, &i64_ids, &self.device).map(|s| s.per_token)
    }
}

fn load_json<T: serde::de::DeserializeOwned>(path: &str) -> Option<T> {
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}
