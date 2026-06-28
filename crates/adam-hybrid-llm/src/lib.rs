// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `adam-hybrid-llm` — **experiment crate.  Branch-scoped to
//! `experiment/hybrid-qlm-verifier`.  NOT for production
//! consumption.**
//!
//! Thin subprocess wrapper around `llama-completion` (llama.cpp
//! single-shot binary, build 9820+) exposing three typed
//! proposer APIs the hybrid experiment needs:
//!
//!   * [`propose_paraphrase`] — surface a paraphrase of an
//!     input Kazakh utterance, for downstream cascade
//!     coverage probes.
//!   * [`rescore_n_best`] — pick the most plausible
//!     candidate from an STT N-best list (Whisper / DTW
//!     ranks audio-acoustic; the LM ranks Kazakh
//!     plausibility).
//!   * [`classify_dialog_act`] — coarse dialog-act label
//!     (greeting / factual_query / clarify / refusal_signal /
//!     other) to inform router routing.
//!
//! ## Contract (load-bearing — do not relax)
//!
//! 1. Every public function is gated on `ADAM_HYBRID_LM=1`.
//!    Without the env var set the function returns `None` /
//!    the canonical no-op variant.  The crate is **default-
//!    off** even when linked.
//! 2. No function in this crate ever speaks a factual claim
//!    to the user.  Outputs are *candidates* — the
//!    deterministic adam-kernel verifier (existing
//!    AnswerCandidate + ProofRef machinery from v6.8.26) is
//!    responsible for grounding them against world_core
//!    before any utterance reaches the user.
//! 3. Subprocess is `llama-completion` with `-ngl 99` so the
//!    Metal GPU is used.  Without `-ngl` the call falls back
//!    to CPU and on M-class hardware will swap-thrash; see
//!    experiments/hybrid_qlm/baseline/day_3_summary.md for
//!    the painful empirical confirmation.
//!
//! ## What this crate is NOT
//!
//! - A general-purpose LLM client.  Three bounded APIs only.
//! - A training harness.  LoRA / continued-pretrain work
//!   lives under experiments/hybrid_qlm/lora_recipes/.
//! - Production-wired.  No `crates/adam-dialog` callsite
//!   imports this crate.  Wiring happens at a later phase,
//!   gated behind the same env var at the call site.

use std::process::{Command, Stdio};
use std::time::Duration;

use thiserror::Error;

/// Default path to the Gemma-3 4B q4_k_m gguf weights chosen
/// at the `baseline` phase.  Overrideable via the
/// `ADAM_HYBRID_LM_MODEL` env var so future experiments can
/// swap base models without touching code.
pub const DEFAULT_MODEL_PATH: &str = "data/lm_models/gemma-3-4b-it-q4_k_m.gguf";

/// Hard cap on a single proposer call.  The hybrid path runs
/// in the per-turn dialog budget, not a batch job — anything
/// longer than this is a stuck-process signal, not a slow
/// inference signal.
pub const PROPOSER_TIMEOUT_SECS: u64 = 30;

/// Coarse-grained dialog-act tag, mirroring the closed-set
/// classification the v6.2 router benefits from when deciding
/// between factual-retrieval / clarification / wellness
/// routes.  Kept small on purpose; expand only when a route
/// needs a distinction this enum doesn't carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DialogAct {
    /// Conversational opener / closer / acknowledgement.
    Greeting,
    /// Factual question — adam should attempt grounded
    /// retrieval before answering.
    FactualQuery,
    /// User is signalling they didn't understand or need
    /// rephrase — adam should clarify, not double down.
    Clarify,
    /// User is rejecting the prior turn (bare «Жоқ»,
    /// «не так», «нет»).
    RefusalSignal,
    /// None of the above.
    Other,
}

/// Errors the proposer subprocess can surface.  Callers
/// should treat any error as «proposer unavailable, fall
/// back to deterministic path» — never user-facing.
#[derive(Debug, Error)]
pub enum ProposerError {
    /// `ADAM_HYBRID_LM` env var not set to `1`.  Not really
    /// an error — it's the default-off state — but represented
    /// as `Err` so callers see the explicit signal rather
    /// than silently swallowing.
    #[error("hybrid LM disabled (ADAM_HYBRID_LM != 1)")]
    Disabled,
    /// `llama-completion` binary not on PATH.
    #[error("llama-completion binary not found on PATH")]
    BinaryMissing,
    /// Model weights file not at the expected path.
    #[error("model not found at {0}")]
    ModelMissing(String),
    /// Subprocess returned non-zero or panicked.
    #[error("llama-completion failed: {0}")]
    SubprocessFailed(String),
    /// Subprocess timed out per [`PROPOSER_TIMEOUT_SECS`].
    #[error("llama-completion timed out after {0}s")]
    Timeout(u64),
    /// Subprocess output didn't parse into the expected shape.
    #[error("could not parse llama-completion output: {0}")]
    ParseFailed(String),
}

