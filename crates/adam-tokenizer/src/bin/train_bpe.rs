use std::{
    collections::{HashMap, HashSet},
    env, fs,
    process::ExitCode,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct PretokSample {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    text: String,
    pretokens: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PretokPack {
    #[allow(dead_code)]
    version: String,
    samples: Vec<PretokSample>,
}

#[derive(Debug, Serialize)]
struct VocabEntry {
    id: u32,
    token: String,
}

#[derive(Debug, Serialize)]
struct VocabFile {
    version: String,
    name: String,
    target_language: String,
    script: String,
    vocab_size: usize,
    special_tokens: Vec<String>,
    training_merge_count: usize,
    training_corpus_sentence_count: usize,
    training_corpus_initial_token_count: usize,
    vocab: Vec<VocabEntry>,
}

#[derive(Debug, Serialize)]
struct MergeRecord {
    rank: usize,
    left: String,
    right: String,
    merged: String,
    frequency: usize,
}

#[derive(Debug, Serialize)]
struct MergesFile {
    version: String,
    name: String,
    target_language: String,
    script: String,
    merge_count: usize,
    merges: Vec<MergeRecord>,
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(input_path) = args.next() else {
        eprintln!(
            "usage: train_bpe <pretokenized_pack.json> [<target_vocab_size=4096>] \
             [<vocab_out>] [<merges_out>]"
        );
        return ExitCode::FAILURE;
    };
    let target_vocab: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(4096);
    let vocab_out = args
        .next()
        .unwrap_or_else(|| "data/tokenizer/bpe_vocab.json".to_string());
    let merges_out = args
        .next()
        .unwrap_or_else(|| "data/tokenizer/bpe_merges.json".to_string());

    let raw = match fs::read_to_string(&input_path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("cannot read input: {e}");
            return ExitCode::FAILURE;
        }
    };
    let pack: PretokPack = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("invalid pretokenized pack: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Load corpus as Vec<Vec<String>>; each sentence is a list of token strings.
    let mut corpus: Vec<Vec<String>> = pack
        .samples
        .into_iter()
        .map(|s| s.pretokens)
        .filter(|v| !v.is_empty())
        .collect();

    let sentence_count = corpus.len();
    let initial_tokens: usize = corpus.iter().map(|s| s.len()).sum();

    // Build initial vocabulary from unique pretokens.
    let mut base_tokens: HashSet<String> = HashSet::new();
    for sentence in &corpus {
        for t in sentence {
            base_tokens.insert(t.clone());
        }
    }
    let mut vocab_tokens: Vec<String> = base_tokens.into_iter().collect();
    vocab_tokens.sort();

    // Reserve space for special tokens at the start of vocab numbering.
    let special_tokens = [
        "<pad>".to_string(),
        "<bos>".to_string(),
        "<eos>".to_string(),
        "<unk>".to_string(),
    ];
    let specials_set: HashSet<String> = special_tokens.iter().cloned().collect();
    vocab_tokens.retain(|t| !specials_set.contains(t));

    // Target merge count = target_vocab - specials - base vocab size.
    let base_size = vocab_tokens.len() + special_tokens.len();
    if target_vocab <= base_size {
        eprintln!(
            "target_vocab {} <= base_size {} (specials {} + unique pretokens {}); nothing to \
             learn",
            target_vocab,
            base_size,
            special_tokens.len(),
            vocab_tokens.len()
        );
    }
    let target_merges = target_vocab.saturating_sub(base_size);

    eprintln!(
        "training BPE: {} sentences, {} initial tokens, {} unique base tokens, target_vocab={}, \
         planned merges={}",
        sentence_count,
        initial_tokens,
        vocab_tokens.len(),
        target_vocab,
        target_merges
    );

    let mut merges: Vec<MergeRecord> = Vec::with_capacity(target_merges);

    for rank in 1..=target_merges {
        // Count adjacent pairs across the corpus, skipping pairs whose right
        // token starts with ▁ (word boundary) — we only merge within words.
        let mut pair_counts: HashMap<(String, String), usize> = HashMap::new();
        for sentence in &corpus {
            for pair in sentence.windows(2) {
                let left = &pair[0];
                let right = &pair[1];
                if right.starts_with('\u{2581}') {
                    continue;
                }
                let key = (left.clone(), right.clone());
                *pair_counts.entry(key).or_default() += 1;
            }
        }

        // Find the most frequent pair; tie-break deterministically by lexicographic order.
        let Some(((best_left, best_right), best_count)) = pair_counts
            .into_iter()
            .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        else {
            eprintln!("no more pairs to merge at rank {rank}; stopping early");
            break;
        };

        if best_count < 2 {
            eprintln!(
                "best pair frequency dropped to {} at rank {}; stopping early",
                best_count, rank
            );
            break;
        }

        let merged = format!("{}{}", best_left, best_right);
        merges.push(MergeRecord {
            rank,
            left: best_left.clone(),
            right: best_right.clone(),
            merged: merged.clone(),
            frequency: best_count,
        });
        vocab_tokens.push(merged.clone());

        // Apply the merge to the corpus.
        for sentence in &mut corpus {
            if sentence.len() < 2 {
                continue;
            }
            let mut i = 0;
            let mut new_sentence: Vec<String> = Vec::with_capacity(sentence.len());
            while i < sentence.len() {
                if i + 1 < sentence.len()
                    && sentence[i] == best_left
                    && sentence[i + 1] == best_right
                {
                    new_sentence.push(merged.clone());
                    i += 2;
                } else {
                    new_sentence.push(sentence[i].clone());
                    i += 1;
                }
            }
            *sentence = new_sentence;
        }

        if rank % 200 == 0 {
            eprintln!(
                "  rank {} / {}: merged '{}'+'{}' -> '{}' (freq {})",
                rank, target_merges, best_left, best_right, merged, best_count
            );
        }
    }

    let final_corpus_tokens: usize = corpus.iter().map(|s| s.len()).sum();
    let compression_ratio = if initial_tokens == 0 {
        0.0
    } else {
        initial_tokens as f64 / final_corpus_tokens as f64
    };

    eprintln!(
        "trained {} merges; corpus compressed from {} to {} tokens ({}×)",
        merges.len(),
        initial_tokens,
        final_corpus_tokens,
        format!("{:.2}", compression_ratio)
    );

    // Build final vocab: specials first, then base tokens, then merges in order.
    let mut final_vocab: Vec<VocabEntry> = Vec::new();
    let mut id = 0u32;
    for t in &special_tokens {
        final_vocab.push(VocabEntry {
            id,
            token: t.clone(),
        });
        id += 1;
    }
    for t in &vocab_tokens {
        final_vocab.push(VocabEntry {
            id,
            token: t.clone(),
        });
        id += 1;
    }

    let vocab_file = VocabFile {
        version: env!("CARGO_PKG_VERSION").to_string(),
        name: "adam-bpe-tokenizer-vocab".to_string(),
        target_language: "kazakh".to_string(),
        script: "cyrillic".to_string(),
        vocab_size: final_vocab.len(),
        special_tokens: special_tokens.to_vec(),
        training_merge_count: merges.len(),
        training_corpus_sentence_count: sentence_count,
        training_corpus_initial_token_count: initial_tokens,
        vocab: final_vocab,
    };

    let merge_count = merges.len();
    let merges_file = MergesFile {
        version: env!("CARGO_PKG_VERSION").to_string(),
        name: "adam-bpe-tokenizer-merges".to_string(),
        target_language: "kazakh".to_string(),
        script: "cyrillic".to_string(),
        merge_count,
        merges,
    };

    if let Err(e) = fs::write(
        &vocab_out,
        serde_json::to_string_pretty(&vocab_file).expect("vocab ser"),
    ) {
        eprintln!("cannot write vocab file {}: {}", vocab_out, e);
        return ExitCode::FAILURE;
    }
    if let Err(e) = fs::write(
        &merges_out,
        serde_json::to_string_pretty(&merges_file).expect("merges ser"),
    ) {
        eprintln!("cannot write merges file {}: {}", merges_out, e);
        return ExitCode::FAILURE;
    }

    eprintln!("wrote {} and {}", vocab_out, merges_out);
    ExitCode::SUCCESS
}
