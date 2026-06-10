// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK · github.com/qazaq-ai/adam
//! # Sentence-coherence gate — v6.5 Movement D
//!
//! Pre-routing gate that combines three independent signals to
//! detect when the post-STT / post-fuzzy / post-LM input is too
//! noisy to act on safely.  When the gate fires, adam emits a
//! polite «Кешіріңіз, толық түсінбедім» template instead of
//! handing the noise to the downstream cascade (which would
//! either guess wrong or hit a low-quality template path).
//!
//! ## Background
//!
//! v6.4.0-rc1..rc12 audits surfaced a recurring failure pattern:
//! whisper transcribes Kazakh imperfectly, the v6.2 cascade
//! grabs whatever it can match against, and adam confidently
//! emits a wrong-template answer.  Each rc patched the surface
//! (operator vocab, name DB, recall surface forms, …) — but
//! the next audit found a new Whisper variant that slipped
//! through.
//!
//! User feedback verbatim (rc8 audit):
//!
//! > «Я имею ввиду валидировать слова не по отдельности, а в
//! >  комплексе всего предложения.»
//!
//! v6.5 Movement D implements that holistic check.
//!
//! ## Signals
//!
//! 1. **LM log-likelihood per token** from the existing
//!    contextual-LM rescorer.  Higher (less negative) = more
//!    plausible Kazakh.  Audit observation: noisy STT
//!    typically lands at ≤ −5.0 / token; clean Kazakh ≥ −2.5.
//!
//! 2. **Intent classifier max-class confidence**.  Audit
//!    observation: clean Kazakh intents hit 0.85–1.00; noisy
//!    drift to 0.20–0.50.
//!
//! 3. **FST morphology parse coverage** — what fraction of the
//!    input's alphabetic tokens were analysed successfully by
//!    `adam_kernel_fst::parser::analyse`.  Clean Kazakh ≈ 0.90+
//!    coverage; STT-noised text drops to 0.30–0.60.
//!
//! ## Decision rule (conservative)
//!
//! Refuse ONLY when at least TWO of three signals vote "noise".
//! Single weak signal is acceptable — Whisper output frequently
//! has one bad metric out of three even on legitimate input.
//! Requiring two-of-three keeps false-rejection low while still
//! catching the cases where the whole utterance is garbled.
//!
//! Thresholds were chosen from the rc11/rc12 audit transcripts;
//! they SHOULD be re-calibrated against a labelled batch of real
//! audio before the layer is enabled by default — see
//! `docs/v6_5_strategic_plan.md` Move D ship gate.

use crate::intent_classifier_runtime::IntentClassifierRuntime;
use crate::neural_rescorer::NeuralRescorer;
use adam_kernel_fst::lexicon::LexiconV1;
use adam_kernel_fst::parser::analyse;

/// One sentence's signal bundle.  All three signals are computed
/// even if one is going to disqualify the input — the caller
/// logs the full triplet for audit visibility.
#[derive(Debug, Clone)]
pub struct CoherenceSignals {
    /// Average log-likelihood per token from the contextual LM.
    /// `None` when the LM rescorer is unavailable or the input
    /// is too short for the LM to score.
    pub lm_logprob_per_token: Option<f32>,
    /// Max-class confidence from the neural intent classifier.
    /// `None` when the classifier checkpoint is unavailable.
    pub intent_confidence: Option<f32>,
    /// Fraction of alphabetic tokens that the FST morphology
    /// analyser produced at least one parse for.  Always present
    /// — the FST runs without external state.
    pub fst_parse_coverage: f32,
}

/// Default thresholds — conservative, chosen from rc11/rc12
/// audit signal distributions.  TODO (v6.5.0 ship gate):
/// re-calibrate against a labelled 100-utterance batch.
pub const LM_LOGPROB_FLOOR: f32 = -5.0;
pub const INTENT_CONF_FLOOR: f32 = 0.30;
pub const FST_COVERAGE_FLOOR: f32 = 0.40;