/// **Proposer #1.**  Generate a paraphrase of the Kazakh input.
/// Returns `None` only when the proposer is genuinely
/// disabled / unavailable; callers should treat
/// `Some(paraphrase)` as a CANDIDATE — verifier downstream
/// decides whether it survives.
///
/// Stub for the API-skeleton commit: returns `Err(Disabled)`
/// when the env var is unset, otherwise invokes
/// [`llama_completion_subprocess`] with a paraphrase prompt
/// template.  Output cleanup mirrors the
/// `run_bare_baseline.sh` awk pipeline.
pub fn propose_paraphrase(input: &str) -> Result<String, ProposerError> {
    require_enabled()?;
    let prompt = format!(
        "Қазақша сұрақты басқаша сөздермен қайталап жаз. \
         Мағынасын сақта.  Қысқа жауап.\n\n\
         Бастапқы: {input}\n\
         Қайталанған:"
    );
    llama_completion_subprocess(&prompt, /*n_predict=*/ 64)
}

/// **Proposer #2.**  Rank an STT N-best list by Kazakh
/// plausibility.  Returns the *index* of the highest-ranked
/// candidate, NOT the text itself, so the caller keeps full
/// control over which acoustic features get propagated.
///
/// Stub returns `Ok(0)` when no env-gated LM call would be
/// distinguishable (single-element list) and otherwise prompts
/// the LM to pick the most natural Kazakh sentence.
pub fn rescore_n_best(candidates: &[String]) -> Result<usize, ProposerError> {
    require_enabled()?;
    if candidates.is_empty() {
        return Err(ProposerError::ParseFailed("empty candidate list".into()));
    }
    if candidates.len() == 1 {
        return Ok(0);
    }
    let numbered: String = candidates
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{i}. {c}"))
        .collect::<Vec<_>>()
        .join("\n");
    let prompt = format!(
        "Қазақша сөйлемдер тізімі.  Ең табиғи және \
         грамматикалық дұрыс сөйлемнің НӨМІРІН ҒАНА жаз.\n\n\
         {numbered}\n\n\
         Ең дұрыс нөмір:"
    );
    // n_predict=24 — must leave room for the model's natural
    // Kazakh continuation («Ең дұрыс нөмір: N») before the
    // digit appears.  Empirically 8 tokens cut off mid-prefix
    // and the parse failed; 24 gives the model space to write
    // the answer.
    let out = llama_completion_subprocess(&prompt, /*n_predict=*/ 24)?;
    out.trim()
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>()
        .chars()
        .next()
        .and_then(|c| c.to_digit(10).map(|d| d as usize))
        .and_then(|idx| (idx < candidates.len()).then_some(idx))
        .ok_or_else(|| ProposerError::ParseFailed(format!("unparseable index from «{out}»")))
}

/// **Proposer #3.**  Classify the input into a coarse
/// [`DialogAct`].  Used by the router to skip retrieval on
/// turns that aren't factual queries (greeting / clarify /
/// refusal-signal).
///
/// Stub maps the LM's free-text classification onto the
/// closed enum via case-insensitive substring matching;
/// returns [`DialogAct::Other`] when the LM emits something
/// outside the known set rather than guessing.
pub fn classify_dialog_act(input: &str) -> Result<DialogAct, ProposerError> {
    require_enabled()?;
    let prompt = format!(
        "Қазақ тіліндегі сөйлемді тек ОСЫНДАЙ бес санаттың \
         бірімен таңбала: greeting, factual_query, clarify, \
         refusal_signal, other.  Тек санатты жаз, басқа сөз \
         қоспа.\n\n\
         Сөйлем: {input}\n\
         Санат:"
    );
    // n_predict=16 — same prefix-room concern as
    // `rescore_n_best`; 8 was too few for Gemma to write
    // the category after its own Kazakh continuation.
    let out = llama_completion_subprocess(&prompt, /*n_predict=*/ 16)?;
    let lower = out.trim().to_lowercase();
    let act = if lower.contains("greeting") {
        DialogAct::Greeting
    } else if lower.contains("factual") {
        DialogAct::FactualQuery
    } else if lower.contains("clarify") {
        DialogAct::Clarify
    } else if lower.contains("refusal") {
        DialogAct::RefusalSignal
    } else {
        DialogAct::Other
    };
    Ok(act)
}

/// Env-gate predicate.  Returns `Err(Disabled)` when
/// `ADAM_HYBRID_LM` is not exactly `"1"`.  Every public
/// proposer calls this first; default-off semantics live
/// here, not at the callsite.
fn require_enabled() -> Result<(), ProposerError> {
    match std::env::var("ADAM_HYBRID_LM").as_deref() {
        Ok("1") => Ok(()),
        _ => Err(ProposerError::Disabled),
    }
}

