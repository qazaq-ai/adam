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
use adam_agg_model::{TinyAgt, TinyAgtConfig};
use adam_kernel::{SegmentationLexicon, SegmentationRuleSet};
use adam_tokenizer::bpe::BpeTokenizer;
use burn::backend::ndarray::{NdArray, NdArrayDevice};
use burn::prelude::*;
use burn::tensor::activation::softmax;
use serde::Deserialize;
use std::collections::HashSet;
use std::env;

type B = NdArray<f32>;

const CHECKPOINT_DIR: &str = "data/checkpoints/contextual_lm_v6_7_stage2";
const BPE_VOCAB: &str = "data/tokenizer/bpe_vocab.json";
const BPE_MERGES: &str = "data/tokenizer/bpe_merges.json";
const SEG_ROOTS: &str = "data/tokenizer/segmentation_roots.json";
const SEG_RULES: &str = "data/tokenizer/segmentation_rules.json";
const EVAL_PATH: &str = "data/eval/v6_7_real_audit_eval.json";
const SEPARATOR: &str = " →→ ";
const MAX_NEW_TOKENS: usize = 64;

/// CTRL-style repetition penalty. Lower values let the model echo
/// itself more often; higher values push it into novel vocabulary.
/// 1.2 is the literature default for ≤1B models on conversational data.
const REP_PENALTY: f32 = 1.2;

/// Penalty applies to tokens within the last N positions of generated
/// output. Past this window, repeats are fine again (a long response
/// is allowed to mention "Дәулет" twice in different clauses).
const REP_WINDOW: usize = 24;

struct Responder {
    model: TinyAgt<B>,
    device: NdArrayDevice,
    bpe: BpeTokenizer,
    lexicon: SegmentationLexicon,
    rules: SegmentationRuleSet,
    /// Per-vocab-id flag: `true` means the token contains non-Cyrillic
    /// alphabetic characters (English letters), code symbols, or other
    /// glyphs that shouldn't appear in a clean Kazakh response. Set at
    /// load time from the BPE vocab and used by
    /// `generate_with_rep_penalty` as a hard mask.
    block_mask: Vec<bool>,
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

        // **v6.7 audit fix 2026-06-13** — FST-lite mask: forbid any
        // BPE token whose surface form contains English letters or
        // common code glyphs. Built once at load time, applied per-
        // step in the generator. Eliminates the "male" / "is" /
        // backtick-code style leaks that the 1M-param LM picked up
        // from the rust_book training corpus.
        let vocab_size = bpe.vocab_size();
        let mut block_mask = vec![false; vocab_size];
        for id in 0..vocab_size as u32 {
            if let Some(tok) = bpe.id_to_token(id) {
                if tok.chars().any(|c| {
                    c.is_ascii_alphabetic()
                        || matches!(c, '`' | '{' | '}' | '\\' | '[' | ']' | '<' | '>')
                }) {
                    block_mask[id as usize] = true;
                }
            }
        }
        let blocked = block_mask.iter().filter(|&&x| x).count();
        eprintln!("[respond] FST-lite mask: blocking {blocked}/{vocab_size} non-Cyrillic tokens");

