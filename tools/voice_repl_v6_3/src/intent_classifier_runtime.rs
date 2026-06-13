// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! **Phase 19 step D (2026-06-02)** — runtime inference for the
//! intent classifier trained in step C.
//!
//! Loads the checkpoint at
//! `data/checkpoints/intent_classifier/{config,labels,model.mpk}.json`,
//! reuses the same BPE + segmentation tokenizer the v6 LM uses,
//! and exposes:
//!
//! ```ignore
//! pub fn classify(rescorer: &IntentClassifierRuntime, text: &str)
//!     -> Option<(String, f32)>;  // (intent_label, confidence)
//! ```
//!
//! Designed for the voice REPL's parallel-with-substring mode:
//! the substring-based `Conversation::turn` keeps running; the
//! neural classifier's prediction is logged alongside so we can
//! see discrepancies before phasing substring out.
//!
//! Missing files → loader returns `None`, runtime quietly skips
//! the neural pass.  Same contract as `neural_rescorer.rs`.

use adam_agg_model::intent_classifier::{IntentClassifier, IntentClassifierConfig};
use adam_kernel::{SegmentationLexicon, SegmentationRuleSet};
use adam_tokenizer::bpe::BpeTokenizer;
use burn::backend::ndarray::{NdArray, NdArrayDevice};
use burn::module::Module;
use burn::prelude::*;
use burn::record::{FullPrecisionSettings, NamedMpkFileRecorder};
use serde::Deserialize;

type B = NdArray<f32>;

const CHECKPOINT_DIR: &str = "data/checkpoints/intent_classifier";
// **v6.6 generative pivot (2026-06-13)** — intent classifier loads the
// PRE-v6.6 BPE + FST (5188 vocab, non-drift-augmented FST) because the
// 2914-sample training pack underfit the 15647-vocab v6.6 BPE (~0.18
// samples per token → most rare-token embeddings untrained). Drift
// battery accuracy regressed 76% → 53% under canonical v6.6 BPE.
// Measured 2026-06-13 — see `intent_drift_eval` results in
// `project_v6_6_pivot_validated_2026_06_12.md` follow-up.
//
// The neural rescorer (LM) keeps loading canonical paths because its
// 18.7M-token training corpus fully covers the 15647 vocab. Two
// tokenizers in production is the cost of letting each consumer pick
// the vocab size it has enough training data to support.
const BPE_VOCAB: &str = "data/tokenizer/bpe_vocab.baseline_v6_5.json";
const BPE_MERGES: &str = "data/tokenizer/bpe_merges.baseline_v6_5.json";
const SEG_ROOTS: &str = "data/tokenizer/segmentation_roots.baseline_v6_5.json";
const SEG_RULES: &str = "data/tokenizer/segmentation_rules.json";

#[derive(Debug, Deserialize)]
struct ClassifierConfigJson {
    vocab_size: usize,
    d_model: usize,
    hidden: usize,
    n_intents: usize,
    max_seq_len: usize,
}

pub struct IntentClassifierRuntime {
    model: IntentClassifier<B>,
    device: NdArrayDevice,
    bpe: BpeTokenizer,
    lexicon: SegmentationLexicon,
    rules: SegmentationRuleSet,
    pub labels: Vec<String>,
    pub max_seq_len: usize,
}

impl IntentClassifierRuntime {
    /// Try to load every artefact the runtime needs.  Returns
    /// `None` (with a stderr diagnostic for each missing piece)
    /// when any loader fails — voice REPL then runs without
    /// neural intent classification.
    pub fn load_default() -> Option<Self> {
        let device = NdArrayDevice::Cpu;
        let dir = std::path::Path::new(CHECKPOINT_DIR);

        let config_path = dir.join("config.json");
        let labels_path = dir.join("labels.json");
        let model_path = dir.join("model.mpk");
        if !config_path.exists() || !labels_path.exists() || !model_path.exists() {
            eprintln!(
                "[voice-repl] intent classifier: missing checkpoint files in {CHECKPOINT_DIR}"
            );
            return None;
        }

        let cfg_json: ClassifierConfigJson = match std::fs::read_to_string(&config_path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
        {
            Some(c) => c,
            None => {
                eprintln!("[voice-repl] intent classifier: bad config.json");
                return None;
            }
        };
        let labels: Vec<String> = match std::fs::read_to_string(&labels_path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
        {
            Some(l) => l,
            None => {
                eprintln!("[voice-repl] intent classifier: bad labels.json");
                return None;
            }
        };

        // Build a freshly-initialised model with the saved config and
        // overlay the weights.  load() variant of burn record API
        // wants an initialised module to receive weights into.
        let cfg = IntentClassifierConfig::new(
            cfg_json.vocab_size,
            cfg_json.d_model,
            cfg_json.hidden,
            cfg_json.n_intents,
        );
        let init: IntentClassifier<B> = cfg.init(&device);
        let recorder = NamedMpkFileRecorder::<FullPrecisionSettings>::new();
        let model = match init.load_file(dir.join("model"), &recorder, &device) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[voice-repl] intent classifier: model load failed ({e})");
                return None;
            }
        };

        let bpe = match BpeTokenizer::load(BPE_VOCAB, BPE_MERGES) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("[voice-repl] intent classifier: BPE load failed ({e})");
                return None;
            }
        };
        let lexicon: SegmentationLexicon = load_json(SEG_ROOTS)?;
        let rules: SegmentationRuleSet = load_json(SEG_RULES)?;

        println!(
            "[voice-repl] intent classifier: ready (vocab={}, n_intents={}, ckpt={})",
            cfg_json.vocab_size, cfg_json.n_intents, CHECKPOINT_DIR
        );

        Some(Self {
            model,
            device,
            bpe,
            lexicon,
            rules,
            labels,
            max_seq_len: cfg_json.max_seq_len,
        })
    }

    /// Predict intent + confidence for an input utterance.
    /// Returns the intent **string label** (not enum), and softmax
    /// max as a confidence in [0, 1].
    pub fn classify(&self, text: &str) -> Option<(String, f32)> {
        let mut ids: Vec<u32> = vec![self.bpe.bos_id];
        ids.extend(self.bpe.encode(text, &self.lexicon, &self.rules));
        ids.push(self.bpe.eos_id);
        // Pad / truncate.
        let take = ids.len().min(self.max_seq_len);
        let mut ids_buf = vec![0i64; self.max_seq_len];
        let mut mask_buf = vec![0.0_f32; self.max_seq_len];
        for i in 0..take {
            ids_buf[i] = ids[i] as i64;
            mask_buf[i] = 1.0;
        }
        let tokens: Tensor<B, 2, Int> = Tensor::from_data(
            burn::tensor::TensorData::new(ids_buf, [1, self.max_seq_len]),
            &self.device,
        );
        let mask: Tensor<B, 2> = Tensor::from_data(
            burn::tensor::TensorData::new(mask_buf, [1, self.max_seq_len]),
            &self.device,
        );
        let (id, conf) = self.model.predict_one(tokens, mask);
        self.labels.get(id).map(|s| (s.clone(), conf))
    }
}

fn load_json<T: serde::de::DeserializeOwned>(path: &str) -> Option<T> {
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}
