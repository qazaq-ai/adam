//! Layer 3.5 — multi-turn session state + dialog state tracking.
//!
//! v0.8.5 introduced a flat slot map. v1.4.0 adds proper Dialog State
//! Tracking (DST): the `Conversation` now carries
//!
//! ```text
//!   { slots: HashMap, active_intent: Option<IntentKind>, intent_history: Vec<IntentKind> }
//! ```
//!
//! - `slots` — accumulated entities across turns (name, age, city, occupation).
//! - `active_intent` — the kind of the LAST recognised intent. Used for
//!   follow-up resolution like "ал сіз?" ("and you?") which re-interpretates
//!   against the previous frame.
//! - `intent_history` — ordered list of every recognised intent kind this
//!   session (bounded to MAX_HISTORY to avoid unbounded growth).
//!
//! Deterministic-by-construction: given (slots, active_intent, input, seed),
//! the next turn's output is fully determined. No probabilistic decisions.

use std::collections::HashMap;

use adam_kernel_fst::lexicon::LexiconV1;
use adam_kernel_fst::morphotactics::{Case, NounFeatures, synthesise_noun};
use adam_reasoning::reasoner::DerivedFact;
use adam_reasoning::{Fact as ReasFact, Predicate as ReasPredicate};
use adam_retrieval::{MorphemeIndex, RankConfig, compose::compose_with_city};

use crate::intent::Intent;
use crate::language_core::canonical_geo_entity;
// v4.0.34 — turn loop now routes through plan_response_with_epistemic
// (Codex Phase 5 part 2). The v4.0.33 `plan_response_with_session`
// remains re-exported from the crate for external callers.
use crate::realiser::realise;
use crate::semantics::interpret_text_with_lexicon;
// **v4.24.0** — `content_roots` moved to `topic_extraction` module.
use crate::templates::TemplateRepository;
use crate::topic_extraction::content_roots;

/// Maximum intent-history length retained across turns. Bounded so a
/// long-running session doesn't accumulate an unbounded trace.
const MAX_HISTORY: usize = 32;

/// How the retrieval-fallback path treats the cited corpus sample.
///
/// v1.9.0 introduces [`InSampleCitySwap`](ComposeMode::InSampleCitySwap),
/// which opts into the option-B composition: when the session has a
/// known city and the cited sample mentions a **different** known city
/// in a biography-free context, the mention is rewritten to the user's
/// city. The quote is no longer byte-identical to the corpus. Default
/// stays [`Verbatim`](ComposeMode::Verbatim) — existing callers see no
/// behavioural change.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ComposeMode {
    /// v1.8.5 and earlier behaviour: cited quote is byte-identical to
    /// the corpus sample. Zero fabrication risk.
    #[default]
    Verbatim,
    /// v1.9.0 option-B experiment: city mentions in the cited sample
    /// are swapped for the user's session city when safe. Feature
    /// bundle is preserved (locative stays locative, etc.). See
    /// [`adam_retrieval::compose`] for the safety guards.
    InSampleCitySwap,
}

/// A running multi-turn dialog.
///
/// v1.6.5 adds an optional [`MorphemeIndex`]: when attached, the
/// `Intent::Unknown` fallback consults the index for the parsed
/// `noun_hint` and cites a concrete Kazakh sentence from the committed
/// corpus in place of a bare "түсінбедім". The index is optional so
/// embedders (CLI, tests) that don't want the retrieval dependency
/// can continue to run the 26-intent pipeline stand-alone.
#[derive(Debug, Clone, Default)]
pub struct Conversation {
    /// Entity slot values accumulated across turns. Core user-facing
    /// slots stay stringly for template compatibility (`name`, `age`,
    /// `city`, `occupation`), while auxiliary canonical slots such as
    /// `city_id` and `geo_kind` carry stable identity for newer layers.
    pub session: HashMap<String, String>,
    /// Kind of the last recognised intent. `None` before the first turn.
    /// Used by follow-up resolution (v1.4.0).
    pub active_intent: Option<IntentKind>,
    /// Ordered history of recognised intent kinds, oldest-first.
    /// Bounded to [`MAX_HISTORY`] items.
    pub intent_history: Vec<IntentKind>,
    /// Optional retrieval index. When `Some`, `Intent::Unknown` gets an
    /// `example` injected for any `noun_hint` that has postings in the
    /// index. Added v1.6.5.
    pub morpheme_index: Option<MorphemeIndex>,
    /// Tunable weights + pack purity priors for the v1.7.0 ranker.
    /// Defaults to `RankConfig::default()` — only override for tests
    /// or experiments. Uses `Option` rather than a direct value to keep
    /// `Conversation::default()` cheap and avoid threading the config
    /// through every embedder.
    pub rank_config: Option<RankConfig>,
    /// How the retrieval fallback treats the cited sample. Default
    /// [`ComposeMode::Verbatim`] keeps the v1.8.5 behaviour (byte-
    /// identical corpus quote); [`ComposeMode::InSampleCitySwap`]
    /// enables the v1.9.0 option-B city rewrite. Added v1.9.0.
    pub compose_mode: ComposeMode,
    /// Rule-derived facts produced by `adam_reasoning::reasoner::run`.
    /// When attached, `Intent::Unknown` can cite a reasoning chain
    /// (e.g. «кітап пен ілім бір-біріне байланысты: екеуі де бұлақ
    /// болып табылады») in addition to or instead of a verbatim corpus
    /// sample. Added v2.7. When absent, dialog behaviour matches v2.6
    /// exactly — retrieval path only.
    pub derived_facts: Vec<DerivedFact>,
    /// Extracted (non-derived) facts, used by the reasoning-integration
    /// path to build human-readable "source_chain" renderings. Typically
    /// loaded alongside `derived_facts` from the same artefact pair.
    /// Added v2.7.
    pub extracted_facts: Vec<ReasFact>,
    /// v4.0.3 — investor-safe reasoning mode. When `true`,
    /// `inject_reasoning_chain` only considers derivations whose entire
    /// `source_chain` is rooted in `data/world_core/*.jsonl` (every
    /// supporting fact human-reviewed). Off by default for backwards
    /// compatibility. `adam_chat --safe` flips it on. Mirrors the
    /// investor-safe filter `adam_demo` Part 4 has applied by default
    /// since v4.0.2.
    pub curated_only_reasoning: bool,
    /// v4.0.27 — structured belief state (Codex v4.0.26 roadmap Phase 1).
    /// Tracks every user assertion with provenance, confidence band,
    /// and contradiction history. Lives **alongside** the flat
    /// `session` map — `absorb_entities` writes to both so existing
    /// template-slot consumers keep working, but higher-level
    /// reasoning now has access to the fuller picture.
    pub belief: crate::belief::BeliefState,
    /// v4.0.29 — goal + task lifecycle state (Codex v4.0.26 roadmap
    /// Phase 2). Rolled forward on every turn AFTER belief
    /// absorption so status reflects the newest intent + belief.
    /// Non-breaking: existing consumers ignore it; Phase 3
    /// `ActionPlanner` will consume it to pick next action.
    pub task: crate::task::TaskState,
    /// v4.0.30 — monotone, **unbounded** turn counter. Codex v4.0.29
    /// review #1 flagged that pre-v4.0.30 turn ids were derived from
    /// `intent_history.len()`, which caps at `MAX_HISTORY = 32`.
    /// After 32 turns the counter plateaued, so belief
    /// `recorded_at_turn` and task `goal_set_at_turn` stopped being
    /// real turn indices — breaking any goal-age / fact-age signal
    /// Phase 3+ will consume.
    ///
    /// This counter increments by 1 at the start of every
    /// `turn_with_trace` call and never resets except on
    /// [`reset`](Self::reset). Saturating-add insures we never panic
    /// even at astronomical turn counts.
    pub turn_counter: usize,
    /// **v4.3.4** — adam's self-identity record, used by the
    /// `ask_about_system.*` template families when the user asks
    /// who/what adam is. Defaults to
    /// [`SystemIdentity::canonical`](crate::system_identity::SystemIdentity::canonical),
    /// which carries the build-time identity (`name = адам`,
    /// `full_name = Agglutinative Reasoning Kernel`, `creator =
    /// Баймурзин Даулет Абузарович`, `birthdate = 2026-04-07`,
    /// architecture summary of how adam differs from mainstream
    /// LLMs). Tests may override individual fields by direct
    /// construction; production callers should leave the default.
    pub system_identity: crate::system_identity::SystemIdentity,
    /// **v4.13.0** — multi-turn topic / domain / focus memory.
    /// Pre-v4.13.0 anaphora resolution consulted only
    /// `session["last_query_topic"]` (one-turn back-reference);
    /// `dialog_context` widens that window to the full conversation
    /// while staying cheap (capped Vec, no graph traversal at update
    /// time). Updated each turn from the resolved Intent's noun_hint
    /// + (when discoverable) the World-Core domain. Consulted on the
    /// next turn for `subject_under_discussion`-driven anaphora and
    /// `current_domain`-aware retrieval scoping. See
    /// [`crate::dialog_context::DialogContext`] for the algorithm.
    pub dialog_context: crate::dialog_context::DialogContext,
    /// **v4.14.0** — topic → World-Core domain index. Built once at
    /// `with_world_core` time and frozen for the lifetime of the
    /// Conversation. Used per turn to populate
    /// `dialog_context.current_domain` from the resolved topic noun.
    /// Empty by default (no curated facts attached) — domain
    /// inference no-ops cleanly when the index is empty.
    pub domain_index: crate::domain_index::DomainIndex,
    /// **v4.15.5** — frequency-based prior over FST suffix-chain
    /// signatures, loaded from
    /// `data/retrieval/suffix_chain_priors.json` (trained offline
    /// in v4.15.0). When `Some`, each turn's `parse_input_with_priors`
    /// re-ranks candidate analyses by `P(chain)` DESC before
    /// picking the first; when `None`, falls back to the v3.2.0
    /// lexicographic deterministic order. Strictly additive: the
    /// stable sort means tied-prior parses preserve v3.2.0 order
    /// exactly.
    pub suffix_priors: Option<adam_kernel_fst::suffix_priors::SuffixPriors>,
    /// **v4.16.5** — Jelinek-Mercer interpolation weight between
    /// unigram and bigram log-probabilities when scoring FST
    /// parses. `α · unigram + (1-α) · bigram`. `None` means
    /// "use pure bigram-with-unigram-fallback (v4.16.0
    /// behaviour)". Recommended default for tuning is `Some(0.3)`
    /// — bigram dominates but unigram smooths sparse rows.
    pub priors_alpha: Option<f32>,
    /// **v4.29.5** — Track A discourse-level prior. Sparse PMI
    /// matrix over root pairs that co-occur in same corpus
    /// sample. When attached, `Tool::dispatch(SearchGraph)`
    /// reranking gains a tiebreaker tier: among candidates with
    /// equal chain priority + equal overlap + equal domain match,
    /// the one whose subject root has higher affinity to the
    /// user's recent topic wins. When `None`, the ranking ladder
    /// preserves v4.29.0 behaviour bit-for-bit. Trained offline
    /// by `train_root_affinity` over the v4.28.5 8.85M-token
    /// corpus; loaded from `data/retrieval/root_affinity.json`
    /// at conversation startup.
    pub root_affinity: Option<adam_kernel_fst::root_affinity::RootAffinity>,
    /// **v4.98.0** — lesson-state curriculum tree. Loaded once from
    /// `data/dialog/curriculum/rust_progression.json` at
    /// `Conversation::new()` time; absent file leaves this `None`
    /// and disables curriculum tracking (back-compat for trimmed
    /// checkouts). See [`crate::curriculum`].
    pub curriculum: Option<crate::curriculum::Curriculum>,
    /// **v4.98.0** — per-conversation per-stage progress map.
    /// Updated after every `Intent::SubmitSolution` turn whose
    /// `cargo_status` slot resolves to `passed` or `failed`. Empty
    /// when no lesson-state turns have happened yet.
    pub curriculum_progress: HashMap<String, crate::curriculum::StageProgress>,
}

/// v4.0.25 — intermediate state captured by
/// [`Conversation::turn_with_trace`]. Each field is the authoritative
/// post-injection snapshot used to drive the final template render.
///
/// Exposed so CLI `--trace` mode surfaces the real runtime path
/// (including `reasoning_chain` and `example` slots populated by
/// `inject_reasoning_chain` / `inject_retrieval_example`) instead of
/// a pre-injection placeholder.
#[derive(Debug, Clone)]
pub struct TurnTrace {
    /// FST parses for each token in the raw input.
    pub parses: Vec<adam_kernel_fst::parser::Analysis>,
    /// **v4.31.5** — first SemFrame consumer migration. One frame
    /// per parse, in the same order as `parses`. Built via
    /// `SemFrame::from_analysis(&analysis)` at trace-capture time.
    /// Lets `--trace` render the unified morphemic-logical IR per
    /// token. v4.31.0 introduced the type; v4.31.5 wires it into
    /// the canonical trace path so every downstream consumer sees
    /// the same frame-per-parse mapping. Substrate for v4.32+
    /// pattern-matcher migration to SemFrame inputs.
    pub sem_frames: Vec<adam_kernel_fst::SemFrame>,
    /// Intent after follow-up resolution AND retrieval/reasoning
    /// injection — this is the shape the planner actually saw.
    pub intent_after_injection: Intent,
    /// v4.0.32 — intent that actually went to `plan_response_with_session`
    /// AFTER Phase 4 verifier gating. Identical to
    /// `intent_after_injection` when `verification.supported`; else
    /// evidence slots stripped so the renderer falls through to safe
    /// templates.
    pub intent_after_verification: Intent,
    /// Session slot snapshot taken immediately after entity absorption.
    pub session_snapshot: HashMap<String, String>,
    /// v4.0.27 — belief-state digest taken after entity absorption.
    /// Cheap to clone (six counters) and sufficient for --trace
    /// output. Consumers needing the full picture can clone
    /// `Conversation::belief` directly.
    pub belief_digest: crate::belief::BeliefDigest,
    /// v4.0.27 — the full belief state at the moment of trace
    /// capture. Lets `adam_chat --trace` render the new / updated
    /// facts and any fresh contradictions.
    pub belief_snapshot: crate::belief::BeliefState,
    /// v4.0.29 — task-state digest (five scalars) for quick trace
    /// rendering.
    pub task_digest: crate::task::TaskDigest,
    /// v4.0.29 — full task state at the moment of trace capture.
    pub task_snapshot: crate::task::TaskState,
    /// v4.0.31 — compact action digest (action + expected output +
    /// rationale count). Phase 3 non-breaking: this is for audit
    /// only; the existing template planner still drives rendering.
    pub action_digest: crate::action::ActionDigest,
    /// v4.0.31 — full `ActionPlan` including rationale list and
    /// required inputs, for auditors who want the full picture.
    pub action_plan: crate::action::ActionPlan,
    /// v4.0.32 — Phase 4 pre-render verification report. When
    /// `supported == false` the turn loop has stripped evidence
    /// from `intent_after_verification` to avoid rendering an
    /// answer on top of an unresolved issue.
    pub verification: crate::verifier::VerificationReport,
    /// v4.0.33 — Phase 5 (part 1) epistemic-status band derived
    /// from `(plan, verification, intent, belief)`. Pure classifier
    /// output; not yet consumed by templates (v4.0.34 wires that).
    pub epistemic_status: crate::uncertainty::EpistemicStatus,
    /// v4.0.37 — Phase 6 (part 1) tool-call audit log. Empty in
    /// v4.0.37: the dispatcher exists and is reachable via
    /// `Tool::dispatch`, but `turn_with_trace` doesn't yet
    /// auto-dispatch. v4.0.38 (Phase 6 part 2) wires the existing
    /// `inject_*` helpers through the tool layer and this Vec
    /// starts populating.
    pub tool_calls: Vec<crate::tool::ToolResult>,
    /// Per-step plan trace emitted by `plan_response_with_session`.
    pub plan_trace: Vec<String>,
}

