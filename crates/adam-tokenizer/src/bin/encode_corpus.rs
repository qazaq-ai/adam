use std::{env, fs, process::ExitCode};

use adam_tokenizer::{SegmentationLexicon, SegmentationRuleSet, bpe::BpeTokenizer};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct InputSample {
    id: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct InputPack {
    #[allow(dead_code)]
    version: String,
    #[allow(dead_code)]
    name: String,
    samples: Vec<InputSample>,
}

#[derive(Debug, Serialize)]
struct EncodedSample {
    id: String,
    text: String,
    ids: Vec<u32>,
    id_count: usize,
}

#[derive(Debug, Serialize)]
struct EncodedPack {
    version: String,
    name: String,
    target_language: String,
    script: String,
    vocab_size: usize,
    pad_id: u32,
    bos_id: u32,
    eos_id: u32,
    unk_id: u32,
    total_id_count: usize,
    unk_id_count: usize,
    sample_count: usize,
    roundtrip_exact_match_count: usize,
    samples: Vec<EncodedSample>,
}

fn main() -> ExitCode {
    let Some(input_path) = env::args().nth(1) else {
        eprintln!(
            "usage: encode_corpus <pack.json> [<vocab=data/tokenizer/bpe_vocab.json>] \
             [<merges=data/tokenizer/bpe_merges.json>]"
        );
        return ExitCode::FAILURE;
    };
    let vocab_path = env::args()
        .nth(2)
        .unwrap_or_else(|| "data/tokenizer/bpe_vocab.json".to_string());
    let merges_path = env::args()
        .nth(3)
        .unwrap_or_else(|| "data/tokenizer/bpe_merges.json".to_string());

    let pack: InputPack = match load(&input_path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("cannot read pack: {e}");
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
    let bpe = match BpeTokenizer::load(&vocab_path, &merges_path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("bpe: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mut samples: Vec<EncodedSample> = Vec::with_capacity(pack.samples.len());
    let mut total_ids = 0usize;
    let mut unk_count = 0usize;
    let mut roundtrip_ok = 0usize;

    for s in &pack.samples {
        // Wrap each sample with <bos> ... <eos> for LM training.
        let inner_ids = bpe.encode(&s.text, &lexicon, &rules);
        let mut ids: Vec<u32> = Vec::with_capacity(inner_ids.len() + 2);
        ids.push(bpe.bos_id);
        ids.extend_from_slice(&inner_ids);
        ids.push(bpe.eos_id);

        total_ids += ids.len();
        unk_count += inner_ids.iter().filter(|id| **id == bpe.unk_id).count();

        // Round-trip check: decode(encode(x)) should equal pretokenize+lowercased concat.
        let decoded = bpe.decode(&ids);
        let lowered = s.text.to_lowercase();
        // Normalize whitespace in lowered — pretokenize loses extra whitespace, and
        // decode inserts a single space before each ▁ token. Compare via a stripped
        // lowercase form.
        let expected = lowered.split_whitespace().collect::<Vec<_>>().join(" ");
        let actual = decoded.split_whitespace().collect::<Vec<_>>().join(" ");
        if expected == actual {
            roundtrip_ok += 1;
        }

        samples.push(EncodedSample {
            id: s.id.clone(),
            text: s.text.clone(),
            id_count: ids.len(),
            ids,
        });
    }

    let sample_count = samples.len();
    let out = EncodedPack {
        version: env!("CARGO_PKG_VERSION").to_string(),
        name: "adam-training-ids-pack".to_string(),
        target_language: "kazakh".to_string(),
        script: "cyrillic".to_string(),
        vocab_size: bpe.vocab_size(),
        pad_id: bpe.pad_id,
        bos_id: bpe.bos_id,
        eos_id: bpe.eos_id,
        unk_id: bpe.unk_id,
        total_id_count: total_ids,
        unk_id_count: unk_count,
        sample_count,
        roundtrip_exact_match_count: roundtrip_ok,
        samples,
    };

    eprintln!(
        "encoded {} samples: {} total ids, {} unknowns ({:.2}%), {} roundtrip OK ({:.2}%)",
        out.sample_count,
        out.total_id_count,
        out.unk_id_count,
        100.0 * out.unk_id_count as f64 / out.total_id_count as f64,
        out.roundtrip_exact_match_count,
        100.0 * out.roundtrip_exact_match_count as f64 / out.sample_count as f64,
    );
    println!("{}", serde_json::to_string_pretty(&out).expect("serialize"));
    ExitCode::SUCCESS
}

fn load<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&contents)?)
}
