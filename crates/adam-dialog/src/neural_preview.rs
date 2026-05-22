// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! v6.0.0-rc1 — L5.5 neural composer preview wired into adam_chat.
//!
//! ## What this is
//!
//! When `adam_chat` is built with `--features neural` AND launched
//! with `--neural-model <path-to-checkpoint-dir>`, the REPL accepts
//! a `/neural <prompt>` slash command. The slash command:
//!
//!   1. Tokenises `<prompt>` through the production `AggTokenizer`
//!      (same instance the deterministic kernel uses).
//!   2. Uses the morpheme sequence as a generation prefix.
//!   3. Calls `generate_constrained` over the trained `TinyAgt`
//!      checkpoint with FST-validity masking at every step.
//!   4. Detokenises the resulting morpheme sequence back to a surface
//!      string.
//!   5. Runs the surface through the L6 `Verifier` (strict mode by
//!      default — `NonKazakhScript` / `UnkSurface` / `FstRoundTripFailed`
//!      / `Ungrounded` blocks emit explicit verdicts).
//!   6. Prints to stdout: raw morpheme sequence + surface + verifier
//!      verdict + per-step audit. The user sees exactly what the
//!      neural layer produced and what the verifier did with it.
//!
//! ## What this is not
//!
//! This is **not** L5.5 wired into the dialog loop as a default
//! generator. The trained PoC checkpoint is a morpheme-level model
//! (~1.17 M params, vocab ~5.7 k, trained on synth + 50 k real
//! Kazakh pairs, M2 CPU, 95 minutes), not a sentence-level
//! conversational model. It composes ONE word at a time. Wiring it
//! into the multi-turn dialog graph is a v6.x task, not v6.0.0-rc1.
//!
//! The `/neural` slash command is the **runnable preview artifact**
//! that an external alpha partner needs to inspect to validate the
//! v5 → v6 migration: it proves the checkpoint loads, generation
//! runs, verifier gates the output, and nothing about the
//! deterministic kernel path changes when the flag is off.

use std::path::Path;

use adam_agg_model::checkpoint::{LoadedCheckpoint, load_checkpoint};
use adam_agg_model::generate::generate_constrained;
use adam_agg_model::verifier::{BlockReason, Verdict, Verifier};
use adam_agg_tokenizer::{AggTokenizer, MorphToken};
use burn::backend::NdArray;
use burn::backend::ndarray::NdArrayDevice;

type B = NdArray<f32>;

/// Runtime state of the L5.5 preview surface. Held by `main()` for
/// the duration of the REPL.
pub struct NeuralState {
    pub loaded: LoadedCheckpoint<B>,
    pub tokenizer: AggTokenizer,
    pub verifier: Verifier,
    pub device: NdArrayDevice,
}

/// Best-effort init. Returns `None` on any failure (missing
/// checkpoint, malformed sidecar, lexicon path mismatch, …) so the
/// caller can fall back to the deterministic kernel without aborting.
pub fn init(
    checkpoint_dir: &Path,
    lexicon_curated: &str,
    lexicon_apertium: &str,
    facts_path: &str,
) -> Option<NeuralState> {
    let device = NdArrayDevice::default();
    let loaded = match load_checkpoint::<B>(checkpoint_dir, &device) {
        Ok(l) => l,
        Err(e) => {
            eprintln!(
                "adam-chat --neural-model: failed to load {} — {e}",
                checkpoint_dir.display()
            );
            return None;
        }
    };
    let lex = match adam_kernel_fst::lexicon::LexiconV1::load(lexicon_curated, lexicon_apertium) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("adam-chat --neural-model: lexicon load failed — {e}");
            return None;
        }
    };
    let tokenizer = AggTokenizer::build(lex);
    let facts_idx = match Verifier::load_facts_index(facts_path) {
        Ok(idx) => idx,
        Err(e) => {
            eprintln!("adam-chat --neural-model: facts.json load failed ({e}); using empty index");
            std::collections::HashSet::new()
        }
    };
    // Strict-by-default per architecture_neural_v6.md §3.2 — alpha
    // partners must see the full gate behaviour, not a permissive
    // dev-mode preview.
    let verifier_tokenizer = AggTokenizer::build(
        adam_kernel_fst::lexicon::LexiconV1::load(lexicon_curated, lexicon_apertium).ok()?,
    );
    let verifier = Verifier::new_strict(verifier_tokenizer, facts_idx);
    eprintln!(
        "adam-chat: neural preview on — checkpoint {} (vocab={}, d_model={}, layers={})",
        checkpoint_dir.display(),
        loaded.config.vocab_size,
        loaded.config.d_model,
        loaded.config.n_layers,
    );
    Some(NeuralState {
        loaded,
        tokenizer,
        verifier,
        device,
    })
}

