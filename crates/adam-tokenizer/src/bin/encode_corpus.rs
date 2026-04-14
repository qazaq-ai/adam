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
             [<merges=data/tokenizer/bpe_merges.json>] [<val_out=>] [<val_fraction=0.05>]\n\
             \n\
             If val_out is empty or omitted, encodes all samples and writes the single\n\
             training pack to stdout. If val_out is a path, deterministically splits\n\
             samples by hash(id): the smaller fraction becomes the validation pack\n\
             written to val_out, the rest is the training pack written to stdout."
        );
        return ExitCode::FAILURE;
    };
    let vocab_path = env::args()
        .nth(2)
        .unwrap_or_else(|| "data/tokenizer/bpe_vocab.json".to_string());
    let merges_path = env::args()
        .nth(3)
        .unwrap_or_else(|| "data/tokenizer/bpe_merges.json".to_string());
    let val_out: Option<String> = env::args().nth(4).filter(|s| !s.is_empty());
    let val_fraction: f64 = env::args()
        .nth(5)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.05);

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

    let mut train_samples: Vec<EncodedSample> = Vec::new();
    let mut val_samples: Vec<EncodedSample> = Vec::new();
    let mut train_total_ids = 0usize;
    let mut val_total_ids = 0usize;
    let mut train_unk = 0usize;
    let mut val_unk = 0usize;
    let mut train_roundtrip = 0usize;
    let mut val_roundtrip = 0usize;
    let val_cutoff: u64 = (val_fraction.clamp(0.0, 1.0) * 10_000.0) as u64;

    for s in &pack.samples {
        let inner_ids = bpe.encode(&s.text, &lexicon, &rules);
        let mut ids: Vec<u32> = Vec::with_capacity(inner_ids.len() + 2);
        ids.push(bpe.bos_id);
        ids.extend_from_slice(&inner_ids);
        ids.push(bpe.eos_id);

        let sample_unk = inner_ids.iter().filter(|id| **id == bpe.unk_id).count();

        let decoded = bpe.decode(&ids);
        let lowered = s.text.to_lowercase();
        let expected = lowered.split_whitespace().collect::<Vec<_>>().join(" ");
        let actual = decoded.split_whitespace().collect::<Vec<_>>().join(" ");
        let roundtrip = expected == actual;

        let encoded = EncodedSample {
            id: s.id.clone(),
            text: s.text.clone(),
            id_count: ids.len(),
            ids,
        };

        let is_val = val_out.is_some() && stable_hash(&encoded.id) % 10_000 < val_cutoff;
        if is_val {
            val_total_ids += encoded.id_count;
            val_unk += sample_unk;
            if roundtrip {
                val_roundtrip += 1;
            }
            val_samples.push(encoded);
        } else {
            train_total_ids += encoded.id_count;
            train_unk += sample_unk;
            if roundtrip {
                train_roundtrip += 1;
            }
            train_samples.push(encoded);
        }
    }

    let train_pack = EncodedPack {
        version: env!("CARGO_PKG_VERSION").to_string(),
        name: "adam-training-ids-pack".to_string(),
        target_language: "kazakh".to_string(),
        script: "cyrillic".to_string(),
        vocab_size: bpe.vocab_size(),
        pad_id: bpe.pad_id,
        bos_id: bpe.bos_id,
        eos_id: bpe.eos_id,
        unk_id: bpe.unk_id,
        total_id_count: train_total_ids,
        unk_id_count: train_unk,
        sample_count: train_samples.len(),
        roundtrip_exact_match_count: train_roundtrip,
        samples: train_samples,
    };

    if let Some(val_path) = val_out {
        let val_pack = EncodedPack {
            version: env!("CARGO_PKG_VERSION").to_string(),
            name: "adam-validation-ids-pack".to_string(),
            target_language: "kazakh".to_string(),
            script: "cyrillic".to_string(),
            vocab_size: bpe.vocab_size(),
            pad_id: bpe.pad_id,
            bos_id: bpe.bos_id,
            eos_id: bpe.eos_id,
            unk_id: bpe.unk_id,
            total_id_count: val_total_ids,
            unk_id_count: val_unk,
            sample_count: val_samples.len(),
            roundtrip_exact_match_count: val_roundtrip,
            samples: val_samples,
        };
        if let Err(e) = fs::write(
            &val_path,
            serde_json::to_string_pretty(&val_pack).expect("serialize val"),
        ) {
            eprintln!("cannot write val pack to {}: {}", val_path, e);
            return ExitCode::FAILURE;
        }
        eprintln!(
            "split: train={} samples ({} ids), val={} samples ({} ids) → {}",
            train_pack.sample_count,
            train_pack.total_id_count,
            val_pack.sample_count,
            val_pack.total_id_count,
            val_path
        );
    }

    eprintln!(
        "train: {} samples, {} total ids, {} unknowns ({:.2}%), {} roundtrip OK ({:.2}%)",
        train_pack.sample_count,
        train_pack.total_id_count,
        train_pack.unk_id_count,
        100.0 * train_pack.unk_id_count as f64 / train_pack.total_id_count.max(1) as f64,
        train_pack.roundtrip_exact_match_count,
        100.0 * train_pack.roundtrip_exact_match_count as f64
            / train_pack.sample_count.max(1) as f64,
    );
    println!(
        "{}",
        serde_json::to_string_pretty(&train_pack).expect("serialize train")
    );
    ExitCode::SUCCESS
}

fn stable_hash(s: &str) -> u64 {
    // Simple FNV-1a-style stable hash (not cryptographic, just deterministic).
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

fn load<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&contents)?)
}
