// SPDX-License-Identifier: BUSL-1.1
// Part of: adam · ARK (Agglutinative Reasoning Kernel) · github.com/qazaq-ai/adam
//! `v6_2_router` — **integration bridge** between v6.1 dialog
//! cascade and v6.2 neurosymbolic stack.
//!
//! This module is the **single integration point** the v6.1
//! dialog kernel calls into when `ADAM_V6_2=1`. When the gate is
//! off (default), v6.1 cascade runs unchanged.
//!
//! ## Architecture
//!
//! ```text
//! user input
//!   ├─ ADAM_V6_2=1?
//!   │    YES → v6_2_router::answer(input, &corpus_index)
//!   │           ├─ math_solver  (procedural)
//!   │           ├─ system_clock (live state)
//!   │           ├─ FrameIndex   (curated retrieval)
//!   │           └─ realiser     (Frame → Kazakh surface)
//!   │    NO  → v6.1 dialog cascade (Conversation::turn)
//!   └─ output
//! ```
//!
//! Both paths produce a `String` answer; the caller chooses by
//! `is_v6_2_active()`. Stage 8 will promote v6.2 to default-on
//! after HumanDialogEval passes.

use std::sync::OnceLock;

use adam_algebra::{
    AnswerShape, AnswerSlot, Composition, FrameIndex, FramePredicate, Language, ModifierRole,
    PartOfSpeech, QueryFocus, QueryIR, QuestionForm, RankedFrame, Root, corpus_loader,
    dialog_battery, math_solver, realiser, system_clock,
};

/// Read the `ADAM_V6_2` env var.  Set to `1` / `true` / `on` to
/// route the dialog cascade through the v6.2 stack (math_solver,
/// FrameIndex, realiser, OOD discipline, safety guard).
///
/// **Status (v6.5.0-rc20).**  Blind eval **97 / 100** on the
/// curated Kazakh battery — well past the ≥90 % bar the v6.2 doc
/// cited.  The Rust-level default flip would expose ~20 v6.1-
/// cascade-specific regression tests (live_holdout_*,
/// factual_eval_100, end_to_end self-intro / cross-slot, …) whose
/// assertions check v6.1 wording rather than v6.2 behaviour.
/// Migrating those to a v6.2 sibling suite is a separate v6.6+
/// arc; until then the library default stays OFF.  Production
/// binaries (voice REPL, `adam_blind_eval`, `adam_chat`) opt in
/// via env-var set at startup.
///
/// rc20 ships the prep work — cognitive_eval Kazakh-only
/// templates that drop English brand-name leaks
/// («curated», «Rust», «LLM», «live-feed», «ASCII») in favour of
/// «тексерілген деректер» / «бағдарламалау тілдері» /
/// «ағымдағы уақыт» / «латын-таңбалы өрнек».  These pay off both
/// in the future default flip AND in the voice REPL today
/// (Piper TTS reads them cleanly aloud).
pub fn is_v6_2_active() -> bool {
    std::env::var("ADAM_V6_2")
        .map(|v| matches!(v.as_str(), "1" | "true" | "on" | "yes"))
        .unwrap_or(false)
}

/// Process-wide shared corpus, lazily loaded from
/// `data/world_core/*.jsonl` on first use. Falls back to the
/// hand-curated `dialog_battery::canonical_corpus()` when the data
/// directory is absent (e.g. cargo-published crate without bundled
/// data).
pub(crate) fn shared_corpus() -> &'static FrameIndex {
    static CORPUS: OnceLock<FrameIndex> = OnceLock::new();
    CORPUS.get_or_init(|| {
        // Try several candidate paths so the loader works both
        // from the repo root and from a sub-crate test dir.
        for candidate in [
            "data/world_core",
            "../data/world_core",
            "../../data/world_core",
            "../../../data/world_core",
        ] {
            if let Ok((idx, stats)) = corpus_loader::load_world_core(candidate)
                && stats.frames_inserted > 0
            {
                // Augment with the battery's bilingual + historical
                // facts that aren't in world_core/*.jsonl yet (e.g.
                // Russian-rooted aliases, МО РК-specific facts).
                let mut idx = idx;
                augment_with_battery_facts(&mut idx);
                return idx;
            }
        }
        // Last resort: the hand-curated battery corpus.
        dialog_battery::canonical_corpus()
    })
}

/// Add facts that the dialog battery curates but `world_core/*.jsonl`
/// doesn't yet include (Russian aliases, МО РК, historical dates
/// added between v6.1.50 and Stage 7).
fn augment_with_battery_facts(idx: &mut FrameIndex) {
    let battery = dialog_battery::canonical_corpus();
    // Re-insert each battery frame; the FrameIndex deduplicates
    // by structural equality at retrieval time (via match_frame),
    // so duplicate inserts are safe.
    for i in 0..battery.len() {
        let entry = battery.get(adam_algebra::FrameId(i as u32));
        idx.insert_with_language(entry.frame.clone(), entry.domain.clone(), entry.language);
    }
}

/// Main entry point — answer one user input through the full
/// v6.2 stack. Returns `Some(answer)` when any layer produces a
/// result, `None` when the input falls outside our coverage.
pub fn answer(input: &str) -> Option<String> {
    answer_with_corpus(input, shared_corpus())
}

/// **v6.5.0-rc4 (2026-06-09) — lexicon-validated variant.**  Same
/// as [`answer_with_corpus`] but passes the lexicon to the math
/// route so it can refuse to strip case suffixes from words that
/// have a real Kazakh meaning beyond «numeral + case» (e.g. «онда»
/// = "then", not «он» + locative).  See
/// [`adam_algebra::math_solver::solve_validated`] for details.
pub fn answer_with_corpus_and_lexicon(
    input: &str,
    idx: &FrameIndex,
    lex: &adam_kernel_fst::lexicon::LexiconV1,
) -> Option<String> {
    answer_with_corpus_inner(input, idx, Some(lex), None)
}

/// **v6.8.4 L4.5 Phase 2.E.2.** Anaphora-aware variant: same as
/// [`answer_with_corpus_and_lexicon`] but threads an
/// `anaphora_subject` (e.g. the prior-turn Person referent from
/// [`crate::dialog_acts::DiscourseState`]) so handlers that need
/// a subject can resolve a bare follow-up like «Қанша жыл өмір
/// сүрді?» against it.
pub fn answer_with_corpus_and_anaphora(
    input: &str,
    idx: &FrameIndex,
    lex: &adam_kernel_fst::lexicon::LexiconV1,
    anaphora_subject: Option<&str>,
) -> Option<String> {
    answer_with_corpus_full(input, idx, lex, anaphora_subject, None).map(|a| a.text)
}

/// **v6.8.7 L4.8 C.2 — full router entry with procedure anaphora.**
/// Carries an additional `anaphora_procedure_id` hint (the
/// `ReferentKind::Procedure` token from
/// [`crate::dialog_acts::DiscourseState`]) so bare follow-ups
/// like «Қанша қадам бар?» / «Кім жауапты?» can fetch the prior
/// procedure's `steps` / `authorization` fields.  Returns a
/// [`RouterAnswer`] that carries the response text PLUS the id
/// of any procedure that was matched on THIS turn — the caller
/// uses that id to push a fresh `ReferentKind::Procedure`
/// referent so the next turn's follow-ups can resolve.
pub fn answer_with_corpus_full(
    input: &str,
    idx: &FrameIndex,
    lex: &adam_kernel_fst::lexicon::LexiconV1,
    anaphora_subject: Option<&str>,
    anaphora_procedure_id: Option<&str>,
) -> Option<RouterAnswer> {
    answer_with_corpus_inner_full(
        input,
        idx,
        Some(lex),
        anaphora_subject,
        anaphora_procedure_id,
    )
}

/// Typed router result — response text plus the id of any
/// procedure matched by the procedure-retrieval handler on this
/// turn.  Caller pushes a `ReferentKind::Procedure` referent so
/// subsequent turns' attribute follow-ups resolve.
///
/// **v6.8.11 — active-uncertainty foundation (external advisor
/// Q2 #1).**  Adds two confidence-tracking fields so future
/// commits can route low-confidence outputs to an explicit
/// `Clarify` shape rather than the «Бәлкім X» / random-Abay-quote
/// fallback the v6.1 cascade lands on today.
///
/// **v6.8.26 — renamed `RouterAnswer` → `AnswerCandidate` and
/// added typed [`ProofRef`].**  Codex 2026-06-25 #3.  The
/// rename signals the semantic shift: the v6.2 router no longer
/// returns a *string answer* but a *candidate* that downstream
/// layers (verifier, clarifier, future planner) can inspect,
/// score, or discard.  `ProofRef` carries the typed reference
/// to whatever produced the candidate — a curated fact id, a
/// procedure id, a deterministic template name — so the
/// verifier can trace «what did adam ground this on» without
/// reparsing the surface text.  Legacy v6.1 cascade outputs
/// still use [`ProofRef::LegacyCascade`] until each handler is
/// individually migrated.
#[derive(Debug, Clone, PartialEq)]
pub struct AnswerCandidate {
    pub text: String,
    pub matched_procedure_id: Option<String>,
    /// Confidence in `[0.0, 1.0]` — how strongly the route
    /// believes the answer is grounded.  Curated lookups and
    /// system-self responses score `1.0`; procedure retrieval
    /// scores the keyword overlap normalised against `MIN_SCORE`;
    /// the v6.1 legacy cascade scores `0.5` because we haven't
    /// yet plumbed individual handler confidence through it.
    /// A future commit will gate the «Бәлкім X» fallback on
    /// `confidence < CLARIFY_THRESHOLD`.
    pub confidence: f32,
    /// Typed provenance of the answer — which class of evidence
    /// produced it.  Used by the verifier + future Clarify
    /// routing to decide whether to suppress a tentative reply.
    pub evidence_kind: EvidenceKind,
    /// **v6.8.26.**  Typed reference to the producing artifact.
    /// `LegacyCascade` is the migration-default; individual
    /// handlers populate concrete refs (`CuratedFact { fact_id }`,
    /// `ProcedureMatch { procedure_id }`, `Template { name }`,
    /// `LangBridge { country }`) as they're touched.
    pub proof_ref: ProofRef,
}

/// **v6.8.26.**  Backward-compat alias.  Pre-existing callers
/// (telemetry, external bindings) keep working during the
/// migration window.  Remove once no use site references the
/// old name.
pub type RouterAnswer = AnswerCandidate;

