// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! **v6.6 generative pivot (2026-06-11)** — drift-class regression
//! harness for the intent classifier.
//!
//! Loads the intent classifier from `data/checkpoints/intent_classifier`
//! and runs it against the 17 rc25-audit cases that the lexicon
//! validator was previously trying (and mostly failing) to repair.
//!
//! Run with the baseline checkpoint, then swap in the new one, and
//! compare the per-case results.  The goal of the v6.6 first
//! experiment: does scaling labelled data from 2 914 → 11 602
//! samples improve drift-case accuracy without any rule changes?
//!
//! Usage:
//!     cargo run --release -p adam-voice-repl-v6-3 --bin intent_drift_eval

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
// **v6.6 generative pivot (2026-06-13)** — mirrors the runtime path
// choice in intent_classifier_runtime.rs: intent classifier loads the
// pre-v6.6 BPE + FST because the 2914-sample training pack underfit
// the larger v6.6 vocab. See that file for the full rationale.
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

struct Runtime {
    model: IntentClassifier<B>,
    device: NdArrayDevice,
    bpe: BpeTokenizer,
    lexicon: SegmentationLexicon,
    rules: SegmentationRuleSet,
    labels: Vec<String>,
    max_seq_len: usize,
}

impl Runtime {
    fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let device = NdArrayDevice::Cpu;
        let dir = std::path::Path::new(CHECKPOINT_DIR);
        let cfg_json: ClassifierConfigJson =
            serde_json::from_str(&std::fs::read_to_string(dir.join("config.json"))?)?;
        let labels: Vec<String> =
            serde_json::from_str(&std::fs::read_to_string(dir.join("labels.json"))?)?;
        let cfg = IntentClassifierConfig::new(
            cfg_json.vocab_size,
            cfg_json.d_model,
            cfg_json.hidden,
            cfg_json.n_intents,
        );
        let init: IntentClassifier<B> = cfg.init(&device);
        let recorder = NamedMpkFileRecorder::<FullPrecisionSettings>::new();
        let model = init.load_file(dir.join("model"), &recorder, &device)?;
        let bpe = BpeTokenizer::load(BPE_VOCAB, BPE_MERGES)?;
        let lexicon: SegmentationLexicon =
            serde_json::from_str(&std::fs::read_to_string(SEG_ROOTS)?)?;
        let rules: SegmentationRuleSet =
            serde_json::from_str(&std::fs::read_to_string(SEG_RULES)?)?;
        Ok(Self {
            model,
            device,
            bpe,
            lexicon,
            rules,
            labels,
            max_seq_len: cfg_json.max_seq_len,
        })
    }

    fn classify(&self, text: &str) -> (String, f32) {
        let mut ids: Vec<u32> = vec![self.bpe.bos_id];
        ids.extend(self.bpe.encode(text, &self.lexicon, &self.rules));
        ids.push(self.bpe.eos_id);
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
        (
            self.labels.get(id).cloned().unwrap_or_else(|| "?".into()),
            conf,
        )
    }
}

/// Each case = (input text, expected intent, category label).
/// Input texts are what Whisper actually emitted in the rc25 audit
/// (NOT what the user actually said) — that's the whole point of
/// the drift-class regression: the classifier must survive raw
/// Whisper noise without the validator rewriting tokens.
fn cases() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        // === rc25 drift cases — validator was rewriting these ===
        (
            "Менің атым — дәулет.",
            "StatementOfName",
            "drift: дәулет (preserved)",
        ),
        (
            "Мен қостанайда тұрамын.",
            "StatementOfLocation",
            "drift: тұрамын (preserved)",
        ),
        (
            "Жасым алпыс алтыға толды.",
            "StatementOfAge",
            "drift: Жасым + толды",
        ),
        (
            "Жасым, алпыс алты да.",
            "StatementOfAge",
            "drift: Жасым (Whisper apocope)",
        ),
        (
            "Мен айттым ғой, қостанайда тұрам деп.",
            "StatementOfLocation",
            "drift: айттым + тұрам",
        ),
        (
            "Судың формулусын жазып берші.",
            "AskAboutTopic",
            "drift: берші (whisper)",
        ),
        (
            "Рақмет, алаған шүкір.",
            "StatementOfWellbeing",
            "drift: алаған",
        ),
        ("Қалыңғыз қалай.", "Greeting", "drift: Қалыңғыз (rc23 case)"),
        // === positive controls — must NOT change behaviour ===
        ("Сәлеметсіз бе?", "Greeting", "valid: greeting"),
        ("Менің атым Дәулет.", "StatementOfName", "valid: name"),
        (
            "Мен Қостанайда тұрамын.",
            "StatementOfLocation",
            "valid: loc",
        ),
        ("Жасым алпыс алты.", "StatementOfAge", "valid: age"),
        ("Қазір сағат неше?", "AskTime", "valid: time"),
        ("Бүгін қай күн?", "AskDate", "valid: date"),
        ("Менің атым кім?", "AskAboutSystem", "valid: ask name back"),
        (
            "Мен қай жерде тұрамын?",
            "AskLocation",
            "valid: ask location",
        ),
        ("Сау бол!", "Farewell", "valid: farewell"),
    ]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rt = Runtime::load()?;
    let cases = cases();
    let mut correct = 0;
    let mut correct_drift = 0;
    let mut correct_valid = 0;
    let n_drift = cases.iter().filter(|c| c.2.starts_with("drift")).count();
    let n_valid = cases.iter().filter(|c| c.2.starts_with("valid")).count();

    println!("{:=^96}", " intent drift eval ");
    println!(
        "{:<6} {:<38} {:<22} {:<22} {:<6}",
        "ok", "input", "expected", "predicted", "conf"
    );
    println!("{:-^96}", "");

    for (text, expected, category) in &cases {
        let (pred, conf) = rt.classify(text);
        let ok = pred == *expected;
        if ok {
            correct += 1;
            if category.starts_with("drift") {
                correct_drift += 1;
            } else {
                correct_valid += 1;
            }
        }
        println!(
            "{:<6} {:<38} {:<22} {:<22} {:>5.2}  [{}]",
            if ok { "✓" } else { "✗" },
            text,
            expected,
            pred,
            conf,
            category,
        );
    }

    println!("{:-^96}", "");
    println!(
        "drift: {}/{} ({:.0}%)   valid: {}/{} ({:.0}%)   total: {}/{} ({:.0}%)",
        correct_drift,
        n_drift,
        100.0 * correct_drift as f32 / n_drift as f32,
        correct_valid,
        n_valid,
        100.0 * correct_valid as f32 / n_valid as f32,
        correct,
        cases.len(),
        100.0 * correct as f32 / cases.len() as f32,
    );
    Ok(())
}
