use std::{collections::HashSet, env, fs, process::ExitCode};

use adam_tokenizer::{SegmentationLexicon, SegmentationRuleSet, pretokenize};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct InputPack {
    version: String,
    name: String,
    target_language: String,
    script: String,
    samples: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct PretokenizedSample {
    id: String,
    text: String,
    pretokens: Vec<String>,
}

#[derive(Debug, Serialize)]
struct PretokenizedPack {
    version: String,
    name: String,
    target_language: String,
    script: String,
    source_pack: String,
    samples: Vec<PretokenizedSample>,
    total_token_count: usize,
    unique_token_count: usize,
    fallback_word_count: usize,
    fsm_segmented_word_count: usize,
}

fn main() -> ExitCode {
    let Some(input_path) = env::args().nth(1) else {
        eprintln!("usage: pretokenize_corpus <input_pack.json>");
        return ExitCode::FAILURE;
    };

    let input: InputPack = match load(&input_path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("cannot read input pack: {e}");
            return ExitCode::FAILURE;
        }
    };
    let lexicon: SegmentationLexicon = match load("data/tokenizer/segmentation_roots.json") {
        Ok(v) => v,
        Err(e) => {
            eprintln!("lexicon: {e}");
            return ExitCode::FAILURE;
        }
    };
    let rules: SegmentationRuleSet = match load("data/tokenizer/segmentation_rules.json") {
        Ok(v) => v,
        Err(e) => {
            eprintln!("rules: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mut out_samples = Vec::with_capacity(input.samples.len());
    let mut unique_tokens: HashSet<String> = HashSet::new();
    let mut total_tokens = 0usize;
    let mut fallback_words = 0usize;
    let mut segmented_words = 0usize;

    for sample in &input.samples {
        let id = sample
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let text = sample
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if text.is_empty() || id.is_empty() {
            continue;
        }

        // Count pre-FSM words vs post-tokenization to compute fallback rate
        let raw_word_count = text.split_whitespace().count();
        let pretokens = pretokenize(&text, &lexicon, &rules);
        let word_starts = pretokens.iter().filter(|t| t.starts_with('▁')).count();
        // Each whole-word fallback produces exactly one ▁-prefixed token.
        // FSM-segmented words produce one ▁-prefixed plus N-1 non-prefixed morphemes.
        // So raw_word_count == word_starts always (when pretokens is non-empty for that word).
        // To estimate fallback: words whose ▁-token corresponds to a whole word rather
        // than a segmented chunk — detect by running FSM per word again.
        for word in text.split_whitespace() {
            let core = word.trim_matches(|c: char| {
                c.is_ascii_punctuation()
                    || matches!(
                        c,
                        '«' | '»'
                            | '…'
                            | '—'
                            | '–'
                            | '\u{201C}'
                            | '\u{201D}'
                            | '\u{2018}'
                            | '\u{2019}'
                    )
            });
            if core.is_empty() {
                continue;
            }
            let core_lower = core.to_lowercase();
            if adam_tokenizer::deterministic_segment_token(&core_lower, &lexicon, &rules).is_some()
            {
                segmented_words += 1;
            } else {
                fallback_words += 1;
            }
        }

        total_tokens += pretokens.len();
        for t in &pretokens {
            unique_tokens.insert(t.clone());
        }
        let _ = raw_word_count;
        let _ = word_starts;

        out_samples.push(PretokenizedSample {
            id,
            text,
            pretokens,
        });
    }

    let pack = PretokenizedPack {
        version: input.version.clone(),
        name: "adam-pretokenized-corpus-pack".to_string(),
        target_language: input.target_language,
        script: input.script,
        source_pack: input.name,
        total_token_count: total_tokens,
        unique_token_count: unique_tokens.len(),
        fallback_word_count: fallback_words,
        fsm_segmented_word_count: segmented_words,
        samples: out_samples,
    };

    let total_words = pack.fallback_word_count + pack.fsm_segmented_word_count;
    eprintln!(
        "pretokenized {} samples: {} total tokens, {} unique; FSM coverage: {}/{} words ({:.2}%)",
        pack.samples.len(),
        pack.total_token_count,
        pack.unique_token_count,
        pack.fsm_segmented_word_count,
        total_words,
        if total_words == 0 {
            0.0
        } else {
            100.0 * pack.fsm_segmented_word_count as f64 / total_words as f64
        }
    );
    println!(
        "{}",
        serde_json::to_string_pretty(&pack).expect("serialize")
    );
    ExitCode::SUCCESS
}

fn load<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&contents)?)
}