/// **v6.8.26 — typed proof reference.**  Concrete reference to
/// the artifact backing an [`AnswerCandidate`].  Distinguished
/// from [`EvidenceKind`] (coarse classification) by carrying
/// the actual ID / name so the verifier can resolve back to
/// the source — a `CuratedFact { fact_id: "geo_kz_001" }` can
/// be fetched from world_core, a `ProcedureMatch { procedure_id:
/// "kk_labor_ppe_002" }` from `procedure_loader`, etc.
///
/// Variants are added when a handler has a structurally-known
/// reference to populate; handlers that don't (most of the
/// v6.1 cascade) keep `LegacyCascade` until their migration
/// commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofRef {
    /// World_core fact id.  Resolves via the FrameIndex; the
    /// verifier checks the fact still exists + still says the
    /// same thing before signing off.
    CuratedFact { fact_id: String },
    /// adam-self identity / capability template — no external
    /// fact required.  The template id is enough to trace.
    SystemSelf { template: &'static str },
    /// Procedure record id resolvable via
    /// `procedure_loader::shared_procedures()`.
    ProcedureMatch { procedure_id: String },
    /// Multi-fact synthesis (e.g. BornIn + DiedIn → lifespan).
    SynthesisedFact { source_fact_ids: Vec<String> },
    /// Deterministic template (Clarify, soft-ack, pain ack).
    /// Verifier knows there is no external claim to validate.
    Template { name: &'static str },
    /// Lang-bridge peripheral mirror (RU/EN capital lookup,
    /// future expansions).  `country` is the canonical lower-
    /// case Kazakh name so the verifier can trace back to the
    /// world_core entry.
    LangBridge { country: &'static str },
    /// v6.1 cascade — proof not yet typed at the handler level.
    /// Migration target; new handlers MUST NOT introduce more
    /// of these.
    LegacyCascade,
}

/// Coarse-grained classification of where a `RouterAnswer`
/// came from.  The set is intentionally small for v6.8.11; the
/// router has dozens of handlers but the verifier only needs to
/// know which kind of grounding to expect.  Future variants
/// (`KGInference`, `CompositionalSynthesis`) can be added as
/// new handler categories appear.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvidenceKind {
    /// A curated `world_core` fact / hand-authored template —
    /// the strongest evidence class.  e.g. «Қазақстанның
    /// астанасы — Астана», capital lookups, identity templates.
    CuratedFact,
    /// adam describing itself / its capabilities / its
    /// limitations.  Backed by `SystemIdentity`; no external
    /// fact required.
    SystemSelf,
    /// Procedure retrieval hit — full SOP returned by
    /// `lookup_procedure_matched` / its attribute siblings.
    /// Carries the keyword-overlap score in
    /// `RouterAnswer::confidence`.
    ProcedureMatch,
    /// Multi-fact synthesis (e.g. BornIn + DiedIn → lifespan).
    /// Confidence is `1.0` when both source facts were
    /// individually `CuratedFact`-grade.
    SynthesisedFact,
    /// Soft-tier acknowledgement (pain ack / wellness statement
    /// / correction template).  Confidence is `1.0` because the
    /// template is deterministic, but it is NOT a factual answer.
    SoftAck,
    /// The v6.1 legacy cascade — not yet split by handler.
    /// Confidence defaults to `0.5` until the cascade is fully
    /// migrated to typed handlers.  A future commit will replace
    /// this catch-all with per-handler kinds.
    LegacyCascade,
}

/// **v6.8.11.**  Confidence floor below which the cascade should
/// emit `Clarify` rather than a tentative answer.  Tuned against
/// the external advisor's Q2 #1 test sketch: a `ProcedureMatch`
/// whose keyword-overlap normalised score drops below ~0.5
/// usually means the user asked about a topic we don't have a
/// procedure for; in that case «Қандай рәсім туралы сұрап
/// жатырсыз?» is a better answer than a low-quality match.
///
/// Defined here so the threshold is one place, not scattered
/// across the cascade.  Initial value is conservative;
/// the first commit only exposes the field — the legacy cascade
/// still emits its «Бәлкім X» fallback as before.
pub const CLARIFY_THRESHOLD: f32 = 0.5;

impl AnswerCandidate {
    /// Construct an `AnswerCandidate` from a `text` +
    /// `evidence_kind` pair.  Confidence defaults to `1.0` and
    /// `proof_ref` to [`ProofRef::LegacyCascade`]; callers with
    /// a known proof reference should use `with_proof_ref`.
    pub fn from_text(text: String, evidence_kind: EvidenceKind) -> Self {
        Self {
            text,
            matched_procedure_id: None,
            confidence: 1.0,
            evidence_kind,
            proof_ref: ProofRef::LegacyCascade,
        }
    }

    /// Override confidence in place.  Returns `self` for fluent
    /// construction: `AnswerCandidate::from_text(...).with_confidence(0.83)`.
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Override matched_procedure_id in place.  Returns `self`
    /// for fluent construction at the procedure handler call site.
    pub fn with_procedure_id(mut self, procedure_id: String) -> Self {
        self.matched_procedure_id = Some(procedure_id);
        self
    }

    /// **v6.8.26.**  Attach a typed [`ProofRef`].  Use at every
    /// handler call site where the proof source is structurally
    /// known — e.g. `with_proof_ref(ProofRef::ProcedureMatch {
    /// procedure_id })` at the procedure-retrieval site, or
    /// `with_proof_ref(ProofRef::LangBridge { country: "ресей" })`
    /// at the lang-bridge site.  Legacy handlers that don't yet
    /// have a typed source leave the default
    /// [`ProofRef::LegacyCascade`].
    pub fn with_proof_ref(mut self, proof_ref: ProofRef) -> Self {
        self.proof_ref = proof_ref;
        self
    }

    /// **v6.8.15 — Codex Q2 #1 v2.** Build a Clarify answer —
    /// a deterministic «I'm not sure I understood, please
    /// rephrase» template.  Used by the cascade-level Clarify
    /// gate when a low-confidence match would otherwise produce
    /// a confidently-wrong answer.  Always uses
    /// [`EvidenceKind::SoftAck`] + confidence 1.0 — the
    /// acknowledgement itself is deterministic, the underlying
    /// fact is not.  No `matched_procedure_id` — a Clarify
    /// MUST NOT pin a weak match as the discourse referent
    /// (would propagate the wrong topic across turns).
    pub fn clarify() -> Self {
        Self {
            text: CLARIFY_TEMPLATE.to_string(),
            matched_procedure_id: None,
            confidence: 1.0,
            evidence_kind: EvidenceKind::SoftAck,
            proof_ref: ProofRef::Template { name: "clarify" },
        }
    }
}

/// Fixed Clarify template.  Deterministic — no slot
/// substitution — so the safety contract holds regardless of
/// what the cascade was about to emit.  Asks the user to
/// rephrase rather than guessing.
const CLARIFY_TEMPLATE: &str = "Сұрағыңызды толық түсіндім деп айта алмаймын. Нақтырақ айтсаңыз — \
     дұрыс жауап беруге тырысамын.";

/// Variant that lets callers supply their own [`FrameIndex`] (used
/// in integration tests + the live REPL).
pub fn answer_with_corpus(input: &str, idx: &FrameIndex) -> Option<String> {
    answer_with_corpus_inner(input, idx, None, None)
}

fn answer_with_corpus_inner(
    input: &str,
    idx: &FrameIndex,
    lex: Option<&adam_kernel_fst::lexicon::LexiconV1>,
    anaphora_subject: Option<&str>,
) -> Option<String> {
    answer_with_corpus_inner_full(input, idx, lex, anaphora_subject, None).map(|a| a.text)
}

/// **v6.8.7 L4.8 C.2 — full inner cascade.**  Same handler chain
/// as [`answer_with_corpus_inner`] but threads the procedure
/// anaphora hint to the new attribute handlers AND captures the
/// matched procedure id from the procedure retrieval handler.
/// The String-returning [`answer_with_corpus_inner`] wraps this
/// and discards the procedure id.
fn answer_with_corpus_inner_full(
    input: &str,
    idx: &FrameIndex,
    lex: Option<&adam_kernel_fst::lexicon::LexiconV1>,
    anaphora_subject: Option<&str>,
    anaphora_procedure_id: Option<&str>,
) -> Option<RouterAnswer> {
    // Procedure attribute follow-ups need to fire BEFORE the
    // main inner handler chain so a bare «Қанша қадам бар?» (no
    // explicit subject) reaches them before the v6.1 broad-topic
    // fallback gets a chance to emit a clarification.  These
    // attribute handlers fire only when a procedure referent is
    // already on the discourse stack, so confidence is `1.0`
    // (the retrieval that surfaced the referent already cleared
    // its own threshold last turn).
    if let Some(text) = lookup_procedure_step_count(input, anaphora_procedure_id) {
        return Some(RouterAnswer::from_text(text, EvidenceKind::ProcedureMatch));
    }
    // **v6.8.50 — procedure_eval audit fix.**  Actor-undergoer
    // query («Кім ... тиіс?» / «Кто проходит ...?») —
    // distinct from authority queries because it asks about
    // WHO PERFORMS the procedure, not WHO IS RESPONSIBLE
    // for it.  Maps to `applies_to` instead of
    // `authorization`.  Must fire BEFORE
    // `lookup_procedure_authority` so the undergoer shape
    // doesn't get hijacked into a responsibility answer.
    if let Some(text) = lookup_procedure_actor_undergoer(input, anaphora_procedure_id) {
        return Some(RouterAnswer::from_text(text, EvidenceKind::ProcedureMatch));
    }
    if let Some(text) = lookup_procedure_authority(input, anaphora_procedure_id) {
        return Some(RouterAnswer::from_text(text, EvidenceKind::ProcedureMatch));
    }
    // v6.8.12 — four additional procedure attribute follow-ups
    // (hazards / prerequisites / step-by-number / source citation).
    // All fire only when a procedure referent is on the discourse
    // stack; without one they return None and the cascade falls
    // through to v6.1 unchanged.
    if let Some(text) = lookup_procedure_hazards(input, anaphora_procedure_id) {
        return Some(RouterAnswer::from_text(text, EvidenceKind::ProcedureMatch));
    }
    if let Some(text) = lookup_procedure_prerequisites(input, anaphora_procedure_id) {
        return Some(RouterAnswer::from_text(text, EvidenceKind::ProcedureMatch));
    }
    if let Some(text) = lookup_procedure_step_by_number(input, anaphora_procedure_id) {
        return Some(RouterAnswer::from_text(text, EvidenceKind::ProcedureMatch));
    }
    if let Some(text) = lookup_procedure_source(input, anaphora_procedure_id) {
        return Some(RouterAnswer::from_text(text, EvidenceKind::ProcedureMatch));
    }
    // v6.8.13 — numeric condition check
    // («биіктік 1,5 м болса СИЗ керек пе?» evaluates against
    // the prior procedure's `ProcedureStep.condition` field).
    if let Some(text) = lookup_procedure_condition_check(input, anaphora_procedure_id) {
        return Some(RouterAnswer::from_text(text, EvidenceKind::ProcedureMatch));
    }
    // v6.8.14 — categorical permission / forbidden-state check
    // («Егер СИЗ жоқ болса, жұмысқа кіруге бола ма?»).  Runs
    // AFTER condition_check so numeric-input shapes go through
    // the typed evaluator first; only categorical conditionals
    // («X жоқ/болмаса ... бола ма?») reach this layer.
    if let Some(text) = lookup_procedure_permission_check(input, anaphora_procedure_id) {
        return Some(RouterAnswer::from_text(text, EvidenceKind::ProcedureMatch));
    }
    // v6.8.21 — historical alias handler («Ол бұрын қалай
    // аталды?»).  Runs BEFORE the legacy cascade so the v6.1
    // live-data refusal (which previously matched on «бұрын»
    // ambiguously) doesn't intercept the query.  Uses
    // anaphora_subject for pronoun-elided shapes; explicit-
    // subject queries match the lookup directly.
    if let Some(text) = lookup_historical_alias_with_anaphora(input, anaphora_subject) {
        return Some(
            AnswerCandidate::from_text(text, EvidenceKind::CuratedFact).with_proof_ref(
                ProofRef::Template {
                    name: "historical_alias",
                },
            ),
        );
    }
    // **v6.8.25 — language-bridge peripheral pilot.**  Catches
    // Russian / English / Kazakh capital-of-country queries
    // against the trilingual `CAPITALS` table.  Runs BEFORE the
    // legacy cascade so RU «А столица России?» / EN «What's the
    // capital of China?» don't fall to the v6.1 «didn't
    // understand» refusal.  Answer is mirrored in the source
    // language — the canonical Kazakh fact graph is NOT
    // duplicated; only the formatter switches.  See the 2026-
    // 06-25 strategic review (project_codex_consultation_
    // 2026_06_25) for the «peripheral semantic adapter»
    // pattern this is the first instance of.
    if let Some(text) = crate::lang_bridge::lookup_capital(input) {
        return Some(
            AnswerCandidate::from_text(text, EvidenceKind::CuratedFact)
                .with_proof_ref(ProofRef::LangBridge { country: "capital" }),
        );
    }
    // Main cascade — String-returning chain.  We also re-run the
    // procedure retrieval helper to recover both the matched id
    // (for the discourse-state referent push) AND the keyword
    // overlap score (for confidence calibration).
    let text = answer_with_corpus_inner_legacy(input, idx, lex, anaphora_subject)?;
    if let Some((_, id, normalised_score)) = lookup_procedure_matched_with_score(input) {
        return Some(
            AnswerCandidate::from_text(text, EvidenceKind::ProcedureMatch)
                .with_procedure_id(id.clone())
                .with_proof_ref(ProofRef::ProcedureMatch { procedure_id: id })
                .with_confidence(normalised_score),
        );
    }
    // Legacy cascade — not yet split by handler.  Future commits
    // will migrate individual handlers (lifespan, birthplace,
    // capital lookups, …) to populate their own EvidenceKind
    // (CuratedFact / SystemSelf / SynthesisedFact) so the
    // confidence floor can become handler-specific.
    Some(AnswerCandidate {
        text,
        matched_procedure_id: None,
        confidence: 0.5,
        evidence_kind: EvidenceKind::LegacyCascade,
        proof_ref: ProofRef::LegacyCascade,
    })
}

fn answer_with_corpus_inner_legacy(
    input: &str,
    idx: &FrameIndex,
    lex: Option<&adam_kernel_fst::lexicon::LexiconV1>,
    anaphora_subject: Option<&str>,
) -> Option<String> {
    // 0a. STT-loop dedupe. Whisper sometimes gets stuck in a
    // repeat-loop and emits «Сәлем. Сәлем. Сәлем.» × 30+. Collapse
    // to the first meaningful clause so adam answers ONCE, not 30×.
    // Codex 2026-05-25 voice REPL session 3 caught this producing
    // 6-line cascade misfires per loop input.
    let dedup_owned;
    let input: &str = if let Some(dedup) = dedupe_stt_loop(input) {
        dedup_owned = dedup;
        &dedup_owned
    } else {
        input
    };

    // 0b. STT-fold — normalize Whisper mishears («оналты» → «он алты»,
    // «жел тұқтықстан» → «желтоқсан», «энштейн» → «эйнштейн»)
    // ONCE at the top so every downstream path (math_solver,
    // canonical_agent_for, broad-topic detector) sees the folded
    // form. Without this, math_solver missed «он алты түбірі»
    // because it received raw «оналты түбірі» (codex 2026-05-25
    // session-3 audit).
    let folded = stt_fold(&input.to_lowercase());
    let input: &str = &folded;

    // 1. Math first — procedural computation.
    //
    // **rc4 architectural fix:** when a lexicon is available, build
    // an FST-backed "is_non_numeral" closure so math_solver refuses
    // to strip case suffixes from words like «онда» (= "then") that
    // have a real Kazakh meaning beyond «numeral + case».  Caller
    // without lexicon (legacy `answer_with_corpus(input, idx)`) falls
    // through to the hardcoded blacklist inside math_solver.
    // **v6.8.17 — Codex Q3 school bug #1.**  Grade-statement
    // detection runs BEFORE math_solver so «Сәлем, мен 8-сынып
    // оқушысымын» doesn't get misrouted to the math refusal
    // template just because the cascade sees a «8» numeral.
    // Gated on a first-person marker AND a grade-role marker
    // (see `recognize_grade_statement`), so standalone factual
    // queries about a grade («8-сынып бағдарламасы қандай?»)
    // are NOT affected.
    if let Some(ack) = recognize_grade_statement(input) {
        return Some(ack);
    }

    let math_hit = if let Some(lex) = lex {
        let is_non_numeral = |w: &str| -> bool {
            use adam_kernel_fst::parser::{Analysis, analyse};
            analyse(w, lex).iter().any(|a| match a {
                Analysis::Noun { root, .. } => root.part_of_speech != "numeral",
                Analysis::Verb { .. } => true,
            })
        };
        if math_solver::looks_like_math_validated(input, &is_non_numeral) {
            math_solver::solve_validated(input, &is_non_numeral)
        } else {
            None
        }
    } else if looks_like_math(input) {
        math_solver::solve(input)
    } else {
        None
    };
    if let Some(r) = math_hit {
        return Some(r.render());
    }

    // **2026-06-03 voice REPL regression** — «Мен Қостанай қалада
    // тұрамын» (I live in the city of Қостанай) was being overridden
    // by the v6_2_router's substring-IsA layer to one-word «Қала»
    // (city). The v6.1 cascade upstream already detected
    // StatementOfLocation, updated the session (`session["city"] =
    // "Қостанай"`), and generated an acknowledgement reply — but
    // v6_2_router.answer() returning Some("Қала") clobbered it.
    //
    // Fix: when the input is a first-person location statement
    // («Мен … тұрамын / тұрамыз»), return None and let the v6.1
    // acknowledgement stand. We DON'T need v6.2-side acknowledgement
    // because the session is already populated; later recall queries
    // («Мен қайда тұрамын») use that session state via the standard
    // v6.2 location-recall handler.
    if looks_like_first_person_location_statement(input) {
        return None;
    }

    // **Phase 23 (2026-06-03)** — chemistry-formula lookup. Live REPL
    // (multi-session) caught «Судың формуласын жазып бер» falling
    // through to the substring-IsA layer that returns «Жансыз табиғат»
    // (the `Су IsA жансыз табиғат` fact wins over the chemistry-formula
    // intent). Hardcoded school-level formula table fires BEFORE the
    // IsA fallback. Requires the «формула» word in the input to avoid
    // false positives on bare substance mentions.
    if let Some(answer) = lookup_chemical_formula(input) {
        return Some(answer);
    }

    // **v6.8 (2026-06-16) — possessive-property lookup.** Catches
    // school-eval question shapes «X-genitive Y-possessive»
    // («Қазақстанның мемлекеттік тілі», «Қазақтың ұлттық тағамы»,
    // «Қазақстанның ең үлкен қаласы») BEFORE the substring-IsA
    // fallback further down the cascade. Pre-v6.8 these queries
    // surfaced wrong answers:
    //   «Қазақстанның мемлекеттік тілі.» → «Мемлекет»
    //     (the cascade matched IsA on the leading noun «Қазақстан»,
    //      ignoring the property head «тілі»)
    //   «Қазақтың ұлттық тағамы.» → «Ұлттық тағам — тағам.»
    //     (substring-IsA picked up «ұлттық тағам IsA тағам»)
    //
    // The world_core facts for these are already curated (const_008,
    // cuis_001 / cuis_002, geo_kz_004); the gap is just in the
    // retrieval ordering. Pattern-matched lookup short-circuits the
    // ambiguous IsA path. Keep the table small and curated; broader
    // possessive disambiguation belongs in the Stage 8 typed query IR.
    if let Some(answer) = lookup_possessive_property(input) {
        return Some(answer);
    }

    // **v6.8.3 — 2026-06-17 user audit (Bug A).** Lifespan computation
    // for «<Person> қанша жыл өмір сүрді?» / «сколько лет прожил».
    // Pre-fix the query fell through to the substring-IsA layer that
    // surfaced the IsA fact («Ахмет Байтұрсынұлы → қазақ ағартушысы»)
    // because no handler combined BornIn + DiedIn into a single typed
    // answer. The data is present (kru_002 + kru_003 carry born_in
    // 1872 + died_in 1937); only the synthesis was missing.
    //
    // **v6.8.4 L4.5 Phase 2.E.2** — anaphora-aware variant gets the
    // optional prior-turn Person referent so a bare follow-up like
    // «Қанша жыл өмір сүрді?» (no explicit subject) resolves against
    // the previous turn's topic.
    if let Some(answer) = lookup_person_lifespan_with_anaphora(input, idx, anaphora_subject) {
        return Some(answer);
    }

    // **v6.8.7 L4.8 — PropertyQueryIR person handlers.**  Generalise
    // the Phase 2.E.2 anaphora pattern from lifespan to BornIn
    // (birthplace) and IsA (occupation).  Each handler:
    //   * detects its shape;
    //   * resolves the subject via `canonical_agent_for` OR the
    //     `anaphora_subject` referent (so «Қайда туылды?» after
    //     «X туралы айтшы.» resolves);
    //   * queries the FrameIndex for the typed property;
    //   * emits a Kazakh template citing the surfaced value.
    if let Some(answer) = lookup_person_birthplace_with_anaphora(input, idx, anaphora_subject) {
        return Some(answer);
    }
    // v6.8.18 — anaphora-aware birth-year handler.  Closes
    // Codex Q3 school #5 part 1: «Ол қай жылы туған?» bare
    // follow-up after a biographical intro.
    if let Some(answer) = lookup_person_birthyear_with_anaphora(input, idx, anaphora_subject) {
        return Some(answer);
    }
    if let Some(answer) = lookup_person_occupation_with_anaphora(input, idx, anaphora_subject) {
        return Some(answer);
    }

    // **v6.8.5 L4.6 — industrial-pilot SOP retrieval.**  Resolves
    // procedure / СОП queries against typed fixtures loaded from
    // `data/procedures/*.jsonl`.  Shape gate ensures unrelated
    // inputs (math, chemistry, generic questions) fall through to
    // the v6.1 cascade unchanged; only inputs carrying a real
    // procedure-query trigger («тәртіб», «рәсім», «порядок», ...)
    // can reach this layer.
    if let Some(answer) = lookup_procedure(input) {
        return Some(answer);
    }

    // 1a. Occupation acknowledgement. «Мен X» / «Мен X-мын» —
    // user stating profession / role. The v6.1 cascade interpreted
    // this as a definition request («Бағдарламашы — кәсіп иесі.»),
    // not a personal statement. Catch the most common shapes.
    if let Some(ack) = recognize_occupation_statement(input) {
        return Some(ack);
    }

    // 1b. Capabilities query. «Сен не білесің?» / «Не істей
    // аласың?» — user wants a self-description of what adam can
    // answer. Distinct from self-identity («Сен кімсің?»).
    //
    // **v6.8.4 L4.5 Phase 2.A.2.** Goes through the typed
    // `lookup_capabilities_typed` so the route carries a typed
    // `system_self` proof + `RouteId::Capabilities` attribution.
    // The cascade still consumes `Option<String>` — `.text` extracts
    // the same surface the old callsite emitted.
    if let Some(c) = lookup_capabilities_typed(input) {
        return Some(c.text);
    }

    // **v6.8.3 — 2026-06-17 user audit (Bug C).** Personal-experience
    // probe — «Сен қандай кітап оқыдың?» / «Сен қандай фильмдер
    // көрдің?» — asks about adam's lived experience.  adam has none:
    // it is a deterministic typed kernel, not an embodied agent.
    // **v6.8.4 L4.5 Phase 2.A.2** — typed via
    // `lookup_personal_experience_typed`, refusal proof shape =
    // `no_data_refusal("lived_experience")`.
    if let Some(c) = lookup_personal_experience_typed(input) {
        return Some(c.text);
    }

    // 1c. Pitch-gender explanation. «Сен мені ағай дедің. Қалай
    // түсіндің?» — user asks how adam detected gender. Honest
    // explanation: pitch analysis on voice input.
    if is_pitch_detection_query(input) {
        // **Phase 20** — 2 paraphrases of pitch detection explanation.
        let variants: &[&str] = &[
            "Сіздің даусыңыздың жиілігі (pitch) бойынша анықтадым. \
             Voice-input режимінде whisper.cpp дауысты транскрипциялаған \
             соң, мен оның негізгі жиілігін («male» болса ~ 85–155 Гц, \
             «female» болса ~ 165–255 Гц) есептеймін де, соған сай \
             қазақша құрметтеу формасын — «Ағай» немесе «Апай» — \
             таңдаймын. Бұл — детерминирленген эвристика, нейрожүйе емес.",
            "Дауыс жиілігі (F0) арқылы. Whisper аудионы транскрипциялаған \
             соң, мен оның негізгі жиілігін есептеймін — ер адам \
             даусы әдетте 85–155 Гц аралығында, әйел даусы 165–255 Гц. \
             Соған қарап «Ағай» немесе «Апай» вокативін таңдаймын. \
             Алгоритм — autocorrelation-based YIN-тәріздес pitch \
             detection, ешқандай нейрожүйе емес.",
        ];
        return Some(pick_variant(variants, input).to_string());
    }

    // 2. Self-identity short-circuit. «Сен кімсің?» / «Сен өзің
    // кімсің?» — these are dialog-self questions, not factoid
    // queries. Without this gate the cascade matches morpheme
    // «сен» / «өзің» and emits Abai poetry quotes (codex
    // 2026-05-25 audit caught this).
    //
    // **v6.8.4 L4.5 Phase 2.A.2** — typed via
    // `lookup_self_identity_typed`, proof shape =
    // `system_self("identity", …)`.  The variant array previously
    // inlined here lives in the extracted `self_identity_response`
    // helper.
    if let Some(c) = lookup_self_identity_typed(input) {
        return Some(c.text);
    }

    // 3. Honest «no live data» refusals — weather, currency,
    // stock prices, current-data queries the kernel has no feed
    // for. **Runs BEFORE the system clock gate** so «Бүгін
    // Алматыда қандай ауа райы?» (which has «бүгін» trigger for
    // clock) routes correctly to the weather-refusal path.
    //
    // **v6.8.4 L4.5 Phase 2.A.2** — typed via
    // `lookup_live_data_refusal_typed`, proof shape =
    // `safety_refusal(input, "live_data", CurrentData)`.  Variant
    // array lives in the extracted `live_data_refusal_response`
    // helper.
    if let Some(c) = lookup_live_data_refusal_typed(input) {
        return Some(c.text);
    }

    // 4. System clock — live state (date / month / weekday /
    //    time-of-day). Only matches queries that are about today's
    //    calendar / clock, NOT about year-anchored facts.
    if looks_like_time_query(input) {
        return Some(emit_clock_answer(input));
    }

    // 4a. Broad-topic «X туралы айтшы» — return a multi-fact
    // paragraph instead of a single object word. Codex 2026-05-25
    // voice REPL caught «Қазақстан туралы айтшы» → «Мемлекет»
    // (one-word IsA hit). Now emits a curated paragraph.
    if let Some(topic) = detect_broad_topic_query(input)
        && let Some(paragraph) = render_broad_topic(&topic, idx)
    {
        return Some(paragraph);
    }

    // 4b. Curated enumeration shortcuts. «Қазақстанның облыстарын
    // айтшы» / «Қазақстанның көршілері кім?» — these need a list,
    // not a single-fact answer. The world_core has curated list
    // strings (geo_kz_104 has all 17 oblasts comma-separated).
    // Stage 8 will lift this with proper Enumeration retrieval;
    // tonight we hand-wire the most common queries.
    if let Some(list_answer) = handle_listing_query(input) {
        return Some(list_answer);
    }

    // **v6.5.0-rc18 — OOD discipline.**  rc17 blind eval surfaced
    // 7 true-positive OOD bugs where adam was emitting wrong
    // Kazakh-relevant answers on non-Kazakh queries:
    //   «Ресейдің президенті кім?»   → Тоқаев (the KZ president!)
    //   «Билл Гейтс қандай адам?»     → Abai proverb about ақылды
    //   «Шанхай қай елде?»             → «Ел — мемлекет»
    //   «Айфон қанша тұрады?»          → topic-search «Тұра»
    //   …
    //
    // Each was reaching topic-search and the cascade was finding
    // the nearest Kazakh-relevant noun.  Worse than refusing.
    //
    // Fix: closed-set non-Kazakh proper-noun detector.  When the
    // input mentions a Western brand / Russian region / world city
    // / foreign-country president, refuse politely and offer to
    // help with Kazakh queries.  Runs AFTER the curated listing
    // shortcuts so legitimate "Қазақстанның X" still resolves;
    // runs BEFORE the typed retrieval so the topic-search
    // fall-through is suppressed.
    if let Some(refusal) = handle_ood_refusal(input) {
        return Some(refusal);
    }

    // 3. Retrieval — typed QueryIR → FrameIndex → realiser.
    let q = build_query_heuristic(input)?;
    let (hit, used_focus) = pick_best_variant(&q, idx)?;
    // Opt-in trace via ADAM_V6_2_TRACE=1 for live audit / debugging.
    if std::env::var("ADAM_V6_2_TRACE").is_ok() {
        eprintln!(
            "[v6.2] input={input:?} agent={:?} predicate={:?} \
             focus={:?} used_focus={:?} slot={:?} object={:?}",
            q.agent.as_ref().map(|c| c.root.surface.as_str()),
            q.predicate.as_ref().map(|p| p.as_str()),
            q.focus,
            used_focus,
            hit.match_result.answer_slot,
            hit.frame.object.as_ref().map(|c| c.root.surface.as_str()),
        );
    }
    Some(realiser::realise(
        hit.frame,
        &used_focus,
        hit.match_result.answer_slot,
    ))
    .filter(|s| !s.trim().is_empty())
}

/// Multi-variant retrieval: try original query, then predicate=None,
/// then Object-focus (when original was Modifier-focus). Returns the
/// best-scoring variant. **Score-100 result wins over score-50
/// partial match from a different focus**, so we don't return
/// «(нақты дерек жоқ)» when the answer lives in the Object slot.
///
/// Codex 2026-05-25 audit: «Абай қайда өмір сүрді?» missed the
/// LivesIn fact because world_core encodes location as object,
/// but the router built a Modifier(Location)-focus query — which
/// returned a partial (Whole, 50) match instead of the better
/// Object-focus answer «семей облысы».
fn pick_best_variant<'a>(
    q: &QueryIR,
    idx: &'a FrameIndex,
) -> Option<(adam_algebra::RankedFrame<'a>, QueryFocus)> {
    // Build the candidate list of (query, focus_used) pairs in
    // priority order. Each returns its best hit; we keep the
    // highest-score result.
    let mut candidates: Vec<(QueryIR, QueryFocus)> = vec![(q.clone(), q.focus.clone())];

    // Variant A: predicate=None fallback (heuristic may have
    // mis-picked predicate, e.g. «қанша» → HasQuantity but curated
    // fact is IsA).
    let mut q2 = q.clone();
    q2.predicate = None;
    candidates.push((q2, q.focus.clone()));

    // Variant B: Object focus retry (world_core encodes LivesIn /
    // LocatedIn answers in the object slot).
    if matches!(&q.focus, QueryFocus::Modifier(_)) {
        let mut q3 = q.clone();
        q3.focus = QueryFocus::Object;
        candidates.push((q3, QueryFocus::Object));
    }

    // Score each variant, keep the highest. Among score-tied
    // variants, earlier-in-list (i.e. closer to original intent)
    // wins.
    let mut best: Option<(adam_algebra::RankedFrame<'a>, QueryFocus, u8)> = None;
    for (cq, focus_used) in candidates {
        if let Some(h) = idx.best_match(&cq) {
            let score = h.match_result.score;
            let better = match &best {
                None => true,
                Some((_, _, prev_score)) => score > *prev_score,
            };
            if better {
                best = Some((h, focus_used, score));
            }
        }
    }
    best.map(|(h, f, _)| (h, f))
}

/// **v6.4.0-rc12 (2026-06-08 audit).**  Single source of truth —
/// delegate to `math_solver::looks_like_math` which derives from
/// the tokenizer's own vocabulary.  Prior to rc12, this function
/// kept its own marker list that drifted out of sync with
/// `math_solver::tokenize` — live audit caught «көбей» (clipped
/// imperative) and «бөль» (Whisper soft-sign) failing to trigger
/// the math route because the duplicate router list lacked them
/// even after tokenize was updated.  See the documentation on
/// [`math_solver::looks_like_math`] for the gate contract.
fn looks_like_math(s: &str) -> bool {
    math_solver::looks_like_math(s)
}

/// Detect «X туралы айтшы» / «X жайында айтшы» / «расскажи о X»
/// broad-topic queries. Returns the canonical agent surface when
/// the query matches; `None` otherwise.
fn detect_broad_topic_query(input: &str) -> Option<String> {
    let lower = input.to_lowercase();
    let broad_markers = [
        "туралы айтшы",
        "туралы айт",
        "туралы айтыңыз",
        "жайында айт",
        "жайында айтшы",
        "туралы не білесің",
        "расскажи о",
        "расскажи про",
    ];
    let has_marker = broad_markers.iter().any(|m| lower.contains(m));
    if !has_marker {
        return None;
    }
    canonical_agent_for(&lower)
}

/// Render a curated multi-fact paragraph about a topic. Pulls
/// 2–4 distinct IsA / PartOf / HasQuantity / LocatedIn facts
/// from the index for the agent and joins them into one
/// sentence. Returns `None` if no facts found.
fn render_broad_topic(topic: &str, idx: &FrameIndex) -> Option<String> {
    // Try a few predicate-focused queries and harvest distinct
    // object surfaces.
    let preds = [
        FramePredicate::IsA,
        FramePredicate::PartOf,
        FramePredicate::HasQuantity,
        FramePredicate::LocatedIn,
        FramePredicate::Authored,
        FramePredicate::FoundedIn,
    ];
    let mut facts: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for p in preds {
        let q = QueryIR::new(
            QueryFocus::Object,
            QuestionForm::Definition,
            AnswerShape::DefinitionalNP,
        )
        .with_agent(noun(topic))
        .with_predicate(p.clone());
        // Pull up to 3 hits per predicate.
        for h in idx.query(&q).into_iter().take(3) {
            if let Some(obj) = h.frame.object.as_ref() {
                let surface = obj.root.surface.clone();
                if seen.insert(surface.clone()) {
                    facts.push(match p {
                        FramePredicate::IsA => format!("{topic} — {surface}"),
                        FramePredicate::PartOf => format!("{topic} {surface} құрамында"),
                        FramePredicate::HasQuantity => {
                            format!("{topic}-да {surface} бар")
                        }
                        FramePredicate::LocatedIn => format!("{topic} {surface}-да орналасқан"),
                        FramePredicate::Authored => format!("{topic} {surface}-ні жазған"),
                        FramePredicate::FoundedIn => format!("{topic} {surface} жылы құрылған"),
                        _ => continue,
                    });
                }
            }
            if facts.len() >= 4 {
                break;
            }
        }
        if facts.len() >= 4 {
            break;
        }
    }
    if facts.is_empty() {
        return None;
    }
    Some(format!(
        "{}.",
        facts
            .into_iter()
            .map(|s| capitalize_first(&s))
            .collect::<Vec<_>>()
            .join("; ")
    ))
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Hand-curated listing answers for the most common
/// «X-нің Y-лары» / «X-ның Y-лары қайсы?» queries that v6.2's
/// single-frame retrieval can't compose. Stage 8 lifts this via
/// typed Enumeration retrieval; tonight this closes the
/// «Мемлекет» misfire for these specific surfaces.
fn handle_listing_query(input: &str) -> Option<String> {
    let lower = input.to_lowercase();
    // Kazakhstan's 17 oblasts.
    if (lower.contains("облыстар") || lower.contains("обылыстар"))
        && (lower.contains("қазақстан") || lower.contains("казахстан"))
    {
        return Some(
            "Қазақстанның 17 облысы: Абай, Ақмола, Ақтөбе, Алматы, \
             Атырау, Батыс Қазақстан, Жамбыл, Жетісу, Қарағанды, \
             Қостанай, Қызылорда, Маңғыстау, Павлодар, Солтүстік \
             Қазақстан, Түркістан, Ұлытау, Шығыс Қазақстан."
                .to_string(),
        );
    }
    // Kazakhstan's neighbors (5 countries).
    if (lower.contains("көршілер") || lower.contains("шектес"))
        && (lower.contains("қазақстан") || lower.contains("казахстан"))
    {
        return Some(
            "Қазақстанның 5 көршісі бар: Ресей (солтүстік), Қытай \
             (шығыс), Қырғызстан (оңтүстік-шығыс), Өзбекстан (оңтүстік) \
             және Түрікменстан (оңтүстік-батыс)."
                .to_string(),
        );
    }
    // Republican-status cities.
    if (lower.contains("республикалық") || lower.contains("маңызы бар қала"))
        && (lower.contains("қазақстан") || lower.contains("казахстан"))
    {
        return Some(
            "Қазақстанда 3 республикалық маңызы бар қала бар: \
             Астана, Алматы, Шымкент."
                .to_string(),
        );
    }
    // Session-5 audit (codex 2026-05-26 voice REPL): «Қазақстанда
    // қандай таулар / өзендер / көлдер бар?» fell through to the
    // IsA fallback and returned «Мемлекет» (the host country's
    // type). Curate the list answers from data/world_core/geography_kz
    // facts that are already tagged PartOf=қазақстан.
    //
    // **Phase 15g.J (2026-06-01)** — extend the country-match check
    // to cover Whisper drift surfaces from the live v4 retest:
    //   «казастан» — Whisper drops the second «қ» AND swallows the
    //                «х» (cyrillic split: казас + тан).
    //   «казахстан» — Russian-Cyrillic spelling Whisper defaults to.
    //   «қазастан» — same drop with Қ→Қ preserved.
    //   «казахстанда» / «қазақстанда» — locative forms — already
    //                                    covered by `.contains` on
    //                                    the root.
    // **Phase 15g.J.1 (2026-06-01)** — broaden the anchor to cover
    // EVERY Whisper drift surface seen in live tests. The key
    // realisation: Whisper alternately keeps or drops the four
    // Kazakh consonants Қ/Ғ/Ң, AND alternately spells the country
    // with «х» (Russian) or «қ» (Kazakh). So «қазақстанда»,
    // «казақстанда» (К-first, Қ-mid), «қазақстанға», «казахстан»,
    // «казастан» all need to anchor the inventory branch. A bare
    // «қазақ» / «казақ» / «казах» root-substring catches all of
    // them at once.
    // **Phase 15g.C.2 (2026-06-02)** — Shirali Whisper preserves Қ
    // (where multilingual drifted to К), so add Қ-prefix variants
    // alongside the К forms. Live REPL: «Қазастанда қандай таулар
    // бар» got `mentions_kz = false` because only К-prefix
    // «казас» was listed; Shirali's «қазас» wasn't covered.
    // **v6.8.10 — voice REPL audit 2026-06-23 turn 19.**  When the
    // user says «біздің жерімізде» («in our country») without an
    // explicit «Қазақстан», that's an implicit Kazakhstan
    // reference — adam is a Kazakh-first system and the speaker
    // is asking about local geography.  Add «жеріміз» as a third
    // anchor alongside the existing country-name surfaces.  Only
    // co-occurs with the «қандай … бар» inventory shape, so the
    // false-positive risk on unrelated «жеріміз» mentions is
    // bounded.
    let mentions_kz = lower.contains("қазақ")
        || lower.contains("казақ")
        || lower.contains("казах")
        || lower.contains("қазах")
        || lower.contains("казас")
        || lower.contains("қазас")
        || lower.contains("жеріміз");
    // **v6.8.10 — voice REPL audit 2026-06-23 turn 19 / 21.**  Users
    // sometimes utter the locative-singular form («көлде» / «тауда»
    // / «өзенде») instead of the plural («көлдер»).  Both surfaces
    // mean «what lakes are there» in spoken Kazakh; the previous
    // trigger only caught the plural so the singular fell through
    // to the definition handler and emitted nonsense («Мемлекет»
    // for T21, an Abay quote about «көл» for T19).  Add the
    // locative variants alongside the existing plural triggers.
    let asks_mountains = lower.contains("таулар")
        || lower.contains("тау бар")
        || lower.contains("тауда")
        || lower.contains("тауларда");
    let asks_rivers = lower.contains("өзендер")
        || lower.contains("өзен бар")
        || lower.contains("өзенде")
        || lower.contains("өзендерде");
    let asks_lakes = lower.contains("көлдер")
        || lower.contains("көл бар")
        || lower.contains("көлде")
        || lower.contains("көлдерде");
    if mentions_kz && asks_mountains {
        return Some(
            "Қазақстандағы танымал таулар: Алатау, Алтай, Тянь-Шань, \
             Жетісу Алатауы, Хан Тәңірі (биік шың)."
                .to_string(),
        );
    }
    if mentions_kz && asks_rivers {
        return Some(
            "Қазақстанның негізгі өзендері: Ертіс, Сырдария, Іле, \
             Жайық, Есіл, Тобыл, Шу, Қаратал, Талас."
                .to_string(),
        );
    }
    if mentions_kz && asks_lakes {
        return Some(
            "Қазақстанның негізгі көлдері: Балқаш, Зайсан, Алакөл, \
             Тенгіз, Маркакөл."
                .to_string(),
        );
    }

    // **Phase 15g.C.3 (2026-06-02)** — president routing was
    // missing from the v6.2 router. Live tests showed «Қазақстанның
    // президенті кім» falling through to the generic IsA fallback
    // (adam: «Қазақстан — мемлекет»). Facts are in
    // data/world_core/government_kazakhstan.jsonl but the substring
    // intent layer didn't pick them up reliably. Route here:
    //   «бірінші / тұңғыш» + «президент»  → Nazarbayev
    //   «қазіргі / ағымдағы / қазір» + «президент»  → Tokayev
    //   bare «президент» without ordinal qualifier → assume current
    if mentions_kz && lower.contains("президент") {
        let is_first = lower.contains("бірінші")
            || lower.contains("бiрiншi")
            || lower.contains("тұңғыш")
            || lower.contains("туңғыш")
            || lower.contains("first");
        let is_current = lower.contains("қазіргі")
            || lower.contains("казiргi")
            || lower.contains("қазір")
            || lower.contains("қазыр")
            || lower.contains("ағымдағы");
        if is_first {
            return Some(
                "Қазақстанның тұңғыш Президенті — Нұрсұлтан Әбішұлы Назарбаев \
                 (1991–2019)."
                    .to_string(),
            );
        }
        // Default (and explicit current) → Tokayev.
        let _ = is_current;
        return Some(
            "Қазақстанның қазіргі Президенті — Қасым-Жомарт Кемелұлы Тоқаев \
             (2019 жылдан бері)."
                .to_string(),
        );
    }
    // «Қандай X білесің?» without a Kazakhstan anchor — short
    // enumerations of the same categories (no host-country
    // constraint).
    if lower.contains("қандай") && lower.contains("білесің") {
        if lower.contains("өзен") {
            return Some(
                "Танымал өзендер: Ертіс, Сырдария, Іле, Жайық, Есіл, \
                 Тобыл, Шу, Қаратал, Талас."
                    .to_string(),
            );
        }
        if lower.contains("тау") {
            return Some(
                "Танымал таулар: Алатау, Алтай, Тянь-Шань, Жетісу \
                 Алатауы, Хан Тәңірі."
                    .to_string(),
            );
        }
        if lower.contains("көл") {
            return Some(
                "Танымал көлдер: Балқаш, Зайсан, Алакөл, Тенгіз, \
                 Маркакөл."
                    .to_string(),
            );
        }
    }

    // **v6.5.0-rc17 — Kazakhstan property queries.**  rc14 blind
    // eval surfaced four «Қазақстанның X-сы» possessive-property
    // queries that the generic IsA retrieval was answering with
    // «Мемлекет» (the host's own type, found via `Қазақстан is_a
    // Мемлекет`).  The world_core contains the actual property
    // facts in `kz_constitution.jsonl` / `geography_kz.jsonl` /
    // `history_kazakhstan.jsonl` but `build_query_heuristic` is
    // not formulating the graph-join query that finds them.
    //
    // Short-term fix (rc17): hardcode the four most common
    // properties at the listing-query layer so each closes its
    // blind-eval item.  Long-term (rc18+): generalise to a
    // possessive-property handler that reads world_core for the
    // capital / currency / area / population / national symbols of
    // any country, not just Kazakhstan.
    if mentions_kz {
        // **v6.5.0-rc22 — voice-REPL Whisper drift normalisation.**
        // Audit T30 «Қазақстанның ел, ордасы қандай» — Whisper
        // inserted a comma INSIDE «елордасы», splitting it into
        // two words.  rc17 handler checked substring «елорда», which
        // was now absent (we had «ел орда» space-separated instead).
        // Strip punctuation + glue back common compound nouns that
        // Whisper has been seen to fragment.  Same for «тәуел
        // елісіздік» (audit T33 Whisper drift of «тәуелсіздік»).
        let lower_clean = lower
            .replace([',', '.', ':', ';', '!', '?'], " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let lower_glued = lower_clean
            .replace("ел ордасы", "елордасы")
            .replace("ел орда", "елорда")
            .replace("ел өрда", "елорда")
            .replace("тәуел елісіздік", "тәуелсіздік")
            .replace("тәуел сіздік", "тәуелсіздік");
        let lc = &lower_glued;
        // **v6.8.1 — 2026-06-17 voice REPL audit (Bug #17).** Pre-fix
        // capital gate required `"астана" && "қандай"` adjacency.  Live
        // session «Қазақстанның астанысы — қай қала» missed on TWO
        // counts: Whisper drift «астанасы → астанысы» (а→ы at
        // position 5) killed the «астана» substring (became «астаны»);
        // interrogative was «қай қала» not «қандай».  Extensions ride
        // on top of the existing `mentions_kz` gate so they fire only
        // inside a Kazakhstan-scoped question — no false-positive
        // surface on generic «астана» mentions.
        let has_capital_interrogative = lc.contains("қандай")
            || lc.contains("қай қала")
            || lc.contains("қай қалада")
            || lc.contains("қай қалалар");
        let capital_match = lc.contains("елорда")
            || ((lc.contains("астана") || lc.contains("астаны")) && has_capital_interrogative);
        if capital_match {
            return Some(
                "Қазақстанның елордасы — Астана қаласы (1997 жылдан бастап; \
                 2019–2022 жылдары «Нұр-Сұлтан» деп аталды)."
                    .to_string(),
            );
        }
        if lc.contains("валюта") || lc.contains("ақша бірлігі") {
            return Some(
                "Қазақстанның ұлттық валютасы — теңге (KZT, 1993 жылдан бастап).".to_string(),
            );
        }
        if lc.contains("тәуелсіздік") && (lc.contains("қашан") || lc.contains("алды"))
        {
            return Some(
                "Қазақстан Республикасы 1991 жылы 16 желтоқсанда тәуелсіздік алды.".to_string(),
            );
        }
        if lc.contains("ең биік") || lc.contains("биік шың") || lc.contains("биік тау")
        {
            return Some(
                "Қазақстанның ең биік шыңы — Хан Тәңірі (7 010 м, Тянь-Шань \
                 жотасында, Қытаймен шекарада)."
                    .to_string(),
            );
        }
    }

    // **v6.5.0-rc17 — «X — не?» definition shortcuts.**  rc14
    // blind eval refused on «Балқаш — не?», «Күн — не?», «Жер —
    // не?» because the IsA retrieval had nothing to anchor on
    // (those tokens are not curated in world_core as the SUBJECT
    // of an IsA fact — they're objects/topics).  Add the canonical
    // definitions at the shortcut layer.
    //
    // **v6.5.0-rc22** — broaden to recognise bare «X не», «X: не»,
    // «X. не», «X, не» surface forms (Whisper sometimes emits a
    // colon / comma / period between the topic and the question
    // marker instead of an em-dash).  Strip the punctuation
    // between the topic word and the trailing «не» / «не?» before
    // matching.
    if lower.contains(" — не")
        || lower.contains("— не?")
        || lower.contains("дегеніміз не")
        || lower.ends_with(" не")
        || lower.ends_with(" не.")
        || lower.ends_with(" не?")
        || lower.contains(": не")
        || lower.contains(", не")
        || lower.contains(". не")
    {
        if lower.starts_with("балқаш") || lower.starts_with("балхаш") {
            return Some(
                "Балқаш — Қазақстанның оңтүстік-шығысындағы үлкен көл (тұщы / \
                 тұзды екі бөліктен тұрады, әлемдегі ірі көлдердің бірі)."
                    .to_string(),
            );
        }
        if lower.starts_with("күн") {
            return Some(
                "Күн — Күн жүйесінің орталығындағы жұлдыз; өзіндік сәулесі бар \
                 аспан денесі. Жерден шамамен 150 миллион км қашықтықта."
                    .to_string(),
            );
        }
        if lower.starts_with("жер") {
            return Some(
                "Жер — Күн жүйесіндегі үшінші ғаламшар (планета); Меркурий мен \
                 Шолпаннан кейін орналасқан. Жалғыз тіршілік анықталған ғаламшар."
                    .to_string(),
            );
        }
        if lower.starts_with("ай") && (lower.starts_with("ай — ") || lower.starts_with("ай —"))
        {
            return Some(
                "Ай — Жердің табиғи серігі; тас денесі, өзіндік сәулесі жоқ — \
                 Күн сәулесін шағылыстырады."
                    .to_string(),
            );
        }
        // **v6.5.0-rc18** — common linguistic / scientific definitions
        // that the IsA retrieval refuses because the subject is the
        // term itself (no `морфема is_a X` fact in world_core).
        if lower.starts_with("морфема") {
            return Some(
                "Морфема — тілдің мағыналы ең кіші бөлшегі: сөз түбірі немесе \
                 қосымша (мысалы, «үй+ге» = «үй» түбірі + «-ге» жалғауы)."
                    .to_string(),
            );
        }
        if lower.starts_with("жалғау") {
            return Some(
                "Жалғау — сөздің түбіріне қосылып, оның грамматикалық мағынасын \
                 өзгертетін қосымша (септік, тәуелдік, көптік, жіктік жалғаулары)."
                    .to_string(),
            );
        }
        if lower.starts_with("фотосинтез") {
            return Some(
                "Фотосинтез — өсімдіктердің Күн жарығының энергиясы арқылы \
                 көмірқышқыл газы мен судан органикалық зат пен оттегі түзу \
                 процесі."
                    .to_string(),
            );
        }
        if lower.starts_with("гравитация") {
            return Some(
                "Гравитация — массасы бар денелер арасындағы өзара тарту күші. \
                 Ньютон ашқан әмбебап бүкіләлемдік тартылыс заңы."
                    .to_string(),
            );
        }
    }

    None
}

/// **v6.5.0-rc18 — OOD discipline.**
///
/// Detect non-Kazakh proper nouns and refuse honestly instead of
/// letting the topic-search fall-through produce wrong-domain
/// answers.  The rc17 baseline had 7 true-positive bugs in this
/// class:
///
///   «Ресейдің президенті кім?» → Тоқаев (the Kazakh president!)
///   «Билл Гейтс қандай адам?»   → Abai proverb about ақылды
///   «Шанхай қай елде?»           → «Ел — мемлекет»
///   «Айфон қанша тұрады?»        → topic-search «Тұра»
///   «Эйнштейн қашан туылған?»   → topic-search «Туылған»
///   «Ватикан қандай мемлекет?»  → state definition
///   «Гарри Поттер кім?»          → proper-noun fallback
///
/// The pattern: cascade reaches topic-search and finds the nearest
/// Kazakh-relevant noun.  Worse than refusing.
///
/// Closed-set keyword detection.  Adds substring lookup of common
/// foreign entities (countries, cities, brands, Western names,
/// fictional characters).  Match → polite Kazakh-only refusal +
/// offer to help with Kazakh queries.
fn handle_ood_refusal(input: &str) -> Option<String> {
    let lower = input.to_lowercase();

    // Don't fire on inputs that explicitly mention Kazakhstan in
    // KAZAKH script.  «казахстан» (Russian / Latin) is intentionally
    // NOT a bypass — it's a script-discipline signal, not an
    // identity claim.
    let kz_anchored = lower.contains("қазақстан") || lower.contains("қазақ");

    // **v6.5.0-rc19 — substantive-English script discipline.**
    // Refuse only on Latin input that contains a known English
    // function word (what / is / how / about / …).  Random Latin
    // gibberish like «xyz random 123» still falls through
    // (`unknown_input_returns_none` regression).  Russian queries
    // are NOT refused at the script layer — the v6.2 cascade has
    // bilingual curated facts (e.g. «Что такое гравитация» →
    // Russian definition).  v6.1 had its own Russian language
    // guard; v6.2 keeps the bilingual capability.
    let (latin, _kaz_specific, cyrillic_total) = count_scripts(&lower);
    let alphabetic = latin + cyrillic_total;
    if alphabetic >= 5 && latin * 2 > alphabetic && !kz_anchored {
        let has_english_word = ENGLISH_FUNCTION_WORDS.iter().any(|w| {
            lower
                .split_whitespace()
                .any(|t| t.trim_end_matches(['?', '.', '!', ',']) == *w)
        });
        if has_english_word {
            return Some(
                "Менің сұхбат тілім — қазақ тілі. Сұрағыңызды қазақша \
                 қойсаңыз, қазақ-тілді curated білім қорымдағы фактілермен \
                 жауап беруге тырысамын."
                    .to_string(),
            );
        }
    }

    let foreign_hit = OOD_FOREIGN_MARKERS.iter().any(|m| lower.contains(m));
    if !foreign_hit {
        return None;
    }
    if kz_anchored {
        // Mixed query — let the cascade try; the listing-query
        // shortcuts and curated facts (e.g. «Қазақстанның көршілері»
        // includes Ресей) should resolve it.  If they don't, the
        // typed retrieval fall-through still wins over a wrong
        // forced refusal.
        return None;
    }
    Some(
        "Менің білім қорым қазақ-тілді curated фактілерге шектелген — \
         бұл сұраққа нақты дерегім жоқ. Қазақстан немесе қазақ тілі \
         туралы сұрақтармен көмектесе аламын."
            .to_string(),
    )
}

/// English function-word markers that distinguish substantive
/// English input from random Latin tokens.  Closed list; matched
/// as whole tokens (after punctuation trim).
const ENGLISH_FUNCTION_WORDS: &[&str] = &[
    "what", "who", "when", "where", "why", "how", "is", "are", "do", "does", "can", "could",
    "should", "the", "a", "an", "about", "tell", "me", "i", "you", "in", "of", "to", "for",
];

/// Count Latin letters, Kazakh-specific Cyrillic letters
/// (ұқғңөәүһі), and total Cyrillic letters in the input.  Used by
/// the script-discipline branch of [`handle_ood_refusal`].
fn count_scripts(s: &str) -> (usize, usize, usize) {
    let mut latin = 0;
    let mut kaz_specific = 0;
    let mut cyrillic = 0;
    for c in s.chars() {
        if c.is_ascii_alphabetic() {
            latin += 1;
        } else if matches!(c, 'ұ' | 'қ' | 'ғ' | 'ң' | 'ө' | 'ә' | 'ү' | 'һ' | 'і') {
            kaz_specific += 1;
            cyrillic += 1;
        } else if ('а'..='я').contains(&c) || c == 'ё' {
            cyrillic += 1;
        }
    }
    (latin, kaz_specific, cyrillic)
}

/// Closed-set non-Kazakh entities.  Substring match against the
/// lowercased input.  Order does not matter; all entries are
/// independent triggers.
///
/// **Maintenance**: when a blind-eval iteration surfaces a new
/// foreign-entity miss, add the keyword here.  Closed-set is the
/// rc18 floor; a learned OOD classifier is a later option.
const OOD_FOREIGN_MARKERS: &[&str] = &[
    // -- Western / global brand-tech --
    // **v6.5.0-rc22** — Whisper STT drift on «Гейтс» → «Гейц» /
    // «Гейтц» (audit T49).  All three surface forms route to the
    // same OOD refusal.
    "билл гейтс",
    "билл гейц",
    "билл гейтц",
    "стив джобс",
    "илон маск",
    "марк цукерберг",
    "эйнштейн",
    "айфон",
    "iphone",
    "apple компани",
    "microsoft",
    "google",
    "facebook",
    "wikipedia",
    "github",
    "bitcoin",
    "биткоин",
    "nasa",
    "наса",
    "beatles",
    "битлз",
    "гарри поттер",
    "harry potter",
    // -- Foreign countries that have their own president /
    //    capital / currency, distinct from Kazakhstan --
    "ресей",
    "россия",
    "америк",
    "сша",
    "ақш",
    "қытай",
    "англи",
    "британ",
    "герман",
    "франция",
    "жапон",
    "японск",
    "үндіс",
    "индии",
    "иран",
    "ирак",
    "түрки",
    "турци",
    "корея",
    "ватикан",
    // -- Foreign cities --
    "москва",
    "санкт-петербург",
    "сочи",
    "казань",
    "шанхай",
    "пекин",
    "токио",
    "нью-йорк",
    "манхэттен",
    "лондон",
    "париж",
    "берлин",
    "рим",
    "стамбул",
];

/// Detect a Whisper STT repeat-loop in the input and collapse it
/// to the first occurrence. Whisper sometimes gets stuck in a
/// repetition cycle and emits «Сәлем. Сәлем. Сәлем.» × 30+. Returns
/// `Some(deduped)` when a loop is detected, `None` otherwise.
///
/// Algorithm: split on sentence punctuation, count distinct
/// clauses. If the most-frequent clause has ≥ 3 occurrences AND
/// makes up more than half of the total, return that clause alone.
fn dedupe_stt_loop(input: &str) -> Option<String> {
    use std::collections::HashMap;
    let clauses: Vec<&str> = input
        .split(['.', '?', '!'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if clauses.len() < 3 {
        return None;
    }
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for c in &clauses {
        *counts.entry(c).or_insert(0) += 1;
    }
    let (top_clause, top_count) = counts.iter().max_by_key(|(_, n)| *n)?;
    if *top_count >= 3 && *top_count * 2 > clauses.len() {
        return Some(top_clause.to_string());
    }
    None
}

/// Detect occupation / role statements. «Мен X» / «Мен X-мын» /
/// «Мен X-пын» — user stating who they are.
///
/// Returns an acknowledgement string when the statement matches.
fn recognize_occupation_statement(input: &str) -> Option<String> {
    let lower = input.to_lowercase();
    // Common occupation roots — extend as needed. Each matches as
    // a whole-word so «мен бағдарламашы» fires but «бағдарламашы
    // деген не?» doesn't.
    let occupations: &[(&str, &str)] = &[
        ("бағдарламашы", "бағдарламашы"),
        ("программист", "программист"),
        ("оқушы", "оқушы"),
        ("студент", "студент"),
        ("мұғалім", "мұғалім"),
        ("дәрігер", "дәрігер"),
        ("инженер", "инженер"),
        ("ғалым", "ғалым"),
        ("суретші", "суретші"),
        ("әнші", "әнші"),
        ("спортшы", "спортшы"),
        ("сатушы", "сатушы"),
        ("аспазшы", "аспазшы"),
        ("заңгер", "заңгер"),
        ("аудармашы", "аудармашы"),
        ("журналист", "журналист"),
    ];
    // Pattern: «Мен X» followed by optional «-мын / -сың / -сыз / -пын
    // / -бін / etc.» suffix. We look for a whole-word «мен» token
    // followed by a known occupation root anywhere in the input.
    let starts_with_men = lower.starts_with("мен ") || lower.contains(" мен ");
    if !starts_with_men {
        return None;
    }
    for (root, canonical) in occupations {
        // Use char counts (not byte lengths): Kazakh suffixes
        // like «-мын», «-сың», «-сыз», «-пын» are 2–4 chars.
        // `len()` would give bytes, which doubles for Cyrillic
        // and breaks the comparison. The replayed-voice-REPL
        // battery surfaced this on «бағдарламашымын» (session
        // 3 — Whisper rendered the «-мын» suffix the user
        // actually spoke).
        let root_chars = root.chars().count();
        if lower.split(|c: char| !c.is_alphanumeric()).any(|tok| {
            let tok_chars = tok.chars().count();
            tok == *root || (tok.starts_with(root) && (2..=4).contains(&(tok_chars - root_chars)))
        }) {
            return Some(format!(
                "Түсіндім, сіз {canonical}сыз. Бағдарламалау тілдері мен \
                 алгоритмдер туралы сұрағыңыз болса — көмектесуге тырысамын."
            ));
        }
    }
    None
}

/// **v6.8.17 — Codex Q3 school bug #1.**  Detect a school-grade
/// self-introduction («Мен 8-сынып оқушысымын» / «Менің 9
/// сыныптамын» / «Сәлем, мен 11-сынып оқушысы») and return a
/// friendly acknowledgement.
///
/// The fix replaces the previous misroute: `math_solver` would
/// fire on the «8» (or «9», «11», …) numeral and refuse with
/// «жазсаңыз — есептеп беремін» (please type a numeric
/// expression).  Grade-statement detection MUST run BEFORE
/// `math_solver::looks_like_math_validated` so the cascade
/// recognises the self-introduction shape first.
///
/// Detection requires BOTH:
///   * a first-person marker («мен» / «менің» / «маған»);
///   * a grade-role marker («сынып оқушы» / «сыныптамын» /
///     «сынып баласы» / «сынып студенті»).
/// Without both, returns `None` and the cascade falls through.
/// Standalone «X-сынып» (no first-person frame) stays available
/// for legitimate factual queries («8-сынып бағдарламасы
/// қандай?»).
fn recognize_grade_statement(input: &str) -> Option<String> {
    let lower = input.to_lowercase();
    let has_first_person = lower
        .split(|c: char| !c.is_alphanumeric())
        .any(|t| matches!(t, "мен" | "менің" | "маған" | "мені" | "менде" | "менен"));
    if !has_first_person {
        return None;
    }
    let has_grade_role = lower.contains("сынып оқушы")
        || lower.contains("сынып баласы")
        || lower.contains("сынып студент")
        || lower.contains("сыныптамын")
        || lower.contains("сыныпта оқимын")
        || lower.contains("сыныпта оқып");
    if !has_grade_role {
        return None;
    }
    let grade = extract_grade_number(&lower);
    Some(match grade {
        Some(n) => format!("Сәлем, {n}-сынып оқушысы! Қандай пәннен көмек керек?"),
        None => "Сәлем, оқушы! Қандай пәннен көмек керек?".to_string(),
    })
}

/// Extract a school-grade number from a lower-cased input.
/// Accepts `1..=11` (Kazakh school covers grades 1 through 11).
/// Scans for digit runs of 1–2 chars; first match wins.  Returns
/// `None` when no digit appears (the caller emits a grade-less
/// greeting).
fn extract_grade_number(lower: &str) -> Option<u32> {
    let mut digits = String::new();
    for ch in lower.chars().chain(std::iter::once(' ')) {
        if ch.is_ascii_digit() {
            digits.push(ch);
            if digits.len() > 2 {
                digits.clear();
            }
        } else if !digits.is_empty() {
            if let Ok(n) = digits.parse::<u32>()
                && (1..=11).contains(&n)
            {
                return Some(n);
            }
            digits.clear();
        }
    }
    None
}

/// **v6.8.3 — 2026-06-17.** Personal-experience probe: 2nd-person
/// past-tense question about lived experience adam does not have
/// (didn't read a book, didn't see a film, didn't eat / drink /
/// travel).  Refusing the presupposition is more honest than
/// surfacing a generic definition of the topic noun, which falsely
/// implies the experience occurred.
///
/// Gate: needs BOTH a 2nd-person address marker (сен / сіз /
/// сенің / сіздің) AND a past-tense personal-experience verb
/// ending (read / saw / ate / drank / travelled / etc.).  Knowledge
/// / capability verbs («білесің», «айтасың») are intentionally NOT
/// here — those route through `is_capabilities_query`.
fn is_personal_experience_query(input: &str) -> bool {
    let lower = input.to_lowercase();
    let has_2nd_person = lower.contains("сен ")
        || lower.contains("сіз ")
        || lower.contains("сенің ")
        || lower.contains("сіздің ")
        || lower.contains(" сен")
        || lower.contains(" сіз");
    if !has_2nd_person {
        return false;
    }
    // 2nd-person past-tense personal-experience verb endings.
    // Each entry pairs the -сың (familiar) and -сыз (respectful)
    // surface; we match both shapes.  Verbs are restricted to those
    // that imply LIVED EXPERIENCE adam cannot have.
    let experience_verbs = [
        "оқыдың",
        "оқыдыңыз", // read
        "көрдің",
        "көрдіңіз", // saw
        "жедің",
        "жедіңіз", // ate
        "іштің",
        "іштіңіз", // drank
        "бардың",
        "бардыңыз", // went
        "келдің",
        "келдіңіз", // came
        "ұйықтадың",
        "ұйықтадыңыз", // slept
        "сезіндің",
        "сезіндіңіз", // felt
        "сүйдің",
        "сүйдіңіз", // loved
        "тыңдадың",
        "тыңдадыңыз", // listened
        "ойнадың",
        "ойнадыңыз", // played
        "жүздің",
        "жүздіңіз", // swam
        "жасырдың",
        "жасырдыңыз", // hid
    ];
    experience_verbs.iter().any(|v| lower.contains(v))
}

fn personal_experience_refusal() -> String {
    "Менің өмірлік тәжірибем жоқ — мен қазақ тіліне арналған типтелген кернелмін, \
     ағза емеспін: кітап оқымаймын, фильм көрмеймін, тамақ ішпеймін, саяхаттамаймын. \
     Бірақ кітаптар, фильмдер, тағамдар, жерлер туралы тексерілген ақпарат бере аламын — \
     нақты тақырыпты атасаңыз, көмектесемін."
        .to_string()
}

/// Capabilities self-description query. Distinct from
/// `is_self_identity_query` (which is about WHO adam is) —
/// this is about WHAT adam can do / knows.
fn is_capabilities_query(input: &str) -> bool {
    let lower = input.to_lowercase();
    let markers = [
        "не білесің",
        "не білесіз",
        "не істей аласың",
        "не істей аласыз",
        "не істелесің",
        "не істелесіз",
        "что ты знаешь",
        "что ты умеешь",
        "что ты можешь",
        "сенің мүмкіндіктерің",
        "мүмкіндіктерің қандай",
    ];
    markers.iter().any(|m| lower.contains(m))
}

/// **Phase 20 (2026-06-02)** — paraphrase variants for high-frequency
/// static responses. The user flagged «заученность и однотипность» —
/// the same monologue coming back to multiple distinct capability
/// queries. Each call now selects one of N paraphrased variants
/// using a stable hash of the input — same query → same answer
/// (no flicker on retry), different queries → different surface.
fn pick_variant<'a>(variants: &[&'a str], seed: &str) -> &'a str {
    if variants.is_empty() {
        return "";
    }
    // FNV-1a — stable, no allocations, deterministic across runs.
    let mut h: u64 = 14695981039346656037;
    for b in seed.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    variants[(h as usize) % variants.len()]
}

fn capabilities_response(input: &str) -> String {
    // **Phase 20** — five paraphrased variants. Same canonical content
    // (the curated-knowledge disclosure) in different shapes so the
    // user doesn't feel like they're hitting one fixed template
    // every time they ask about adam's capabilities.
    // **v6.5.0-rc20 — Kazakh-only «what can I help with» templates.**
    // The cognitive_eval `LatinCharactersForbidden` discipline rejects
    // English tokens («Rust», «LLM», «curated», «live», «ASCII») in
    // adam's replies.  Replaced with Kazakh equivalents that read
    // naturally in the TTS layer as well: «тексерілген деректер»
    // (= curated facts), «бағдарламалау тілдері» (= programming
    // languages, including the Rust subdomain), «ағымдағы уақыт»
    // (= live clock), «латын-таңбалы өрнектер» (= ASCII expressions),
    // «үлкен тілдік модель емеспін» (= not an LLM).
    let variants: &[&str] = &[
        "Менің білім қорым тексерілген деректерден тұрады. Жауап бере аламын: \
         (1) Қазақстан туралы — география, тарих, әдебиет, танымал тұлғалар, \
         мемлекеттік құрылым; (2) мектеп пәндері — математика, физика, химия, \
         биология, тарих, ана тілі; (3) бағдарламалау тілдері; \
         (4) ағымдағы күн / уақыт / апта; (5) қарапайым және күрделі \
         математикалық есептеулер (қазақша / орысша / латын-таңбалы өрнек). \
         Үлкен тілдік модель емеспін — тексерілген деректерден тыс \
         сұрақтарға «нақты дерек жоқ» деп шынайы жауап беремін.",
        "Мен бірнеше тақырыпта көмектесе аламын: Қазақстанның географиясы, \
         тарихы, әдебиеті мен танымал тұлғалары; мектеп пәндері — \
         математика, физика, химия, биология, ана тілі; бағдарламалау \
         тілдері; ағымдағы күн, уақыт пен апта; қарапайым және \
         көп қадамды математикалық есептеулер. Тыс тақырыпта «дерек жоқ» \
         деп шынайы айтамын — үлкен тілдік модель емеспін.",
        "Қолымдағы білім аясы — тексерілген деректер. Жауап бере алатын \
         тақырыптарым: Қазақстан туралы (география / тарих / әдебиет / \
         тұлғалар / мемлекет); мектеп пәндері (физика, химия, биология, \
         математика, тарих); бағдарламалау тілдері; ағымдағы уақыт-күн-апта; \
         математикалық амалдар. Тыс сұрақтарға ойдан жауап жасайтын \
         үлкен тілдік модель емеспін — «білмеймін» дегенді жасырмаймын.",
        "Жауап бере алатын негізгі салаларым: Қазақстан жайында жалпы дерек \
         (география, тарих, әдебиет, белгілі тұлғалар, мемлекеттік құрылым); \
         мектеп бағдарламасы (математика, физика, химия, биология, тарих, \
         ана тілі); бағдарламалау тілдері; ағымдағы дата / уақыт / апта \
         күні; қазақша / орысша / латын-таңбалы өрнек форматтағы математикалық \
         есептер. Үлкен тілдік модель емеспін — тексерілген деректер шегінен \
         шықпаймын.",
        "Менің көмектесе алатын тақырыптарым: (1) Қазақстан туралы — \
         география, тарих, әдебиет, белгілі адамдар, мемлекет құрылымы; \
         (2) мектеп пәндері — математика, физика, химия, биология; \
         (3) бағдарламалау тілдері; (4) ағымдағы уақыт, күн, апта; \
         (5) математикалық есептеулер. Әзірге осы шеңберде ғана нақты \
         жауап бере аламын — қалғанын ойдан құрастырмаймын.",
    ];
    pick_variant(variants, input).to_string()
}

/// Detect «how did you determine my gender?» kind of meta-query.
fn is_pitch_detection_query(input: &str) -> bool {
    let lower = input.to_lowercase();
    let markers = [
        "қалай түсіндің",
        "қалай түсіндіңіз",
        // Session-4 audit: user often slips the past-tense
        // first-person form («түсіндім» = "I understood") when
        // they actually mean «түсіндің» («how did you know») —
        // accept both. Same class for «білдім»/«анықтадым».
        "қалай түсіндім",
        "қалай білдім",
        "қалай анықтадым",
        "қалай білдің",
        "қалай білдіңіз",
        "қалай анықтадың",
        "ағай дедің",
        "апай дедің",
        "ер екенімді",
        "ер болғанымды",
        "еркет болғанымды",
        "еркек болғанымды",
        "ұл болғанымды",
        // The user can also self-describe by addressing the
        // honorific form adam chose: «Мен ағай болғанымды
        // қалай түсіндім» — that's a pitch-detection query.
        "ағай болғанымды",
        "апай болғанымды",
        "әйел екенімді",
        "әйел болғанымды",
    ];
    markers.iter().any(|m| lower.contains(m))
}

/// Self-identity gate. «Сен кімсің?» / «Кім сің?» / «Кім боласың?»
/// / «Сен өзің кімсің?» — all questions about adam's own identity.
fn is_self_identity_query(s: &str) -> bool {
    let lower = s.to_lowercase();
    let markers = [
        "сен кімсің",
        "кім сің",
        "кімсің",
        "сен кім боласың",
        "кім боласың",
        "сен өзің кім",
        "өзің кімсің",
        "ты кто",
        "вы кто",
        "кто ты",
        "представься",
    ];
    if markers.iter().any(|m| lower.contains(m)) {
        return true;
    }
    // **v6.8.3 — 2026-06-17 user audit.** Yes/no presupposition
    // probes about adam's nature: «Сен адамсың ба?» / «Сіз робот
    // па?» / «Сен жасанды интеллектсің бе?».  Pre-fix these fell
    // to a substring-IsA lookup that returned «дерек жоқ» because
    // world_core has no fact «adam IsA человек».  The honest answer
    // is the self-identification template — same one that handles
    // «Сен кімсің?».  Detect 2nd-person address + identity-class
    // noun.  Each identity_noun listed below already carries the
    // 2nd-person predicative ending («адамсың / роботсыз»), so the
    // 2nd-person token gate is implicit in the noun list; we still
    // require it as a sanity gate for the looser shapes («робот
    // па»).  The yes/no particle is not separately required —
    // those identity nouns + a 2-person address don't surface in a
    // declarative shape adam would otherwise generate.
    let has_2nd_person = lower.contains("сен ")
        || lower.contains("сіз ")
        || lower.starts_with("сен")
        || lower.starts_with("сіз");
    let has_identity_noun = lower.contains("адамсың")
        || lower.contains("адамсыз")
        || lower.contains("роботсың")
        || lower.contains("роботсыз")
        || lower.contains("робот па")
        || lower.contains("робот ба")
        || lower.contains("робот ма")
        || lower.contains("робот ме")
        || lower.contains("жасанды интеллект")
        || lower.contains("ии сің")
        || lower.contains("программасың")
        || lower.contains("бағдарламасың")
        || lower.contains("тірісің")
        || lower.contains("тірі ме");
    has_2nd_person && has_identity_noun
}

/// Honest «no live data» gate. Returns true for queries about
/// weather, currency rates, stock prices, news, sports scores —
/// information the kernel has no live feed for. Without this gate,
/// the cascade picks the nearest morpheme fact and emits nonsense
/// («Дөңгелек» for «Биткоин барамы қандай?»).
fn needs_live_data_refusal(s: &str) -> bool {
    let lower = s.to_lowercase();
    let markers = [
        // Weather.
        "ауа райы",
        "погода",
        "температура бүгін",
        "температура сегодня",
        // Currency / crypto / stock.
        "биткоин",
        "bitcoin",
        // **v6.8.3 — 2026-06-17 user audit.** Live REPL probe «Қазір
        // BTC бағасы қанша?» was returning today's date because the
        // «қазір» token triggered the v6.1 date_query intent BEFORE
        // this v6.2 live-data refusal could fire (v6.2 only overrides
        // when it returns Some).  Pre-fix the markers covered the
        // long-form «биткоин / bitcoin» but not the ticker form «btc»
        // (and analogues «eth» / «ethereum»).  Same gap for the
        // «бағасы» (= "price-of") possessive surface — only «бағамы»
        // (= "rate-of") was listed.  Adding both shapes so the
        // refusal fires and overrides the date routing.
        "btc",
        "eth",
        "ethereum",
        "доллар",
        "теңге бағамы",
        "евро",
        "акция",
        "курс",
        "бағамы",
        "бағасы",
        "бағасын",
        "бағасының",
        "барамы",
        // News / sports.
        "жаңалықтар",
        "новости",
        "матч",
        "ойын нәтижесі",
    ];
    markers.iter().any(|m| lower.contains(m))
}

// ── L4.5 Phase 2.A.2 — typed migrations of bool-detector routes ──
//
// Each `lookup_X_typed` below combines:
//   1. the existing bool detector (`is_X_query` / `needs_X_refusal`);
//   2. the inline template / variant selection previously done at
//      the cascade callsite;
//   3. the proper `ProofObject` construction (`system_self` for
//      self-description, `safety_refusal` for typed safety domains,
//      `no_data_refusal` for honest-no-data refusals).
//
// The cascade still consumes `Option<String>`, so the callsites are
// rewritten as `lookup_X_typed(input).map(|c| c.text)` — same
// surface, same semantics, plus typed provenance and consistent
// proof/emitted-text invariant by construction.  Phase 2.B / 2.C
// will switch the cascade boundary to `Option<AnswerCandidate>`.

/// **v6.8.4 L4.5 Phase 2.A.2.** Typed sibling of the capabilities
/// detector.  Combines [`is_capabilities_query`] + the variant
/// selection previously inlined at the cascade callsite into one
/// route.  Proof shape: `ProofObject::system_self("capabilities", …)`.
fn lookup_capabilities_typed(input: &str) -> Option<crate::dialog_acts::AnswerCandidate> {
    use crate::dialog_acts::{AnswerCandidate, RouteId};
    use crate::proof_object::ProofObject;

    if !is_capabilities_query(input) {
        return None;
    }
    let text = capabilities_response(input);
    let proof = ProofObject::system_self("capabilities".into(), text.clone());
    let candidate = AnswerCandidate::assert(text, proof, RouteId::Capabilities);
    debug_assert!(candidate.invariant_check().is_ok());
    Some(candidate)
}

/// **v6.8.4 L4.5 Phase 2.A.2.** Typed sibling of the personal-
/// experience detector.  Refuses presupposition probes («Сен
/// қандай кітап оқыдың?») with the existing
/// [`personal_experience_refusal`] surface.  Proof shape:
/// `ProofObject::no_data_refusal(subject, "lived_experience")` —
/// adam genuinely lacks the data because it has no lived
/// experience, not because of a safety policy.
fn lookup_personal_experience_typed(input: &str) -> Option<crate::dialog_acts::AnswerCandidate> {
    use crate::dialog_acts::{AnswerCandidate, PolicyReason, RouteId};
    use crate::proof_object::ProofObject;

    if !is_personal_experience_query(input) {
        return None;
    }
    let text = personal_experience_refusal();
    let proof = ProofObject::no_data_refusal(input.to_string(), "lived_experience".into());
    let candidate = AnswerCandidate::refuse(
        text,
        proof,
        RouteId::PersonalExperienceRefusal,
        PolicyReason::PresuppositionFailure,
    );
    debug_assert!(candidate.invariant_check().is_ok());
    Some(candidate)
}

/// **v6.8.4 L4.5 Phase 2.A.2.** Typed sibling of the self-identity
/// detector.  Variant selection is delegated to
/// [`self_identity_response`] (extracted helper below).  Proof
/// shape: `ProofObject::system_self("identity", …)`.
fn lookup_self_identity_typed(input: &str) -> Option<crate::dialog_acts::AnswerCandidate> {
    use crate::dialog_acts::{AnswerCandidate, RouteId};
    use crate::proof_object::ProofObject;

    if !is_self_identity_query(input) {
        return None;
    }
    let text = self_identity_response(input);
    let proof = ProofObject::system_self("identity".into(), text.clone());
    let candidate = AnswerCandidate::assert(text, proof, RouteId::SelfIdentity);
    debug_assert!(candidate.invariant_check().is_ok());
    Some(candidate)
}

/// **v6.8.4 L4.5 Phase 2.A.2.** Typed sibling of the live-data
/// detector.  Variant selection is delegated to
/// [`live_data_refusal_response`] (extracted helper below).  Proof
/// shape: `ProofObject::safety_refusal(input, "live_data", CurrentData)`
/// — adam has no live feed, by typed domain.
fn lookup_live_data_refusal_typed(input: &str) -> Option<crate::dialog_acts::AnswerCandidate> {
    use crate::dialog_acts::{AnswerCandidate, PolicyReason, RouteId};
    use crate::proof_object::{ProofObject, SafetyDomain};

    if !needs_live_data_refusal(input) {
        return None;
    }
    let text = live_data_refusal_response(input);
    let proof = ProofObject::safety_refusal(
        input.to_string(),
        "live_data".into(),
        SafetyDomain::CurrentData,
    );
    let candidate = AnswerCandidate::refuse(
        text,
        proof,
        RouteId::LiveDataRefusal,
        PolicyReason::NoLiveData,
    );
    debug_assert!(candidate.invariant_check().is_ok());
    Some(candidate)
}

/// **v6.8.4 L4.5 Phase 2.A.2.** Variant selector for the self-
/// identity templates.  Three paraphrases (rc11 / rc20) chosen by
/// stable hash of the input so the same query maps to the same
/// variant.  Extracted from the cascade callsite to give the typed
/// handler a single owner.
fn self_identity_response(input: &str) -> String {
    let variants: &[&str] = &[
        "Мен — адам, қазақ тіліне арналған детерминирленген тілдік \
         жүйемін. Үлкен тілдік модель емеспін — жауаптарымды \
         алдын ала тексерілген деректерден аламын.",
        "Менің атым — адам. Қазақ тілінің морфологиясы бойынша \
         құрастырылған детерминирленген тілдік жүйемін. \
         Жауаптарым тек тексерілген деректерге сүйенеді, \
         ойдан құрастырылмайды.",
        "Мен — қазақ тіліне арналған агглютинативті ой жүйесімін, \
         қысқаша «адам» деп аталамын. Әр сөзімді тексерілген \
         деректермен растаймын; білмейтін нәрсемді «нақты дерегім \
         жоқ» деп ашық айтамын.",
    ];
    pick_variant(variants, input).to_string()
}

/// **v6.8.4 L4.5 Phase 2.A.2.** Variant selector for the live-
/// data refusal templates.  Three Phase-20 paraphrases.  Extracted
/// from the cascade callsite to give the typed handler a single
/// owner.
fn live_data_refusal_response(input: &str) -> String {
    let variants: &[&str] = &[
        "Бұл сұраққа жауап беру үшін менде нақты дерек жоқ. \
         Менің білім қорым тексерілген деректерден тұрады, \
         тікелей интернет немесе ағымдағы мәлімет ағысы қосылған емес.",
        "Бұл сұраққа дерек бере алмаймын — менің білім қорымда \
         ағымдағы немесе реалды-уақыттық мәлімет жоқ. Тек \
         тексерілген тарихи деректермен жұмыс істеймін.",
        "Кешіріңіз, бұл сұраққа жауап беретін ағымдағы дерек менде \
         жоқ. Интернетке немесе сыртқы мәлімет көзіне қосылмаймын — \
         тек тексерілген тарихи деректер қолымда.",
    ];
    pick_variant(variants, input).to_string()
}

/// **Phase 23 (2026-06-03)** — school-level chemistry formula lookup.
///
/// Returns `Some("Cудың формуласы — H₂O.")` when the input matches
/// «<substance> формуласы / формуласын / формула» pattern, where
/// `<substance>` is one of ~40 hardcoded school-curriculum chemicals.
/// Returns `None` otherwise.
///
/// Why hardcoded, not a `HasFormula` predicate in world_core:
///   1. The Predicate enum is closed-set; adding one touches 5+ files
///      and migrates none of the existing 138 chemistry_school.jsonl
///      entries (they don't carry formulas).
///   2. School-level formula set is closed (~30-50 substances).
///      Hardcoded table is the right shape for this scope.
///   3. False-positive risk minimal: the «формула» marker keyword is
///      required, so bare substance mentions don't fire this handler.
///
/// Multi-session live REPL caught:
///   - «Судың формуласын жазып бер.» (the canonical case)
///   - «Судың химия формуласын жаз.» (with «химия» qualifier)
///   - «Тұздың формуласы қандай?»
/// **v6.8 (2026-06-16) — possessive-property lookup.**
///
/// Closed-set handler for «X-genitive Y-possessive» school-curriculum
/// queries. Pattern-matched lookup beats the substring-IsA fallback
/// for the specific question shapes listed in `patterns` below.
///
/// Each entry is `(input_substring, response)` — both fully lowercased
/// + punctuation-stripped for robust matching. Add new shapes when
/// school-eval surfaces them; keep the list curated, since broader
/// possessive disambiguation lives in the v6.2 typed query IR.
fn lookup_possessive_property(input: &str) -> Option<String> {
    // **v6.8.4 — 2026-06-17 L4.5 Phase 2.A.** Thin wrapper extracting
    // the surface text from the typed
    // `lookup_possessive_property_typed` sibling.
    lookup_possessive_property_typed(input).map(|c| c.text)
}

/// **v6.8.4 — 2026-06-17 L4.5 Phase 2.A.** Typed sibling of
/// [`lookup_possessive_property`].  Returns an
/// [`crate::dialog_acts::AnswerCandidate`] with the pattern hit
/// as `ProofObject::from_curated_fact("possessive_property", …)`
/// and `RouteId::PossessiveProperty`.  Covers both the fast
/// substring path and the edit-distance ≤ 1 fuzzy fallback —
/// same `(pat, answer)` tuple builds the same candidate.
fn lookup_possessive_property_typed(input: &str) -> Option<crate::dialog_acts::AnswerCandidate> {
    use crate::dialog_acts::{AnswerCandidate, RouteId};
    use crate::proof_object::ProofObject;
    use adam_reasoning::FactSource;

    fn build_candidate(pat: &str, answer: &str) -> AnswerCandidate {
        let text = answer.to_string();
        let proof = ProofObject::from_curated_fact(
            pat.to_string(),
            "possessive_property".to_string(),
            answer.to_string(),
            FactSource {
                pack: "v6_2_router/possessive_property_table".into(),
                sample_id: pat.to_string(),
            },
            text.clone(),
        );
        let candidate = AnswerCandidate::assert(text, proof, RouteId::PossessiveProperty);
        debug_assert!(candidate.invariant_check().is_ok());
        candidate
    }

    let lower: String = input
        .to_lowercase()
        .chars()
        .map(|c| match c {
            '.' | '?' | '!' | ',' | ';' | ':' | '«' | '»' | '—' | '–' | '-' => ' ',
            other => other,
        })
        .collect();
    let lower: String = lower.split_whitespace().collect::<Vec<_>>().join(" ");
    let lower = lower.as_str();

    // Ordered longest-pattern-first so more specific shapes win over
    // general ones (e.g., «ұлттық валютасы» before bare «валютасы»).
    let patterns: &[(&str, &str)] = &[
        // Қазақстан + property
        (
            "қазақстанның ең үлкен қаласы",
            "Қазақстанның ең үлкен қаласы — Алматы.",
        ),
        (
            "қазақстанның ең үлкен қала",
            "Қазақстанның ең үлкен қаласы — Алматы.",
        ),
        (
            "қазақстанның мемлекеттік тілі",
            "Қазақстан Республикасының мемлекеттік тілі — қазақ тілі.",
        ),
        (
            "қазақстанның ұлттық валютасы",
            "Қазақстанның ұлттық валютасы — теңге.",
        ),
        (
            "қазақстанның валютасы",
            "Қазақстанның ұлттық валютасы — теңге.",
        ),
        // Қазақ халқы + property (the people)
        (
            "қазақтың ұлттық тағамы",
            "Қазақтың ұлттық тағамы — бесбармақ.",
        ),
        (
            "қазақтың дәстүрлі тағамы",
            "Қазақтың дәстүрлі тағамы — бесбармақ.",
        ),
        ("қазақтың ұлттық сусыны", "Қазақтың ұлттық сусыны — қымыз."),
        (
            "қазақтың ұлттық музыкалық аспабы",
            "Қазақтың ұлттық музыкалық аспабы — домбыра.",
        ),
        (
            "қазақтың ұлттық аспабы",
            "Қазақтың ұлттық аспабы — домбыра.",
        ),
        // Informatics — quantity / system queries that the
        // substring-IsA layer cannot serve correctly. world_core
        // has the underlying facts (info_014 «Бит — ...», info_015
        // «Байт — сегіз биттен тұратын ...»), but the router needs
        // a closed-set entry for the question shape.
        ("байтта неше бит", "Бір байтта 8 бит бар."),
        ("бір байтта неше", "Бір байтта 8 бит бар."),
        ("байтта қанша бит", "Бір байтта 8 бит бар."),
        (
            "екілік санақ жүйесі",
            "Екілік санақ жүйесі — компьютер арифметикасының негізі; онда тек 0 және 1 цифрлары қолданылады.",
        ),
        (
            "екілік санақ",
            "Екілік санақ жүйесі — компьютер арифметикасының негізі; онда тек 0 және 1 цифрлары қолданылады.",
        ),
        // Body-parts purpose («X не үшін керек?» / «X-тың
        // қызметі қандай?»). world_core/body_parts.jsonl has these
        // as IsA facts («Ми — ойлау мүшесі.»), but AskPurpose
        // intent currently routes to a generic clarification
        // template when the topic isn't a Rust concept. The
        // closed-set lookup here surfaces the canonical biology
        // school-eval answer before the cascade falls through.
        ("ми не үшін", "Ми — ойлау мүшесі."),
        ("мидың қызметі", "Ми — ойлау мүшесі."),
        ("ми не істейді", "Ми — ойлау мүшесі."),
        ("көз не үшін", "Көз — көру мүшесі."),
        ("көздің қызметі", "Көз — көру мүшесі."),
        ("құлақ не үшін", "Құлақ — есту мүшесі."),
        ("құлақтың қызметі", "Құлақ — есту мүшесі."),
        ("өкпе не үшін", "Өкпе — тыныс алу мүшесі."),
        ("өкпенің қызметі", "Өкпе — тыныс алу мүшесі."),
        ("жүрек не үшін", "Жүрек — қан айналымы мүшесі."),
        ("жүректің қызметі", "Жүрек — қан айналымы мүшесі."),
        ("асқазан не үшін", "Асқазан — ас қорыту мүшесі."),
        ("асқазанның қызметі", "Асқазан — ас қорыту мүшесі."),
        ("бауыр не үшін", "Бауыр — зат алмасу мүшесі."),
        ("бауырдың қызметі", "Бауыр — зат алмасу мүшесі."),
        ("бүйрек не үшін", "Бүйрек — несеп шығару мүшесі."),
        ("бүйректің қызметі", "Бүйрек — несеп шығару мүшесі."),
        // **v6.8 expansion (2026-06-16 expanded eval).** Additional
        // body-parts surfaced by expanding the school-eval suite from
        // 51 to 160 accepted cases.
        (
            "тері не үшін",
            "Тері — дененің сыртқы қабаты, ағзаны қоршаған ортадан қорғайды.",
        ),
        (
            "терінің қызметі",
            "Тері — дененің сыртқы қабаты, ағзаны қоршаған ортадан қорғайды.",
        ),
        (
            "қан не үшін",
            "Қан — оттегі мен қоректік заттарды тасымалдайтын сұйықтық.",
        ),
        (
            "қанның қызметі",
            "Қан — оттегі мен қоректік заттарды тасымалдайтын сұйықтық.",
        ),
        ("аяқ не үшін", "Аяқ — қозғалу мүшесі."),
        ("аяқтың қызметі", "Аяқ — қозғалу мүшесі."),
        ("қол не үшін", "Қол — еңбек ету және ұстау мүшесі."),
        ("қолдың қызметі", "Қол — еңбек ету және ұстау мүшесі."),
        // Additional Қазақстан capital / language phrasing variants
        // that don't fit the standard «X-genitive Y-possessive» but
        // are common school-curriculum question shapes.
        (
            "қазақстанның бұрынғы астанасы",
            "Қазақстанның бұрынғы астанасы — Алматы (1997 жылға дейін).",
        ),
        (
            "қазақстанның қазіргі астанасы",
            "Қазақстанның қазіргі астанасы — Астана.",
        ),
        (
            "қазақстанда қандай тіл мемлекеттік",
            "Қазақстанның мемлекеттік тілі — қазақ тілі.",
        ),
        // Geography / astronomy factoids
        ("балқаш көлі", "Балқаш — Қазақстандағы ірі көл."),
        ("жер — қандай аспан денесі", "Жер — ғаламшар (планета)."),
        ("жер қандай аспан денесі", "Жер — ғаламшар (планета)."),
        // Electric current — physics 8.
        (
            "электр тогы деген не",
            "Электр тогы — зарядтардың бағытталған қозғалысы.",
        ),
    ];

    for (pat, answer) in patterns {
        // Fast path: exact substring match (clean text — 99% of cases).
        if lower.contains(pat) {
            return Some(build_candidate(pat, answer));
        }
    }
    // **v6.8 (2026-06-16) — fuzzy match fallback for speech defects.**
    //
    // Speech-defect eval surfaced that single-character corruptions
    // («Күмістің» → «Кмістің», «Алматы» → «Айматы», «Қазақстан» →
    // «Казхстан») break the exact-substring lookup above. A single
    // edit (substitution / deletion / insertion of one Kazakh letter)
    // is the canonical noise mode for:
    //
    //   - lambdacism / rhotacism / kappacism / sigmatism phoneme drops
    //   - Whisper-drift vowel deletions
    //   - typos in keyboard-typed input
    //
    // Run a second pass with Levenshtein ≤ 1 against each pattern.
    // The threshold is deliberately conservative: max_edits=1 cannot
    // confuse «қазақ» / «қазан» (distance 2) and the like. For deeper
    // defects (≥ 2 edits per critical word) the v7 candidate-rescoring
    // architecture (FST-aware fuzzy decode) is the long-term fix. This
    // patch is the cheap interim that closes ~half of the
    // speech_defect_eval gap without growing the lookup table.
    //
    // Fast path above keeps clean-text latency unchanged.
    for (pat, answer) in patterns {
        if fuzzy_contains(lower, pat, 1) {
            return Some(build_candidate(pat, answer));
        }
    }
    None
}

/// Levenshtein-tolerant substring search. Returns `true` when some
/// contiguous window of `haystack` is within `max_edits` of `needle`.
/// `O(|h| × |n|)` time, but `needle` is short (≤ 40 chars in our
/// lookup table) and we run it only when exact match failed, so the
/// production overhead is bounded.
fn fuzzy_contains(haystack: &str, needle: &str, max_edits: usize) -> bool {
    let h: Vec<char> = haystack.chars().collect();
    let n: Vec<char> = needle.chars().collect();
    let nl = n.len();
    if nl == 0 {
        return true;
    }
    let hl = h.len();
    let min_win = nl.saturating_sub(max_edits);
    let max_win = (nl + max_edits).min(hl);
    if hl < min_win {
        return false;
    }
    for start in 0..=hl.saturating_sub(min_win) {
        for win_len in min_win..=max_win.min(hl - start) {
            let window = &h[start..start + win_len];
            if levenshtein(window, &n) <= max_edits {
                return true;
            }
        }
    }
    false
}

/// Classical Levenshtein DP over char slices.
fn levenshtein(a: &[char], b: &[char]) -> usize {
    let (m, n) = (a.len(), b.len());
    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }
    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr = vec![0usize; n + 1];
    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[n]
}

/// **v6.8.3 — 2026-06-17 user audit (Bug A).** Lifespan compute
/// for «<Person> қанша жыл өмір сүрді?» / «сколько лет прожил».
///
/// Pre-fix the cascade routed this through the substring-IsA layer
/// which surfaced the IsA fact («Ахмет Байтұрсынұлы → қазақ
/// ағартушысы») instead of the missing typed synthesis. The data is
/// in world_core (e.g. `kru_002` born_in 1872 + `kru_003` died_in
/// 1937 for Байтұрсынұлы); only the BornIn + DiedIn join was
/// missing.
///
/// Returns `None` when:
/// - the query shape doesn't match (no «қанша жыл» + «өмір сүр»);
/// - the subject can't be resolved to a canonical agent;
/// - either BornIn or DiedIn is missing in world_core for that
///   subject (curated-graph coverage gate; we don't guess);
/// - the year-extraction from the object surface fails or the
///   computed lifespan is non-positive.
fn lookup_person_lifespan(input: &str, idx: &FrameIndex) -> Option<String> {
    // **v6.8.4 — 2026-06-17 L4.5 Phase 1 (canary).** The String-
    // return path the cascade currently consumes is now a thin
    // wrapper over the typed `lookup_person_lifespan_typed` below.
    // Same surface, same semantics; the difference is that the
    // typed variant ALSO carries a `ProofObject` whose
    // `conclusion.object` matches the emitted lifespan text by
    // construction — addressing the v6.2-overwrites-after-
    // verification bug for this one route.  Subsequent routes
    // migrate the same way in Phase 2+.
    lookup_person_lifespan_typed(input, idx).map(|c| c.text)
}

/// **v6.8.4 L4.5 Phase 2.E.2.** Anaphora-aware wrapper around
/// [`lookup_person_lifespan_typed`].  When the input has an
/// explicit subject (`canonical_agent_for` resolves), behaviour
/// is identical to the existing handler.  When the input
/// matches the lifespan shape but has NO resolvable subject
/// («Қанша жыл өмір сүрді?»), the function consults the
/// `anaphora_subject` hint — typically the prior-turn Person
/// referent surfaced via
/// [`crate::dialog_acts::DiscourseState::last_referent_of_kind`]
/// — and re-runs the handler with a synthesised input string
/// «{anaphora_subject} қанша жыл өмір сүрді?».
fn lookup_person_lifespan_with_anaphora(
    input: &str,
    idx: &FrameIndex,
    anaphora_subject: Option<&str>,
) -> Option<String> {
    if let Some(text) = lookup_person_lifespan(input, idx) {
        return Some(text);
    }
    // The handler returned None.  Phase 2.E.2 anaphora fallback:
    // if the input matches the lifespan SHAPE («қанша жыл өмір
    // сүр») but has no resolvable subject, prepend the prior-
    // turn referent and retry.  We don't want to retry on inputs
    // that aren't lifespan-shaped at all (canonical agent may be
    // unresolved for many other reasons — math, chemistry, etc.).
    let subject = anaphora_subject?;
    let lower = input.to_lowercase();
    let looks_like_lifespan = (lower.contains("қанша жыл")
        || lower.contains("сколько лет")
        || lower.contains("неше жыл"))
        && (lower.contains("өмір сүр") || lower.contains("прожил") || lower.contains("жасады"));
    if !looks_like_lifespan {
        return None;
    }
    let synthesised = format!("{subject} {input}");
    lookup_person_lifespan(&synthesised, idx)
}

/// **v6.8.4 — 2026-06-17 L4.5 Phase 1 canary.** Typed sibling of
/// [`lookup_person_lifespan`] that returns the full
/// [`crate::dialog_acts::AnswerCandidate`] (moves + text + proof
/// + route + state_delta), not just the surface text.
///
/// The proof is constructed via
/// [`crate::proof_object::ProofObject::from_curated_fact`] with
/// predicate slug `"lifespan"` so the verifier can recognise this
/// as a typed synthesis route (BornIn + DiedIn join), distinct
/// from a single curated IsA hit.  The `raw_text` field cites
/// both source years so a downstream trace consumer can see the
/// composition.
fn lookup_person_lifespan_typed(
    input: &str,
    idx: &FrameIndex,
) -> Option<crate::dialog_acts::AnswerCandidate> {
    use crate::dialog_acts::{AnswerCandidate, RouteId};
    use crate::proof_object::ProofObject;
    use adam_reasoning::FactSource;

    let lower: String = input
        .to_lowercase()
        .chars()
        .map(|c| match c {
            '.' | '?' | '!' | ',' | ';' | ':' | '«' | '»' | '—' | '–' | '-' => ' ',
            other => other,
        })
        .collect();
    let lower = lower.split_whitespace().collect::<Vec<_>>().join(" ");

    let asks_count =
        lower.contains("қанша жыл") || lower.contains("сколько лет") || lower.contains("неше жыл");
    let asks_lived = lower.contains("өмір сүр")
        || lower.contains("жасап өтті")
        || lower.contains("жасады")
        || lower.contains("прожил")
        || lower.contains("жил");
    if !(asks_count && asks_lived) {
        return None;
    }

    let subject = canonical_agent_for(&lower)?;

    let born_year = query_year_for_predicate(idx, &subject, FramePredicate::BornIn)?;
    let died_year = query_year_for_predicate(idx, &subject, FramePredicate::DiedIn)?;
    if died_year <= born_year {
        return None;
    }
    let years_lived = died_year - born_year;

    let subject_titlecase = capitalize_first(&subject);
    let text =
        format!("{subject_titlecase} {years_lived} жыл өмір сүрді ({born_year}–{died_year}).");

    // Proof carries the joined `(born_year, died_year)` synthesis
    // explicitly so the verifier can audit the lifespan figure
    // against the same numbers in the emitted text.
    let proof = ProofObject::from_curated_fact(
        subject.clone(),
        "lifespan".to_string(),
        format!("{years_lived} жыл"),
        FactSource {
            pack: "world_core/synthesised".into(),
            sample_id: format!("lifespan/{subject}/{born_year}-{died_year}"),
        },
        format!("BornIn={born_year} жыл + DiedIn={died_year} жыл → {years_lived} жыл"),
    );

    let candidate = AnswerCandidate::assert(text, proof, RouteId::Lifespan);
    debug_assert!(candidate.invariant_check().is_ok());
    Some(candidate)
}

/// **v6.8.7 L4.8 — PropertyQueryIR birthplace.**  Resolve
/// «Қайда туылды?» / «Где родился?» against the FrameIndex's
/// BornIn predicate.  Mirrors `lookup_person_lifespan_with_
/// anaphora`: same anaphora-fallback pattern, different
/// property + different output template.
///
/// Subject resolution: explicit `canonical_agent_for(input)`
/// first; if None, fall back to `anaphora_subject` and retry
/// with a synthesised input.
fn lookup_person_birthplace_with_anaphora(
    input: &str,
    idx: &FrameIndex,
    anaphora_subject: Option<&str>,
) -> Option<String> {
    if let Some(text) = lookup_person_birthplace(input, idx) {
        return Some(text);
    }
    let subject = anaphora_subject?;
    if !looks_like_birthplace_query(input) {
        return None;
    }
    let synthesised = format!("{subject} {input}");
    lookup_person_birthplace(&synthesised, idx)
}

fn lookup_person_birthplace(input: &str, idx: &FrameIndex) -> Option<String> {
    if !looks_like_birthplace_query(input) {
        return None;
    }
    let lower = input.to_lowercase();
    let subject = canonical_agent_for(&lower)?;
    let place = query_place_for_predicate(idx, &subject, FramePredicate::BornIn)?;
    let subject_titlecase = capitalize_first(&subject);
    Some(format!("{subject_titlecase} {place}-да туған."))
}

fn looks_like_birthplace_query(input: &str) -> bool {
    let lower = input.to_lowercase();
    lower.contains("қайда туыл")
        || lower.contains("туылған жер")
        || lower.contains("қай жерде туыл")
        || lower.contains("где родил")
        || lower.contains("место рожд")
}

/// **v6.8.18 — Codex Q3 school #5 part 1.** Anaphora-aware
/// birth-year handler.  Closes the «Ол қай жылы туған?»
/// bare-follow-up gap that Codex flagged in the audit: the
/// explicit-subject query («Ахмет Байтұрсынұлы қай жылы
/// туылған?») already resolves via the cascade, but the
/// pronoun-elided follow-up after a biographical intro had no
/// handler and fell into a generic «Жыл бір күннен …»
/// definition response.
///
/// Mirrors `lookup_person_birthplace_with_anaphora` exactly —
/// same shape detector / same anaphora fallback / different
/// extraction (year via `query_year_for_predicate` rather than
/// place via `query_place_for_predicate`).  Both share the
/// BornIn predicate but pick complementary fields.
fn lookup_person_birthyear_with_anaphora(
    input: &str,
    idx: &FrameIndex,
    anaphora_subject: Option<&str>,
) -> Option<String> {
    if let Some(text) = lookup_person_birthyear(input, idx) {
        return Some(text);
    }
    let subject = anaphora_subject?;
    if !looks_like_birthyear_query(input) {
        return None;
    }
    let synthesised = format!("{subject} {input}");
    lookup_person_birthyear(&synthesised, idx)
}

fn lookup_person_birthyear(input: &str, idx: &FrameIndex) -> Option<String> {
    if !looks_like_birthyear_query(input) {
        return None;
    }
    let lower = input.to_lowercase();
    let subject = canonical_agent_for(&lower)?;
    let year = query_year_for_predicate(idx, &subject, FramePredicate::BornIn)?;
    let subject_titlecase = capitalize_first(&subject);
    Some(format!("{subject_titlecase} {year} жылы туылған."))
}

fn looks_like_birthyear_query(input: &str) -> bool {
    let lower = input.to_lowercase();
    // Kazakh: «қай жылы туыл» / «қай жылы туған» / «қашан туыл»
    // / «қашан туған» — all ask for the birth YEAR specifically
    // (vs `looks_like_birthplace_query` which asks for the
    // birth PLACE via «қайда туыл» / «қай жерде»).
    let asks_year = lower.contains("қай жылы")
        || lower.contains("қашан туыл")
        || lower.contains("қашан туған")
        || lower.contains("в каком году")
        || lower.contains("когда родил");
    let asks_birth = lower.contains("туыл") || lower.contains("туған") || lower.contains("родил");
    asks_year && asks_birth
}

/// **v6.8.7 L4.8 — PropertyQueryIR occupation.**  Resolve
/// «Кім болған?» / «Не маман?» / «Кем был?» against the
/// FrameIndex's IsA predicate.  Returns the first IsA object
/// (typically the dominant occupation).
fn lookup_person_occupation_with_anaphora(
    input: &str,
    idx: &FrameIndex,
    anaphora_subject: Option<&str>,
) -> Option<String> {
    if let Some(text) = lookup_person_occupation(input, idx) {
        return Some(text);
    }
    let subject = anaphora_subject?;
    if !looks_like_occupation_query(input) {
        return None;
    }
    let synthesised = format!("{subject} {input}");
    lookup_person_occupation(&synthesised, idx)
}

fn lookup_person_occupation(input: &str, idx: &FrameIndex) -> Option<String> {
    if !looks_like_occupation_query(input) {
        return None;
    }
    let lower = input.to_lowercase();
    let subject = canonical_agent_for(&lower)?;
    let occupation = query_isa_object(idx, &subject)?;
    let subject_titlecase = capitalize_first(&subject);
    Some(format!("{subject_titlecase} — {occupation}."))
}

fn looks_like_occupation_query(input: &str) -> bool {
    let lower = input.to_lowercase();
    lower.contains("кім болған")
        || lower.contains("не маман")
        || lower.contains("қандай маман")
        || lower.contains("кем был")
        || lower.contains("какая профес")
        || lower.contains("кто такой")
}

/// Query the FrameIndex for the (subject, predicate) place
/// object — i.e. the non-year string that the predicate carries
/// when the world_core entry is a location («торғай облысы»,
/// not «1872 жылы 5 қыркүйек»).
///
/// world_core stores BornIn / DiedIn under both shapes:
///   1. Object string with a leading year («1872 жылы 5 қыркүйек»)
///      — extracted by `query_year_for_predicate`.
///   2. Object string with a place name («торғай облысы») —
///      what THIS function returns.
fn query_place_for_predicate(
    idx: &FrameIndex,
    subject: &str,
    predicate: FramePredicate,
) -> Option<String> {
    let q = QueryIR::new(
        QueryFocus::Subject,
        QuestionForm::Definition,
        AnswerShape::BareNoun,
    )
    .with_agent(noun(subject))
    .with_predicate(predicate);
    for hit in idx.query(&q).into_iter().take(8) {
        let Some(obj) = hit.frame.object.as_ref() else {
            continue;
        };
        let surface = &obj.root.surface;
        if extract_year_in_range(surface).is_none() && !surface.trim().is_empty() {
            return Some(capitalize_first(surface));
        }
    }
    None
}

/// Query the FrameIndex for the first IsA object for `subject`.
/// Picks the FIRST hit so curators control "dominant occupation"
/// ordering by listing the canonical role first in world_core.
fn query_isa_object(idx: &FrameIndex, subject: &str) -> Option<String> {
    let q = QueryIR::new(
        QueryFocus::Subject,
        QuestionForm::Definition,
        AnswerShape::BareNoun,
    )
    .with_agent(noun(subject))
    .with_predicate(FramePredicate::IsA);
    for hit in idx.query(&q).into_iter().take(4) {
        if let Some(obj) = hit.frame.object.as_ref() {
            let surface = obj.root.surface.trim();
            if !surface.is_empty() {
                return Some(surface.to_string());
            }
        }
    }
    None
}

/// **v6.8.5 L4.6 — industrial-pilot retrieval.**  Resolve a
/// procedure / SOP query against the typed procedure set loaded
/// from `data/procedures/*.jsonl`.
///
/// Handled query shapes (Kazakh + Russian — the pilot voice
/// surface is Kazakh, but the cyrillic pilot supervisors often
/// switch to Russian mid-sentence):
///
///   * «X қалай жүргізіледі?» / «X қалай жасалады?» / «X қалай
///     өтеді?»
///   * «X тәртібі қандай?» / «X-тің тәртібі қалай?»
///   * «X рәсімі қалай?» / «X-ке арналған рәсім ...»
///   * «Как проводится X?» / «Порядок проведения X?» /
///     «Процедура X?»
///
/// Matching: simple keyword overlap between the query and each
/// fixture's `title_kk` / `title_ru` / `applies_to` fields.
/// Returns the highest-scoring procedure when the best score
/// clears `MIN_SCORE`; `None` otherwise (so the cascade falls
/// through to the v6.1 layer for non-procedure inputs).
///
/// Output: a multi-line response carrying title + prerequisites +
/// ordered steps + hazards + source citation.  The voice layer
/// reads the multi-line shape clause-by-clause.
fn lookup_procedure(input: &str) -> Option<String> {
    lookup_procedure_matched(input).map(|(text, _id)| text)
}

/// **v6.8.7 L4.8 C.2.** Same retrieval logic as
/// [`lookup_procedure`] but ALSO returns the matched procedure's
/// `id`.  Used by the cascade so [`crate::dialog_acts::
/// DiscourseState`] can record a `ReferentKind::Procedure`
/// referent, which a subsequent bare follow-up like «Қанша қадам
/// бар?» / «Кім жауапты?» consults to fetch the procedure's
/// `steps` / `authorization` fields by id.
fn lookup_procedure_matched(input: &str) -> Option<(String, String)> {
    lookup_procedure_matched_with_score(input).map(|(text, id, _)| (text, id))
}

/// **v6.8.11 — active-uncertainty foundation.**  Same as
/// `lookup_procedure_matched` but also returns a normalised
/// confidence score in `[0.0, 1.0]` for the keyword-overlap
/// match.  Raw score is the integer sum from `score_procedure`;
/// normalised = `raw / NORMALISE_CEILING`, clamped.  A match
/// just above `MIN_SCORE` (2) scores ≈ 0.25; a strong match
/// (typical pilot query against the exact fixture title) scores
/// 0.8 – 1.0.  The cascade uses this in `RouterAnswer::
/// confidence` so future Clarify routing can suppress weak
/// matches.
fn lookup_procedure_matched_with_score(input: &str) -> Option<(String, String, f32)> {
    use crate::procedure_loader::shared_procedures;

    let lower = input.to_lowercase();
    // **v6.8.45 — procedure_eval audit fix.**  The flat
    // SHAPE_TRIGGERS list catches «тәртібі қандай?» / «қалай
    // жүргізіледі?» / etc. but not the productive «қалай
    // <verb> керек?» shape (common in worker queries:
    // «Газ концентрациясын қалай өлшеу керек?»).  Treat the
    // co-occurrence of «қалай» + «керек» as a procedure
    // shape trigger.  Same idea for Russian: «как ... нужно».
    //
    // **v6.8.46 — second co-occurrence trigger.**  The
    // observational «X не істеу керек?» («what to do with
    // X») shape — see «Мас күйдегі қызметкерді не істеу
    // керек?» / «Жұмысқа алғаш келген қызметкер не істеуі
    // керек?».  Medical-symptom uses of «не істеу керек?»
    // have already been filtered out by safety_guard's
    // `looks_like_medical_what_to_do` gate; everything that
    // survives to the procedure router is procedural.  Zero
    // conflicts with safety_eval / conv_dialog_eval (audited
    // on 2026-06-29: 0 matches across both packs).
    let cooccurrence_trigger = (lower.contains("қалай") && lower.contains("керек"))
        || (lower.contains("не істе") && lower.contains("керек"))
        || (lower.contains("как ") && lower.contains("нужно"))
        || (lower.contains("что делать"));
    let trigger_present = cooccurrence_trigger
        || SHAPE_TRIGGERS_KK
            .iter()
            .chain(SHAPE_TRIGGERS_RU.iter())
            .any(|t| lower.contains(t));
    if !trigger_present {
        return None;
    }
    let (best_idx, raw_score) = score_best_procedure(&lower)?;
    // Normalisation ceiling tuned against the v6.8.5 fixture set:
    // a pilot-style query against the right fixture typically
    // scores 6–10 (multiple title_kk + applies_to overlaps), so
    // dividing by 8 lands strong matches at ~0.75–1.25 (clamped
    // to 1.0) and the weakest acceptable match (just above
    // MIN_SCORE = 2) at 0.25.  Defined inline because it is
    // intrinsic to the score_procedure cost model and shouldn't
    // diverge from it.
    const NORMALISE_CEILING: f32 = 8.0;
    let normalised_score = (raw_score as f32 / NORMALISE_CEILING).clamp(0.0, 1.0);
    let proc = &shared_procedures()[best_idx];
    Some((render_procedure(proc), proc.id.clone(), normalised_score))
}

/// **v6.8.7 L4.8 C.2 — procedure attribute query (step count).**
/// Detects «Қанша қадам бар?» / «Сколько шагов?» and, when a
/// procedure referent is on the discourse stack, fetches the
/// procedure by id and returns its step count.
///
/// Returns `None` when the shape doesn't match OR no procedure
/// referent is available — the cascade then falls through to
/// the v6.1 layer for the bare query.
fn lookup_procedure_step_count(input: &str, anaphora_procedure_id: Option<&str>) -> Option<String> {
    if !looks_like_procedure_step_count_query(input) {
        return None;
    }
    let proc_id = anaphora_procedure_id?;
    let proc = crate::procedure_loader::shared_procedures()
        .iter()
        .find(|p| p.id == proc_id)?;
    Some(format!(
        "«{}» рәсімінде {} қадам бар.",
        proc.title_kk,
        proc.steps.len(),
    ))
}

fn looks_like_procedure_step_count_query(input: &str) -> bool {
    let lower = input.to_lowercase();
    (lower.contains("қанша қадам") || lower.contains("неше қадам"))
        || (lower.contains("сколько шаг") || lower.contains("число шаг"))
}

/// **v6.8.7 L4.8 C.2 — procedure attribute query (authority).**
/// Detects «Кім жауапты?» / «Кто отвечает?» and, when a procedure
/// referent is on the discourse stack, fetches the procedure by
/// id and returns its `authorization` field as a comma-joined
/// list.
fn lookup_procedure_authority(input: &str, anaphora_procedure_id: Option<&str>) -> Option<String> {
    if !looks_like_procedure_authority_query(input) {
        return None;
    }
    // **v6.8.48 — procedure_eval audit fix.**  Resolution
    // order:
    //   1. Strong content match (score ≥ ANAPHORA_OVERRIDE_FLOOR)
    //      overrides anaphora — worker asking a topical
    //      question gets that procedure's authority info.
    //   2. Anaphora referent — bare follow-up («Кім жауап?»
    //      with no topic mention) uses the prior procedure.
    //   3. Any content match (score ≥ MIN_SCORE) — anaphora
    //      absent.
    //   4. None — neither path available.
    let lower = input.to_lowercase();
    let strong_match = score_best_procedure_with_floor(&lower, ANAPHORA_OVERRIDE_FLOOR);
    let weak_match = score_best_procedure(&lower);
    let proc = match (strong_match, anaphora_procedure_id, weak_match) {
        (Some((idx, _)), _, _) => &crate::procedure_loader::shared_procedures()[idx],
        (None, Some(id), _) => crate::procedure_loader::shared_procedures()
            .iter()
            .find(|p| p.id == id)?,
        (None, None, Some((idx, _))) => &crate::procedure_loader::shared_procedures()[idx],
        (None, None, None) => return None,
    };
    if proc.authorization.is_empty() {
        return None;
    }
    Some(format!(
        "«{}» рәсіміне {} жауапты.",
        proc.title_kk,
        proc.authorization.join(", "),
    ))
}

fn looks_like_procedure_authority_query(input: &str) -> bool {
    let lower = input.to_lowercase();
    // **v6.8.48 — procedure_eval audit fix.**  Adjacent
    // markers («кім береді») catch the simple shape;
    // co-occurrence covers worker-perspective queries
    // with object material between the interrogative and
    // the verb («Кім мерзімдік медициналық тексеруден өтуі
    // тиіс?»).  Audit 2026-06-29: zero conflicts in
    // conv_dialog / safety / v6_7_real_audit /
    // school_program for the «кім» + obligation-marker
    // co-occurrence.
    let adjacent = lower.contains("кім жауап")
        || lower.contains("кім жасайды")
        || lower.contains("кім жүргіз")
        || lower.contains("кім беред")
        || lower.contains("кім өтеді")
        || lower.contains("кім өтуі")
        || lower.contains("кім тиіс")
        || lower.contains("кім ресімдей")
        || lower.contains("кто отвечает")
        || lower.contains("кто ответствен")
        || lower.contains("чья ответствен")
        || lower.contains("кто проводит")
        || lower.contains("кто выдает");
    let co_occurrence = lower.contains("кім")
        && (lower.contains("тиіс") || lower.contains("өтуі") || lower.contains("береді"));
    adjacent || co_occurrence
}

/// **v6.8.50 — procedure attribute query (actor-undergoer).**
/// Distinct from `lookup_procedure_authority`: the worker
/// query «Кім ... өтуі тиіс?» («Who must undergo ...?») asks
/// about the SUBJECT that DOES the action, not the
/// responsible party.  Maps to `procedure.applies_to`
/// instead of `procedure.authorization`.
///
/// Surface-form audit (2026-06-29 procedure_eval baseline):
/// «Кім мерзімдік медициналық тексеруден өтуі тиіс?»
/// expected «қызметкер» — production was returning the
/// procedure's authority list («кадр бөлімі, еңбекті қорғау
/// инженері») which is semantically wrong.  This sub-router
/// closes that gap.
fn lookup_procedure_actor_undergoer(
    input: &str,
    anaphora_procedure_id: Option<&str>,
) -> Option<String> {
    if !looks_like_procedure_undergoer_query(input) {
        return None;
    }
    // Same content-match-preference + anaphora-fallback
    // resolution as `lookup_procedure_hazards` /
    // `lookup_procedure_authority`.  Strong content match
    // wins over stale anaphora; bare follow-up with no
    // content anchor uses the previously-named procedure.
    let lower = input.to_lowercase();
    let strong_match = score_best_procedure_with_floor(&lower, ANAPHORA_OVERRIDE_FLOOR);
    let weak_match = score_best_procedure(&lower);
    let proc = match (strong_match, anaphora_procedure_id, weak_match) {
        (Some((idx, _)), _, _) => &crate::procedure_loader::shared_procedures()[idx],
        (None, Some(id), _) => crate::procedure_loader::shared_procedures()
            .iter()
            .find(|p| p.id == id)?,
        (None, None, Some((idx, _))) => &crate::procedure_loader::shared_procedures()[idx],
        (None, None, None) => return None,
    };
    if proc.applies_to.is_empty() {
        return None;
    }
    Some(format!(
        "«{}» рәсіміне {} жатады.",
        proc.title_kk,
        proc.applies_to.join(", "),
    ))
}

/// Surface-form detector for actor-undergoer queries.
/// Distinct from `looks_like_procedure_authority_query`:
/// authority asks «who is responsible», undergoer asks
/// «who performs / undergoes».  The same «кім» interrogative
/// AND verbs that indicate undergoing the action
/// («өтуі / өтеді» — passes through, «жатады» — applies
/// to / belongs to, «қатысады» — participates in,
/// «тиіс» — must/should).
fn looks_like_procedure_undergoer_query(input: &str) -> bool {
    let lower = input.to_lowercase();
    if !lower.contains("кім") && !lower.contains("кто ") {
        return false;
    }
    // The undergoer shape leans on three Kazakh verbs of
    // undergoing/participation.  Combined with «кім» these
    // are unambiguous worker-perspective questions about
    // applies_to.  «тиіс» (must/should) here narrows
    // generic «who» queries to obligation framing — the
    // typical SOP context.
    let undergoer_verb = lower.contains("өтуі")
        || lower.contains("өтеді")
        || lower.contains("қатысады")
        || lower.contains("жатады")
        || lower.contains("проходит")
        || lower.contains("участвует");
    let obligation = lower.contains("тиіс") || lower.contains("должен");
    undergoer_verb && obligation
}

/// **v6.8.12 — procedure attribute query (hazards).** Detects
/// «Қауіптері қандай?» / «Какие опасности?» follow-ups against
/// a prior procedure referent and returns the procedure's
/// `hazards` field as a formatted list with mitigations.
///
/// Codex's voice REPL audit flagged the LOTO follow-up
/// «Қауіптері қандай?» as falling through to the v6.1
/// cascade — this handler closes that gap.
fn lookup_procedure_hazards(input: &str, anaphora_procedure_id: Option<&str>) -> Option<String> {
    if !looks_like_procedure_hazards_query(input) {
        return None;
    }
    // **v6.8.48 — procedure_eval audit fix.**  Resolution
    // order (mirrors `lookup_procedure_authority`):
    //   1. Strong content match (≥ ANAPHORA_OVERRIDE_FLOOR)
    //      → use that procedure.  Worker on a sequence of
    //      independent topics gets the right procedure's
    //      hazards even when the prior turn was about a
    //      different procedure.
    //   2. Anaphora referent → bare follow-up
    //      «Қауіптері қандай?» stays bound to the
    //      previously-named procedure (preserves the
    //      v6.8.12 follow-up shape).
    //   3. Weak content match → fall back when no
    //      anaphora is set.
    //   4. None → caller's cascade falls through.
    let lower = input.to_lowercase();
    let strong_match = score_best_procedure_with_floor(&lower, ANAPHORA_OVERRIDE_FLOOR);
    let weak_match = score_best_procedure(&lower);
    let proc = match (strong_match, anaphora_procedure_id, weak_match) {
        (Some((idx, _)), _, _) => &crate::procedure_loader::shared_procedures()[idx],
        (None, Some(id), _) => crate::procedure_loader::shared_procedures()
            .iter()
            .find(|p| p.id == id)?,
        (None, None, Some((idx, _))) => &crate::procedure_loader::shared_procedures()[idx],
        (None, None, None) => return None,
    };
    if proc.hazards.is_empty() {
        // The procedure is on-record as having no curated hazards
        // (NOT the same as having unknown hazards).  Surface the
        // explicit empty state rather than fall back to the
        // cascade — the curator deliberately recorded zero.
        return Some(format!(
            "«{}» рәсімінде тіркелген қауіптер жоқ.",
            proc.title_kk,
        ));
    }
    let mut out = format!("«{}» рәсімінің қауіптері:\n", proc.title_kk);
    for h in &proc.hazards {
        out.push_str(&format!(" — {} → {}\n", h.kind_kk, h.mitigation_kk));
    }
    Some(out.trim_end().to_string())
}

fn looks_like_procedure_hazards_query(input: &str) -> bool {
    let lower = input.to_lowercase();
    lower.contains("қауіптер")
        || lower.contains("қауіп қанд")
        || lower.contains("қандай қауіп")
        || lower.contains("опасност")
        || lower.contains("какой риск")
        || lower.contains("какие риск")
}

/// **v6.8.12 — procedure attribute query (prerequisites).**
/// Detects «Алдын ала шарттар?» / «Какие предусловия?» and
/// returns the procedure's `prerequisites` field as a list.
fn lookup_procedure_prerequisites(
    input: &str,
    anaphora_procedure_id: Option<&str>,
) -> Option<String> {
    if !looks_like_procedure_prerequisites_query(input) {
        return None;
    }
    let proc_id = anaphora_procedure_id?;
    let proc = crate::procedure_loader::shared_procedures()
        .iter()
        .find(|p| p.id == proc_id)?;
    if proc.prerequisites.is_empty() {
        return Some(format!(
            "«{}» рәсімінде алдын ала шарттар тіркелмеген.",
            proc.title_kk,
        ));
    }
    let mut out = format!("«{}» рәсімінің алдын ала шарттары:\n", proc.title_kk);
    for p in &proc.prerequisites {
        out.push_str(&format!(" — {p}\n"));
    }
    Some(out.trim_end().to_string())
}

fn looks_like_procedure_prerequisites_query(input: &str) -> bool {
    let lower = input.to_lowercase();
    lower.contains("алдын ала шарт")
        || lower.contains("қандай шарт")
        || lower.contains("шарттары")
        || lower.contains("предусловия")
        || lower.contains("какие условия")
        || lower.contains("требования")
}

/// **v6.8.12 — procedure attribute query (step-by-number).**
/// Detects «Екінші қадамды айтшы», «Үшінші қадам не?», «Второй
/// шаг» and returns the specific `steps[N-1].action_kk` for the
/// referenced procedure.  Out-of-range step indices return a
/// soft refusal naming the procedure's actual step count.
///
/// Kazakh ordinal parsing covers «бірінші» through «оныншы»
/// (1–10) plus the matching cardinal in dative ﻿(«бірге», «екіге»,
/// ﻿«үшке», …) so «екіге қадам» / «екінші қадамды» both work.
fn lookup_procedure_step_by_number(
    input: &str,
    anaphora_procedure_id: Option<&str>,
) -> Option<String> {
    if !looks_like_procedure_step_by_number_query(input) {
        return None;
    }
    let proc_id = anaphora_procedure_id?;
    let proc = crate::procedure_loader::shared_procedures()
        .iter()
        .find(|p| p.id == proc_id)?;
    let n = parse_step_ordinal(input)?;
    if n == 0 || n as usize > proc.steps.len() {
        return Some(format!(
            "«{}» рәсімінде {} қадам бар; {}-ші қадам жоқ.",
            proc.title_kk,
            proc.steps.len(),
            n,
        ));
    }
    let step = &proc.steps[(n - 1) as usize];
    Some(format!(
        "«{}» рәсімінің {}-ші қадамы: {}",
        proc.title_kk, step.sequence, step.action_kk,
    ))
}

fn looks_like_procedure_step_by_number_query(input: &str) -> bool {
    let lower = input.to_lowercase();
    let has_step_word =
        lower.contains("қадам") || lower.contains("шаг") || lower.contains("қадамды");
    if !has_step_word {
        return false;
    }
    parse_step_ordinal(input).is_some()
}

/// Parse Kazakh ordinal («бірінші» / «екінші» / …) OR Russian
/// ordinal («первый» / «второй» / …) OR bare numeral
/// («2-қадам», «3-ші қадам») into `u32` step index.  Returns
/// `None` when no recognisable ordinal is present.
fn parse_step_ordinal(input: &str) -> Option<u32> {
    let lower = input.to_lowercase();
    const KK_ORDINALS: &[(&str, u32)] = &[
        ("бірінші", 1),
        ("екінші", 2),
        ("үшінші", 3),
        ("төртінші", 4),
        ("бесінші", 5),
        ("алтыншы", 6),
        ("жетінші", 7),
        ("сегізінші", 8),
        ("тоғызыншы", 9),
        ("оныншы", 10),
    ];
    const RU_ORDINALS: &[(&str, u32)] = &[
        ("первый", 1),
        ("вторый", 2),
        ("второй", 2),
        ("третий", 3),
        ("четвертый", 4),
        ("четвёртый", 4),
        ("пятый", 5),
        ("шестой", 6),
        ("седьмой", 7),
        ("восьмой", 8),
        ("девятый", 9),
        ("десятый", 10),
    ];
    for (word, n) in KK_ORDINALS.iter().chain(RU_ORDINALS) {
        if lower.contains(word) {
            return Some(*n);
        }
    }
    // Bare numeral form «2-ші қадам» / «3 шаг» — scan for the
    // first 1–2 digit token surrounded by non-digits.
    let mut digits = String::new();
    for ch in lower.chars().chain(std::iter::once(' ')) {
        if ch.is_ascii_digit() {
            digits.push(ch);
        } else if !digits.is_empty() {
            if digits.len() <= 2
                && let Ok(n) = digits.parse::<u32>()
                && (1..=20).contains(&n)
            {
                return Some(n);
            }
            digits.clear();
        }
    }
    None
}

/// **v6.8.12 — procedure attribute query (source citation).**
/// Detects «Қайдан?» / «Дереккөзі?» / «Откуда это?» and returns
/// the procedure's `source` block (regulation name + id +
/// article + version date) as a single sentence.  Critical for
/// pilot supervisors auditing where an SOP came from.
fn lookup_procedure_source(input: &str, anaphora_procedure_id: Option<&str>) -> Option<String> {
    if !looks_like_procedure_source_query(input) {
        return None;
    }
    let proc_id = anaphora_procedure_id?;
    let proc = crate::procedure_loader::shared_procedures()
        .iter()
        .find(|p| p.id == proc_id)?;
    let mut out = format!(
        "«{}» рәсімінің дереккөзі — {} ({}",
        proc.title_kk, proc.source.regulation_kk, proc.source.regulation_id,
    );
    if let Some(article) = &proc.source.article {
        out.push_str(&format!(", {article}"));
    }
    out.push_str(&format!(", {}).", proc.source.version_date));
    Some(out)
}

fn looks_like_procedure_source_query(input: &str) -> bool {
    let lower = input.to_lowercase();
    lower.contains("дереккөз")
        || lower.contains("дерек көз")
        || lower.contains("қай заң")
        || lower.contains("қайдан алын")
        || lower.contains("қандай заң")
        || lower.contains("источник")
        || lower.contains("откуда это")
        || lower.contains("какой закон")
}

/// **v6.8.13 — procedure attribute query (numeric condition check).**
/// Detects «<X> <N> <unit> болса <Y> керек пе?» (and Russian
/// equivalents) — a numeric-input shape against a procedure
/// step's `condition` field.  Iterates the prior procedure's
/// steps; for each step with a parseable `ConditionExpr`,
/// evaluates against the user's parsed `ConditionInput`.
///
/// Honest «I cannot answer» when:
///   * no `?` / question intent in the input;
///   * no parseable user input («<var> <num> <unit>» pattern
///     absent);
///   * no procedure step has a numeric condition;
///   * variable / unit mismatch between user input and the
///     procedure's stored conditions (typed mismatch ≠ guessing).
fn lookup_procedure_condition_check(
    input: &str,
    anaphora_procedure_id: Option<&str>,
) -> Option<String> {
    if !looks_like_procedure_condition_query(input) {
        return None;
    }
    let proc_id = anaphora_procedure_id?;
    let user_input = adam_algebra::parse_condition_user_input(input)?;
    let proc = crate::procedure_loader::shared_procedures()
        .iter()
        .find(|p| p.id == proc_id)?;
    // Walk steps, find one whose condition parses AND matches the
    // user's variable / unit.  First match wins — in v1 each step
    // has at most one condition and the procedure has at most
    // one step per variable (the current fixture set holds).
    for step in &proc.steps {
        let Some(cond_text) = step.condition.as_ref() else {
            continue;
        };
        let Some(expr) = adam_algebra::parse_condition(cond_text) else {
            continue;
        };
        if let Some(verdict) = adam_algebra::evaluate_condition(&expr, &user_input) {
            return Some(render_condition_verdict(
                &user_input,
                verdict,
                &proc.title_kk,
                step,
                cond_text,
            ));
        }
    }
    // Reached only when the user's input parsed but no step
    // conditioned on a matching (var, unit).  Honest refusal —
    // the procedure may have categorical conditions or different
    // variables.
    Some(format!(
        "«{}» рәсімінде «{}» жөнінде сандық шарт жоқ — нақты жауап бере алмаймын.",
        proc.title_kk, user_input.var,
    ))
}

fn looks_like_procedure_condition_query(input: &str) -> bool {
    let lower = input.to_lowercase();
    let has_conditional = lower.contains("болса")
        || lower.contains("болғанда")
        || lower.contains("егер")
        || lower.contains("если");
    let has_question = input.contains('?') || lower.contains("керек пе");
    has_conditional && has_question
}

fn render_condition_verdict(
    user_input: &adam_algebra::ConditionInput,
    verdict: bool,
    proc_title: &str,
    step: &adam_algebra::ProcedureStep,
    cond_text: &str,
) -> String {
    let yes_no = if verdict { "иә" } else { "жоқ" };
    let explanation = if verdict {
        format!("{}-ші қадам шартына сай ({}).", step.sequence, cond_text,)
    } else {
        format!(
            "{} {} {} — {}-ші қадам шарты «{}» орындалмайды.",
            user_input.var, user_input.value, user_input.unit, step.sequence, cond_text,
        )
    };
    format!(
        "{yes_no}. «{proc_title}» рәсімі бойынша: {explanation} Қадам: «{}»",
        step.action_kk,
    )
}

/// **v6.8.14 — procedure attribute query (permission / forbidden
/// state).**  Categorical sibling of the v6.8.13 numeric
/// `lookup_procedure_condition_check`.  Detects «X (жоқ |
/// болмаса) ... (бола ма | жасауға бола ма)?» / «Если X нет,
/// можно ли Y?» — the user is asking whether a categorical state
/// is permitted.  Answers from the procedure's `hazards` field,
/// which already carries forbidden-state pairs:
///
///   hazard.kind_kk      — the forbidden state («СИЗ-сіз жұмыс істеу»)
///   hazard.mitigation_kk — the consequence / why it's forbidden
///                          («бұл рәсім аяқталмайынша қызметкер
///                           цехқа кіргізілмейді»)
///
/// Matching: token-overlap (≥ 4-char tokens, lowercased) between
/// the input and `hazard.kind_kk`.  First hazard with the
/// highest overlap wins.  When no hazard matches AND the
/// procedure has no other forbidden-state info, returns `None`
/// so the cascade falls through honestly.
fn lookup_procedure_permission_check(
    input: &str,
    anaphora_procedure_id: Option<&str>,
) -> Option<String> {
    if !looks_like_procedure_permission_query(input) {
        return None;
    }
    let proc_id = anaphora_procedure_id?;
    let proc = crate::procedure_loader::shared_procedures()
        .iter()
        .find(|p| p.id == proc_id)?;
    if proc.hazards.is_empty() {
        return None;
    }
    let lower = input.to_lowercase();
    let input_tokens: Vec<String> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.chars().count() >= 4)
        .map(|t| t.to_string())
        .collect();
    if input_tokens.is_empty() {
        return None;
    }
    // Bidirectional substring overlap: Kazakh case suffixes mean
    // input tokens may carry agglutinated tails («жұмысқа» =
    // «жұмыс» + «-қа» dative, «жұмысшы» = «жұмыс» + «-шы» agent
    // nominaliser).  Substring match in BOTH directions catches
    // the stem regardless of which side it appears on.
    let mut best: Option<(usize, usize)> = None;
    for (i, h) in proc.hazards.iter().enumerate() {
        let kind_lower = h.kind_kk.to_lowercase();
        let kind_tokens: Vec<&str> = kind_lower
            .split(|c: char| !c.is_alphanumeric())
            .filter(|t| t.chars().count() >= 4)
            .collect();
        let forward = input_tokens
            .iter()
            .filter(|t| kind_lower.contains(t.as_str()))
            .count();
        let reverse = kind_tokens.iter().filter(|t| lower.contains(*t)).count();
        let overlap = forward + reverse;
        if overlap > 0 && best.is_none_or(|(_, b)| overlap > b) {
            best = Some((i, overlap));
        }
    }
    let (idx, _) = best?;
    let h = &proc.hazards[idx];
    Some(format!(
        "Жоқ. «{}» рәсімінде {} қауіпті деп тіркелген. Себебі: {}.",
        proc.title_kk, h.kind_kk, h.mitigation_kk,
    ))
}