/// Lightweight "kind" summary of an `Intent` — the payload (name /
/// years / city / …) is already held in `slots`, so history doesn't
/// need to copy it. Keeping this separate from `intent::Intent` avoids
/// retaining potentially large `String`s in the session log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentKind {
    Greeting,
    Farewell,
    Affirmation,
    Negation,
    Thanks,
    Apology,
    /// **v4.42.7** — companion to `Intent::UserDisagrees`.
    UserDisagrees,
    AskHowAreYou,
    StatementOfWellbeing,
    AskName,
    /// **v4.3.3** — companion to `Intent::AskAboutSystem`.
    AskAboutSystem,
    StatementOfName,
    AskAge,
    StatementOfAge,
    AskLocation,
    StatementOfLocation,
    AskOccupation,
    StatementOfOccupation,
    /// **v4.51.0** — companion to `Intent::StatementOfActivity`.
    StatementOfActivity,
    /// **v4.51.0** — companion to `Intent::AskActivity`.
    AskActivity,
    AskFamily,
    StatementOfFamily,
    AskWeather,
    StatementOfWeather,
    AskTime,
    Compliment,
    Request,
    WellWishes,
    Insult,
    UserAcknowledgement,
    /// **v4.14.0** — curriculum-content honest fallback.
    AskCurriculumContent,
    /// **v4.17.5** — willingness / readiness-to-improve question.
    AskWillingness,
    /// **v4.93.5** — pedagogical intents (Codex 2026-05-07 audit P2).
    AskExercise,
    CodeRequest,
    ExplainCompilerError,
    AskPurpose,
    /// **v4.95.0** — student submits Rust code for verification via cargo_verify.
    SubmitSolution,
    /// **v4.96.0** — Codex round-2 audit Bug 7. Cross-language
    /// contrast: «Python-да ownership бар ма?».
    CrossLanguageContrast,
    /// **v4.99.0** — student-side curriculum-query intents.
    AskNextTopic,
    AskCurrentProgress,
    Unknown,
}

impl From<&Intent> for IntentKind {
    fn from(intent: &Intent) -> Self {
        match intent {
            Intent::Greeting { .. } => Self::Greeting,
            Intent::Farewell => Self::Farewell,
            Intent::Affirmation => Self::Affirmation,
            Intent::Negation => Self::Negation,
            Intent::Thanks => Self::Thanks,
            Intent::Apology => Self::Apology,
            Intent::UserDisagrees => Self::UserDisagrees,
            Intent::AskHowAreYou => Self::AskHowAreYou,
            Intent::StatementOfWellbeing => Self::StatementOfWellbeing,
            Intent::AskName => Self::AskName,
            Intent::AskAboutSystem { .. } => Self::AskAboutSystem,
            Intent::StatementOfName { .. } => Self::StatementOfName,
            Intent::AskAge => Self::AskAge,
            Intent::StatementOfAge { .. } => Self::StatementOfAge,
            Intent::AskLocation => Self::AskLocation,
            Intent::StatementOfLocation { .. } => Self::StatementOfLocation,
            Intent::AskOccupation => Self::AskOccupation,
            Intent::StatementOfOccupation { .. } => Self::StatementOfOccupation,
            Intent::StatementOfActivity { .. } => Self::StatementOfActivity,
            Intent::AskActivity => Self::AskActivity,
            Intent::AskFamily => Self::AskFamily,
            Intent::StatementOfFamily => Self::StatementOfFamily,
            Intent::AskWeather => Self::AskWeather,
            Intent::StatementOfWeather => Self::StatementOfWeather,
            Intent::AskTime => Self::AskTime,
            Intent::Compliment => Self::Compliment,
            Intent::Request => Self::Request,
            Intent::WellWishes => Self::WellWishes,
            Intent::Insult => Self::Insult,
            Intent::UserAcknowledgement => Self::UserAcknowledgement,
            Intent::AskCurriculumContent => Self::AskCurriculumContent,
            Intent::AskWillingness => Self::AskWillingness,
            Intent::AskExercise { .. } => Self::AskExercise,
            Intent::CodeRequest { .. } => Self::CodeRequest,
            Intent::ExplainCompilerError { .. } => Self::ExplainCompilerError,
            Intent::AskPurpose { .. } => Self::AskPurpose,
            Intent::SubmitSolution { .. } => Self::SubmitSolution,
            Intent::CrossLanguageContrast { .. } => Self::CrossLanguageContrast,
            Intent::AskNextTopic => Self::AskNextTopic,
            Intent::AskCurrentProgress => Self::AskCurrentProgress,
            Intent::Unknown { .. } => Self::Unknown,
        }
    }
}

impl Conversation {
    /// Start a fresh session — no remembered entities and no history.
    pub fn new() -> Self {
        let mut conv = Self::default();
        // **v4.98.0** — silently load the committed curriculum if
        // present. Absent file (trimmed checkout) leaves
        // `curriculum = None`, disabling progress tracking — same
        // behaviour as if no curriculum file ever existed. Errors
        // during load are also silenced; callers needing strict
        // load semantics can call `Curriculum::load_from_path`
        // directly and assign.
        if let Ok(Some(c)) = crate::curriculum::Curriculum::load_default() {
            conv.curriculum = Some(c);
        }
        conv
    }

    /// Attach a retrieval index so `Intent::Unknown` can quote a
    /// concrete Kazakh sentence for any recognised noun. Without this,
    /// the fallback behaviour is the v1.1.0 noun-echo path.
    pub fn with_morpheme_index(mut self, index: MorphemeIndex) -> Self {
        self.morpheme_index = Some(index);
        self
    }

    /// Opt into v1.9.0 option-B in-sample composition. When the mode
    /// is `InSampleCitySwap` and the session has a recognised city,
    /// `Intent::Unknown` responses rewrite city mentions in the cited
    /// sample to the user's city (feature-preserving, biography-guarded).
    /// Leave at the default `Verbatim` for the v1.8.5 behaviour.
    pub fn with_compose_mode(mut self, mode: ComposeMode) -> Self {
        self.compose_mode = mode;
        self
    }

    /// Attach rule-derived facts + their supporting extracted facts so
    /// `Intent::Unknown` can cite reasoning chains ("X and Y are both
    /// kinds of Z, therefore related"). `extracted` is used to render
    /// the source_chain into Kazakh evidence phrases; `derived` is the
    /// list of `RuleInferred` facts to match against noun hints.
    ///
    /// Both lists default to empty, in which case dialog behaviour is
    /// bit-identical to v2.6 — retrieval path only, zero reasoning
    /// citations.
    pub fn with_reasoning_chains(
        mut self,
        extracted: Vec<ReasFact>,
        derived: Vec<DerivedFact>,
    ) -> Self {
        self.extracted_facts = extracted;
        self.derived_facts = derived;
        self
    }

    /// **v4.14.0** — builder: attach a `DomainIndex` so each turn
    /// can populate `dialog_context.current_domain` from the
    /// resolved topic. Built by the caller from `WorldCoreEntry`s
    /// (typically loaded via
    /// `adam_reasoning::world_core::load_world_core_dir`). Empty by
    /// default — domain inference no-ops cleanly when the index is
    /// empty.
    pub fn with_domain_index(mut self, index: crate::domain_index::DomainIndex) -> Self {
        self.domain_index = index;
        self
    }

    /// **v4.15.5** — builder: attach the trained suffix-chain
    /// prior so each turn's FST parse list is re-ranked by
    /// `P(chain)` before downstream consumers see the
    /// `Vec<Analysis>`. Built by the caller via
    /// `adam_kernel_fst::suffix_priors::SuffixPriors::load(...)`.
    /// Empty / missing: pre-v4.15.5 lexicographic order
    /// preserved bit-for-bit.
    pub fn with_suffix_priors(
        mut self,
        priors: adam_kernel_fst::suffix_priors::SuffixPriors,
    ) -> Self {
        self.suffix_priors = Some(priors);
        self
    }

    /// **v4.16.5** — builder: set the unigram-vs-bigram
    /// interpolation weight for FST parse re-ranking. `α=0.0` =
    /// pure bigram (v4.16.0); `α=1.0` = pure unigram (v4.15.5);
    /// `α≈0.3` = bigram dominates with unigram smoothing.
    /// Without this builder, parse selection uses the v4.16.0
    /// pure-bigram-with-fallback path.
    pub fn with_priors_alpha(mut self, alpha: f32) -> Self {
        self.priors_alpha = Some(alpha);
        self
    }

    /// **v4.29.5** — builder: attach the trained root-affinity
    /// PMI matrix so `Tool::dispatch(SearchGraph)` reranking
    /// gains a discourse-level tiebreaker. Among candidates
    /// with equal chain priority + equal overlap + equal domain
    /// match, the one whose subject root has higher PMI to the
    /// turn's primary topic root wins. Built by the caller via
    /// `adam_kernel_fst::root_affinity::RootAffinity::load(...)`
    /// from `data/retrieval/root_affinity.json`. When `None`,
    /// the ranking ladder preserves v4.29.0 order bit-for-bit.
    pub fn with_root_affinity(
        mut self,
        affinity: adam_kernel_fst::root_affinity::RootAffinity,
    ) -> Self {
        self.root_affinity = Some(affinity);
        self
    }

    /// v4.0.3 — builder: enable investor-safe reasoning mode.
    /// When enabled, `inject_reasoning_chain` only cites derivations
    /// whose full `source_chain` comes from `data/world_core/*.jsonl`
    /// (every supporting fact human-reviewed). Extracted-pack sources
    /// (wikipedia, textbooks, abai) are refused.
    ///
    /// Default (`false`) preserves v4.0.2 behaviour — the chat cites
    /// any derivation that references the user's topic. Only
    /// `adam_chat --safe` flips this on.
    pub fn with_curated_only_reasoning(mut self, enabled: bool) -> Self {
        self.curated_only_reasoning = enabled;
        self
    }

    /// **v4.95.5** — read-only session accessor. Surfaces a single
    /// session slot (e.g. `last_exercise_topic`) for callers that
    /// need to inspect the lesson state without copying the whole
    /// session HashMap. Returns owned String to avoid lifetime
    /// gymnastics across turns. Tests use this to verify the
    /// multi-turn lesson-state contract without touching internals.
    pub fn session_value(&self, key: &str) -> Option<String> {
        self.session.get(key).cloned()
    }

    /// Run one conversational turn. Parses the input, recognises the
    /// intent, folds any new entities into [`session`](Self::session),
    /// updates [`active_intent`](Self::active_intent) and
    /// [`intent_history`](Self::intent_history), then plans + realises
    /// a response using the merged slot map.
    ///
    /// Deterministic given (current state, input, seed). The state
    /// mutation is the ONLY side-effect.
    pub fn turn(
        &mut self,
        input: &str,
        lexicon: &LexiconV1,
        repo: &TemplateRepository,
        rng_seed: u64,
    ) -> String {
        let (out, _) = self.turn_with_trace(input, lexicon, repo, rng_seed);
        out
    }

