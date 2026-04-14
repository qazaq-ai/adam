use std::{env, fs, path::PathBuf, process::ExitCode, time::Instant};

use adam_tokenizer::{SegmentationLexicon, SegmentationRuleSet, bpe::BpeTokenizer};
use adam_train::model::{AdamBaseline, ModelConfig, default_device};
use candle_core::{DType, IndexOp, Tensor};
use candle_nn::{VarBuilder, VarMap};
use serde::Serialize;

const DEFAULT_CHECKPOINT: &str = "data/training/adam_baseline_checkpoint.safetensors";
const DEFAULT_REPORT: &str = "data/training/generation_showcase_report.json";
const LEXICON_PATH: &str = "data/tokenizer/segmentation_roots.json";
const RULES_PATH: &str = "data/tokenizer/segmentation_rules.json";
const VOCAB_PATH: &str = "data/tokenizer/bpe_vocab.json";
const MERGES_PATH: &str = "data/tokenizer/bpe_merges.json";

struct Prompt {
    id: &'static str,
    prompt: &'static str,
    category: &'static str,
}

const PROMPTS: &[Prompt] = &[
    Prompt {
        id: "p01",
        prompt: "жақсы адам",
        category: "Adj+N",
    },
    Prompt {
        id: "p02",
        prompt: "үлкен қала",
        category: "Adj+N",
    },
    Prompt {
        id: "p03",
        prompt: "бала мектепке",
        category: "N+N(dat)",
    },
    Prompt {
        id: "p04",
        prompt: "ол",
        category: "Pronoun 3sg",
    },
    Prompt {
        id: "p05",
        prompt: "олар",
        category: "Pronoun 3pl",
    },
    Prompt {
        id: "p06",
        prompt: "мен қазір",
        category: "Pron+Adv",
    },
    Prompt {
        id: "p07",
        prompt: "адам және",
        category: "N+conj",
    },
    Prompt {
        id: "p08",
        prompt: "екі адам",
        category: "Num+N",
    },
    Prompt {
        id: "p09",
        prompt: "үлкен жақсы адам",
        category: "Adj+Adj+N",
    },
    Prompt {
        id: "p10",
        prompt: "мектеп туралы",
        category: "N+postp",
    },
    Prompt {
        id: "p11",
        prompt: "жақсымын",
        category: "Predicative 1sg",
    },
    Prompt {
        id: "p12",
        prompt: "кітап",
        category: "N bare",
    },
    Prompt {
        id: "p13",
        prompt: "баланың кітабы",
        category: "N(gen)+N(poss)",
    },
    Prompt {
        id: "p14",
        prompt: "қалада",
        category: "N(loc)",
    },
    Prompt {
        id: "p15",
        prompt: "тез",
        category: "Adverb bare",
    },
    Prompt {
        id: "p16",
        prompt: "керек",
        category: "Modal bare",
    },
    Prompt {
        id: "p17",
        prompt: "мен",
        category: "Pronoun 1sg",
    },
    Prompt {
        id: "p18",
        prompt: "адамдар келді",
        category: "N(pl)+V(past)",
    },
    Prompt {
        id: "p19",
        prompt: "жаңа кітап",
        category: "Adj+N",
    },
    Prompt {
        id: "p20",
        prompt: "ол тез",
        category: "Pron+Adv",
    },
];

struct Config {
    name: &'static str,
    temperature: f32,
    top_k: usize,
    top_p: f32,
    rep_pen: f32,
}

const CONFIGS: &[Config] = &[
    Config {
        name: "greedy",
        temperature: 0.0,
        top_k: 0,
        top_p: 0.0,
        rep_pen: 1.0,
    },
    Config {
        name: "temp_0.8",
        temperature: 0.8,
        top_k: 0,
        top_p: 0.0,
        rep_pen: 1.0,
    },
    Config {
        name: "nucleus_p0.9_rp1.2",
        temperature: 1.0,
        top_k: 0,
        top_p: 0.9,
        rep_pen: 1.2,
    },
];

const MAX_NEW_TOKENS: usize = 24;
const SEED: u64 = 42;

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

fn sample_next(logits: &[f32], history: &[u32], cfg: &Config, rng: &mut Lcg) -> u32 {
    let mut logits: Vec<f32> = logits.to_vec();

    if cfg.rep_pen > 1.0 && !history.is_empty() {
        for &tok in history {
            if let Some(l) = logits.get_mut(tok as usize) {
                if *l > 0.0 {
                    *l /= cfg.rep_pen;
                } else {
                    *l *= cfg.rep_pen;
                }
            }
        }
    }

    if cfg.temperature <= 0.0 {
        let (idx, _) =
            logits
                .iter()
                .enumerate()
                .fold((0usize, f32::NEG_INFINITY), |acc, (i, v)| {
                    if *v > acc.1 { (i, *v) } else { acc }
                });
        return idx as u32;
    }

    for l in logits.iter_mut() {
        *l /= cfg.temperature;
    }

    let max = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let exp_sum: f32 = logits.iter().map(|l| (l - max).exp()).sum();
    let mut idx_probs: Vec<(usize, f32)> = logits
        .iter()
        .enumerate()
        .map(|(i, l)| (i, (l - max).exp() / exp_sum))
        .collect();
    idx_probs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    if cfg.top_p > 0.0 && cfg.top_p < 1.0 {
        let mut cum = 0.0_f32;
        let mut cutoff = idx_probs.len();
        for (i, (_, p)) in idx_probs.iter().enumerate() {
            cum += *p;
            if cum >= cfg.top_p {
                cutoff = i + 1;
                break;
            }
        }
        idx_probs.truncate(cutoff);
    }

    if cfg.top_k > 0 && cfg.top_k < idx_probs.len() {
        idx_probs.truncate(cfg.top_k);
    }

    let total: f32 = idx_probs.iter().map(|(_, p)| *p).sum();
    if total <= 0.0 {
        return idx_probs.first().map(|(i, _)| *i as u32).unwrap_or(0);
    }
    let u = rng.unit() * total;
    let mut acc = 0.0_f32;
    for (i, p) in &idx_probs {
        acc += p;
        if u <= acc {
            return *i as u32;
        }
    }
    idx_probs.last().map(|(i, _)| *i as u32).unwrap_or(0)
}

