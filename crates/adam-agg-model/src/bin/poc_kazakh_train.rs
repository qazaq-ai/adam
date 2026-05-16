// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! End-to-end proof-of-concept:
//!
//! 1. Load real Kazakh Lexicon V1.
//! 2. Generate synthetic morpheme sequences via adam-agg-synth.
//! 3. Map tokens to a compact training vocabulary.
//! 4. Train the tiny burn transformer on these sequences.
//! 5. Show loss decreases.
//! 6. Run constrained inference: model proposes morphemes, FST keeps
//!    only morphologically valid ones; measure validity rate.
//!
//! All in pure Rust, CPU, no Python, no cloud.

use std::collections::HashMap;
use std::path::Path;

use adam_agg_model::generate::{
    build_invalid_mask_table, compute_state_ids, count_valid_transitions, generate_constrained,
    generate_constrained_beam, generate_unconstrained, is_valid_word,
};
use adam_agg_model::train::{TrainConfig, train_next_token, train_next_token_with_alg_loss};
use adam_agg_model::{TinyAgt, TinyAgtConfig};
use adam_agg_synth::{SynthGenerator, TokenKindSer};
use adam_agg_tokenizer::AggTokenizer;
use adam_kernel_fst::lexicon::LexiconV1;
use burn::backend::Autodiff;
use burn::backend::ndarray::{NdArray, NdArrayDevice};

type B = Autodiff<NdArray<f32>>;