/// Resolve the model path.  Honours `ADAM_HYBRID_LM_MODEL`
/// override, falls back to [`DEFAULT_MODEL_PATH`].
fn resolve_model_path() -> String {
    std::env::var("ADAM_HYBRID_LM_MODEL").unwrap_or_else(|_| DEFAULT_MODEL_PATH.to_string())
}

/// Subprocess the request through `llama-completion`.  Pure
/// fork + wait + parse; no async, no streaming — single-turn
/// proposer calls fit comfortably in the per-turn dialog
/// budget at ~2-3 s on Metal with the 4B base.
///
/// **Stdout is drained in a worker thread, NOT read after
/// exit.**  Previous «poll try_wait, then read stdout»
/// shape produced an effective deadlock: child cleanup
/// blocked until the parent had drained stdout, but the
/// parent's drain was scheduled AFTER child exit was
/// observed — so try_wait kept returning `Ok(None)` until
/// the [`PROPOSER_TIMEOUT_SECS`] cap killed the process.
/// All four smoke-probe calls timed out at exactly 30 s
/// before this fix.  Worker-thread drain decouples read
/// from wait and lets the child reach exit normally.
fn llama_completion_subprocess(prompt: &str, n_predict: u32) -> Result<String, ProposerError> {
    let model_path = resolve_model_path();
    if !std::path::Path::new(&model_path).exists() {
        return Err(ProposerError::ModelMissing(model_path));
    }
    let mut child = Command::new("llama-completion")
        .arg("-m")
        .arg(&model_path)
        .args(["-ngl", "99"])
        .arg("--no-warmup")
        .args(["-c", "512"])
        .args(["-n", &n_predict.to_string()])
        .args(["--temp", "0.2"])
        .arg("-p")
        .arg(prompt)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ProposerError::BinaryMissing
            } else {
                ProposerError::SubprocessFailed(e.to_string())
            }
        })?;

    let stdout_handle = child
        .stdout
        .take()
        .ok_or_else(|| ProposerError::SubprocessFailed("no stdout handle".into()))?;
    let reader = std::thread::spawn(move || -> std::io::Result<String> {
        use std::io::Read;
        let mut s = String::new();
        let mut h = stdout_handle;
        h.read_to_string(&mut s)?;
        Ok(s)
    });

    let started = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return Err(ProposerError::SubprocessFailed(format!("exit {status:?}")));
                }
                break;
            }
            Ok(None) => {
                if started.elapsed() >= Duration::from_secs(PROPOSER_TIMEOUT_SECS) {
                    let _ = child.kill();
                    let _ = reader.join();
                    return Err(ProposerError::Timeout(PROPOSER_TIMEOUT_SECS));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(ProposerError::SubprocessFailed(e.to_string())),
        }
    }

    let raw = reader
        .join()
        .map_err(|_| ProposerError::SubprocessFailed("stdout reader panicked".into()))?
        .map_err(|e| ProposerError::SubprocessFailed(e.to_string()))?;
    Ok(strip_chat_template(&raw))
}

/// Strip the Gemma chat-template framing — `user` / `model`
/// role lines and the trailing `> EOF by user` marker — so
/// the returned text is just the model's response payload.
/// Mirrors the awk pipeline in
/// `experiments/hybrid_qlm/baseline/run_bare_baseline.sh`.
fn strip_chat_template(raw: &str) -> String {
    let mut in_model = false;
    let mut out: Vec<&str> = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed == "model" {
            in_model = true;
            continue;
        }
        if trimmed.starts_with("> EOF") {
            in_model = false;
            continue;
        }
        if in_model && !trimmed.is_empty() {
            out.push(trimmed);
        }
    }
    out.join("\n").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proposers_disabled_by_default() {
        // SAFETY: env-var read in a unit test — race only with
        // other tests in the same crate that also touch this
        // var.  None do here.
        unsafe {
            std::env::remove_var("ADAM_HYBRID_LM");
        }
        assert!(matches!(
            propose_paraphrase("Сәлем"),
            Err(ProposerError::Disabled)
        ));
        assert!(matches!(
            rescore_n_best(&["a".into(), "b".into()]),
            Err(ProposerError::Disabled)
        ));
        assert!(matches!(
            classify_dialog_act("Қалайсыз?"),
            Err(ProposerError::Disabled)
        ));
    }

    #[test]
    fn strip_chat_template_handles_gemma_format() {
        let raw = "user\nWhat is X?\nЖауап:\nmodel\nАстана.\n\n> EOF by user\n";
        assert_eq!(strip_chat_template(raw), "Астана.");
    }

    #[test]
    fn strip_chat_template_no_model_section_yields_empty() {
        // Defensive: if the format ever changes and the «model»
        // line disappears, we surface empty rather than the
        // whole prompt echo.
        let raw = "user\nfoo\nbar\n";
        assert_eq!(strip_chat_template(raw), "");
    }
}