/// **v6.5.0-rc6 (2026-06-09 live-audit fix).**  When the intent
/// classifier is THIS confident on its top class, the gate trusts
/// the classifier outright and refuses to refuse — even if the LM
/// and FST signals both vote noise.
///
/// Rationale: rc5 audit T4 «Рақмет, аллаға шұқыр» was a perfectly
/// recognisable Kazakh "thank God" expression (= "thanks, praise be
/// to Allah").  Coherence saw lm=-6.17 (LM is unfamiliar with the
/// religious vocabulary, scores it badly), fst=0.33 (Whisper's
/// «шұқыр» drift didn't parse as «шүкір»), intent=1.00
/// (classifier nailed it).  Old 2-of-3 rule refused.  New rule:
/// when the classifier hits ≥0.85 confidence on a label, the
/// utterance is communicatively coherent regardless of LM/FST
/// disagreement — refusing it loses a turn that adam could have
/// answered.
pub const HIGH_INTENT_TRUST: f32 = 0.85;

impl CoherenceSignals {
    /// Compute all three signals for `text`.  Each is taken at
    /// best-effort; missing signals collapse to `None` (treated
    /// as neutral by `is_coherent`).
    pub fn collect(
        text: &str,
        lm: Option<&NeuralRescorer>,
        ic: Option<&IntentClassifierRuntime>,
        lex: Option<&LexiconV1>,
    ) -> Self {
        let lm_logprob_per_token = lm.and_then(|r| r.score_text(text));
        let intent_confidence = ic.and_then(|c| c.classify(text).map(|(_, conf)| conf));
        let fst_parse_coverage = lex.map(|l| fst_coverage(text, l)).unwrap_or(1.0);
        Self {
            lm_logprob_per_token,
            intent_confidence,
            fst_parse_coverage,
        }
    }

    /// Returns `false` when adam should refuse to route this
    /// turn.  Two-of-three vote: refuse only when at least two
    /// independent signals say "noise".
    ///
    /// **rc6 amendment (2026-06-09):** when the intent classifier is
    /// very confident (≥ [`HIGH_INTENT_TRUST`]), this method always
    /// returns `true` regardless of LM/FST.  The classifier is the
    /// most reliable signal we have — it was trained on labelled
    /// data — and refusing a label it nailed at ≥0.85 is more
    /// expensive than the rare case of trusting a confident but
    /// wrong label.
    pub fn is_coherent(&self) -> bool {
        // rc6: high-confidence intent overrides LM/FST.
        if let Some(c) = self.intent_confidence
            && c >= HIGH_INTENT_TRUST
        {
            return true;
        }

        let lm_says_noise = self
            .lm_logprob_per_token
            .map(|s| s < LM_LOGPROB_FLOOR)
            .unwrap_or(false);
        let intent_says_noise = self
            .intent_confidence
            .map(|c| c < INTENT_CONF_FLOOR)
            .unwrap_or(false);
        let fst_says_noise = self.fst_parse_coverage < FST_COVERAGE_FLOOR;

        let noise_votes = [lm_says_noise, intent_says_noise, fst_says_noise]
            .iter()
            .filter(|&&x| x)
            .count();
        // Coherent unless 2+ signals agree it's noise.
        noise_votes < 2
    }

    /// One-line log summary for the voice REPL audit trail.
    pub fn render_log(&self) -> String {
        let lm = self
            .lm_logprob_per_token
            .map(|s| format!("{s:.2}"))
            .unwrap_or_else(|| "n/a".into());
        let intent = self
            .intent_confidence
            .map(|c| format!("{c:.2}"))
            .unwrap_or_else(|| "n/a".into());
        format!("lm={lm} intent={intent} fst={:.2}", self.fst_parse_coverage)
    }
}

/// Polite refusal template — emitted when `is_coherent` returns
/// false.  Names the issue (the model didn't fully understand)
/// and invites a retry without putting blame on the speaker.
pub const REFUSE_TEMPLATE: &str = "Кешіріңіз, сұрағыңызды толық түсінбедім. Қайталай аласыз ба — \
     жайлап, басқа сөздермен?";