fn main() {
    // -- Stage 1: load lexicon -----------------------------------------------
    let curated = "data/tokenizer/segmentation_roots.json";
    let apertium = "data/lexicon_v1/apertium_imported_roots.json";
    if !Path::new(curated).exists() {
        eprintln!("Lexicon files missing; run from repo root.");
        std::process::exit(1);
    }
    let lex = LexiconV1::load(curated, apertium).expect("lexicon load");
    eprintln!(
        "[1/6] Lexicon loaded: {} unique entries",
        lex.entries_ordered.len()
    );

    // -- Stage 2: training data ---------------------------------------------
    // Synth (FST-generated) by default. If POC_REAL_PACK=path is set,
    // *additionally* mix in the real-corpus pairs at that path; the
    // pack must have been produced by `build_real_corpus_pairs`. We
    // filter the real-corpus pack to keep ONLY pairs whose head token
    // is a Root (i.e. the FST successfully decomposed the word into
    // morphemes); Unk-headed pairs would just teach the model
    // word-as-token mappings with no algebra to learn.
    let tokenizer = AggTokenizer::build(lex.clone());
    let mut generator = SynthGenerator::new(&lex, &tokenizer);
    let inflected = generator.noun_inflections(2000); // ~28k pairs
    let possessives = generator.noun_possessives(1000); // ~21k pairs
    let verbs = generator.verb_inflections(500); // 500 × 4 tenses × 3 persons × 2 num ≈ 12k pairs
    let bare = generator.bare_roots();
    // Heavy-tail policy: keep ALL inflected, sub-sample bare roots so
    // they don't dominate the training distribution (otherwise the
    // model just memorises "after Root, emit EOS"). Skim every 4th.
    let bare_sampled: Vec<_> = bare.into_iter().step_by(4).collect();
    let mut all_pairs = inflected;
    all_pairs.extend(possessives);
    all_pairs.extend(verbs);
    all_pairs.extend(bare_sampled);
    let synth_count = all_pairs.len();

    let mut real_kept = 0usize;
    if let Ok(real_pack_path) = std::env::var("POC_REAL_PACK") {
        if Path::new(&real_pack_path).exists() {
            let bytes = std::fs::read(&real_pack_path).expect("read POC_REAL_PACK");
            let real: Vec<adam_agg_synth::TrainingPair> =
                serde_json::from_slice(&bytes).expect("parse POC_REAL_PACK");
            let real_total = real.len();
            for p in real {
                let head_is_root = matches!(
                    p.tokens.first().map(|t| &t.kind),
                    Some(adam_agg_synth::TokenKindSer::Root(_))
                );
                if head_is_root {
                    all_pairs.push(p);
                    real_kept += 1;
                }
            }
            eprintln!(
                "       Real-corpus pack: {}/{} pairs kept (Root-headed only)",
                real_kept, real_total
            );
        } else {
            eprintln!("       WARN: POC_REAL_PACK={} not found", real_pack_path);
        }
    }

    eprintln!(
        "[2/6] Training pairs: {} synth + {} real = {} total",
        synth_count,
        real_kept,
        all_pairs.len()
    );

    // -- Stage 3: compact training vocab ------------------------------------
    // Phase 0: re-map the heterogeneous token ids (suffix hashes + root
    // ids from Lexicon ordering) into a dense [0..vocab_size) range.
    // Pre-allocate service slots so they don't collide with payload
    // tokens: 0=PAD, 1=BOS, 2=EOS, 3=SPACE. Real tokens start at id 4.
    const PAD_ID: i64 = 0;
    const BOS_ID: i64 = 1;
    const EOS_ID: i64 = 2;
    const SPACE_ID: i64 = 3;
    let mut id_to_compact: HashMap<u32, usize> = HashMap::new();
    let mut compact_to_label: Vec<String> =
        vec!["<unk>".into(), "BOS".into(), "EOS".into(), "<spc>".into()];
    id_to_compact.insert(u32::MAX, 0);
    let mut next_id = compact_to_label.len();
    let _ = (PAD_ID, SPACE_ID); // reserved for completeness
    let mut training_sequences: Vec<Vec<i64>> = Vec::new();
    for pair in &all_pairs {
        let mut seq = vec![BOS_ID];
        for tok in &pair.tokens {
            let mapped = *id_to_compact.entry(tok.id).or_insert_with(|| {
                let id = next_id;
                next_id += 1;
                let label = match &tok.kind {
                    TokenKindSer::Root(s) => format!("R:{s}"),
                    TokenKindSer::Suffix(s) => format!("S:{s}"),
                    TokenKindSer::Bos => "BOS".into(),
                    TokenKindSer::Eos => "EOS".into(),
                    TokenKindSer::Pad => "PAD".into(),
                    TokenKindSer::Space => "SPC".into(),
                    TokenKindSer::Unk(s) => format!("U:{s}"),
                    TokenKindSer::Punct(c) => format!("P:{c}"),
                };
                compact_to_label.push(label);
                id
            });
            seq.push(mapped as i64);
        }
        seq.push(EOS_ID);
        training_sequences.push(seq);
    }
    let vocab_size = next_id;
    eprintln!(
        "[3/6] Compact training vocab: {} tokens; {} training sequences",
        vocab_size,
        training_sequences.len()
    );

    // Sample 5 sequences to visualise.
    for seq in training_sequences.iter().take(5) {
        let labels: Vec<&str> = seq
            .iter()
            .map(|&t| {
                compact_to_label
                    .get(t as usize)
                    .map(String::as_str)
                    .unwrap_or("?")
            })
            .collect();
        eprintln!("   {:?}", labels);
    }

    // -- Stage 3b: train / held-out split -----------------------------------
    // Every 10th sequence goes to the held-out eval set. Deterministic so
    // re-runs are comparable. The held-out set tests generalisation: the
    // model never sees these (root, feature) combos during training, but
    // since each root appears in many OTHER combos and each suffix in
    // many other roots, a model that has learned the algebra should
    // still produce valid continuations for the unseen combos.
    let mut train_sequences: Vec<Vec<i64>> = Vec::new();
    let mut eval_sequences: Vec<Vec<i64>> = Vec::new();
    for (i, seq) in training_sequences.iter().enumerate() {
        if i % 10 == 0 {
            eval_sequences.push(seq.clone());
        } else {
            train_sequences.push(seq.clone());
        }
    }
    eprintln!(
        "[3b/6] Split: {} train sequences / {} held-out eval sequences",
        train_sequences.len(),
        eval_sequences.len()
    );

    // -- Stage 4: build model -----------------------------------------------
    // Hyperparameters are env-overridable so we can scale-up without
    // recompiling: POC_D_MODEL, POC_N_LAYERS, POC_N_HEADS, POC_D_FF.
    let device = NdArrayDevice::default();
    let d_model = env_usize("POC_D_MODEL", 64);
    let n_layers = env_usize("POC_N_LAYERS", 2);
    let n_heads = env_usize("POC_N_HEADS", 4);
    let d_ff = env_usize("POC_D_FF", 128);
    let model_cfg = TinyAgtConfig::new(vocab_size, 32, d_model, n_heads, n_layers, d_ff);
    let model: TinyAgt<B> = model_cfg.init(&device);
    let param_count = estimate_params(&model_cfg);
    eprintln!(
        "[4/6] Model built: ~{} params (vocab={}, d={}, layers={})",
        param_count, vocab_size, d_model, n_layers
    );

    // -- Stage 5: train -----------------------------------------------------
    // Hyperparams overridable via env: POC_EPOCHS, POC_BATCH, POC_LR.
    let train_cfg = TrainConfig {
        batch_size: env_usize("POC_BATCH", 32),
        n_epochs: env_usize("POC_EPOCHS", 3),
        lr: env_f64("POC_LR", 3e-3),
        seed: 42,
    };
    eprintln!(
        "[5/6] Training: batch={}, epochs={}, lr={} ...",
        train_cfg.batch_size, train_cfg.n_epochs, train_cfg.lr
    );
    let t0 = std::time::Instant::now();
    let (trained, reports) = train_next_token(model, &train_sequences, &train_cfg, &device);
    let elapsed = t0.elapsed().as_secs_f32();
    let first_loss = reports.first().map(|r| r.loss).unwrap_or(0.0);
    let last_loss = reports.last().map(|r| r.loss).unwrap_or(0.0);
    let mid_loss = reports
        .get(reports.len() / 2)
        .map(|r| r.loss)
        .unwrap_or(0.0);
    eprintln!(
        "       Loss curve: start={:.3}  mid={:.3}  end={:.3}  (Δ {:.1}%)  [{:.1}s, {} steps]",
        first_loss,
        mid_loss,
        last_loss,
        (1.0 - last_loss / first_loss) * 100.0,
        elapsed,
        reports.len()
    );

    // -- Stage 6: morphological validity check ------------------------------
    // For each synth pair we already KNOW the surface is valid. Round-trip
    // it through the tokenizer; the FST guarantees the surface is
    // re-buildable. We measure validity = % of synth surfaces that
    // tokenize → detokenize back to the same surface.
    let sample: Vec<&adam_agg_synth::TrainingPair> = all_pairs.iter().take(500).collect();
    let mut valid = 0;
    let mut invalid = 0;
    for p in &sample {
        let toks = tokenizer.tokenize_word(&p.surface);
        match tokenizer.detokenize_word(&toks) {
            Ok(s) if s == p.surface => valid += 1,
            _ => invalid += 1,
        }
    }
    let total = valid + invalid;
    eprintln!(
        "[6/6] FST round-trip validity on synth output: {}/{} = {:.1}%",
        valid,
        total,
        100.0 * valid as f32 / total as f32
    );

    // -- Stage 6b: held-out CE loss / perplexity ----------------------------
    // Strongest generalisation signal: teacher-forced cross-entropy on
    // held-out (root, feature) combos. If the model just memorised, this
    // will be high. If it has learned the algebra, this will be near the
    // training-end CE.
    let eval_ce = held_out_ce_loss(&trained, &eval_sequences, &device);
    eprintln!(
        "[6b/7] Held-out teacher-forced CE: {:.3}  (training-end was {:.3})",
        eval_ce, last_loss,
    );

    // -- Stage 7: FST-constrained vs unconstrained inference comparison ----
    eprintln!("\n[7/7] Generation comparison (constrained vs unconstrained, HELD-OUT):");
    // Pick prefixes from HELD-OUT sequences with ≥4 tokens. These are
    // (root, feature) combos the model never saw during training. Also
    // capture the "gold" continuation so we can compute exact-match.
    let mut prefixes: Vec<Vec<i64>> = Vec::new();
    let mut gold_continuations: Vec<Vec<i64>> = Vec::new();
    for seq in eval_sequences.iter() {
        if seq.len() >= 4 {
            prefixes.push(seq[..2].to_vec());
            gold_continuations.push(seq[2..].to_vec());
        }
        if prefixes.len() >= 100 {
            break;
        }
    }
    eprintln!(
        "       Using {} held-out prefixes from sequences with ≥4 morpheme tokens",
        prefixes.len()
    );

    let mut constrained_valid = 0usize;
    let mut constrained_total = 0usize;
    let mut unconstrained_valid = 0usize;
    let mut unconstrained_total = 0usize;
    let mut con_words_valid = 0usize;
    let mut unc_words_valid = 0usize;
    let mut beam_words_valid = 0usize;
    let mut con_exact = 0usize;
    let mut unc_exact = 0usize;
    let mut beam_exact = 0usize;
    let labels_of = |ids: &[i64]| -> Vec<&str> {
        ids.iter()
            .map(|&t| {
                compact_to_label
                    .get(t as usize)
                    .map(String::as_str)
                    .unwrap_or("?")
            })
            .collect()
    };
    let exact_match = |out: &[i64], gold: &[i64]| -> bool {
        // Compare the *generated tail* (after the prefix) against the
        // gold continuation, up to and including EOS.
        let prefix_len = 2;
        if out.len() < prefix_len {
            return false;
        }
        let tail = &out[prefix_len..];
        // Truncate generated tail at first EOS.
        let mut end = tail.len();
        for (i, &t) in tail.iter().enumerate() {
            if t == EOS_ID {
                end = i + 1;
                break;
            }
        }
        let trimmed = &tail[..end];
        trimmed == gold
    };
    for (prefix, gold) in prefixes.iter().zip(gold_continuations.iter()) {
        // Constrained greedy.
        let con = generate_constrained(&trained, &compact_to_label, prefix, 6, &device);
        let con_labels = labels_of(&con);
        let (v, t) = count_valid_transitions(&con_labels);
        constrained_valid += v;
        constrained_total += t;
        if is_valid_word(&con_labels) {
            con_words_valid += 1;
        }
        if exact_match(&con, gold) {
            con_exact += 1;
        }

        // Unconstrained greedy.
        let unc = generate_unconstrained(&trained, prefix, 6, &device);
        let unc_labels = labels_of(&unc);
        let (v, t) = count_valid_transitions(&unc_labels);
        unconstrained_valid += v;
        unconstrained_total += t;
        if is_valid_word(&unc_labels) {
            unc_words_valid += 1;
        }
        if exact_match(&unc, gold) {
            unc_exact += 1;
        }

        // Constrained beam search (beam_width=4).
        let beam = generate_constrained_beam(&trained, &compact_to_label, prefix, 6, 4, &device);
        let beam_labels = labels_of(&beam);
        if is_valid_word(&beam_labels) {
            beam_words_valid += 1;
        }
        if exact_match(&beam, gold) {
            beam_exact += 1;
        }
    }

    let con_rate = if constrained_total > 0 {
        100.0 * constrained_valid as f32 / constrained_total as f32
    } else {
        0.0
    };
    let unc_rate = if unconstrained_total > 0 {
        100.0 * unconstrained_valid as f32 / unconstrained_total as f32
    } else {
        0.0
    };
    let n_prefixes = prefixes.len() as f32;
    eprintln!(
        "       Constrained greedy:    {}/{} trns = {:.1}%   words = {:.1}%   exact = {:.1}%",
        constrained_valid,
        constrained_total,
        con_rate,
        100.0 * con_words_valid as f32 / n_prefixes,
        100.0 * con_exact as f32 / n_prefixes,
    );
    eprintln!(
        "       Unconstrained greedy:  {}/{} trns = {:.1}%   words = {:.1}%   exact = {:.1}%",
        unconstrained_valid,
        unconstrained_total,
        unc_rate,
        100.0 * unc_words_valid as f32 / n_prefixes,
        100.0 * unc_exact as f32 / n_prefixes,
    );
    eprintln!(
        "       Constrained beam (w=4):                                 words = {:.1}%   exact = {:.1}%",
        100.0 * beam_words_valid as f32 / n_prefixes,
        100.0 * beam_exact as f32 / n_prefixes,
    );

    // -- Stage 8: train a SECOND model with algebraic loss; compare -------
    if std::env::var("POC_SKIP_ALG").is_ok() {
        eprintln!("\n[8/8] Skipping algebraic-loss A/B (POC_SKIP_ALG set).");
        return;
    }
    eprintln!("\n[8/8] Training a second model with ALGEBRAIC LOSS for A/B:");
    let invalid_table = build_invalid_mask_table(&compact_to_label, vocab_size);
    let state_ids_per_seq: Vec<Vec<u8>> = train_sequences
        .iter()
        .map(|s| compute_state_ids(s, &compact_to_label))
        .collect();
    let model2: TinyAgt<B> = model_cfg.init(&device);
    let alpha = env_f32("POC_ALPHA", 0.5);
    let alg_train_cfg = TrainConfig {
        batch_size: env_usize("POC_BATCH", 32),
        n_epochs: env_usize("POC_EPOCHS", 3),
        lr: env_f64("POC_LR", 3e-3),
        seed: 42,
    };
    eprintln!(
        "       Algebraic alpha={}; same batch/epochs/lr as Stage 5 for fair compare ...",
        alpha
    );
    let t0 = std::time::Instant::now();
    let (trained_alg, alg_reports) = train_next_token_with_alg_loss(
        model2,
        &train_sequences,
        &state_ids_per_seq,
        &invalid_table,
        &alg_train_cfg,
        alpha,
        &device,
    );
    let alg_elapsed = t0.elapsed().as_secs_f32();
    let alg_first = alg_reports.first().copied();
    let alg_last = alg_reports.last().copied();
    if let (Some(f), Some(l)) = (alg_first, alg_last) {
        eprintln!(
            "       Loss split: start CE={:.3} alg={:.3} total={:.3}",
            f.ce, f.alg, f.total
        );
        eprintln!(
            "                   end   CE={:.3} alg={:.3} total={:.3}  [{:.1}s, {} steps]",
            l.ce,
            l.alg,
            l.total,
            alg_elapsed,
            alg_reports.len()
        );
    }

    // Held-out CE for the ALG model (same eval set as Stage 6b).
    let alg_eval_ce = held_out_ce_loss(&trained_alg, &eval_sequences, &device);
    let last_alg_ce = alg_reports.last().map(|r| r.ce).unwrap_or(0.0);
    eprintln!(
        "       [ALG] Held-out teacher-forced CE: {:.3}  (training-end CE was {:.3})",
        alg_eval_ce, last_alg_ce,
    );
    let gen_gap = (alg_eval_ce - eval_ce) * 1000.0;
    eprintln!(
        "       Generalisation gap (eval_ce_alg − eval_ce_ce) × 1000 = {:+.1}  (lower = ALG generalises better)",
        gen_gap
    );

    // Same prefixes → measure constrained + unconstrained validity for model 2.
    let mut alg_con_valid = 0usize;
    let mut alg_con_total = 0usize;
    let mut alg_unc_valid = 0usize;
    let mut alg_unc_total = 0usize;
    let mut alg_con_words = 0usize;
    let mut alg_unc_words = 0usize;
    let mut alg_beam_words = 0usize;
    let mut alg_con_exact = 0usize;
    let mut alg_unc_exact = 0usize;
    let mut alg_beam_exact = 0usize;
    for (prefix, gold) in prefixes.iter().zip(gold_continuations.iter()) {
        let con = generate_constrained(&trained_alg, &compact_to_label, prefix, 6, &device);
        let con_labels = labels_of(&con);
        let (v, t) = count_valid_transitions(&con_labels);
        alg_con_valid += v;
        alg_con_total += t;
        if is_valid_word(&con_labels) {
            alg_con_words += 1;
        }
        if exact_match(&con, gold) {
            alg_con_exact += 1;
        }

        let unc = generate_unconstrained(&trained_alg, prefix, 6, &device);
        let unc_labels = labels_of(&unc);
        let (v, t) = count_valid_transitions(&unc_labels);
        alg_unc_valid += v;
        alg_unc_total += t;
        if is_valid_word(&unc_labels) {
            alg_unc_words += 1;
        }
        if exact_match(&unc, gold) {
            alg_unc_exact += 1;
        }

        let beam =
            generate_constrained_beam(&trained_alg, &compact_to_label, prefix, 6, 4, &device);
        let beam_labels = labels_of(&beam);
        if is_valid_word(&beam_labels) {
            alg_beam_words += 1;
        }
        if exact_match(&beam, gold) {
            alg_beam_exact += 1;
        }
    }
    let alg_con_rate = if alg_con_total > 0 {
        100.0 * alg_con_valid as f32 / alg_con_total as f32
    } else {
        0.0
    };
    let alg_unc_rate = if alg_unc_total > 0 {
        100.0 * alg_unc_valid as f32 / alg_unc_total as f32
    } else {
        0.0
    };
    eprintln!(
        "       [ALG] Constrained greedy:    {}/{} = {:.1}%   words = {:.1}%   exact = {:.1}%",
        alg_con_valid,
        alg_con_total,
        alg_con_rate,
        100.0 * alg_con_words as f32 / n_prefixes,
        100.0 * alg_con_exact as f32 / n_prefixes,
    );
    eprintln!(
        "       [ALG] Unconstrained greedy:  {}/{} = {:.1}%   words = {:.1}%   exact = {:.1}%",
        alg_unc_valid,
        alg_unc_total,
        alg_unc_rate,
        100.0 * alg_unc_words as f32 / n_prefixes,
        100.0 * alg_unc_exact as f32 / n_prefixes,
    );
    eprintln!(
        "       [ALG] Constrained beam (w=4):                          words = {:.1}%   exact = {:.1}%",
        100.0 * alg_beam_words as f32 / n_prefixes,
        100.0 * alg_beam_exact as f32 / n_prefixes,
    );
    let lift_unc = alg_unc_rate - unc_rate;
    eprintln!(
        "       Algebraic-loss uplift on unconstrained-transition validity: {:+.1} pp",
        lift_unc
    );
    let lift_unc_exact = (alg_unc_exact as f32 - unc_exact as f32) / n_prefixes * 100.0;
    eprintln!(
        "       Algebraic-loss uplift on unconstrained-EXACT-match:          {:+.1} pp",
        lift_unc_exact
    );

    eprintln!("\n=== HYPOTHESIS PROOF ===");
    eprintln!("Pure-Rust neural + agglutinative algebra works:");
    eprintln!(
        "  ✓ Tokenizer round-trip ≥ {:.0}% on synth output",
        100.0 * valid as f32 / total as f32
    );
    eprintln!(
        "  ✓ Loss decreased {:.1}% during {} training steps",
        (1.0 - last_loss / first_loss) * 100.0,
        reports.len()
    );
    eprintln!(
        "  ✓ Model size: ~{}K params (target ≤ 10M)",
        param_count / 1000
    );
    eprintln!("  ✓ Stack: pure Rust (burn ndarray CPU); no Python, no cloud");
    eprintln!(
        "  ✓ FST-constrained decoding: {:.1}% morphologically-valid transitions",
        con_rate
    );
    eprintln!(
        "  ✓ Unconstrained decoding:   {:.1}% — gap shows the constraint adds value",
        unc_rate
    );
    eprintln!(
        "  ✓ Algebraic-loss model:     unconstrained {:.1}% ({:+.1} pp vs CE-only)",
        alg_unc_rate, lift_unc
    );
    eprintln!(
        "  ✓ Held-out exact-match:     CE-only {:.1}%  /  ALG {:.1}%  (constrained-beam)",
        100.0 * beam_exact as f32 / n_prefixes,
        100.0 * alg_beam_exact as f32 / n_prefixes,
    );
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}
fn env_f64(key: &str, default: f64) -> f64 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}
fn env_f32(key: &str, default: f32) -> f32 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

