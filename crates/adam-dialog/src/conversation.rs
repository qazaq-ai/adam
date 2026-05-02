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
    /// `full_name = Nano Language Model`, `creator =
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
            Intent::Unknown { .. } => Self::Unknown,
        }
    }
}

impl Conversation {
    /// Start a fresh session — no remembered entities and no history.
    pub fn new() -> Self {
        Self::default()
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
        let raw_intent = interpret_text_with_lexicon(stripped, &parses, Some(lexicon));

        // v1.4.0: follow-up resolution. "ал сіз?" after AskHowAreYou
        // becomes a re-interpretation: "user is asking me the same
        // thing back", which is still AskHowAreYou for planning
        // purposes (planner picks a response without asking back).
        let mut intent = resolve_follow_up(raw_intent, input, self.active_intent);

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
                    ref mut noun_hint, ..
                } = intent
                {
                    *noun_hint = Some(prev_topic);
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
        let russian_input = crate::discourse::input_is_likely_russian(input);

        // **v4.6.12** — Math-expression detection. Real-REPL
        // 2026-04-29: «5+5» / «7 + 3 =» / «6:2=» / «5-ті 7-ге
        // көбейткенде» / «алтыны екіге бөліңіз» — adam doesn't
        // compute math (per `limitations_summary`), so route to
        // the dedicated `math_refusal` template family. Detection
        // requires a clear math shape (operator near digits OR
        // math verb + numeric tokens) — pure mentions of numbers
        // («Қазақстанда 17 облыс бар») don't trigger.
        let math_input = crate::discourse::input_is_math_expression(input);

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
            if let Some(value) = crate::discourse::try_evaluate_arithmetic(input) {
                extra_slots.insert("__math_answer__".into(), value.to_string());
            } else {
                extra_slots.insert("__math_input__".into(), "1".into());
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

        // **v4.6.0** — capture the topic noun this turn answered
        // about into `session["last_query_topic"]` so the next
        // turn's discourse-anaphora resolver («Ал онда қанша
        // аймақ бар?») can recover it. Updated only when the
        // current turn carried a recognised topic — empty / refused
        // turns leave the previous value intact (so a follow-up
        // anaphor still resolves to whatever was actually being
        // discussed).
        if let Intent::Unknown {
            noun_hint: Some(topic),
            ..
        } = &intent_for_render
        {
            self.session
                .insert("last_query_topic".into(), topic.clone());
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
        let trace = TurnTrace {
            parses,
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