fn fst_coverage(text: &str, lex: &LexiconV1) -> f32 {
    // Tokenise on non-alphabetic boundaries; lowercase; count
    // tokens whose analyse() returns a non-empty parse set.
    let cleaned: String = text
        .chars()
        .map(|c| if c.is_alphabetic() { c } else { ' ' })
        .collect();
    let tokens: Vec<&str> = cleaned.split_whitespace().collect();
    if tokens.is_empty() {
        return 1.0;
    }
    let parsed = tokens
        .iter()
        .filter(|t| !analyse(t, lex).is_empty())
        .count();
    parsed as f32 / tokens.len() as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coherent_when_all_three_signals_strong() {
        let s = CoherenceSignals {
            lm_logprob_per_token: Some(-1.5),
            intent_confidence: Some(0.95),
            fst_parse_coverage: 0.90,
        };
        assert!(s.is_coherent());
    }

    #[test]
    fn refuse_when_two_of_three_signals_say_noise() {
        // Audit case: «Менің атым да улет» — LM bad + intent bad
        let s = CoherenceSignals {
            lm_logprob_per_token: Some(-5.5),
            intent_confidence: Some(0.20),
            fst_parse_coverage: 0.70, // single-good-signal not enough
        };
        assert!(!s.is_coherent());
    }

    /// **v6.5.0-rc6 audit fix.**  Live audit T4: «Рақмет, аллаға
    /// шұқыр» — LM bad (religious vocabulary unfamiliar to the
    /// rescorer), FST bad (Whisper drift «шұқыр» didn't parse), but
    /// intent classifier at 1.00 on AskAboutTopic.  Old rule
    /// refused.  New rule trusts the classifier when confidence ≥
    /// HIGH_INTENT_TRUST = 0.85.
    #[test]
    fn rc6_high_intent_confidence_overrides_lm_and_fst() {
        let s = CoherenceSignals {
            lm_logprob_per_token: Some(-6.17),
            intent_confidence: Some(1.00),
            fst_parse_coverage: 0.33,
        };
        assert!(
            s.is_coherent(),
            "intent=1.00 should override LM/FST noise votes"
        );
    }

    /// Boundary: at exactly HIGH_INTENT_TRUST the override fires.
    /// Just below it, the standard 2-of-3 rule applies again.
    #[test]
    fn rc6_high_intent_trust_boundary() {
        let just_at = CoherenceSignals {
            lm_logprob_per_token: Some(-7.0),
            intent_confidence: Some(HIGH_INTENT_TRUST),
            fst_parse_coverage: 0.10,
        };
        assert!(just_at.is_coherent());

        let just_below = CoherenceSignals {
            lm_logprob_per_token: Some(-7.0),
            intent_confidence: Some(HIGH_INTENT_TRUST - 0.01),
            fst_parse_coverage: 0.10,
        };
        // Two votes for noise (lm + fst) and intent isn't high
        // enough to override → refuse.
        assert!(!just_below.is_coherent());
    }

    #[test]
    fn coherent_when_only_one_signal_weak() {
        // Common case: clean Kazakh but classifier confidence is
        // moderate — should still pass.
        let s = CoherenceSignals {
            lm_logprob_per_token: Some(-1.8),
            intent_confidence: Some(0.25), // below floor
            fst_parse_coverage: 0.85,
        };
        assert!(s.is_coherent());
    }

    #[test]
    fn missing_signals_are_neutral() {
        // When LM / classifier checkpoints are absent, those
        // signals don't vote.  FST coverage alone shouldn't
        // refuse unless it's catastrophically bad.
        let s = CoherenceSignals {
            lm_logprob_per_token: None,
            intent_confidence: None,
            fst_parse_coverage: 0.30, // below floor — but lone vote
        };
        assert!(s.is_coherent());
    }

    #[test]
    fn refuse_template_names_the_issue_and_invites_retry() {
        assert!(REFUSE_TEMPLATE.contains("Кешіріңіз"));
        assert!(REFUSE_TEMPLATE.contains("толық түсінбедім"));
        assert!(REFUSE_TEMPLATE.contains("Қайталай"));
    }
}