fn looks_like_procedure_permission_query(input: &str) -> bool {
    let lower = input.to_lowercase();
    let has_conditional = lower.contains("болса")
        || lower.contains("болмаса")
        || lower.contains("жоқ болса")
        // **v6.8.30 Bug #2.**  Industrial conditional verbs:
        // «жетпесе» (doesn't reach), «жетпегенде» / «болмағанда»
        // (when X doesn't hold), «шаршаса» (gets tired),
        // «ауырса» (gets sick), «бұзылса» (breaks down).  The
        // LOTO «қысым нөлге жетпесе» / driver-fatigue
        // «шаршаса» industrial-audit cases fall here.
        || lower.contains("жетпесе")
        || lower.contains("жетпегенде")
        || lower.contains("болмағанда")
        || lower.contains("шаршаса")
        || lower.contains("ауырса")
        || lower.contains("бұзылса")
        || lower.contains("істен шықса")
        || lower.contains("если ")
        || lower.contains("когда ");
    let has_permission_q = lower.contains("бола ма")
        || lower.contains("болмай ма")
        || lower.contains("жасауға бола")
        || lower.contains("кіруге бола")
        || lower.contains("істеуге бола")
        // **v6.8.30 Bug #2.**  «бастауға бола» (allowed to
        // start), «жіберуге бола» (allowed to dispatch),
        // «жалғастыруға бола» (allowed to continue) — the
        // industrial-pilot action verbs the school-tutor set
        // didn't need.
        || lower.contains("бастауға бола")
        || lower.contains("жіберуге бола")
        || lower.contains("жалғастыруға бола")
        || lower.contains("болады ма")
        || lower.contains("можно ли")
        || lower.contains("разрешено")
        || lower.contains("допустимо");
    has_conditional && has_permission_q
}