#[derive(Debug, Serialize)]
struct GenerationEntry {
    prompt_id: String,
    prompt: String,
    category: String,
    config: String,
    temperature: f32,
    top_k: usize,
    top_p: f32,
    repetition_penalty: f32,
    output_text: String,
    generated_token_count: usize,
    hit_eos: bool,
    wall_ms: u128,
}

#[derive(Debug, Serialize)]
struct ShowcaseReport {
    version: String,
    run_name: String,
    checkpoint_path: String,
    model_param_count: usize,
    vocab_size: usize,
    prompt_count: usize,
    config_count: usize,
    generation_count: usize,
    total_wall_ms: u128,
    seed: u64,
    max_new_tokens: usize,
    generations: Vec<GenerationEntry>,
}

fn main() -> ExitCode {
    let checkpoint_path: PathBuf = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CHECKPOINT));
    let report_path: PathBuf = env::args()
        .nth(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_REPORT));

    let device = match default_device() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("device: {e}");
            return ExitCode::FAILURE;
        }
    };

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
    if let Err(e) = varmap.load(&checkpoint_path) {
        eprintln!("checkpoint load: {e}");
        return ExitCode::FAILURE;
    }
    let param_count: usize = varmap.all_vars().iter().map(|v| v.elem_count()).sum();

    eprintln!(
        "showcase: {} prompts × {} configs = {} generations; model={} params, vocab={}",
        PROMPTS.len(),
        CONFIGS.len(),
        PROMPTS.len() * CONFIGS.len(),
        param_count,
        bpe.vocab_size()
    );

    let mut generations: Vec<GenerationEntry> = Vec::with_capacity(PROMPTS.len() * CONFIGS.len());
    let total_start = Instant::now();

    for prompt in PROMPTS {
        for sampling in CONFIGS {
            let start = Instant::now();
            let mut rng = Lcg(SEED);

            let mut sequence: Vec<u32> = vec![bpe.bos_id];
            sequence.extend(bpe.encode(prompt.prompt, &lexicon, &rules));

            let mut generated: Vec<u32> = Vec::new();
            let mut hit_eos = false;

            for _ in 0..MAX_NEW_TOKENS {
                if sequence.len() >= cfg.max_seq_len {
                    break;
                }
                let t = sequence.len();
                let input = match Tensor::from_vec(sequence.clone(), (1, t), &device) {
                    Ok(x) => x,
                    Err(e) => {
                        eprintln!("tensor: {e}");
                        return ExitCode::FAILURE;
                    }
                };
                let logits = match model.forward(&input, false) {
                    Ok(l) => l,
                    Err(e) => {
                        eprintln!("forward: {e}");
                        return ExitCode::FAILURE;
                    }
                };
                let last = match logits.i((0, t - 1, ..)) {
                    Ok(x) => x,
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
                let next = sample_next(&last_host, &sequence, sampling, &mut rng);
                if next == bpe.eos_id {
                    hit_eos = true;
                    break;
                }
                sequence.push(next);
                generated.push(next);
            }

            let output_text = bpe.decode(&sequence);
            let wall_ms = start.elapsed().as_millis();

            generations.push(GenerationEntry {
                prompt_id: prompt.id.to_string(),
                prompt: prompt.prompt.to_string(),
                category: prompt.category.to_string(),
                config: sampling.name.to_string(),
                temperature: sampling.temperature,
                top_k: sampling.top_k,
                top_p: sampling.top_p,
                repetition_penalty: sampling.rep_pen,
                output_text,
                generated_token_count: generated.len(),
                hit_eos,
                wall_ms,
            });
        }
    }

    let total_wall_ms = total_start.elapsed().as_millis();

    let report = ShowcaseReport {
        version: env!("CARGO_PKG_VERSION").to_string(),
        run_name: "adam-generation-showcase".to_string(),
        checkpoint_path: checkpoint_path.display().to_string(),
        model_param_count: param_count,
        vocab_size: bpe.vocab_size(),
        prompt_count: PROMPTS.len(),
        config_count: CONFIGS.len(),
        generation_count: generations.len(),
        total_wall_ms,
        seed: SEED,
        max_new_tokens: MAX_NEW_TOKENS,
        generations,
    };

    if let Some(parent) = report_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            let _ = fs::create_dir_all(parent);
        }
    }
    if let Err(e) = fs::write(
        &report_path,
        serde_json::to_string_pretty(&report).expect("serialize"),
    ) {
        eprintln!("cannot write report: {e}");
        return ExitCode::FAILURE;
    }

    eprintln!(
        "wrote {} ({} generations in {:.1}s)",
        report_path.display(),
        report.generation_count,
        total_wall_ms as f64 / 1000.0
    );
    ExitCode::SUCCESS
}

fn load_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&contents)?)
}