    /// v4.0.25 — trace-aware variant of [`turn`]. Identical runtime
    /// behaviour (same state mutation, same output); additionally
    /// returns a [`TurnTrace`] capturing the intermediate state
    /// **after** retrieval + reasoning-chain injection, so consumers
    /// like `adam_chat --trace` can print the real runtime path.
    ///
    /// Codex v4.0.23 re-review #2 flagged that the pre-v4.0.25 trace
    /// mode manually duplicated `turn` but stopped before
    /// `inject_retrieval_example` + `inject_reasoning_chain`, making
    /// trace output materially false for v4.0.20+ features. This
    /// method closes that gap by making trace a first-class output
    /// of the single canonical code path.
    pub fn turn_with_trace(
        &mut self,
        input: &str,
        lexicon: &LexiconV1,
        repo: &TemplateRepository,
        rng_seed: u64,
    ) -> (String, TurnTrace) {
        // **v4.6.20** — preamble stripper. Real-REPL surfaced
        // sentences like «Айтайын дегенім, қолданыстағы жасанды
        // интеллект модельдерінен қалай жақсырақ бола аласыз?» —
        // the leading clause carries no semantic content but the
        // greedy noun-hint extractor still grabbed the first
        // content noun (`қолданыс`) and pulled a contract-template
        // quote, missing the actual question. Strip the preamble
        // BEFORE parsing so all downstream layers see only the
        // meaningful clause. Math/Russian/anaphor detection still
        // sees the raw input below — those checks operate on
        // surface signals (digits, Cyrillic-script class) where
        // preambles never interfere.
        // **v4.11.5** — vocative-addressee stripper runs after the
        // v4.6.20 preamble stripper so a preamble + vocative
        // combination (e.g. «Айтпақшы, адам, мектептің физика
        // бағдарламасын білесің бе?») collapses cleanly. Pre-
        // v4.11.5 the leading vocative `адам` was taken as the
        // first content noun and routed retrieval to
        // `адам IsA сүтқоректі` — completely off-topic. Stripping
        // happens BEFORE FST parsing so all downstream layers
        // (parses, noun_hint, retrieval) see only the meaningful
        // residual; surface-level checks (Russian/math detection)
        // still operate on the raw input above.
        let stripped = crate::discourse::strip_addressee(crate::discourse::strip_preamble(input));
        // **v4.15.5 + v4.16.5** — priors-aware parse path. When a
        // trained `SuffixPriors` artifact is attached, each
        // token's candidate analyses are re-ranked by `P(chain)`
        // DESC before downstream consumers see the
        // `Vec<Analysis>`. `priors_alpha` selects the scoring
        // mode: `Some(α)` uses Jelinek-Mercer interpolation,
        // `None` uses pure-bigram-with-unigram-fallback (v4.16.0).
        let parses = crate::parse_input_with_priors(
            stripped,
            lexicon,
            self.suffix_priors.as_ref(),
            self.priors_alpha,
        );
        // **v4.33.5** — build SemFrames immediately after parses so
        // they're available BEFORE intent finalisation. v4.31.5 built
        // them at the END of the turn (trace-assembly time); v4.33.5
        // moves the build forward so populated fields (polarity,
        // modality, evidence) can influence Intent / planner routing.
        // The detector chain (periphrastic modality / ability /
        // sentential negation) runs once here; the result flows into
        // both `intent.noun_hint_polarity` (v4.33.5 consumer) and
        // `TurnTrace.sem_frames` (v4.31.5 trace consumer).
        let mut sem_frames: Vec<adam_kernel_fst::SemFrame> =
            parses.iter().map(adam_kernel_fst::SemFrame::from).collect();
        adam_kernel_fst::populate_periphrastic_modality(&mut sem_frames);
        adam_kernel_fst::populate_ability_modality(&mut sem_frames);
        adam_kernel_fst::populate_sentential_negation(&mut sem_frames);
        let raw_intent = interpret_text_with_lexicon(stripped, &parses, Some(lexicon));

        // v1.4.0: follow-up resolution. "ал сіз?" after AskHowAreYou
        // becomes a re-interpretation: "user is asking me the same
        // thing back", which is still AskHowAreYou for planning
        // purposes (planner picks a response without asking back).
        let mut intent = resolve_follow_up(raw_intent, input, self.active_intent);

        // **v4.76.5** — comparison shape detection. «X пен Y
        // айырмашылығы қандай?» — extract X as primary topic so the
        // existing retrieval pipeline finds X's definition; carry Y
        // as a separate slot so the comparison template can suggest
        // a follow-up query for Y. Honest hedge: full side-by-side
        // comparison (both definitions in one turn) requires dual
        // retrieval and is deferred to v5+.
        let comparison_topics = crate::discourse::try_extract_comparison_topics(input);
        if let Some((x_topic, _y_topic)) = comparison_topics.as_ref() {
            // Override noun_hint with X if it would otherwise be empty
            // or low-confidence. This routes the rest of the pipeline
            // to retrieve X's facts.
            if let crate::intent::Intent::Unknown {
                ref mut noun_hint,
                ref mut noun_hint_confidence,
                ..
            } = intent
            {
                if noun_hint.is_none()
                    || matches!(
                        noun_hint_confidence,
                        crate::topic_extraction::TopicConfidence::Low
                    )
                {
                    *noun_hint = Some(x_topic.clone());
                    *noun_hint_confidence = crate::topic_extraction::TopicConfidence::High;
                }
            }
        }

        // **v4.33.5** — first consumption of SemFrame fields by
        // intent: copy `Polarity::Negated` onto Intent::Unknown
        // when the noun_hint frame carries it. Looks up the SemFrame
        // whose root matches noun_hint (case-insensitive) and reads
        // its polarity. When v4.33.0's `populate_sentential_negation`
        // detected «X емес» on the noun, polarity is Negated and the
        // planner will route this turn to `unknown.with_negated_topic`
        // instead of asserting a definition that contradicts the
        // user's claim. Default Affirmative path preserves all pre-
        // v4.33.5 routing exactly. This is the FIRST SemFrame field
        // that influences the answer text — v4.31.0–v4.33.0 all
        // populated fields without consumption.
        if let Intent::Unknown {
            noun_hint,
            noun_hint_polarity,
            input_modality,
            input_evidence,
            input_is_inversion_question,
            ..
        } = &mut intent
        {
            // **v4.33.5** — populate noun_hint_polarity from the
            // matching SemFrame. Only meaningful when noun_hint is
            // Some — polarity is per-noun, no noun → no polarity to
            // copy.
            if let Some(hint) = noun_hint.as_deref() {
                let hint_lower = hint.to_lowercase();
                if let Some(frame) = sem_frames
                    .iter()
                    .find(|f| f.root.to_lowercase() == hint_lower)
                {
                    *noun_hint_polarity = frame.polarity;
                }
            }
            // **v4.34.7 + v4.35.5** — populate input_modality from
            // any frame in the stream with modality set. Modality is
            // on the LEXICAL VERB frame (not on noun_hint), so the
            // walk is independent of noun_hint. v4.35.5 moved this
            // out of the noun_hint-Some guard so verb-only modal
            // claims like «Жаза аламын» (battery case 21) populate
            // the field — pre-v4.35.5 the population was nested
            // inside the noun_hint check and silently skipped.
            if input_modality.is_none() {
                *input_modality = sem_frames.iter().find_map(|f| f.modality);
            }
            // **v4.36.0** — third SemFrame field consumption.
            // EvidenceKind is on the verb frame (set when
            // Tense::PastEvidential triggers the auto-derivation in
            // SemFrame::from_analysis). Walk the frame stream for
            // the first non-None evidence and copy onto Intent.
            // Independent of noun_hint — like modality.
            if input_evidence.is_none() {
                *input_evidence = sem_frames.iter().find_map(|f| f.evidence);
            }
            // **v4.37.0** — inversion-question detection. When the
            // sentence has BOTH «емес» (any inflection) AND a tag-
            // question particle («бе / ма / ме / па / пе») in the
            // SemFrame stream, it's a confirmation-seeking
            // inversion («Бұл дұрыс емес пе?» — "isn't this
            // correct?"), NOT a denial. The polarity-from-emes
            // detector still fires, but the planner will route
            // through the inversion-question family (which
            // engages with the confirmation-seeking shape) instead
            // of the v4.33.5 negation-acknowledgment family
            // (which would misread the speaker's intent as
            // denial).
            if !*input_is_inversion_question {
                let has_emes = sem_frames.iter().any(|f| f.root == "емес");
                let has_qparticle = sem_frames
                    .iter()
                    .any(|f| matches!(f.root.as_str(), "бе" | "ма" | "ме" | "па" | "пе"));
                if has_emes && has_qparticle {
                    *input_is_inversion_question = true;
                }
            }
        }

        // **v4.6.0** — discourse-anaphora resolution. When the user's
        // input contains a discourse anaphor («онда / сонда / осында
        // / мұнда / бұнда / одан / содан / бұдан / осыдан»),
        // **override** the current turn's noun_hint with the most-
        // recent query topic (`session["last_query_topic"]`). Real-
        // REPL 2026-04-29 transcript: «Ал онда қанша аймақ бар?»
        // following «Қазақстан туралы не білесіз?» — `онда` refers
        // to Қазақстан (locative). The override is unconditional
        // (not gated on `noun_hint == None`) because the FST often
        // does recover SOME content noun from the same turn (e.g.
        // `аймақ` here), but that content noun is the predicate of
        // the question, not its topic. The previous-turn topic is
        // the right subject; the current-turn content noun stays
        // in the user's raw input and continues to influence
        // retrieval ranking via the v4.4.11 input-overlap reranker.
        if crate::discourse::input_contains_discourse_anaphor(input) {
            // **v4.13.0** — DialogContext-driven anaphora resolution.
            // Pre-v4.13.0 only `session["last_query_topic"]` was
            // consulted (one-turn back-reference). DialogContext
            // widens the window to the full conversation: if a
            // sticky `subject_under_discussion` exists, it wins over
            // the immediate `last_topic` (closes the 2026-05-01
            // transcript pattern where «оны» refers to a topic
            // established several turns ago). Falls back to the
            // legacy session entry for full backward compatibility
            // when DialogContext has no entries (older callers that
            // never populate it).
            let resolved = self
                .dialog_context
                .resolve_anaphor()
                .map(|s| s.to_string())
                .or_else(|| self.session.get("last_query_topic").cloned());
            if let Some(prev_topic) = resolved {
                if let Intent::Unknown {
                    ref mut noun_hint,
                    ref mut noun_hint_confidence,
                    ..
                } = intent
                {
                    *noun_hint = Some(prev_topic);
                    // **v4.37.5** — discourse-anaphora resolution is a
                    // structural recovery of the topic from the
                    // dialog context. The current turn's confidence
                    // band reflects the surface-form pick (often Low
                    // when the only token left is a pronoun like
                    // `олар`); promoting to High here lets the
                    // resolved topic continue to drive the standard
                    // confident path instead of being intercepted by
                    // the `clarify_low_confidence` fork. Without this
                    // promotion, multi-turn anaphor cases like
                    // «Оларды тізімдей аласыз ба?» following an
                    // established list-class topic would clarify
                    // instead of listing.
                    *noun_hint_confidence = crate::topic_extraction::TopicConfidence::High;
                }
            }
        }

        // **v4.6.12** — Russian-input detection. When the user types
        // Russian («Это очень круто, а кто тебя создал?») the
        // standard pipeline tries to extract a topic noun from
        // whatever leaks through (e.g. `Это`) and surfaces a
        // half-Russian half-Kazakh refusal. Per
        // `project_kazakh_only_directive`, adam is Kazakh-only —
        // Russian inputs should refuse cleanly. Mark the intent so
        // the planner picks the dedicated `unknown.non_kazakh`
        // template family.
        // **v5.2.5** — Codex round-3 audit Bug 6. English-input
        // refusal joins the Russian-input refusal so adam stays
        // Kazakh-only as advertised. Both flow into the same
        // `unknown.non_kazakh` template family via `__non_kazakh__`.
        let russian_input = crate::discourse::input_is_likely_russian(input)
            || crate::discourse::input_is_likely_english(input);

        // **v4.6.12** — Math-expression detection. Real-REPL
        // 2026-04-29: «5+5» / «7 + 3 =» / «6:2=» / «5-ті 7-ге
        // көбейткенде» / «алтыны екіге бөліңіз» — adam doesn't
        // compute math (per `limitations_summary`), so route to
        // the dedicated `math_refusal` template family. Detection
        // requires a clear math shape (operator near digits OR
        // math verb + numeric tokens) — pure mentions of numbers
        // («Қазақстанда 17 облыс бар») don't trigger.
        // **v4.77.0** — code-snippet detection (Codex round-2 Bug 8).
        // Detect Python-style code BEFORE math classification so
        // «for i in range(3): print(i)» doesn't fall to math_refusal.
        // Routes to dedicated `code_refusal` template via
        // `__code_input__` slot.
        let code_input = crate::discourse::input_is_code_snippet(input);
        let math_input = !code_input && crate::discourse::input_is_math_expression(input);

        // **v4.2.0** — tool-loop orchestration replaces the v4.0.37
        // `inject_*` helpers + audit block with a single uniform
        // pipeline: build a `Vec<ToolCall>` declaring which lookups
        // this turn needs, dispatch them all once, fold the results
        // back into the intent. Adding a new tool consult in the
        // future means appending a `ToolCall` to the plan, not
        // writing a new `inject_*` helper.
        //
        // - **Intent enrichment** (v1.6.5 retrieval example, v2.7
        //   reasoning chain): driven by `apply_tool_results`. The
        //   v1.9.0 city-swap composition still applies inside the
        //   retrieval-result path; ranker ties still break on
        //   `(pack, sample_id)`; the rendered reasoning chain still
        //   carries the «байланыс-» trust marker.
        // - **Audit-only tools** (`SearchBelief`, `SearchGraph`):
        //   dispatched and recorded on `TurnTrace.tool_calls`; never
        //   mutate the intent.
        let tool_plan = self.tool_plan_for_turn(&intent, &parses);
        let tool_ctx = crate::tool::ToolContext {
            belief: &self.belief,
            extracted: &self.extracted_facts,
            derived: &self.derived_facts,
            retrieval: self.morpheme_index.as_ref(),
            rank_config: self.rank_config.as_ref(),
            // v4.4.11 — pass the raw user input so SearchGraph can
            // rerank candidate facts by content-token overlap with
            // the question, not just by predicate centrality.
            query_input: Some(input),
            // **v4.14.5** — domain-aware retrieval. The
            // currently-discussed domain (from DialogContext) +
            // the DomainIndex are passed so SearchGraph reranking
            // can break ties in favour of facts in the current
            // subject area. Both are `Some` only when v4.14.0+
            // domain wiring is attached; older callers pass
            // through with `None`/empty index, preserving
            // pre-v4.14.5 behaviour bit-for-bit.
            current_domain: self.dialog_context.current_domain.as_deref(),
            domain_index: if self.domain_index.is_empty() {
                None
            } else {
                Some(&self.domain_index)
            },
            // **v4.18.0** — previous turn's grounded_fact text,
            // for list-class context tracking.
            previous_grounded_fact: self.session.get("last_grounded_fact").map(String::as_str),
            // **v4.29.5** — Track A discourse-level prior. Sparse
            // PMI matrix over root pairs, used as a tiebreaker in
            // SearchGraph reranking. `None` (no PMI matrix
            // attached) preserves v4.29.0 behaviour bit-for-bit.
            root_affinity: self.root_affinity.as_ref(),
            // **v4.47.0** — Stage B bundle 5: previous turn's topic
            // root, threaded through to `selection::audit_compare`
            // so the recency-match feature fires on real session
            // data. `None` on the first turn or when no topic was
            // resolved.
            last_topic: self.session.get("last_query_topic").map(String::as_str),
        };
        let tool_calls: Vec<crate::tool::ToolResult> = tool_plan
            .into_iter()
            .map(|call| crate::tool::Tool::dispatch(call, &tool_ctx))
            .collect();
        self.apply_tool_results(&mut intent, &tool_calls, lexicon);

        // v4.0.30 — unbounded monotone turn id (Codex v4.0.29 #1 fix).
        // Captured BEFORE absorption so belief.record_user_fact,
        // task.roll_forward, and entity touches all share the same
        // turn number for this turn. Increment after capture so the
        // next turn sees the next integer.
        let turn_id = self.turn_counter;
        self.turn_counter = self.turn_counter.saturating_add(1);
        // v4.0.41 — if a contradiction is pending and this turn names
        // one of the candidate values, treat the turn as a resolution
        // (flip statuses + drop the conflict) and SKIP absorb_entities
        // so we don't double-record the chosen value as a fresh Active
        // fact. Closes aspirational scenario
        // `aspirational_contradiction_resolution_via_user_choice`.
        //
        // **v4.4.0** — try DISMISSAL first (`try_dismiss_pending_contradiction`):
        // when the user replies "neither / I don't know / skip" to a
        // pending CheckContradiction, drop both contested values to
        // Superseded without choosing one. Symmetric "neither" path
        // alongside the v4.0.41 "pick one" path. If dismissal fires,
        // also skip absorb_entities — it was a meta-dialog answer,
        // not a fresh claim.
        let dismissed_contradiction = self.try_dismiss_pending_contradiction(input);
        let resolved_contradiction = if dismissed_contradiction {
            false
        } else {
            self.try_resolve_pending_contradiction(input, &intent)
        };
        if !dismissed_contradiction && !resolved_contradiction {
            self.absorb_entities(&intent, turn_id);
            // **v4.51.0** — secondary activity-slot scan. The primary
            // intent detector picks one intent per turn (occupation
            // OR activity, not both). For compound inputs like
            // «Мен бағдарламашымын және жасанды интеллект әзірлеймін»
            // the user states BOTH; we scan the raw input for
            // activity-verb patterns AFTER the primary absorption
            // and capture activity into session if found, regardless
            // of which intent the primary detector fired.
            if !matches!(intent, crate::intent::Intent::StatementOfActivity { .. }) {
                if let Some(Some(activity)) = crate::semantics::detect_activity_in_compound(input) {
                    if !activity.is_empty() {
                        self.session.insert("activity".into(), activity.clone());
                        self.belief.record_user_fact(
                            crate::belief::USER_SELF_KEY,
                            "activity",
                            &activity,
                            turn_id,
                        );
                    }
                }
            }
            // **v4.52.0** — secondary occupation-slot scan. Mirrors
            // the activity scan above. When the primary intent is
            // StatementOfName (or anything else), but the input
            // also contains a copula-suffixed occupation noun
            // («бағдарламашымын», «инженермін»), capture it. Lets
            // the user say «Менің атым X, мен Y-мын» and have
            // both name AND occupation stored from a single turn.
            if !matches!(intent, crate::intent::Intent::StatementOfOccupation { .. })
                && !self.session.contains_key("occupation")
                && let Some(Some(occupation)) =
                    crate::semantics::detect_occupation_in_compound(input, Some(lexicon))
                && !occupation.is_empty()
            {
                self.session.insert("occupation".into(), occupation.clone());
                self.belief.record_user_fact(
                    crate::belief::USER_SELF_KEY,
                    "occupation",
                    &occupation,
                    turn_id,
                );
            }
        }
        // v4.0.29 — roll task state forward AFTER belief absorption,
        // BEFORE record_intent so the turn id used by task matches
        // the one already used by absorb_entities.
        self.task.roll_forward(&intent, &self.belief, turn_id);
        // v4.0.31 Phase 3 — classify the chosen action for this
        // turn AFTER belief + task are up to date. The result is
        // stored on task.last_action and echoed in TurnTrace for
        // audit. The template planner below still drives the
        // surface form in v4.0.31 — the verifier (Phase 4) is what
        // will actually gate outputs on `ActionPlan`.
        //
        // **v4.4.0** — when this turn just dismissed a pending
        // contradiction (`try_dismiss_pending_contradiction`
        // returned true above), short-circuit the planner with a
        // dedicated `DismissContradiction` action. The contradiction
        // has already been cleared from `self.belief`, so the
        // planner's step 1 ("contradictions dominate") would NOT
        // fire — but we still want the reply to acknowledge the
        // dismissal explicitly rather than fall through to a
        // generic "I don't understand". Mirrors the v4.0.41 short-
        // circuit pattern for resolution turns.
        let action_plan = if dismissed_contradiction {
            crate::action::ActionPlan::new(
                crate::action::Action::DismissContradiction,
                crate::action::OutputKind::SocialPleasantry,
                vec!["user dismissed pending contradiction; both values superseded".into()],
                vec!["belief.dismiss_contradiction".into()],
            )
        } else {
            // **v4.4.0** — `plan_with_turn` applies the
            // contradiction-priority cap so a stale conflict
            // doesn't dominate every turn. `turn_id` is the
            // current turn we just consumed for absorption above.
            crate::action::ActionPlanner::plan_with_turn(&intent, &self.belief, &self.task, turn_id)
        };
        self.task.last_action = Some(action_plan.clone());
        // v4.0.32 Phase 4 — verify the chosen plan against evidence
        // present. If the verifier rejects (e.g. the plan is
        // RunReasoner under a belief contradiction), we strip the
        // injected evidence from a clone of the intent before
        // template planning. The template planner then naturally
        // falls through to `unknown.with_noun` → «ах, X туралы
        // айтасыз ба» or `unknown` → «түсінбедім», neither of which
        // can make a false claim on top of an unresolved conflict.
        // The **original** intent is preserved on TurnTrace so
        // auditors can see what the injection passes actually
        // produced before the gate intervened.
        let verification = crate::verifier::Verifier::verify(&action_plan, &intent, &self.belief);
        // v4.0.33 Phase 5 (part 1) — derive the epistemic status
        // band from `(plan, verification, intent, belief)`. Pure
        // classifier in v4.0.33 — the status is recorded on the
        // trace but templates don't yet consume it. v4.0.34 (Phase 5
        // part 2) adds `unknown.conflicted` / `unknown.tentative`
        // template families and the reply text starts reflecting
        // the status.
        let epistemic_status = crate::uncertainty::UncertaintyPolicy::derive(
            &action_plan,
            &verification,
            &intent,
            &self.belief,
        );
        let intent_for_render = if verification.supported {
            intent.clone()
        } else {
            crate::verifier::strip_evidence(intent.clone())
        };
        self.record_intent(&intent);
        // v4.0.34 Phase 5 part 2 — build `extra_slots` carrying
        // conflict details if the epistemic policy landed on
        // `Conflicted`. Pulled from the MOST RECENT conflict so
        // re-contradictions replace older ones, and the
        // single-active-fact invariant (v4.0.28) is respected.
        let mut extra_slots: HashMap<String, String> = HashMap::new();
        // v4.4.5 — populate conflict slots whenever either the
        // epistemic policy landed on Conflicted OR the action plan
        // chose CheckContradiction. The two used to coincide, but
        // we should not depend on that — `Action::CheckContradiction`
        // is the upstream signal that the user-facing reply needs
        // `{old_value}` / `{new_value}`, not the downstream band.
        let conflict_render = epistemic_status == crate::uncertainty::EpistemicStatus::Conflicted
            || matches!(
                action_plan.action,
                crate::action::Action::CheckContradiction
            );
        if conflict_render {
            if let Some(c) = self.belief.contradictions.last() {
                let old = self
                    .belief
                    .facts
                    .get(c.fact_a_index)
                    .map(|f| f.object.clone())
                    .unwrap_or_default();
                let new = self
                    .belief
                    .facts
                    .get(c.fact_b_index)
                    .map(|f| f.object.clone())
                    .unwrap_or_default();
                // v4.0.34 — map the internal English predicate key
                // onto a Kazakh surface form so the rendered
                // conflict-surfacing template reads naturally.
                // Unmapped predicates fall through to their raw
                // form — better a slight stylistic oddity than a
                // silent omission.
                let predicate_kz = match c.predicate.as_str() {
                    "name" => "атыңыз",
                    "age" => "жасыңыз",
                    "city" => "қалаңыз",
                    "occupation" => "мамандығыңыз",
                    other => other,
                };
                extra_slots.insert("predicate".into(), predicate_kz.into());
                extra_slots.insert("old_value".into(), old);
                extra_slots.insert("new_value".into(), new);
            }
        }
        // **v4.3.4** — when the user is asking about adam itself,
        // inject the `system_*` template slots so the
        // `ask_about_system.*` family can render the canonical
        // identity (name / creator / birthdate / architecture).
        // Slots are only injected on this intent so the
        // `system_*` namespace doesn't leak into unrelated
        // templates and `template_is_fillable` stays accurate.
        if matches!(intent, crate::intent::Intent::AskAboutSystem { .. }) {
            for (k, v) in self.system_identity.template_slots() {
                extra_slots.insert(k, v);
            }
        }
        // **v4.4.0** — marker slot tells the planner to use the
        // `dismiss_contradiction` template family. Sentinel name
        // starts with `__` so it's filtered out of the slot map
        // before realisation; the marker is meta-state, not a
        // user-facing value.
        if dismissed_contradiction {
            extra_slots.insert("__dismiss_contradiction__".into(), "1".into());
        }
        // **v5.3.0** — Codex round-3 audit Bug 2 (sub-fix). When the
        // user resolves a pending contradiction by EXPLICITLY naming
        // one of the contested values («Жоқ, Алматы дұрыс» — Almaty
        // wins, Astana drops), `try_resolve_pending_contradiction`
        // already updated belief state correctly (active=1), but the
        // intent stays `Negation` (because «Жоқ» fired the
        // negation parser). Without this sentinel the planner would
        // fall through to the generic negation template («Дұрыс
        // емес.») — wrong response. The marker routes the planner
        // to an explicit resolution-acceptance template that mirrors
        // statement_of_location semantics with the chosen value.
        if resolved_contradiction {
            extra_slots.insert("__resolve_contradiction__".into(), "1".into());
            // Surface the chosen value as a slot so the template can
            // confirm: «Түсіндім, мекеніңіз — Алматы екен.»
            if let Some(city) = self.session.get("city").cloned() {
                extra_slots.insert("city".into(), city);
            }
            if let Some(name) = self.session.get("name").cloned() {
                extra_slots.insert("name".into(), name);
            }
            if let Some(occupation) = self.session.get("occupation").cloned() {
                extra_slots.insert("occupation".into(), occupation);
            }
        }
        // **v4.4.5** — symmetric marker for `Action::CheckContradiction`
        // routes the renderer to the new `check_contradiction`
        // template family. Pre-v4.4.5 the action layer correctly
        // chose CheckContradiction whenever a fresh `BeliefConflict`
        // landed (within the v4.4.0 priority cap), but the planner
        // still keyed on `intent_key(intent)` and emitted a
        // statement-of-* confirmation for one of the contested
        // values. Using the action variant directly (not
        // epistemic_status) is intentional — the action plan IS
        // the source of truth for "which template family"; the
        // epistemic band is downstream metadata.
        if matches!(
            action_plan.action,
            crate::action::Action::CheckContradiction
        ) {
            extra_slots.insert("__check_contradiction__".into(), "1".into());
        }
        // v4.6.12 — Russian-input marker (set above based on
        // `input_is_likely_russian`). Carried into the planner so
        // the `unknown.non_kazakh` template family fires.
        if russian_input {
            extra_slots.insert("__non_kazakh__".into(), "1".into());
        }
        // **v4.77.0** — Code-snippet marker (Codex round-2 Bug 8).
        // Routes to dedicated `code_refusal` template family explaining
        // adam recognised code but doesn't execute it yet. Closes the
        // false-positive where «for i in range(3): print(i)» fell to
        // math_refusal.
        if code_input {
            extra_slots.insert("__code_input__".into(), "1".into());
        }
        // **v4.78.0** — political-recommendation safety (Codex Bug 3).
        // adam is a school tutor and must not push partisan views.
        // When user asks adam to recommend a party / candidate / vote
        // / political opinion, route to dedicated political_safety
        // refusal that names what adam CAN do (factual info on
        // institutions / parties / candidates) vs what it won't do
        // (give a recommendation).
        if crate::discourse::is_political_recommendation(input) {
            extra_slots.insert("__political_safety__".into(), "1".into());
        }
        // v4.6.12 — Math-input marker (set above based on
        // `input_is_math_expression`). Carried into the planner
        // so the `math_refusal` template family fires.
        // **v4.6.15** — when input parses as pure integer
        // arithmetic, compute the result and pass it through
        // `__math_answer__` so the planner picks `math_answer`
        // instead of the refusal family. Falls through to
        // `__math_input__` (math_refusal) when computation
        // returns None — Kazakh-language phrasings, complex
        // expressions, division-by-zero, etc.
        if math_input {
            // **v4.41.0** — word-form math fallback. The digit-only
            // `try_evaluate_arithmetic` returns None for Kazakh
            // phrasings like «бесті отызға көбейту» (5×30); pre-
            // -v4.41.0 these fell through to `__math_input__` →
            // refusal family. Now the `try_evaluate_kazakh_word_math`
            // path catches them. When either evaluator succeeds,
            // we ALSO compute the Kazakh-word rendering of the
            // result and surface both — the template
            // `"{math_value} ({math_words})"` shows e.g.
            // «150 (жүз елу)», matching the user's expectation
            // («стопятьдесят») while keeping the digit for
            // copy-pasting / readability on large numbers.
            // **v4.51.5** — math anaphora: when input refers to the
            // previous result («алдыңғы есептеу нәтижесі», «оны»,
            // «соны», «алдыңғы нәтиже»), substitute the stored
            // `last_math_result` slot value into the input before
            // evaluation. Lets follow-up turns like «алдыңғы есептеу
            // нәтижесін екіге көбейтіңіз» (multiply previous result
            // by two) compute through.
            let resolved_input: std::borrow::Cow<'_, str> = {
                let lower = input.to_lowercase();
                let has_anaphor = lower.contains("алдыңғы есептеу")
                    || lower.contains("алдыңғы нәтиже")
                    || lower.contains("соңғы есептеу")
                    || lower.contains("соңғы нәтиже")
                    // **v4.52.0** — demonstrative «бұл (есептеу )?нәтиже»
                    // ("this result"). Same anaphoric reference as
                    // «алдыңғы»; user-transcript session 5 surfaced
                    // it: «Енді бұл нәтижені 4-ке бөліңіз».
                    || lower.contains("бұл есептеу")
                    || lower.contains("бұл нәтиже");
                if has_anaphor && let Some(prev) = self.session.get("last_math_result") {
                    // **v4.51.5** — preserve case marker. The
                    // word-math evaluator requires operands to be
                    // case-marked (Acc «-ні»/«-ды», Dat «-ге»/«-ке»);
                    // substituting the bare digit «10» loses that
                    // signal. We append a case-suffix («10-ні»)
                    // matching the anaphoric phrase's case so the
                    // digit-prefix-Acc path (v4.41.7) parses
                    // correctly. Most queries use Acc («нәтижесін»);
                    // Dat is added defensively.
                    let prev_acc = format!("{}-ні", prev);
                    let prev_dat = format!("{}-ге", prev);
                    let with_prev = lower
                        .replace("алдыңғы есептеу нәтижесіне", &prev_dat)
                        .replace("алдыңғы есептеу нәтижесін", &prev_acc)
                        .replace("алдыңғы есептеу нәтижесі", &prev_acc)
                        .replace("алдыңғы нәтижеге", &prev_dat)
                        .replace("алдыңғы нәтижені", &prev_acc)
                        .replace("алдыңғы нәтижесін", &prev_acc)
                        .replace("алдыңғы нәтижесіне", &prev_dat)
                        .replace("алдыңғы нәтижесі", &prev_acc)
                        .replace("алдыңғы нәтиже", &prev_acc)
                        .replace("соңғы есептеу нәтижесін", &prev_acc)
                        .replace("соңғы есептеу нәтижесі", &prev_acc)
                        .replace("соңғы нәтижеге", &prev_dat)
                        .replace("соңғы нәтижесін", &prev_acc)
                        .replace("соңғы нәтиже", &prev_acc)
                        // **v4.52.0** — demonstrative «бұл нәтиже».
                        .replace("бұл есептеу нәтижесіне", &prev_dat)
                        .replace("бұл есептеу нәтижесін", &prev_acc)
                        .replace("бұл есептеу нәтижесі", &prev_acc)
                        .replace("бұл нәтижесіне", &prev_dat)
                        .replace("бұл нәтижесін", &prev_acc)
                        .replace("бұл нәтижесі", &prev_acc)
                        .replace("бұл нәтижеге", &prev_dat)
                        .replace("бұл нәтижені", &prev_acc)
                        .replace("бұл нәтиже", &prev_acc);
                    std::borrow::Cow::Owned(with_prev)
                } else {
                    std::borrow::Cow::Borrowed(input)
                }
            };
            // **v4.75.5** — check_answer pre-check. Before running any
            // math evaluator, see if input matches the «verify my
            // answer» pattern AND session has stored prior solve
            // (last_math_unknown + last_math_result). If so, skip math
            // eval entirely and surface check_answer slots; the planner
            // will pick `check_answer.correct` / `check_answer.incorrect`
            // family. Without this gate, «Жауабымды тексер: x=3» would
            // be re-solved as a fresh equation (x=3 → x is 3 trivially)
            // and the math_answer template family would fire instead of
            // the verification template.
            let mut check_answer_fired = false;
            if let (Some(last_unknown), Some(last_result_str)) = (
                self.session.get("last_math_unknown").cloned(),
                self.session.get("last_math_result").cloned(),
            ) && let Ok(last_value) = last_result_str.parse::<i64>()
                && let Some((user_var, user_value, correct)) =
                    crate::discourse::try_check_answer(input, &last_unknown, last_value)
            {
                extra_slots.insert("__check_answer_user_value__".into(), user_value.to_string());
                extra_slots.insert(
                    "__check_answer_correct_value__".into(),
                    last_value.to_string(),
                );
                extra_slots.insert("__check_answer_unknown__".into(), user_var);
                extra_slots.insert(
                    "__check_answer_correct__".into(),
                    if correct { "1".into() } else { "0".into() },
                );
                check_answer_fired = true;
            }
            let computed = if check_answer_fired {
                None
            } else {
                crate::discourse::try_evaluate_arithmetic(resolved_input.as_ref()).or_else(|| {
                    crate::discourse::try_evaluate_kazakh_word_math(resolved_input.as_ref())
                })
            };
            if check_answer_fired {
                // Skip the rest of the math eval cascade — the
                // check_answer slots are populated.
            } else if let Some(value) = computed {
                extra_slots.insert("__math_answer__".into(), value.to_string());
                if let Some(words) = crate::discourse::render_kazakh_number_words(value) {
                    extra_slots.insert("__math_words__".into(), words);
                }
                // **v4.51.5** — store the result in session for the
                // NEXT turn's anaphora resolution. Overwrite any
                // previous value (only the most-recent matters for
                // «алдыңғы нәтиже»).
                self.session
                    .insert("last_math_result".into(), value.to_string());
            } else if let Some((unknown, value, steps)) =
                crate::discourse::try_apply_formula(resolved_input.as_ref())
            {
                // **v4.74.5** — formula-applier. **v4.76.0** — also
                // receives step-narrative for explain_steps follow-up.
                extra_slots.insert("__math_answer__".into(), value.to_string());
                extra_slots.insert("__math_unknown__".into(), unknown.clone());
                if let Some(words) = crate::discourse::render_kazakh_number_words(value) {
                    extra_slots.insert("__math_words__".into(), words);
                }
                self.session
                    .insert("last_math_result".into(), value.to_string());
                self.session.insert("last_math_unknown".into(), unknown);
                self.session.insert("last_math_steps".into(), steps);
            } else if let Some((unknown, value, steps)) =
                crate::discourse::try_solve_linear_equation(resolved_input.as_ref())
            {
                // **v4.74.0** — linear-equation solver. **v4.76.0** —
                // also receives step-narrative for explain_steps.
                extra_slots.insert("__math_answer__".into(), value.to_string());
                extra_slots.insert("__math_unknown__".into(), unknown.clone());
                if let Some(words) = crate::discourse::render_kazakh_number_words(value) {
                    extra_slots.insert("__math_words__".into(), words);
                }
                self.session
                    .insert("last_math_result".into(), value.to_string());
                self.session.insert("last_math_unknown".into(), unknown);
                self.session.insert("last_math_steps".into(), steps);
            } else {
                extra_slots.insert("__math_input__".into(), "1".into());
            }
        }
        // **v4.76.0** — explain_steps post-pass. After the math-eval
        // block above, check whether input is a bare follow-up like
        // «Қалай шештің?» / «Процесін көрсет» / «Қадам-қадаммен
        // түсіндір». These don't match `input_is_math_expression` (no
        // digits/operators), so the math block above skips. Run
        // explain_steps as a separate gate: if session has
        // `last_math_steps` AND input contains an explain phrase,
        // surface the stored narrative. Skip if math eval already
        // produced an answer this turn (fresh solve takes priority).
        if !extra_slots.contains_key("__math_answer__")
            && !extra_slots.contains_key("__check_answer_correct__")
            && let Some(last_steps) = self.session.get("last_math_steps").cloned()
            && let Some(steps_text) = crate::discourse::try_explain_steps(input, &last_steps)
        {
            extra_slots.insert("__explain_steps__".into(), steps_text);
        }
        // **v4.76.5 / v4.77.0** — comparison-shape slot setup with
        // **v4.77.0 dual retrieval**. When try_extract_comparison_topics
        // fired earlier (line ~575), look up X's and Y's definitions
        // directly from `self.extracted_facts` (each Fact carries the
        // curated `raw_text` from world_core entries). Surface both
        // definitions in the comparison template so the user gets a
        // proper side-by-side answer instead of the v4.76.5 hedge.
        // Skip if a stronger signal already fired (math/check/explain).
        if let Some((ref x_topic, ref y_topic)) = comparison_topics
            && !extra_slots.contains_key("__math_answer__")
            && !extra_slots.contains_key("__check_answer_correct__")
            && !extra_slots.contains_key("__explain_steps__")
        {
            extra_slots.insert("__compare_x__".into(), x_topic.clone());
            extra_slots.insert("__compare_y__".into(), y_topic.clone());
            // **v4.77.0** — dual lookup. Search extracted_facts for the
            // first IsA fact about each topic; take raw_text as the
            // definition. IsA preferred (most definitional); any
            // predicate accepted as fallback. Lowercased root match.
            let lookup = |needle: &str| -> Option<String> {
                let needle_lower = needle.to_lowercase();
                // Pass 1: prefer IsA facts (most definitional)
                let by_isa = self.extracted_facts.iter().find(|f| {
                    f.subject.root.to_lowercase() == needle_lower
                        && matches!(f.predicate, adam_reasoning::Predicate::IsA)
                });
                if let Some(f) = by_isa {
                    return Some(f.raw_text.clone());
                }
                // Pass 2: any predicate
                self.extracted_facts
                    .iter()
                    .find(|f| f.subject.root.to_lowercase() == needle_lower)
                    .map(|f| f.raw_text.clone())
            };
            if let Some(x_def) = lookup(x_topic) {
                extra_slots.insert("__compare_x_def__".into(), x_def);
            }
            if let Some(y_def) = lookup(y_topic) {
                extra_slots.insert("__compare_y_def__".into(), y_def);
            }
        }
        // **v4.98.5** — curriculum slot pre-stuffing for the
        // `submit_solution.passed_stage_closed` /
        // `passed_curriculum_complete` planner sub-key remap. We
        // pre-resolve "if THIS pass succeeds, would it close the
        // stage?" and "what's the recommended next stage?" before the
        // planner runs cargo_verify. The planner reads these slots via
        // `extra_slots` and routes accordingly. If the verdict turns
        // out to be `failed`, the planner ignores the closure slots
        // (sub-key remap only fires on `passed`).
        // **v4.99.0** — student-side curriculum-query slot
        // population. AskNextTopic + AskCurrentProgress need
        // curriculum-derived slots so the realiser can fill the new
        // template families. AskNextTopic populates `next_stage_*` (or
        // `__curriculum_complete__` if all stages are closed);
        // AskCurrentProgress populates `progress_recap` with a
        // pre-rendered Kazakh prose recap (or `__progress_empty__` when
        // the student has no progress yet).
        // **v4.99.5** — adaptive-difficulty wiring. When the intent is
        // `AskExercise` with a topic that maps to a curriculum stage,
        // compute the per-stage difficulty hint from progress and
        // pre-stuff the corresponding tailored exercise body via
        // `extra_slots["exercise_body"]`. Because `extra_slots` is
        // merged into the planner's slots map AFTER `extract_slots`
        // runs (v4.95.0 plumbing), this overrides the default content
        // from `pedagogical::exercise_for` when a difficulty-tuned
        // variant exists. Hint == Normal AND no tailored variant →
        // no override; planner uses the canonical exercise. So
        // students with no recorded progress get the same content as
        // pre-v4.99.5.
        if let (Some(curriculum), Intent::AskExercise { topic: Some(t) }) =
            (self.curriculum.as_ref(), &intent_for_render)
            && let Some(stage) = curriculum.stage(t)
        {
            let progress = self
                .curriculum_progress
                .get(&stage.id)
                .copied()
                .unwrap_or_default();
            let hint = progress.difficulty_hint();
            if let Some(body) = crate::pedagogical::exercise_for_with_hint(t, hint) {
                extra_slots.insert("exercise_body".into(), body.into());
                extra_slots.insert("difficulty_hint".into(), format!("{hint:?}"));
            }
        }
        if let Some(curriculum) = self.curriculum.as_ref() {
            match &intent_for_render {
                Intent::AskNextTopic => {
                    if let Some(next) = curriculum.next_unlocked(&self.curriculum_progress) {
                        extra_slots.insert("next_stage_label_kk".into(), next.label_kk.clone());
                        extra_slots.insert("next_stage_summary_kk".into(), next.summary_kk.clone());
                        extra_slots.insert("next_stage_id".into(), next.id.clone());
                    } else {
                        extra_slots.insert("__curriculum_complete__".into(), "1".into());
                    }
                }
                Intent::AskCurrentProgress => {
                    if self.curriculum_progress.is_empty() {
                        extra_slots.insert("__progress_empty__".into(), "1".into());
                        // Surface the first-stage label as a starting
                        // hint inside `current_progress.empty`.
                        if let Some(first) = curriculum.next_unlocked(&self.curriculum_progress) {
                            extra_slots
                                .insert("next_stage_label_kk".into(), first.label_kk.clone());
                        }
                    } else {
                        let recap = curriculum.render_progress_recap_kk(&self.curriculum_progress);
                        extra_slots.insert("progress_recap".into(), recap);
                    }
                }
                _ => {}
            }
        }
        if let Intent::SubmitSolution {
            topic: ss_topic, ..
        } = &intent_for_render
            && let Some(curriculum) = self.curriculum.as_ref()
        {
            let lesson_stage_id = ss_topic
                .as_ref()
                .and_then(|t| curriculum.stage(t).map(|s| s.id.clone()))
                .or_else(|| {
                    self.session
                        .get("last_exercise_topic")
                        .and_then(|t| curriculum.stage(t).map(|s| s.id.clone()))
                });
            if let Some(stage_id) = lesson_stage_id
                && let Some(stage) = curriculum.stage(&stage_id)
            {
                let cur = self
                    .curriculum_progress
                    .get(&stage.id)
                    .copied()
                    .unwrap_or_default();
                let would_close = (cur.passed + 1) >= stage.exercises_to_pass;
                if would_close {
                    extra_slots.insert("__stage_closes__".into(), "1".into());
                    extra_slots.insert("stage_label_kk".into(), stage.label_kk.clone());
                    extra_slots.insert("stage_passes".into(), stage.exercises_to_pass.to_string());
                    // Build a hypothetical post-pass progress map and
                    // ask the curriculum what the next unlocked
                    // stage is.
                    let mut hypo = self.curriculum_progress.clone();
                    let hypo_entry = hypo.entry(stage.id.clone()).or_default();
                    hypo_entry.record_pass();
                    if let Some(next) = curriculum.next_unlocked(&hypo) {
                        extra_slots.insert("next_stage_label_kk".into(), next.label_kk.clone());
                        extra_slots.insert("next_stage_summary_kk".into(), next.summary_kk.clone());
                        extra_slots.insert("next_stage_id".into(), next.id.clone());
                    } else {
                        // Curriculum complete after this pass.
                        extra_slots.insert("__curriculum_complete__".into(), "1".into());
                    }
                }
            }
        }
        // **v5.4.0** — bare-yes/no-IsA query wiring. When the question
        // shape is YesNoCheck and the input parses as «<X> — <Y> Q?»,
        // walk the IsA chain across extracted + derived facts. The
        // outcome (`confirm` / `unknown`) and the rendered chain are
        // pushed through `extra_slots`; the planner branches on
        // `__yes_no_isa__` BEFORE the standard unknown-answer routing.
        // The bridge facts shipped in v5.4.0 (life_bridges + concept_
        // bridges) made transitive paths reachable for these queries.
        if let Intent::Unknown {
            question_shape: Some(crate::question_shape::QuestionShape::YesNoCheck),
            ..
        } = &intent_for_render
        {
            if let Some((subject, predicate)) =
                crate::question_shape::extract_yes_no_isa_pair(input)
            {
                let chain = find_isa_chain(
                    &self.extracted_facts,
                    &self.derived_facts,
                    &subject,
                    &predicate,
                );
                extra_slots.insert("__yes_no_isa__".into(), "1".into());
                extra_slots.insert("subject_term".into(), {
                    let mut chars = subject.chars();
                    match chars.next() {
                        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                        None => String::new(),
                    }
                });
                extra_slots.insert("predicate_term".into(), predicate.clone());
                match chain {
                    Some(path) if path.len() >= 2 => {
                        extra_slots.insert("__yes_no_outcome__".into(), "confirm".into());
                        extra_slots.insert("chain".into(), path.join(" → "));
                    }
                    _ => {
                        // **v5.4.8** — closed-class antonym denial.
                        // For known antonym pairs (тірі / жансыз),
                        // when the positive chain failed BUT the
                        // subject reaches the antonym hub, return an
                        // explicit "no" instead of "honest unknown".
                        // Pre-v5.4.8 «Тас — тірі ме?» fell to honest
                        // unknown despite reachable chain тас → жансыз
                        // нәрсе → жансыз. Conservative — only fires
                        // for the тірі / жансыз pair today.
                        let denial = check_antonym_denial(
                            &self.extracted_facts,
                            &self.derived_facts,
                            &subject,
                            &predicate,
                        );
                        if let Some(antonym_chain) = denial {
                            extra_slots.insert("__yes_no_outcome__".into(), "deny".into());
                            extra_slots.insert("chain".into(), antonym_chain.join(" → "));
                        } else {
                            extra_slots.insert("__yes_no_outcome__".into(), "unknown".into());
                        }
                    }
                }
            }
        }
        let plan = crate::planner::plan_response_with_epistemic(
            &intent_for_render,
            rng_seed,
            repo,
            &self.session,
            epistemic_status,
            &extra_slots,
        );
        let output = realise(&plan);