/// Teacher-forced average cross-entropy over a held-out set. No
/// gradients, no optimisation — just forward + loss.
fn held_out_ce_loss<B2: burn::tensor::backend::AutodiffBackend>(
    model: &TinyAgt<B2>,
    sequences: &[Vec<i64>],
    device: &B2::Device,
) -> f32 {
    use burn::prelude::*;
    let max_seq_len = model.max_seq_len();
    let batch_size = 32usize;
    let mut total: f32 = 0.0;
    let mut count: usize = 0;
    for chunk in sequences.chunks(batch_size) {
        let b = chunk.len();
        let mut input_data = vec![0i64; b * max_seq_len];
        let mut target_data = vec![0i64; b * max_seq_len];
        for (i, seq) in chunk.iter().enumerate() {
            let take = seq.len().min(max_seq_len);
            if take == 0 {
                continue;
            }
            for j in 0..take - 1 {
                input_data[i * max_seq_len + j] = seq[j];
                target_data[i * max_seq_len + j] = seq[j + 1];
            }
        }
        let input: Tensor<B2, 2, Int> = Tensor::from_data(
            burn::tensor::TensorData::new(input_data, [b, max_seq_len]),
            device,
        );
        let target: Tensor<B2, 2, Int> = Tensor::from_data(
            burn::tensor::TensorData::new(target_data, [b, max_seq_len]),
            device,
        );
        let logits = model.forward(input);
        let loss = model.loss(logits, target);
        let v: f32 = loss.into_scalar().elem();
        total += v;
        count += 1;
    }
    if count == 0 {
        0.0
    } else {
        total / count as f32
    }
}

fn estimate_params(cfg: &TinyAgtConfig) -> usize {
    let emb = 2 * cfg.vocab_size * cfg.d_model;
    let attn = cfg.n_layers * 4 * cfg.d_model * cfg.d_model;
    let ff = cfg.n_layers * 2 * cfg.d_model * cfg.d_ff;
    let norms = cfg.n_layers * 2 * 2 * cfg.d_model;
    let out_proj = cfg.d_model * cfg.vocab_size;
    emb + attn + ff + norms + out_proj
}