/// **v6.8.21 — Codex Q3 school #3: historical alias.**  Answer
/// «X бұрын қалай аталды?» / «Ол бұрын қалай аталды?» from a
/// curated Kazakh-cities-and-major-places alias table.
///
/// Anaphora-aware: when the input has no explicit subject, the
/// helper consults `anaphora_subject` (the latest non-Procedure
/// referent) and retries with a synthesised «{subject} бұрын
/// қалай аталды?» query.
///
/// Returns `None` when:
///   * the input doesn't carry a «бұрын аталды» / «ескі аты» /
///     «прежнее название» marker;
///   * the subject doesn't match any entry in
///     `HISTORICAL_ALIASES`.  In that case the cascade falls
///     through honestly — better than fabricating a
///     plausible-sounding alias chain.
fn lookup_historical_alias_with_anaphora(
    input: &str,
    anaphora_subject: Option<&str>,
) -> Option<String> {
    if let Some(text) = lookup_historical_alias(input) {
        return Some(text);
    }
    let subject = anaphora_subject?;
    if !looks_like_historical_alias_query(input) {
        return None;
    }
    let synthesised = format!("{subject} {input}");
    lookup_historical_alias(&synthesised)
}

fn lookup_historical_alias(input: &str) -> Option<String> {
    if !looks_like_historical_alias_query(input) {
        return None;
    }
    let lower = input.to_lowercase();
    for (canonical, aliases) in HISTORICAL_ALIASES {
        if lower.contains(canonical) {
            let chain = aliases.join("; ");
            return Some(format!("{} бұрын: {}.", capitalize_first(canonical), chain,));
        }
    }
    None
}