        // **v4.98.0** — lesson-state curriculum tracking. After a
        // SubmitSolution turn, look at the planner-produced
        // `cargo_status` slot and update per-stage progress on the
        // **lesson topic**.
        //
        // Lesson-topic resolution is two-tier:
        // 1. Prefer `plan.slots["topic"]` IF it matches a curriculum
        //    stage (the planner's v4.95.5 fallback already picked the
        //    most-specific topic available — student-named or
        //    session-derived).
        // 2. Else fall back to `session.last_exercise_topic` IF that
        //    matches a curriculum stage. This handles the case where
        //    the SubmitSolution detector extracted an incidental
        //    topic from the code itself (e.g. `println` from a
        //    snippet that happens to use that macro) instead of the
        //    actual lesson topic established by the prior
        //    AskExercise turn.
        //
        // Curriculum is `Option<_>` — when `None` (trimmed checkout)
        // the block is a no-op.
        if let Intent::SubmitSolution { .. } = &intent_for_render
            && let Some(curriculum) = self.curriculum.as_ref()
        {
            let cargo_status = plan.slots.get("cargo_status").map(String::as_str);
            let lesson_stage_id = plan
                .slots
                .get("topic")
                .and_then(|t| curriculum.stage(t).map(|s| s.id.clone()))
                .or_else(|| {
                    self.session
                        .get("last_exercise_topic")
                        .and_then(|t| curriculum.stage(t).map(|s| s.id.clone()))
                });
            if let (Some(status), Some(stage_id)) = (cargo_status, lesson_stage_id) {
                let entry = self.curriculum_progress.entry(stage_id).or_default();
                match status {
                    "passed" => entry.record_pass(),
                    "failed" => entry.record_fail(),
                    // env_error doesn't count either way — the
                    // failure is in the local `cargo` setup, not in
                    // the student's solution.
                    _ => {}
                }
            }
        }