        Ok(Self {
            model: ckpt.model,
            device,
            bpe,
            lexicon,
            rules,
            block_mask,
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
        let out = generate_with_rep_penalty(
            &self.model,
            &ids,
            MAX_NEW_TOKENS,
            REP_PENALTY,
            REP_WINDOW,
            self.bpe.eos_id as i64,
            &self.block_mask,
            &self.device,
        );
        // Decode the generated suffix and STOP at the first occurrence
        // of the "→→" separator the model emits — that's where it
        // started a new turn (it learned from the pair-pack pattern
        // "{prompt} →→ {response} <BOS> {next_prompt} →→ ...").
        // **v6.7 audit fix 2026-06-13** — caps the rambling that
        // FST-lite alone couldn't.
        let response_ids: Vec<u32> = out[prefix_len..]
            .iter()
            .filter_map(|&i| if i >= 0 { Some(i as u32) } else { None })
            .collect();
        let mut text = self.bpe.decode(&response_ids);
        // **v6.7 audit fix 2026-06-13** — natural sentence boundary
        // stop. The model otherwise rambles past a perfectly good
        // single-sentence reply. Truncate after the first sentence-
        // terminating punctuation followed by whitespace.
        let mut earliest: Option<usize> = None;
        for stop in [". ", "! ", "? ", ".\n", "!\n", "?\n"] {
            if let Some(idx) = text.find(stop) {
                let end = idx + 1; // include the punct char (1 ASCII byte)
                if earliest.map(|e| end < e).unwrap_or(true) {
                    earliest = Some(end);
                }
            }
        }
        if let Some(end) = earliest {
            text.truncate(end);
        }
        // Also stop at any future "→→" separator (next-turn artefact)
        if let Some(idx) = text.find("→→") {
            text.truncate(idx);
        }
        text.trim().to_string()
    }
}

/// Greedy decoder with **repetition penalty + n-gram blocking**.
///
/// `rep_penalty` (>1.0) applies the CTRL-paper trick: every token in
/// the last `rep_window` slots gets its raw logit divided by penalty
/// (when positive) or multiplied (when negative) — making it
/// progressively less likely to fire again. This breaks the
/// «бірдей болатын бірдей болатын …» loop the v6.7 prototype showed.
///
/// `rep_window` bounds how far back we look — keeping the penalty
/// local so legitimate repeats (numbers, names) elsewhere in long
/// responses aren't punished.
///
/// Additional n-gram block: if the SAME 3-token sequence has already
/// appeared in the generated text, that next token is forbidden.
/// This catches the case where a phrase repeats verbatim — the per-
/// token penalty alone isn't enough when each token is also common
/// outside the loop ("бір", "дей", "болатын").
fn generate_with_rep_penalty<B: Backend>(
    model: &TinyAgt<B>,
    prefix: &[i64],
    max_new_tokens: usize,
    rep_penalty: f32,
    rep_window: usize,
    eos_id: i64,
    block_mask: &[bool],
    device: &B::Device,
) -> Vec<i64> {
    let mut tokens: Vec<i64> = prefix.to_vec();
    let max_seq_len = model.max_seq_len();
    for _ in 0..max_new_tokens {
        let start = tokens.len().saturating_sub(max_seq_len);
        let window: Vec<i64> = tokens[start..].to_vec();
        let seq_len = window.len();
        let input: Tensor<B, 2, Int> =
            Tensor::from_data(burn::tensor::TensorData::new(window, [1, seq_len]), device);
        let logits = model.forward(input);
        let last = logits.slice([0..1, (seq_len - 1)..seq_len, 0..model.vocab_size()]);
        let last_2d = last.squeeze::<2>(1);
        let probs = softmax(last_2d, 1);
        let mut probs_vec: Vec<f32> =
            probs.into_data().as_slice::<f32>().unwrap_or(&[]).to_vec();

        // **v6.7 FST-lite mask** — hard-block any token that contains
        // non-Cyrillic / code glyphs. Applied BEFORE repetition penalty
        // so blocked tokens never compete.
        for (id, blocked) in block_mask.iter().enumerate() {
            if *blocked && id < probs_vec.len() {
                probs_vec[id] = 0.0;
            }
        }

        // Repetition penalty over the last `rep_window` generated tokens.
        let look_start = tokens.len().saturating_sub(rep_window);
        let recent: Vec<i64> = tokens[look_start..].to_vec();
        for &tid in &recent {
            if tid >= 0 && (tid as usize) < probs_vec.len() {
                probs_vec[tid as usize] /= rep_penalty;
            }
        }

        // 3-gram block: forbid any token that would complete an
        // already-seen 3-gram in the generated suffix.
        if tokens.len() >= 2 {
            let prev2 = tokens[tokens.len() - 2];
            let prev1 = tokens[tokens.len() - 1];
            let mut seen: HashSet<i64> = HashSet::new();
            for i in 0..tokens.len().saturating_sub(2) {
                if tokens[i] == prev2 && tokens[i + 1] == prev1 {
                    seen.insert(tokens[i + 2]);
                }
            }
            for &blocked in &seen {
                if blocked >= 0 && (blocked as usize) < probs_vec.len() {
                    probs_vec[blocked as usize] = 0.0;
                }
            }
        }

        let mut best_id: usize = 0;
        let mut best_p: f32 = f32::NEG_INFINITY;
        for (id, &p) in probs_vec.iter().enumerate() {
            if p > best_p {
                best_p = p;
                best_id = id;
            }
        }
        tokens.push(best_id as i64);
        if best_id as i64 == eos_id {
            break;
        }
    }
    tokens
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