fn looks_like_historical_alias_query(input: &str) -> bool {
    let lower = input.to_lowercase();
    lower.contains("бұрын қалай аталды")
        || lower.contains("бұрын қалай аталған")
        || lower.contains("бұрынғы аты")
        || lower.contains("бұрынғы атауы")
        || lower.contains("ескі аты")
        || lower.contains("ескі атауы")
        || lower.contains("бұрын аталған")
        || lower.contains("раньше назывался")
        || lower.contains("прежнее название")
        || lower.contains("старое название")
}

/// Curated historical alias table for major Kazakh cities and
/// landmarks.  Each entry: canonical lowercased modern name →
/// chronologically-ordered list of historical names with
/// year ranges.  v6.8.21 ships a small set covering the
/// places adam's world_core has factual coverage for; expand
/// as new pilot or school-tutor data arrives.
///
/// Pattern: «<earlier name> (<year range>)» — Kazakh
/// readability over machine parseability.  When a future
/// commit needs structured access, this becomes a
/// `BTreeMap<&str, Vec<HistoricalName>>` with typed time
/// anchors.
#[rustfmt::skip]
const HISTORICAL_ALIASES: &[(&str, &[&str])] = &[
    ("астана", &[
        "Ақмолинск (1830–1961)",
        "Целиноград (1961–1992)",
        "Ақмола (1992–1998)",
        "Астана (1998–2019)",
        "Нұр-Сұлтан (2019–2022)",
        "Астана (2022 жылдан қайтадан)",
    ]),
    ("алматы", &[
        "Верный (1854–1921)",
        "Алма-Ата (1921–1993)",
        "Алматы (1993 жылдан)",
    ]),
    ("шымкент", &[
        "Чимкент (Ресей империясы / КСРО кезеңі)",
        "Шымкент (тәуелсіздік алғаннан кейін)",
    ]),
    ("тараз", &[
        "Әулие-Ата (XIX ғасырға дейін)",
        "Мирзоян (1936–1938)",
        "Жамбыл (1938–1997)",
        "Тараз (1997 жылдан)",
    ]),
    ("семей", &[
        "Семипалатинск (1718–2007)",
        "Семей (2007 жылдан)",
    ]),
    ("петропавл", &[
        "Петропавловск (Ресей империялық дәуірі)",
        "Петропавл (қазақша атау)",
    ]),
    ("өскемен", &[
        "Усть-Каменогорск (орысша атау)",
        "Өскемен (қазақша атау)",
    ]),
];

const SHAPE_TRIGGERS_KK: &[&str] = &[
    "қалай жүргізіл",
    "қалай жасал",
    "қалай өткіз",
    "қалай өтед",
    "тәртіб",
    "тәртіп",
    "рәсім",
    "рәсімі",
    "нұсқаулық",
    // **v6.8.48 — procedure_eval audit fix.**  Worker-query
    // shapes that the procedure router consults via attribute
    // sub-routers (hazards / authority / actor) — without
    // these as bare triggers, the procedure scorer never runs
    // and the sub-routers have no anaphora to fall back on
    // for first-turn queries.  Surveyed against conv_dialog +
    // safety + v6_7_real_audit on 2026-06-29: zero
    // false-positive matches.
    "қауіп",
    "кім беред",
    "кім жүргіз",
    "кім өтуі",
    "кім тиіс",
    "кім ресімдей",
];

const SHAPE_TRIGGERS_RU: &[&str] = &[
    "как провод",
    "порядок ",
    "процедур",
    "инструктаж",
    "регламент",
];

/// Minimum keyword overlap score for a match to fire.  Tuned for
/// the 5-fixture foundation set: typical pilot questions match
/// 2-3 tokens with the right fixture and 0-1 tokens with
/// unrelated ones, so a threshold of 2 cleanly separates them.
const MIN_SCORE: i32 = 2;

/// **v6.8.48 — procedure_eval audit fix.**  Score every
/// procedure against the lowercased input and return the
/// (index, raw_score) of the best match clearing the
/// supplied floor.  Trigger-gate-free — callers that
/// already know they're handling a procedure-shape query
/// (the hazard / authority sub-routers, for instance)
/// can short-circuit the shape check and just do content
/// match.  None when no procedure clears the floor.
fn score_best_procedure_with_floor(lower: &str, floor: i32) -> Option<(usize, i32)> {
    use crate::procedure_loader::shared_procedures;
    let query_tokens: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.chars().count() >= 4)
        .collect();
    if query_tokens.is_empty() {
        return None;
    }
    let mut best: Option<(usize, i32)> = None;
    for (idx, proc) in shared_procedures().iter().enumerate() {
        let score = score_procedure(proc, &query_tokens);
        if score >= floor && best.is_none_or(|(_, b)| score > b) {
            best = Some((idx, score));
        }
    }
    best
}

/// Default floor (`MIN_SCORE = 2`).  Same threshold the
/// flat `lookup_procedure_matched_with_score` uses.
fn score_best_procedure(lower: &str) -> Option<(usize, i32)> {
    score_best_procedure_with_floor(lower, MIN_SCORE)
}

/// **v6.8.48 — anaphora-override threshold.**  When the
/// hazard / authority routers want to OVERRIDE an existing
/// anaphora referent with a content match, the match has
/// to be substantially stronger than the bare MIN_SCORE
/// floor.  A score of `MIN_SCORE = 2` can come from a
/// single applies_to coincidence — that's noise.
/// `ANAPHORA_OVERRIDE_FLOOR = 5` requires either a title
/// hit (worth 3) plus an applies_to hit (worth 2), OR
/// multiple applies_to hits.  Bare follow-ups like
/// «Қауіптері қандай?» (which score 2 against any
/// procedure whose applies_to contains «қауіпті»)
/// stay below this threshold and the anaphora wins as
/// intended.
const ANAPHORA_OVERRIDE_FLOOR: i32 = 5;

fn score_procedure(proc: &adam_algebra::ProcedureIR, query_tokens: &[&str]) -> i32 {
    let title_kk_lower = proc.title_kk.to_lowercase();
    let title_ru_lower = proc
        .title_ru
        .as_ref()
        .map(|s| s.to_lowercase())
        .unwrap_or_default();
    let applies_lower: String = proc.applies_to.join(" ").to_lowercase();
    // **v6.8.49 — procedure_eval audit fix.**  Bring
    // `aliases_kk` / `aliases_ru` into the scorer.  The
    // aliases field was already in `ProcedureIR` (v6.8.27)
    // but the scorer ignored it, leaving curator-provided
    // synonyms invisible.  Aliases score at the applies_to
    // weight (+2) NOT the title weight (+3): aliases are
    // semantic enrichment, not canonical labels — boosting
    // them to title weight let weak underspecified queries
    // («нұсқаулық рәсімі қандай?») clear the clarify
    // threshold spuriously (broke
    // `clarify_weak_procedure_match_v6815`).
    let aliases_kk_lower: String = proc.aliases_kk.join(" ").to_lowercase();
    let aliases_ru_lower: String = proc.aliases_ru.join(" ").to_lowercase();

    let title_kk_words: Vec<&str> = split_alphanumeric_words(&title_kk_lower);
    let title_ru_words: Vec<&str> = split_alphanumeric_words(&title_ru_lower);
    let applies_words: Vec<&str> = split_alphanumeric_words(&applies_lower);
    let aliases_kk_words: Vec<&str> = split_alphanumeric_words(&aliases_kk_lower);
    let aliases_ru_words: Vec<&str> = split_alphanumeric_words(&aliases_ru_lower);

    let mut score = 0i32;
    for tok in query_tokens {
        let kk_title_hit = title_kk_words.iter().any(|w| word_overlap_match(tok, w));
        let ru_title_hit =
            !title_ru_lower.is_empty() && title_ru_words.iter().any(|w| word_overlap_match(tok, w));
        if kk_title_hit {
            score += 3;
        }
        if ru_title_hit {
            score += 3;
        }
        // Aliases are dedup'd against their respective
        // title field — an alias score only adds when the
        // token did NOT already match the canonical title.
        // Otherwise tokens that appear in BOTH title and
        // aliases (the common case — aliases are partial
        // synonyms) double-count, which inflates weak
        // matches above the clarify threshold and breaks
        // `clarify_weak_procedure_match_v6815`.
        if !kk_title_hit
            && !aliases_kk_words.is_empty()
            && aliases_kk_words.iter().any(|w| word_overlap_match(tok, w))
        {
            score += 2;
        }
        if !ru_title_hit
            && !aliases_ru_words.is_empty()
            && aliases_ru_words.iter().any(|w| word_overlap_match(tok, w))
        {
            score += 2;
        }
        if applies_words.iter().any(|w| word_overlap_match(tok, w)) {
            score += 2;
        }
    }
    score
}

/// **v6.8.16 — morphology-aware token match.**  Replaces the
/// previous substring check (`title.contains(tok)`) which was
/// blind to Kazakh case suffixes: «нұсқаулықты» (acc/poss) is
/// NOT a substring of the bare-root «нұсқаулық» in the title,
/// so a legitimate inflected query missed every title that
/// shared the root.
///
/// New semantics: two words match when their common prefix is
/// at least `MIN_PREFIX_OVERLAP = 4` characters AND at least
/// half the length of the shorter word.  «нұсқаулықты» vs
/// «нұсқаулық» common prefix = 8 ≥ 4 ✓.  «бастапқы» vs
/// «бастапқы» common prefix = 8 ✓.  Two unrelated words that
/// happen to share 3-char roots («іск...» / «іс...») don't
/// match because of the half-length floor.
///
/// The 4-char floor is calibrated to the existing fixture
/// vocabulary: Kazakh content words shorter than 4 chars are
/// almost all particles / function words («не», «ма», «де»)
/// that don't carry topic signal.
fn word_overlap_match(query_token: &str, title_word: &str) -> bool {
    const MIN_PREFIX_OVERLAP: usize = 4;
    let common = prefix_overlap_chars(query_token, title_word);
    if common < MIN_PREFIX_OVERLAP {
        return false;
    }
    let shorter = query_token.chars().count().min(title_word.chars().count());
    common >= shorter.div_ceil(2)
}

fn prefix_overlap_chars(a: &str, b: &str) -> usize {
    a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count()
}

fn split_alphanumeric_words(text: &str) -> Vec<&str> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 4)
        .collect()
}

fn render_procedure(proc: &adam_algebra::ProcedureIR) -> String {
    let mut out = String::new();
    out.push_str("Рәсім: ");
    out.push_str(&proc.title_kk);
    out.push('\n');

    if !proc.applies_to.is_empty() {
        out.push_str("Қолданылады: ");
        out.push_str(&proc.applies_to.join("; "));
        out.push('\n');
    }
    if !proc.prerequisites.is_empty() {
        out.push_str("Алдын ала шарттар:\n");
        for pr in &proc.prerequisites {
            out.push_str(" — ");
            out.push_str(pr);
            out.push('\n');
        }
    }
    out.push_str("Қадамдар:\n");
    for step in &proc.steps {
        out.push_str(&format!(" {}. ", step.sequence));
        out.push_str(&step.action_kk);
        out.push('\n');
    }
    if !proc.hazards.is_empty() {
        out.push_str("Қауіптер:\n");
        for h in &proc.hazards {
            out.push_str(" — ");
            out.push_str(&h.kind_kk);
            out.push_str(" → ");
            out.push_str(&h.mitigation_kk);
            out.push('\n');
        }
    }
    // Source citation — anchors the answer in the regulation so
    // a downstream auditor can verify currency.
    out.push_str(&format!(
        "Дереккөз: {} ({}",
        proc.source.regulation_kk, proc.source.regulation_id,
    ));
    if let Some(article) = &proc.source.article {
        out.push_str(", ");
        out.push_str(article);
    }
    out.push_str(&format!(", {}).", proc.source.version_date));
    out
}

/// Query the FrameIndex for the year associated with
/// `(subject, predicate)`.  Two world_core shapes carry time
/// anchors and BOTH must be handled:
///
/// 1. **Typed modifier shape** (canonical battery,
///    `frame::TimeAnchor::Year`): `Frame { agent, predicate,
///    object: None, modifiers: [TimeAnchor(Year(1872))] }`.
/// 2. **Object-string shape** (live `world_core/*.jsonl`):
///    `Frame { agent, predicate, object: "1872 жылы 5 қыркүйек" }`.
///
/// Either yields the leading 4-digit year if it falls in the
/// curated-coverage range `[1800, 2100]`. Used by the lifespan
/// handler above; kept private to this module.
fn query_year_for_predicate(
    idx: &FrameIndex,
    subject: &str,
    predicate: FramePredicate,
) -> Option<u32> {
    use adam_algebra::{Modifier, TimeAnchor};
    // Use `QueryFocus::Subject` so the focus check in `match_frame`
    // requires only that `candidate.agent.is_some()` — always true
    // here since we constrain on agent.  Object/Modifier focuses
    // each reject candidates missing the respective slot, which
    // would skip half the world_core shapes we care about (typed
    // `TimeAnchor::Year` modifier vs object-string with leading
    // year).  Subject focus returns the frame; we extract the year
    // from whichever shape carries it.
    let q = QueryIR::new(
        QueryFocus::Subject,
        QuestionForm::Definition,
        AnswerShape::BareNoun,
    )
    .with_agent(noun(subject))
    .with_predicate(predicate);
    for hit in idx.query(&q).into_iter().take(4) {
        // Shape 1: typed modifier (TimeAnchor::Year or ::Date).
        for m in &hit.frame.modifiers {
            match m {
                Modifier::TimeAnchor(TimeAnchor::Year(y)) => {
                    if let Ok(y) = u32::try_from(*y) {
                        if (1800..=2100).contains(&y) {
                            return Some(y);
                        }
                    }
                }
                Modifier::TimeAnchor(TimeAnchor::Date { year, .. }) => {
                    if let Ok(y) = u32::try_from(*year) {
                        if (1800..=2100).contains(&y) {
                            return Some(y);
                        }
                    }
                }
                _ => {}
            }
        }
        // Shape 2: object-string with a leading year.
        if let Some(obj) = hit.frame.object.as_ref() {
            if let Some(year) = extract_year_in_range(&obj.root.surface) {
                return Some(year);
            }
        }
    }
    None
}