        // **v4.6.0** — capture the topic noun this turn answered
        // about into `session["last_query_topic"]` so the next
        // turn's discourse-anaphora resolver («Ал онда қанша
        // аймақ бар?») can recover it. Updated only when the
        // current turn carried a recognised topic — empty / refused
        // turns leave the previous value intact (so a follow-up
        // anaphor still resolves to whatever was actually being
        // discussed).
        //
        // **v4.41.7** — skip update on math turns (math_input
        // OR math_answer set in extra_slots). Real-REPL transcript
        // 2026-05-03: «Бес санын үшке көбейтіп, 30-ды азайтыңыз»
        // (math turn) set last_query_topic to «сан» (the meta-
        // word "number" surfaced as topic before math detection
        // kicked in); the next turn «Содан кейін маған Rust
        // бағдарламалау тілі туралы айтып беріңізші?» fired
        // discourse-anaphora on «Содан» and reused «сан» as the
        // topic — surfacing «Сан — есептеу мен өлшеуге арналған
        // математикалық ұғым» instead of Rust info. Math turns
        // are not knowledge turns; they should not pollute the
        // anaphora-resolution context.
        let is_math_turn = extra_slots.contains_key("__math_answer__")
            || extra_slots.contains_key("__math_input__");
        if let Intent::Unknown {
            noun_hint: Some(topic),
            ..
        } = &intent_for_render
        {
            if !is_math_turn {
                self.session
                    .insert("last_query_topic".into(), topic.clone());
            }
            // **v4.18.0** — list-class context for cross-turn
            // anaphor list-requests. Stash the prior grounded
            // fact's text so the next turn's SearchGraph
            // tiebreaker can scan it for list-class tokens
            // (облыс/өзен/тау/etc) when the current query has
            // no explicit list-class token. Cleared on turns
            // with no grounded_fact so stale context doesn't
            // leak.
            if let Intent::Unknown {
                grounded_fact: Some(text),
                ..
            } = &intent_for_render
            {
                self.session
                    .insert("last_grounded_fact".into(), text.clone());
            } else {
                self.session.remove("last_grounded_fact");
            }
            // **v4.13.0** — multi-turn topic memory.
            // **v4.14.0** — domain inference now populated from
            // `domain_index`. When a `DomainIndex` is attached
            // (built from world_core at conversation startup),
            // each turn looks up the topic's primary domain and
            // records it. With enough turns the majority-vote in
            // `DialogContext::recompute_domain` settles on the
            // currently-discussed subject area; an empty index
            // (no `with_domain_index` call) leaves
            // `current_domain` at None, preserving v4.13.0
            // behaviour bit-for-bit.
            let domain_hint = self.domain_index.lookup_domain(topic);
            self.dialog_context
                .record_turn(self.turn_counter, topic, domain_hint, false);
        }
        // **v4.96.5** — Codex round-2 audit Bug 5. Pedagogical
        // intents (AskExercise / CodeRequest / ExplainCompilerError /
        // AskPurpose / CrossLanguageContrast / SubmitSolution) used
        // to NOT populate DialogContext — only `Intent::Unknown` did.
        // Consequence: after «Ownership туралы жаттығу беріңізші»
        // emitted a curated exercise, the next turn «оны қалай
        // шешеміз?» (anaphoric «how do we solve it?») had no
        // last_topic / subject_under_discussion to resolve against,
        // so «оны» fell through to last_query_topic which was unset.
        // Fix: record the topic from every topic-bearing pedagogical
        // intent so anaphora resolution and domain inference work
        // multi-turn within a lesson.
        let pedagogical_topic: Option<String> = match &intent_for_render {
            Intent::AskExercise { topic: Some(t) }
            | Intent::CodeRequest { topic: Some(t) }
            | Intent::ExplainCompilerError { topic: Some(t), .. }
            | Intent::AskPurpose { topic: Some(t) }
            | Intent::SubmitSolution { topic: Some(t), .. } => Some(t.clone()),
            Intent::CrossLanguageContrast { rust_concept, .. } => Some(rust_concept.clone()),
            _ => None,
        };
        if let Some(topic) = pedagogical_topic {
            self.session
                .insert("last_query_topic".into(), topic.clone());
            let domain_hint = self.domain_index.lookup_domain(&topic);
            self.dialog_context
                .record_turn(self.turn_counter, &topic, domain_hint, false);
        }
        // **v4.33.5** — sem_frames are now built earlier (right after
        // `parses`, before intent finalisation) so populated fields
        // can influence Intent / planner routing. The build below is
        // moved up; this block is preserved as a sentinel for the
        // historical pipeline location.
        let trace = TurnTrace {
            parses,
            sem_frames,
            intent_after_injection: intent,
            intent_after_verification: intent_for_render,
            session_snapshot: self.session.clone(),
            belief_digest: self.belief.digest(),
            belief_snapshot: self.belief.clone(),
            task_digest: self.task.digest(),
            task_snapshot: self.task.clone(),
            action_digest: action_plan.digest(),
            action_plan,
            verification,
            epistemic_status,
            tool_calls,
            plan_trace: plan.trace.clone(),
        };
        (output, trace)
    }

    /// v2.7: for `Intent::Unknown { noun_hint: Some(n), .. }`, scan
    /// the attached `derived_facts` for any derivation whose subject
    /// OR object root equals `n`. If found, render the source_chain
    /// into a Kazakh prose sentence and fill the `reasoning_chain`
    /// slot. The planner then routes to `unknown.with_derived_chain`.
    ///
    /// **Trust invariant** — every rendered chain carries the marker
    /// stem «байланыс-» ("connect-/relate-") so the user can always
    /// tell a reasoning citation apart from a verbatim corpus quote.
    /// This mirrors v1.9.5's «бейімд-» marker for adapted evidence.
    ///
    /// Deterministic: picks the first matching derivation (sorted
    /// order from the reasoner). No-op when `derived_facts` is empty,
    /// `example` is already set (retrieval wins if it ran), or the
    /// intent isn't Unknown / has no noun_hint.
    /// v4.0.24 — BFS depth from `subject` to `target` through **base**
    /// IsA edges only (`extracted_facts`). Used as a tie-breaker in
    /// `inject_reasoning_chain`: when two equally-scored R1 derivations
    /// both claim «X IsA Y», prefer the one whose Y is closer to X in
    /// the IsA graph (more direct).
    ///
    /// **Critically excludes derived facts** — otherwise the R1
    /// transitive closure itself creates depth-1 IsA edges from
    /// subject to every reachable object, collapsing all tied
    /// candidates to equal depth and defeating the tie-break. The
    /// base-fact-only walk recovers the authentic hop count.
    ///
    /// Returns `usize::MAX` when unreachable so tied callers fall through
    /// to the canonical-triple tie-break. Guarded by `MAX_DEPTH = 8` to
    /// bound pathological cases (graph cycles blocked by `visited`).

    /// **v4.2.0** — Build the list of `ToolCall`s the orchestrator
    /// should dispatch for this turn, given the resolved intent and
    /// parses. Replaces the hardcoded `inject_*` dispatch from v4.0.37
    /// → v4.1.5. The resulting plan is **data**, not code: adding a
    /// new tool consult means appending a `ToolCall` to this Vec,
    /// not writing a new helper method.
    ///
    /// Returns an empty Vec for non-`Intent::Unknown` intents and for
    /// `Unknown` without a `noun_hint` — same gate the pre-v4.2.0
    /// audit block enforced.
    ///
    /// Order is fixed and deterministic: SearchBelief (audit),
    /// SearchGraph (curated direct facts), SearchRetrieval (safe
    /// packs only, when `morpheme_index` attached), then
    /// RunLocalReasoner (when `derived_facts` non-empty). This gives
    /// user-facing chat a grounded direct-fact path before evidence
    /// quotes and keeps reasoning as the last fallback.
    fn tool_plan_for_turn(
        &self,
        intent: &Intent,
        parses: &[adam_kernel_fst::parser::Analysis],
    ) -> Vec<crate::tool::ToolCall> {
        let Intent::Unknown {
            noun_hint: Some(noun),
            ..
        } = intent
        else {
            return Vec::new();
        };
        let mut plan = Vec::new();

        // SearchBelief — always cheap audit. `predicate: None` so the
        // findings render as full triples for the trace.
        plan.push(crate::tool::ToolCall::SearchBelief {
            subject: crate::belief::USER_SELF_KEY.into(),
            predicate: None,
        });

        // SearchGraph — only human-approved direct facts are surfaced
        // into user-facing chat. Grammar-extracted graph entries stay
        // audit-only via the trace until they are curated.
        if !self.extracted_facts.is_empty() {
            if let Some(predicate) = graph_predicate_hint(intent) {
                plan.push(crate::tool::ToolCall::SearchGraph {
                    subject: noun.clone(),
                    predicate: Some(predicate),
                });
            }
            plan.push(crate::tool::ToolCall::SearchGraph {
                subject: noun.clone(),
                predicate: None,
            });
        }

        // SearchRetrieval — only when an index is attached. Empty
        // content_roots fall back to the bare `noun_hint` (matches
        // pre-v4.2.0 behaviour).
        if self.morpheme_index.is_some() {
            let roots = content_roots(parses);
            let morphemes = if roots.is_empty() {
                vec![noun.clone()]
            } else {
                roots
            };
            plan.push(crate::tool::ToolCall::SearchRetrieval { morphemes });
        }

        // RunLocalReasoner — only meaningful when derived_facts
        // attached. Honours `curated_only_reasoning` (v4.0.3
        // investor-safe gate, v4.1.2 routed through the tool).
        if !self.derived_facts.is_empty() {
            plan.push(crate::tool::ToolCall::RunLocalReasoner {
                topic: noun.clone(),
                curated_only: self.curated_only_reasoning,
            });
        }

        plan
    }

    /// **v4.2.0** — Fold tool results back into the intent. Pattern-
    /// matches on the originating `ToolCall` variant and writes the
    /// finding into the appropriate `Intent::Unknown` field. Replaces
    /// the v4.0.37 → v4.1.5 `inject_*` helpers' intent-mutation
    /// logic with a single uniform fold.
    ///
    /// `SearchBelief` stays audit-only; `SearchGraph` can now seed a
    /// grounded direct fact into `Intent::Unknown.example` before
    /// retrieval or reasoning run.
    fn apply_tool_results(
        &self,
        intent: &mut Intent,
        results: &[crate::tool::ToolResult],
        lexicon: &LexiconV1,
    ) {
        for result in results {
            match &result.call {
                crate::tool::ToolCall::SearchGraph { .. } => {
                    self.apply_graph_result(intent, result);
                }
                crate::tool::ToolCall::SearchRetrieval { .. } => {
                    self.apply_retrieval_result(intent, result, lexicon);
                }
                crate::tool::ToolCall::RunLocalReasoner { .. } => {
                    apply_reasoning_result(intent, result);
                }
                crate::tool::ToolCall::SearchBelief { .. } => {
                    // Audit-only: no intent mutation.
                }
            }
        }
    }

    fn apply_graph_result(&self, intent: &mut Intent, result: &crate::tool::ToolResult) {
        let Intent::Unknown { grounded_fact, .. } = intent else {
            return;
        };
        if grounded_fact.is_some() {
            return;
        }
        if let Some(text) = result.findings.first().cloned() {
            *grounded_fact = Some(text);
        }
    }

    /// Apply a `SearchRetrieval` tool result to `Intent::Unknown.example`,
    /// honouring (a) the v1.7.0 ranker's top-1 hit when present, (b) the
    /// v1.6.5 single-morpheme postings fallback when the tool found
    /// nothing, and (c) the v1.9.0 city-swap composition with the
    /// v1.9.5 `example_adapted` flag. Postings fallback stays local
    /// because `index.search()` is a different lookup mechanism than
    /// ranked search and doesn't fit `Tool::SearchRetrieval` semantics.
    fn apply_retrieval_result(
        &self,
        intent: &mut Intent,
        result: &crate::tool::ToolResult,
        lexicon: &LexiconV1,
    ) {
        let Intent::Unknown {
            noun_hint: Some(noun),
            example,
            grounded_fact: _,
            example_adapted,
            ..
        } = intent
        else {
            return;
        };
        if example.is_some() {
            return;
        }
        let Some(index) = self.morpheme_index.as_ref() else {
            return;
        };
        let candidate_text = result.findings.first().cloned().or_else(|| {
            index
                .search(noun)
                .iter()
                .find(|s| crate::tool::pack_is_chat_safe(&s.pack))
                .and_then(|s| index.sample_text(s).map(|t| t.to_string()))
        });
        if let Some(text) = candidate_text {
            let (composed_text, was_adapted) = self.maybe_compose(&text, lexicon);
            *example = Some(composed_text);
            *example_adapted = was_adapted;
        }
    }

    /// v1.9.0 option-B step. If [`compose_mode`](Self::compose_mode) is
    /// `InSampleCitySwap` and the session has a recognised city,
    /// rewrite city mentions in `text` to the user's city. Returns the
    /// (possibly rewritten) text and a flag indicating whether any
    /// swap actually happened. v1.9.5: the flag propagates to
    /// `Intent::Unknown.example_adapted` so the planner can route to
    /// the `unknown.with_adapted_evidence` family and the user sees an
    /// explicit "this quote was adapted" framing.
    ///
    /// All safety guards (biography year, known-place list) live in
    /// [`compose_with_city`].
    fn maybe_compose(&self, text: &str, lexicon: &LexiconV1) -> (String, bool) {
        if !matches!(self.compose_mode, ComposeMode::InSampleCitySwap) {
            return (text.to_string(), false);
        }
        let Some(user_city) = self.session.get("city") else {
            return (text.to_string(), false);
        };
        let composition = compose_with_city(text, user_city, lexicon);
        if composition.was_changed() {
            (composition.output, true)
        } else {
            (composition.original, false)
        }
    }

    /// Extract persistent entities from an intent and push them into
    /// the running session AND (v4.0.27) the structured belief state.
    ///
    /// The dual write is intentional: legacy template-slot consumers
    /// keep reading from `self.session`; the new belief-aware paths
    /// (contradiction detection, provenance tracing, future
    /// verifier / action planner) read from `self.belief`. Keeping
    /// them in lock-step avoids a disruptive migration.
    ///
    /// v4.0.30 — `turn_id` is threaded from the caller instead of
    /// derived from `intent_history.len()`. Codex v4.0.29 review #1:
    /// the old derivation plateaued at `MAX_HISTORY = 32`, breaking
    /// `recorded_at_turn` as a real turn index in long sessions.
    /// **v4.0.41** — given a fresh user turn, see if it resolves a
    /// pending `BeliefConflict`. The user's chosen value can come
    /// from an explicit `Statement*` intent (preferred) or from a
    /// raw-input substring match against the candidate object values
    /// (handles short replies like «астанада дұрыс» where the noun
    /// reaches the surface in locative form).
    ///
    /// On a match, mutates belief: chosen → `Active`, others →
    /// `Superseded`, drops the matching `BeliefConflict` and
    /// `ContradictionToResolve` pending question. The caller should
    /// **skip** `absorb_entities` for this turn so the user's
    /// chosen value isn't recorded as a duplicate `Active` fact.
    ///
    /// Returns `true` iff at least one contradiction was resolved.
    fn try_resolve_pending_contradiction(&mut self, input: &str, intent: &Intent) -> bool {
        if self.belief.contradictions.is_empty() {
            return false;
        }
        let input_lc = input.to_lowercase();
        let pending: Vec<(String, String)> = self
            .belief
            .contradictions
            .iter()
            .map(|c| (c.subject.clone(), c.predicate.clone()))
            .collect();
        let mut any_resolved = false;
        for (subject, predicate) in pending {
            let candidates: Vec<String> = self
                .belief
                .facts
                .iter()
                .filter(|f| f.subject == subject && f.predicate == predicate)
                .map(|f| f.object.clone())
                .collect();
            let chosen_from_intent: Option<String> = match (intent, predicate.as_str()) {
                (Intent::StatementOfLocation { city: Some(c) }, "city") => Some(c.clone()),
                (
                    Intent::StatementOfOccupation {
                        occupation: Some(o),
                    },
                    "occupation",
                ) => Some(o.clone()),
                (Intent::StatementOfName { name }, "name") => Some(name.clone()),
                (Intent::StatementOfAge { years: Some(y) }, "age") => Some(y.to_string()),
                _ => None,
            };
            let chosen = chosen_from_intent
                .as_ref()
                .and_then(|val| {
                    candidates
                        .iter()
                        .find(|c| c.eq_ignore_ascii_case(val))
                        .cloned()
                })
                .or_else(|| {
                    candidates
                        .iter()
                        .find(|c| input_lc.contains(&c.to_lowercase()))
                        .cloned()
                });
            if let Some(value) = chosen {
                if self
                    .belief
                    .resolve_contradiction(&subject, &predicate, &value)
                {
                    any_resolved = true;
                    // **v5.3.0** — Codex round-3 audit Bug 2 fix.
                    // Sync session profile slot to the chosen value
                    // so subsequent Ask{Predicate} turns surface it
                    // instead of the last-absorbed (now-Superseded)
                    // contested value. Mirrors the v4.96.0 dismissal
                    // sync logic.
                    if subject == crate::belief::USER_SELF_KEY {
                        self.session.insert(predicate.clone(), value.clone());
                        if predicate == "city" {
                            self.session.remove("city_id");
                            self.session.remove("geo_kind");
                        }
                        if predicate == "name" {
                            self.session.remove("name_id");
                            self.session.remove("name_respect");
                            self.session.remove("name_respect_distinct");
                        }
                    }
                }
            }
        }
        any_resolved
    }

    /// **v4.4.0** — Companion to
    /// [`try_resolve_pending_contradiction`]: instead of picking
    /// one of the contested values, drop them all to `Superseded`
    /// when the user explicitly opts out ("neither / I don't know
    /// / skip it").
    ///
    /// Detector phrases (gated by an existing pending contradiction
    /// — fires only when there is something to dismiss):
    /// - `екеуі де жоқ` / `екеуі де емес` ("neither")
    /// - `ешқайсысы дұрыс емес` ("none is correct")
    /// - `білмеймін` ("I don't know")
    /// - `өткізіп жібер` / `өткізіп жіберіңіз` ("skip it")
    /// - `маңызды емес` ("doesn't matter")
    /// - `аластат` / `жадтан өшір` / `ұмыт` ("clear / forget")
    ///
    /// Iterates over every pending contradiction and dismisses
    /// each — the user's blanket "I don't know" opts out of all
    /// pending conflicts at once, which is the natural UX. If
    /// future patches need per-conflict dismissal, the detector
    /// can be split.
    ///
    /// Returns `true` iff at least one contradiction was dismissed.
    /// Caller (turn_with_trace) then skips `absorb_entities` for
    /// the same reason `try_resolve_pending_contradiction` does:
    /// the user's input was a meta-dialog answer, not a fresh
    /// claim.
    fn try_dismiss_pending_contradiction(&mut self, input: &str) -> bool {
        if self.belief.contradictions.is_empty() {
            return false;
        }
        let input_lc = input.to_lowercase();
        let dismiss_phrase = input_lc.contains("екеуі де жоқ")
            || input_lc.contains("екеуі де емес")
            || input_lc.contains("ешқайсысы дұрыс емес")
            || input_lc.contains("білмеймін")
            || input_lc.contains("өткізіп жібер")
            || input_lc.contains("маңызды емес")
            || input_lc.contains("жадтан өшір")
            || input_lc.contains("ұмыт")
            || input_lc.contains("аластат");
        if !dismiss_phrase {
            return false;
        }
        let pending: Vec<(String, String)> = self
            .belief
            .contradictions
            .iter()
            .map(|c| (c.subject.clone(), c.predicate.clone()))
            .collect();
        let mut any_dismissed = false;
        for (subject, predicate) in pending {
            if self.belief.dismiss_contradiction(&subject, &predicate) {
                any_dismissed = true;
                // **v4.96.0** — Codex round-2 audit Bug 1 fix:
                // sync session profile slots with belief after
                // dismissal. Pre-fix `belief` correctly cleared
                // the disputed value, but `session.{predicate}`
                // stayed populated, so the next `Ask{Predicate}`
                // turn surfaced the stale value. Belief is now
                // single source of truth — clear matching session
                // slot when the dismissal targets a profile field.
                // (USER_SELF_KEY is the standard subject for user-
                // profile facts; only sync session for those.)
                if subject == crate::belief::USER_SELF_KEY {
                    let slot_key = predicate.as_str();
                    self.session.remove(slot_key);
                    // Side slots that derive from the predicate.
                    if predicate == "city" {
                        self.session.remove("city_id");
                        self.session.remove("geo_kind");
                    }
                    if predicate == "name" {
                        self.session.remove("name_id");
                        self.session.remove("name_respect");
                        self.session.remove("name_respect_distinct");
                    }
                }
            }
        }
        any_dismissed
    }

    pub(crate) fn absorb_entities(&mut self, intent: &Intent, turn_id: usize) {
        use crate::belief::{EntityKind, USER_SELF_KEY};
        match intent {
            Intent::StatementOfName { name } => {
                // **v4.3.1** — route the surface name through
                // `language_core::canonical_person_entity`. When the
                // resolver returns Some, the *canonical form*
                // (title-cased, mixed-script-cleaned) is what we
                // store in session and belief, and the stable
                // `person:<canonical>` id is recorded as
                // `EntityMemory.canonical_id` plus the auxiliary
                // `name_id` session slot. Surface variants like
                // `Дәулет` / `дәулет` / `дӘУЛEТ` therefore collapse
                // to one memory record.
                //
                // When the resolver returns None (single-char input,
                // digit-bearing, or a known geography name being
                // mis-stated as a person), we fall back to the
                // pre-v4.3.1 behaviour: store the raw surface and
                // skip `name_id`.
                if let Some(person) = crate::language_core::canonical_person_entity(name) {
                    self.session.insert("name".into(), person.canonical.clone());
                    self.session.insert("name_id".into(), person.id.clone());
                    // **v4.18.0** — also store the respectful
                    // address form. Adam uses `{name_respect}` in
                    // most templates so every post-introduction
                    // turn addresses the user as «Дәке / Мәке /
                    // Сәке» per Kazakh tradition. Vowel-initial
                    // names fall back to the literal name.
                    let respect_opt =
                        crate::language_core::kazakh_respectful_address(&person.canonical);
                    let respect = respect_opt
                        .clone()
                        .unwrap_or_else(|| person.canonical.clone());
                    self.session.insert("name_respect".into(), respect);
                    // **v4.18.5** — distinct slot only set when
                    // respect form differs from literal. Drives
                    // the warm-intro template that introduces
                    // both forms; vowel-initial names omit it
                    // and fall back to the simpler ack templates.
                    if let Some(distinct) = respect_opt {
                        self.session
                            .insert("name_respect_distinct".into(), distinct);
                    } else {
                        self.session.remove("name_respect_distinct");
                    }
                    self.belief.touch_entity(
                        USER_SELF_KEY,
                        EntityKind::User,
                        USER_SELF_KEY,
                        name,
                        Some(&person.id),
                        turn_id,
                    );
                    self.belief
                        .record_user_fact(USER_SELF_KEY, "name", &person.canonical, turn_id);
                } else {
                    self.session.insert("name".into(), name.clone());
                    self.session.remove("name_id");
                    // **v4.18.0** — same respectful form for non-
                    // canonical-registry names (covers any
                    // person-name shape the FST recovered).
                    let respect_opt = crate::language_core::kazakh_respectful_address(name);
                    let respect = respect_opt.clone().unwrap_or_else(|| name.clone());
                    self.session.insert("name_respect".into(), respect);
                    if let Some(distinct) = respect_opt {
                        self.session
                            .insert("name_respect_distinct".into(), distinct);
                    } else {
                        self.session.remove("name_respect_distinct");
                    }
                    self.belief.touch_entity(
                        USER_SELF_KEY,
                        EntityKind::User,
                        USER_SELF_KEY,
                        name,
                        None,
                        turn_id,
                    );
                    self.belief
                        .record_user_fact(USER_SELF_KEY, "name", name, turn_id);
                }
            }
            Intent::StatementOfAge { years: Some(years) } => {
                self.session.insert("age".into(), years.to_string());
                self.belief.touch_entity(
                    USER_SELF_KEY,
                    EntityKind::User,
                    USER_SELF_KEY,
                    "__self__",
                    None,
                    turn_id,
                );
                self.belief
                    .record_user_fact(USER_SELF_KEY, "age", &years.to_string(), turn_id);
            }
            Intent::StatementOfLocation { city: Some(city) } => {
                if let Some(entity) = canonical_geo_entity(city) {
                    self.session.insert("city".into(), entity.canonical.clone());
                    self.session.insert("city_id".into(), entity.id.clone());
                    self.session.insert("geo_kind".into(), entity.kind.clone());
                    self.belief.touch_entity(
                        &entity.id,
                        EntityKind::Place,
                        &entity.canonical,
                        city,
                        Some(&entity.id),
                        turn_id,
                    );
                    self.belief
                        .record_user_fact(USER_SELF_KEY, "city", &entity.canonical, turn_id);
                } else {
                    self.session.insert("city".into(), city.clone());
                    self.session.remove("city_id");
                    self.session.remove("geo_kind");
                    self.belief
                        .touch_entity(city, EntityKind::Place, city, city, None, turn_id);
                    self.belief
                        .record_user_fact(USER_SELF_KEY, "city", city, turn_id);
                }
            }
            Intent::StatementOfOccupation {
                occupation: Some(occupation),
            } => {
                self.session.insert("occupation".into(), occupation.clone());
                self.belief.touch_entity(
                    occupation,
                    EntityKind::Occupation,
                    occupation,
                    occupation,
                    None,
                    turn_id,
                );
                self.belief
                    .record_user_fact(USER_SELF_KEY, "occupation", occupation, turn_id);
            }
            // **v4.51.0** — capture user activity (current-work content)
            // distinct from occupation (profession label). Stored in
            // session slot `activity`; recalled by `Intent::AskActivity`
            // routing.
            Intent::StatementOfActivity {
                activity: Some(activity),
            } => {
                self.session.insert("activity".into(), activity.clone());
                self.belief
                    .record_user_fact(USER_SELF_KEY, "activity", activity, turn_id);
            }
            // **v4.95.5** — multi-turn lesson state. When adam emits
            // an exercise tied to a topic, remember the topic in
            // session so a subsequent SubmitSolution turn (which may
            // arrive with topic=None — the user types only the code
            // block) can still attribute the verdict to the active
            // exercise.
            Intent::AskExercise { topic: Some(topic) } => {
                self.session
                    .insert("last_exercise_topic".into(), topic.clone());
                self.session
                    .insert("last_exercise_turn".into(), turn_id.to_string());
            }
            // **v4.95.5** — same: when the user explicitly states a
            // topic in their CodeRequest, prime the lesson context.
            Intent::CodeRequest { topic: Some(topic) } => {
                self.session
                    .insert("last_exercise_topic".into(), topic.clone());
                self.session
                    .insert("last_exercise_turn".into(), turn_id.to_string());
            }
            _ => {}
        }
    }

    /// Update `active_intent` + push to `intent_history` with a bounded
    /// capacity. Called on every turn by [`turn`](Self::turn).
    fn record_intent(&mut self, intent: &Intent) {
        let kind = IntentKind::from(intent);
        self.active_intent = Some(kind);
        if self.intent_history.len() >= MAX_HISTORY {
            self.intent_history.remove(0);
        }
        self.intent_history.push(kind);
    }

    /// Clear all conversation state — slots, active intent, history,
    /// (v4.0.27) the belief state, (v4.0.29) the task state, and
    /// (v4.0.30) the turn counter.
    pub fn reset(&mut self) {
        self.session.clear();
        self.active_intent = None;
        self.intent_history.clear();
        self.belief = crate::belief::BeliefState::new();
        self.task = crate::task::TaskState::new();
        self.turn_counter = 0;
    }
}