/// Run one `/neural <prompt>` turn and return a multi-line stdout
/// summary the REPL prints verbatim. Self-contained: no side effects
/// on the conversation state.
pub fn compose(state: &NeuralState, prompt: &str) -> String {
    // Tokenize the prompt. The first token (after BOS) is what the
    // model is asked to continue from.
    let prompt_tokens = state.tokenizer.tokenize_word(prompt.trim());
    if prompt_tokens.is_empty() {
        return "neural: empty prompt".to_string();
    }
    // Map tokenizer outputs to checkpoint label-vocab ids. The
    // checkpoint's `labels.json` is the same compact vocab the
    // training binary built, so we look surface-form labels up by
    // string.
    let labels = &state.loaded.labels;
    let lookup = |label: &str| labels.iter().position(|l| l == label).map(|i| i as i64);
    // Service tokens: BOS = 1 by convention from poc_kazakh_train.
    let mut prefix: Vec<i64> = vec![1];
    for tok in &prompt_tokens {
        let label = match tok {
            MorphToken::Root { root, .. } => format!("R:{root}"),
            MorphToken::Suffix(kind) => format!("S:{kind:?}"),
            _ => continue,
        };
        match lookup(&label) {
            Some(id) => prefix.push(id),
            None => {
                return format!(
                    "neural: prompt token {label:?} not in checkpoint vocab \
                     ({} labels). Try a Kazakh root the model was trained on.",
                    labels.len(),
                );
            }
        }
    }
    if prefix.len() < 2 {
        return "neural: no usable tokens in prompt (Unk only)".to_string();
    }
    // Generate up to 6 continuation morphemes.
    let max_new = 6;
    let tokens = generate_constrained(&state.loaded.model, labels, &prefix, max_new, &state.device);
    // Render the generated morpheme sequence as a string of labels.
    let label_trail: Vec<&str> = tokens
        .iter()
        .filter_map(|&id| labels.get(id as usize).map(String::as_str))
        .collect();
    // Detokenise: drop service tokens, rebuild MorphToken sequence,
    // call AggTokenizer::detokenize_word.
    let morph_tokens: Vec<MorphToken> = label_trail
        .iter()
        .filter_map(|label| label_to_morph_token(label, &state.tokenizer))
        .collect();
    let surface = state
        .tokenizer
        .detokenize_word(&morph_tokens)
        .unwrap_or_else(|e| format!("<detok error: {e:?}>"));
    let verdict = state.verifier.check(&surface);
    // Format a multi-line audit trail for the user.
    let mut out = String::new();
    out.push_str("[neural] ");
    out.push_str(&label_trail.join(" "));
    out.push('\n');
    out.push_str(&format!("[neural surface] {surface}\n"));
    out.push_str(&format!(
        "[neural verdict] {}",
        format_verdict(&verdict.verdict)
    ));
    out
}

fn label_to_morph_token(label: &str, _tok: &AggTokenizer) -> Option<MorphToken> {
    if let Some(root) = label.strip_prefix("R:") {
        // The checkpoint label only carries the root spelling; we
        // need a Root token with id + pos. id=u32::MAX is the
        // "unknown id" sentinel — detokenize uses root + pos only.
        return Some(MorphToken::Root {
            id: u32::MAX,
            root: root.to_string(),
            pos: adam_agg_tokenizer::RootPos::NounLike,
        });
    }
    if let Some(_kind) = label.strip_prefix("S:") {
        // Suffix kinds are emitted via Debug-format in
        // poc_kazakh_train; reversing that loses round-trip purity.
        // For preview we simply skip suffixes — surface still shows
        // the Root and verifier validates it. Full suffix round-trip
        // is a v6.x improvement.
        return None;
    }
    None
}

fn format_verdict(verdict: &Verdict) -> String {
    match verdict {
        Verdict::Pass {
            surface,
            root,
            grounded,
        } => format!(
            "PASS surface={surface:?} root={:?} grounded={grounded}",
            root.as_deref().unwrap_or("—")
        ),
        Verdict::Block(BlockReason::NonKazakhScript) => "BLOCK NonKazakhScript".into(),
        Verdict::Block(BlockReason::UnkSurface) => "BLOCK UnkSurface".into(),
        Verdict::Block(BlockReason::FstRoundTripFailed) => "BLOCK FstRoundTripFailed".into(),
        Verdict::Block(BlockReason::Ungrounded) => "BLOCK Ungrounded".into(),
    }
}