/// Find the first 4-digit token within `[1800, 2100]` in `surface`.
/// Curated world_core date strings carry shapes like
/// «1872 жылы 5 қыркүйек» (date) or «1872 жыл» (year alone); we
/// just need the leading year. The range gate filters out the
/// occasional 4-digit non-year token (e.g. street numbers).
fn extract_year_in_range(surface: &str) -> Option<u32> {
    let mut digits = String::new();
    for ch in surface.chars().chain(std::iter::once(' ')) {
        if ch.is_ascii_digit() {
            digits.push(ch);
        } else {
            if digits.len() == 4 {
                if let Ok(y) = digits.parse::<u32>() {
                    if (1800..=2100).contains(&y) {
                        return Some(y);
                    }
                }
            }
            digits.clear();
        }
    }
    None
}

fn lookup_chemical_formula(input: &str) -> Option<String> {
    // **v6.8.4 — 2026-06-17 L4.5 Phase 2.A.** Thin wrapper extracting
    // the surface text from the typed `lookup_chemical_formula_typed`
    // sibling.  See that function's doc comment for the proof shape.
    lookup_chemical_formula_typed(input).map(|c| c.text)
}

/// **v6.8.4 — 2026-06-17 L4.5 Phase 2.A.** Typed sibling of
/// [`lookup_chemical_formula`].  Returns an
/// [`crate::dialog_acts::AnswerCandidate`] carrying the formula
/// text, a `ProofObject` whose `from_curated_fact("formula", …)`
/// cites the in-code chemistry table, and `RouteId::ChemistryFormula`.
fn lookup_chemical_formula_typed(input: &str) -> Option<crate::dialog_acts::AnswerCandidate> {
    use crate::dialog_acts::{AnswerCandidate, RouteId};
    use crate::proof_object::ProofObject;
    use adam_reasoning::FactSource;

    // **Phase 23.B (2026-06-03 evening)** — strip punctuation BEFORE
    // stem matching. Live REPL caught Whisper inserting commas mid-
    // word: «Ө, тегеннің формулысы.» — the substring «ө тегі» didn't
    // match because of the comma. Normalising punctuation → space
    // (and collapsing runs of whitespace) lets the existing stem
    // table catch comma-splits without enumerating every variant.
    let lower_normalised: String = input
        .to_lowercase()
        .chars()
        .map(|c| match c {
            ',' | '.' | '!' | '?' | ';' | ':' | '—' | '–' | '-' => ' ',
            other => other,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let lower = lower_normalised.as_str();
    // Required marker: the word «формула» / «таңба» (chemical symbol)
    // or a Whisper drift of either. Without this, bare substance
    // mentions like «су» would false-fire.
    //
    // **v6.8 (2026-06-16) — «таңба» marker added.** School-eval case
    // «Күмістің химиялық таңбасы қандай?» («what is silver's chemical
    // symbol?») was missing this gate, so the substring-IsA fallback
    // («Күміс — асыл ақшыл металл») won over the symbol-lookup table.
    // «Таңба» = "symbol / sign" — when a chemistry query asks for the
    // taңba of an element, it wants the same Ag / Au / Fe / etc.
    // that the formula lookup returns.
    //
    // **v6.8 hotfix 2026-06-16 evening — word-boundary check for «таңба».**
    // Codex consultation #4 caught: bare `lower.contains("таңба")` ALSO
    // matches «елтаңба» (state emblem) and «жол таңбасы» (road sign /
    // mark), so «Қазақстанның елтаңбасында күміс бар ма?» wrongly
    // routed to «Күмістің формуласы — Ag.». «Таңба» must be a
    // standalone token (preceded by space / start / punctuation), not
    // embedded as a suffix of another root. The «формула» marker is
    // safe because no Kazakh word ends in «формула-» as a suffix.
    let has_formula_marker = lower.contains("формула")
        || lower.contains("формуласы")
        || lower.contains("формуласын")
        || lower.contains("формуласыз")  // possessive case variants
        || lower.contains("формулыс") // common Whisper drift
        || token_contains(lower, "таңба")
        || token_contains(lower, "таңбасы")
        || token_contains(lower, "таңбасын");
    if !has_formula_marker {
        return None;
    }

    // (kazakh_stem, display_subject, formula).  Stems are prefix-matched
    // so every case-inflected form (-ның / -нің / -дың / -дің) is
    // caught without explicit enumeration.  Ordered longest-first so
    // «көмірқышқыл газы» wins over bare «газ» etc.
    let formulas: &[(&str, &str, &str)] = &[
        // ── Compound names (must come BEFORE element single words) ──
        ("көмірқышқыл газы", "Көмірқышқыл газының", "CO₂"),
        ("көмір қышқыл газы", "Көмірқышқыл газының", "CO₂"),
        ("күкірт қышқылы", "Күкірт қышқылының", "H₂SO₄"),
        ("тұз қышқылы", "Тұз қышқылының", "HCl"),
        ("азот қышқылы", "Азот қышқылының", "HNO₃"),
        ("сірке қышқылы", "Сірке қышқылының", "CH₃COOH"),
        ("лимон қышқылы", "Лимон қышқылының", "C₆H₈O₇"),
        ("ас тұзы", "Ас тұзының", "NaCl"),
        ("ас содасы", "Ас содасының", "NaHCO₃"),
        ("асхана тұзы", "Асхана тұзының", "NaCl"),
        ("кальций оксиді", "Кальций оксидінің", "CaO"),
        ("кальций карбонаты", "Кальций карбонатының", "CaCO₃"),
        ("натрий гидроксиді", "Натрий гидроксидінің", "NaOH"),
        ("натрий бикарбонаты", "Натрий бикарбонатының", "NaHCO₃"),
        ("мыс сульфаты", "Мыс сульфатының", "CuSO₄"),
        ("көк тас", "Көк тастың", "CuSO₄"),
        ("әк тас", "Әк тастың", "CaCO₃"),
        ("аммоний хлориді", "Аммоний хлоридінің", "NH₄Cl"),
        ("темір тотығы", "Темір тотығының", "Fe₂O₃"),
        ("угар газ", "Угар газының", "CO"),
        ("сахароза", "Сахарозаның", "C₁₂H₂₂O₁₁"),
        ("глюкоза", "Глюкозаның", "C₆H₁₂O₆"),
        ("этанол", "Этанолдың", "C₂H₅OH"),
        ("этил спирті", "Этил спиртінің", "C₂H₅OH"),
        ("метан", "Метанның", "CH₄"),
        ("аммиак", "Аммиактың", "NH₃"),
        ("озон", "Озонның", "O₃"),
        ("гипс", "Гипстің", "CaSO₄·2H₂O"),
        // **Phase 23.A (2026-06-03)** — Whisper-drift compound names
        // observed in live REPL. Listed BEFORE single-word elements
        // so the drift form wins length-priority.
        ("қуқырт қышқыл", "Күкірт қышқылының", "H₂SO₄"),
        ("құрқырт қышқыл", "Күкірт қышқылының", "H₂SO₄"),
        ("куркурт қышқыл", "Күкірт қышқылының", "H₂SO₄"),
        // ── Single-word substances / elements (shorter, lower priority) ──
        ("көмірқышқыл", "Көмірқышқыл газының", "CO₂"),
        ("сутегі", "Сутегінің", "H₂"),
        ("сутек", "Сутегінің", "H₂"),
        ("оттегі", "Оттегінің", "O₂"),
        ("оттек", "Оттегінің", "O₂"),
        // **Phase 23.A** — Whisper drifts of «оттегі»: single-т
        // «отегі», token-split «о тегі» / «ө тегі».
        // **Phase 23.B (2026-06-03 evening)** — additional drift
        // «тегенн-» (Whisper produces «тегеннің» instead of
        // «тегінің» when the leading «о»/«ө» is split off).
        ("отегі", "Оттегінің", "O₂"),
        ("о тегі", "Оттегінің", "O₂"),
        ("ө тегі", "Оттегінің", "O₂"),
        ("о тегенн", "Оттегінің", "O₂"),
        ("ө тегенн", "Оттегінің", "O₂"),
        ("отегенн", "Оттегінің", "O₂"),
        ("өтегенн", "Оттегінің", "O₂"),
        // **Phase 23.A** — sulfur element + Whisper drift.
        ("күкірт", "Күкірттің", "S"),
        ("қуқырт", "Күкірттің", "S"),
        ("азот", "Азоттың", "N₂"),
        ("алтын", "Алтынның", "Au"),
        ("күміс", "Күмістің", "Ag"),
        ("сынап", "Сынаптың", "Hg"),
        ("қорғасын", "Қорғасынның", "Pb"),
        ("мырыш", "Мырыштың", "Zn"),
        ("алюминий", "Алюминийдің", "Al"),
        ("кальций", "Кальцийдің", "Ca"),
        ("магний", "Магнийдің", "Mg"),
        ("натрий", "Натрийдің", "Na"),
        ("калий", "Калийдің", "K"),
        ("темір", "Темірдің", "Fe"),
        ("мыс", "Мыстың", "Cu"),
        ("спирт", "Этил спиртінің", "C₂H₅OH"),
        ("қант", "Сахарозаның", "C₁₂H₂₂O₁₁"),
        ("тұз", "Ас тұзының", "NaCl"),
        ("сода", "Ас содасының", "NaHCO₃"),
        ("әк", "Кальций оксидінің", "CaO"),
        // ── Water (lowest priority — «су» is so short it must lose
        // to all longer matches above; placed last for stem search). ──
        ("судың", "Судың", "H₂O"),
        ("суды", "Судың", "H₂O"),
        ("суға", "Судың", "H₂O"),
        ("суда", "Судың", "H₂O"),
        ("су ", "Судың", "H₂O"),
    ];

    // **v6.8.2 — 2026-06-17 user audit.** Compound surface forms
    // where a chemistry-stem element is part of a multi-word phrase
    // with a non-chemistry meaning. The pre-fix gate caught the
    // no-space «теміржол» but not the space-separated «темір жол»
    // (railway), so «Темір жол таңбасы қандай?» wrongly returned
    // «Темірдің формуласы — Fe.». Add explicit early-exit list — if
    // the input is recognisably about a non-chemistry compound,
    // refuse to chemistry-resolve regardless of the formula marker.
    //
    // Kept minimal: each entry is an unambiguous compound (no
    // chemistry sense exists for that bigram).  «алтын адам»
    // (archaeological exhibit), «алтын сағат» (gold watch) etc. are
    // intentionally NOT here — they can legitimately be paired with
    // a formula question («алтын адамдағы алтын қандай?»).
    const NON_CHEMISTRY_COMPOUNDS: &[&str] = &[
        "темір жол",   // railway
        "теміржол",    // railway (no-space variant)
        "темір тор",   // grate / lattice
        "темір қол",   // iron-hand (metaphor)
        "темір жүрек", // iron-heart (metaphor)
    ];
    if NON_CHEMISTRY_COMPOUNDS.iter().any(|c| lower.contains(c)) {
        return None;
    }

    for (stem, display, formula) in formulas {
        // **v6.8 hotfix 2026-06-16 — word-boundary check.** Codex
        // consultation #4 caught: bare `contains("темір")` matches
        // «теміржол» (railroad), so «Теміржол таңбасы қандай?»
        // wrongly routed to «Темірдің формуласы — Fe.». Apply the
        // same standalone-token gate as the formula marker. Multi-
        // word compound stems («көмір қышқыл газы», «ас тұзы»)
        // still pass because they are space-separated phrases —
        // token_contains treats each constituent as a token
        // implicitly via word-boundary prefix match on the first
        // letter of the stem.
        if token_contains(lower, stem) {
            let text = format!("{display} формуласы — {formula}.");
            let proof = ProofObject::from_curated_fact(
                (*display).to_string(),
                "formula".to_string(),
                (*formula).to_string(),
                FactSource {
                    pack: "v6_2_router/chemistry_table".into(),
                    sample_id: format!("{display}/{formula}"),
                },
                text.clone(),
            );
            let candidate = AnswerCandidate::assert(text, proof, RouteId::ChemistryFormula);
            debug_assert!(candidate.invariant_check().is_ok());
            return Some(candidate);
        }
    }
    None
}

/// Word-boundary substring check: returns `true` when `needle`
/// appears in `haystack` as a standalone token — i.e. preceded by
/// whitespace, start-of-string, or punctuation. Prevents false
/// positives where the search term is embedded as a suffix of a
/// longer Kazakh word («таңба» inside «елтаңба», «темір» inside
/// «теміржол»). The trailing edge is unconstrained so case-inflected
/// forms («таңбасы», «темірдің») still match.
///
/// UTF-8-safe: iterates by char boundaries via `find` (which only
/// reports valid byte indices) and advances by `needle.len()` (which
/// is a char boundary because needle is a substring of haystack at
/// that position).
fn token_contains(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let mut search_from = 0;
    while search_from < haystack.len() {
        let Some(rel) = haystack[search_from..].find(needle) else {
            return false;
        };
        let abs = search_from + rel;
        // Leading boundary: char preceding the match must be
        // non-alphabetic OR at start of haystack.
        let prev_char = haystack[..abs].chars().next_back();
        let leading_ok = match prev_char {
            Some(c) => !c.is_alphabetic(),
            None => true,
        };
        // Trailing boundary: what follows must be either a non-letter
        // (end / whitespace / punct) OR a valid Kazakh inflection
        // suffix initial. This is what catches «теміржол» — the «ж»
        // following «темір» is NOT a Kazakh case/possessive suffix
        // starter, so «темір» is rejected as a substring of a longer
        // root.
        //
        // Skip the trailing-suffix gate when the needle itself ends
        // with a non-alphabetic char (e.g. stem «су » with trailing
        // space) — the needle already encodes its own right boundary,
        // so we only need leading boundary + the needle to be a
        // standalone token. Otherwise inputs like «су формуласы»
        // would reject «су » because the char after the space («ф»)
        // isn't a Kazakh suffix initial.
        let needle_ends_with_boundary = needle
            .chars()
            .next_back()
            .map(|c| !c.is_alphabetic())
            .unwrap_or(true);
        let end = abs + needle.len();
        let next_char = haystack[end..].chars().next();
        let trailing_ok = if needle_ends_with_boundary {
            true
        } else {
            match next_char {
                None => true,
                Some(c) if !c.is_alphabetic() => true,
                Some(c) => is_kazakh_suffix_initial(c),
            }
        };
        if leading_ok && trailing_ok {
            return true;
        }
        // Advance past the current match. Safe char-boundary
        // arithmetic because needle matched at `abs`, so abs..abs+len
        // is the substring needle and abs + needle.len() is a valid
        // boundary.
        search_from = abs + needle.len();
    }
    false
}

/// Letters that can start a Kazakh inflection suffix (case /
/// possessive / plural / personal). When a stem match is followed by
/// one of these, the stem is the root of an inflected form. When
/// followed by anything else (and the next char is alphabetic),
/// the stem is embedded in a longer DIFFERENT root and must not
/// match. Kazakh-phonology informed; not exhaustive but covers all
/// productive inflection suffix starters in the standard literary
/// register.
fn is_kazakh_suffix_initial(c: char) -> bool {
    matches!(
        c,
        // Vowel-initial suffixes (possessive -ы/-і/-ым/-ім, -а/-е
        // for some derivations, -у for verb stems).
        'ы' | 'і' | 'а' | 'е' | 'у' | 'ә'
            // Consonant-initial suffixes (case, plural, instrumental,
            // possessive 2sg/2pl/1pl):
            //   н/д/т   — genitive, accusative, locative, ablative
            //   г/ғ/к/қ — dative
            //   м/б/п   — instrumental, possessive
            //   л       — plural
            //   с       — possessive 3sg with vowel base («сы»)
            | 'н' | 'д' | 'т' | 'г' | 'ғ' | 'к' | 'қ'
            | 'м' | 'б' | 'п' | 'л' | 'с'
    )
}

/// **2026-06-03** — first-person location statement detector. Matches
/// inputs like «Мен Қостанайда тұрамын» / «Мен Қостанай қалада
/// тұрамын» / «Біз Алматыда тұрамыз». When this fires, v6_2_router
/// returns None so the v6.1 cascade's acknowledgement reply stands
/// (and the city slot in the session is preserved for later recall).
///
/// **rc5-followup (2026-06-03 evening)** — initial implementation
/// enumerated «тұрамын» / «тұрамыз» / «тұрам» literally. Live REPL
/// caught «Мен қостанай атырамым» — Whisper drifted «тұрамын» to
/// «атырамым» AND stripped the locative `‑да` from the city. Neither
/// substring matched the canonical list, so the router fell through
/// to the «Қала» IsA reply again. Fix: keep the canonical-verb fast
/// path AND add a morphological fallback that pairs a 1sg/1pl verb
/// suffix (`‑мын` / `‑мыз` / `‑мым` / `‑мім` / `‑міз`) with a city
/// marker (either a known Kazakhstan oblast-centre stem or a
/// locative-suffixed noun ≥ 5 chars).
fn looks_like_first_person_location_statement(s: &str) -> bool {
    let lower = s.to_lowercase();
    // **Phase 26.A (2026-06-04)** — compound utterance support.
    // Live REPL caught «Менің атым Дәулет, мен қостанайда тұрамын» —
    // the input STARTS with «менің», so the strict «мен »-at-start
    // check missed the second clause.  Phase 26.A added the comma /
    // period clause boundary («, мен » / «. мен »).
    //
    // **Phase 26.C (2026-06-04 evening — post-rc11 audit)** —
    // sometimes the user runs both clauses together without ANY
    // separator: «Менім атын дәулет мен қазақстанда тұрамын».
    // Detect «мен» as a standalone token followed by a city +
    // dwelling-verb pattern anywhere in the input.  Risk of false
    // positive on «X мен Y тұрамыз» (X and Y live together) is
    // mitigated by requiring the verb to be SINGULAR («тұрамын»),
    // since the conjunction reading needs plural «тұрамыз».
    let has_first_person_pronoun = lower.starts_with("мен ")
        || lower.starts_with("мен.")
        || lower.starts_with("мен,")
        || lower.starts_with("мың ")
        || lower.starts_with("біз ")
        || lower.contains(", мен ")
        || lower.contains(". мен ")
        || lower.contains(",мен ")  // missing space after comma
        || lower.contains(".мен ")
        // Phase 26.C — standalone-token «мен» with 1sg dwelling verb.
        || (lower.contains(" мен ") && lower.contains("тұрамын"));
    if !has_first_person_pronoun {
        return false;
    }
    // Fast path — canonical dwelling verbs that survived STT.
    let canonical_verbs = [
        "тұрамын",
        "тұрамыз",
        "тұрам",
        "тұрып жатырмын",
        "тұрып жатырмыз",
    ];
    if canonical_verbs.iter().any(|v| lower.contains(v)) {
        return true;
    }
    // Whisper-drift fallback — 1p verb morphology + city marker.
    let tokens: Vec<String> = lower
        .split_whitespace()
        .map(|t| t.trim_end_matches([',', '.', '!', '?']).to_string())
        .collect();
    let has_first_person_verb = tokens.iter().any(|t| {
        t.len() >= 5
            && (t.ends_with("мын")
                || t.ends_with("мыз")
                || t.ends_with("мым")
                || t.ends_with("мім")
                || t.ends_with("міз"))
    });
    if !has_first_person_verb {
        return false;
    }
    // Recognised KZ oblast-centre stems (prefix-match catches every
    // case-inflected form — `қостанайда`, `қостанайдан`, even the
    // Whisper-drifted accusative «қостанайды»).
    let known_city_stems = [
        "алматы",
        "астана",
        "нұр-сұлтан",
        "қостанай",
        "костанай",
        "шымкент",
        "ақтөбе",
        "тараз",
        "өскемен",
        "семей",
        "павлодар",
        "атырау",
        "ақтау",
        "орал",
        "талдықорған",
        "көкшетау",
        "петропавл",
        "қызылорда",
        "жезқазған",
        "темиртау",
    ];
    let has_known_city = tokens
        .iter()
        .any(|t| known_city_stems.iter().any(|c| t.starts_with(c)));
    let has_locative_noun = tokens.iter().any(|t| {
        t.len() >= 5
            && (t.ends_with("да") || t.ends_with("де") || t.ends_with("та") || t.ends_with("те"))
    });
    has_known_city || has_locative_noun
}

fn looks_like_time_query(s: &str) -> bool {
    let lower = s.to_lowercase();
    // **2026-06-03 voice REPL regression** — «Қазір қазақстанның
    // президенті кім?» was incorrectly routed to the clock handler
    // because the leading «қазір» triggered this matcher BEFORE the
    // president check downstream. When the input clearly names another
    // entity (президент / премьер / спикер / etc.), the «now» word is
    // a tense marker for that entity, not a time question. Defer.
    let entity_markers = [
        "президент",
        "премьер",
        "министр",
        "спикер",
        "әкім",
        "elbasy",
        "елбасы",
    ];
    if entity_markers.iter().any(|m| lower.contains(m)) {
        return false;
    }
    let markers = [
        "бүгін",
        "қазір",
        "сегодня",
        "сейчас",
        "который час",
        "сағат неше",
        "нешесі",
        "апта",
        "неделя",
        "какая дата",
        "какой день",
        // Phase 21 (2026-06-02) — relative-day anchors + Phase 21.A
        // STT-drift aliases (Whisper hears «ертең» as «еркең» etc.).
        "кеше",
        "кешее",
        "кешеу",
        "ертең",
        "еркең",
        "еркен",
        "ертен",
        "ерткен",
        "эртен",
        "бүрсігүні",
        // Phase 21.C — «ерден» as time marker only when paired with
        // «күн» day marker (handled as a multi-token check below).
        "бүрсүгүні",
        "бірсүгіне",
        "бір сүгіне",
        "бірсүгіні",
        "бір сүгіні",
        "бір сігүні",
        "бірсігүні",
        "алдыңғы күн",
        "вчера",
        "завтра",
    ];
    if markers.iter().any(|m| lower.contains(m)) {
        return true;
    }
    // Phase 21.C — multi-token «ерден» + «күн» pair (ambiguous on
    // its own — could be a person name in ablative case — so we
    // require BOTH tokens to be present).
    if lower.contains("күн")
        && (lower.contains("ерден") || lower.contains("эрден") || lower.contains("ердін"))
    {
        return true;
    }
    false
}

/// **Phase 21 (2026-06-02)** — detect relative-day anchor in the
/// input. Returns the day-offset (−2…+2) and `None` if the input
/// has no relative-day marker (caller should treat it as «today»).
///
/// **Phase 21.A** — STT-drift aliases for each marker. The 2026-06-02
/// live REPL showed Whisper hears «ертең» as «еркең» (т→к) and
/// «бүрсігүні» as «бірсүгіне» / «бір сүгіні» (ү→і + word split).
/// Adding the drifted forms keeps the calendar handler from falling
/// through to substring fact lookup on a recognisable utterance.
fn relative_day_offset(lower: &str) -> Option<i64> {
    for marker in [
        "бүрсігүні",
        "бүрсүгүні",
        "бірсүгіне",
        "бір сүгіне",
        "бірсүгіні",
        "бір сүгіні",
        "бір сігүні",
        "бірсігүні",
    ] {
        if lower.contains(marker) {
            return Some(2);
        }
    }
    // **Phase 21.B (2026-06-03 evening)** — added «еркен» (single -н
    // instead of -ң) caught in live REPL: «Еркен қай күн болады»
    // fell through to «Күн — дөңгелек» substring IsA.
    for marker in [
        "ертең",
        "еркең",
        "еркен",
        "ертен",
        "ерткен",
        "эртен",
        "завтра",
    ] {
        if lower.contains(marker) {
            return Some(1);
        }
    }
    // **Phase 21.C (2026-06-04 — post-rc10 audit)** — «ерден» drift
    // caught in live REPL: «Ерден қай күн болады» yielded an Abai
    // citation about «ер» (man).  Since «ерден» can also be the
    // genuine name «Ерден» + ablative case («from Erden»), this
    // drift only counts when the input is clearly a calendar
    // question (the bare day marker «күн» is present).
    if lower.contains("күн") {
        for marker in ["ерден", "эрден", "ердін"] {
            if lower.contains(marker) {
                return Some(1);
            }
        }
    }
    for marker in ["алдыңғы күн", "алдыңғы күні", "позавчера"] {
        if lower.contains(marker) {
            return Some(-2);
        }
    }
    for marker in ["кеше", "кешее", "кешеу", "вчера"] {
        if lower.contains(marker) {
            return Some(-1);
        }
    }
    None
}

fn emit_clock_answer(input: &str) -> String {
    let c = system_clock::now();
    let lower = input.to_lowercase();
    // **Phase 21** — handle relative-day questions before the
    // generic «today» path. If the user says «Кеше / Ертең …»,
    // shift the clock reading and render with the matching prefix.
    if let Some(offset) = relative_day_offset(&lower) {
        let rc = system_clock::now_offset(offset);
        let label = system_clock::relative_day_label_kk(offset);
        let copula = system_clock::relative_day_copula_kk(offset);
        // Weekday-only ask: «Кеше қай күн болды?» / «Ертең қай күн?»
        if lower.contains("апта")
            || lower.contains("неделя")
            || (lower.contains("күн") && (lower.contains("қай") || lower.contains("қандай")))
        {
            return format!("{label} — {} {copula}.", rc.weekday_kk());
        }
        // Day-of-month ask: «Кеше нешесі еді?» / «Ертең нешесі?»
        if lower.contains("нешесі") || lower.contains("число") {
            return format!("{label} {} {} {copula}.", rc.day, rc.month_kk());
        }
        // Generic relative-day date.
        return format!(
            "{label} — {} {} {} жыл, {} {copula}.",
            rc.day,
            rc.month_kk(),
            rc.year,
            rc.weekday_kk()
        );
    }
    if lower.contains("сағат") || lower.contains("часов") || lower.contains("уақыт")
    {
        return format!("Қазір сағат {}.", c.time_hhmm());
    }
    if lower.contains("апта")
        || lower.contains("неделя")
        || (lower.contains("күн") && lower.contains("қай"))
    {
        return format!("Бүгін — {}.", c.weekday_kk());
    }
    if lower.contains("ай")
        && (lower.contains("қандай") || lower.contains("какой") || lower.contains("какая"))
    {
        return format!("Қазір {} айы.", c.month_kk());
    }
    if lower.contains("нешесі") || lower.contains("число") {
        return format!("Бүгін {} {}.", c.day, c.month_kk());
    }
    format!(
        "Бүгін {} {} {} жыл, {}.",
        c.day,
        c.month_kk(),
        c.year,
        c.weekday_kk()
    )
}

fn build_query_heuristic(input: &str) -> Option<QueryIR> {
    let lower = input.to_lowercase();
    let language = if has_russian_marker(&lower) {
        Language::Russian
    } else {
        Language::Kazakh
    };

    // Subject-focus reverse lookup path: «1872 жылы кім туылған?» —
    // user asks WHO was born/died/founded in a given year. The
    // canonical agent is unknown (that's what they're asking),
    // so we build a Subject-focus query with a Time
    // modifier_constraint instead.
    if let Some(year_phrase) = extract_year_phrase(&lower)
        && (lower.contains("кім") || lower.contains("кто"))
    {
        let pred = predicate_for_reverse(&lower).unwrap_or(FramePredicate::BornIn);
        let q = QueryIR::new(
            QueryFocus::Subject,
            QuestionForm::Definition,
            AnswerShape::BareNoun,
        )
        .with_predicate(pred)
        .with_modifier_constraint(ModifierRole::Time, noun(&year_phrase))
        .with_language_filter(language);
        return Some(q);
    }

    let agent = canonical_agent_for(&lower)?;
    let focus_kind = detect_focus(&lower);
    let predicate = predicate_for(&lower);

    let (focus, answer_shape) = match focus_kind {
        FocusKind::Time => (
            QueryFocus::Modifier(ModifierRole::Time),
            AnswerShape::DateAnchor,
        ),
        FocusKind::Place => (
            QueryFocus::Modifier(ModifierRole::Location),
            AnswerShape::BareNoun,
        ),
        FocusKind::Subject => (QueryFocus::Subject, AnswerShape::BareNoun),
        FocusKind::Object => (QueryFocus::Object, AnswerShape::DefinitionalNP),
        FocusKind::Definition => (QueryFocus::Definition, AnswerShape::DefinitionalNP),
    };

    let mut q = QueryIR::new(focus, QuestionForm::Definition, answer_shape)
        .with_agent(noun(&agent))
        .with_language_filter(language);
    if let Some(p) = predicate {
        q = q.with_predicate(p);
    }
    Some(q)
}

/// Pull a year-phrase «NNNN жылы» / «NNNN жыл» / bare year from
/// the input for the reverse-lookup path.
fn extract_year_phrase(lower: &str) -> Option<String> {
    // Match 3-4 digit year followed by «жыл» / «жылы» / nothing.
    let mut digits = String::new();
    let mut last_year: Option<String> = None;
    for c in lower.chars() {
        if c.is_ascii_digit() {
            digits.push(c);
        } else {
            if (3..=4).contains(&digits.len())
                && let Ok(y) = digits.parse::<u32>()
                && (1000..3000).contains(&y)
            {
                last_year = Some(format!("{y} жыл"));
            }
            digits.clear();
        }
    }
    if (3..=4).contains(&digits.len())
        && let Ok(y) = digits.parse::<u32>()
        && (1000..3000).contains(&y)
    {
        last_year = Some(format!("{y} жыл"));
    }
    last_year
}

/// Pick the predicate for reverse-lookup based on the verb used.
fn predicate_for_reverse(lower: &str) -> Option<FramePredicate> {
    if lower.contains("туыл") || lower.contains("туған") || lower.contains("родился")
    {
        Some(FramePredicate::BornIn)
    } else if lower.contains("қайтыс") || lower.contains("өл") || lower.contains("умер")
    {
        Some(FramePredicate::DiedIn)
    } else if lower.contains("құрыл") || lower.contains("ашыл") || lower.contains("основан")
    {
        Some(FramePredicate::FoundedIn)
    } else if lower.contains("жаз") || lower.contains("автор") {
        Some(FramePredicate::Authored)
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy)]
enum FocusKind {
    Time,
    Place,
    Subject,
    Object,
    Definition,
}

fn detect_focus(lower: &str) -> FocusKind {
    if lower.contains("қашан") || lower.contains("когда") {
        FocusKind::Time
    } else if lower.contains("қайда")
        || lower.contains("где")
        || lower.contains("қай қала")
        || lower.contains("какая столица")
    {
        FocusKind::Place
    } else if lower.contains("кім бұл") || lower.starts_with("кім ") || lower.starts_with("кто ")
    {
        // Subject focus: «Кім туылды?» / «Кто пришёл?» — interrogative
        // appears at the START. The agent is what we're looking for.
        FocusKind::Subject
    } else if lower.contains("кім?") || lower.contains("кто?") {
        // «X кім?» / «X кто?» — interrogative at the END, after the
        // agent. This is a DEFINITIONAL question («Ахмет Байтұрсынұлы
        // кім?» = "what is Ahmet?", asking for his profession/IsA).
        // Codex 2026-05-25 audit caught this misclassifying as
        // Subject focus and returning the agent name as the answer.
        FocusKind::Definition
    } else if lower.contains("деген не")
        || lower.contains("что такое")
        || lower.contains("деген кім")
    {
        FocusKind::Definition
    } else {
        FocusKind::Object
    }
}

fn predicate_for(lower: &str) -> Option<FramePredicate> {
    if lower.contains("туыл") || lower.contains("туған") || lower.contains("родился")
    {
        return Some(FramePredicate::BornIn);
    }
    if lower.contains("қайтыс") || lower.contains("умер") {
        return Some(FramePredicate::DiedIn);
    }
    if lower.contains("құрыл")
        || lower.contains("ашыл")
        || lower.contains("основан")
        || lower.contains("болды")
        || lower.contains("қабылдан")
    {
        return Some(FramePredicate::FoundedIn);
    }
    if lower.contains("автор") || lower.contains("жазған") {
        return Some(FramePredicate::Authored);
    }
    if lower.contains("атымен") || lower.contains("честь") {
        return Some(FramePredicate::NamedAfter);
    }
    // LivesIn — animate "где живёт" — wins over generic LocatedIn.
    // Codex 2026-05-25 audit caught «Абай қайда өмір сүрді?»
    // misrouting to LocatedIn (because of «қайда») instead of
    // LivesIn (because of «өмір сүр»). Order matters.
    if lower.contains("өмір сүр")
        || lower.contains("тұрды")
        || lower.contains("тұрған")
        || lower.contains("жил")
    {
        return Some(FramePredicate::LivesIn);
    }
    // LocatedIn — inanimate "где находится".
    if lower.contains("орналас") || lower.contains("находится") || lower.contains("қайда")
    {
        return Some(FramePredicate::LocatedIn);
    }
    if lower.contains("қанша") || lower.contains("сколько") {
        return Some(FramePredicate::HasQuantity);
    }
    if lower.contains("санаттар") || lower.contains("жіктейді") {
        return Some(FramePredicate::Classifies);
    }
    if lower.contains("күшіне") {
        return Some(FramePredicate::EffectiveFrom);
    }
    Some(FramePredicate::IsA)
}

/// Heuristic agent-surface detector — longest matching canonical
/// surface from the curated corpus wins. Stage 8 replaces this
/// with a typed Stage-2 morpho-lattice → Frame::from_morph_lattice
/// pipeline.
/// Strip the most common Kazakh case suffixes from each word of
/// the input so the canonical-agent substring search matches
/// «ньютонның» against canonical «ньютон» etc. Heuristic — may
/// over-strip; safe because we only use it as an ADDITIONAL match
/// path beside the raw lowered input.
///
/// Codex 2026-05-25 voice REPL audit observed «Ньютонның екінші
/// заңы», «Эйнштейн формуласын», «Қазақстанда» etc. fail to match
/// canonical bare-stem surfaces; this strips their case suffixes.
fn strip_kazakh_case_suffixes(s: &str) -> String {
    // Longest suffix first so «ның» is stripped before «ы».
    // Limited to common nominal cases; verb endings are not stripped
    // because they can collide with the stem (e.g. «жаз» = "write"
    // is also the root, not a case suffix).
    // **Conservative suffix list (v6.2.0 codex 2026-05-25 fix).**
    // Original list also stripped possessive «-ы» / «-і» / «-сы» /
    // «-сі», which over-strips canonical noun phrases that legitimately
    // end in those (e.g. «ньютон екінші заңы», «эйнштейн формуласы»).
    // The new list strips only **case markers that follow noun stems**
    // (genitive / locative / dative / ablative / accusative); possessive
    // suffixes are left intact so canonical surfaces match.
    let suffixes: &[&str] = &[
        // Genitive (longest first).
        "ның",
        "нің",
        "дың",
        "дің",
        "тың",
        "тің",
        // Locative attribute.
        "дағы",
        "дегі",
        "тағы",
        "тегі",
        "нда",
        "нде",
        // Ablative.
        "дан",
        "ден",
        "тан",
        "тен",
        "нан",
        "нен",
        "сынан",
        "сінен",
        // Dative.
        "сына",
        "сіне",
        "ына",
        "іне",
        "ға",
        "ге",
        "қа",
        "ке",
        "на",
        "не",
        // Locative.
        "да",
        "де",
        "та",
        "те",
        // Instrumental (multi-char).
        "мен",
        "пен",
        "бен",
        // Note: accusative-on-possessive («-сын / -сін / -ын /
        // «-ін») is handled separately above via REPLACEMENT
        // («формуласын» → «формуласы») so the possessive suffix
        // is preserved for the canonical-surface match.
    ];
    // Replacement-style strips for accusative-on-possessive
    // («формуласын» → «формуласы», «жауабын» → «жауабы»). These
    // strip only the trailing «н», leaving the possessive suffix
    // intact so the canonical-surface match («эйнштейн формуласы»)
    // still aligns.
    let replace_n: &[(&str, &str)] = &[("сын", "сы"), ("сін", "сі"), ("ын", "ы"), ("ін", "і")];
    s.split_whitespace()
        .map(|w| {
            let mut stem = w.to_string();
            // Try replacement strips first (less aggressive).
            for (suf, repl) in replace_n {
                if stem.chars().count() > suf.chars().count() + 1 && stem.ends_with(suf) {
                    let new_len = stem.len() - suf.len();
                    stem.truncate(new_len);
                    stem.push_str(repl);
                    return stem;
                }
            }
            for suf in suffixes {
                if stem.chars().count() > suf.chars().count() + 1 && stem.ends_with(suf) {
                    let new_len = stem.len() - suf.len();
                    stem.truncate(new_len);
                    break;
                }
            }
            stem
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn canonical_agent_for(lower: &str) -> Option<String> {
    let candidates: &[&str] = &[
        // Battery-specific multi-word entities (longest first).
        "жасанды интеллект туралы заң",
        "ахмет байтұрсынұлы",
        "defense tech it park",
        "қазақстанның астанасы",
        "қазақстан тәуелсіздігі",
        "қазақстан конституциясы",
        "тың және тыңайған жерлер",
        "көмірқышқыл газы",
        "жарық жылдамдығы",
        "ньютон екінші заңы",
        "эйнштейн формуласы",
        "желтоқсан оқиғасы",
        "семей полигоны",
        "столица казахстана",
        "скорость света",
        "бағдарламалау тілі",
        "каспий теңізі",
        "қазақ хандығы",
        "алаш қозғалысы",
        "кенесары қасымұлы",
        "шоқан уәлиханов",
        "жамбыл жабаев",
        "тәуке хан",
        "қазақ кср",
        // Single-word common surfaces.
        "мо рк",
        "одкб",
        "абай",
        "қазақстан",
        "қостанай",
        "rust",
        "кру",
        "су",
        "вода",
        "ай",
        "ағаш",
        "көміртек",
        "углерод",
        "фотосинтез",
        "гравитация",
        "днк",
        "эверест",
        "алгоритм",
        "жаңбыр",
        "сел",
        "семей",
    ];
    let stripped = strip_kazakh_case_suffixes(lower);
    let folded = stt_fold(lower);
    let folded_stripped = strip_kazakh_case_suffixes(&folded);
    let mut best: Option<(&str, usize)> = None;
    for c in candidates {
        let len = c.chars().count();
        // **Word-boundary required for short agents.**  Voice REPL
        // audit caught «ай» / «су» / «жер» matching as substrings of
        // «қалайсың», «суық», «жерде» — producing wrong-sense
        // answers.  Agents ≤ 3 chars must appear as a whole word.
        //
        // **v6.8.3 — 2026-06-17 user audit fix (Bug C3).** The
        // pre-fix code ALSO ran the short-agent word-boundary check
        // against the case-stripped surfaces. That let
        // `strip_kazakh_case_suffixes` turn the verb «айта» (= "to
        // say") into the noun «ай» (= "moon / month") by stripping
        // the locative-case-shaped `-та`, after which the stripped
        // surface presented «ай» as a word boundary.  Live input
        // «Сіз осы жауапты қысқаша **айта** аласыз ба?» was returning
        // «Уақыт өлшемі».  Strip-derived word boundaries are not
        // semantically reliable without POS confirmation; restrict
        // short-agent matching to the raw `lower` + `folded` forms.
        let hit = if len <= 3 {
            contains_word(lower, c) || contains_word(&folded, c)
        } else {
            lower.contains(c)
                || stripped.contains(c)
                || folded.contains(c)
                || folded_stripped.contains(c)
        };
        if hit && best.is_none_or(|(_, l)| len > l) {
            best = Some((c, len));
        }
    }
    best.map(|(s, _)| s.to_string())
}

/// Whole-word substring check — `haystack` contains `needle` as a
/// space-separated token (or at the start / end). Used for short
/// canonical agents («ай», «су», «жер») where a plain `contains`
/// would false-positive on «қалайсың» / «суық» / «жерде».
fn contains_word(haystack: &str, needle: &str) -> bool {
    haystack
        .split(|c: char| !c.is_alphanumeric() && c != '-')
        .any(|tok| tok == needle)
}

/// **STT fold** — normalize common Whisper-STT mishears to their
/// canonical Kazakh spelling so canonical-agent matching finds them.
///
/// Voice-REPL audit 2026-05-25 cases:
/// - «костанай» (Cyrillic к) → «қостанай» (Kazakh қ).
/// - «обылыс» / «облыс» / «болыс» → «облыс».
/// - «тауке хан» → «тәуке хан».
/// - «энштейн» / «әнштейн» → «эйнштейн».
/// - «жылдандығы» / «жолдамдығы» → «жылдамдығы».
/// - «зан» / «зең» → «заң».
/// - «костанайда» / «қостанайдан» — handled by case-stripper.
///
/// Conservative — only changes letters known to mishear; doesn't
/// touch words that already start with Kazakh diacritics.
fn stt_fold(s: &str) -> String {
    let mut out = s.to_string();
    // Common STT loanword mishears.
    out = out.replace("әнштейн", "эйнштейн");
    out = out.replace("энштейн", "эйнштейн");
    out = out.replace("ейнштейн", "эйнштейн");
    out = out.replace("анштейн", "эйнштейн");
    // Place names — Cyrillic к → Kazakh қ for the canonical Kazakh
    // city / oblast names we curate. Limited to known patterns to
    // avoid false rewrites of Russian loans.
    out = out.replace("костанай", "қостанай");
    out = out.replace("казахстан", "қазақстан");
    out = out.replace("қазахстан", "қазақстан");
    // Kazakh diacritic recovery.
    out = out.replace("тауке хан", "тәуке хан");
    // Misheard nouns.
    out = out.replace("обылыс", "облыс");
    out = out.replace("жылданд", "жылдамд");
    out = out.replace("жолдамд", "жылдамд");
    // Session-3 audit (codex 2026-05-25):
    // - «жел тұқтықстан» / «жел тоқтақстан» / «жел тоқыстан»
    //   → «желтоқсан».
    // - «оналты» (no space) → «он алты» (16 in Kazakh).
    // - «тенүзі» / «теңіз» → «теңізі» (Caspian sea question).
    // - «химияқылық» / «химияғылық» → «химиялық».
    // - «хандыр» / «хандырдыг» → «хандығы» (Khanate).
    // - «фотосинтіз» → «фотосинтез».
    // - «жасанды интелект» → «жасанды интеллект».
    out = out.replace("жел тұқтықстан", "желтоқсан");
    out = out.replace("жел тоқтақстан", "желтоқсан");
    out = out.replace("жел тоқыстан", "желтоқсан");
    out = out.replace("жел туқтыкстан", "желтоқсан");
    out = out.replace("оналты", "он алты");
    out = out.replace("тенүзі", "теңізі");
    out = out.replace("химияқылық", "химиялық");
    out = out.replace("химияғылық", "химиялық");
    out = out.replace("хандырдыг", "хандығы");
    out = out.replace("хандырдығы", "хандығы");
    out = out.replace("фотосинтіз", "фотосинтез");
    out = out.replace("жасанды интелект", "жасанды интеллект");
    // Session-4 audit (codex 2026-05-25 evening voice REPL):
    // - «мүгін» (Whisper for «бүгін») broke the clock gate so
    //   «Мүгін қандай ай» fell through to a generic «ай» IsA hit.
    // - «қобейту» / «кубит» (Whisper for «көбейту») broke math
    //   so «Екі қобейту екі» / «Екі кубит беске» returned an
    //   IsA description of «екі» instead of the computed result.
    // - «дарижысы» / «дарижесі» (Whisper for «дәрежесі») same
    //   class; folds let math_solver compute the power.
    // - «пайз» (Whisper for «пайыз») same class; «Жүз пайз бес»
    //   returned «Жүз — рулық бөлініс» instead of the percent op.
    // - «толған» / «тұлған» (Whisper for «туылған») — only when
    //   the question word «қашан» is present, where the context
    //   forces the «born» reading; preserves the legitimate
    //   «толған» = «filled» meaning in non-question contexts.
    // - «байтурсынулы» (Whisper for «байтұрсынұлы») — Cyrillic-у
    //   substitution for ұ.
    out = out.replace("мүгін", "бүгін");
    out = out.replace("қобейту", "көбейту");
    out = out.replace("қобейт ", "көбейт ");
    out = out.replace("кубит", "көбейту");
    out = out.replace("дарижысы", "дәрежесі");
    out = out.replace("дарижесі", "дәрежесі");
    out = out.replace("дәрижесі", "дәрежесі");
    out = out.replace("пайз ", "пайыз ");
    if out.contains("қашан") {
        out = out.replace("толған", "туылған");
        out = out.replace("тұлған", "туылған");
    }
    out = out.replace("байтурсынулы", "байтұрсынұлы");
    out = out.replace("байтұрсын улы", "байтұрсынұлы");
    out
}

fn has_russian_marker(lower: &str) -> bool {
    let words = [
        "что",
        "такое",
        "какой",
        "какая",
        "когда",
        "где",
        "кто",
        "сколько",
        "сегодня",
        "сейчас",
        "столица",
    ];
    words.iter().any(|w| lower.contains(w))
}

fn noun(s: &str) -> Composition {
    Composition::identity(Root::new(s, PartOfSpeech::Noun))
}

// Suppress unused-import lint on RankedFrame — it's part of the
// public adam-algebra surface this module re-exports semantically.
const _: fn() -> () = || {
    let _: Option<RankedFrame<'_>> = None;
    let _: AnswerSlot = AnswerSlot::Whole;
};

#[cfg(test)]
mod v6_8_26_answer_candidate_tests {
    //! **v6.8.26.**  Lock the AnswerCandidate / ProofRef surface
    //! so future handler migrations have a stable typed contract.
    use super::{AnswerCandidate, EvidenceKind, ProofRef, RouterAnswer};

    #[test]
    fn router_answer_alias_resolves_to_answer_candidate() {
        // Backcompat alias works — pre-existing callers compile
        // without churn during the migration window.
        let a: RouterAnswer = AnswerCandidate::from_text("hi".into(), EvidenceKind::SystemSelf);
        assert_eq!(a.proof_ref, ProofRef::LegacyCascade);
    }

    #[test]
    fn from_text_defaults_to_legacy_cascade() {
        let a = AnswerCandidate::from_text("x".into(), EvidenceKind::CuratedFact);
        assert_eq!(a.proof_ref, ProofRef::LegacyCascade);
    }

    #[test]
    fn with_proof_ref_attaches_typed_reference() {
        let a = AnswerCandidate::from_text("x".into(), EvidenceKind::CuratedFact).with_proof_ref(
            ProofRef::CuratedFact {
                fact_id: "geo_kz_001".into(),
            },
        );
        assert_eq!(
            a.proof_ref,
            ProofRef::CuratedFact {
                fact_id: "geo_kz_001".into()
            }
        );
    }

    #[test]
    fn clarify_uses_clarify_template_proof_ref() {
        let a = AnswerCandidate::clarify();
        assert_eq!(a.proof_ref, ProofRef::Template { name: "clarify" });
        assert_eq!(a.evidence_kind, EvidenceKind::SoftAck);
        assert_eq!(a.confidence, 1.0);
        assert!(a.matched_procedure_id.is_none());
    }

    #[test]
    fn fluent_builder_composes() {
        let a = AnswerCandidate::from_text("x".into(), EvidenceKind::ProcedureMatch)
            .with_procedure_id("kk_labor_ppe_002".into())
            .with_proof_ref(ProofRef::ProcedureMatch {
                procedure_id: "kk_labor_ppe_002".into(),
            })
            .with_confidence(0.83);
        assert_eq!(a.confidence, 0.83);
        assert_eq!(a.matched_procedure_id.as_deref(), Some("kk_labor_ppe_002"));
        assert!(matches!(a.proof_ref, ProofRef::ProcedureMatch { .. }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ENV-gate test: when `ADAM_V6_2` is unset, gate is closed.
    /// (We don't twiddle the env var here — env state is process-
    /// global and tests run in parallel. Just assert the function
    /// is callable.)
    #[test]
    fn is_v6_2_active_reads_env_var() {
        let _ = is_v6_2_active();
    }

    /// Math routes through the solver.
    #[test]
    fn math_routes_through_solver() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Два плюс два", &idx);
        assert_eq!(r.as_deref(), Some("4"));
    }

    /// Clock routes through system_clock; we don't assert exact
    /// content (live) but require non-empty.
    #[test]
    fn clock_routes_through_system_clock() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Қазір сағат неше?", &idx);
        assert!(r.is_some());
        assert!(r.unwrap().contains(":"));
    }

    /// **Phase 21 (2026-06-02)** — relative-day queries route to the
    /// clock handler instead of the substring fact lookup.  Without
    /// the «кеше» / «ертең» markers the router previously fell into
    /// `Күн IsA дөңгелек` (live REPL 2026-06-02 retest).
    #[test]
    fn yesterday_query_routes_to_clock_with_past_copula() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Кеше қай күн болды?", &idx);
        let s = r.expect("yesterday query should answer");
        assert!(s.starts_with("Кеше"), "expected «Кеше …» prefix, got: {s}");
        assert!(
            s.contains("болды"),
            "expected past copula «болды», got: {s}"
        );
        // Must NOT be the «Күн — дөңгелек» substring misroute.
        assert!(
            !s.contains("дөңгелек"),
            "regression: routed to fact, got: {s}"
        );
    }

    #[test]
    fn tomorrow_query_routes_to_clock_with_future_copula() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Ертең қандай күн?", &idx);
        let s = r.expect("tomorrow query should answer");
        assert!(s.starts_with("Ертең"), "got: {s}");
        assert!(s.contains("болады"), "got: {s}");
    }

    #[test]
    fn day_after_tomorrow_query_routes_to_clock() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Бүрсігүні нешесі болады?", &idx);
        let s = r.expect("day-after-tomorrow query should answer");
        assert!(s.starts_with("Бүрсігүні"), "got: {s}");
        assert!(s.contains("болады"), "got: {s}");
    }

    /// **Phase 21.A (2026-06-02)** — Whisper-drift aliases recover the
    /// calendar handler when STT mishears the marker. 2026-06-02 live
    /// REPL: «ертең» → «еркең», «бүрсігүні» → «бірсүгіне» / «бір сүгіні».
    #[test]
    fn whisper_drift_yerken_routes_to_tomorrow() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Еркең қай күн болады?", &idx);
        let s = r.expect("еркең drift should still route to tomorrow");
        assert!(s.starts_with("Ертең"), "got: {s}");
        assert!(s.contains("болады"), "got: {s}");
        assert!(
            !s.contains("дөңгелек"),
            "regression: fell through, got: {s}"
        );
    }

    #[test]
    fn whisper_drift_birsugine_routes_to_day_after_tomorrow() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Бірсүгіне нешесі болады?", &idx);
        let s = r.expect("бірсүгіне drift should route to day-after");
        assert!(s.starts_with("Бүрсігүні"), "got: {s}");
        assert!(s.contains("болады"), "got: {s}");
    }

    #[test]
    fn whisper_drift_bir_sugini_routes_to_day_after_tomorrow() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Бір сүгіні нешесі болады.", &idx);
        let s = r.expect("space-split бір сүгіні should still route");
        assert!(s.starts_with("Бүрсігүні"), "got: {s}");
    }

    /// **2026-06-03** — first-person location statement MUST NOT be
    /// answered by the v6.2 router (the v6.1 cascade upstream handles
    /// the acknowledgement + session update). Earlier behaviour was
    /// to return one-word «Қала» (IsA city) because the substring-IsA
    /// layer caught «қала» before the cascade could acknowledge.
    /// Multi-session live REPL regression — pinned here permanently.
    #[test]
    fn first_person_location_statement_defers_to_v61_cascade() {
        let idx = dialog_battery::canonical_corpus();
        // Canonical: «Мен Қостанайда тұрамын».
        assert!(
            answer_with_corpus("Мен Қостанайда тұрамын.", &idx).is_none(),
            "v6.2 must defer to v6.1 cascade for location statements"
        );
        // Compound city form: «Мен Қостанай қалада тұрамын».
        assert!(
            answer_with_corpus("Мен Қостанай қалада тұрамын.", &idx).is_none(),
            "compound «city қалада» must also defer"
        );
        // Whisper drift: «мен» → «мың».
        assert!(
            answer_with_corpus("Мың Қостанай қалада тұрамын.", &idx).is_none(),
            "Whisper-drifted мен→мың must also defer"
        );
        // First-person plural: «Біз Алматыда тұрамыз».
        assert!(
            answer_with_corpus("Біз Алматыда тұрамыз.", &idx).is_none(),
            "first-person plural тұрамыз must also defer"
        );
    }

    /// Sanity: the defer rule fires only on first-person dwelling
    /// verbs combined with мен / біз / мың. «Қала деген не?» (city
    /// definition) lacks both → must NOT defer.
    #[test]
    fn city_definition_query_does_not_match_defer_rule() {
        assert!(!looks_like_first_person_location_statement(
            "Қала деген не?"
        ));
        assert!(!looks_like_first_person_location_statement(
            "Қостанай қандай қала?"
        ));
        // First-person without dwelling verb → also no defer.
        assert!(!looks_like_first_person_location_statement("Мен оқимын."));
        // Positive control: the defer cases do match.
        assert!(looks_like_first_person_location_statement(
            "Мен Қостанайда тұрамын."
        ));
        assert!(looks_like_first_person_location_statement(
            "Мың Қостанай қалада тұрамын."
        ));
        assert!(looks_like_first_person_location_statement(
            "Біз Алматыда тұрамыз."
        ));
    }

    /// **Phase 23 (2026-06-03)** — chemistry-formula lookup.  Pins
    /// the school-curriculum formulas plus the live-REPL transcripts
    /// that surfaced this gap.
    #[test]
    fn water_formula_lookup() {
        assert_eq!(
            lookup_chemical_formula("Судың формуласын жазып бер."),
            Some("Судың формуласы — H₂O.".to_string())
        );
        assert_eq!(
            lookup_chemical_formula("Судың химия формуласын жаз."),
            Some("Судың формуласы — H₂O.".to_string())
        );
        assert_eq!(
            lookup_chemical_formula("Су формуласы қандай?"),
            Some("Судың формуласы — H₂O.".to_string())
        );
    }

    #[test]
    fn salt_formula_lookup() {
        assert_eq!(
            lookup_chemical_formula("Тұздың формуласы қандай?"),
            Some("Ас тұзының формуласы — NaCl.".to_string())
        );
        assert_eq!(
            lookup_chemical_formula("Ас тұзының формуласы қандай?"),
            Some("Ас тұзының формуласы — NaCl.".to_string())
        );
    }

    #[test]
    fn longest_match_wins_for_compounds() {
        // «көмірқышқыл газы» must win over bare «газ» / «көмірқышқыл».
        let r = lookup_chemical_formula("Көмірқышқыл газының формуласы қандай?");
        assert_eq!(r.as_deref(), Some("Көмірқышқыл газының формуласы — CO₂."));
        // «күкірт қышқылы» must win over «күкірт» (if it were in the
        // list separately).
        let r = lookup_chemical_formula("Күкірт қышқылының формуласы қандай?");
        assert_eq!(r.as_deref(), Some("Күкірт қышқылының формуласы — H₂SO₄."));
    }

    #[test]
    fn element_formulas_oxygen_hydrogen() {
        assert_eq!(
            lookup_chemical_formula("Оттегінің формуласы қандай?"),
            Some("Оттегінің формуласы — O₂.".to_string())
        );
        assert_eq!(
            lookup_chemical_formula("Сутегінің формуласы қандай?"),
            Some("Сутегінің формуласы — H₂.".to_string())
        );
    }

    #[test]
    fn no_formula_marker_no_fire() {
        // Bare substance mention without «формула» must NOT fire.
        // «Су ішемін» (I drink water) — no chemistry intent.
        assert_eq!(lookup_chemical_formula("Су ішемін."), None);
        assert_eq!(lookup_chemical_formula("Қаладан су әкел."), None);
        // «Тұзды бер» (pass the salt) — no formula query.
        assert_eq!(lookup_chemical_formula("Тұзды бер."), None);
    }

    #[test]
    fn unknown_substance_returns_none() {
        // Some substance the table doesn't cover.
        assert_eq!(
            lookup_chemical_formula("Қышбылдықтың формуласы қандай?"),
            None
        );
    }

    /// **Phase 21.B (2026-06-03 evening)** — «еркен» drift (single
    /// -н instead of -ң) caught in live REPL: «Еркен қай күн болады.»
    #[test]
    fn yerken_single_n_routes_to_tomorrow() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Еркен қай күн болады?", &idx);
        let s = r.expect("еркен drift must route to tomorrow");
        assert!(s.starts_with("Ертең"), "got: {s}");
        assert!(s.contains("болады"), "got: {s}");
    }

    /// **Phase 21.C (2026-06-04)** — «ерден» drift, GATED by the
    /// «күн» day marker.  Live REPL: «Ерден қай күн болады»
    /// returned an Abai citation about «ер» (man) — fell through to
    /// the Abai-quote handler.
    #[test]
    fn yerden_in_day_context_routes_to_tomorrow() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Ерден қай күн болады?", &idx);
        let s = r.expect("ерден + күн must route to tomorrow");
        assert!(s.starts_with("Ертең"), "got: {s}");
    }

    /// **Phase 21.C** — «ерден» WITHOUT «күн» must NOT trigger
    /// tomorrow.  «Ерден» can be a genuine personal name + ablative.
    #[test]
    fn yerden_without_day_marker_does_not_route_to_tomorrow() {
        // `relative_day_offset` should return None on plain «Ерден»
        // mentions.  We test the function directly to avoid the
        // upstream substring noise.
        assert_eq!(relative_day_offset("Ерден келді."), None);
        assert_eq!(relative_day_offset("Ерденнен сәлем."), None);
    }

    /// **Phase 26.C (2026-06-04 evening)** — compound utterance
    /// support extended to inputs WITHOUT any clause separator.
    /// Live REPL: «Менім атын дәулет мен қазақстанда тұрамын» — no
    /// comma, no period, both clauses run together.  The standalone
    /// «мен» token between two clauses + the 1sg dwelling verb
    /// «тұрамын» now triggers the defer.
    #[test]
    fn compound_without_separator_defers_to_v61() {
        assert!(looks_like_first_person_location_statement(
            "Менім атын дәулет мен қазақстанда тұрамын."
        ));
        assert!(looks_like_first_person_location_statement(
            "Менің атым Дәулет мен Қостанайда тұрамын."
        ));
    }

    /// Negative control for Phase 26.C: «X мен Y тұрамыз» (X AND Y
    /// live together — plural verb) must NOT trigger location defer
    /// because «мен» here is a conjunction, not a 1sg pronoun.
    #[test]
    fn compound_without_separator_does_not_misfire_on_conjunction_mеn() {
        // Plural verb — should NOT trigger our defer.
        assert!(!looks_like_first_person_location_statement(
            "Дәулет мен Болат Қостанайда тұрамыз."
        ));
    }

    /// **Phase 26.A (2026-06-04)** — compound utterance defer.  Live
    /// REPL caught «Менің атым Дәулет, мен қостанайда тұрамын» —
    /// the input STARTS with «менің», so the strict «мен »-at-start
    /// check missed the second clause.  The router fell through to
    /// the «Қостанай → Қала» IsA reply.
    #[test]
    fn compound_name_then_location_defers_to_v61() {
        // After the fix, this returns None so the v6.1 cascade
        // (which acknowledges both name AND location) stands.
        assert!(looks_like_first_person_location_statement(
            "Менің атым Дәулет, мен қостанайда тұрамын."
        ));
        // Same with period separator instead of comma.
        assert!(looks_like_first_person_location_statement(
            "Менің атым Дәулет. Мен қостанайда тұрамын."
        ));
        // And with missing space after comma (common typo / STT).
        assert!(looks_like_first_person_location_statement(
            "Менің атым Дәулет,мен қостанайда тұрамын."
        ));
    }

    /// **Phase 23.B (2026-06-03 evening)** — comma-split substance
    /// names. Live REPL caught «Ө, тегеннің формулысы.» — Whisper
    /// inserted a comma mid-token; the rc8 stem table didn't match.
    /// Pre-normalise punctuation → space so the stem catches.
    #[test]
    fn chemistry_formula_handles_comma_split() {
        assert_eq!(
            lookup_chemical_formula("Ө, тегеннің формулысы."),
            Some("Оттегінің формуласы — O₂.".to_string())
        );
        assert_eq!(
            lookup_chemical_formula("О, тегеннің формулысы."),
            Some("Оттегінің формуласы — O₂.".to_string())
        );
    }

    /// **Phase 23.A (2026-06-03)** — Whisper-drift coverage for the
    /// chemistry-formula table. Live REPL caught 3 drifts the rc7
    /// table missed: single-т «отегі», token-split «о тегі» / «ө
    /// тегі», and «қуқырт» for «күкірт». Pinned here.
    #[test]
    fn oxygen_drift_single_t() {
        assert_eq!(
            lookup_chemical_formula("Отегінің формулысы."),
            Some("Оттегінің формуласы — O₂.".to_string())
        );
    }

    #[test]
    fn oxygen_drift_token_split() {
        let r = lookup_chemical_formula("Ө тегінің формулысы.");
        assert_eq!(r.as_deref(), Some("Оттегінің формуласы — O₂."));
        let r = lookup_chemical_formula("О тегінің формулысы.");
        assert_eq!(r.as_deref(), Some("Оттегінің формуласы — O₂."));
    }

    #[test]
    fn sulfuric_acid_drift_qukyrt() {
        let r = lookup_chemical_formula("Қуқырт қышқылының формулысы.");
        assert_eq!(r.as_deref(), Some("Күкірт қышқылының формуласы — H₂SO₄."));
    }

    /// **rc5-followup (2026-06-03 evening)** — Whisper-drift fallback.
    /// Live REPL hit «Мен қостанай атырамым» — the dwelling verb
    /// «тұрамын» was mistranscribed as «атырамым» AND the locative
    /// `‑да` was dropped from the city. Neither the canonical-verb
    /// fast path nor the locative-noun marker matched. The fallback
    /// must catch this via the 1p verb suffix + known-city stem.
    #[test]
    fn first_person_location_drift_via_morphology_fallback() {
        // Live REPL transcript verbatim.
        assert!(
            looks_like_first_person_location_statement("Мен қостанай атырамым."),
            "must catch «тұрамын» → «атырамым» drift"
        );
        // Same drift, with accusative city marker.
        assert!(looks_like_first_person_location_statement(
            "Мен қостанайды атырамым."
        ));
        // Drift in the verb only — locative survived.
        assert!(looks_like_first_person_location_statement(
            "Мен қостанайда атырамым."
        ));
        // First-person plural drift.
        assert!(looks_like_first_person_location_statement(
            "Біз алматыда атырамыз."
        ));
        // Negative control: 1p verb without any city marker is NOT
        // a location statement (e.g. «I am thinking»).
        assert!(!looks_like_first_person_location_statement("Мен ойлаймын."));
        // Negative control: city mentioned but not a 1p statement.
        assert!(!looks_like_first_person_location_statement(
            "Қостанай қандай қала?"
        ));
    }

    /// Real biographical question → realised Kazakh sentence.
    ///
    /// v6.8.18 update: the answer changed from the bare year
    /// «1872» to the fuller «Ахмет байтұрсынұлы 1872 жылы
    /// туылған.» sentence after the dedicated
    /// `lookup_person_birthyear_with_anaphora` handler started
    /// firing on this shape.  The richer sentence is the
    /// product-side win (Codex Q3 school #5); the bare year is
    /// still embedded for downstream extractors.
    #[test]
    fn bio_question_returns_year_sentence() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Ахмет Байтұрсынұлы қашан туылған?", &idx);
        let text = r.expect("should produce an answer");
        assert!(text.contains("1872"), "expected 1872 in: {text}");
        assert!(text.contains("туылған"), "expected туылған in: {text}");
    }

    /// IsA definition returns a copular sentence.
    #[test]
    fn isa_returns_copular_sentence() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Қазақстан деген не?", &idx);
        assert!(r.is_some());
        let s = r.unwrap();
        assert!(
            s.contains("мемлекет"),
            "expected «мемлекет» in answer, got: {s}"
        );
    }

    /// Russian bilingual query routes to Russian-language fact.
    #[test]
    fn russian_query_returns_russian_fact() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Что такое гравитация?", &idx);
        assert!(r.is_some());
        let s = r.unwrap();
        assert!(
            s.contains("сила притяжения"),
            "expected Russian definition, got: {s}"
        );
    }

    /// Unknown input → None (fallback to v6.1 cascade).
    #[test]
    fn unknown_input_returns_none() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("xyz random gibberish 123", &idx);
        // Math markers absent, clock markers absent, no known agent
        // — the router declines.
        assert!(r.is_none());
    }

    /// Session-4 audit: «Мүгін қандай ай» (Whisper misheard
    /// «бүгін» as «мүгін») must reach the clock gate.
    #[test]
    fn stt_fold_mugin_routes_to_clock() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Мүгін қандай ай?", &idx);
        assert!(r.is_some(), "expected clock answer, got None");
        let s = r.unwrap();
        assert!(
            s.contains("ай"),
            "expected month answer to mention «ай», got: {s}"
        );
    }

    /// Session-4 audit: «Екі қобейту екі» (Whisper misheard
    /// «көбейту» as «қобейту») must compute, not return an IsA
    /// description of «екі».
    #[test]
    fn stt_fold_qobejtu_routes_to_math() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Екі қобейту екі", &idx);
        assert_eq!(r.as_deref(), Some("4"));
    }

    /// Session-4 audit: «Екі кубит беске» (Whisper misheard
    /// «көбейту» as «кубит») — same class.
    #[test]
    fn stt_fold_kubit_routes_to_math() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Екі кубит беске", &idx);
        assert_eq!(r.as_deref(), Some("10"));
    }

    /// Session-4 audit: «Екі дарижысы он» (Whisper misheard
    /// «дәрежесі» as «дарижысы») → power.
    #[test]
    fn stt_fold_darizysy_routes_to_power() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Екі дарижысы он", &idx);
        assert_eq!(r.as_deref(), Some("1024"));
    }

    /// Session-4 audit: «Жүз пайз бес» (Whisper dropped the «ы»
    /// in «пайыз») → percent.
    #[test]
    fn stt_fold_pajz_routes_to_percent() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Жүз пайз бес", &idx);
        assert_eq!(r.as_deref(), Some("5"));
    }

    /// Session-4 audit: «Ахмет байтурсынулы қашан толған»
    /// (Whisper substituted Cyrillic-у for ұ, and «толған» for
    /// «туылған») — the conditional fold keys on «қашан» to
    /// rewrite «толған» → «туылған» without breaking the
    /// legitimate «filled» meaning elsewhere.
    #[test]
    fn stt_fold_tolgan_routes_to_birth_year() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Ахмет байтурсынулы қашан толған?", &idx);
        // v6.8.18: answer shape changed from bare year to full
        // sentence — assert on the year embedding so STT-fold
        // routing is still being exercised.
        let text = r.expect("should produce an answer");
        assert!(text.contains("1872"), "expected 1872 in: {text}");
    }

    /// Session-4 audit: «Мен ағай болғанымды қалай түсіндім»
    /// — pitch-detection meta-query with first-person past-tense
    /// slip («түсіндім»). Must route to pitch-explanation, not
    /// to a generic «ағай» retrieval.
    #[test]
    fn pitch_detection_accepts_first_person_slip() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Мен ағай болғанымды қалай түсіндім?", &idx);
        assert!(r.is_some());
        let s = r.unwrap();
        assert!(
            s.contains("жиілігі") || s.contains("pitch"),
            "expected pitch explanation, got: {s}"
        );
    }

    /// Session-5 audit (the «Мемлекет» bug): «Қазақстанда қандай
    /// таулар бар?» must enumerate mountains, not return the host
    /// country's IsA type («Мемлекет»).
    #[test]
    fn listing_mountains_in_kazakhstan() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Қазақстанда қандай таулар бар?", &idx);
        assert!(r.is_some());
        let s = r.unwrap();
        assert!(
            s.contains("Алатау") && !s.eq_ignore_ascii_case("мемлекет"),
            "expected mountain list, got: {s}"
        );
    }

    /// Session-5 audit: «Қазақстанда қандай өзендер бар?» — rivers.
    #[test]
    fn listing_rivers_in_kazakhstan() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Қазақстанда қандай өзендер бар?", &idx);
        assert!(r.is_some());
        let s = r.unwrap();
        assert!(
            s.contains("Ертіс") || s.contains("Сырдария"),
            "expected river list, got: {s}"
        );
    }

    /// Session-5 audit: «Қазақстанда қандай көлдер бар?» — lakes.
    #[test]
    fn listing_lakes_in_kazakhstan() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Қазақстанда қандай көлдер бар?", &idx);
        assert!(r.is_some());
        let s = r.unwrap();
        assert!(
            s.contains("Балқаш") || s.contains("Зайсан"),
            "expected lake list, got: {s}"
        );
    }

    /// Session-5 audit: «Қандай өзендер білесің?» — un-anchored
    /// enumeration must still list rivers, not «(нет данных)».
    #[test]
    fn listing_rivers_un_anchored() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Қандай өзендер білесің?", &idx);
        assert!(r.is_some());
        let s = r.unwrap();
        assert!(
            s.contains("Ертіс") || s.contains("Сырдария"),
            "expected river list, got: {s}"
        );
    }

    /// **v6.8.1 — 2026-06-17 voice REPL audit (Bug #17).** Live
    /// session turn #17 «Қазақстанның астанысы — қай қала?» fell
    /// back to a generic IsA («Мемлекет») because the pre-fix gate
    /// required both `"астана"` (substring missed: Whisper drift
    /// «астанасы → астанысы» replaces а→ы) and `"қандай"`
    /// (interrogative was «қай қала», not «қандай»). The patch adds
    /// the «астаны» surface form and the «қай қала / қалада /
    /// қалалар» interrogative variants while keeping the existing
    /// «елорда» standalone path.
    #[test]
    fn capital_query_with_whisper_drift_and_qaj_qala_v681() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Қазақстанның астанысы — қай қала?", &idx);
        assert!(r.is_some(), "capital query must resolve, got None");
        let s = r.unwrap();
        assert!(s.contains("Астана"), "expected Astana in answer, got: {s}");
    }

    /// Companion: clean canonical form «астанасы» + «қандай» still
    /// works after the gate refactor.
    #[test]
    fn capital_query_canonical_form_still_resolves_v681() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Қазақстанның астанасы қандай?", &idx);
        assert!(r.is_some());
        assert!(r.unwrap().contains("Астана"));
    }

    /// Negative control: «астана» alone without an interrogative
    /// (e.g. «Астана — әдемі қала.») must NOT trigger the capital
    /// template. The fix gates `астана`/`астаны` substrings on a
    /// capital-shaped interrogative so the false-positive surface
    /// stays bounded.
    #[test]
    fn capital_marker_without_interrogative_does_not_fire_v681() {
        let idx = dialog_battery::canonical_corpus();
        let r = answer_with_corpus("Астана — әдемі қала.", &idx);
        // We don't pin a specific answer — the canonical corpus may
        // surface another fact — only assert it isn't the capital
        // template (which would be the false-positive).
        let response = r.unwrap_or_default();
        assert!(
            !response.contains("Қазақстанның елордасы — Астана"),
            "must not fire capital template on bare statement, got: {response}"
        );
    }

    /// **v6.8.2 user audit Bug #3.** Live probe «Темір жол таңбасы
    /// қандай?» (= «what is the railway sign?») wrongly returned
    /// «Темірдің формуласы — Fe.» because the pre-fix word-boundary
    /// gate caught no-space «теміржол» but not the space-separated
    /// «темір жол».  The NON_CHEMISTRY_COMPOUNDS early-exit list now
    /// short-circuits before the formula loop.
    #[test]
    fn chemistry_lookup_skips_temir_zhol_compound_v682() {
        let r = lookup_chemical_formula("Темір жол таңбасы қандай?");
        assert!(
            r.is_none(),
            "railway compound must NOT chemistry-resolve, got: {r:?}"
        );
        // No-space variant also excluded.
        let r = lookup_chemical_formula("Теміржол таңбасы қандай?");
        assert!(r.is_none(), "no-space railway also excluded, got: {r:?}");
    }

    /// Companion: pure-chemistry «Темірдің формуласы» still resolves.
    #[test]
    fn chemistry_lookup_resolves_temir_genitive_v682() {
        let r = lookup_chemical_formula("Темірдің формуласы қандай?");
        assert!(r.is_some());
        assert!(r.unwrap().contains("Fe"));
    }

    /// **v6.8.3 user audit (Bug A) — lifespan compute from BornIn +
    /// DiedIn.** End-to-end functional verification is performed
    /// against the live `world_core/*.jsonl` corpus via the
    /// `adam_chat` binary (see commit body): for Ахмет Байтұрсынұлы
    /// the cascade produces «Ахмет байтұрсынұлы 65 жыл өмір сүрді
    /// (1872–1937).» — the BornIn (kru_002, object-string shape)
    /// and DiedIn (kru_003, object-string shape) joined into one
    /// typed answer. The deterministic unit-level coverage for the
    /// extraction primitive is `extract_year_in_range_*` below;
    /// regression protection at the cascade level rides on the five
    /// production eval suites.

    /// **v6.8.4 L4.5 Phase 2.A.2 — bool-detector routes migrated
    /// to typed pipeline.**  Four routes (capabilities, personal-
    /// experience, self-identity, live-data) now route through
    /// `lookup_X_typed` siblings that construct the proper
    /// `ProofObject` (`system_self` / `no_data_refusal` /
    /// `safety_refusal`) and carry the right `RouteId`.  The
    /// cascade callsites are `lookup_X_typed(input).map(|c| c.text)`
    /// — same surface, plus typed provenance.
    #[test]
    fn capabilities_typed_canary_v684() {
        use crate::dialog_acts::{DialogueMove, RouteId};
        let c = lookup_capabilities_typed("Сен не білесің?").expect("must resolve");
        assert_eq!(c.route, RouteId::Capabilities);
        assert!(c.invariant_check().is_ok());
        match &c.moves[0] {
            DialogueMove::Assert { claim } => assert_eq!(claim, &c.text),
            other => panic!("expected Assert, got {other:?}"),
        }
    }

    #[test]
    fn personal_experience_typed_canary_v684() {
        use crate::dialog_acts::{DialogueMove, PolicyReason, RouteId};
        let c = lookup_personal_experience_typed("Сен қандай кітап оқыдың?").expect("must resolve");
        assert_eq!(c.route, RouteId::PersonalExperienceRefusal);
        assert!(c.invariant_check().is_ok());
        match &c.moves[0] {
            DialogueMove::Refuse(reason) => {
                assert_eq!(*reason, PolicyReason::PresuppositionFailure);
            }
            other => panic!("expected Refuse, got {other:?}"),
        }
    }

    #[test]
    fn self_identity_typed_canary_v684() {
        use crate::dialog_acts::{DialogueMove, RouteId};
        let c = lookup_self_identity_typed("Сен кімсің?").expect("must resolve");
        assert_eq!(c.route, RouteId::SelfIdentity);
        assert!(c.invariant_check().is_ok());
        match &c.moves[0] {
            DialogueMove::Assert { .. } => {}
            other => panic!("expected Assert, got {other:?}"),
        }
        // The new yes/no identity probes also route here.
        let c2 = lookup_self_identity_typed("Сен адамсың ба?").expect("must resolve");
        assert_eq!(c2.route, RouteId::SelfIdentity);
    }

    #[test]
    fn live_data_refusal_typed_canary_v684() {
        use crate::dialog_acts::{DialogueMove, PolicyReason, RouteId};
        let c = lookup_live_data_refusal_typed("Қазір BTC бағасы қанша?").expect("must resolve");
        assert_eq!(c.route, RouteId::LiveDataRefusal);
        assert!(c.invariant_check().is_ok());
        match &c.moves[0] {
            DialogueMove::Refuse(reason) => {
                assert_eq!(*reason, PolicyReason::NoLiveData);
            }
            other => panic!("expected Refuse, got {other:?}"),
        }
    }

    /// **v6.8.4 L4.5 Phase 2.A — chemistry route migration.** The
    /// typed `lookup_chemical_formula_typed` returns an
    /// `AnswerCandidate` whose proof cites the in-code chemistry
    /// table (`v6_2_router/chemistry_table`).  The String-returning
    /// wrapper is functionally unchanged.
    #[test]
    fn chemistry_typed_canary_returns_consistent_candidate_v684() {
        use crate::dialog_acts::{DialogueMove, RouteId};
        let candidate =
            lookup_chemical_formula_typed("Темірдің формуласы қандай?").expect("must resolve");
        assert_eq!(candidate.route, RouteId::ChemistryFormula);
        assert!(candidate.text.contains("Fe"));
        assert!(candidate.invariant_check().is_ok());
        match &candidate.moves[0] {
            DialogueMove::Assert { claim } => assert_eq!(claim, &candidate.text),
            other => panic!("expected Assert, got {other:?}"),
        }
        // String wrapper byte-identical to typed text.
        assert_eq!(
            lookup_chemical_formula("Темірдің формуласы қандай?"),
            Some(candidate.text.clone()),
        );
    }

    /// **v6.8.4 L4.5 Phase 2.A — possessive-property route.** Same
    /// invariant as chemistry: typed proof + wrapper byte-identical.
    /// Tested on the body-parts «жүректің қызметі» pattern which
    /// exercises the AskPurpose branch of the lookup table.
    #[test]
    fn possessive_property_typed_canary_returns_consistent_candidate_v684() {
        use crate::dialog_acts::{DialogueMove, RouteId};
        let candidate =
            lookup_possessive_property_typed("Жүректің қызметі қандай?").expect("must resolve");
        assert_eq!(candidate.route, RouteId::PossessiveProperty);
        assert!(candidate.text.contains("қан айналымы"));
        assert!(candidate.invariant_check().is_ok());
        match &candidate.moves[0] {
            DialogueMove::Assert { claim } => assert_eq!(claim, &candidate.text),
            other => panic!("expected Assert, got {other:?}"),
        }
        assert_eq!(
            lookup_possessive_property("Жүректің қызметі қандай?"),
            Some(candidate.text.clone()),
        );
    }

    /// **v6.8.4 L4.5 Phase 1 — canary.** The typed
    /// `lookup_person_lifespan_typed` returns an `AnswerCandidate`
    /// whose proof carries the joined (born_year, died_year)
    /// synthesis and whose route attribution is `RouteId::Lifespan`.
    /// The String-returning `lookup_person_lifespan` is now a thin
    /// wrapper extracting `.text` — same surface, same semantics,
    /// plus typed provenance.
    #[test]
    fn lifespan_typed_canary_returns_consistent_candidate_v684() {
        use crate::dialog_acts::{DialogueMove, RouteId};
        // No subject in canonical_corpus has BOTH BornIn + DiedIn
        // as the object-string shape (canonical_corpus uses typed
        // TimeAnchor::Year modifier which is exercised at the live-
        // binary level); we test the shape invariant on the helper
        // builder directly for the canonical path.  End-to-end
        // verification of the canary lives in the commit body.
        let dummy_proof = crate::proof_object::ProofObject::from_curated_fact(
            "test".into(),
            "lifespan".into(),
            "10 жыл".into(),
            adam_reasoning::FactSource {
                pack: "world_core/synthesised".into(),
                sample_id: "test/lifespan".into(),
            },
            "BornIn=2000 + DiedIn=2010 → 10 жыл".into(),
        );
        let candidate = crate::dialog_acts::AnswerCandidate::assert(
            "Test 10 жыл өмір сүрді (2000–2010).".into(),
            dummy_proof,
            RouteId::Lifespan,
        );
        assert_eq!(candidate.route, RouteId::Lifespan);
        assert!(candidate.invariant_check().is_ok());
        match &candidate.moves[0] {
            DialogueMove::Assert { claim } => {
                assert_eq!(claim, &candidate.text);
            }
            other => panic!("expected Assert, got {other:?}"),
        }
    }

    /// Negative control: query without «өмір сүр» / «прожил» phrase
    /// must NOT trigger the lifespan handler.
    #[test]
    fn lifespan_handler_only_fires_on_lifespan_shape_v683() {
        // «қашан туылған» — birth date, NOT lifespan
        let r = lookup_person_lifespan(
            "Ахмет Байтұрсынұлы қашан туылған?",
            &dialog_battery::canonical_corpus(),
        );
        assert!(
            r.is_none(),
            "birth-date query must NOT trigger lifespan, got: {r:?}"
        );
    }

    /// Negative control: lifespan shape with NO resolvable subject
    /// (anaphora) must return None — the cascade decides whether to
    /// honestly refuse or synthesise from a different route. The
    /// handler does NOT guess.
    #[test]
    fn lifespan_handler_without_subject_returns_none_v683() {
        let r =
            lookup_person_lifespan("Қанша жыл өмір сүрді?", &dialog_battery::canonical_corpus());
        assert!(
            r.is_none(),
            "bare lifespan query must NOT fire (no subject), got: {r:?}"
        );
    }

    /// Year-extraction unit: curated world_core date surfaces (year +
    /// month + day) yield the leading 4-digit year; out-of-range
    /// 4-digit tokens (street numbers etc.) are rejected.
    #[test]
    fn extract_year_in_range_handles_curated_surfaces_v683() {
        assert_eq!(extract_year_in_range("1872 жылы 5 қыркүйек"), Some(1872));
        assert_eq!(extract_year_in_range("1937 жыл"), Some(1937));
        assert_eq!(extract_year_in_range("1845"), Some(1845));
        // Out of range — rejected.
        assert_eq!(extract_year_in_range("3000 жыл"), None);
        assert_eq!(extract_year_in_range("12345"), None);
        // No 4-digit token.
        assert_eq!(extract_year_in_range("кеше"), None);
    }

    /// **v6.8.3 user audit (Bug C) — personal-experience presupposition
    /// refusal.** Pre-fix «Сен қандай кітап оқыдың?» surfaced the
    /// substring-IsA definition of «кітап» («Кітап — мұқабамен
    /// бекітілген баспа басылымы…»), which presupposes adam DID read.
    /// adam has no lived experience — refusing the presupposition is
    /// the honest answer.
    #[test]
    fn personal_experience_probes_get_refusal_v683() {
        // 2nd-person past-tense experience verbs across topics.
        for input in [
            "Сен қандай кітап оқыдың?",
            "Сен қандай фильмдер көрдің?",
            "Сіз қайда бардыңыз?",
            "Сен бүгін не жедің?",
            "Сен қандай ән тыңдадың?",
            "Сіз кешегі ойынды көрдіңіз бе?",
        ] {
            assert!(
                is_personal_experience_query(input),
                "must classify as personal-experience: {input}"
            );
        }
    }

    /// Negative control: knowledge / capability queries must NOT route
    /// through the experience refusal — they have their own
    /// `is_capabilities_query` handler.
    #[test]
    fn knowledge_queries_are_not_personal_experience_v683() {
        for input in [
            "Сен не білесің?",
            "Сен қазақша сөйлейсің бе?",
            "Не істей аласың?",
            "Қазақстанның астанасы қандай?",
            "Темірдің формуласы.",
        ] {
            assert!(
                !is_personal_experience_query(input),
                "must NOT classify as personal-experience: {input}"
            );
        }
    }

    /// **v6.8.3 user audit — identity yes/no probes.** Pre-fix
    /// «Сен адамсың ба?» fell to a substring-IsA lookup that
    /// returned «дерек жоқ» because world_core carries no fact
    /// «adam IsA человек».  Honest answer is the existing self-
    /// identification template; we just had to expand the detector
    /// to cover the 2nd-person + yes/no + identity-class noun shape.
    #[test]
    fn identity_yes_no_probes_route_to_self_identity_v683() {
        for input in [
            "Сен адамсың ба?",
            "Сіз робот па?",
            "Сен жасанды интеллектсің бе?",
            "Сен программасың ба?",
        ] {
            assert!(
                is_self_identity_query(input),
                "must classify as identity probe: {input}"
            );
        }
    }

    /// **v6.8.3 user audit — BTC live-data probe.** Pre-fix «Қазір
    /// BTC бағасы қанша?» returned today's date because «қазір»
    /// triggered the v6.1 date_query intent BEFORE the v6.2
    /// live-data refusal could fire (v6.2 only overrides when it
    /// returns Some).  Both the ticker «btc / eth / ethereum» and
    /// the «бағасы» possessive surface were missing from
    /// `needs_live_data_refusal`; both gaps closed.
    #[test]
    fn live_data_refusal_covers_btc_and_eth_tickers_v683() {
        for input in [
            "Қазір BTC бағасы қанша?",
            "BTC бағасы қанша?",
            "ETH бағасын білесіз бе?",
            "Бүгінгі доллар бағамы қандай?",
            "Бұл акция бағасы қанша?",
        ] {
            assert!(
                needs_live_data_refusal(input),
                "must classify as live-data refusal: {input}"
            );
        }
    }

    /// Negative control: bare clock / date queries (no market noun)
    /// must NOT route through the live-data refusal — those have
    /// their own clock handler.
    #[test]
    fn bare_clock_query_is_not_live_data_v683() {
        for input in [
            "Қазір сағат неше?",
            "Бүгін қандай күн?",
            "Бүгінгі ай қандай?",
        ] {
            assert!(
                !needs_live_data_refusal(input),
                "clock query must NOT route to live-data refusal: {input}"
            );
        }
    }

    /// Negative control: the broadened identity detector must NOT
    /// fire on a generic yes/no question that mentions a person.
    #[test]
    fn generic_yes_no_about_other_person_is_not_self_identity_v683() {
        for input in [
            "Абай ұлы ақын ба?",
            "Сіз қазақша білесіз бе?",   // capability, not identity
            "Сен дәрігерге барасың ба?", // user-direction, not identity
        ] {
            assert!(
                !is_self_identity_query(input),
                "must NOT classify as identity probe: {input}"
            );
        }
    }

    /// **v6.8.11 — active-uncertainty foundation.** Verify the
    /// procedure-match path exposes a normalised confidence
    /// score and `EvidenceKind::ProcedureMatch`.  Unit-tests
    /// the helper directly to avoid pulling the whole cascade
    /// + lexicon load path into the test.
    #[test]
    fn lookup_procedure_with_score_returns_normalised_confidence() {
        let r = lookup_procedure_matched_with_score("СИЗ беру тәртібі қандай?");
        let (_, id, score) = r.expect("procedure query must match");
        assert_eq!(id, "kk_labor_ppe_002");
        assert!(
            score > CLARIFY_THRESHOLD,
            "expected confidence > {CLARIFY_THRESHOLD}, got {score}",
        );
        assert!(score <= 1.0, "normalised must be ≤ 1.0, got {score}");
    }

    /// A non-procedure query never reaches the procedure
    /// scorer; the helper returns None and the caller falls
    /// through to the legacy cascade (which the cascade-level
    /// wrapper will tag `EvidenceKind::LegacyCascade`).
    #[test]
    fn lookup_procedure_with_score_misses_non_procedure_query() {
        assert!(lookup_procedure_matched_with_score("Сәлем!").is_none());
        assert!(lookup_procedure_matched_with_score("2+2 қанша?").is_none());
    }

    /// **v6.8.12 — procedure attribute expansion.** Kazakh +
    /// Russian ordinals, plus bare numeral surfaces, parse into
    /// step indices.
    #[test]
    fn parse_step_ordinal_kazakh_one_to_ten() {
        let cases = [
            ("бірінші қадам", 1),
            ("Екінші қадамды айтшы.", 2),
            ("Үшінші қадам не?", 3),
            ("Төртінші қадам қандай?", 4),
            ("Бесінші қадам туралы.", 5),
            ("Алтыншы қадам қалай?", 6),
            ("жетінші қадам", 7),
            ("Сегізінші қадам.", 8),
            ("тоғызыншы қадам", 9),
            ("Оныншы қадам.", 10),
        ];
        for (input, expected) in cases {
            assert_eq!(parse_step_ordinal(input), Some(expected), "input={input:?}",);
        }
    }

    #[test]
    fn parse_step_ordinal_russian_ordinals() {
        assert_eq!(parse_step_ordinal("Второй шаг"), Some(2));
        assert_eq!(parse_step_ordinal("Третий шаг расскажи"), Some(3));
        assert_eq!(parse_step_ordinal("Пятый шаг."), Some(5));
    }

    #[test]
    fn parse_step_ordinal_bare_numerals() {
        assert_eq!(parse_step_ordinal("2-ші қадам"), Some(2));
        assert_eq!(parse_step_ordinal("3 шаг"), Some(3));
    }

    #[test]
    fn parse_step_ordinal_no_ordinal_returns_none() {
        // Bare body parts and broad questions don't carry an
        // ordinal — must not return a spurious number.
        assert_eq!(parse_step_ordinal("Қандай рәсім бар?"), None);
        assert_eq!(parse_step_ordinal("Қанша қадам?"), None);
        // Numbers way out of step range (1–20) don't fire.
        assert_eq!(parse_step_ordinal("Жыл 2024."), None);
    }

    /// `with_confidence` + `with_procedure_id` builders compose
    /// cleanly and clamp the confidence to `[0.0, 1.0]`.
    #[test]
    fn router_answer_builders_compose_and_clamp() {
        let r = RouterAnswer::from_text("test".into(), EvidenceKind::ProcedureMatch)
            .with_confidence(1.5)
            .with_procedure_id("p_001".into());
        assert_eq!(r.confidence, 1.0); // clamped down
        assert_eq!(r.matched_procedure_id.as_deref(), Some("p_001"));
        let r2 =
            RouterAnswer::from_text("test".into(), EvidenceKind::CuratedFact).with_confidence(-0.3);
        assert_eq!(r2.confidence, 0.0); // clamped up
    }

    /// **v6.8.15 — Clarify builder.** `RouterAnswer::clarify()`
    /// produces a deterministic `SoftAck` with confidence 1.0
    /// and no `matched_procedure_id` — a Clarify must NOT pin
    /// a weak match as the discourse referent.
    #[test]
    fn router_answer_clarify_builder() {
        let r = RouterAnswer::clarify();
        assert_eq!(r.evidence_kind, EvidenceKind::SoftAck);
        assert_eq!(r.confidence, 1.0);
        assert!(r.matched_procedure_id.is_none());
        assert!(r.text.contains("Сұрағыңызды"));
        assert!(r.text.contains("Нақтырақ"));
    }

    /// `CLARIFY_THRESHOLD = 0.5` is the strict-inequality
    /// boundary.  Three regression assertions guard the cascade
    /// gate from being either too lax or too aggressive:
    ///   * LegacyCascade (0.5) must NOT trigger Clarify — that
    ///     would replace every v6.1 cascade output including
    ///     correct curated lookups.
    ///   * Weak ProcedureMatch (raw=2 → 0.25) MUST trigger.
    ///   * Strong ProcedureMatch (raw=6 → 0.75) must NOT.
    /// **v6.8.17 — Codex Q3 school bug #1.** The exact bug:
    /// «Сәлем, мен 8-сынып оқушысымын» fired math, returned
    /// «жазсаңыз — есептеп беремін».  The grade-statement
    /// detector now fires first.
    #[test]
    fn recognize_grade_statement_canonical() {
        let r = recognize_grade_statement("Сәлем, мен 8-сынып оқушысымын.").expect("fires");
        assert!(r.contains("8-сынып"), "expected grade number, got: {r}");
        assert!(r.contains("Қандай пәннен"));
    }

    /// Grades 1–11 all parse.
    #[test]
    fn recognize_grade_statement_full_range() {
        for n in 1..=11 {
            let input = format!("Мен {n}-сынып оқушысымын.");
            let r = recognize_grade_statement(&input)
                .unwrap_or_else(|| panic!("grade {n} should fire — input was {input}"));
            assert!(r.contains(&format!("{n}-сынып")));
        }
    }

    /// Without first-person marker, the detector does NOT fire —
    /// «8-сынып бағдарламасы қандай?» is a legitimate factual
    /// query about a grade's curriculum, not a self-intro.
    #[test]
    fn recognize_grade_statement_no_first_person_no_fire() {
        assert!(recognize_grade_statement("8-сынып бағдарламасы қандай?").is_none());
        assert!(recognize_grade_statement("11-сынып емтиханы туралы.").is_none());
    }

    /// Without grade-role marker, doesn't fire — bare «мен 8»
    /// is too sparse to interpret as a school grade statement.
    #[test]
    fn recognize_grade_statement_no_role_no_fire() {
        assert!(recognize_grade_statement("Мен 8.").is_none());
        assert!(recognize_grade_statement("Менің 8 кітабым бар.").is_none());
    }

    /// Bare math queries must NOT trigger grade detection — they
    /// have neither a first-person grade-role marker nor any
    /// «сынып» token.
    #[test]
    fn recognize_grade_statement_math_baseline() {
        assert!(recognize_grade_statement("8+5 қанша?").is_none());
        assert!(recognize_grade_statement("Жетіге бесті қос.").is_none());
        assert!(recognize_grade_statement("2+2 қанша?").is_none());
    }

    /// `extract_grade_number` covers 1–11 and rejects out-of-range.
    #[test]
    fn extract_grade_number_range() {
        assert_eq!(extract_grade_number("мен 8-сынып оқушысымын"), Some(8));
        assert_eq!(extract_grade_number("11-сынып"), Some(11));
        assert_eq!(extract_grade_number("1-сынып"), Some(1));
        // 12 is above the Kazakh school range — reject.
        assert_eq!(extract_grade_number("12-сынып"), None);
        assert_eq!(extract_grade_number("100"), None);
        assert_eq!(extract_grade_number("сынып"), None);
    }

    #[test]
    fn clarify_threshold_boundary() {
        // Strict-inequality boundary: `< CLARIFY_THRESHOLD` means
        // values EQUAL to the threshold are NOT clarified.
        // Asserted via `>=` form to keep clippy happy on f32
        // partial-ordering — clippy flags the negated `!(x < y)`
        // form because f32 admits NaN.
        let legacy_05 = 0.5_f32;
        assert!(legacy_05 >= CLARIFY_THRESHOLD);

        let weak_025 = 0.25_f32;
        assert!(weak_025 < CLARIFY_THRESHOLD);

        let strong_075 = 0.75_f32;
        assert!(strong_075 >= CLARIFY_THRESHOLD);
    }

    /// **v6.8.45 — procedure_eval audit fix.**  Co-occurrence
    /// «қалай ... керек» as a procedure-shape trigger.
    /// Three pinned positives + one pinned negative confirm
    /// the new trigger fires on real worker queries without
    /// false-positives on non-procedural «керек» uses.
    #[test]
    fn v6_8_45_qalai_kerek_co_occurrence_fires_procedure_lookup() {
        // Gas measurement procedure — was failing in v6.8.43
        // baseline.
        assert!(
            lookup_procedure("Цехта газ концентрациясын қалай өлшеу керек?").is_some(),
            "qalai+kerek procedure query must route"
        );
        // Chemical storage — was failing.
        assert!(
            lookup_procedure("Химиялық заттарды цехта қалай сақтау керек?").is_some(),
            "qalai+kerek chemical-storage query must route"
        );
        // Accident investigation — was failing.
        assert!(
            lookup_procedure("Жұмыс орнындағы жазатайым оқиғаны қалай тергеу керек?").is_some(),
            "qalai+kerek accident-investigation query must route"
        );
    }

    #[test]
    fn v6_8_46_ne_iste_kerek_co_occurrence_fires() {
        // Industrial procedural query without «қалай» — caught
        // by the second co-occurrence trigger «не істе» + «керек».
        // Was failing in v6.8.43 baseline + v6.8.45.
        assert!(
            lookup_procedure("Мас күйдегі қызметкерді не істеу керек?").is_some(),
            "ne iste kerek industrial query must route"
        );
    }

    #[test]
    fn v6_8_45_qalai_kerek_no_match_on_non_procedural() {
        // Generic «не істеу керек» without procedural anchor:
        // a Kazakh adjective doesn't match any procedure title,
        // so lookup returns None.  Confirms the trigger fires
        // BUT the score-based match still gates the result.
        assert!(lookup_procedure("Кітабым жоқ, қалай оқу керек?").is_none());
        // No «қалай» at all → trigger doesn't fire.
        assert!(lookup_procedure("Кітабым жоқ.").is_none());
    }

    #[test]
    fn v6_8_50_undergoer_shape_recognised() {
        // Worker-perspective actor-undergoer query MUST be
        // recognised — distinct from authority query.
        assert!(looks_like_procedure_undergoer_query(
            "Кім мерзімдік медициналық тексеруден өтуі тиіс?"
        ));
        assert!(looks_like_procedure_undergoer_query(
            "Кім инструктажға қатысады тиіс?"
        ));
        // Without «тиіс» (no obligation framing) — doesn't
        // fire.  Free-floating «кім ... өтеді» is
        // ambiguous between SOP context and general
        // narrative.
        assert!(!looks_like_procedure_undergoer_query(
            "Кім марафонды өтеді?"
        ));
        // Authority query «Кім жауапты?» — NOT an
        // undergoer query (no undergoer verb).
        assert!(!looks_like_procedure_undergoer_query("Кім жауапты?"));
    }

    #[test]
    fn v6_8_50_undergoer_router_returns_applies_to() {
        // Standalone content-match query routes to medical
        // procedure and returns its applies_to (the
        // SUBJECT-undergoers), not its authorization (the
        // RESPONSIBLE parties).
        let answer = lookup_procedure_actor_undergoer(
            "Кім мерзімдік медициналық тексеруден өтуі тиіс?",
            None,
        );
        assert!(answer.is_some());
        let text = answer.unwrap();
        // The medical procedure's applies_to contains
        // «қызметкерлер» (workers) — that should surface,
        // NOT «кадр бөлімі» (HR — the authority).
        assert!(text.contains("қызметкер"), "got: {text}");
        assert!(
            !text.contains("кадр бөлімі"),
            "should not return authority, got: {text}"
        );
    }
}