/// v1.4.0 follow-up resolution. Some Kazakh utterances are meaningless
/// out of context but carry a pointer back to the previous turn:
///
///   "ал сіз?"     — "and you?"       (flip subject of last Ask-)
///   "ал сен?"     — informal version
///
/// When we detect a bare reflective-query as `Unknown` (or any
/// weak-intent), and the PREVIOUS turn was an Ask- intent, we re-tag
/// the current turn as the same Ask kind, so the planner picks a
/// matching response template.
/// Render a [`DerivedFact`] as a Kazakh prose sentence suitable for the
/// `unknown.with_derived_chain` template family. Every output contains
/// the marker stem «байланыс-» so consumers can distinguish a reasoning
/// citation from a verbatim corpus quote at the textual level alone
/// (trust invariant; mirrors v1.9.5's «бейімд-» marker for adapted
/// evidence).
///
/// v2.7 only handles `Predicate::RelatedTo` (the sole rule-derived
/// predicate currently emitted). Other predicates produce a generic
/// fallback that still contains the marker. Future releases will add
/// predicate-specific renderings as reasoner output grows.

/// v4.0.22 — composite score for ranking candidate derivations. Higher
/// scores pick first. Implements Codex v4.0.19 review recommendation
/// #3 — prefer curated+short+taxonomically-direct chains, penalize
/// text-only and R5/R2 shared-target fan-out.
///
/// The score ranges roughly 0..10. Terms:
///
///   + 4 if every `source_chain.pack` starts with `world_core/` (fully
///       curated — strongest trust signal)
///   + 1 if at least one source is curated but not all (mixed)
///   - 2 if no source is curated (text-only — last resort)
///
///   + 2 if `source_chain.len() <= 1`
///   + 1 if `source_chain.len() == 2`
///   (+ 0 longer — deep chains drift semantically)
///
///   + rule_weight — Codex ordering:
///       R1 IsA-transitivity, R10 InDomain-inheritance: 3 (clean taxonomic)
///       R2 Has-inheritance: 2
///       R3 Has-via-PartOf, R6 LivesIn-via-PartOf, R7 GoesTo-via-PartOf,
///       R9 PartOf-transitivity, R8 After-transitivity: 2 (mereological/temporal)
///       R5 shared-IsA, R11 shared-InDomain: 1 (weakest — combinatorial fan-out)
///       unknown rule: 1 (defensive default)
///
///   + 1 if subject matches `noun` (preserves pre-v4.0.22 subject-first preference)
///
/// Tie-break is by canonical triple (see `inject_reasoning_chain`).
/// **v4.2.0** — Apply a `RunLocalReasoner` tool result to the
/// `Intent::Unknown.reasoning_chain` field. Pure function — no
/// `Conversation` dependency since the picker + renderer (and the
/// `curated_only` gate) live entirely inside `Tool::RunLocalReasoner`
/// since v4.1.2; this helper only writes the rendered finding.
fn apply_reasoning_result(intent: &mut Intent, result: &crate::tool::ToolResult) {
    let Intent::Unknown {
        reasoning_chain, ..
    } = intent
    else {
        return;
    };
    if reasoning_chain.is_some() {
        return;
    }
    if let Some(rendered) = result.findings.first().cloned() {
        *reasoning_chain = Some(rendered);
    }
}

