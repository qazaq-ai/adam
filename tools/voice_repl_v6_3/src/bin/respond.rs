// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! **v6.7 generative pivot (2026-06-13)** — first-cut response
//! generator. Loads the v6.7 fine-tuned LM checkpoint, takes a prompt,
//! encodes "{prompt} →→" and greedy-generates the response.
//!
//! Unconstrained (no FST mask yet) — primary goal is to verify the
//! fine-tuned model can produce coherent Kazakh responses from
//! conditional prompts. Once verified, we wire in
//! `generate_constrained` for hallucination-free output.
//!
//! Usage:
//!   ./target/release/respond "{prompt text}"
//!   ./target/release/respond --eval  # run on data/eval/v6_7_real_audit_eval.json

use adam_agg_model::checkpoint::load_checkpoint;
use adam_agg_model::generate::generate_unconstrained;
use adam_agg_model::{TinyAgt, TinyAgtConfig};
use adam_kernel::{SegmentationLexicon, SegmentationRuleSet};
use adam_tokenizer::bpe::BpeTokenizer;
use burn::backend::ndarray::{NdArray, NdArrayDevice};
use serde::Deserialize;
use std::env;

type B = NdArray<f32>;

const CHECKPOINT_DIR: &str = "data/checkpoints/contextual_lm_v6_7_ft";
const BPE_VOCAB: &str = "data/tokenizer/bpe_vocab.json";
const BPE_MERGES: &str = "data/tokenizer/bpe_merges.json";
const SEG_ROOTS: &str = "data/tokenizer/segmentation_roots.json";
const SEG_RULES: &str = "data/tokenizer/segmentation_rules.json";
const EVAL_PATH: &str = "data/eval/v6_7_real_audit_eval.json";
const SEPARATOR: &str = " →→ ";
const MAX_NEW_TOKENS: usize = 64;

struct Responder {
    model: TinyAgt<B>,
    device: NdArrayDevice,
    bpe: BpeTokenizer,
    lexicon: SegmentationLexicon,
    rules: SegmentationRuleSet,
}

impl Responder {
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

    fn respond(&self, prompt: &str) -> String {
        // Encode "{prompt} →→" as prefix and let the model continue.
        let conditioned = format!("{}{}", prompt.trim(), SEPARATOR);
        let mut ids: Vec<i64> = vec![self.bpe.bos_id as i64];
        for id in self.bpe.encode(&conditioned, &self.lexicon, &self.rules) {
            ids.push(id as i64);
        }
        let prefix_len = ids.len();
        let out = generate_unconstrained(&self.model, &ids, MAX_NEW_TOKENS, &self.device);
        // Decode only the generated suffix.
        let response_ids: Vec<u32> = out[prefix_len..]
            .iter()
            .filter_map(|&i| if i >= 0 { Some(i as u32) } else { None })
            .collect();
        self.bpe.decode(&response_ids)
    }
}

#[derive(Debug, Deserialize)]
struct EvalCase {
    input: String,
    expected_response: Option<String>,
    was_accepted: bool,
    #[allow(dead_code)]
    notes: String,
}

#[derive(Debug, Deserialize)]
struct EvalPack {
    #[allow(dead_code)]
    version: String,
    cases: Vec<EvalCase>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let responder = Responder::load()?;

    if args.iter().any(|a| a == "--eval") {
        let pack: EvalPack = serde_json::from_str(&std::fs::read_to_string(EVAL_PATH)?)?;
        println!("[respond] eval mode — {} cases", pack.cases.len());
        let mut accepted_correct = 0;
        let mut accepted_total = 0;
        for (i, c) in pack.cases.iter().enumerate() {
            let predicted = responder.respond(&c.input);
            let expected = c.expected_response.clone().unwrap_or_else(|| "<none>".into());
            let pass = c.was_accepted
                && c.expected_response.as_ref().is_some_and(|e| {
                    let p = predicted.to_lowercase();
                    let e = e.to_lowercase();
                    // simple normalised-substring match (lenient)
                    p == e || p.contains(&e) || e.contains(&p)
                });
            if c.was_accepted {
                accepted_total += 1;
                if pass {
                    accepted_correct += 1;
                }
            }
            println!(
                "#{:<3} [{}] in: «{}»\n     expected: «{}»\n     predicted: «{}»\n     {}",
                i,
                if c.was_accepted { "ACC" } else { "REJ" },
                c.input,
                expected,
                predicted,
                if c.was_accepted {
                    if pass { "✓" } else { "✗" }
                } else {
                    "(was-rejected — any response is a probe)"
                }
            );
        }
        println!(
            "\n[respond] accepted cases: {}/{} = {:.0}%",
            accepted_correct,
            accepted_total,
            100.0 * accepted_correct as f32 / accepted_total.max(1) as f32
        );
        return Ok(());
    }

    let prompt = args
        .iter()
        .skip(1)
        .find(|a| !a.starts_with("--"))
        .cloned()
        .unwrap_or_else(|| {
            "Сәлеметсіз бе?".into()
        });
    let response = responder.respond(&prompt);
    println!("prompt:   «{}»", prompt);
    println!("response: «{}»", response);
    Ok(())
}
