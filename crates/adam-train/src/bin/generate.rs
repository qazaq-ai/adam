use std::{env, path::PathBuf, process::ExitCode};

use adam_tokenizer::{SegmentationLexicon, SegmentationRuleSet, bpe::BpeTokenizer};
use adam_train::model::{AdamBaseline, ModelConfig, default_device};
use candle_core::{DType, IndexOp, Tensor};
use candle_nn::{VarBuilder, VarMap};

const DEFAULT_CHECKPOINT: &str = "data/training/adam_baseline_checkpoint.safetensors";
const LEXICON_PATH: &str = "data/tokenizer/segmentation_roots.json";
const RULES_PATH: &str = "data/tokenizer/segmentation_rules.json";
const VOCAB_PATH: &str = "data/tokenizer/bpe_vocab.json";
const MERGES_PATH: &str = "data/tokenizer/bpe_merges.json";

struct Args {
    prompt: String,
    max_new_tokens: usize,
    temperature: f32,
    top_k: usize,
    seed: u64,
    checkpoint: PathBuf,
}

fn parse_args() -> Result<Args, String> {
    let mut iter = env::args().skip(1);
    let prompt = iter.next().ok_or_else(|| {
        "usage: generate <prompt> [max_new_tokens=32] [temperature=0.0] [top_k=0] \
         [seed=42] [checkpoint=data/training/adam_baseline_checkpoint.safetensors]"
            .to_string()
    })?;
    let max_new_tokens = iter
        .next()
        .map(|s| s.parse())
        .transpose()
        .map_err(|e: std::num::ParseIntError| format!("max_new_tokens: {e}"))?
        .unwrap_or(32);
    let temperature = iter
        .next()
        .map(|s| s.parse())
        .transpose()
        .map_err(|e: std::num::ParseFloatError| format!("temperature: {e}"))?
        .unwrap_or(0.0_f32);
    let top_k = iter
        .next()
        .map(|s| s.parse())
        .transpose()
        .map_err(|e: std::num::ParseIntError| format!("top_k: {e}"))?
        .unwrap_or(0);
    let seed = iter
        .next()
        .map(|s| s.parse())
        .transpose()
        .map_err(|e: std::num::ParseIntError| format!("seed: {e}"))?
        .unwrap_or(42);
    let checkpoint = iter
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CHECKPOINT));
    Ok(Args {
        prompt,
        max_new_tokens,
        temperature,
        top_k,
        seed,
        checkpoint,
    })
}

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }
    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
}

fn sample_next(logits: &[f32], temperature: f32, top_k: usize, rng: &mut Lcg) -> u32 {
    if temperature <= 0.0 {
        let (idx, _) =
            logits
                .iter()
                .enumerate()
                .fold((0usize, f32::NEG_INFINITY), |acc, (i, v)| {
                    if *v > acc.1 { (i, *v) } else { acc }
                });
        return idx as u32;
    }

    // Temperature scaling
    let scaled: Vec<f32> = logits.iter().map(|l| l / temperature).collect();

    // Optional top-k truncation
    let mut filtered: Vec<(usize, f32)> = scaled.iter().enumerate().map(|(i, v)| (i, *v)).collect();
    if top_k > 0 && top_k < filtered.len() {
        filtered.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        filtered.truncate(top_k);
    }

    // Softmax
    let max = filtered
        .iter()
        .map(|(_, v)| *v)
        .fold(f32::NEG_INFINITY, f32::max);
    let exp_sum: f32 = filtered.iter().map(|(_, v)| (v - max).exp()).sum();
    let probs: Vec<(usize, f32)> = filtered
        .iter()
        .map(|(i, v)| (*i, (v - max).exp() / exp_sum))
        .collect();

    let u = rng.unit();
    let mut acc = 0.0_f32;
    for (i, p) in &probs {
        acc += p;
        if u <= acc {
            return *i as u32;
        }
    }
    probs.last().map(|(i, _)| *i as u32).unwrap_or(0)
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    let device = match default_device() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("device: {e}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!("device: {:?}", device);

    let lexicon: SegmentationLexicon = match load_json(LEXICON_PATH) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("lexicon: {e}");
            return ExitCode::FAILURE;
        }
    };
    let rules: SegmentationRuleSet = match load_json(RULES_PATH) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("rules: {e}");
            return ExitCode::FAILURE;
        }
    };
    let bpe = match BpeTokenizer::load(VOCAB_PATH, MERGES_PATH) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("bpe: {e}");
            return ExitCode::FAILURE;
        }
    };

    let cfg = ModelConfig::tiny();
    let mut varmap = VarMap::new();
    let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);
    let model = match AdamBaseline::new(&cfg, vb) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("model init: {e}");
            return ExitCode::FAILURE;
        }
    };

    if let Err(e) = varmap.load(&args.checkpoint) {
        eprintln!("checkpoint load: {e}");
        return ExitCode::FAILURE;
    }
    eprintln!("checkpoint loaded: {}", args.checkpoint.display());

    // Encode prompt, prepend <bos>.
    let mut sequence: Vec<u32> = vec![bpe.bos_id];
    sequence.extend(bpe.encode(&args.prompt, &lexicon, &rules));

    eprintln!(
        "prompt: {:?}\nprompt ids ({}): {:?}",
        args.prompt,
        sequence.len(),
        sequence
    );

    let mut rng = Lcg(args.seed);
    let mut generated: Vec<u32> = Vec::new();
    let mut generated_eos = false;

    for _ in 0..args.max_new_tokens {
        if sequence.len() >= cfg.max_seq_len {
            break;
        }
        let seq_ids: Vec<u32> = sequence.clone();
        let seq_len = seq_ids.len();
        let input = match Tensor::from_vec(seq_ids, (1, seq_len), &device) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("tensor build: {e}");
                return ExitCode::FAILURE;
            }
        };
        let logits = match model.forward(&input) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("forward: {e}");
                return ExitCode::FAILURE;
            }
        };
        let last = match logits.i((0, seq_len - 1, ..)) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("slice: {e}");
                return ExitCode::FAILURE;
            }
        };
        let last_host: Vec<f32> = match last.to_vec1() {
            Ok(v) => v,
            Err(e) => {
                eprintln!("to_vec1: {e}");
                return ExitCode::FAILURE;
            }
        };
        let next = sample_next(&last_host, args.temperature, args.top_k, &mut rng);
        if next == bpe.eos_id {
            generated_eos = true;
            break;
        }
        sequence.push(next);
        generated.push(next);
    }

    let full_decoded = bpe.decode(&sequence);
    let only_generated = bpe.decode(&generated);

    eprintln!(
        "generated {} tokens, eos={}, final_seq_len={}",
        generated.len(),
        generated_eos,
        sequence.len()
    );
    eprintln!("generated ids: {:?}", generated);
    eprintln!("generated text only: {:?}", only_generated);
    println!("{}", full_decoded);
    ExitCode::SUCCESS
}

fn load_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&contents)?)
}