fn graph_predicate_hint(intent: &Intent) -> Option<String> {
    let Intent::Unknown { raw_tokens, .. } = intent else {
        return None;
    };
    if raw_tokens
        .iter()
        .any(|token| token.starts_with("шектес") || token.starts_with("шекара"))
    {
        return Some("related_to".into());
    }
    if raw_tokens
        .iter()
        .any(|token| matches!(token.as_str(), "қанша" | "неше"))
    {
        return Some("has_quantity".into());
    }
    None
}

/// **v5.4.0** — find the shortest IsA chain from `subject` to `target`
/// across both extracted (curated/world_core) and derived (rule-inferred)
/// facts. Returns the chain of node roots `[subject, hop1, …, target]`
/// when reachable, `None` otherwise.
///
/// Used by the bare-yes/no-IsA route («X — Y ме?») wired in v5.4.0:
/// the planner needs the full path so the response can cite it
/// («қасқыр → жыртқыш → жануар → тіршілік иесі → тірі»), not just a
/// boolean. BFS so the cited path is the *shortest*; depth is capped at
/// 8 (matching the reasoner's `MAX_ITER`) so cycles never deadlock.
///
/// Both `subject` and `target` are matched against `f.subject.root` /
/// `f.object.root` after lower-casing on both sides; multi-word roots
/// like `"тіршілік иесі"` are preserved as-is by the upstream loader.
pub(crate) fn find_isa_chain(
    extracted: &[ReasFact],
    derived: &[DerivedFact],
    subject: &str,
    target: &str,
) -> Option<Vec<String>> {
    // **v5.4.7** — possessive-multiword fallback. Try the literal
    // (subject, target) first; if no chain is found and `target` is
    // an "izafet" construction like «дененің бөлігі» (genitive +
    // possessive on a two-word noun phrase), retry with the bare
    // form «дене бөлігі». World_core stores the bare form; user
    // queries can use either surface, so the chain-lookup is the
    // right place to bridge them.
    if let Some(chain) = find_isa_chain_inner(extracted, derived, subject, target) {
        return Some(chain);
    }
    if let Some(normalised) = strip_izafet_genitive(target) {
        if normalised != target {
            if let Some(chain) = find_isa_chain_inner(extracted, derived, subject, &normalised) {
                return Some(chain);
            }
        }
    }
    // **v5.4.8** — «X түрі» / «X-тың түрі» head-extraction. The user
    // phrasing «Кітап — заттың түрі ме?» asks "is a book a kind of
    // object?" — semantically equivalent to "is a book an object?"
    // The head noun is what the chain should match. Strip the
    // trailing «түрі» / «түр» marker and any preceding genitive,
    // then retry. Sister fix to izafet stripping above; reduces
    // possessive-of meaning to plain IsA.
    if let Some(normalised) = strip_kind_of_marker(target) {
        if normalised != target {
            if let Some(chain) = find_isa_chain_inner(extracted, derived, subject, &normalised) {
                return Some(chain);
            }
        }
    }
    None
}

