// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! **v6.6 generative pivot (2026-06-11)** — does the contextual LM
//! correctly distinguish a valid Kazakh sentence from the rewrite
//! the lexicon_validator was producing?
//!
//! For each rc25-audit case, we have two strings:
//!  * `original` — what Whisper produced (already valid Kazakh)
//!  * `rewrite`  — what the validator turned it into (broken)
//!
//! A correct LM rescore picks `original > rewrite` per the runtime
//! contract in neural_rescorer.rs: "If fuzzy's rewrite has a LOWER
//! score (= the rewrite is less plausible than the original), the
//! REPL reverts to the original."
//!
//! Mixed-class controls: a few cases where the validator was
//! actually CORRECT (legitimate Whisper drift repair). The LM
//! should pick rewrite > original on those.
//!
//! Run:
//!   cargo run --release -p adam-voice-repl-v6-3 --bin lm_rescore_eval
//!
//! Swap checkpoint dirs (data/checkpoints/contextual_lm) to A/B
//! the baseline against the v6.6 LM trained on 18.7M tokens.

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

    /// Per-token average log-likelihood. Higher (closer to 0) = more
    /// like training-distribution Kazakh.
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

    /// Same as `score_text` but also returns total log-prob and the
    /// number of sub-word tokens for the per-token vs per-char debate.
    fn score_detail(&self, text: &str) -> (f32, f32, usize) {
        let mut ids: Vec<i64> = vec![self.bpe.bos_id as i64];
        for id in self.bpe.encode(text, &self.lexicon, &self.rules) {
            ids.push(id as i64);
        }
        ids.push(self.bpe.eos_id as i64);
        let n = ids.len();
        match score_sequence(&self.model, &ids, &self.device) {
            Some(r) => (r.total_log_prob, r.per_token, n),
            None => (f32::NEG_INFINITY, f32::NEG_INFINITY, n),
        }
    }

    /// Dump the BPE token sequence (incl. ▁ markers) so we can see
    /// exactly how the encoder dropped the input into sub-words.
    fn tokenize_str(&self, text: &str) -> Vec<String> {
        let mut out = Vec::new();
        for id in self.bpe.encode(text, &self.lexicon, &self.rules) {
            out.push(self.bpe.id_to_token(id).unwrap_or("?").to_string());
        }
        out
    }
}

/// (original_whisper, validator_rewrite, expected_winner, label)
fn cases() -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    vec![
        // === validator FALSE positives (rc25 audit — wrong) ===
        (
            "Менің атым — дәулет.",
            "Менің атым — сәулет.",
            "original",
            "FP: дәулет (name)",
        ),
        (
            "Мен қостанайда тұрамын.",
            "Мен қостанайда тұратын.",
            "original",
            "FP: тұрамын",
        ),
        (
            "Жасым алпыс алтыға толды.",
            "Жасы алпыс алтыға толы.",
            "original",
            "FP: Жасым+толды",
        ),
        (
            "Мен айттым ғой, қостанайда тұрам деп.",
            "Мен айтты ғой, қостанай тұра деп.",
            "original",
            "FP: айттым+тұрам",
        ),
        (
            "Судың формулусын жазып берші.",
            "Судың формулусын жазып бері.",
            "original",
            "FP: берші",
        ),
        // === validator TRUE positives (real Whisper drift repair) ===
        (
            "Қалыңғыз қалай.",
            "Қалыңыз қалай.",
            "rewrite",
            "TP: Қалыңғыз→Қалыңыз",
        ),
        (
            "Рақмет, алаған шүкір.",
            "Рақмет, алған шүкір.",
            "rewrite",
            "TP: алаған→алған",
        ),
        (
            "Менің есімім даулет.",
            "Менің есімім дәулет.",
            "rewrite",
            "TP: даулет→дәулет",
        ),
        (
            "Он дареже үш.",
            "Он дәреже үш.",
            "rewrite",
            "TP: дареже→дәреже",
        ),
        // === controls: clearly-valid original ===
        (
            "Сәлеметсіз бе?",
            "Сәлемсіз бе?",
            "original",
            "Ctrl: greeting valid",
        ),
        (
            "Мен Қостанайда тұрамын.",
            "Мен Қостанайда тұратын.",
            "original",
            "Ctrl: loc valid",
        ),
        (
            "Қазір сағат неше?",
            "Қазір сағат нешеу?",
            "original",
            "Ctrl: time valid",
        ),
    ]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rescorer = Rescorer::load()?;
    let cases = cases();

    // Quick BPE tokenization dump for the trouble words.
    let trouble = [
        "Менің атым — дәулет.",
        "Менің атым — сәулет.",
        "Менің есімім даулет.",
        "Менің есімім дәулет.",
        "Он дәреже үш.",
        "Он дареже үш.",
        "Рақмет, алаған шүкір.",
        "Рақмет, алған шүкір.",
    ];
    println!("{:=^120}", " BPE tokenisation trace ");
    for t in &trouble {
        let toks = rescorer.tokenize_str(t);
        let (total, per_tok, n) = rescorer.score_detail(t);
        println!(
            "  «{}»\n     tokens ({}): {}\n     total={:+.3}  per_token={:+.3}  n_bpe={}",
            t,
            toks.len(),
            toks.iter()
                .map(|s| format!("«{}»", s))
                .collect::<Vec<_>>()
                .join(" "),
            total,
            per_tok,
            n,
        );
    }
    println!();

    println!("{:=^120}", " LM rescore eval ");
    println!(
        "{:<6} {:<10} {:<38} {:<38} {:>9} {:>9}",
        "ok", "expected", "original (raw)", "rewrite (validator)", "score_o", "score_r"
    );
    println!("{:-^120}", "");

    let mut correct = 0;
    let (mut fp_c, mut fp_t) = (0, 0);
    let (mut tp_c, mut tp_t) = (0, 0);
    let (mut c_c, mut c_t) = (0, 0);

    for (orig, rewr, expect, label) in &cases {
        let so = rescorer.score_text(orig);
        let sr = rescorer.score_text(rewr);
        let winner = if so > sr { "original" } else { "rewrite" };
        let ok = winner == *expect;
        if ok {
            correct += 1;
        }
        match &label[..2] {
            "FP" => {
                fp_t += 1;
                if ok {
                    fp_c += 1;
                }
            }
            "TP" => {
                tp_t += 1;
                if ok {
                    tp_c += 1;
                }
            }
            _ => {
                c_t += 1;
                if ok {
                    c_c += 1;
                }
            }
        }
        println!(
            "{:<6} {:<10} {:<38} {:<38} {:>+9.3} {:>+9.3}  [{}]",
            if ok { "✓" } else { "✗" },
            expect,
            orig,
            rewr,
            so,
            sr,
            label,
        );
    }
    println!("{:-^120}", "");
    println!(
        "FP (validator-wrong, LM picks original): {}/{} ({:.0}%)",
        fp_c,
        fp_t,
        100.0 * fp_c as f32 / fp_t as f32
    );
    println!(
        "TP (validator-right, LM picks rewrite):  {}/{} ({:.0}%)",
        tp_c,
        tp_t,
        100.0 * tp_c as f32 / tp_t as f32
    );
    println!(
        "Ctrl:                                    {}/{} ({:.0}%)",
        c_c,
        c_t,
        100.0 * c_c as f32 / c_t as f32
    );
    println!(
        "Total:                                   {}/{} ({:.0}%)",
        correct,
        cases.len(),
        100.0 * correct as f32 / cases.len() as f32
    );
    Ok(())
}
