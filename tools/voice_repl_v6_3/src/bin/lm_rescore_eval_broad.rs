// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! **v6.6 broad rescore eval (2026-06-13)** — generalisation test.
//!
//! The original `lm_rescore_eval` proves the hypothesis on 12
//! curated rc25-audit cases. This broader harness reads
//! `data/eval/v6_6_broad_rescore_eval.json` (mined from real
//! wikipedia sentences, 32 substitution drifts across ə ө ұ ү
//! i қ ғ ң + 6 rc25 reference cases = 38 total) and runs both
//! baseline and v6.6-drift LM, printing per-category pass rates.
//!
//! Usage:
//!     cargo run --release -p adam-voice-repl-v6-3 --bin lm_rescore_eval_broad

use adam_agg_model::checkpoint::load_checkpoint;
use adam_agg_model::score::score_sequence;
use adam_agg_model::{TinyAgt, TinyAgtConfig};
use adam_kernel::{SegmentationLexicon, SegmentationRuleSet};
use adam_tokenizer::bpe::BpeTokenizer;
use burn::backend::ndarray::{NdArray, NdArrayDevice};
use serde::Deserialize;
use std::collections::BTreeMap;

type B = NdArray<f32>;

const EVAL_PATH: &str = "data/eval/v6_6_broad_rescore_eval.json";
const CHECKPOINT_DIR: &str = "data/checkpoints/contextual_lm";
const BPE_VOCAB: &str = "data/tokenizer/bpe_vocab.json";
const BPE_MERGES: &str = "data/tokenizer/bpe_merges.json";
const SEG_ROOTS: &str = "data/tokenizer/segmentation_roots.json";
const SEG_RULES: &str = "data/tokenizer/segmentation_rules.json";

#[derive(Debug, Deserialize)]
struct Case {
    a: String,
    b: String,
    /// "a" → a should win (higher LM score), "b" → b should win.
    expected: String,
    category: String,
    #[serde(default)]
    note: String,
}

#[derive(Debug, Deserialize)]
struct EvalPack {
    #[allow(dead_code)]
    version: String,
    cases: Vec<Case>,
}

struct Rescorer {
    model: TinyAgt<B>,
    device: NdArrayDevice,
    bpe: BpeTokenizer,
    lexicon: SegmentationLexicon,
    rules: SegmentationRuleSet,
}

impl Rescorer {
    fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let device = NdArrayDevice::Cpu;
        let ckpt = load_checkpoint::<B>(std::path::Path::new(CHECKPOINT_DIR), &device)?;
        let TinyAgtConfig { .. } = ckpt.config;
        let bpe = BpeTokenizer::load(BPE_VOCAB, BPE_MERGES)?;
        let lexicon: SegmentationLexicon =
            serde_json::from_str(&std::fs::read_to_string(SEG_ROOTS)?)?;
        let rules: SegmentationRuleSet =
            serde_json::from_str(&std::fs::read_to_string(SEG_RULES)?)?;
        Ok(Self {
            model: ckpt.model,
            device,
            bpe,
            lexicon,
            rules,
        })
    }

    fn score_text(&self, text: &str) -> f32 {
        let mut ids: Vec<i64> = vec![self.bpe.bos_id as i64];
        for id in self.bpe.encode(text, &self.lexicon, &self.rules) {
            ids.push(id as i64);
        }
        ids.push(self.bpe.eos_id as i64);
        match score_sequence(&self.model, &ids, &self.device) {
            Some(r) => r.per_token,
            None => f32::NEG_INFINITY,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pack: EvalPack = serde_json::from_str(&std::fs::read_to_string(EVAL_PATH)?)?;
    println!("[v6.6 broad eval] {} cases", pack.cases.len());
    let rescorer = Rescorer::load()?;

    // Per-category counters: (correct, total)
    let mut per_cat: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut total_correct = 0;
    let mut wrong_examples: Vec<(usize, &Case, f32, f32, &'static str)> = Vec::new();

    println!("{:=^120}", " LM rescore broad eval ");
    for (i, c) in pack.cases.iter().enumerate() {
        let sa = rescorer.score_text(&c.a);
        let sb = rescorer.score_text(&c.b);
        let winner = if sa > sb { "a" } else { "b" };
        let ok = winner == c.expected;
        if ok {
            total_correct += 1;
        }
        let cat_key = c.category.split('_').next().unwrap_or("?").to_string();
        let e = per_cat.entry(cat_key.clone()).or_insert((0, 0));
        e.1 += 1;
        if ok {
            e.0 += 1;
        }
        if !ok {
            let predicted = if winner == "a" { "a" } else { "b" };
            wrong_examples.push((i, c, sa, sb, if predicted == "a" { "a" } else { "b" }));
        }
    }

    println!("\n{:-^60}", " summary by category ");
    println!("{:<14} {:>6}  {:>6}  {:>6}", "category", "ok", "total", "rate");
    for (cat, (ok, tot)) in &per_cat {
        println!(
            "{:<14} {:>6}  {:>6}  {:>5.0}%",
            cat,
            ok,
            tot,
            100.0 * (*ok as f32) / (*tot as f32)
        );
    }
    println!(
        "{:<14} {:>6}  {:>6}  {:>5.0}%",
        "TOTAL",
        total_correct,
        pack.cases.len(),
        100.0 * (total_correct as f32) / (pack.cases.len() as f32)
    );

    if !wrong_examples.is_empty() {
        println!("\n{:-^60}", " misclassified ");
        for (i, c, sa, sb, predicted) in wrong_examples.iter().take(10) {
            println!(
                "#{:<3} [{}] expected={} predicted={}\n  a (s={:+.3}): «{}»\n  b (s={:+.3}): «{}»\n  note: {}",
                i, c.category, c.expected, predicted, sa, c.a, sb, c.b, c.note
            );
        }
    }
    Ok(())
}