/// **v5.4.8** — strip the trailing «X (-тың)? түрі» / «X (-тың)? түр»
/// "kind-of" marker. Returns `Some(head)` when the input matches the
/// pattern; `None` otherwise. Conservative — only fires on two-word
/// phrases ending with the marker word, and the head must be ≥ 3 chars.
fn strip_kind_of_marker(phrase: &str) -> Option<String> {
    let trimmed = phrase.trim();
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    if parts.len() != 2 {
        return None;
    }
    if parts[1] != "түрі" && parts[1] != "түр" {
        return None;
    }
    // Strip optional genitive on the head before returning.
    if let Some(stripped) = strip_izafet_genitive(&format!("{} {}", parts[0], parts[1])) {
        let bare: Vec<&str> = stripped.split_whitespace().collect();
        if bare.len() == 2 && bare[0].chars().count() >= 3 {
            return Some(bare[0].to_string());
        }
    }
    if parts[0].chars().count() >= 3 {
        return Some(parts[0].to_string());
    }
    None
}

/// **v5.4.8** — closed-class antonym denial. When the positive IsA
/// chain failed for `(subject, predicate)`, check whether the
/// subject reaches the antonym hub of the predicate; if so, return
/// the antonym chain so the planner can render an explicit "no".
///
/// Antonym table (intentionally tiny — only architecturally clear
/// closed-class pairs):
///
///   - тірі ↔ жансыз / жансыз нәрсе
///
/// Future pairs need a curator decision; we don't speculate.
fn check_antonym_denial(
    extracted: &[ReasFact],
    derived: &[DerivedFact],
    subject: &str,
    predicate: &str,
) -> Option<Vec<String>> {
    const ANTONYMS: &[(&str, &[&str])] = &[("тірі", &["жансыз", "жансыз нәрсе"])];
    let predicate_lc = predicate.to_lowercase();
    for (positive, antonym_targets) in ANTONYMS {
        if predicate_lc == *positive {
            for antonym in *antonym_targets {
                if let Some(chain) = find_isa_chain_inner(extracted, derived, subject, antonym) {
                    return Some(chain);
                }
            }
        }
    }
    None
}

/// **v5.4.7** — strip the genitive marker from a two-word
/// "X-Genitive Y-Possessive" izafet construction so the bare
/// compound form can be looked up. Returns `None` for inputs that
/// don't match the pattern.
///
/// Recognised genitive suffixes (Kazakh): `-ның / -нің / -дың /
/// -дің / -тың / -тің`. Conservative: only fires on a two-word
/// input where the first word ends in one of those suffixes and
/// the residue is at least 3 chars.
fn strip_izafet_genitive(phrase: &str) -> Option<String> {
    let parts: Vec<&str> = phrase.split_whitespace().collect();
    if parts.len() != 2 {
        return None;
    }
    const GENITIVE_SUFFIXES: &[&str] = &["ның", "нің", "дың", "дің", "тың", "тің"];
    for suf in GENITIVE_SUFFIXES {
        if let Some(stem) = parts[0].strip_suffix(suf) {
            if stem.chars().count() >= 3 {
                return Some(format!("{stem} {}", parts[1]));
            }
        }
    }
    None
}

fn find_isa_chain_inner(
    extracted: &[ReasFact],
    derived: &[DerivedFact],
    subject: &str,
    target: &str,
) -> Option<Vec<String>> {
    const MAX_DEPTH: usize = 8;
    let subject = subject.to_lowercase();
    let target = target.to_lowercase();
    if subject == target {
        return Some(vec![subject]);
    }
    let mut parent: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut frontier: Vec<String> = vec![subject.clone()];
    parent.insert(subject.clone(), String::new());

    for _ in 0..MAX_DEPTH {
        let mut next: Vec<String> = Vec::new();
        for node in &frontier {
            // Direct extracted IsA edges out of `node`.
            for f in extracted {
                if matches!(f.predicate, ReasPredicate::IsA)
                    && f.subject.root.to_lowercase() == *node
                {
                    let obj = f.object.root.to_lowercase();
                    if !parent.contains_key(&obj) {
                        parent.insert(obj.clone(), node.clone());
                        if obj == target {
                            return Some(reconstruct_chain(&parent, &target));
                        }
                        next.push(obj);
                    }
                }
            }
            // Rule-derived IsA edges (R1 transitivity) out of `node`.
            // Treat them as one BFS hop — derivations already telescope
            // across multiple original facts, so a chain through derived
            // edges is shorter than the same chain through extracted
            // ones; preserve that for the cited path.
            for d in derived {
                if matches!(d.predicate, adam_reasoning::Predicate::IsA)
                    && d.subject.root.to_lowercase() == *node
                {
                    let obj = d.object.root.to_lowercase();
                    if !parent.contains_key(&obj) {
                        parent.insert(obj.clone(), node.clone());
                        if obj == target {
                            return Some(reconstruct_chain(&parent, &target));
                        }
                        next.push(obj);
                    }
                }
            }
        }
        if next.is_empty() {
            return None;
        }
        frontier = next;
    }
    None
}

fn reconstruct_chain(
    parent: &std::collections::HashMap<String, String>,
    target: &str,
) -> Vec<String> {
    let mut chain = vec![target.to_string()];
    let mut cur = target.to_string();
    while let Some(p) = parent.get(&cur) {
        if p.is_empty() {
            break;
        }
        chain.push(p.clone());
        cur = p.clone();
    }
    chain.reverse();
    chain
}

/// **v4.1.2** — extracted from `Conversation::isa_chain_depth` so the
/// reasoning-chain tool dispatcher can compute IsA-depth tie-breaks
/// without requiring a `Conversation` reference. Pure function over
/// the extracted-fact slice.
pub(crate) fn isa_chain_depth(extracted: &[ReasFact], subject: &str, target: &str) -> usize {
    const MAX_DEPTH: usize = 8;
    if subject == target {
        return 0;
    }
    let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut frontier: Vec<String> = vec![subject.to_string()];
    visited.insert(subject.to_string());
    for depth in 1..=MAX_DEPTH {
        let mut next: Vec<String> = Vec::new();
        for node in &frontier {
            for f in extracted {
                if f.predicate == ReasPredicate::IsA && f.subject.root == *node {
                    if f.object.root == target {
                        return depth;
                    }
                    if visited.insert(f.object.root.clone()) {
                        next.push(f.object.root.clone());
                    }
                }
            }
        }
        if next.is_empty() {
            break;
        }
        frontier = next;
    }
    usize::MAX
}

pub(crate) fn score_derivation(d: &DerivedFact, noun: &str) -> i32 {
    let mut score: i32 = 0;

    // 1. Trust (source_chain provenance).
    if d.source_chain.is_empty() {
        score -= 2;
    } else {
        let wc_count = d
            .source_chain
            .iter()
            .filter(|s| s.pack.starts_with("world_core/"))
            .count();
        if wc_count == d.source_chain.len() {
            score += 4; // fully curated
        } else if wc_count > 0 {
            score += 1; // mixed
        } else {
            score -= 2; // text-only
        }
    }

    // 2. Chain length.
    match d.source_chain.len() {
        0 | 1 => score += 2,
        2 => score += 1,
        _ => {}
    }

    // 3. Rule weight.
    score += match d.rule_id.as_str() {
        "R1_is_a_transitivity" | "R10_in_domain_inheritance" => 3,
        "R2_has_inheritance"
        | "R3_has_inheritance_via_part_of"
        | "R6_lives_in_via_part_of"
        | "R7_goes_to_via_part_of"
        | "R8_after_transitivity"
        | "R9_part_of_transitivity" => 2,
        "R5_shared_is_a_target" | "R11_in_domain_shared_target" => 1,
        _ => 1,
    };

    // 4. Subject-side preference.
    if d.subject.root == noun {
        score += 1;
    }

    // v4.0.24 — 5. Predicate preference. For "tell me about X" dialog
    // queries, an IsA answer ("X is a Y") is the most semantically
    // direct form. Codex v4.0.23 re-review flagged that the pre-v4.0.24
    // reranker tied multiple curated R1 + R10 candidates on the same
    // noun and the canonical-triple tie-break surfaced weaker picks
    // (InDomain over IsA for `немере`). This +2 boost makes IsA the
    // default winner when all other trust/length/rule terms are equal.
    if d.predicate == adam_reasoning::Predicate::IsA {
        score += 2;
    }

    score
}

pub(crate) fn render_derivation_as_kazakh(d: &DerivedFact) -> String {
    // Every output MUST keep the marker stem «байланыс-» so the user
    // can distinguish a reasoned statement from a direct fact or a
    // retrieval quote. The marker now lives in one consistent prefix;
    // the predicate-specific clause that follows is kept short and
    // human-readable.
    let clause = match d.predicate {
        ReasPredicate::RelatedTo => {
            format!("{} мен {} өзара байланысты", d.subject.root, d.object.root)
        }
        ReasPredicate::IsA => format!("{} — {}", d.subject.root, d.object.root),
        ReasPredicate::Has => {
            format!(
                "{} {} ие болады",
                d.subject.root,
                inflect(&d.object.root, Case::Dative)
            )
        }
        ReasPredicate::GoesTo => {
            format!(
                "{} {} барады",
                d.subject.root,
                inflect(&d.object.root, Case::Dative)
            )
        }
        ReasPredicate::LivesIn => {
            format!(
                "{} {} тұрады",
                d.subject.root,
                inflect(&d.object.root, Case::Locative)
            )
        }
        ReasPredicate::PartOf => {
            format!(
                "{} {} құрамына кіреді",
                d.subject.root,
                inflect(&d.object.root, Case::Dative)
            )
        }
        ReasPredicate::Causes => {
            format!(
                "{} {} себеп болады",
                d.subject.root,
                inflect(&d.object.root, Case::Dative)
            )
        }
        ReasPredicate::After => {
            format!(
                "{} {} кейін келеді",
                d.subject.root,
                inflect(&d.object.root, Case::Ablative)
            )
        }
        ReasPredicate::HasQuantity => {
            format!(
                "{} {} қатысты сандық байланысқа ие",
                d.subject.root,
                inflect(&d.object.root, Case::Instrumental)
            )
        }
        ReasPredicate::DoesTo => {
            format!("{} {} үстінде әрекет етеді", d.subject.root, d.object.root)
        }
        ReasPredicate::InDomain => {
            format!("{} {} саласына жатады", d.subject.root, d.object.root)
        }
    };
    format!("байланыс бойынша, {clause}")
}

/// v3.8.5 — synthesise a noun in the requested grammatical case via FST.
/// Replaces the pre-v3.8.5 practice of manually concatenating a hyphen +
/// invariant suffix (`{root}-ға` etc.), which produced surface forms
/// like `атау-ға` that violate Kazakh vowel harmony.
fn inflect(root: &str, case: Case) -> String {
    let mut features = NounFeatures::default();
    features.case = Some(case);
    synthesise_noun(root, features)
}

fn resolve_follow_up(raw: Intent, input: &str, active: Option<IntentKind>) -> Intent {
    let normalised: String = input.to_lowercase();
    let is_reflective = normalised.trim() == "ал сіз"
        || normalised.trim() == "ал сіз?"
        || normalised.trim() == "ал сен"
        || normalised.trim() == "ал сен?"
        || normalised.trim() == "сіз ше"
        || normalised.trim() == "сен ше";
    if !is_reflective {
        return raw;
    }
    // Only reroute when the raw intent is weak (Unknown / Affirmation)
    // — never override a clearly-recognised strong intent.
    if !matches!(
        raw,
        Intent::Unknown { .. } | Intent::Affirmation | Intent::Negation
    ) {
        return raw;
    }
    match active {
        Some(IntentKind::AskHowAreYou) | Some(IntentKind::StatementOfWellbeing) => {
            Intent::AskHowAreYou
        }
        Some(IntentKind::AskName) | Some(IntentKind::StatementOfName) => Intent::AskName,
        Some(IntentKind::AskAge) | Some(IntentKind::StatementOfAge) => Intent::AskAge,
        Some(IntentKind::AskLocation) | Some(IntentKind::StatementOfLocation) => {
            Intent::AskLocation
        }
        Some(IntentKind::AskOccupation) | Some(IntentKind::StatementOfOccupation) => {
            Intent::AskOccupation
        }
        _ => raw,
    }
}
